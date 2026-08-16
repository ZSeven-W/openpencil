use std::fmt;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use zeroize::Zeroizing;

use crate::locator::require_exact;
use crate::{
    RelayLocatorV1, RelayLocatorVerifier, RelayProtocolError, RouteCapability, VerifiedRelayRoute,
    MAX_INVITE_CHARS, RELAY_LOCATOR_BINARY_BYTES, RELAY_PROTOCOL_VERSION,
};

pub const RELAY_INVITE_PREFIX: &str = "opc1_";
pub const RELAY_INVITE_BINARY_BYTES: usize = 373;

const LOCATOR_OFFSET: usize = 1;
const CAPABILITY_OFFSET: usize = LOCATOR_OFFSET + RELAY_LOCATOR_BINARY_BYTES;
const CHECKSUM_OFFSET: usize = CAPABILITY_OFFSET + 32;
const CHECKSUM_CONTEXT: &str = "openpencil/op-collab-relay-protocol/invite-corruption-checksum/v1";

/// A single pasteable invite containing a signed locator and bearer capability.
///
/// Treat its encoded form as a secret. It is suitable for a protected URL
/// fragment, but never for a URL path or query where intermediaries log it.
/// Decoding does not authenticate the locator; call [`Self::verify`] before
/// constructing a client hello.
#[derive(Clone, PartialEq, Eq)]
pub struct RelayInviteV1 {
    locator: RelayLocatorV1,
    capability: RouteCapability,
}

impl RelayInviteV1 {
    pub fn new(route: &VerifiedRelayRoute) -> Self {
        Self {
            locator: route.locator().locator().clone(),
            capability: route.capability().clone(),
        }
    }

    /// Combines a signed locator with a locally held route capability.
    ///
    /// This constructor is intended for the owner-side publish flow after the
    /// locator issuer has returned a signature and the caller has verified the
    /// signed claims against its pending publish request. It deliberately does
    /// not mark the locator as verified; invite consumers must still call
    /// [`Self::verify`] with their pinned locator-key policy.
    pub fn from_signed_locator(locator: RelayLocatorV1, capability: RouteCapability) -> Self {
        Self {
            locator,
            capability,
        }
    }

    pub fn locator(&self) -> &RelayLocatorV1 {
        &self.locator
    }

    pub fn to_fragment(&self) -> String {
        let binary = Zeroizing::new(self.to_binary());
        let encoded = Zeroizing::new(URL_SAFE_NO_PAD.encode(binary.as_slice()));
        debug_assert!(RELAY_INVITE_PREFIX.len() + encoded.len() <= MAX_INVITE_CHARS);
        format!("{RELAY_INVITE_PREFIX}{}", encoded.as_str())
    }

    pub fn from_fragment(fragment: &str) -> Result<Self, RelayProtocolError> {
        if fragment.len() > MAX_INVITE_CHARS {
            return Err(RelayProtocolError::LocatorTooLong {
                actual: fragment.len(),
                maximum: MAX_INVITE_CHARS,
            });
        }
        let body = fragment
            .strip_prefix(RELAY_INVITE_PREFIX)
            .ok_or(RelayProtocolError::InvalidLocatorPrefix)?;
        if body.is_empty()
            || body.contains('=')
            || !body
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(RelayProtocolError::InvalidLocatorEncoding);
        }
        let raw = Zeroizing::new(
            URL_SAFE_NO_PAD
                .decode(body)
                .map_err(|_| RelayProtocolError::InvalidLocatorEncoding)?,
        );
        let canonical = Zeroizing::new(URL_SAFE_NO_PAD.encode(raw.as_slice()));
        if canonical.as_str() != body {
            return Err(RelayProtocolError::InvalidLocatorEncoding);
        }
        Self::decode_binary(&raw)
    }

    pub fn verify(
        &self,
        verifier: &impl RelayLocatorVerifier,
        now_unix: u64,
    ) -> Result<VerifiedRelayRoute, RelayProtocolError> {
        let locator = self.locator.verify(verifier, now_unix)?;
        Ok(VerifiedRelayRoute::new(locator, self.capability.clone()))
    }

    fn to_binary(&self) -> [u8; RELAY_INVITE_BINARY_BYTES] {
        let mut raw = [0_u8; RELAY_INVITE_BINARY_BYTES];
        raw[0] = RELAY_PROTOCOL_VERSION;
        raw[LOCATOR_OFFSET..CAPABILITY_OFFSET].copy_from_slice(&self.locator.to_binary());
        raw[CAPABILITY_OFFSET..CHECKSUM_OFFSET].copy_from_slice(self.capability.secret_bytes());
        let checksum = corruption_checksum(&raw[..CHECKSUM_OFFSET]);
        raw[CHECKSUM_OFFSET..].copy_from_slice(&checksum);
        raw
    }

    fn decode_binary(raw: &[u8]) -> Result<Self, RelayProtocolError> {
        require_exact("relay invite", raw.len(), RELAY_INVITE_BINARY_BYTES)?;
        if raw[0] != RELAY_PROTOCOL_VERSION {
            return Err(RelayProtocolError::UnsupportedVersion {
                actual: raw[0],
                expected: RELAY_PROTOCOL_VERSION,
            });
        }
        let locator = RelayLocatorV1::decode_binary(&raw[LOCATOR_OFFSET..CAPABILITY_OFFSET])?;
        let capability = RouteCapability::new(
            raw[CAPABILITY_OFFSET..CHECKSUM_OFFSET]
                .try_into()
                .expect("fixed invite capability range"),
        )?;
        let expected_checksum = corruption_checksum(&raw[..CHECKSUM_OFFSET]);
        if raw[CHECKSUM_OFFSET..] != expected_checksum {
            return Err(RelayProtocolError::ChecksumMismatch);
        }
        Ok(Self {
            locator,
            capability,
        })
    }
}

impl fmt::Debug for RelayInviteV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RelayInviteV1([REDACTED])")
    }
}

fn corruption_checksum(bytes: &[u8]) -> [u8; 4] {
    let mut hasher = blake3::Hasher::new_derive_key(CHECKSUM_CONTEXT);
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut checksum = [0_u8; 4];
    checksum.copy_from_slice(&digest.as_bytes()[..4]);
    checksum
}
