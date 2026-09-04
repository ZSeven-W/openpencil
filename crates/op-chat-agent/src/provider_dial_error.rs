//! Typed failures for the connect-time endpoint guard (`provider_dial.rs`).
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. Every
//! variant carries STRUCTURED fields and `Display` re-formats the sentence,
//! so the text a browser sees (these messages reach the chat SSE stream and
//! the image routes' JSON error bodies) is reproduced byte for byte while
//! callers can match on the reason instead of the prose.
//!
//! What the enum adds is a name for the two failure shapes this guard exists
//! to separate. [`ProviderDialError::Reserved`] / [`ProviderDialError::Unresolved`]
//! are the SECURITY verdicts — the endpoint resolved to something the daemon
//! must not connect to (DNS rebinding, an SSRF hop) — whereas `NotAUrl` /
//! `MissingHost` / `MissingPort` are ordinary malformed-input faults and
//! `ClientBuild` is a local reqwest configuration failure that says nothing
//! about the endpoint at all. Stringly-typed, those three kinds were
//! indistinguishable to every caller.
//!
//! Two seams still speak `String`, both deliberately:
//!
//! - `import_html_url.rs::fetch_capped` calls [`super::provider_dial::client_for`]
//!   with `?` inside a `Result<_, String>` of its own. That file is outside
//!   this conversion pass, so the [`From<ProviderDialError> for String`] bridge
//!   at the bottom keeps it compiling — and byte-identical — without an edit.
//!   Delete the bridge when that module converts.
//! - `chat_builtin_http::BuiltinHttpError` and
//!   `web_image_generate::ImageGenerateError` are both `pub` while this
//!   module is crate-private (`mod provider_dial;` in `lib.rs`), so they
//!   carry the dial failure as an already-rendered sentence rather than
//!   leaking a crate-private type into a public enum.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderDialError {
    /// The configured endpoint is not a parseable URL.
    NotAUrl,
    /// The URL parsed but names no host, so there is nothing to resolve or
    /// screen.
    MissingHost,
    /// The URL parsed but carries neither an explicit port nor a scheme with
    /// a known default, so no socket address can be formed.
    MissingPort,
    /// DNS lookup for the endpoint host failed outright.
    ResolveFailed { host: String, message: String },
    /// DNS lookup succeeded but returned an empty address set. A security
    /// verdict, not a transport hiccup: with nothing to screen there is
    /// nothing to pin the connection to.
    Unresolved { host: String },
    /// At least one resolved address falls in a reserved range. The whole
    /// resolution is refused — a mixed public/private answer is exactly the
    /// DNS-rebinding shape this guard exists for.
    Reserved { host: String },
    /// reqwest refused to build the client (local configuration, not the
    /// endpoint). Shared with `chat_builtin_http::builtin_http_client`, which
    /// produces the same sentence for the same reason.
    ClientBuild { message: String },
}

impl fmt::Display for ProviderDialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderDialError::NotAUrl => f.write_str("provider endpoint is not a valid URL"),
            ProviderDialError::MissingHost => f.write_str("provider endpoint has no host"),
            ProviderDialError::MissingPort => f.write_str("provider endpoint has no port"),
            ProviderDialError::ResolveFailed { host, message } => {
                write!(f, "provider endpoint {host} did not resolve: {message}")
            }
            ProviderDialError::Unresolved { host } => {
                write!(f, "provider endpoint {host} did not resolve")
            }
            ProviderDialError::Reserved { host } => {
                write!(f, "provider endpoint {host} resolves to a reserved address")
            }
            ProviderDialError::ClientBuild { message } => {
                write!(f, "Failed to configure provider HTTP client: {message}")
            }
        }
    }
}

impl std::error::Error for ProviderDialError {}
