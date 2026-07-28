use std::fmt;

/// Closed, log-safe failures returned by a JWKS transport adapter.
///
/// Implementations must not carry response bodies, URLs containing credentials,
/// or other remote-controlled text into this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollabJwksFetchError {
    Cancelled,
    Unavailable,
    RejectedResponse,
    ResponseTooLarge,
}

impl fmt::Display for CollabJwksFetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Cancelled => "collaboration key request was cancelled",
            Self::Unavailable => "collaboration key endpoint is unavailable",
            Self::RejectedResponse => "collaboration key endpoint response was rejected",
            Self::ResponseTooLarge => "collaboration key endpoint response was too large",
        })
    }
}

impl std::error::Error for CollabJwksFetchError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollabJwkErrorKind {
    InvalidKeyId,
    WrongKeyType,
    WrongCurve,
    WrongAlgorithm,
    WrongUse,
    WrongKeyOperations,
    InvalidPublicKey,
}

/// Invalid offline-signed regional union-policy responses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CollabUnionPolicyError {
    #[error("collaboration union policy body is empty or too large")]
    InvalidBodySize,
    #[error("collaboration union policy JSON is malformed")]
    MalformedJson,
    #[error("collaboration union policy profile is invalid")]
    InvalidProfile,
    #[error("collaboration union policy issuer does not match the pinned issuer")]
    InvalidIssuer,
    #[error("collaboration union policy region set is invalid")]
    InvalidRegions,
    #[error("collaboration union policy key set is invalid")]
    InvalidKeys,
    #[error("collaboration union policy key lifecycle is invalid")]
    InvalidKeyLifecycle,
    #[error("collaboration union policy rotation phase is invalid")]
    InvalidRotationPhase,
    #[error("collaboration union policy signature is invalid")]
    InvalidSignature,
    #[error("collaboration union policy is not active")]
    Inactive,
    #[error("collaboration union policy generation rolled back")]
    GenerationRollback,
    #[error("collaboration union policy changed without increasing its generation")]
    GenerationRewrite,
}

impl CollabJwkErrorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidKeyId => "invalid_key_id",
            Self::WrongKeyType => "wrong_key_type",
            Self::WrongCurve => "wrong_curve",
            Self::WrongAlgorithm => "wrong_algorithm",
            Self::WrongUse => "wrong_use",
            Self::WrongKeyOperations => "wrong_key_operations",
            Self::InvalidPublicKey => "invalid_public_key",
        }
    }
}

/// Bounded keyset parsing and cache failures.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CollabJwksError {
    #[error("collaboration keyset body is empty or exceeds {maximum} bytes")]
    InvalidBodySize { maximum: usize },
    #[error("collaboration keyset JSON is malformed")]
    MalformedJson,
    #[error("collaboration keyset must contain at least one key")]
    EmptyKeyset,
    #[error("collaboration keyset contains more than {maximum} keys")]
    TooManyKeys { maximum: usize },
    #[error("collaboration keyset contains a duplicate key id")]
    DuplicateKeyId,
    #[error("collaboration key {index} is invalid: {}", kind.as_str())]
    InvalidKey {
        index: usize,
        kind: CollabJwkErrorKind,
    },
    #[error("collaboration keyset ETag is invalid or exceeds {maximum} bytes")]
    InvalidEtag { maximum: usize },
    #[error("collaboration key refresh was throttled")]
    RefreshThrottled,
    #[error("collaboration key cache is unavailable")]
    CacheUnavailable,
    #[error("collaboration key endpoint returned not-modified without a cached keyset")]
    NotModifiedWithoutCache,
    #[error("collaboration signing key is unknown")]
    UnknownKey,
    #[error(transparent)]
    Policy(#[from] CollabUnionPolicyError),
    #[error(transparent)]
    Fetch(#[from] CollabJwksFetchError),
}

/// Invalid verifier trust-root or resource-limit configuration.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CollabVerifierConfigError {
    #[error("collaboration issuer must be a bounded HTTPS origin")]
    InvalidIssuer,
    #[error("collaboration JWKS endpoint must be a bounded HTTPS URL")]
    InvalidJwksEndpoint,
    #[error("collaboration policy endpoint must be a bounded HTTPS URL")]
    InvalidPolicyEndpoint,
    #[error("collaboration JWKS and policy endpoints cannot both be selected")]
    ConflictingKeyEndpoints,
    #[error("collaboration JWKS cache limits are invalid")]
    InvalidCacheLimits,
}

/// Ticket parsing, signature, claim, and channel-binding failures.
///
/// Variants intentionally contain no ticket bytes or identity values so they
/// are safe to surface in host diagnostics.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CollabVerifyError {
    #[error("collaboration ticket verification was cancelled")]
    Cancelled,
    #[error("collaboration ticket is empty or exceeds {maximum} bytes")]
    InvalidTicketSize { maximum: usize },
    #[error("collaboration ticket compact serialization is malformed")]
    MalformedCompactJws,
    #[error("collaboration ticket {part} segment exceeds {maximum} bytes")]
    SegmentTooLarge { part: &'static str, maximum: usize },
    #[error("collaboration ticket {part} segment is not canonical base64url")]
    InvalidBase64 { part: &'static str },
    #[error("collaboration ticket {part} JSON is malformed")]
    MalformedJson { part: &'static str },
    #[error("collaboration ticket uses an unsupported signing algorithm")]
    WrongAlgorithm,
    #[error("collaboration ticket has the wrong explicit type")]
    WrongType,
    #[error("collaboration ticket key id is invalid")]
    InvalidKeyId,
    #[error("collaboration ticket signature is invalid")]
    InvalidSignature,
    #[error("collaboration ticket issuer is invalid")]
    InvalidIssuer,
    #[error("collaboration ticket audience is invalid")]
    InvalidAudience,
    #[error("collaboration ticket version is unsupported")]
    InvalidVersion,
    #[error("collaboration ticket scope is invalid")]
    InvalidScope,
    #[error("collaboration ticket subject is invalid")]
    InvalidSubject,
    #[error("collaboration ticket device id is invalid")]
    InvalidDeviceId,
    #[error("collaboration ticket channel binding is invalid")]
    InvalidChannelBinding,
    #[error("collaboration ticket is bound to a different channel")]
    ChannelBindingMismatch,
    #[error("collaboration ticket id is invalid")]
    InvalidTicketId,
    #[error("collaboration ticket display name is invalid")]
    InvalidDisplayName,
    #[error("collaboration ticket avatar URL is invalid")]
    InvalidAvatarUrl,
    #[error("collaboration ticket timestamps are invalid")]
    InvalidTimestamps,
    #[error("collaboration ticket is not valid yet")]
    NotYetValid,
    #[error("collaboration ticket has expired")]
    Expired,
    #[error("collaboration ticket lifetime exceeds the verifier policy")]
    LifetimeTooLong,
    #[error("collaboration ticket expiry cannot be represented by the host")]
    ExpiryOverflow,
    #[error(transparent)]
    Jwks(#[from] CollabJwksError),
}
