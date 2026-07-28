use std::fmt;

/// Errors exposed by the public collaboration-ticket provider boundary.
///
/// Provider implementations must keep credentials, raw tickets, and remote
/// error bodies out of these values because hosts may surface or log them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollabTicketError {
    Unavailable,
    InvalidDhPublicKey,
    InvalidTicketSize { actual: usize, maximum: usize },
    RequestNotFound { id: u64 },
    ProviderFailure { code: CollabTicketProviderErrorCode },
}

/// Closed, log-safe errors accepted from the private provider boundary.
///
/// Raw HTTP bodies, credentials, and provider messages stay private.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollabTicketProviderErrorCode {
    NotSignedIn,
    NetworkUnavailable,
    RateLimited,
    RequestRejected,
    Internal,
}

impl fmt::Display for CollabTicketError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => write!(formatter, "collaboration ticket provider unavailable"),
            Self::InvalidDhPublicKey => {
                write!(formatter, "collaboration DH public key is invalid")
            }
            Self::InvalidTicketSize { actual, maximum } => write!(
                formatter,
                "collaboration ticket is {actual} bytes; maximum is {maximum}"
            ),
            Self::RequestNotFound { id } => {
                write!(formatter, "collaboration ticket request {id} was not found")
            }
            Self::ProviderFailure { code } => {
                write!(
                    formatter,
                    "collaboration ticket provider failed with code {}",
                    code.as_str()
                )
            }
        }
    }
}

impl std::error::Error for CollabTicketError {}

impl CollabTicketProviderErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotSignedIn => "not_signed_in",
            Self::NetworkUnavailable => "network_unavailable",
            Self::RateLimited => "rate_limited",
            Self::RequestRejected => "request_rejected",
            Self::Internal => "internal",
        }
    }
}
