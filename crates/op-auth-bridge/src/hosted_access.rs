use crate::HostedAccessError;
use std::fmt;
use std::num::NonZeroU64;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

pub const MAX_HOSTED_ACCESS_BYTES: usize = 8 * 1024;

/// A short-lived, non-persistent Gateway credential from the private runtime.
///
/// This receipt is not proof of the current UI account. Hosts must also enforce
/// their current verified session, session epoch, cancellation, and expiry.
/// No serialization or cloning implementation is provided deliberately.
pub struct HostedAccess {
    pub(crate) subject: String,
    pub(crate) issuer: String,
    pub(crate) gateway_base_url: String,
    pub(crate) expires_at_unix: u64,
    pub(crate) bearer: Zeroizing<String>,
}

impl HostedAccess {
    /// Validate and consume a trusted provider's wire reply, zeroizing the input
    /// on success and rejection. Do not accept this payload from UI or RPC input.
    pub fn from_provider_json(bytes: Vec<u8>) -> Result<Self, HostedAccessError> {
        let bytes = Zeroizing::new(bytes);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| HostedAccessError::Internal)?
            .as_secs();
        crate::hosted_access_wire::decode(&bytes, now)
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn gateway_base_url(&self) -> &str {
        &self.gateway_base_url
    }

    pub fn expires_at_unix(&self) -> u64 {
        self.expires_at_unix
    }

    /// Expose only to the native Gateway HTTP transport, never UI or persistence.
    pub fn bearer(&self) -> &str {
        &self.bearer
    }
}

impl fmt::Debug for HostedAccess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostedAccess")
            .field("credential", &"[REDACTED]")
            .finish()
    }
}

/// An independent, non-zero Hosted request handle; never a login or collab id.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HostedAccessRequestId(NonZeroU64);

impl HostedAccessRequestId {
    pub fn new(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self)
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug)]
pub enum HostedAccessPoll {
    Pending,
    Ready(HostedAccess),
    Failed(HostedAccessError),
}

/// Native-only credential provider. Ready and Failed consume a request, and
/// subsequent polls return RequestNotFound. Cancellation affects only its id.
pub trait HostedAccessProvider: Send + Sync {
    fn available(&self) -> bool;
    /// Current private session generation. Zero means uninitialized/exhausted;
    /// a nonzero generation alone does not prove the user is signed in. Hosts
    /// compare it before begin, after completion, and while streaming.
    fn session_epoch(&self) -> u64 {
        0
    }
    fn begin(&self) -> Result<HostedAccessRequestId, HostedAccessError>;
    fn poll(&self, id: HostedAccessRequestId) -> Result<HostedAccessPoll, HostedAccessError>;
    fn cancel(&self, id: HostedAccessRequestId);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableHostedAccessProvider;

impl HostedAccessProvider for UnavailableHostedAccessProvider {
    fn available(&self) -> bool {
        false
    }

    fn begin(&self) -> Result<HostedAccessRequestId, HostedAccessError> {
        Err(HostedAccessError::Unavailable)
    }

    fn poll(&self, _id: HostedAccessRequestId) -> Result<HostedAccessPoll, HostedAccessError> {
        Err(HostedAccessError::Unavailable)
    }

    fn cancel(&self, _id: HostedAccessRequestId) {}
}

#[cfg(test)]
#[path = "hosted_access_tests.rs"]
mod tests;
