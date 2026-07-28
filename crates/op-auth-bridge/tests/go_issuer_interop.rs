//! Frozen cross-language regression vector, not a live dependency on the
//! private issuer repository. It catches verifier drift against one known Go
//! output; the Go repository must independently keep its producer-side edge
//! and resource-bound tests.

use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use op_auth_bridge::{
    CollabJwksCacheLimits, CollabJwksFetchError, CollabJwksFetchRequest, CollabJwksFetchResponse,
    CollabJwksFetcher, CollabTicketVerifier, CollabVerifierConfig,
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

const GO_ISSUER_FIXTURE: &str = include_str!("fixtures/zseven-sso-go-v1.json");
const TEST_ISSUER: &str = "https://sso.example.invalid";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoIssuerFixture {
    description: String,
    security_notice: String,
    profile_version: u32,
    now_unix_seconds: u64,
    expected_dh_pub_x25519: String,
    ticket_segments: [String; 3],
    jwks_json: String,
    vector_sha256: String,
    vector_sha256_input: String,
}

struct FrozenGoJwks(Vec<u8>);

impl CollabJwksFetcher for FrozenGoJwks {
    fn fetch(
        &self,
        request: CollabJwksFetchRequest<'_>,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        assert_eq!(
            request.endpoint,
            "https://sso.example.invalid/api/v1/collab/jwks"
        );
        assert!(self.0.len() <= request.maximum_body_bytes);
        Ok(CollabJwksFetchResponse::Modified {
            body: self.0.clone(),
            etag: Some("\"frozen-go-v1\"".to_owned()),
            // zseven-sso uses this response policy for its bounded
            // last-known-good keyset during a signer outage.
            max_age_seconds: 0,
        })
    }
}

#[test]
fn frozen_go_issuer_v1_output_verifies_without_private_repo_dependencies() {
    let fixture: GoIssuerFixture = serde_json::from_str(GO_ISSUER_FIXTURE).unwrap();
    assert!(!fixture.description.is_empty());
    assert!(fixture.security_notice.contains("no device token"));
    assert_eq!(fixture.profile_version, 1);
    assert_eq!(
        fixture.vector_sha256_input,
        "SHA-256(join(ticket_segments, '.') || 0x00 || jwks_json)"
    );
    let ticket = fixture.ticket_segments.join(".");
    let mut vector_digest = Sha256::new();
    vector_digest.update(ticket.as_bytes());
    vector_digest.update([0]);
    vector_digest.update(fixture.jwks_json.as_bytes());
    assert_eq!(
        format!("{:x}", vector_digest.finalize()),
        fixture.vector_sha256
    );

    let decoded_binding = URL_SAFE_NO_PAD
        .decode(&fixture.expected_dh_pub_x25519)
        .unwrap();
    assert_eq!(
        URL_SAFE_NO_PAD.encode(&decoded_binding),
        fixture.expected_dh_pub_x25519
    );
    let binding: [u8; 32] = decoded_binding.try_into().unwrap();

    let verifier = CollabTicketVerifier::new(
        CollabVerifierConfig::for_sso_origin(TEST_ISSUER).unwrap(),
        FrozenGoJwks(fixture.jwks_json.into_bytes()),
        CollabJwksCacheLimits::default(),
    )
    .unwrap();
    let cache_now = Instant::now();
    let verified = verifier
        .verify_at(
            ticket.as_bytes(),
            &binding,
            fixture.now_unix_seconds,
            cache_now,
        )
        .unwrap();

    assert_eq!(verified.issuer(), TEST_ISSUER);
    assert_eq!(verified.subject(), "01010101-0101-4101-8101-010101010101");
    assert_eq!(verified.device_id(), "02020202-0202-4202-8202-020202020202");
    assert_eq!(verified.dh_pub_x25519(), &binding);
    assert_eq!(verified.issued_at_unix_seconds(), 1_800_000_000);
    assert_eq!(verified.not_before_unix_seconds(), 1_800_000_000);
    assert_eq!(verified.expires_at_unix_seconds(), 1_800_000_900);
    assert_eq!(
        verified.ticket_id(),
        "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8"
    );
    assert_eq!(verified.display_name(), Some("Interop Test User"));
    assert_eq!(
        verified.avatar_url(),
        Some("https://cdn.example.test/avatar.png?size=128")
    );

    // A successful max-age=0 refresh may reuse this known key only during
    // the bounded one-second backoff; this locks the SSO LKG interoperability.
    verifier
        .verify_at(
            ticket.as_bytes(),
            &binding,
            fixture.now_unix_seconds,
            cache_now + Duration::from_millis(500),
        )
        .unwrap();
}
