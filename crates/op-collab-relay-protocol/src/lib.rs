//! Bounded public-relay protocol primitives for OpenPencil collaboration.
//!
//! The protocol keeps routing metadata separate from the bearer route
//! capability. A decoded locator is untrusted until a caller explicitly
//! verifies its canonical bytes with [`RelayLocatorVerifier`].

mod challenge;
mod control;
mod error;
mod invite;
mod locator;
mod pairing;
mod pairing_window;
mod reauth;
mod sensitive;

pub use challenge::{
    relay_challenge_proof_binding_digest, RelayChallengeKeyId, RelayChallengeProofV2,
    RelayServerChallengeV1, MAX_RELAY_CHALLENGE_HEADER_BYTES, MAX_RELAY_CHALLENGE_KEY_ID_BYTES,
    RELAY_CHALLENGE_HEADER_NAME, RELAY_CHALLENGE_NONCE_BYTES, RELAY_CHALLENGE_PREFIX,
    RELAY_CHALLENGE_PROOF_V2_BYTES, RELAY_CHALLENGE_PROOF_VERSION, RELAY_CHALLENGE_VERSION,
};
pub use control::{
    RelayAuthExtensionV1, RelayClientHello, RelayHelloAuthMode, RelayRejectCode, RelayRole,
    RelayServerStatus, VerifiedRelayClientHello, MAX_POSSESSION_PROOF_BYTES,
    RELAY_AUTH_EXTENSION_VERSION, RELAY_CLIENT_HELLO_BYTES, RELAY_REJECT_CLOSE_PREFIX,
    RELAY_REJECT_CODES, RELAY_SERVER_STATUS_BYTES,
};
pub use error::RelayProtocolError;
pub use invite::{RelayInviteV1, RELAY_INVITE_BINARY_BYTES, RELAY_INVITE_PREFIX};
pub use locator::{
    ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, RelayLocatorV1, RelayLocatorVerifier,
    RelayRegion, UnsignedRelayLocatorV1, VerifiedRelayLocator, VerifiedRelayRoute,
    LOCATOR_CANONICAL_SIGNING_BYTES, LOCATOR_PREFIX, MAX_EXPECTED_DISCOVERY_ID_BYTES,
    MAX_INVITE_CHARS, MAX_LOCATOR_KEY_ID_BYTES, MAX_PAIRING_LIFETIME_SECS,
    RELAY_LOCATOR_BINARY_BYTES, RELAY_PROTOCOL_VERSION,
};
pub use pairing::{
    PairingCode, SealedPairingInvite, MAX_SEALED_INVITE_BYTES, MAX_SEALED_PAIRING_INVITE_V2_BYTES,
    PAIRING_CODE_ALPHABET, PAIRING_CODE_CHARS, PAIRING_CODE_ID_BYTES, SEALED_INVITE_NONCE_BYTES,
    SEALED_INVITE_TAG_BYTES, SEALED_INVITE_V1_NONCE_BYTES, SEALED_INVITE_V1_TAG_BYTES,
    SEALED_PAIRING_INVITE_V1_VERSION, SEALED_PAIRING_INVITE_VERSION,
};
pub use pairing_window::{
    RelayWaitingAdvertisementV1, DEFAULT_RELAY_WAITING_TIMEOUT_SECS, MAX_ADVERTISED_WAITING_SECS,
    MAX_RELAY_WAITING_HEADER_BYTES, MAX_RELAY_WAITING_TIMEOUT_SECS,
    MIN_DERIVED_OWNER_LANE_BUDGET_SECS, MIN_RELAY_WAITING_TIMEOUT_SECS,
    RELAY_OWNER_LANE_RECYCLE_SECS, RELAY_WAITING_HEADER_NAME, RELAY_WAITING_HEADER_PREFIX,
    RELAY_WAITING_HEADROOM_SECS, RELAY_WAITING_SAFETY_MARGIN_SECS,
};
pub use reauth::{
    is_valid_relay_bearer_token, RelayReauthChallengeV1, RelayReauthResponseV1,
    MAX_RELAY_BEARER_BYTES, MAX_RELAY_REAUTH_CHALLENGE_TEXT_BYTES,
    MAX_RELAY_REAUTH_RESPONSE_TEXT_BYTES, RELAY_REAUTH_CHALLENGE_PREFIX,
    RELAY_REAUTH_RESPONSE_PREFIX, RELAY_REAUTH_VERSION,
};
pub use sensitive::{
    CallerDeviceDhPublic, OwnerNoiseStatic, RouteCapability, RouteId, RouteMapKey,
};
