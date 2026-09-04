use std::fmt;

use crate::locator::require_exact;
use crate::{
    CallerDeviceDhPublic, RelayChallengeProofV2, RelayLocatorV1, RelayLocatorVerifier,
    RelayProtocolError, RouteCapability, RouteMapKey, VerifiedRelayRoute, RELAY_PROTOCOL_VERSION,
};

pub const RELAY_AUTH_EXTENSION_VERSION: u8 = 1;
pub const MAX_POSSESSION_PROOF_BYTES: usize = 96;
pub const RELAY_CLIENT_HELLO_BYTES: usize = 501;
pub const RELAY_SERVER_STATUS_BYTES: usize = 3;

const CALLER_DH_OFFSET: usize = 4;
const CAPABILITY_OFFSET: usize = 36;
const PROOF_LEN_OFFSET: usize = 68;
const PROOF_OFFSET: usize = 69;
const LOCATOR_OFFSET: usize = 165;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RelayRole {
    Owner = 1,
    Guest = 2,
}

impl TryFrom<u8> for RelayRole {
    type Error = RelayProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Owner),
            2 => Ok(Self::Guest),
            other => Err(RelayProtocolError::InvalidRole(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RelayHelloAuthMode {
    SignedLocatorAndBearerTicketV1 = 1,
    ChallengeBoundX25519V2 = 2,
}

impl TryFrom<u8> for RelayHelloAuthMode {
    type Error = RelayProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::SignedLocatorAndBearerTicketV1),
            2 => Ok(Self::ChallengeBoundX25519V2),
            other => Err(RelayProtocolError::InvalidAuthMode(other)),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RelayAuthExtensionV1 {
    caller_device_dh_pub_x25519: CallerDeviceDhPublic,
    possession_proof: Vec<u8>,
}

impl RelayAuthExtensionV1 {
    pub fn new(
        caller_device_dh_pub_x25519: CallerDeviceDhPublic,
        possession_proof: Option<Vec<u8>>,
    ) -> Result<Self, RelayProtocolError> {
        let possession_proof = possession_proof.unwrap_or_default();
        if possession_proof.len() > MAX_POSSESSION_PROOF_BYTES {
            return Err(RelayProtocolError::PossessionProofTooLong {
                actual: possession_proof.len(),
                maximum: MAX_POSSESSION_PROOF_BYTES,
            });
        }
        Ok(Self {
            caller_device_dh_pub_x25519,
            possession_proof,
        })
    }

    pub fn without_possession_proof(caller_device_dh_pub_x25519: CallerDeviceDhPublic) -> Self {
        Self {
            caller_device_dh_pub_x25519,
            possession_proof: Vec::new(),
        }
    }

    pub const fn caller_device_dh_pub_x25519(&self) -> &CallerDeviceDhPublic {
        &self.caller_device_dh_pub_x25519
    }

    pub fn possession_proof(&self) -> Option<&[u8]> {
        (!self.possession_proof.is_empty()).then_some(self.possession_proof.as_slice())
    }
}

impl fmt::Debug for RelayAuthExtensionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelayAuthExtensionV1")
            .field(
                "caller_device_dh_pub_x25519",
                &self.caller_device_dh_pub_x25519,
            )
            .field("possession_proof", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RelayClientHello {
    role: RelayRole,
    auth_mode: RelayHelloAuthMode,
    auth: RelayAuthExtensionV1,
    capability: RouteCapability,
    locator: RelayLocatorV1,
}

impl RelayClientHello {
    pub fn new(role: RelayRole, route: &VerifiedRelayRoute, auth: RelayAuthExtensionV1) -> Self {
        Self {
            role,
            auth_mode: RelayHelloAuthMode::SignedLocatorAndBearerTicketV1,
            auth,
            capability: route.capability().clone(),
            locator: route.locator().locator().clone(),
        }
    }

    /// Builds an authentication-mode-2 hello.
    ///
    /// An absent proof is accepted only so callers can build the normalized
    /// pre-proof template used by [`crate::RelayChallengeProofV2::derive`].
    /// It is not an authenticated wire message; servers must reject a missing
    /// proof before pairing.
    pub fn new_challenge_bound_v2(
        role: RelayRole,
        route: &VerifiedRelayRoute,
        auth: RelayAuthExtensionV1,
    ) -> Result<Self, RelayProtocolError> {
        validate_challenge_bound_proof(auth.possession_proof())?;
        Ok(Self {
            role,
            auth_mode: RelayHelloAuthMode::ChallengeBoundX25519V2,
            auth,
            capability: route.capability().clone(),
            locator: route.locator().locator().clone(),
        })
    }

    pub const fn role(&self) -> RelayRole {
        self.role
    }

    pub const fn auth_mode(&self) -> RelayHelloAuthMode {
        self.auth_mode
    }

    pub fn auth_extension(&self) -> &RelayAuthExtensionV1 {
        &self.auth
    }

    pub fn locator(&self) -> &RelayLocatorV1 {
        &self.locator
    }

    pub fn expires_at_unix(&self) -> u64 {
        self.locator.claims().expires_at_unix()
    }

    pub fn encode(&self) -> [u8; RELAY_CLIENT_HELLO_BYTES] {
        let mut raw = [0_u8; RELAY_CLIENT_HELLO_BYTES];
        raw[0] = RELAY_PROTOCOL_VERSION;
        raw[1] = self.role as u8;
        raw[2] = self.auth_mode as u8;
        raw[3] = RELAY_AUTH_EXTENSION_VERSION;
        raw[CALLER_DH_OFFSET..CAPABILITY_OFFSET]
            .copy_from_slice(self.auth.caller_device_dh_pub_x25519.as_bytes());
        raw[CAPABILITY_OFFSET..PROOF_LEN_OFFSET].copy_from_slice(self.capability.secret_bytes());
        raw[PROOF_LEN_OFFSET] = self.auth.possession_proof.len() as u8;
        raw[PROOF_OFFSET..PROOF_OFFSET + self.auth.possession_proof.len()]
            .copy_from_slice(&self.auth.possession_proof);
        raw[PROOF_OFFSET + self.auth.possession_proof.len()..LOCATOR_OFFSET].fill(0);
        raw[LOCATOR_OFFSET..].copy_from_slice(&self.locator.to_binary());
        raw
    }

    pub fn decode(raw: &[u8]) -> Result<Self, RelayProtocolError> {
        require_exact("relay client hello", raw.len(), RELAY_CLIENT_HELLO_BYTES)?;
        if raw[0] != RELAY_PROTOCOL_VERSION {
            return Err(RelayProtocolError::UnsupportedVersion {
                actual: raw[0],
                expected: RELAY_PROTOCOL_VERSION,
            });
        }
        let role = RelayRole::try_from(raw[1])?;
        let auth_mode = RelayHelloAuthMode::try_from(raw[2])?;
        if raw[3] != RELAY_AUTH_EXTENSION_VERSION {
            return Err(RelayProtocolError::UnsupportedAuthExtension {
                actual: raw[3],
                expected: RELAY_AUTH_EXTENSION_VERSION,
            });
        }
        let caller_device_dh_pub_x25519 =
            CallerDeviceDhPublic::new(read_array(raw, CALLER_DH_OFFSET)?)?;
        let capability = RouteCapability::new(read_array(raw, CAPABILITY_OFFSET)?)?;
        let proof_len = usize::from(raw[PROOF_LEN_OFFSET]);
        if proof_len > MAX_POSSESSION_PROOF_BYTES {
            return Err(RelayProtocolError::PossessionProofTooLong {
                actual: proof_len,
                maximum: MAX_POSSESSION_PROOF_BYTES,
            });
        }
        if raw[PROOF_OFFSET + proof_len..LOCATOR_OFFSET]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(RelayProtocolError::NonZeroReserved);
        }
        let possession_proof = raw[PROOF_OFFSET..PROOF_OFFSET + proof_len].to_vec();
        if auth_mode == RelayHelloAuthMode::ChallengeBoundX25519V2 {
            validate_challenge_bound_proof(
                (!possession_proof.is_empty()).then_some(possession_proof.as_slice()),
            )?;
        }
        let locator = RelayLocatorV1::decode_binary(&raw[LOCATOR_OFFSET..])?;
        Ok(Self {
            role,
            auth_mode,
            auth: RelayAuthExtensionV1 {
                caller_device_dh_pub_x25519,
                possession_proof,
            },
            capability,
            locator,
        })
    }

    /// Verifies only the signed locator and its time window.
    ///
    /// The server must still validate its external authorization bearer ticket,
    /// bind that ticket to `caller_device_dh_pub_x25519`, and validate any
    /// possession proof before admitting or pairing this hello.
    pub fn verify_locator(
        &self,
        verifier: &impl RelayLocatorVerifier,
        now_unix: u64,
    ) -> Result<VerifiedRelayClientHello, RelayProtocolError> {
        let verified_locator = self.locator.verify(verifier, now_unix)?;
        Ok(VerifiedRelayClientHello {
            role: self.role,
            auth_mode: self.auth_mode,
            auth: self.auth.clone(),
            route: VerifiedRelayRoute::new(verified_locator, self.capability.clone()),
        })
    }

    pub(crate) fn challenge_proof_binding_bytes(&self) -> [u8; RELAY_CLIENT_HELLO_BYTES] {
        let mut raw = self.encode();
        raw[PROOF_LEN_OFFSET..LOCATOR_OFFSET].fill(0);
        raw
    }
}

impl fmt::Debug for RelayClientHello {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelayClientHello")
            .field("role", &self.role)
            .field("auth_mode", &self.auth_mode)
            .field("auth", &self.auth)
            .field("capability", &self.capability)
            .field("locator", &self.locator)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedRelayClientHello {
    role: RelayRole,
    auth_mode: RelayHelloAuthMode,
    auth: RelayAuthExtensionV1,
    route: VerifiedRelayRoute,
}

impl VerifiedRelayClientHello {
    pub const fn role(&self) -> RelayRole {
        self.role
    }

    pub const fn auth_mode(&self) -> RelayHelloAuthMode {
        self.auth_mode
    }

    pub fn auth_extension(&self) -> &RelayAuthExtensionV1 {
        &self.auth
    }

    pub fn route(&self) -> &VerifiedRelayRoute {
        &self.route
    }

    pub fn route_map_key(&self) -> RouteMapKey {
        self.route.route_map_key()
    }
}

impl fmt::Debug for VerifiedRelayClientHello {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedRelayClientHello")
            .field("role", &self.role)
            .field("auth_mode", &self.auth_mode)
            .field("auth", &self.auth)
            .field("route", &self.route)
            .finish()
    }
}

fn validate_challenge_bound_proof(proof: Option<&[u8]>) -> Result<(), RelayProtocolError> {
    if let Some(proof) = proof {
        RelayChallengeProofV2::decode(proof)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RelayRejectCode {
    MalformedHello = 1,
    UnsupportedVersion = 2,
    AuthenticationRequired = 3,
    AuthenticationFailed = 4,
    LocatorNotYetValid = 5,
    LocatorExpired = 6,
    ExpiryTooFarFuture = 7,
    UnknownRoute = 8,
    RoleConflict = 9,
    Capacity = 10,
    RateLimited = 11,
    PairingTimeout = 12,
    RelayUnavailable = 13,
    Internal = 14,
}

impl RelayRejectCode {
    /// Stable, credential-free label for this reject reason.
    ///
    /// Used for relay operational logging and as the machine-readable half of
    /// [`RelayRejectCode::close_reason`].
    pub const fn label(self) -> &'static str {
        match self {
            Self::MalformedHello => "malformed-hello",
            Self::UnsupportedVersion => "unsupported-version",
            Self::AuthenticationRequired => "authentication-required",
            Self::AuthenticationFailed => "authentication-failed",
            Self::LocatorNotYetValid => "locator-not-yet-valid",
            Self::LocatorExpired => "locator-expired",
            Self::ExpiryTooFarFuture => "expiry-too-far-future",
            Self::UnknownRoute => "unknown-route",
            Self::RoleConflict => "role-conflict",
            Self::Capacity => "capacity",
            Self::RateLimited => "rate-limited",
            Self::PairingTimeout => "pairing-timeout",
            Self::RelayUnavailable => "relay-unavailable",
            Self::Internal => "internal",
        }
    }

    /// The WebSocket close-frame reason that carries this reject code.
    ///
    /// The relay writes the [`RelayServerStatus::Rejected`] status frame and
    /// the close frame back to back. Repeating the reason inside the close
    /// frame means a peer that only observes the closing handshake — an
    /// intermediary that forwards the close but drops a trailing data frame,
    /// or a client that was already draining towards close — can still tell a
    /// deliberate rejection from an unexplained transport fault.
    ///
    /// The payload of a close frame is capped at 125 bytes, two of which the
    /// status code takes, so every reason must stay under that.
    pub const fn close_reason(self) -> &'static str {
        match self {
            Self::MalformedHello => "relay-reject:malformed-hello",
            Self::UnsupportedVersion => "relay-reject:unsupported-version",
            Self::AuthenticationRequired => "relay-reject:authentication-required",
            Self::AuthenticationFailed => "relay-reject:authentication-failed",
            Self::LocatorNotYetValid => "relay-reject:locator-not-yet-valid",
            Self::LocatorExpired => "relay-reject:locator-expired",
            Self::ExpiryTooFarFuture => "relay-reject:expiry-too-far-future",
            Self::UnknownRoute => "relay-reject:unknown-route",
            Self::RoleConflict => "relay-reject:role-conflict",
            Self::Capacity => "relay-reject:capacity",
            Self::RateLimited => "relay-reject:rate-limited",
            Self::PairingTimeout => "relay-reject:pairing-timeout",
            Self::RelayUnavailable => "relay-reject:relay-unavailable",
            Self::Internal => "relay-reject:internal",
        }
    }

    /// Recover a reject code from a relay close-frame reason.
    ///
    /// Returns `None` for any reason the relay did not write, so an ordinary
    /// close never masquerades as a rejection.
    pub fn from_close_reason(reason: &str) -> Option<Self> {
        let label = reason.strip_prefix(RELAY_REJECT_CLOSE_PREFIX)?;
        RELAY_REJECT_CODES
            .into_iter()
            .find(|code| code.label() == label)
    }
}

/// Prefix of every relay close-frame reason that carries a reject code.
pub const RELAY_REJECT_CLOSE_PREFIX: &str = "relay-reject:";

/// Every reject code, for exhaustive decoding and tests.
pub const RELAY_REJECT_CODES: [RelayRejectCode; 14] = [
    RelayRejectCode::MalformedHello,
    RelayRejectCode::UnsupportedVersion,
    RelayRejectCode::AuthenticationRequired,
    RelayRejectCode::AuthenticationFailed,
    RelayRejectCode::LocatorNotYetValid,
    RelayRejectCode::LocatorExpired,
    RelayRejectCode::ExpiryTooFarFuture,
    RelayRejectCode::UnknownRoute,
    RelayRejectCode::RoleConflict,
    RelayRejectCode::Capacity,
    RelayRejectCode::RateLimited,
    RelayRejectCode::PairingTimeout,
    RelayRejectCode::RelayUnavailable,
    RelayRejectCode::Internal,
];

impl TryFrom<u8> for RelayRejectCode {
    type Error = RelayProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::MalformedHello),
            2 => Ok(Self::UnsupportedVersion),
            3 => Ok(Self::AuthenticationRequired),
            4 => Ok(Self::AuthenticationFailed),
            5 => Ok(Self::LocatorNotYetValid),
            6 => Ok(Self::LocatorExpired),
            7 => Ok(Self::ExpiryTooFarFuture),
            8 => Ok(Self::UnknownRoute),
            9 => Ok(Self::RoleConflict),
            10 => Ok(Self::Capacity),
            11 => Ok(Self::RateLimited),
            12 => Ok(Self::PairingTimeout),
            13 => Ok(Self::RelayUnavailable),
            14 => Ok(Self::Internal),
            other => Err(RelayProtocolError::InvalidRejectCode(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayServerStatus {
    Ready,
    Paired,
    Rejected(RelayRejectCode),
}

impl RelayServerStatus {
    pub fn encode(self) -> [u8; RELAY_SERVER_STATUS_BYTES] {
        match self {
            Self::Ready => [RELAY_PROTOCOL_VERSION, 1, 0],
            Self::Paired => [RELAY_PROTOCOL_VERSION, 2, 0],
            Self::Rejected(code) => [RELAY_PROTOCOL_VERSION, 3, code as u8],
        }
    }

    pub fn decode(raw: &[u8]) -> Result<Self, RelayProtocolError> {
        require_exact("relay server status", raw.len(), RELAY_SERVER_STATUS_BYTES)?;
        if raw[0] != RELAY_PROTOCOL_VERSION {
            return Err(RelayProtocolError::UnsupportedVersion {
                actual: raw[0],
                expected: RELAY_PROTOCOL_VERSION,
            });
        }
        match (raw[1], raw[2]) {
            (1, 0) => Ok(Self::Ready),
            (2, 0) => Ok(Self::Paired),
            (3, code) if code != 0 => Ok(Self::Rejected(RelayRejectCode::try_from(code)?)),
            (1..=3, _) => Err(RelayProtocolError::InvalidStatusDetail),
            (status, _) => Err(RelayProtocolError::InvalidServerStatus(status)),
        }
    }
}

fn read_array<const N: usize>(raw: &[u8], offset: usize) -> Result<[u8; N], RelayProtocolError> {
    raw.get(offset..offset + N)
        .and_then(|slice| slice.try_into().ok())
        .ok_or(RelayProtocolError::Truncated {
            context: "relay client hello",
            expected: offset + N,
            actual: raw.len(),
        })
}
