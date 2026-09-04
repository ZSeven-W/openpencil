#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RelayProtocolError {
    #[error("{context} is truncated: expected {expected} bytes, received {actual}")]
    Truncated {
        context: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("{context} has trailing bytes: expected {expected}, received {actual}")]
    TrailingBytes {
        context: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("relay protocol version {actual} is unsupported; expected {expected}")]
    UnsupportedVersion { actual: u8, expected: u8 },
    #[error("relay region code {0} is unknown")]
    InvalidRegion(u8),
    #[error("relay role code {0} is unknown")]
    InvalidRole(u8),
    #[error("relay hello authentication mode {0} is unsupported")]
    InvalidAuthMode(u8),
    #[error("relay challenge has an invalid prefix")]
    InvalidChallengePrefix,
    #[error("relay challenge is not canonical base64url without padding")]
    InvalidChallengeEncoding,
    #[error("relay challenge header length {actual} exceeds maximum {maximum}")]
    ChallengeHeaderTooLong { actual: usize, maximum: usize },
    #[error("relay challenge version {actual} is unsupported; expected {expected}")]
    UnsupportedChallengeVersion { actual: u8, expected: u8 },
    #[error("relay challenge nonce must not be all zero")]
    ZeroChallengeNonce,
    #[error("relay challenge proof version {actual} is unsupported; expected {expected}")]
    UnsupportedChallengeProofVersion { actual: u8, expected: u8 },
    #[error("relay challenge proof is invalid")]
    InvalidChallengeProof,
    #[error("relay challenge proof requires authentication mode 2")]
    ChallengeProofRequiresV2AuthMode,
    #[error("X25519 produced a non-contributory shared secret")]
    NonContributoryX25519SharedSecret,
    #[error("relay challenge proof derivation failed")]
    ChallengeProofDerivationFailed,
    #[error("relay challenge proof verification failed")]
    ChallengeProofVerificationFailed,
    #[error("relay reauthentication control has an invalid prefix")]
    InvalidReauthControlPrefix,
    #[error("relay reauthentication control is not canonical base64url without padding")]
    InvalidReauthControlEncoding,
    #[error("relay reauthentication control length {actual} exceeds maximum {maximum}")]
    ReauthControlTooLong { actual: usize, maximum: usize },
    #[error("relay reauthentication version {actual} is unsupported; expected {expected}")]
    UnsupportedReauthVersion { actual: u8, expected: u8 },
    #[error("relay reauthentication {field} length {actual} exceeds maximum {maximum}")]
    ReauthControlFieldTooLong {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("relay bearer credential is invalid")]
    InvalidRelayBearer,
    #[error("relay reauthentication requires a complete mode-2 challenge proof")]
    ReauthRequiresChallengeProof,
    #[error("relay authentication extension version {actual} is unsupported; expected {expected}")]
    UnsupportedAuthExtension { actual: u8, expected: u8 },
    #[error("relay hello reserved bytes must be zero")]
    NonZeroReserved,
    #[error("relay server status code {0} is unknown")]
    InvalidServerStatus(u8),
    #[error("relay rejection code {0} is unknown")]
    InvalidRejectCode(u8),
    #[error("relay server status carries an invalid detail code")]
    InvalidStatusDetail,
    #[error("relay locator or invite has an invalid prefix")]
    InvalidLocatorPrefix,
    #[error("locator is not canonical base64url without padding")]
    InvalidLocatorEncoding,
    #[error("relay locator or invite length {actual} exceeds maximum {maximum}")]
    LocatorTooLong { actual: usize, maximum: usize },
    #[error("locator corruption checksum does not match")]
    ChecksumMismatch,
    #[error("route id must not be all zero")]
    ZeroRouteId,
    #[error("route capability must not be all zero")]
    ZeroRouteCapability,
    #[error("owner Noise static key must not be all zero")]
    ZeroOwnerNoiseStatic,
    #[error("caller device DH public key must not be all zero")]
    ZeroCallerDeviceDhPublic,
    #[error("route generation must be non-zero")]
    ZeroGeneration,
    #[error("locator not-before timestamp must be non-zero")]
    ZeroNotBefore,
    #[error("locator expiry must be later than its not-before timestamp")]
    InvalidExpiry,
    #[error("locator is not valid yet")]
    NotYetValid,
    #[error("locator has expired")]
    Expired,
    #[error("locator expiry is more than one hour in the future")]
    ExpiryTooFarFuture,
    #[error("locator validity period exceeds one hour")]
    ValidityWindowTooLong,
    #[error("locator signature verification failed")]
    SignatureVerificationFailed,
    #[error("{field} must be non-empty printable ASCII")]
    InvalidAsciiField { field: &'static str },
    #[error("{field} length {actual} exceeds maximum {maximum}")]
    AsciiFieldTooLong {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("locator signature must not be all zero")]
    ZeroSignature,
    #[error("possession proof length {actual} exceeds maximum {maximum}")]
    PossessionProofTooLong { actual: usize, maximum: usize },
    #[error("secure random generation is unavailable")]
    RandomUnavailable,
    #[error("pairing code is malformed or does not match")]
    InvalidPairingCode,
    #[error("sealed pairing invite is malformed")]
    InvalidSealedInvite,
    #[error("sealed pairing invite version {actual} is unsupported; expected {expected}")]
    UnsupportedSealedInviteVersion { actual: u8, expected: u8 },
    #[error("sealed pairing invite nonce must not be all zero")]
    ZeroSealedInviteNonce,
    #[error("sealed pairing invite key derivation failed")]
    SealedInviteKeyDerivationFailed,
    #[error("sealed pairing invite encryption failed")]
    SealedInviteEncryptionFailed,
    #[error("relay waiting advertisement header is malformed or out of range")]
    InvalidWaitingAdvertisement,
}
