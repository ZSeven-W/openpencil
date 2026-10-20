//! Typed failures for resolving a browser AI request onto a `ChatProvider`
//! (`ai_proxy::proxy_provider_for_request{,_with_chat_session}`).
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. The three
//! sentences this module authors are re-formatted from unit variants, and the
//! endpoint verdict is carried as the typed [`WebCredentialError`] it already
//! was, so the SSE `error` event the browser receives is byte-identical.
//!
//! What the enum adds is the distinction the flat strings hid: two of these
//! are a MISMATCH between the request envelope and the transient credential
//! attached to it (a confused or malicious client), while the third and
//! fourth are the SSRF endpoint screen refusing the credential's `base_url`.
//! The first pair means "this request contradicts itself"; the second pair
//! means "this endpoint is not one the deployment will dial". Only the second
//! is a deployment-policy decision an operator can change (via
//! `OPENPENCIL_WEB_AI_ENDPOINT_ALLOWLIST`), which is exactly the difference an
//! operator reading a log needs.

use std::fmt;

use crate::web_credentials_error::WebCredentialError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyProviderError {
    Hosted(crate::hosted_models::HostedRuntimeError),
    /// The request's `model` and its attached transient credential's `model`
    /// disagree. Refused rather than resolved against either one.
    TransientModelMismatch,
    /// The request names a `provider` that the attached transient
    /// credential's kind does not map to.
    TransientProviderMismatch,
    /// The credential's `base_url` failed the shared SSRF screen. Carries
    /// [`WebCredentialError`] so the specific verdict (malformed / not
    /// allowed / needs HTTPS / not in the allowlist) survives the hop.
    EndpointRejected(WebCredentialError),
    /// The endpoint parsed and passed the generic screen, but the deployment's
    /// public-demo policy still refuses it — a private, loopback, or reserved
    /// address needs an explicit allowlist entry.
    EndpointNotPermittedByDeployment,
}

impl fmt::Display for ProxyProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hosted(error) => error.fmt(f),
            ProxyProviderError::TransientModelMismatch => {
                f.write_str("transient credential model does not match the request")
            }
            ProxyProviderError::TransientProviderMismatch => {
                f.write_str("transient credential provider does not match the request")
            }
            ProxyProviderError::EndpointRejected(error) => error.fmt(f),
            ProxyProviderError::EndpointNotPermittedByDeployment => f.write_str(
                "provider endpoint is not allowed: private, loopback, and reserved addresses \
                 require an OPENPENCIL_WEB_AI_ENDPOINT_ALLOWLIST entry",
            ),
        }
    }
}

impl std::error::Error for ProxyProviderError {}

/// Lets the endpoint screen be applied with a plain `?`.
impl From<WebCredentialError> for ProxyProviderError {
    fn from(error: WebCredentialError) -> ProxyProviderError {
        ProxyProviderError::EndpointRejected(error)
    }
}
