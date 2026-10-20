use std::fmt;

/// Closed errors intentionally exclude endpoints, credentials and upstream bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedRuntimeError {
    Unavailable,
    NotSignedIn,
    SessionChanged,
    AccessRejected,
    CatalogInvalid,
    CatalogTooLarge,
    RouteUnavailable,
    UnsupportedProtocol,
    UnsupportedAttachments,
    Network,
    GatewayRejected,
    Cancelled,
}

impl fmt::Display for HostedRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "Hosted models are unavailable in this build",
            Self::NotSignedIn => "Sign in to use Hosted models",
            Self::SessionChanged => "Hosted session changed; select a current model",
            Self::AccessRejected => "Hosted access could not be authorized",
            Self::CatalogInvalid => "Hosted model catalog is invalid",
            Self::CatalogTooLarge => "Hosted model catalog exceeds its size limit",
            Self::RouteUnavailable => "Hosted model is no longer available",
            Self::UnsupportedProtocol => "This Hosted model protocol is not supported by this host",
            Self::UnsupportedAttachments => {
                "Attachments are not supported by this Hosted transport"
            }
            Self::Network => "Hosted gateway connection failed",
            Self::GatewayRejected => "Hosted gateway rejected the request",
            Self::Cancelled => "Hosted request was cancelled",
        })
    }
}

impl std::error::Error for HostedRuntimeError {}
