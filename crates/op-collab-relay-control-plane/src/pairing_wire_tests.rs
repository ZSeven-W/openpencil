//! Pairing publish/claim wire-format tests, split from `tests.rs` at the
//! 800-line cap.

use crate::{
    PairingClaimRequest, PairingPublishRequest, RelayLocatorIssueError, MAX_PAIRING_CODE_TTL_SECS,
    MAX_PAIRING_PUBLISH_REQUEST_BYTES, PAIRING_CLAIM_REQUEST_BYTES,
};
use op_collab_relay_protocol::MAX_SEALED_INVITE_BYTES;

#[test]
fn pairing_publish_round_trips_and_bounds_every_field() {
    let request =
        PairingPublishRequest::new([7; 32], [9; 16], 600, vec![1, 2, 3, 4]).expect("valid");
    let raw = request.encode_binary();
    assert!(raw.len() <= MAX_PAIRING_PUBLISH_REQUEST_BYTES);
    let decoded = PairingPublishRequest::decode_binary(&raw).expect("round trip");
    assert_eq!(decoded, request);

    // Truncation and trailing bytes both fail the exact length check.
    for length in 0..raw.len() {
        assert!(PairingPublishRequest::decode_binary(&raw[..length]).is_err());
    }
    let mut trailing = raw.clone();
    trailing.push(0);
    assert!(PairingPublishRequest::decode_binary(&trailing).is_err());

    let mut wrong_version = raw;
    wrong_version[0] ^= 0xFF;
    assert!(matches!(
        PairingPublishRequest::decode_binary(&wrong_version),
        Err(RelayLocatorIssueError::UnsupportedRequestVersion { .. })
    ));

    // TTL and sealed-blob bounds are enforced at construction.
    assert!(PairingPublishRequest::new([7; 32], [9; 16], 0, vec![1]).is_err());
    assert!(
        PairingPublishRequest::new([7; 32], [9; 16], MAX_PAIRING_CODE_TTL_SECS + 1, vec![1])
            .is_err()
    );
    assert!(PairingPublishRequest::new([7; 32], [9; 16], 600, Vec::new()).is_err());

    let legacy_ceiling =
        PairingPublishRequest::new([7; 32], [8; 16], 600, vec![0; MAX_SEALED_INVITE_BYTES])
            .expect("opaque transport retains the v0.8.4 sealed-blob ceiling");
    assert_eq!(
        legacy_ceiling.encode_binary().len(),
        MAX_PAIRING_PUBLISH_REQUEST_BYTES
    );
}

#[test]
fn pairing_claim_round_trips_and_is_exact() {
    let request = PairingClaimRequest::new([3; 32], [5; 16]);
    let raw = request.encode_binary();
    assert_eq!(raw.len(), PAIRING_CLAIM_REQUEST_BYTES);
    let decoded = PairingClaimRequest::decode_binary(&raw).expect("round trip");
    assert_eq!(decoded, request);
    for length in 0..raw.len() {
        assert!(PairingClaimRequest::decode_binary(&raw[..length]).is_err());
    }
    let mut trailing = raw.to_vec();
    trailing.push(0);
    assert!(PairingClaimRequest::decode_binary(&trailing).is_err());
    assert_eq!(format!("{request:?}"), "PairingClaimRequest([REDACTED])");
}

mod pairing_service_tests {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use op_auth_bridge::{
        CollabJwksCacheLimits, CollabTicketVerifier, OpaqueCollabTicket, StaticTestJwksFetcher,
        TestCollabIssuer, TestCollabTicketSpec,
    };

    use crate::{
        PairingClaimRequest, PairingCodeStore, PairingPublishRequest, PairingPutOutcome,
        PairingStoreRejection, RelayPairingService, RelayPairingServiceError,
        MAX_PAIRING_CODE_TTL_SECS,
    };

    type PutLog = Arc<Mutex<Vec<(([u8; 32], [u8; 16]), u64, u64)>>>;

    struct RecordingStore(PutLog);

    impl PairingCodeStore for RecordingStore {
        fn put(
            &self,
            owner: [u8; 32],
            code_id: [u8; 16],
            _sealed: Vec<u8>,
            now_unix: u64,
            expires_at_unix: u64,
        ) -> Result<PairingPutOutcome, PairingStoreRejection> {
            self.0
                .lock()
                .expect("puts lock")
                .push(((owner, code_id), now_unix, expires_at_unix));
            Ok(PairingPutOutcome::Stored)
        }

        fn claim(&self, _code_id: &[u8; 16], _now_unix: u64) -> Option<Vec<u8>> {
            Some(vec![0xAB])
        }
    }

    fn service_fixture(
        device: [u8; 32],
    ) -> (
        RelayPairingService<StaticTestJwksFetcher, RecordingStore>,
        OpaqueCollabTicket,
        u64,
        PutLog,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_secs();
        let issuer = TestCollabIssuer::initial();
        let ticket = issuer
            .issue(&TestCollabTicketSpec::valid_at(now, device))
            .expect("ticket");
        let verifier = CollabTicketVerifier::new(
            TestCollabIssuer::verifier_config().expect("verifier config"),
            StaticTestJwksFetcher::new(issuer.jwks_json().expect("JWKS"), 300),
            CollabJwksCacheLimits::default(),
        )
        .expect("ticket verifier");
        let log: PutLog = Arc::default();
        (
            RelayPairingService::new(verifier, RecordingStore(Arc::clone(&log))),
            ticket,
            now,
            log,
        )
    }

    #[test]
    fn pairing_endpoints_reject_a_ticket_bound_to_a_different_device() {
        let (service, ticket, now, log) = service_fixture([0x11; 32]);
        // Body claims device 0x22 while the ticket is bound to 0x11.
        let request =
            PairingPublishRequest::new([0x22; 32], [9; 16], 600, vec![1, 2, 3]).expect("request");
        assert_eq!(
            service.publish_at(
                &request.encode_binary(),
                ticket.expose(),
                now,
                Instant::now()
            ),
            Err(RelayPairingServiceError::AuthenticationFailed)
        );
        assert!(log.lock().expect("puts lock").is_empty());

        let claim = PairingClaimRequest::new([0x22; 32], [9; 16]);
        assert_eq!(
            service.claim_at(&claim.encode_binary(), ticket.expose(), now, Instant::now()),
            Err(RelayPairingServiceError::AuthenticationFailed)
        );
    }

    #[test]
    fn publish_clamps_ttl_and_records_the_verified_device() {
        let device = [0x11; 32];
        let (service, ticket, now, log) = service_fixture(device);
        let request =
            PairingPublishRequest::new(device, [7; 16], MAX_PAIRING_CODE_TTL_SECS, vec![1, 2, 3])
                .expect("request");
        service
            .publish_at(
                &request.encode_binary(),
                ticket.expose(),
                now,
                Instant::now(),
            )
            .expect("publish");
        let puts = log.lock().expect("puts lock").clone();
        assert_eq!(puts.len(), 1);
        let ((owner, code_id), seen_now, expires) = puts[0];
        assert_eq!(owner, device);
        assert_eq!(code_id, [7; 16]);
        assert_eq!(seen_now, now);
        assert_eq!(expires, now + u64::from(MAX_PAIRING_CODE_TTL_SECS));
    }
}
