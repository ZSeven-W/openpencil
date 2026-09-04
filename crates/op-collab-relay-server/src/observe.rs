//! Credential-free operational logging for the relay.
//!
//! Until this module existed the relay logged nothing between "listening" and
//! "shutdown requested", which left a pairing outage undiagnosable from the
//! server side. Two levels are emitted, deliberately:
//!
//! * **debug** — one line per connection lifecycle transition (registered /
//!   paired / rejected / closed). An operator flips
//!   `OPENPENCIL_COLLAB_RELAY_LOG_LEVEL=debug` to get the full per-connection
//!   trail; at the default level it costs nothing.
//! * **info** — a periodic census of what happened in the last window plus the
//!   current waiting-queue depth. One line per minute at most, and none at all
//!   while the relay is idle, so it is safe to leave on in production and is
//!   enough on its own to see "owners are registering but nothing pairs".
//!
//! Nothing here may carry a credential. Routes appear only as [`RouteRef`], a
//! one-way 24-hex digest that mirrors the `security_event=` reference shape the
//! SSO backend audits with, and connection identity is a process-local counter.

use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use op_collab_relay_protocol::{RelayRejectCode, RelayRole, RouteMapKey};

use crate::close::RelayCloseReason;

/// Domain separation for the route logging reference.
///
/// A [`RouteMapKey`] is itself derived from the route capability, so it is
/// secret-derived material and must never be logged. Re-deriving under a
/// distinct context yields a stable correlation handle that cannot be walked
/// back to the capability.
const ROUTE_LOG_REF_CONTEXT: &str = "openpencil/op-collab-relay-server/route-log-ref/v1";

/// Bytes of digest kept for a route reference: 12 bytes rendered as 24 hex
/// characters, matching the SSO audit reference width.
const ROUTE_LOG_REF_BYTES: usize = 12;

/// How often the census line is emitted while the relay has something to say.
const CENSUS_INTERVAL: Duration = Duration::from_secs(60);

/// Opaque, one-way route handle safe to write to logs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct RouteRef([u8; ROUTE_LOG_REF_BYTES]);

impl RouteRef {
    pub(crate) fn derive(route: &RouteMapKey) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(ROUTE_LOG_REF_CONTEXT);
        hasher.update(route.as_bytes());
        let digest = hasher.finalize();
        let mut bytes = [0_u8; ROUTE_LOG_REF_BYTES];
        bytes.copy_from_slice(&digest.as_bytes()[..ROUTE_LOG_REF_BYTES]);
        Self(bytes)
    }
}

impl fmt::Display for RouteRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for RouteRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

/// Renders an optional field as `-`, the SSO audit convention for "absent".
struct Absent<T>(Option<T>);

impl<T: fmt::Display> fmt::Display for Absent<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(value) => fmt::Display::fmt(value, formatter),
            None => formatter.write_str("-"),
        }
    }
}

fn role_label(role: RelayRole) -> &'static str {
    match role {
        RelayRole::Owner => "owner",
        RelayRole::Guest => "guest",
    }
}

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

/// Per-connection logging context.
///
/// Carries no credential: the id is a process-local counter and the route is
/// only ever the one-way [`RouteRef`].
pub(crate) struct ConnectionTrace<'a> {
    id: u64,
    started_at: Instant,
    role: Option<RelayRole>,
    route: Option<RouteRef>,
    census: &'a RelayCensus,
}

impl<'a> ConnectionTrace<'a> {
    pub(crate) fn new(census: &'a RelayCensus) -> Self {
        Self {
            id: NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed),
            started_at: Instant::now(),
            role: None,
            route: None,
            census,
        }
    }

    /// Attach the authenticated role and route once the hello has been
    /// verified. Never called with unauthenticated client-supplied material.
    pub(crate) fn identify(&mut self, role: RelayRole, route: &RouteMapKey) {
        self.role = Some(role);
        self.route = Some(RouteRef::derive(route));
    }

    fn elapsed_ms(&self) -> u128 {
        self.started_at.elapsed().as_millis()
    }

    fn role_field(&self) -> Absent<&'static str> {
        Absent(self.role.map(role_label))
    }

    /// The relay took the socket and started the handshake.
    ///
    /// Emitted before authentication, so it carries no role or route yet: its
    /// job is to give every later line — and every silent disappearance — a
    /// connection id to hang off.
    pub(crate) fn accepted(&self) {
        self.census.accepted.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(
            relay_event = "accepted",
            conn = self.id,
            "relay accepted a socket"
        );
    }

    /// The connection died before it could be authenticated and registered.
    ///
    /// Without this a connection that fails during the upgrade or the hello
    /// simply vanishes: nothing else on the server ever mentions it again.
    pub(crate) fn handshake_failed(&self, stage: HandshakeStage) {
        self.census.handshake_failed.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(
            relay_event = "handshake-failed",
            conn = self.id,
            stage = stage.label(),
            elapsed_ms = self.elapsed_ms(),
            "relay connection ended before authentication"
        );
    }

    pub(crate) fn registered(&self, waiting_on_route: usize) {
        self.census.registered.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(
            relay_event = "registered",
            conn = self.id,
            role = %self.role_field(),
            route_ref = %Absent(self.route),
            waiting_on_route,
            "relay accepted an unpaired peer"
        );
    }

    pub(crate) fn paired(&self) {
        self.census.paired.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(
            relay_event = "paired",
            conn = self.id,
            role = %self.role_field(),
            route_ref = %Absent(self.route),
            waited_ms = self.elapsed_ms(),
            "relay paired a route"
        );
    }

    pub(crate) fn rejected(&self, code: RelayRejectCode) {
        self.census.record_reject(code);
        tracing::debug!(
            relay_event = "rejected",
            conn = self.id,
            role = %self.role_field(),
            route_ref = %Absent(self.route),
            reason = code.label(),
            elapsed_ms = self.elapsed_ms(),
            "relay rejected a peer"
        );
    }

    pub(crate) fn closed(&self, reason: RelayCloseReason) {
        self.census.record_close(reason);
        tracing::debug!(
            relay_event = "closed",
            conn = self.id,
            role = %self.role_field(),
            route_ref = %Absent(self.route),
            reason = reason.label(),
            elapsed_ms = self.elapsed_ms(),
            "relay closed a connection"
        );
    }
}

/// Where a connection died before it was ever authenticated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandshakeStage {
    /// No authentication concurrency permit was available.
    AuthConcurrency,
    /// The authenticator could not produce a challenge key.
    ChallengeKey,
    /// Challenge generation failed.
    ChallengeGeneration,
    /// The WebSocket upgrade itself failed or was refused.
    Upgrade,
    /// The upgrade did not complete inside the handshake deadline.
    UpgradeTimeout,
    /// The peer never sent a hello, or the socket ended first.
    Hello,
}

impl HandshakeStage {
    const fn label(self) -> &'static str {
        match self {
            Self::AuthConcurrency => "auth-concurrency",
            Self::ChallengeKey => "challenge-key",
            Self::ChallengeGeneration => "challenge-generation",
            Self::Upgrade => "upgrade",
            Self::UpgradeTimeout => "upgrade-timeout",
            Self::Hello => "hello",
        }
    }
}

/// Live queue depths, sampled from the registry when the census is emitted.
///
/// Split by role and by route because a global `waiting` total cannot tell the
/// two shapes of a pairing outage apart: owners absent from the queue (guests
/// pile up with nothing to pair with) versus one route wedged at its per-route
/// ceiling while the relay as a whole looks healthy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct QueueCensus {
    pub(crate) waiting_owners: usize,
    pub(crate) waiting_guests: usize,
    pub(crate) waiting_routes: usize,
    /// Deepest single-route queue right now.
    pub(crate) max_route_queue_depth: usize,
    /// Routes sitting at `max_waiting_per_route`, which is where further
    /// peers on that route are refused with `Capacity`.
    pub(crate) routes_at_waiting_capacity: usize,
    pub(crate) active_pairs: usize,
}

impl QueueCensus {
    fn is_idle(self) -> bool {
        self.waiting_owners == 0
            && self.waiting_guests == 0
            && self.waiting_routes == 0
            && self.active_pairs == 0
    }
}

/// Rolling counters summarised on a fixed interval.
///
/// Counted rather than logged per event because a scheduled owner-lane recycle
/// produces a `PairingTimeout` rejection every recycle window per lane: at line
/// rate that is a log firehose, but as a counter it is the single most useful
/// number for spotting a pairing outage.
#[derive(Default)]
pub(crate) struct RelayCensus {
    accepted: AtomicU64,
    admission_refused: AtomicU64,
    handshake_failed: AtomicU64,
    registered: AtomicU64,
    paired: AtomicU64,
    rejected_pairing_timeout: AtomicU64,
    rejected_authentication: AtomicU64,
    rejected_capacity: AtomicU64,
    rejected_other: AtomicU64,
    closed_orderly: AtomicU64,
    closed_peer_eof: AtomicU64,
    closed_peer_reset: AtomicU64,
}

impl RelayCensus {
    /// A socket the accept loop refused before it reached a connection task.
    pub(crate) fn record_admission_refused(&self) {
        self.admission_refused.fetch_add(1, Ordering::Relaxed);
    }

    /// Separate an orderly teardown from a transport fault.
    ///
    /// A relay that is retiring lanes on schedule and a relay whose peers are
    /// being reset by an intermediary look identical in a single `closed`
    /// count, and they call for opposite responses.
    fn record_close(&self, reason: RelayCloseReason) {
        let counter = match reason {
            RelayCloseReason::PeerEof => &self.closed_peer_eof,
            RelayCloseReason::PeerReset => &self.closed_peer_reset,
            _ => &self.closed_orderly,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    fn record_reject(&self, code: RelayRejectCode) {
        let counter = match code {
            RelayRejectCode::PairingTimeout => &self.rejected_pairing_timeout,
            RelayRejectCode::AuthenticationRequired
            | RelayRejectCode::AuthenticationFailed
            | RelayRejectCode::LocatorNotYetValid
            | RelayRejectCode::LocatorExpired
            | RelayRejectCode::ExpiryTooFarFuture => &self.rejected_authentication,
            RelayRejectCode::Capacity | RelayRejectCode::RateLimited => &self.rejected_capacity,
            RelayRejectCode::MalformedHello
            | RelayRejectCode::UnsupportedVersion
            | RelayRejectCode::UnknownRoute
            | RelayRejectCode::RoleConflict
            | RelayRejectCode::RelayUnavailable
            | RelayRejectCode::Internal => &self.rejected_other,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    fn take(&self) -> CensusWindow {
        CensusWindow {
            accepted: self.accepted.swap(0, Ordering::Relaxed),
            admission_refused: self.admission_refused.swap(0, Ordering::Relaxed),
            handshake_failed: self.handshake_failed.swap(0, Ordering::Relaxed),
            registered: self.registered.swap(0, Ordering::Relaxed),
            paired: self.paired.swap(0, Ordering::Relaxed),
            rejected_pairing_timeout: self.rejected_pairing_timeout.swap(0, Ordering::Relaxed),
            rejected_authentication: self.rejected_authentication.swap(0, Ordering::Relaxed),
            rejected_capacity: self.rejected_capacity.swap(0, Ordering::Relaxed),
            rejected_other: self.rejected_other.swap(0, Ordering::Relaxed),
            closed_orderly: self.closed_orderly.swap(0, Ordering::Relaxed),
            closed_peer_eof: self.closed_peer_eof.swap(0, Ordering::Relaxed),
            closed_peer_reset: self.closed_peer_reset.swap(0, Ordering::Relaxed),
        }
    }

    /// Emit one census line, unless nothing happened and nothing is queued.
    pub(crate) fn emit(&self, queues: QueueCensus) {
        let window = self.take();
        if window.is_quiet() && queues.is_idle() {
            return;
        }
        tracing::info!(
            relay_event = "census",
            window_secs = CENSUS_INTERVAL.as_secs(),
            accepted = window.accepted,
            admission_refused = window.admission_refused,
            handshake_failed = window.handshake_failed,
            registered = window.registered,
            paired = window.paired,
            rejected_pairing_timeout = window.rejected_pairing_timeout,
            rejected_authentication = window.rejected_authentication,
            rejected_capacity = window.rejected_capacity,
            rejected_other = window.rejected_other,
            closed_orderly = window.closed_orderly,
            closed_peer_eof = window.closed_peer_eof,
            closed_peer_reset = window.closed_peer_reset,
            waiting_owners = queues.waiting_owners,
            waiting_guests = queues.waiting_guests,
            waiting_routes = queues.waiting_routes,
            max_route_queue_depth = queues.max_route_queue_depth,
            routes_at_waiting_capacity = queues.routes_at_waiting_capacity,
            active_pairs = queues.active_pairs,
            "relay census"
        );
    }
}

/// How often [`RelayCensus::emit`] should be driven.
pub(crate) const fn census_interval() -> Duration {
    CENSUS_INTERVAL
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CensusWindow {
    accepted: u64,
    admission_refused: u64,
    handshake_failed: u64,
    registered: u64,
    paired: u64,
    rejected_pairing_timeout: u64,
    rejected_authentication: u64,
    rejected_capacity: u64,
    rejected_other: u64,
    closed_orderly: u64,
    closed_peer_eof: u64,
    closed_peer_reset: u64,
}

impl CensusWindow {
    fn is_quiet(self) -> bool {
        self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use op_collab_relay_protocol::{RouteCapability, RouteId};

    use super::*;

    fn route(byte: u8) -> RouteMapKey {
        RouteMapKey::derive(
            &RouteId::new([byte; 16]).expect("non-zero route"),
            NonZeroU64::MIN,
            &RouteCapability::new([byte; 32]).expect("non-zero capability"),
        )
    }

    #[test]
    fn a_route_reference_is_stable_opaque_and_never_the_route_key() {
        let key = route(7);
        let reference = RouteRef::derive(&key).to_string();

        assert_eq!(reference, RouteRef::derive(&key).to_string());
        assert_eq!(reference.len(), ROUTE_LOG_REF_BYTES * 2);
        assert!(reference.chars().all(|c| c.is_ascii_hexdigit()));
        let key_hex: String = key.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
        assert!(!key_hex.contains(&reference));
        assert_ne!(reference, RouteRef::derive(&route(8)).to_string());
    }

    #[test]
    fn rejections_are_bucketed_by_operational_cause() {
        let census = RelayCensus::default();
        census.record_reject(RelayRejectCode::PairingTimeout);
        census.record_reject(RelayRejectCode::PairingTimeout);
        census.record_reject(RelayRejectCode::AuthenticationFailed);
        census.record_reject(RelayRejectCode::Capacity);
        census.record_reject(RelayRejectCode::MalformedHello);

        let window = census.take();
        assert_eq!(window.rejected_pairing_timeout, 2);
        assert_eq!(window.rejected_authentication, 1);
        assert_eq!(window.rejected_capacity, 1);
        assert_eq!(window.rejected_other, 1);
        // Counters reset, so the next window reports only new events.
        assert!(census.take().is_quiet());
    }

    #[test]
    fn a_clean_retirement_is_never_counted_as_a_transport_fault() {
        // The whole point of the split: "the relay retired lanes on schedule"
        // and "something is severing tunnels" must not share a counter.
        let census = RelayCensus::default();
        census.record_reject(RelayRejectCode::PairingTimeout);
        census.record_close(RelayCloseReason::Rejected(RelayRejectCode::PairingTimeout));
        census.record_close(RelayCloseReason::IdleTimeout);
        census.record_close(RelayCloseReason::PeerEof);
        census.record_close(RelayCloseReason::PeerReset);
        census.record_close(RelayCloseReason::PeerReset);

        let window = census.take();
        assert_eq!(window.rejected_pairing_timeout, 1);
        assert_eq!(window.closed_orderly, 2);
        assert_eq!(window.closed_peer_eof, 1);
        assert_eq!(window.closed_peer_reset, 2);
    }

    #[test]
    fn an_idle_relay_stays_silent_but_a_queued_one_does_not() {
        let census = RelayCensus::default();
        assert!(census.take().is_quiet());
        assert!(QueueCensus::default().is_idle());
        // A queue that is not draining is a signal even with no events in the
        // window: that is the shape of an outage nobody is retrying through.
        assert!(!QueueCensus {
            waiting_guests: 3,
            waiting_routes: 1,
            ..QueueCensus::default()
        }
        .is_idle());
    }

    #[test]
    fn an_absent_field_renders_as_a_dash() {
        assert_eq!(Absent::<&str>(None).to_string(), "-");
        assert_eq!(Absent(Some("owner")).to_string(), "owner");
    }
}
