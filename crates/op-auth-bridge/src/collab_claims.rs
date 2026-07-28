//! Frozen claims and protected-header profile for collaboration tickets.
//!
//! The algorithm name follows the fully specified Ed25519 JWS registration;
//! the legacy polymorphic `EdDSA` identifier is deliberately rejected.

use std::fmt;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::{CollabVerifierConfigError, CollabVerifyError};

/// Required protected-header `alg`.
pub const COLLAB_JWS_ALGORITHM: &str = "Ed25519";
/// Required protected-header `typ`.
pub const COLLAB_JWS_TYPE: &str = "openpencil-collab+jwt";
/// Required `aud` claim.
pub const COLLAB_TICKET_AUDIENCE: &str = "openpencil-collab";
/// Required authorization scope.
pub const COLLAB_TICKET_SCOPE: &str = "collab:connect";
/// Supported ticket claims revision.
pub const COLLAB_TICKET_VERSION: u32 = 1;
/// Maximum signed `exp - iat` interval.
pub const COLLAB_TICKET_MAX_LIFETIME_SECONDS: u64 = 15 * 60;
/// Bounded wall-clock tolerance used for `iat`, `nbf`, and `exp`.
pub const COLLAB_TICKET_CLOCK_SKEW_SECONDS: u64 = 60;
/// Maximum UTF-8 size of a signed collaboration display name.
pub const MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES: usize = 320;
/// Maximum Unicode scalar count of a signed collaboration display name.
pub const MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS: usize = 80;
/// Maximum UTF-8 size of a signed collaboration avatar URL.
pub const MAX_COLLAB_PROFILE_AVATAR_URL_BYTES: usize = 2_048;

/// Issuer pinned by [`CollabVerifierConfig::production`].
pub const PRODUCTION_COLLAB_ISSUER: &str = "https://sso.zseven.cn";
/// Fixed legacy collaboration keyset path.
pub const COLLAB_JWKS_PATH: &str = "/api/v1/collab/jwks";
/// Fixed offline-signed union-policy path served by every trusted SSO origin.
pub const COLLAB_POLICY_PATH: &str = "/api/v1/collab/policy";
/// Legacy public-key endpoint retained for explicit self-hosted deployments.
pub const PRODUCTION_COLLAB_JWKS_ENDPOINT: &str = "https://sso.zseven.cn/api/v1/collab/jwks";
/// Signed union-policy endpoint pinned by [`CollabVerifierConfig::production`].
pub const PRODUCTION_COLLAB_POLICY_ENDPOINT: &str = "https://sso.zseven.cn/api/v1/collab/policy";

pub(crate) const MAX_ISSUER_BYTES: usize = 256;
pub(crate) const MAX_JWKS_ENDPOINT_BYTES: usize = 512;
pub(crate) const MAX_KEY_ID_BYTES: usize = 128;
pub(crate) const MAX_TICKET_ID_BYTES: usize = 128;
const MIN_TICKET_ID_BYTES: usize = 22;

/// A verifier trust root pinned by the open host, never by ticket contents.
#[derive(Clone, PartialEq, Eq)]
pub struct CollabVerifierConfig {
    issuer: String,
    endpoint: String,
    key_source: CollabKeySource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollabKeySource {
    SignedPolicy,
    LegacyJwks,
}

impl CollabVerifierConfig {
    pub fn production() -> Self {
        Self {
            issuer: PRODUCTION_COLLAB_ISSUER.to_owned(),
            endpoint: PRODUCTION_COLLAB_POLICY_ENDPOINT.to_owned(),
            key_source: CollabKeySource::SignedPolicy,
        }
    }

    /// Build an explicit legacy JWKS trust root from a strict HTTPS SSO origin.
    ///
    /// This compatibility path does not validate an offline-signed regional
    /// union policy. New production deployments should use
    /// [`Self::for_policy_origin`].
    pub fn for_sso_origin(
        sso_origin: impl Into<String>,
    ) -> Result<Self, CollabVerifierConfigError> {
        let issuer = sso_origin.into();
        if !valid_https_origin(&issuer) {
            return Err(CollabVerifierConfigError::InvalidIssuer);
        }
        let jwks_endpoint = format!("{issuer}{COLLAB_JWKS_PATH}");
        Self::new_pinned(issuer, jwks_endpoint)
    }

    /// Build a pinned trust root for a controlled deployment or test.
    ///
    /// The caller must source both values from trusted application
    /// configuration. Neither value may come from a ticket or provider result.
    pub fn new_pinned(
        issuer: impl Into<String>,
        jwks_endpoint: impl Into<String>,
    ) -> Result<Self, CollabVerifierConfigError> {
        let issuer = issuer.into();
        let jwks_endpoint = jwks_endpoint.into();
        if !valid_https_origin(&issuer) {
            return Err(CollabVerifierConfigError::InvalidIssuer);
        }
        if !valid_jwks_endpoint(&jwks_endpoint) {
            return Err(CollabVerifierConfigError::InvalidJwksEndpoint);
        }
        Ok(Self {
            issuer,
            endpoint: jwks_endpoint,
            key_source: CollabKeySource::LegacyJwks,
        })
    }

    /// Build an offline-signed union-policy trust root on an SSO origin.
    pub fn for_policy_origin(
        sso_origin: impl Into<String>,
    ) -> Result<Self, CollabVerifierConfigError> {
        let issuer = sso_origin.into();
        if !valid_https_origin(&issuer) {
            return Err(CollabVerifierConfigError::InvalidIssuer);
        }
        let policy_endpoint = format!("{issuer}{COLLAB_POLICY_PATH}");
        Self::new_signed_policy(issuer, policy_endpoint)
    }

    /// Pin the logical issuer and regional signed-policy mirror independently.
    pub fn new_signed_policy(
        issuer: impl Into<String>,
        policy_endpoint: impl Into<String>,
    ) -> Result<Self, CollabVerifierConfigError> {
        let issuer = issuer.into();
        let policy_endpoint = policy_endpoint.into();
        if !valid_https_origin(&issuer) {
            return Err(CollabVerifierConfigError::InvalidIssuer);
        }
        if !valid_jwks_endpoint(&policy_endpoint) {
            return Err(CollabVerifierConfigError::InvalidPolicyEndpoint);
        }
        Ok(Self {
            issuer,
            endpoint: policy_endpoint,
            key_source: CollabKeySource::SignedPolicy,
        })
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Return the configured key-authority endpoint.
    pub fn keyset_endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Return the endpoint for the explicit legacy JWKS source.
    ///
    /// Kept as a non-optional compatibility accessor. Call
    /// [`Self::uses_signed_policy`] before interpreting the endpoint body.
    pub fn jwks_endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn policy_endpoint(&self) -> Option<&str> {
        self.uses_signed_policy().then_some(&self.endpoint)
    }

    pub fn uses_signed_policy(&self) -> bool {
        self.key_source == CollabKeySource::SignedPolicy
    }

    pub(crate) fn key_source(&self) -> CollabKeySource {
        self.key_source
    }
}

impl fmt::Debug for CollabVerifierConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollabVerifierConfig")
            .field("issuer", &self.issuer)
            .field("key_source", &self.key_source)
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CollabJwsHeader {
    pub alg: String,
    pub typ: String,
    pub kid: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UnverifiedCollabClaims {
    pub iss: String,
    pub aud: String,
    pub ver: u32,
    pub sub: String,
    pub device_id: String,
    pub dh_pub_x25519: String,
    pub scope: String,
    pub iat: u64,
    pub nbf: u64,
    pub exp: u64,
    pub jti: String,
    /// Optional for compatibility with issuers that have not yet rolled out
    /// signed collaboration profile metadata.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Optional for compatibility with issuers that have not yet rolled out
    /// signed collaboration profile metadata.
    #[serde(default)]
    pub avatar_url: Option<String>,
}

/// Identity metadata produced only after signature, claim, and channel checks.
#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedCollabClaims {
    issuer: String,
    subject: String,
    device_id: String,
    dh_pub_x25519: [u8; 32],
    issued_at_unix_seconds: u64,
    not_before_unix_seconds: u64,
    expires_at_unix_seconds: u64,
    expires_at_unix_ms: u64,
    ticket_id: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

impl VerifiedCollabClaims {
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn dh_pub_x25519(&self) -> &[u8; 32] {
        &self.dh_pub_x25519
    }

    pub fn issued_at_unix_seconds(&self) -> u64 {
        self.issued_at_unix_seconds
    }

    pub fn not_before_unix_seconds(&self) -> u64 {
        self.not_before_unix_seconds
    }

    pub fn expires_at_unix_seconds(&self) -> u64 {
        self.expires_at_unix_seconds
    }

    pub fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at_unix_ms
    }

    pub fn ticket_id(&self) -> &str {
        &self.ticket_id
    }

    /// Display name authenticated by the ticket signature, or `None` when the
    /// issuer omitted the optional profile claim.
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// HTTPS avatar URL authenticated by the ticket signature, or `None` when
    /// the issuer omitted the optional profile claim.
    pub fn avatar_url(&self) -> Option<&str> {
        self.avatar_url.as_deref()
    }
}

impl fmt::Debug for VerifiedCollabClaims {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedCollabClaims")
            .field("issuer", &self.issuer)
            .field("subject", &"[REDACTED]")
            .field("device_id", &"[REDACTED]")
            .field("dh_pub_x25519", &"[REDACTED]")
            .field("issued_at_unix_seconds", &self.issued_at_unix_seconds)
            .field("not_before_unix_seconds", &self.not_before_unix_seconds)
            .field("expires_at_unix_seconds", &self.expires_at_unix_seconds)
            .field("ticket_id", &"[REDACTED]")
            .field(
                "display_name",
                &self.display_name.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "avatar_url",
                &self.avatar_url.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

pub(crate) fn validate_claims(
    claims: UnverifiedCollabClaims,
    config: &CollabVerifierConfig,
    expected_dh_pub_x25519: &[u8; 32],
    decoded_dh_pub_x25519: [u8; 32],
    now_unix_seconds: u64,
) -> Result<VerifiedCollabClaims, CollabVerifyError> {
    if claims.iss != config.issuer {
        return Err(CollabVerifyError::InvalidIssuer);
    }
    if claims.aud != COLLAB_TICKET_AUDIENCE {
        return Err(CollabVerifyError::InvalidAudience);
    }
    if claims.ver != COLLAB_TICKET_VERSION {
        return Err(CollabVerifyError::InvalidVersion);
    }
    if claims.scope != COLLAB_TICKET_SCOPE {
        return Err(CollabVerifyError::InvalidScope);
    }
    if !valid_canonical_uuid(&claims.sub) {
        return Err(CollabVerifyError::InvalidSubject);
    }
    if !valid_canonical_uuid(&claims.device_id) {
        return Err(CollabVerifyError::InvalidDeviceId);
    }
    if decoded_dh_pub_x25519 != *expected_dh_pub_x25519 {
        return Err(CollabVerifyError::ChannelBindingMismatch);
    }
    if !valid_ticket_id(&claims.jti) {
        return Err(CollabVerifyError::InvalidTicketId);
    }
    if claims
        .display_name
        .as_deref()
        .is_some_and(|value| !valid_profile_display_name(value))
    {
        return Err(CollabVerifyError::InvalidDisplayName);
    }
    if claims
        .avatar_url
        .as_deref()
        .is_some_and(|value| !valid_profile_avatar_url(value))
    {
        return Err(CollabVerifyError::InvalidAvatarUrl);
    }
    if claims.iat > claims.nbf || claims.nbf >= claims.exp {
        return Err(CollabVerifyError::InvalidTimestamps);
    }
    let latest_accepted_future = now_unix_seconds.saturating_add(COLLAB_TICKET_CLOCK_SKEW_SECONDS);
    if claims.iat > latest_accepted_future {
        return Err(CollabVerifyError::InvalidTimestamps);
    }
    if claims.nbf > latest_accepted_future {
        return Err(CollabVerifyError::NotYetValid);
    }
    let earliest_accepted_expiry =
        now_unix_seconds.saturating_sub(COLLAB_TICKET_CLOCK_SKEW_SECONDS);
    if claims.exp <= earliest_accepted_expiry {
        return Err(CollabVerifyError::Expired);
    }
    let lifetime = claims
        .exp
        .checked_sub(claims.iat)
        .ok_or(CollabVerifyError::InvalidTimestamps)?;
    if lifetime > COLLAB_TICKET_MAX_LIFETIME_SECONDS {
        return Err(CollabVerifyError::LifetimeTooLong);
    }
    let expires_at_unix_ms = claims
        .exp
        .checked_mul(1_000)
        .ok_or(CollabVerifyError::ExpiryOverflow)?;

    Ok(VerifiedCollabClaims {
        issuer: claims.iss,
        subject: claims.sub,
        device_id: claims.device_id,
        dh_pub_x25519: decoded_dh_pub_x25519,
        issued_at_unix_seconds: claims.iat,
        not_before_unix_seconds: claims.nbf,
        expires_at_unix_seconds: claims.exp,
        expires_at_unix_ms,
        ticket_id: claims.jti,
        display_name: claims.display_name,
        avatar_url: claims.avatar_url,
    })
}

pub(crate) fn valid_key_id(value: &str) -> bool {
    valid_bounded_base64url_identifier(value, 1, MAX_KEY_ID_BYTES)
}

pub(crate) fn valid_bounded_base64url_identifier(
    value: &str,
    minimum: usize,
    maximum: usize,
) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= minimum
        && bytes.len() <= maximum
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_'))
}

fn valid_ticket_id(value: &str) -> bool {
    if !valid_bounded_base64url_identifier(value, MIN_TICKET_ID_BYTES, MAX_TICKET_ID_BYTES) {
        return false;
    }
    URL_SAFE_NO_PAD
        .decode(value)
        .ok()
        .is_some_and(|decoded| URL_SAFE_NO_PAD.encode(decoded) == value)
}

fn valid_canonical_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36
        || bytes[8] != b'-'
        || bytes[13] != b'-'
        || bytes[18] != b'-'
        || bytes[23] != b'-'
    {
        return false;
    }
    let mut any_nonzero = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            continue;
        }
        if !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return false;
        }
        any_nonzero |= byte != b'0';
    }
    any_nonzero
}

pub(crate) fn valid_profile_display_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES
        && value.chars().count() <= MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

pub(crate) fn valid_profile_avatar_url(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_COLLAB_PROFILE_AVATAR_URL_BYTES
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        || value.contains(['#', '\\'])
    {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = rest
        .split_once(['/', '?'])
        .map_or(rest, |(authority, _)| authority);
    valid_https_authority(authority)
}

pub(crate) fn valid_https_origin(value: &str) -> bool {
    if value.len() > MAX_ISSUER_BYTES || !valid_https_url(value, MAX_ISSUER_BYTES) {
        return false;
    }
    let authority = &value["https://".len()..];
    !authority.contains('/')
}

pub(crate) fn valid_jwks_endpoint(value: &str) -> bool {
    valid_https_url(value, MAX_JWKS_ENDPOINT_BYTES)
}

fn valid_https_url(value: &str, maximum: usize) -> bool {
    if value.is_empty()
        || value.len() > maximum
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or_default();
    valid_https_authority(authority) && !value.contains(['?', '#', '\\'])
}

fn valid_https_authority(authority: &str) -> bool {
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    if let Some(bracketed) = authority.strip_prefix('[') {
        let Some(closing_bracket) = bracketed.find(']') else {
            return false;
        };
        let host = &bracketed[..closing_bracket];
        let suffix = &bracketed[closing_bracket + 1..];
        return host.parse::<std::net::Ipv6Addr>().is_ok()
            && (suffix.is_empty() || suffix.strip_prefix(':').is_some_and(valid_https_port));
    }
    if authority.contains('[') || authority.contains(']') {
        return false;
    }
    let host = match authority.rsplit_once(':') {
        Some((host, port)) => {
            if host.contains(':') || !valid_https_port(port) {
                return false;
            }
            host
        }
        None => authority,
    };
    valid_dns_or_ipv4_host(host)
}

fn valid_https_port(port: &str) -> bool {
    !port.is_empty()
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|port| port != 0)
}

fn valid_dns_or_ipv4_host(host: &str) -> bool {
    if host.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TEST_TICKET_ID;

    #[test]
    fn pinned_configuration_requires_bounded_https_values() {
        assert!(CollabVerifierConfig::new_pinned(
            "https://issuer.example",
            "https://issuer.example/api/jwks"
        )
        .is_ok());
        assert_eq!(
            CollabVerifierConfig::new_pinned(
                "http://issuer.example",
                "https://issuer.example/api/jwks"
            ),
            Err(CollabVerifierConfigError::InvalidIssuer)
        );
        assert_eq!(
            CollabVerifierConfig::new_pinned(
                "https://issuer.example/path",
                "https://issuer.example/api/jwks"
            ),
            Err(CollabVerifierConfigError::InvalidIssuer)
        );
        assert_eq!(
            CollabVerifierConfig::new_pinned(
                "https://issuer.example",
                "https://user@issuer.example/api/jwks"
            ),
            Err(CollabVerifierConfigError::InvalidJwksEndpoint)
        );
        assert_eq!(
            CollabVerifierConfig::new_pinned("https://:443", "https://issuer.example/api/jwks"),
            Err(CollabVerifierConfigError::InvalidIssuer)
        );
        assert_eq!(
            CollabVerifierConfig::new_pinned(
                "https://issuer.example",
                "https://issuer.example:0/api/jwks"
            ),
            Err(CollabVerifierConfigError::InvalidJwksEndpoint)
        );
        assert!(CollabVerifierConfig::new_pinned(
            "https://[::1]:8443",
            "https://[::1]:8443/api/jwks"
        )
        .is_ok());
    }

    #[test]
    fn sso_origin_helper_pins_the_fixed_same_origin_keyset() {
        let config =
            CollabVerifierConfig::for_sso_origin("https://auth.example:8443").expect("config");
        assert!(!config.uses_signed_policy());
        assert_eq!(config.issuer(), "https://auth.example:8443");
        assert_eq!(
            config.jwks_endpoint(),
            "https://auth.example:8443/api/v1/collab/jwks"
        );

        for invalid in [
            "http://auth.example",
            "https://auth.example/",
            "https://auth.example/path",
            "https://user@auth.example",
            "https://auth.example?jwks=https://peer.example",
            " https://auth.example",
        ] {
            assert_eq!(
                CollabVerifierConfig::for_sso_origin(invalid),
                Err(CollabVerifierConfigError::InvalidIssuer)
            );
        }
    }

    #[test]
    fn production_and_policy_helpers_pin_the_signed_policy_source() {
        let production = CollabVerifierConfig::production();
        assert!(production.uses_signed_policy());
        assert_eq!(
            production.policy_endpoint(),
            Some(PRODUCTION_COLLAB_POLICY_ENDPOINT)
        );

        let regional =
            CollabVerifierConfig::for_policy_origin("https://auth.example:8443").unwrap();
        assert!(regional.uses_signed_policy());
        assert_eq!(
            regional.policy_endpoint(),
            Some("https://auth.example:8443/api/v1/collab/policy")
        );
        assert_eq!(
            CollabVerifierConfig::new_signed_policy(
                "https://collab.example",
                "http://auth.example/api/v1/collab/policy"
            ),
            Err(CollabVerifierConfigError::InvalidPolicyEndpoint)
        );
    }

    #[test]
    fn identifiers_are_canonical() {
        assert!(valid_key_id("key_A-1"));
        assert!(!valid_key_id(""));
        assert!(!valid_key_id("key.with.dot"));
        assert!(valid_ticket_id(TEST_TICKET_ID));
        assert!(!valid_ticket_id("AAAAAAAAAAAAAAAAAAAAAB"));
        assert!(valid_canonical_uuid("123e4567-e89b-12d3-a456-426614174000"));
        assert!(!valid_canonical_uuid(
            "123E4567-e89b-12d3-a456-426614174000"
        ));
        assert!(!valid_canonical_uuid(
            "00000000-0000-0000-0000-000000000000"
        ));
    }

    #[test]
    fn signed_profile_fields_are_strictly_bounded_and_https_only() {
        assert!(valid_profile_display_name("Kay 沈"));
        assert!(!valid_profile_display_name(""));
        assert!(!valid_profile_display_name(" Kay"));
        assert!(!valid_profile_display_name("Kay\nAdmin"));
        assert!(!valid_profile_display_name(
            &"a".repeat(MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS + 1)
        ));

        assert!(valid_profile_avatar_url(
            "https://cdn.example.test/avatar.png?size=128"
        ));
        assert!(!valid_profile_avatar_url(
            "http://cdn.example.test/avatar.png"
        ));
        assert!(!valid_profile_avatar_url(
            "https://user@cdn.example.test/avatar.png"
        ));
        assert!(!valid_profile_avatar_url(
            "https://cdn.example.test/avatar.png#fragment"
        ));
        assert!(!valid_profile_avatar_url(
            "https://cdn.example.test\\@attacker.invalid/avatar.png"
        ));
    }
}
