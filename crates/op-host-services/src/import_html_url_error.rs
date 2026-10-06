//! Typed failures for the `import_html_url` MCP tool
//! (`import_html_url.rs`) — the daemon-side "fetch a live web page and import
//! it as nodes" path.
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. The variants
//! carry STRUCTURED fields and `Display` re-formats the sentence, so the text
//! the model receives in `ToolOutcome::Err` is reproduced byte for byte while
//! the tool can match on the reason instead of the prose.
//!
//! What the enum adds is [`ImportHtmlUrlError::tool_error_code`]: which
//! `ToolErrorCode` a failure reports was previously a literal chosen
//! independently at each of the tool's four `return ToolOutcome::Err(…)`
//! sites, and the split it encodes is load-bearing — an
//! [`ToolErrorCode::InvalidArgument`] tells the model to fix the `url`
//! argument, while [`ToolErrorCode::ToolFailed`] tells it the argument was
//! fine and the network was not. Centralising it means the SSRF screen cannot
//! drift into being reported as a transient failure the model will retry.
//!
//! One inbound seam speaks `String`: `provider_dial::client_for`, a sibling
//! module this pass does not own, whose message is carried verbatim by
//! [`ImportHtmlUrlError::Dial`].

use std::fmt;

use op_mcp::ToolErrorCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportHtmlUrlError {
    /// The `url` argument (or a redirect target) is not a parseable URL.
    UrlInvalid,
    /// The URL parsed but is refused by the SSRF screen: a non-HTTP(S)
    /// scheme, embedded credentials, or — absent an explicit
    /// `OPENPENCIL_WEB_AI_ENDPOINT_ALLOWLIST` entry — a private, loopback,
    /// link-local, or cloud-metadata target.
    UrlNotAllowed,
    /// Building a dialing client for the screened endpoint failed.
    /// `provider_dial` is not owned by this pass, so its message is carried
    /// verbatim.
    Dial(String),
    /// The request never produced a response (DNS, TLS, timeout, reset).
    Fetch { url: String, detail: String },
    /// The server answered, but not with success.
    HttpStatus { url: String, status: String },
    /// The redirect chain exceeded the hop cap, or the loop fell through
    /// without a terminal response.
    TooManyRedirects,
    /// A `3xx` response carried no usable `Location` header.
    RedirectMissingLocation,
    /// A `Location` header that will not resolve against the current URL.
    RedirectLocationInvalid,
    /// The response body exceeded the byte cap for its tier (10 MiB for the
    /// page, 4 MiB for a subresource).
    ResponseTooLarge { url: String },
    /// No import slot was available for this page.
    TooManyConcurrentJobs,
    /// The successful response was not HTML and did not look like HTML.
    NotHtmlPage,
    /// The HTML importer produced no nodes.
    NoImportableContent { detail: String },
}

impl ImportHtmlUrlError {
    /// The `ToolErrorCode` this failure reports to the model.
    ///
    /// The two URL-screen verdicts are argument faults — the model can fix
    /// them by passing a different `url` — so they answer
    /// `InvalidArgument`, matching the pre-conversion call site exactly.
    /// Everything else happened after the argument was accepted and is
    /// reported as `ToolFailed`, again matching the previous literals.
    pub(crate) fn tool_error_code(&self) -> ToolErrorCode {
        match self {
            ImportHtmlUrlError::UrlInvalid | ImportHtmlUrlError::UrlNotAllowed => {
                ToolErrorCode::InvalidArgument
            }
            ImportHtmlUrlError::Dial(_)
            | ImportHtmlUrlError::Fetch { .. }
            | ImportHtmlUrlError::HttpStatus { .. }
            | ImportHtmlUrlError::TooManyRedirects
            | ImportHtmlUrlError::RedirectMissingLocation
            | ImportHtmlUrlError::RedirectLocationInvalid
            | ImportHtmlUrlError::ResponseTooLarge { .. }
            | ImportHtmlUrlError::TooManyConcurrentJobs
            | ImportHtmlUrlError::NotHtmlPage => ToolErrorCode::ToolFailed,
            ImportHtmlUrlError::NoImportableContent { .. } => ToolErrorCode::InvalidArgument,
        }
    }
}

impl fmt::Display for ImportHtmlUrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportHtmlUrlError::UrlInvalid => f.write_str("import URL is invalid"),
            ImportHtmlUrlError::UrlNotAllowed => f.write_str("import URL is not allowed"),
            ImportHtmlUrlError::Dial(message) => f.write_str(message),
            ImportHtmlUrlError::Fetch { url, detail } => {
                write!(f, "failed to fetch {url}: {detail}")
            }
            ImportHtmlUrlError::HttpStatus { url, status } => {
                write!(f, "failed to fetch {url}: HTTP {status}")
            }
            ImportHtmlUrlError::TooManyRedirects => {
                f.write_str("too many redirects while fetching html")
            }
            ImportHtmlUrlError::RedirectMissingLocation => {
                f.write_str("redirect response is missing a valid Location")
            }
            ImportHtmlUrlError::RedirectLocationInvalid => {
                f.write_str("redirect Location is not a valid URL")
            }
            ImportHtmlUrlError::ResponseTooLarge { url } => {
                write!(f, "response from {url} exceeds the size cap")
            }
            ImportHtmlUrlError::TooManyConcurrentJobs => {
                f.write_str("too many concurrent import jobs")
            }
            ImportHtmlUrlError::NotHtmlPage => f.write_str("not an html page"),
            ImportHtmlUrlError::NoImportableContent { detail } => {
                write!(f, "no importable content: {detail}")
            }
        }
    }
}

impl std::error::Error for ImportHtmlUrlError {}
