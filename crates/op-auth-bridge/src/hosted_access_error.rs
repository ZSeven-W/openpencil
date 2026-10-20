use std::fmt;

/// Closed, credential-free failures across the Hosted access boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostedAccessError {
    Unavailable,
    NotSignedIn,
    NetworkUnavailable,
    RateLimited,
    RequestRejected,
    Cancelled,
    RequestNotFound,
    InvalidPayload,
    Expired,
    Internal,
}

impl fmt::Display for HostedAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "Hosted access is unavailable",
            Self::NotSignedIn => "Hosted access requires a signed-in account",
            Self::NetworkUnavailable => "Hosted access network is unavailable",
            Self::RateLimited => "Hosted access was rate limited",
            Self::RequestRejected => "Hosted access request was rejected",
            Self::Cancelled => "Hosted access request was cancelled",
            Self::RequestNotFound => "Hosted access request was not found",
            Self::InvalidPayload => "Hosted access response is invalid",
            Self::Expired => "Hosted access has expired",
            Self::Internal => "Hosted access failed",
        })
    }
}

impl std::error::Error for HostedAccessError {}
