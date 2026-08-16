//! Round-trip, corruption, and redaction coverage for short pairing codes.

use std::num::NonZeroU64;

use op_collab_relay_protocol::{
    ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic, PairingCode,
    RelayInviteV1, RelayLocatorVerifier, RelayProtocolError, RelayRegion, RouteCapability, RouteId,
    SealedPairingInvite, UnsignedRelayLocatorV1, VerifiedRelayRoute, MAX_SEALED_INVITE_BYTES,
    MAX_SEALED_PAIRING_INVITE_V2_BYTES, PAIRING_CODE_CHARS, PAIRING_CODE_ID_BYTES,
    RELAY_PROTOCOL_VERSION, SEALED_INVITE_NONCE_BYTES, SEALED_INVITE_TAG_BYTES,
    SEALED_PAIRING_INVITE_VERSION,
};
use sha2::{Digest as _, Sha256};

const NOW: u64 = 1_754_000_000;
const NOT_BEFORE: u64 = NOW - 60;
const EXPIRES: u64 = NOW + 600;

struct AcceptVerifier;

impl RelayLocatorVerifier for AcceptVerifier {
    fn verify(
        &self,
        _key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        true
    }
}

fn invite() -> RelayInviteV1 {
    let unsigned = UnsignedRelayLocatorV1::new(
        RelayRegion::Global,
        RouteId::new([0x11; 16]).unwrap(),
        NonZeroU64::new(7).unwrap(),
        OwnerNoiseStatic::new([0x22; 32]).unwrap(),
        ExpectedDiscoveryId::new("discover-owner-a").unwrap(),
        NOT_BEFORE,
        EXPIRES,
        LocatorKeyId::new("relay-key-2026").unwrap(),
    )
    .unwrap();
    let locator = unsigned.attach_signature(LocatorSignature::new([0x55; 64]).unwrap());
    let verified = locator.verify(&AcceptVerifier, NOW).unwrap();
    RelayInviteV1::new(&VerifiedRelayRoute::new(
        verified,
        RouteCapability::new([0x33; 32]).unwrap(),
    ))
}

fn code() -> PairingCode {
    PairingCode::parse("2A2C4E6G8J").unwrap()
}

fn nonce() -> [u8; SEALED_INVITE_NONCE_BYTES] {
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
}

#[test]
fn pairing_code_parses_confusables_case_and_grouping() {
    let canonical = code();
    for variant in ["2a2c4e6g8j", "2A2C-4E6G-8J", " 2A2C 4E6G 8J "] {
        let parsed = PairingCode::parse(variant).unwrap();
        assert_eq!(parsed.expose_str(), canonical.expose_str(), "{variant}");
        assert!(PairingCode::looks_like(variant), "{variant}");
    }
    // Crockford confusables: I/L → 1, O → 0.
    assert_eq!(
        PairingCode::parse("1ICLEOGHJX").unwrap().expose_str(),
        "11C1E0GHJX"
    );

    for rejected in [
        "",
        "2A2C4E6G8",   // short
        "2A2C4E6G8J0", // long
        "2A2C4E6G8U",  // U is out of the alphabet
        "2A2C4E6G8*",  // symbol
        "opc1_abcdef", // invite fragment shape
        "192.168.1.8:43120",
    ] {
        assert!(
            PairingCode::parse(rejected).is_err(),
            "must reject {rejected:?}"
        );
        assert!(!PairingCode::looks_like(rejected), "{rejected}");
    }
}

#[test]
fn generated_codes_are_canonical_distinct_and_region_tagged() {
    let first = PairingCode::generate_for(RelayRegion::Global).unwrap();
    let second = PairingCode::generate_for(RelayRegion::Global).unwrap();
    assert_eq!(first.expose_str().len(), PAIRING_CODE_CHARS);
    assert!(PairingCode::looks_like(first.expose_str()));
    assert_eq!(first.region(), Some(RelayRegion::Global));
    assert_eq!(
        PairingCode::generate_for(RelayRegion::Cn).unwrap().region(),
        Some(RelayRegion::Cn)
    );
    assert_ne!(
        first.expose_str(),
        second.expose_str(),
        "two generated codes colliding is a broken RNG"
    );
    assert_eq!(first.code_id().len(), PAIRING_CODE_ID_BYTES);
    assert_ne!(first.code_id(), second.code_id());
}

#[test]
fn code_id_preserves_the_v0_8_4_blake3_known_answer() {
    assert_eq!(
        code().code_id(),
        [
            0x1e, 0x1d, 0x4d, 0x17, 0xc3, 0xd3, 0x8e, 0x53, 0xa1, 0xfd, 0xb7, 0xa3, 0x41, 0x4f,
            0xcc, 0xa2,
        ]
    );
}

#[test]
fn deterministic_seal_has_the_v2_rfc8439_layout() {
    assert_eq!(SEALED_INVITE_NONCE_BYTES, 12);
    assert_eq!(SEALED_INVITE_TAG_BYTES, 16);
    assert_eq!(MAX_SEALED_PAIRING_INVITE_V2_BYTES, 541);
    assert_eq!(MAX_SEALED_INVITE_BYTES, 569);
    assert_ne!(SEALED_PAIRING_INVITE_VERSION, RELAY_PROTOCOL_VERSION);
    let sealed = SealedPairingInvite::seal(&code(), &invite(), nonce()).unwrap();
    assert_eq!(sealed.as_bytes()[0], SEALED_PAIRING_INVITE_VERSION);
    assert_eq!(
        &sealed.as_bytes()[1..1 + SEALED_INVITE_NONCE_BYTES],
        &nonce()
    );
    assert_eq!(
        sealed.as_bytes().len(),
        1 + SEALED_INVITE_NONCE_BYTES + invite().to_fragment().len() + SEALED_INVITE_TAG_BYTES
    );
    // Cross-checked independently with Node's OpenSSL-backed HKDF-SHA256 and
    // ChaCha20-Poly1305 implementations, not generated from this Rust code.
    let sealed_digest: [u8; 32] = Sha256::digest(sealed.as_bytes()).into();
    assert_eq!(
        sealed_digest,
        [
            0xbc, 0x9a, 0x72, 0x13, 0xed, 0xac, 0x8b, 0x76, 0x01, 0xb2, 0x38, 0xe2, 0x53, 0x30,
            0xdc, 0xd2, 0x48, 0x56, 0x3f, 0x87, 0xb1, 0xa6, 0xee, 0x3d, 0x3c, 0xd8, 0xca, 0x07,
            0x96, 0x42, 0xf0, 0xdb,
        ]
    );
    assert_eq!(sealed.open(&code()).unwrap(), invite());
}

#[test]
fn seal_and_parser_reject_an_all_zero_nonce() {
    assert!(matches!(
        SealedPairingInvite::seal(&code(), &invite(), [0; SEALED_INVITE_NONCE_BYTES]),
        Err(RelayProtocolError::ZeroSealedInviteNonce)
    ));

    let mut raw = SealedPairingInvite::seal(&code(), &invite(), nonce())
        .unwrap()
        .as_bytes()
        .to_vec();
    raw[1..1 + SEALED_INVITE_NONCE_BYTES].fill(0);
    assert!(matches!(
        SealedPairingInvite::from_bytes(&raw),
        Err(RelayProtocolError::ZeroSealedInviteNonce)
    ));
}

#[test]
fn sealing_the_same_invite_twice_uses_fresh_nonces() {
    let first = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    let second = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    assert_ne!(
        first.as_bytes(),
        second.as_bytes(),
        "identical sealed bytes mean nonce reuse"
    );
    assert_ne!(
        &first.as_bytes()[1..1 + SEALED_INVITE_NONCE_BYTES],
        &second.as_bytes()[1..1 + SEALED_INVITE_NONCE_BYTES],
        "nonces must differ per seal"
    );
}

#[test]
fn sealed_invite_round_trips_only_with_the_right_code() {
    let sealed = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    assert!(sealed.as_bytes().len() <= MAX_SEALED_PAIRING_INVITE_V2_BYTES);

    let reopened = SealedPairingInvite::from_bytes(sealed.as_bytes())
        .unwrap()
        .open(&code())
        .unwrap();
    assert_eq!(reopened, invite());

    let wrong = PairingCode::parse("ZZZZZZZZZZ").unwrap();
    assert!(matches!(
        sealed.open(&wrong),
        Err(RelayProtocolError::InvalidPairingCode)
    ));
}

#[test]
fn sealed_invite_rejects_truncation_trailing_and_bit_flips() {
    let sealed = SealedPairingInvite::seal(&code(), &invite(), nonce()).unwrap();
    let raw = sealed.as_bytes().to_vec();

    let minimum = 1 + SEALED_INVITE_NONCE_BYTES + SEALED_INVITE_TAG_BYTES + 1;
    for length in [0, 1, minimum - 1] {
        assert!(SealedPairingInvite::from_bytes(&raw[..length]).is_err());
    }
    let mut oversized_v2 = raw.clone();
    oversized_v2.resize(MAX_SEALED_PAIRING_INVITE_V2_BYTES + 1, 0);
    assert!(matches!(
        SealedPairingInvite::from_bytes(&oversized_v2),
        Err(RelayProtocolError::InvalidSealedInvite)
    ));
    let mut oversized_transport = raw.clone();
    oversized_transport.resize(MAX_SEALED_INVITE_BYTES + 1, 0);
    assert!(SealedPairingInvite::from_bytes(&oversized_transport).is_err());

    let mut wrong_version = raw.clone();
    wrong_version[0] = 1;
    assert!(matches!(
        SealedPairingInvite::from_bytes(&wrong_version),
        Err(RelayProtocolError::UnsupportedSealedInviteVersion {
            actual: 1,
            expected: SEALED_PAIRING_INVITE_VERSION,
        })
    ));

    // Every byte of the nonce, ciphertext, and full tag is authenticated.
    for index in 1..raw.len() {
        let mut corrupted = raw.clone();
        corrupted[index] ^= 0x01;
        assert!(
            matches!(
                SealedPairingInvite::from_bytes(&corrupted)
                    .unwrap()
                    .open(&code()),
                Err(RelayProtocolError::InvalidPairingCode)
            ),
            "flip at {index} must not open"
        );
    }

    // Appending a byte within the public size ceiling changes the inferred
    // ciphertext/tag boundary and therefore must also fail authentication.
    let mut trailing = raw.clone();
    trailing.push(0);
    assert!(trailing.len() <= MAX_SEALED_INVITE_BYTES);
    assert!(matches!(
        SealedPairingInvite::from_bytes(&trailing)
            .unwrap()
            .open(&code()),
        Err(RelayProtocolError::InvalidPairingCode)
    ));
}

#[test]
fn published_v0_8_4_sealed_v1_is_rejected_without_fallback() {
    // Keeping the v0.8.4 code id lets both versions address the same opaque
    // slot, but does not make their cipher envelopes interoperable. Pairing
    // therefore requires a same-version peer or a product-exposed manual path.
    let mut legacy = vec![0_u8; 1 + 24 + invite().to_fragment().len() + 32];
    legacy[0] = 1;
    assert_eq!(legacy.len(), 560);
    assert!(legacy.len() <= MAX_SEALED_INVITE_BYTES);
    assert!(matches!(
        SealedPairingInvite::from_bytes(&legacy),
        Err(RelayProtocolError::UnsupportedSealedInviteVersion {
            actual: 1,
            expected: SEALED_PAIRING_INVITE_VERSION,
        })
    ));
}

#[test]
fn code_id_does_not_reveal_the_sealing_key_derivation() {
    // Distinct contexts: the id and the subkeys must differ even over the
    // same code bytes, so the server-side handle cannot double as key
    // material.
    let sealed = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    let id = code().code_id();
    assert!(
        !sealed
            .as_bytes()
            .windows(id.len())
            .any(|window| window == id),
        "code id must not appear inside the sealed blob"
    );
}

#[test]
fn pairing_debug_is_redacted() {
    let debug_code = format!("{:?}", code());
    assert_eq!(debug_code, "PairingCode([REDACTED])");
    let sealed = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    assert_eq!(format!("{sealed:?}"), "SealedPairingInvite([REDACTED])");
}

#[test]
fn dispatch_shape_requires_a_region_tag() {
    // Parses as a code shape, but the first char names no region — join
    // dispatch must not route it to the pairing branch. This is what keeps
    // 10-char LAN hostnames out of the claim path.
    for hostname in ["renderfarm", "A2C4E6G8J0", "myhostname"] {
        assert!(PairingCode::parse(hostname).is_ok(), "{hostname}");
        assert!(!PairingCode::looks_like(hostname), "{hostname}");
        assert!(PairingCode::parse(hostname).unwrap().region().is_none());
    }
    assert_eq!(
        PairingCode::parse("1A2C4E6G8J").unwrap().region(),
        Some(RelayRegion::Cn)
    );
    assert_eq!(code().region(), Some(RelayRegion::Global));
}
