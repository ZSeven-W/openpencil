//! Compact-JWS parser and typed collaboration-ticket verifier.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};

use crate::{
    collab_claims::{
        valid_key_id, validate_claims, CollabJwsHeader, CollabKeySource, UnverifiedCollabClaims,
        COLLAB_JWS_ALGORITHM, COLLAB_JWS_TYPE,
    },
    CollabJwksCache, CollabJwksCacheLimits, CollabJwksError, CollabJwksFetchError,
    CollabJwksFetcher, CollabVerifierConfig, CollabVerifierConfigError, CollabVerifyError,
    VerifiedCollabClaims, MAX_COLLAB_TICKET_BYTES,
};

pub const MAX_COLLAB_JWS_HEADER_BYTES: usize = 1_024;
pub const MAX_COLLAB_JWS_CLAIMS_BYTES: usize = 8 * 1_024;
const MAX_COLLAB_JWS_SIGNATURE_BYTES: usize = 64;

/// Signature, claims, timing, and channel-binding verifier.
pub struct CollabTicketVerifier<F> {
    config: CollabVerifierConfig,
    cache: CollabJwksCache<F>,
}

impl<F: CollabJwksFetcher> CollabTicketVerifier<F> {
    pub fn production(fetcher: F) -> Result<Self, CollabVerifierConfigError> {
        Self::new(
            CollabVerifierConfig::production(),
            fetcher,
            CollabJwksCacheLimits::default(),
        )
    }

    pub fn new(
        config: CollabVerifierConfig,
        fetcher: F,
        cache_limits: CollabJwksCacheLimits,
    ) -> Result<Self, CollabVerifierConfigError> {
        let cache = match config.key_source() {
            CollabKeySource::SignedPolicy => CollabJwksCache::new_signed_policy(
                config.keyset_endpoint(),
                config.issuer(),
                fetcher,
                cache_limits,
            )?,
            CollabKeySource::LegacyJwks => {
                CollabJwksCache::new(config.keyset_endpoint(), fetcher, cache_limits)?
            }
        };
        Ok(Self { config, cache })
    }

    pub fn config(&self) -> &CollabVerifierConfig {
        &self.config
    }

    /// Verify with explicit wall and monotonic clocks for deterministic hosts.
    pub fn verify_at(
        &self,
        ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
        now_unix_seconds: u64,
        cache_now: Instant,
    ) -> Result<VerifiedCollabClaims, CollabVerifyError> {
        self.verify_at_inner(
            ticket,
            expected_dh_pub_x25519,
            now_unix_seconds,
            cache_now,
            &never_cancelled,
            false,
        )
    }

    /// Verify with cancellation propagated through any in-flight fetch.
    pub fn verify_at_cancellable(
        &self,
        ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
        now_unix_seconds: u64,
        cache_now: Instant,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<VerifiedCollabClaims, CollabVerifyError> {
        self.verify_at_inner(
            ticket,
            expected_dh_pub_x25519,
            now_unix_seconds,
            cache_now,
            cancelled,
            true,
        )
    }

    fn verify_at_inner(
        &self,
        ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
        now_unix_seconds: u64,
        cache_now: Instant,
        cancelled: &dyn Fn() -> bool,
        cancellation_enabled: bool,
    ) -> Result<VerifiedCollabClaims, CollabVerifyError> {
        ensure_not_cancelled(cancelled)?;
        if ticket.is_empty() || ticket.len() > MAX_COLLAB_TICKET_BYTES {
            return Err(CollabVerifyError::InvalidTicketSize {
                maximum: MAX_COLLAB_TICKET_BYTES,
            });
        }
        if expected_dh_pub_x25519.iter().all(|byte| *byte == 0) {
            return Err(CollabVerifyError::InvalidChannelBinding);
        }

        let parsed = parse_compact_jws(ticket)?;
        ensure_not_cancelled(cancelled)?;
        let key_bytes = match (self.config.key_source(), cancellation_enabled) {
            (CollabKeySource::SignedPolicy, true) => {
                self.cache.policy_verification_key_cancellable(
                    &parsed.header.kid,
                    cache_now,
                    now_unix_seconds,
                    cancelled,
                )
            }
            (CollabKeySource::SignedPolicy, false) => {
                self.cache
                    .policy_verification_key(&parsed.header.kid, cache_now, now_unix_seconds)
            }
            (CollabKeySource::LegacyJwks, true) => {
                self.cache
                    .verification_key_cancellable(&parsed.header.kid, cache_now, cancelled)
            }
            (CollabKeySource::LegacyJwks, false) => {
                self.cache.verification_key(&parsed.header.kid, cache_now)
            }
        }
        .map_err(map_jwks_error)?;
        ensure_not_cancelled(cancelled)?;
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| CollabVerifyError::InvalidSignature)?;
        let signature = Signature::from_bytes(&parsed.signature);
        verifying_key
            .verify_strict(parsed.signing_input, &signature)
            .map_err(|_| CollabVerifyError::InvalidSignature)?;
        ensure_not_cancelled(cancelled)?;

        let claims_bytes =
            decode_segment(parsed.claims_segment, "claims", MAX_COLLAB_JWS_CLAIMS_BYTES)?;
        let claims: UnverifiedCollabClaims = serde_json::from_slice(&claims_bytes)
            .map_err(|_| CollabVerifyError::MalformedJson { part: "claims" })?;
        let decoded_dh_pub_x25519 = decode_channel_binding(&claims.dh_pub_x25519)?;
        ensure_not_cancelled(cancelled)?;
        let verified = validate_claims(
            claims,
            &self.config,
            expected_dh_pub_x25519,
            decoded_dh_pub_x25519,
            now_unix_seconds,
        )?;
        ensure_not_cancelled(cancelled)?;
        Ok(verified)
    }

    /// Verify using the process wall and monotonic clocks.
    pub fn verify(
        &self,
        ticket: &[u8],
        expected_dh_pub_x25519: &[u8; 32],
    ) -> Result<VerifiedCollabClaims, CollabVerifyError> {
        let now_unix_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CollabVerifyError::InvalidTimestamps)?
            .as_secs();
        self.verify_at(
            ticket,
            expected_dh_pub_x25519,
            now_unix_seconds,
            Instant::now(),
        )
    }

    pub fn cached_key_count(&self) -> Result<usize, crate::CollabJwksError> {
        self.cache.cached_key_count()
    }
}

fn never_cancelled() -> bool {
    false
}

fn ensure_not_cancelled(cancelled: &dyn Fn() -> bool) -> Result<(), CollabVerifyError> {
    if cancelled() {
        Err(CollabVerifyError::Cancelled)
    } else {
        Ok(())
    }
}

fn map_jwks_error(error: CollabJwksError) -> CollabVerifyError {
    if matches!(
        error,
        CollabJwksError::Fetch(CollabJwksFetchError::Cancelled)
    ) {
        CollabVerifyError::Cancelled
    } else {
        CollabVerifyError::Jwks(error)
    }
}

impl<F> std::fmt::Debug for CollabTicketVerifier<F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CollabTicketVerifier")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

struct ParsedCompactJws<'a> {
    header: CollabJwsHeader,
    claims_segment: &'a str,
    signature: [u8; 64],
    signing_input: &'a [u8],
}

fn parse_compact_jws(ticket: &[u8]) -> Result<ParsedCompactJws<'_>, CollabVerifyError> {
    let text = std::str::from_utf8(ticket).map_err(|_| CollabVerifyError::MalformedCompactJws)?;
    if !text.is_ascii() {
        return Err(CollabVerifyError::MalformedCompactJws);
    }
    let mut segments = text.split('.');
    let header_segment = segments
        .next()
        .filter(|value| !value.is_empty())
        .ok_or(CollabVerifyError::MalformedCompactJws)?;
    let claims_segment = segments
        .next()
        .filter(|value| !value.is_empty())
        .ok_or(CollabVerifyError::MalformedCompactJws)?;
    let signature_segment = segments
        .next()
        .filter(|value| !value.is_empty())
        .ok_or(CollabVerifyError::MalformedCompactJws)?;
    if segments.next().is_some() {
        return Err(CollabVerifyError::MalformedCompactJws);
    }

    let signing_input_length = header_segment
        .len()
        .checked_add(1)
        .and_then(|length| length.checked_add(claims_segment.len()))
        .ok_or(CollabVerifyError::MalformedCompactJws)?;
    let signing_input = ticket
        .get(..signing_input_length)
        .ok_or(CollabVerifyError::MalformedCompactJws)?;
    let header_bytes = decode_segment(header_segment, "header", MAX_COLLAB_JWS_HEADER_BYTES)?;
    let header: CollabJwsHeader = serde_json::from_slice(&header_bytes)
        .map_err(|_| CollabVerifyError::MalformedJson { part: "header" })?;
    if header.alg != COLLAB_JWS_ALGORITHM {
        return Err(CollabVerifyError::WrongAlgorithm);
    }
    if header.typ != COLLAB_JWS_TYPE {
        return Err(CollabVerifyError::WrongType);
    }
    if !valid_key_id(&header.kid) {
        return Err(CollabVerifyError::InvalidKeyId);
    }
    let signature_bytes = decode_segment(
        signature_segment,
        "signature",
        MAX_COLLAB_JWS_SIGNATURE_BYTES,
    )?;
    let signature = signature_bytes
        .try_into()
        .map_err(|_| CollabVerifyError::InvalidSignature)?;
    Ok(ParsedCompactJws {
        header,
        claims_segment,
        signature,
        signing_input,
    })
}

fn decode_segment(
    segment: &str,
    part: &'static str,
    maximum_decoded_bytes: usize,
) -> Result<Vec<u8>, CollabVerifyError> {
    let full_groups = maximum_decoded_bytes
        .checked_div(3)
        .and_then(|value| value.checked_mul(4))
        .ok_or(CollabVerifyError::SegmentTooLarge {
            part,
            maximum: maximum_decoded_bytes,
        })?;
    let tail = match maximum_decoded_bytes % 3 {
        0 => 0,
        1 => 2,
        _ => 3,
    };
    let maximum_encoded_bytes =
        full_groups
            .checked_add(tail)
            .ok_or(CollabVerifyError::SegmentTooLarge {
                part,
                maximum: maximum_decoded_bytes,
            })?;
    if segment.len() > maximum_encoded_bytes {
        return Err(CollabVerifyError::SegmentTooLarge {
            part,
            maximum: maximum_decoded_bytes,
        });
    }
    if segment.contains('=') {
        return Err(CollabVerifyError::InvalidBase64 { part });
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| CollabVerifyError::InvalidBase64 { part })?;
    if decoded.len() > maximum_decoded_bytes || URL_SAFE_NO_PAD.encode(&decoded) != segment {
        return Err(CollabVerifyError::InvalidBase64 { part });
    }
    Ok(decoded)
}

fn decode_channel_binding(value: &str) -> Result<[u8; 32], CollabVerifyError> {
    if value.len() > 64 || value.contains('=') {
        return Err(CollabVerifyError::InvalidChannelBinding);
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| CollabVerifyError::InvalidChannelBinding)?;
    if URL_SAFE_NO_PAD.encode(&decoded) != value {
        return Err(CollabVerifyError::InvalidChannelBinding);
    }
    let binding: [u8; 32] = decoded
        .try_into()
        .map_err(|_| CollabVerifyError::InvalidChannelBinding)?;
    if binding.iter().all(|byte| *byte == 0) {
        return Err(CollabVerifyError::InvalidChannelBinding);
    }
    Ok(binding)
}

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    use ed25519_dalek::{Signer as _, SigningKey};
    use serde_json::{json, Value};

    use super::*;
    use crate::{
        StaticTestJwksFetcher, TestCollabIssuer, TestCollabTicketSpec, COLLAB_TICKET_AUDIENCE,
        COLLAB_TICKET_SCOPE, COLLAB_TICKET_VERSION, MAX_COLLAB_PROFILE_AVATAR_URL_BYTES,
        MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS, TEST_AVATAR_URL, TEST_COLLAB_ISSUER, TEST_DEVICE_ID,
        TEST_DISPLAY_NAME, TEST_SUBJECT, TEST_TICKET_ID,
    };

    const NOW: u64 = 2_000_000_000;
    const CHANNEL_BINDING: [u8; 32] = [0x42; 32];
    const OTHER_CHANNEL_BINDING: [u8; 32] = [0x24; 32];
    const TEST_KEY_A_SEED: [u8; 32] = [0x11; 32];

    fn test_verifier() -> CollabTicketVerifier<StaticTestJwksFetcher> {
        let issuer = TestCollabIssuer::initial();
        CollabTicketVerifier::new(
            TestCollabIssuer::verifier_config().unwrap(),
            StaticTestJwksFetcher::new(issuer.jwks_json().unwrap(), 300),
            CollabJwksCacheLimits::default(),
        )
        .unwrap()
    }

    fn claims() -> Value {
        json!({
            "iss": TEST_COLLAB_ISSUER,
            "aud": COLLAB_TICKET_AUDIENCE,
            "ver": COLLAB_TICKET_VERSION,
            "sub": TEST_SUBJECT,
            "device_id": TEST_DEVICE_ID,
            "dh_pub_x25519": URL_SAFE_NO_PAD.encode(CHANNEL_BINDING),
            "scope": COLLAB_TICKET_SCOPE,
            "iat": NOW,
            "nbf": NOW,
            "exp": NOW + 900,
            "jti": TEST_TICKET_ID,
            "display_name": TEST_DISPLAY_NAME,
            "avatar_url": TEST_AVATAR_URL,
        })
    }

    fn header() -> Value {
        json!({
            "alg": COLLAB_JWS_ALGORITHM,
            "typ": COLLAB_JWS_TYPE,
            "kid": "test_key_A",
        })
    }

    fn sign(header: &Value, claims: &Value) -> Vec<u8> {
        let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(header).unwrap());
        let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
        sign_encoded_segments(&header, &claims)
    }

    fn sign_encoded_segments(header_segment: &str, claims_segment: &str) -> Vec<u8> {
        let signing_input = format!("{header_segment}.{claims_segment}");
        let signature = SigningKey::from_bytes(&TEST_KEY_A_SEED)
            .sign(signing_input.as_bytes())
            .to_bytes();
        format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(signature)).into_bytes()
    }

    fn verify_claims(
        verifier: &CollabTicketVerifier<StaticTestJwksFetcher>,
        claims: &Value,
        now_unix_seconds: u64,
    ) -> Result<VerifiedCollabClaims, CollabVerifyError> {
        verifier.verify_at(
            &sign(&header(), claims),
            &CHANNEL_BINDING,
            now_unix_seconds,
            Instant::now(),
        )
    }

    #[test]
    fn cancellable_verification_has_closed_cancelled_error() {
        assert_eq!(
            test_verifier().verify_at_cancellable(
                b"not-a-ticket",
                &CHANNEL_BINDING,
                NOW,
                Instant::now(),
                &|| true,
            ),
            Err(CollabVerifyError::Cancelled)
        );
    }

    #[test]
    fn verifies_the_frozen_profile_and_redacts_identity_debug_output() {
        let issuer = TestCollabIssuer::initial();
        let ticket = issuer
            .issue(&TestCollabTicketSpec::valid_at(NOW, CHANNEL_BINDING))
            .unwrap();
        let verifier = test_verifier();
        let verified = verifier
            .verify_at(ticket.expose(), &CHANNEL_BINDING, NOW, Instant::now())
            .unwrap();

        assert_eq!(verified.issuer(), TEST_COLLAB_ISSUER);
        assert_eq!(verified.subject(), TEST_SUBJECT);
        assert_eq!(verified.device_id(), TEST_DEVICE_ID);
        assert_eq!(verified.dh_pub_x25519(), &CHANNEL_BINDING);
        assert_eq!(verified.issued_at_unix_seconds(), NOW);
        assert_eq!(verified.not_before_unix_seconds(), NOW);
        assert_eq!(verified.expires_at_unix_seconds(), NOW + 900);
        assert_eq!(verified.expires_at_unix_ms(), (NOW + 900) * 1_000);
        assert_eq!(verified.ticket_id(), TEST_TICKET_ID);
        assert_eq!(verified.display_name(), Some(TEST_DISPLAY_NAME));
        assert_eq!(verified.avatar_url(), Some(TEST_AVATAR_URL));
        assert_eq!(verifier.cached_key_count().unwrap(), 1);

        let debug = format!("{verified:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(TEST_SUBJECT));
        assert!(!debug.contains(TEST_DEVICE_ID));
        assert!(!debug.contains(TEST_TICKET_ID));
        assert!(!debug.contains(TEST_DISPLAY_NAME));
        assert!(!debug.contains(TEST_AVATAR_URL));

        let spec_debug = format!("{:?}", TestCollabTicketSpec::valid_at(NOW, CHANNEL_BINDING));
        assert!(!spec_debug.contains(TEST_SUBJECT));
        assert!(!spec_debug.contains(TEST_DEVICE_ID));
        assert!(!spec_debug.contains(TEST_DISPLAY_NAME));
        assert!(!spec_debug.contains(TEST_AVATAR_URL));
    }

    #[test]
    fn accepts_legacy_ticket_without_optional_signed_profile() {
        let verifier = test_verifier();
        let mut legacy = claims();
        legacy.as_object_mut().unwrap().remove("display_name");
        legacy.as_object_mut().unwrap().remove("avatar_url");
        let verified = verify_claims(&verifier, &legacy, NOW).unwrap();
        assert_eq!(verified.display_name(), None);
        assert_eq!(verified.avatar_url(), None);
    }

    #[test]
    fn production_signed_policy_path_never_falls_back_to_raw_jwks() {
        let issuer = TestCollabIssuer::initial();
        let ticket = issuer
            .issue(&TestCollabTicketSpec::valid_at(NOW, CHANNEL_BINDING))
            .unwrap();
        let verifier = CollabTicketVerifier::production(StaticTestJwksFetcher::new(
            issuer.jwks_json().unwrap(),
            300,
        ))
        .unwrap();

        assert!(verifier.config().uses_signed_policy());
        assert_eq!(
            verifier.verify_at(ticket.expose(), &CHANNEL_BINDING, NOW, Instant::now()),
            Err(CollabVerifyError::Jwks(CollabJwksError::Policy(
                crate::CollabUnionPolicyError::MalformedJson
            )))
        );
    }

    #[test]
    fn enforces_signature_profile_and_channel_binding() {
        let verifier = test_verifier();

        let mut wrong_algorithm = header();
        wrong_algorithm["alg"] = json!("EdDSA");
        assert_eq!(
            verifier.verify_at(
                &sign(&wrong_algorithm, &claims()),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::WrongAlgorithm)
        );

        let mut wrong_type = header();
        wrong_type["typ"] = json!("JWT");
        assert_eq!(
            verifier.verify_at(
                &sign(&wrong_type, &claims()),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::WrongType)
        );

        let mut extra_header = header();
        extra_header["jku"] = json!("https://attacker.invalid/jwks");
        assert_eq!(
            verifier.verify_at(
                &sign(&extra_header, &claims()),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::MalformedJson { part: "header" })
        );

        assert_eq!(
            verifier.verify_at(
                &sign(&header(), &claims()),
                &OTHER_CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::ChannelBindingMismatch)
        );
        assert_eq!(
            verifier.verify_at(&sign(&header(), &claims()), &[0; 32], NOW, Instant::now()),
            Err(CollabVerifyError::InvalidChannelBinding)
        );
    }

    #[test]
    fn rejects_tampering_and_unknown_claims() {
        let verifier = test_verifier();
        let mut signed = sign(&header(), &claims());
        let first_separator = signed.iter().position(|byte| *byte == b'.').unwrap();
        let claims_byte = first_separator + 2;
        signed[claims_byte] = if signed[claims_byte] == b'A' {
            b'B'
        } else {
            b'A'
        };
        assert_eq!(
            verifier.verify_at(&signed, &CHANNEL_BINDING, NOW, Instant::now()),
            Err(CollabVerifyError::InvalidSignature)
        );

        let mut extra_claim = claims();
        extra_claim["role"] = json!("owner");
        assert_eq!(
            verify_claims(&verifier, &extra_claim, NOW),
            Err(CollabVerifyError::MalformedJson { part: "claims" })
        );
    }

    #[test]
    fn rejects_invalid_identity_authorization_and_time_claims() {
        let verifier = test_verifier();

        let mut invalid = claims();
        invalid["aud"] = json!("another-service");
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidAudience)
        );
        invalid = claims();
        invalid["ver"] = json!(2);
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidVersion)
        );
        invalid = claims();
        invalid["scope"] = json!("collab:admin");
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidScope)
        );
        invalid = claims();
        invalid["sub"] = json!("123E4567-e89b-12d3-a456-426614174000");
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidSubject)
        );
        invalid = claims();
        invalid["device_id"] = json!("00000000-0000-0000-0000-000000000000");
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidDeviceId)
        );
        invalid = claims();
        invalid["jti"] = json!("not.a.canonical.id");
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidTicketId)
        );
        invalid = claims();
        invalid["dh_pub_x25519"] = json!("AA");
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidChannelBinding)
        );
        invalid = claims();
        invalid["display_name"] = json!(" leading-space");
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidDisplayName)
        );
        invalid = claims();
        invalid["display_name"] = json!("x".repeat(MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS + 1));
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidDisplayName)
        );
        invalid = claims();
        invalid["avatar_url"] = json!("http://cdn.test.invalid/avatar.png");
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidAvatarUrl)
        );
        invalid = claims();
        invalid["avatar_url"] = json!(format!(
            "https://cdn.test.invalid/{}",
            "a".repeat(MAX_COLLAB_PROFILE_AVATAR_URL_BYTES)
        ));
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidAvatarUrl)
        );

        invalid = claims();
        invalid["iat"] = json!(NOW + 1);
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::InvalidTimestamps)
        );
        invalid = claims();
        invalid["nbf"] = json!(NOW + 61);
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::NotYetValid)
        );
        invalid = claims();
        invalid["iat"] = json!(NOW - 900);
        invalid["nbf"] = json!(NOW - 900);
        invalid["exp"] = json!(NOW - 61);
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::Expired)
        );
        invalid = claims();
        invalid["exp"] = json!(NOW + 901);
        assert_eq!(
            verify_claims(&verifier, &invalid, NOW),
            Err(CollabVerifyError::LifetimeTooLong)
        );
        invalid = claims();
        invalid["iat"] = json!(u64::MAX - 1);
        invalid["nbf"] = json!(u64::MAX - 1);
        invalid["exp"] = json!(u64::MAX);
        assert_eq!(
            verify_claims(&verifier, &invalid, u64::MAX - 1),
            Err(CollabVerifyError::ExpiryOverflow)
        );
    }

    #[test]
    fn rejects_malformed_and_oversized_compact_inputs_before_fetch() {
        let verifier = test_verifier();
        assert_eq!(
            verifier.verify_at(b"", &CHANNEL_BINDING, NOW, Instant::now()),
            Err(CollabVerifyError::InvalidTicketSize {
                maximum: MAX_COLLAB_TICKET_BYTES
            })
        );
        assert_eq!(
            verifier.verify_at(b"only.two", &CHANNEL_BINDING, NOW, Instant::now()),
            Err(CollabVerifyError::MalformedCompactJws)
        );
        let padded_header = format!(
            "{}=.{}.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header()).unwrap()),
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims()).unwrap()),
            URL_SAFE_NO_PAD.encode([0; 64])
        );
        assert_eq!(
            verifier.verify_at(
                padded_header.as_bytes(),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::InvalidBase64 { part: "header" })
        );
        let mut invalid_key_id = header();
        invalid_key_id["kid"] = json!("bad.key");
        assert_eq!(
            verifier.verify_at(
                &sign(&invalid_key_id, &claims()),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::InvalidKeyId)
        );
        let short_signature = format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header()).unwrap()),
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims()).unwrap()),
            URL_SAFE_NO_PAD.encode([0; 63])
        );
        assert_eq!(
            verifier.verify_at(
                short_signature.as_bytes(),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::InvalidSignature)
        );
        let oversized_header = URL_SAFE_NO_PAD.encode(vec![b'a'; MAX_COLLAB_JWS_HEADER_BYTES + 1]);
        let valid_claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims()).unwrap());
        assert_eq!(
            verifier.verify_at(
                &sign_encoded_segments(&oversized_header, &valid_claims),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::SegmentTooLarge {
                part: "header",
                maximum: MAX_COLLAB_JWS_HEADER_BYTES
            })
        );
        let valid_header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header()).unwrap());
        let oversized_claims = URL_SAFE_NO_PAD.encode(vec![b'a'; MAX_COLLAB_JWS_CLAIMS_BYTES + 1]);
        assert_eq!(
            verifier.verify_at(
                &sign_encoded_segments(&valid_header, &oversized_claims),
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::SegmentTooLarge {
                part: "claims",
                maximum: MAX_COLLAB_JWS_CLAIMS_BYTES
            })
        );
        assert_eq!(
            verifier.verify_at(
                &vec![b'a'; MAX_COLLAB_TICKET_BYTES + 1],
                &CHANNEL_BINDING,
                NOW,
                Instant::now()
            ),
            Err(CollabVerifyError::InvalidTicketSize {
                maximum: MAX_COLLAB_TICKET_BYTES
            })
        );
    }
}
