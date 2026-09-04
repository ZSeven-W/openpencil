use std::num::NonZeroU64;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use op_collab_relay_protocol::{
    CallerDeviceDhPublic, ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic,
    RelayAuthExtensionV1, RelayClientHello, RelayHelloAuthMode, RelayInviteV1, RelayLocatorV1,
    RelayLocatorVerifier, RelayProtocolError, RelayRegion, RelayRejectCode, RelayRole,
    RelayServerStatus, RouteCapability, RouteId, UnsignedRelayLocatorV1, VerifiedRelayRoute,
    LOCATOR_CANONICAL_SIGNING_BYTES, LOCATOR_PREFIX, MAX_EXPECTED_DISCOVERY_ID_BYTES,
    MAX_INVITE_CHARS, MAX_LOCATOR_KEY_ID_BYTES, MAX_PAIRING_LIFETIME_SECS,
    MAX_POSSESSION_PROOF_BYTES, RELAY_CLIENT_HELLO_BYTES, RELAY_INVITE_BINARY_BYTES,
    RELAY_INVITE_PREFIX, RELAY_LOCATOR_BINARY_BYTES, RELAY_SERVER_STATUS_BYTES,
};

const NOW: u64 = 1_000_100;
const NOT_BEFORE: u64 = 1_000_000;
const EXPIRES: u64 = 1_000_600;

struct AcceptVerifier;

impl RelayLocatorVerifier for AcceptVerifier {
    fn verify(
        &self,
        key_id: &LocatorKeyId,
        canonical_signing_bytes: &[u8],
        signature: &[u8; 64],
    ) -> bool {
        key_id.as_str() == "relay-key-2026"
            && canonical_signing_bytes.len() == LOCATOR_CANONICAL_SIGNING_BYTES
            && signature == &[0x55; 64]
    }
}

struct RejectVerifier;

impl RelayLocatorVerifier for RejectVerifier {
    fn verify(
        &self,
        _key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        false
    }
}

fn unsigned(region: RelayRegion) -> UnsignedRelayLocatorV1 {
    UnsignedRelayLocatorV1::new(
        region,
        RouteId::new([0x11; 16]).unwrap(),
        NonZeroU64::new(7).unwrap(),
        OwnerNoiseStatic::new([0x22; 32]).unwrap(),
        ExpectedDiscoveryId::new("discover-owner-a").unwrap(),
        NOT_BEFORE,
        EXPIRES,
        LocatorKeyId::new("relay-key-2026").unwrap(),
    )
    .unwrap()
}

fn locator(region: RelayRegion) -> RelayLocatorV1 {
    unsigned(region).attach_signature(LocatorSignature::new([0x55; 64]).unwrap())
}

fn route(region: RelayRegion) -> VerifiedRelayRoute {
    let verified = locator(region).verify(&AcceptVerifier, NOW).unwrap();
    VerifiedRelayRoute::new(verified, RouteCapability::new([0x33; 32]).unwrap())
}

fn hello(region: RelayRegion) -> RelayClientHello {
    RelayClientHello::new(
        RelayRole::Guest,
        &route(region),
        RelayAuthExtensionV1::new(
            CallerDeviceDhPublic::new([0x44; 32]).unwrap(),
            Some(vec![0x66; 48]),
        )
        .unwrap(),
    )
}

fn decode_prefixed(prefix: &str, value: &str) -> Vec<u8> {
    URL_SAFE_NO_PAD
        .decode(value.strip_prefix(prefix).unwrap())
        .unwrap()
}

fn encode_prefixed(prefix: &str, raw: &[u8]) -> String {
    format!("{prefix}{}", URL_SAFE_NO_PAD.encode(raw))
}

#[test]
fn locator_is_fixed_canonical_and_round_trips_for_all_regions() {
    for region in [RelayRegion::Cn, RelayRegion::Global] {
        let locator = locator(region);
        let encoded = locator.encode();
        assert!(encoded.starts_with(LOCATOR_PREFIX));
        assert!(!encoded.contains('='));
        assert!(encoded.len() <= MAX_INVITE_CHARS);
        assert_eq!(
            decode_prefixed(LOCATOR_PREFIX, &encoded).len(),
            RELAY_LOCATOR_BINARY_BYTES
        );
        assert_eq!(RelayLocatorV1::decode(&encoded).unwrap(), locator);
        let canonical = locator.canonical_signing_bytes();
        assert_eq!(canonical.len(), LOCATOR_CANONICAL_SIGNING_BYTES);
        assert_eq!(canonical, unsigned(region).canonical_signing_bytes());
    }
}

#[test]
fn locator_verification_is_explicit_and_fail_closed() {
    let locator = locator(RelayRegion::Cn);
    assert!(matches!(
        locator.verify(&RejectVerifier, NOW),
        Err(RelayProtocolError::SignatureVerificationFailed)
    ));
    let verified = locator.verify(&AcceptVerifier, NOW).unwrap();
    assert_eq!(verified.claims().home_region(), RelayRegion::Cn);
}

#[test]
fn locator_server_time_window_is_bounded() {
    let locator = locator(RelayRegion::Global);
    assert!(matches!(
        locator.verify(&AcceptVerifier, NOT_BEFORE - 1),
        Err(RelayProtocolError::NotYetValid)
    ));
    assert!(matches!(
        locator.verify(&AcceptVerifier, EXPIRES),
        Err(RelayProtocolError::Expired)
    ));

    let too_far = UnsignedRelayLocatorV1::new(
        RelayRegion::Global,
        RouteId::new([1; 16]).unwrap(),
        NonZeroU64::new(1).unwrap(),
        OwnerNoiseStatic::new([2; 32]).unwrap(),
        ExpectedDiscoveryId::new("discover").unwrap(),
        NOW,
        NOW + MAX_PAIRING_LIFETIME_SECS + 1,
        LocatorKeyId::new("key").unwrap(),
    )
    .unwrap()
    .attach_signature(LocatorSignature::new([3; 64]).unwrap());
    assert!(matches!(
        too_far.verify(&RejectVerifier, NOW),
        Err(RelayProtocolError::ExpiryTooFarFuture)
    ));

    let stale_long_window = UnsignedRelayLocatorV1::new(
        RelayRegion::Global,
        RouteId::new([1; 16]).unwrap(),
        NonZeroU64::new(1).unwrap(),
        OwnerNoiseStatic::new([2; 32]).unwrap(),
        ExpectedDiscoveryId::new("discover").unwrap(),
        NOW - MAX_PAIRING_LIFETIME_SECS,
        NOW + 1,
        LocatorKeyId::new("key").unwrap(),
    )
    .unwrap()
    .attach_signature(LocatorSignature::new([3; 64]).unwrap());
    assert!(matches!(
        stale_long_window.verify(&RejectVerifier, NOW),
        Err(RelayProtocolError::ValidityWindowTooLong)
    ));
}

#[test]
fn locator_exact_decode_rejects_truncation_and_trailing_data() {
    let encoded = locator(RelayRegion::Cn).encode();
    let raw = decode_prefixed(LOCATOR_PREFIX, &encoded);
    assert_eq!(
        RelayLocatorV1::decode(LOCATOR_PREFIX).unwrap_err(),
        RelayProtocolError::InvalidLocatorEncoding
    );
    for length in 1..raw.len() {
        let truncated = encode_prefixed(LOCATOR_PREFIX, &raw[..length]);
        assert!(matches!(
            RelayLocatorV1::decode(&truncated),
            Err(RelayProtocolError::Truncated { .. })
        ));
    }
    let mut trailing = raw;
    trailing.push(0);
    assert!(matches!(
        RelayLocatorV1::decode(&encode_prefixed(LOCATOR_PREFIX, &trailing)),
        Err(RelayProtocolError::TrailingBytes { .. })
    ));
}

#[test]
fn locator_rejects_version_region_secrets_expiry_and_corruption() {
    let encoded = locator(RelayRegion::Cn).encode();
    let raw = decode_prefixed(LOCATOR_PREFIX, &encoded);
    let cases: Vec<(usize, Vec<u8>, RelayProtocolError)> = vec![
        (
            0,
            vec![2],
            RelayProtocolError::UnsupportedVersion {
                actual: 2,
                expected: 1,
            },
        ),
        (1, vec![99], RelayProtocolError::InvalidRegion(99)),
        (2, vec![0; 16], RelayProtocolError::ZeroRouteId),
        (18, vec![0; 8], RelayProtocolError::ZeroGeneration),
        (26, vec![0; 32], RelayProtocolError::ZeroOwnerNoiseStatic),
        (187, vec![0; 8], RelayProtocolError::ZeroNotBefore),
        (
            195,
            NOT_BEFORE.to_be_bytes().to_vec(),
            RelayProtocolError::InvalidExpiry,
        ),
    ];
    for (offset, replacement, expected) in cases {
        let mut mutated = raw.clone();
        mutated[offset..offset + replacement.len()].copy_from_slice(&replacement);
        assert_eq!(
            RelayLocatorV1::decode(&encode_prefixed(LOCATOR_PREFIX, &mutated)).unwrap_err(),
            expected
        );
    }

    let mut bad_checksum = raw;
    bad_checksum[300] ^= 1;
    assert_eq!(
        RelayLocatorV1::decode(&encode_prefixed(LOCATOR_PREFIX, &bad_checksum)).unwrap_err(),
        RelayProtocolError::ChecksumMismatch
    );
}

#[test]
fn locator_rejects_noncanonical_text_and_padded_fields() {
    assert_eq!(
        RelayLocatorV1::decode("opc1_A").unwrap_err(),
        RelayProtocolError::InvalidLocatorPrefix
    );
    assert_eq!(
        RelayLocatorV1::decode("opcl1_A=").unwrap_err(),
        RelayProtocolError::InvalidLocatorEncoding
    );
    assert!(matches!(
        RelayLocatorV1::decode(&format!("opcl1_{}", "A".repeat(MAX_INVITE_CHARS))),
        Err(RelayProtocolError::LocatorTooLong { .. })
    ));

    let mut raw = decode_prefixed(LOCATOR_PREFIX, &locator(RelayRegion::Global).encode());
    raw[59 + "discover-owner-a".len()] = 1;
    assert_eq!(
        RelayLocatorV1::decode(&encode_prefixed(LOCATOR_PREFIX, &raw)).unwrap_err(),
        RelayProtocolError::NonZeroReserved
    );
}

#[test]
fn ascii_fields_are_nonempty_printable_and_bounded() {
    assert!(matches!(
        ExpectedDiscoveryId::new(""),
        Err(RelayProtocolError::InvalidAsciiField { .. })
    ));
    assert!(matches!(
        ExpectedDiscoveryId::new("bad id"),
        Err(RelayProtocolError::InvalidAsciiField { .. })
    ));
    assert!(matches!(
        ExpectedDiscoveryId::new("x".repeat(MAX_EXPECTED_DISCOVERY_ID_BYTES + 1)),
        Err(RelayProtocolError::AsciiFieldTooLong { .. })
    ));
    assert!(matches!(
        LocatorKeyId::new("x".repeat(MAX_LOCATOR_KEY_ID_BYTES + 1)),
        Err(RelayProtocolError::AsciiFieldTooLong { .. })
    ));
}

#[test]
fn single_string_invite_round_trips_and_verifies() {
    let route = route(RelayRegion::Cn);
    let invite = RelayInviteV1::new(&route);
    let fragment = invite.to_fragment();
    assert!(fragment.starts_with(RELAY_INVITE_PREFIX));
    assert!(!fragment.contains('='));
    assert!(fragment.len() <= MAX_INVITE_CHARS);
    assert_eq!(
        decode_prefixed(RELAY_INVITE_PREFIX, &fragment).len(),
        RELAY_INVITE_BINARY_BYTES
    );
    let decoded = RelayInviteV1::from_fragment(&fragment).unwrap();
    assert_eq!(decoded, invite);
    assert_eq!(
        decoded
            .verify(&AcceptVerifier, NOW)
            .unwrap()
            .route_map_key(),
        route.route_map_key()
    );
}

#[test]
fn invite_exact_decode_rejects_truncation_trailing_and_zero_capability() {
    let fragment = RelayInviteV1::new(&route(RelayRegion::Global)).to_fragment();
    let raw = decode_prefixed(RELAY_INVITE_PREFIX, &fragment);
    assert_eq!(
        RelayInviteV1::from_fragment(RELAY_INVITE_PREFIX).unwrap_err(),
        RelayProtocolError::InvalidLocatorEncoding
    );
    for length in 1..raw.len() {
        assert!(matches!(
            RelayInviteV1::from_fragment(&encode_prefixed(RELAY_INVITE_PREFIX, &raw[..length])),
            Err(RelayProtocolError::Truncated { .. })
        ));
    }
    let mut trailing = raw.clone();
    trailing.push(0);
    assert!(matches!(
        RelayInviteV1::from_fragment(&encode_prefixed(RELAY_INVITE_PREFIX, &trailing)),
        Err(RelayProtocolError::TrailingBytes { .. })
    ));
    let mut zero_capability = raw;
    zero_capability[337..369].fill(0);
    assert_eq!(
        RelayInviteV1::from_fragment(&encode_prefixed(RELAY_INVITE_PREFIX, &zero_capability))
            .unwrap_err(),
        RelayProtocolError::ZeroRouteCapability
    );
}

#[test]
fn route_map_key_is_domain_separated_and_capability_is_nonzero() {
    assert_eq!(
        RouteCapability::new([0; 32]).unwrap_err(),
        RelayProtocolError::ZeroRouteCapability
    );
    let first = route(RelayRegion::Cn);
    let first_key = first.route_map_key();
    assert_ne!(first_key.as_bytes(), &[0x33; 32]);

    let second = VerifiedRelayRoute::new(
        first.locator().clone(),
        RouteCapability::new([0x34; 32]).unwrap(),
    );
    assert_ne!(first_key, second.route_map_key());
}

#[test]
fn hello_is_fixed_exact_and_locator_verification_remains_explicit() {
    let hello = hello(RelayRegion::Global);
    assert_eq!(
        hello.auth_mode(),
        RelayHelloAuthMode::SignedLocatorAndBearerTicketV1
    );
    assert_eq!(hello.expires_at_unix(), EXPIRES);
    let encoded = hello.encode();
    assert_eq!(encoded.len(), RELAY_CLIENT_HELLO_BYTES);
    let decoded = RelayClientHello::decode(&encoded).unwrap();
    assert_eq!(decoded, hello);
    assert!(matches!(
        decoded.verify_locator(&RejectVerifier, NOW),
        Err(RelayProtocolError::SignatureVerificationFailed)
    ));
    let verified = decoded.verify_locator(&AcceptVerifier, NOW).unwrap();
    assert_eq!(verified.role(), RelayRole::Guest);
    assert_eq!(
        verified
            .auth_extension()
            .caller_device_dh_pub_x25519()
            .as_bytes(),
        &[0x44; 32]
    );
    assert_eq!(
        verified.auth_extension().possession_proof(),
        Some(&[0x66; 48][..])
    );
}

#[test]
fn hello_rejects_all_truncations_and_trailing_bytes() {
    let encoded = hello(RelayRegion::Cn).encode();
    for length in 0..encoded.len() {
        assert!(matches!(
            RelayClientHello::decode(&encoded[..length]),
            Err(RelayProtocolError::Truncated { .. })
        ));
    }
    let mut trailing = encoded.to_vec();
    trailing.push(0);
    assert!(matches!(
        RelayClientHello::decode(&trailing),
        Err(RelayProtocolError::TrailingBytes { .. })
    ));
}

#[test]
fn hello_rejects_invalid_header_auth_and_secrets() {
    let encoded = hello(RelayRegion::Cn).encode();
    for (offset, value, expected) in [
        (
            0,
            2,
            RelayProtocolError::UnsupportedVersion {
                actual: 2,
                expected: 1,
            },
        ),
        (1, 9, RelayProtocolError::InvalidRole(9)),
        (2, 0, RelayProtocolError::InvalidAuthMode(0)),
        (
            3,
            2,
            RelayProtocolError::UnsupportedAuthExtension {
                actual: 2,
                expected: 1,
            },
        ),
    ] {
        let mut mutated = encoded;
        mutated[offset] = value;
        assert_eq!(RelayClientHello::decode(&mutated).unwrap_err(), expected);
    }

    let mut zero_caller = encoded;
    zero_caller[4..36].fill(0);
    assert_eq!(
        RelayClientHello::decode(&zero_caller).unwrap_err(),
        RelayProtocolError::ZeroCallerDeviceDhPublic
    );
    let mut zero_capability = encoded;
    zero_capability[36..68].fill(0);
    assert_eq!(
        RelayClientHello::decode(&zero_capability).unwrap_err(),
        RelayProtocolError::ZeroRouteCapability
    );
    let mut proof_too_long = encoded;
    proof_too_long[68] = (MAX_POSSESSION_PROOF_BYTES + 1) as u8;
    assert!(matches!(
        RelayClientHello::decode(&proof_too_long),
        Err(RelayProtocolError::PossessionProofTooLong { .. })
    ));
    let mut dirty_padding = encoded;
    dirty_padding[69 + 48] = 1;
    assert_eq!(
        RelayClientHello::decode(&dirty_padding).unwrap_err(),
        RelayProtocolError::NonZeroReserved
    );
}

#[test]
fn server_status_round_trips_and_is_exact() {
    let all = [
        RelayServerStatus::Ready,
        RelayServerStatus::Paired,
        RelayServerStatus::Rejected(RelayRejectCode::MalformedHello),
        RelayServerStatus::Rejected(RelayRejectCode::UnsupportedVersion),
        RelayServerStatus::Rejected(RelayRejectCode::AuthenticationRequired),
        RelayServerStatus::Rejected(RelayRejectCode::AuthenticationFailed),
        RelayServerStatus::Rejected(RelayRejectCode::LocatorNotYetValid),
        RelayServerStatus::Rejected(RelayRejectCode::LocatorExpired),
        RelayServerStatus::Rejected(RelayRejectCode::ExpiryTooFarFuture),
        RelayServerStatus::Rejected(RelayRejectCode::UnknownRoute),
        RelayServerStatus::Rejected(RelayRejectCode::RoleConflict),
        RelayServerStatus::Rejected(RelayRejectCode::Capacity),
        RelayServerStatus::Rejected(RelayRejectCode::RateLimited),
        RelayServerStatus::Rejected(RelayRejectCode::PairingTimeout),
        RelayServerStatus::Rejected(RelayRejectCode::RelayUnavailable),
        RelayServerStatus::Rejected(RelayRejectCode::Internal),
    ];
    for status in all {
        let encoded = status.encode();
        assert_eq!(encoded.len(), RELAY_SERVER_STATUS_BYTES);
        assert_eq!(RelayServerStatus::decode(&encoded).unwrap(), status);
    }
    assert!(matches!(
        RelayServerStatus::decode(&[1, 1]),
        Err(RelayProtocolError::Truncated { .. })
    ));
    assert!(matches!(
        RelayServerStatus::decode(&[1, 1, 0, 0]),
        Err(RelayProtocolError::TrailingBytes { .. })
    ));
    assert_eq!(
        RelayServerStatus::decode(&[2, 1, 0]).unwrap_err(),
        RelayProtocolError::UnsupportedVersion {
            actual: 2,
            expected: 1
        }
    );
    assert_eq!(
        RelayServerStatus::decode(&[1, 9, 0]).unwrap_err(),
        RelayProtocolError::InvalidServerStatus(9)
    );
    assert_eq!(
        RelayServerStatus::decode(&[1, 1, 1]).unwrap_err(),
        RelayProtocolError::InvalidStatusDetail
    );
    assert_eq!(
        RelayServerStatus::decode(&[1, 3, 0]).unwrap_err(),
        RelayProtocolError::InvalidStatusDetail
    );
    assert_eq!(
        RelayServerStatus::decode(&[1, 3, 99]).unwrap_err(),
        RelayProtocolError::InvalidRejectCode(99)
    );
}

#[test]
fn debug_redacts_every_route_and_auth_credential() {
    let route = route(RelayRegion::Cn);
    let invite = RelayInviteV1::new(&route);
    let hello = hello(RelayRegion::Cn);
    let rendered = format!(
        "{:?} {:?} {:?} {:?} {:?} {:?} {:?}",
        route,
        invite,
        hello,
        RouteCapability::new([0x33; 32]).unwrap(),
        OwnerNoiseStatic::new([0x22; 32]).unwrap(),
        CallerDeviceDhPublic::new([0x44; 32]).unwrap(),
        ExpectedDiscoveryId::new("discover-owner-a").unwrap(),
    );
    assert!(rendered.contains("[REDACTED]"));
    assert!(!rendered.contains("discover-owner-a"));
    assert!(!rendered.contains("relay-key-2026"));
    assert!(!rendered.contains(&"51, ".repeat(8)));
    assert!(!rendered.contains(&"68, ".repeat(8)));
    assert!(!rendered.contains(&"102, ".repeat(8)));
}

#[test]
fn auth_extension_bounds_proof_and_supports_absence() {
    let caller = CallerDeviceDhPublic::new([1; 32]).unwrap();
    let none = RelayAuthExtensionV1::without_possession_proof(caller);
    assert_eq!(none.possession_proof(), None);
    assert!(matches!(
        RelayAuthExtensionV1::new(caller, Some(vec![0; MAX_POSSESSION_PROOF_BYTES + 1])),
        Err(RelayProtocolError::PossessionProofTooLong { .. })
    ));
}

#[cfg(feature = "random")]
#[test]
fn random_capability_and_route_id_are_nonzero() {
    assert_ne!(
        RouteCapability::generate().unwrap(),
        RouteCapability::new([0; 32]).unwrap_or_else(|_| RouteCapability::new([1; 32]).unwrap())
    );
    assert_ne!(RouteId::generate().unwrap().as_bytes(), &[0; 16]);
}

#[test]
fn reject_close_reasons_round_trip_and_stay_inside_a_close_frame() {
    use op_collab_relay_protocol::{
        RelayRejectCode, RELAY_REJECT_CLOSE_PREFIX, RELAY_REJECT_CODES,
    };

    let mut seen = std::collections::HashSet::new();
    for code in RELAY_REJECT_CODES {
        let reason = code.close_reason();
        assert!(reason.starts_with(RELAY_REJECT_CLOSE_PREFIX));
        assert_eq!(
            reason.strip_prefix(RELAY_REJECT_CLOSE_PREFIX),
            Some(code.label())
        );
        // A WebSocket close payload is 125 bytes, two of which are the code.
        assert!(reason.len() <= 123);
        assert_eq!(RelayRejectCode::from_close_reason(reason), Some(code));
        assert!(seen.insert(reason), "close reasons must be unique per code");
        // Every code is reachable from its wire byte, so the table cannot
        // silently omit one.
        assert_eq!(RelayRejectCode::try_from(code as u8).unwrap(), code);
    }
    assert_eq!(seen.len(), RELAY_REJECT_CODES.len());

    for other in ["", "idle timeout", "relay-reject:", "relay-reject:unknown"] {
        assert_eq!(RelayRejectCode::from_close_reason(other), None);
    }
}

#[test]
fn the_waiting_advertisement_round_trips_and_rejects_everything_else() {
    use op_collab_relay_protocol::{
        RelayWaitingAdvertisementV1, MAX_ADVERTISED_WAITING_SECS, MAX_RELAY_WAITING_HEADER_BYTES,
    };

    for (window, renew) in [
        (60, false),
        (60, true),
        (MAX_ADVERTISED_WAITING_SECS, true),
        (1, false),
    ] {
        let advertisement = RelayWaitingAdvertisementV1::new(window, renew).unwrap();
        let header = advertisement.encode_header();
        assert!(header.len() <= MAX_RELAY_WAITING_HEADER_BYTES);
        assert_eq!(
            RelayWaitingAdvertisementV1::decode_header(&header).unwrap(),
            advertisement
        );
        assert_eq!(advertisement.window_secs(), window);
        assert_eq!(advertisement.renewable(), renew);
    }

    assert!(RelayWaitingAdvertisementV1::new(0, true).is_err());
    assert!(RelayWaitingAdvertisementV1::new(MAX_ADVERTISED_WAITING_SECS + 1, true).is_err());
    for malformed in [
        "",
        "window=60 renew=1",
        "oprw1 window=60",
        "oprw1 renew=1 window=60",
        "oprw1 window=60 renew=2",
        "oprw1 window=abc renew=1",
        "oprw1 window=60 renew=1 extra=1",
        "oprw2 window=60 renew=1",
        "oprw1 window=0 renew=1",
    ] {
        assert!(
            RelayWaitingAdvertisementV1::decode_header(malformed).is_err(),
            "{malformed:?} must not decode"
        );
    }
}

#[test]
fn a_derived_lane_budget_only_ever_narrows_towards_the_relay() {
    use std::time::Duration;

    use op_collab_relay_protocol::{
        RelayWaitingAdvertisementV1, MIN_DERIVED_OWNER_LANE_BUDGET_SECS,
        RELAY_WAITING_SAFETY_MARGIN_SECS,
    };

    let unrenewable_cap = Duration::from_secs(45);
    let renewable_cap = Duration::from_secs(300);

    // A fixed 60 s countdown leaves the client's own 45 s ceiling in charge:
    // 60 - 10 = 50, capped to 45.
    let fixed = RelayWaitingAdvertisementV1::new(60, false).unwrap();
    assert_eq!(
        fixed.derive_lane_budget(unrenewable_cap, renewable_cap),
        unrenewable_cap
    );

    // A tighter relay narrows the client below its own ceiling.
    let tight = RelayWaitingAdvertisementV1::new(30, false).unwrap();
    assert_eq!(
        tight.derive_lane_budget(unrenewable_cap, renewable_cap),
        Duration::from_secs(30 - RELAY_WAITING_SAFETY_MARGIN_SECS)
    );

    // A renewable lease unlocks the longer ceiling but never exceeds it.
    let lease = RelayWaitingAdvertisementV1::new(12 * 60 * 60, true).unwrap();
    assert_eq!(
        lease.derive_lane_budget(unrenewable_cap, renewable_cap),
        renewable_cap
    );

    // Without a lease the long window is still bounded by the short ceiling,
    // so a relay cannot talk a client into parking past its own recycle.
    let long_without_lease = RelayWaitingAdvertisementV1::new(12 * 60 * 60, false).unwrap();
    assert_eq!(
        long_without_lease.derive_lane_budget(unrenewable_cap, renewable_cap),
        unrenewable_cap
    );

    // A degenerate advertisement floors instead of collapsing to zero.
    for window in [1, RELAY_WAITING_SAFETY_MARGIN_SECS] {
        let degenerate = RelayWaitingAdvertisementV1::new(window, false).unwrap();
        assert_eq!(
            degenerate.derive_lane_budget(unrenewable_cap, renewable_cap),
            Duration::from_secs(MIN_DERIVED_OWNER_LANE_BUDGET_SECS)
        );
    }
}
