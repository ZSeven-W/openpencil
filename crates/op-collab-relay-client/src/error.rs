use std::io;

use op_collab_relay_protocol::RelayRejectCode;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelayBridgePhase {
    Starting,
    Waiting,
    Active,
    Degraded,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelayBridgeStatus {
    pub phase: RelayBridgePhase,
    pub waiting_lanes: usize,
    pub active_tunnels: usize,
    pub last_error: Option<RelayFailureKind>,
    /// How many lanes the relay retired with its own pairing timeout.
    ///
    /// Deliberately not folded into `last_error`: the lane simply never paired,
    /// so it is a scheduled recycle as far as the pool is concerned and must
    /// not advertise a broken relay. It is still worth counting, because a
    /// non-zero value means the relay's waiting window expired before the
    /// client's `RelayLimits::owner_pair` recycle budget — the pairing contract
    /// inverted, which is exactly the condition that leaves a room unjoinable.
    pub relay_pairing_timeouts: u32,
}

impl RelayBridgeStatus {
    pub(crate) const fn starting(waiting_lanes: usize) -> Self {
        Self {
            phase: RelayBridgePhase::Starting,
            waiting_lanes,
            active_tunnels: 0,
            last_error: None,
            relay_pairing_timeouts: 0,
        }
    }

    pub(crate) const fn stopped() -> Self {
        Self {
            phase: RelayBridgePhase::Stopped,
            waiting_lanes: 0,
            active_tunnels: 0,
            last_error: None,
            relay_pairing_timeouts: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelayFailureKind {
    Authentication,
    Connect,
    ConnectTimeout,
    HelloTimeout,
    PairTimeout,
    Rejected(RelayRejectCode),
    /// The relay retired this lane with its own pairing timeout.
    ///
    /// Distinct from [`RelayFailureKind::Rejected`] so a relay that closed an
    /// unpaired lane on schedule is never confused with a real rejection, and
    /// distinct from [`RelayFailureKind::Protocol`], which is where it used to
    /// land when the reject frame was lost to a connection reset.
    RejectedPairingTimeout,
    Protocol,
    TextFrame,
    /// The relay sent a reauthentication challenge sooner than the protocol's
    /// slowest possible legitimate rotation cadence allows.
    ReauthTooFrequent,
    /// The relay exhausted this connection's server-initiated
    /// reauthentication budget.
    ReauthBudgetExhausted,
    BinaryFrameTooLarge,
    IdleTimeout,
    LifetimeExceeded,
    ByteLimitExceeded,
    LocalIo,
    RelayIo,
    Closed,
}

#[derive(Debug, Error)]
pub enum RelayClientError {
    #[error("owner relay lane count must be between 1 and {max}")]
    InvalidLaneCount { max: usize },
    #[error("local collaboration socket must be a nonzero loopback address")]
    InvalidLocalSocket,
    #[error("unauthenticated development relay must use a numeric loopback endpoint")]
    DevelopmentEndpointNotLoopback,
    #[error("relay did not accept an owner lane before the readiness deadline")]
    ReadyTimeout,
    #[error("relay owner bridge stopped before a lane became ready")]
    StoppedBeforeReady,
    #[error("relay guest bridge did not pair before the readiness deadline")]
    PairedTimeout,
    #[error("relay guest bridge stopped before its tunnel paired")]
    StoppedBeforePaired,
    #[error("relay guest bridge failed before pairing: {kind:?}")]
    GuestPairingFailed { kind: RelayFailureKind },
    #[error("failed to bind the guest loopback bridge")]
    BindLoopback { kind: io::ErrorKind },
    #[error("failed to read the guest loopback bridge address")]
    ReadLocalAddress { kind: io::ErrorKind },
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RelayStopError {
    #[error("relay bridge did not stop before its deadline")]
    Timeout,
    #[error("relay bridge task failed")]
    TaskFailed,
}

impl RelayFailureKind {
    /// Classify a reject code the relay delivered, by status frame or by the
    /// reason it repeated in the close frame.
    pub(crate) const fn from_reject(code: RelayRejectCode) -> Self {
        match code {
            RelayRejectCode::PairingTimeout => Self::RejectedPairingTimeout,
            _ => Self::Rejected(code),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TunnelError {
    Cancelled,
    Failure(RelayFailureKind),
}

impl TunnelError {
    pub(crate) fn failure_kind(self) -> Option<RelayFailureKind> {
        match self {
            Self::Cancelled => None,
            Self::Failure(kind) => Some(kind),
        }
    }
}
