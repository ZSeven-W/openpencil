//! The relay pairing-window contract, shared by both ends of the tunnel.
//!
//! An unpaired peer occupies a slot in the relay's waiting queue. Two
//! independent clocks decide when that slot is released:
//!
//! * the relay's own waiting window (`RelayConfig::waiting_timeout`), after
//!   which it rejects the peer with [`crate::RelayRejectCode::PairingTimeout`];
//! * the owner client's lane recycle budget
//!   (`op_collab_relay_client::RelayLimits::owner_pair`), after which the
//!   client retires the lane and dials a fresh one.
//!
//! The client budget must expire first. If the relay wins the race, an owner
//! lane learns about the close late — or, behind a NAT that silently reaps the
//! idle flow, not at all — and the owner is absent from the waiting queue while
//! it re-dials. A guest that registers in that window has no counterpart to
//! pair with, which is exactly how a room becomes unjoinable while both ends
//! believe they are healthy.
//!
//! Both constants live here so neither side can be moved without the other's
//! bound failing to compile or its configuration failing to validate.

use std::{fmt, time::Duration};

use crate::RelayProtocolError;

/// The owner lane recycle budget shipped by the client, in seconds.
///
/// `op_collab_relay_client::RelayLimits::owner_pair` is built from this value.
pub const RELAY_OWNER_LANE_RECYCLE_SECS: u64 = 45;

/// Headroom the relay's waiting window must keep above the client's recycle
/// budget so the two clocks never race on a slow or loaded link.
pub const RELAY_WAITING_HEADROOM_SECS: u64 = 10;

/// Smallest relay waiting window that preserves the owner-lane contract.
///
/// A relay configured below this retires owner lanes before the client
/// recycles them, which empties the waiting queue for the whole re-dial.
pub const MIN_RELAY_WAITING_TIMEOUT_SECS: u64 =
    RELAY_OWNER_LANE_RECYCLE_SECS + RELAY_WAITING_HEADROOM_SECS;

/// Default relay waiting window.
pub const DEFAULT_RELAY_WAITING_TIMEOUT_SECS: u64 = 60;

/// Largest relay waiting window an operator may configure.
///
/// An unpaired peer holds a pending-admission slot and a per-source quota slot
/// for the whole window, so an unbounded value turns a stalled dialler into a
/// capacity leak.
pub const MAX_RELAY_WAITING_TIMEOUT_SECS: u64 = 15 * 60;

const _: () = assert!(RELAY_OWNER_LANE_RECYCLE_SECS < MIN_RELAY_WAITING_TIMEOUT_SECS);
const _: () = assert!(MIN_RELAY_WAITING_TIMEOUT_SECS <= DEFAULT_RELAY_WAITING_TIMEOUT_SECS);
const _: () = assert!(DEFAULT_RELAY_WAITING_TIMEOUT_SECS <= MAX_RELAY_WAITING_TIMEOUT_SECS);

/// HTTP response header carrying the relay's waiting-window capability.
///
/// Advertised on the WebSocket upgrade response, next to
/// [`crate::RELAY_CHALLENGE_HEADER_NAME`], because the upgrade response is the
/// only place a capability can travel without disturbing the fixed three-byte
/// [`crate::RelayServerStatus`] wire format. A client that does not know the
/// header ignores it; a relay that does not send it leaves the client on its
/// compiled-in fallback. Neither end may treat it as required.
pub const RELAY_WAITING_HEADER_NAME: &str = "openpencil-relay-waiting";

/// Canonical version tag of the waiting advertisement.
pub const RELAY_WAITING_HEADER_PREFIX: &str = "oprw1 ";

/// Upper bound on the header value, so a hostile relay cannot inflate the
/// upgrade response.
pub const MAX_RELAY_WAITING_HEADER_BYTES: usize = 40;

/// Slack a client subtracts from an advertised window before trusting it.
///
/// Covers clock skew, a slow link, and the round trip the recycle itself
/// costs. A client that consumed the advertised window exactly would race the
/// relay for the retirement it is trying to win.
pub const RELAY_WAITING_SAFETY_MARGIN_SECS: u64 = 10;

/// Floor for a derived lane budget.
///
/// A degenerate advertisement (a window at or under the safety margin) must
/// still leave a usable budget rather than collapsing into a reconnect spin.
pub const MIN_DERIVED_OWNER_LANE_BUDGET_SECS: u64 = 5;

/// Largest window a relay may advertise: one day, comfortably above the
/// longest tunnel lifetime and far below anything that could overflow.
pub const MAX_ADVERTISED_WAITING_SECS: u64 = 24 * 60 * 60;

/// What the relay will do with an un-paired peer's waiting slot.
///
/// `renewable` means the relay renews the slot for as long as the peer answers
/// the relay's WebSocket pings — the authenticated waiting lease — so `window`
/// is the ceiling on one lease, not a countdown from registration.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RelayWaitingAdvertisementV1 {
    window_secs: u64,
    renewable: bool,
}

impl RelayWaitingAdvertisementV1 {
    pub fn new(window_secs: u64, renewable: bool) -> Result<Self, RelayProtocolError> {
        if window_secs == 0 || window_secs > MAX_ADVERTISED_WAITING_SECS {
            return Err(RelayProtocolError::InvalidWaitingAdvertisement);
        }
        Ok(Self {
            window_secs,
            renewable,
        })
    }

    pub const fn window_secs(self) -> u64 {
        self.window_secs
    }

    pub const fn renewable(self) -> bool {
        self.renewable
    }

    pub fn encode_header(self) -> String {
        format!(
            "{RELAY_WAITING_HEADER_PREFIX}window={} renew={}",
            self.window_secs,
            u8::from(self.renewable)
        )
    }

    pub fn decode_header(value: &str) -> Result<Self, RelayProtocolError> {
        if value.len() > MAX_RELAY_WAITING_HEADER_BYTES {
            return Err(RelayProtocolError::InvalidWaitingAdvertisement);
        }
        let body = value
            .strip_prefix(RELAY_WAITING_HEADER_PREFIX)
            .ok_or(RelayProtocolError::InvalidWaitingAdvertisement)?;
        let mut fields = body.split(' ');
        let window = fields
            .next()
            .and_then(|field| field.strip_prefix("window="))
            .ok_or(RelayProtocolError::InvalidWaitingAdvertisement)?;
        let renew = fields
            .next()
            .and_then(|field| field.strip_prefix("renew="))
            .ok_or(RelayProtocolError::InvalidWaitingAdvertisement)?;
        if fields.next().is_some() {
            return Err(RelayProtocolError::InvalidWaitingAdvertisement);
        }
        let window_secs = window
            .parse()
            .map_err(|_| RelayProtocolError::InvalidWaitingAdvertisement)?;
        let renewable = match renew {
            "0" => false,
            "1" => true,
            _ => return Err(RelayProtocolError::InvalidWaitingAdvertisement),
        };
        Self::new(window_secs, renewable)
    }

    /// Derive a lane's waiting budget from this advertisement.
    ///
    /// Deliberately never the advertised value itself. The client keeps its own
    /// compiled-in ceiling and only ever narrows towards the relay:
    /// `min(advertised - margin, cap)`. A relay that advertises a renewable
    /// lease unlocks the larger `renewable_cap`, because the relay will hold
    /// the slot for as long as the lane answers pings; without a lease the lane
    /// stays on the short `unrenewable_cap` and recycles itself.
    pub fn derive_lane_budget(
        self,
        unrenewable_cap: Duration,
        renewable_cap: Duration,
    ) -> Duration {
        let cap = if self.renewable {
            renewable_cap.max(unrenewable_cap)
        } else {
            unrenewable_cap
        };
        let floor = Duration::from_secs(MIN_DERIVED_OWNER_LANE_BUDGET_SECS).min(cap);
        Duration::from_secs(self.window_secs)
            .saturating_sub(Duration::from_secs(RELAY_WAITING_SAFETY_MARGIN_SECS))
            .min(cap)
            .max(floor)
    }
}

impl fmt::Debug for RelayWaitingAdvertisementV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelayWaitingAdvertisementV1")
            .field("window_secs", &self.window_secs)
            .field("renewable", &self.renewable)
            .finish()
    }
}
