//! Typed failures for the built-in API-key chat providers: the plain
//! streaming path (`chat_builtin_http.rs`), its SSE/endpoint helpers
//! (`chat_builtin_http_wire.rs`), and the tool-executing agent loops
//! (`chat_agent_loop.rs` + `chat_agent_loop/{anthropic,openai}.rs`).
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. Most
//! variants carry STRUCTURED fields and `Display` re-formats the sentence, so
//! the text is reproduced byte for byte. That matters more here than almost
//! anywhere else in the crate: these messages become `ChatDelta::Error`, land
//! in the chat transcript, and — for
//! [`BuiltinHttpError::RateLimited`] — are PATTERN-MATCHED downstream.
//! `op_orchestrator::retry::is_non_retryable` looks for the literal substring
//! `http 429` (case-insensitively) to stop the subtask retry ladder at
//! attempt 1; an earlier wording without "http" silently missed that
//! classifier and burned two more full LLM calls on a live rate limit. Do not
//! reword that variant's `Display` without updating the classifier.
//!
//! ## Why one enum for both the plain path and the agent loops
//!
//! `ConfiguredBuiltinProvider::send` picks between the plain streaming path
//! and the agent loop at runtime and matches on ONE `Result`, so the two
//! branches must report the same type. They are also genuinely one failure
//! domain — the agent loop reuses this module's `send_with_backoff`, its
//! backoff knobs, and the same Anthropic / OpenAI-compatible endpoints. The
//! only failure the loops add is [`BuiltinHttpError::StreamReported`]: an
//! error the provider announced INSIDE an otherwise healthy HTTP-200 SSE
//! body, which the plain path turns into a `ChatDelta::Error` mid-stream but
//! the loops must return so the finalize backstop runs (see
//! `run_anthropic_agent_loop`'s doc comment on the 0718-1-k3-1 postmortem).
//!
//! ## Seams that still report `String`
//!
//! - [`BuiltinHttpError::Dial`] carries the connect-time guard's sentence
//!   already rendered. `provider_dial::ProviderDialError` is crate-private
//!   (`mod provider_dial;`) while this enum is `pub`, so embedding it would
//!   leak a private type into a public interface.
//! - `op_ai::chat_provider::ChatDelta::Error(String)` is the transcript sink
//!   and belongs to a crate this pass does not own, so `send` renders this
//!   enum with `to_string()` on the way out. That is a value conversion, not
//!   a `Result<_, String>` signature.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinHttpError {
    /// The eagerly-built `Trusted` client is missing because construction
    /// already failed — the provider still answers so the turn reports a
    /// cause instead of panicking.
    ClientUnavailable,
    /// The connect-time endpoint guard (`provider_dial`) refused to produce a
    /// client. Text is that guard's own sentence, verbatim.
    Dial(String),
    /// The account is rate-limited and the backoff ladder could not ride it
    /// out. `Display` MUST keep the literal `HTTP 429` — see the module doc.
    RateLimited { label: String, max_retries: u32 },
    /// A non-retryable (or retry-exhausted) HTTP status. The provider's own
    /// error BODY is deliberately dropped, not carried: those bodies are
    /// untrusted and can echo request headers, and this text reaches the
    /// chat transcript and browser SSE stream.
    HttpStatus {
        label: String,
        status: reqwest::StatusCode,
    },
    /// The request exceeded the client's deadline. Split from
    /// [`BuiltinHttpError::Transport`] because reqwest reports it distinctly
    /// and the wording differs.
    Timeout {
        label: String,
        url: String,
        message: String,
    },
    /// A connection-level failure after the retry ladder was exhausted.
    Transport {
        label: String,
        url: String,
        message: String,
    },
    /// Reading the SSE body failed part-way through. The turn may already
    /// have streamed text.
    SseStream { message: String },
    /// The provider announced an error inside an HTTP-200 SSE body. Text is
    /// the provider's own message (Anthropic's `error.message`, the
    /// OpenAI-compatible `/error/message`), which the agent loops surface
    /// verbatim.
    StreamReported(String),
    /// The configured base URL is not a parseable URL. Message is the `url`
    /// crate's own parse error, an upstream crate this pass does not own.
    InvalidEndpointUrl { message: String },
    /// The base URL parsed but uses a scheme this transport cannot dial.
    EndpointUnsupportedScheme { scheme: String },
    /// The base URL parsed but names no host.
    EndpointMissingHost,
    /// The base URL carries a query string or fragment. Refused rather than
    /// silently dropped, because endpoint paths are concatenated.
    EndpointHasQueryOrFragment,
}

impl fmt::Display for BuiltinHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuiltinHttpError::ClientUnavailable => {
                f.write_str("Provider HTTP client is unavailable")
            }
            BuiltinHttpError::Dial(message) | BuiltinHttpError::StreamReported(message) => {
                f.write_str(message)
            }
            BuiltinHttpError::RateLimited { label, max_retries } => write!(
                f,
                "The model provider is rate-limiting this account (HTTP 429) and the run could not ride it out after {max_retries} retries. Wait a moment, then send the prompt again to continue. ({label})"
            ),
            BuiltinHttpError::HttpStatus { label, status } => write!(f, "{label} http {status}"),
            BuiltinHttpError::Timeout {
                label,
                url,
                message,
            } => write!(f, "{label} POST {url} timed out: {message}"),
            BuiltinHttpError::Transport {
                label,
                url,
                message,
            } => write!(f, "{label} POST {url}: {message}"),
            BuiltinHttpError::SseStream { message } => write!(f, "sse stream: {message}"),
            BuiltinHttpError::InvalidEndpointUrl { message } => {
                write!(f, "Invalid provider endpoint: {message}")
            }
            BuiltinHttpError::EndpointUnsupportedScheme { scheme } => write!(
                f,
                "Invalid provider endpoint: unsupported URL scheme '{scheme}'"
            ),
            BuiltinHttpError::EndpointMissingHost => {
                f.write_str("Invalid provider endpoint: URL must include a host")
            }
            BuiltinHttpError::EndpointHasQueryOrFragment => f.write_str(
                "Invalid provider endpoint: query strings and fragments are not allowed",
            ),
        }
    }
}

impl std::error::Error for BuiltinHttpError {}

/// Single-table adaptation of the connect-time endpoint guard, replacing the
/// per-call-site `.map_err(...)` the dial sites used to carry. `Display` on
/// both sides is the same sentence, so the transcript text is unchanged.
impl From<crate::provider_dial::ProviderDialError> for BuiltinHttpError {
    fn from(error: crate::provider_dial::ProviderDialError) -> BuiltinHttpError {
        BuiltinHttpError::Dial(error.to_string())
    }
}
