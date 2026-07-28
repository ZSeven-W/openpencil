use crate::CollabTicketError;
use std::fmt;
use std::num::NonZeroU64;
use zeroize::Zeroizing;

/// Maximum opaque ticket size accepted from a provider.
pub const MAX_COLLAB_TICKET_BYTES: usize = 48 * 1024;

/// Input to a collaboration-ticket request.
///
/// Identity, device id, role, and author are deliberately absent: the private
/// provider derives identity from its authenticated device session, while the
/// open verifier derives the principal from the signed ticket.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CollabTicketRequest {
    dh_pub_x25519: [u8; 32],
}

impl CollabTicketRequest {
    pub fn new(dh_pub_x25519: [u8; 32]) -> Result<Self, CollabTicketError> {
        if dh_pub_x25519.iter().all(|byte| *byte == 0) {
            return Err(CollabTicketError::InvalidDhPublicKey);
        }
        Ok(Self { dh_pub_x25519 })
    }

    pub fn dh_pub_x25519(&self) -> &[u8; 32] {
        &self.dh_pub_x25519
    }
}

impl fmt::Debug for CollabTicketRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollabTicketRequest")
            .field("dh_pub_x25519", &"[REDACTED]")
            .finish()
    }
}

/// Non-zero handle returned by an asynchronous ticket provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollabTicketRequestId(NonZeroU64);

impl CollabTicketRequestId {
    pub fn new(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self)
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

/// Signed ticket bytes returned by the private provider.
///
/// Debug output is always redacted. The wrapper is intentionally not `Clone`
/// so callers do not accidentally multiply credential-bearing buffers.
pub struct OpaqueCollabTicket(Zeroizing<Vec<u8>>);

impl OpaqueCollabTicket {
    pub fn new(bytes: Vec<u8>) -> Result<Self, CollabTicketError> {
        // Wrap before validation so rejected credential-bearing input is also
        // zeroized on every return path.
        let bytes = Zeroizing::new(bytes);
        if bytes.is_empty() || bytes.len() > MAX_COLLAB_TICKET_BYTES {
            return Err(CollabTicketError::InvalidTicketSize {
                actual: bytes.len(),
                maximum: MAX_COLLAB_TICKET_BYTES,
            });
        }
        Ok(Self(bytes))
    }

    /// Expose the ticket only to the transport/verifier boundary.
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for OpaqueCollabTicket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpaqueCollabTicket")
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}

/// Result of polling an asynchronous collaboration-ticket request.
#[derive(Debug)]
pub enum CollabTicketPoll {
    Pending,
    Ready {
        ticket: OpaqueCollabTicket,
        /// Non-authoritative scheduling hint. The open verifier must use the
        /// signed `exp` claim as the security boundary.
        expires_at_unix_hint: Option<u64>,
    },
    Failed(CollabTicketError),
}

/// Public adapter implemented by the proprietary credential holder.
///
/// The adapter returns only an opaque ticket. Verification keys come from the
/// pinned issuer's public key endpoint, never from this provider. Request ids
/// are unique for the provider lifetime; `Ready` and `Failed` are consuming
/// terminal states, so later polls return `RequestNotFound`.
pub trait CollabTicketProvider: Send + Sync {
    fn available(&self) -> bool;

    fn begin_ticket(
        &self,
        request: CollabTicketRequest,
    ) -> Result<CollabTicketRequestId, CollabTicketError>;

    fn poll_ticket(&self, id: CollabTicketRequestId) -> CollabTicketPoll;

    fn cancel_ticket(&self, id: CollabTicketRequestId);
}

/// Default provider for builds without the proprietary ABI implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableCollabTicketProvider;

impl CollabTicketProvider for UnavailableCollabTicketProvider {
    fn available(&self) -> bool {
        false
    }

    fn begin_ticket(
        &self,
        _request: CollabTicketRequest,
    ) -> Result<CollabTicketRequestId, CollabTicketError> {
        Err(CollabTicketError::Unavailable)
    }

    fn poll_ticket(&self, _id: CollabTicketRequestId) -> CollabTicketPoll {
        CollabTicketPoll::Failed(CollabTicketError::Unavailable)
    }

    fn cancel_ticket(&self, _id: CollabTicketRequestId) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_rejects_an_all_zero_dh_key() {
        assert_eq!(
            CollabTicketRequest::new([0; 32]),
            Err(CollabTicketError::InvalidDhPublicKey)
        );
        assert!(CollabTicketRequest::new([7; 32]).is_ok());
    }

    #[test]
    fn request_debug_output_redacts_the_device_binding_key() {
        let request = CollabTicketRequest::new([7; 32]).unwrap();
        let debug = format!("{request:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("7, 7"));
    }

    #[test]
    fn request_ids_must_be_non_zero() {
        assert_eq!(CollabTicketRequestId::new(0), None);
        assert_eq!(CollabTicketRequestId::new(9).map(|id| id.get()), Some(9));
    }

    #[test]
    fn opaque_ticket_debug_output_is_redacted() {
        let ticket = OpaqueCollabTicket::new(b"header.payload.signature".to_vec()).unwrap();
        let debug = format!("{ticket:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("payload"));
        assert_eq!(ticket.expose(), b"header.payload.signature");
    }

    #[test]
    fn opaque_ticket_enforces_bounds() {
        assert!(matches!(
            OpaqueCollabTicket::new(Vec::new()),
            Err(CollabTicketError::InvalidTicketSize { actual: 0, .. })
        ));
        assert!(matches!(
            OpaqueCollabTicket::new(vec![0; MAX_COLLAB_TICKET_BYTES + 1]),
            Err(CollabTicketError::InvalidTicketSize { .. })
        ));
    }

    #[test]
    fn unavailable_provider_fails_closed() {
        let provider = UnavailableCollabTicketProvider;
        let request = CollabTicketRequest::new([3; 32]).unwrap();
        assert!(!provider.available());
        assert_eq!(
            provider.begin_ticket(request),
            Err(CollabTicketError::Unavailable)
        );
        let id = CollabTicketRequestId::new(1).unwrap();
        assert!(matches!(
            provider.poll_ticket(id),
            CollabTicketPoll::Failed(CollabTicketError::Unavailable)
        ));
        provider.cancel_ticket(id);
    }

    #[test]
    #[cfg(not(op_auth_collab_ticket_prebuilt))]
    fn current_v1_bridge_exposes_collaboration_as_a_separate_capability() {
        assert!(!crate::collab_ticket_available());
        assert!(!crate::collab_ticket_provider().available());
        assert_eq!(crate::REQUIRED_ABI_VERSION, 1);
        assert_eq!(crate::COLLAB_TICKET_ABI_VERSION, 2);
    }
}
