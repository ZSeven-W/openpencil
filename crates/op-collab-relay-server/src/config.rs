use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::ParseIntError,
    time::Duration,
};

use op_collab_relay_protocol::{
    RelayWaitingAdvertisementV1, DEFAULT_RELAY_WAITING_TIMEOUT_SECS, MAX_ADVERTISED_WAITING_SECS,
    MAX_RELAY_WAITING_TIMEOUT_SECS, MIN_RELAY_WAITING_TIMEOUT_SECS,
};

/// Smallest interval between waiting-lease pings.
///
/// Only reachable with a deliberately tiny waiting or idle window (tests): a
/// validated production waiting window is at least
/// `MIN_RELAY_WAITING_TIMEOUT_SECS`, which puts the real interval well above
/// this floor.
const MIN_LEASE_PING_INTERVAL: Duration = Duration::from_secs(1);

/// The lease ping must fire several times inside both the waiting window and
/// the idle window: the peer's pong is what renews the lease AND what keeps the
/// idle reaper away, so a ping slower than the idle timeout would have the
/// relay reaping the very peers it is trying to hold.
const LEASE_PINGS_PER_WINDOW: u32 = 3;

const LISTEN_ENV: &str = "OPENPENCIL_COLLAB_RELAY_LISTEN";
const MAX_PENDING_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_PENDING";
const MAX_PENDING_PER_SOURCE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_PENDING_PER_SOURCE";
const MAX_AUTH_IN_FLIGHT_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_AUTH_IN_FLIGHT";
const MAX_REAUTH_IN_FLIGHT_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_REAUTH_IN_FLIGHT";
const MAX_ACTIVE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_ACTIVE";
const MAX_WAITING_PER_ROUTE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_WAITING_PER_ROUTE";
const RELAY_QUEUE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_QUEUE_CAPACITY";
const MAX_QUEUED_BYTES_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_QUEUED_BYTES";
const MAX_QUEUED_BYTES_PER_ROUTE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_QUEUED_BYTES_PER_ROUTE";
const WAITING_TIMEOUT_SECS_ENV: &str = "OPENPENCIL_COLLAB_RELAY_WAITING_TIMEOUT_SECS";
const WAITING_LEASE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_WAITING_LEASE";

#[derive(Clone, Debug)]
pub struct RelayConfig {
    pub listen: SocketAddr,
    pub handshake_timeout: Duration,
    /// How long an un-paired peer may sit in the waiting queue before the
    /// relay rejects it with `RelayRejectCode::PairingTimeout`.
    ///
    /// This is one half of a two-sided contract: the owner client retires an
    /// idle lane after `op_collab_relay_client::RelayLimits::owner_pair` and
    /// dials a fresh one, and that budget must expire first or the relay
    /// empties its own waiting queue for the length of a client re-dial. The
    /// bounds enforced by [`RelayConfig::validate`] come from
    /// `op_collab_relay_protocol::pairing_window`, which both sides build
    /// their constants from, so neither can drift silently.
    ///
    /// Operators override it with `OPENPENCIL_COLLAB_RELAY_WAITING_TIMEOUT_SECS`.
    pub waiting_timeout: Duration,
    pub idle_timeout: Duration,
    pub tunnel_lifetime: Duration,
    pub max_message_bytes: usize,
    pub max_pending: usize,
    /// How many un-paired connections one source address may hold at once.
    pub max_pending_per_source: usize,
    pub max_auth_in_flight: usize,
    /// Renewal budget for already-authenticated tunnels.
    ///
    /// Kept separate from `max_auth_in_flight` so a flood of unauthenticated
    /// connections cannot starve the reauthentication of live sessions, which
    /// closes them with a policy error when it cannot complete in time.
    pub max_reauth_in_flight: usize,
    pub max_active_pairs: usize,
    pub max_waiting_per_route: usize,
    pub relay_queue_capacity: usize,
    pub max_queued_bytes: usize,
    pub max_queued_bytes_per_route: usize,
    /// Renew an un-paired peer's waiting slot for as long as it answers the
    /// relay's WebSocket pings.
    ///
    /// Without a lease, `waiting_timeout` is a hard countdown from
    /// registration, so every healthy owner lane is retired on a fixed cadence
    /// whether or not anything is wrong with it — the relay-driven half of the
    /// reconnect churn. With a lease the countdown restarts on every pong, so a
    /// live owner simply stays in the queue; a dead one still stops ponging and
    /// is reaped by `idle_timeout`, and `max_waiting_per_route` still bounds
    /// how many slots one route can hold.
    ///
    /// The renewal is capped by the connection's authentication deadline and
    /// `tunnel_lifetime` (see `RelayAuthState::effective_deadline`), so a lease
    /// can never outlive the credential that opened it.
    ///
    /// Operators disable it with `OPENPENCIL_COLLAB_RELAY_WAITING_LEASE=0`.
    pub waiting_lease: bool,
    unauthenticated_dev: bool,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8091),
            handshake_timeout: Duration::from_secs(5),
            waiting_timeout: Duration::from_secs(DEFAULT_RELAY_WAITING_TIMEOUT_SECS),
            idle_timeout: Duration::from_secs(90),
            tunnel_lifetime: Duration::from_secs(12 * 60 * 60),
            max_message_bytes: 64 * 1024,
            max_pending: 1_024,
            max_pending_per_source: 16,
            max_auth_in_flight: 128,
            max_reauth_in_flight: 128,
            max_active_pairs: 10_000,
            max_waiting_per_route: 4,
            relay_queue_capacity: 32,
            max_queued_bytes: 64 * 1024 * 1024,
            max_queued_bytes_per_route: 1024 * 1024,
            waiting_lease: true,
            unauthenticated_dev: false,
        }
    }
}

impl RelayConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        if let Some(value) = env::var_os(LISTEN_ENV) {
            let value = value
                .into_string()
                .map_err(|_| ConfigError::NonUnicode(LISTEN_ENV))?;
            config.listen = value
                .parse()
                .map_err(|source| ConfigError::Listen { value, source })?;
        }
        config.max_pending = parse_positive_usize(MAX_PENDING_ENV, config.max_pending)?;
        config.max_pending_per_source =
            parse_positive_usize(MAX_PENDING_PER_SOURCE_ENV, config.max_pending_per_source)?;
        config.max_auth_in_flight =
            parse_positive_usize(MAX_AUTH_IN_FLIGHT_ENV, config.max_auth_in_flight)?;
        config.max_reauth_in_flight =
            parse_positive_usize(MAX_REAUTH_IN_FLIGHT_ENV, config.max_reauth_in_flight)?;
        config.max_active_pairs = parse_positive_usize(MAX_ACTIVE_ENV, config.max_active_pairs)?;
        config.max_waiting_per_route =
            parse_positive_usize(MAX_WAITING_PER_ROUTE_ENV, config.max_waiting_per_route)?;
        config.relay_queue_capacity =
            parse_positive_usize(RELAY_QUEUE_ENV, config.relay_queue_capacity)?;
        config.max_queued_bytes =
            parse_positive_usize(MAX_QUEUED_BYTES_ENV, config.max_queued_bytes)?;
        config.max_queued_bytes_per_route = parse_positive_usize(
            MAX_QUEUED_BYTES_PER_ROUTE_ENV,
            config.max_queued_bytes_per_route,
        )?;
        config.waiting_timeout =
            parse_positive_seconds(WAITING_TIMEOUT_SECS_ENV, config.waiting_timeout)?;
        config.waiting_lease = parse_flag(WAITING_LEASE_ENV, config.waiting_lease)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        for (name, value) in [
            ("max_message_bytes", self.max_message_bytes),
            ("max_pending", self.max_pending),
            ("max_pending_per_source", self.max_pending_per_source),
            ("max_auth_in_flight", self.max_auth_in_flight),
            ("max_reauth_in_flight", self.max_reauth_in_flight),
            ("max_active_pairs", self.max_active_pairs),
            ("max_waiting_per_route", self.max_waiting_per_route),
            ("relay_queue_capacity", self.relay_queue_capacity),
            ("max_queued_bytes", self.max_queued_bytes),
            (
                "max_queued_bytes_per_route",
                self.max_queued_bytes_per_route,
            ),
        ] {
            if value == 0 {
                return Err(ConfigError::Zero(name));
            }
        }
        if self.max_message_bytes > 64 * 1024 {
            return Err(ConfigError::TooLarge {
                name: "max_message_bytes",
                maximum: 64 * 1024,
            });
        }
        if self.max_queued_bytes > u32::MAX as usize {
            return Err(ConfigError::TooLarge {
                name: "max_queued_bytes",
                maximum: u32::MAX as usize,
            });
        }
        if self.max_queued_bytes_per_route > self.max_queued_bytes {
            return Err(ConfigError::RouteBudgetExceedsGlobal);
        }
        if self.max_pending_per_source > self.max_pending {
            return Err(ConfigError::SourceBudgetExceedsGlobal);
        }
        for (name, value) in [
            ("handshake_timeout", self.handshake_timeout),
            ("waiting_timeout", self.waiting_timeout),
            ("idle_timeout", self.idle_timeout),
            ("tunnel_lifetime", self.tunnel_lifetime),
        ] {
            if value.is_zero() {
                return Err(ConfigError::ZeroDuration(name));
            }
        }
        self.validate_waiting_timeout()?;
        Ok(())
    }

    /// Keep the waiting window inside the owner-lane pairing contract.
    ///
    /// Rejecting rather than clamping matches the sibling budgets: a relay
    /// that silently runs a different window than the operator asked for is
    /// exactly the failure this bound exists to prevent.
    fn validate_waiting_timeout(&self) -> Result<(), ConfigError> {
        let seconds = self.waiting_timeout.as_secs();
        if self.waiting_timeout < Duration::from_secs(MIN_RELAY_WAITING_TIMEOUT_SECS) {
            return Err(ConfigError::WaitingTimeoutTooShort {
                seconds,
                minimum: MIN_RELAY_WAITING_TIMEOUT_SECS,
            });
        }
        if self.waiting_timeout > Duration::from_secs(MAX_RELAY_WAITING_TIMEOUT_SECS) {
            return Err(ConfigError::WaitingTimeoutTooLong {
                seconds,
                maximum: MAX_RELAY_WAITING_TIMEOUT_SECS,
            });
        }
        Ok(())
    }

    /// How often the relay pings an un-paired peer to renew its lease.
    ///
    /// Bounded by the idle window as well as the waiting window so a pong
    /// always arrives before the idle reaper would fire.
    pub(crate) fn lease_ping_interval(&self) -> Duration {
        (self.waiting_timeout.min(self.idle_timeout) / LEASE_PINGS_PER_WINDOW)
            .max(MIN_LEASE_PING_INTERVAL)
    }

    /// The waiting capability advertised on the WebSocket upgrade response.
    ///
    /// With a lease the advertised window is the ceiling a healthy peer may
    /// reach — the tunnel lifetime — because the per-lease countdown restarts
    /// on every pong. Without one it is the countdown itself. Clients only ever
    /// narrow towards this value; see
    /// `RelayWaitingAdvertisementV1::derive_lane_budget`.
    pub(crate) fn waiting_advertisement(&self) -> RelayWaitingAdvertisementV1 {
        let window = if self.waiting_lease {
            self.tunnel_lifetime
                .as_secs()
                .clamp(self.waiting_timeout.as_secs(), MAX_ADVERTISED_WAITING_SECS)
        } else {
            self.waiting_timeout.as_secs()
        };
        RelayWaitingAdvertisementV1::new(window.max(1), self.waiting_lease)
            .unwrap_or_else(|_| unreachable!("a validated waiting window is advertisable"))
    }

    /// Enable capability-only pairing for local development and tests.
    ///
    /// This mode proves neither possession of a collaboration ticket nor a
    /// device key and therefore must never be exposed as a production relay.
    pub fn allow_unauthenticated_dev(mut self) -> Self {
        self.unauthenticated_dev = true;
        self
    }

    pub(crate) fn unauthenticated_dev(&self) -> bool {
        self.unauthenticated_dev
    }
}

fn parse_positive_usize(name: &'static str, default: usize) -> Result<usize, ConfigError> {
    let Some(value) = env::var_os(name) else {
        return Ok(default);
    };
    let value = value
        .into_string()
        .map_err(|_| ConfigError::NonUnicode(name))?;
    value.parse().map_err(|source| ConfigError::Number {
        name,
        value,
        source,
    })
}

fn parse_positive_seconds(name: &'static str, default: Duration) -> Result<Duration, ConfigError> {
    let Some(value) = env::var_os(name) else {
        return Ok(default);
    };
    let value = value
        .into_string()
        .map_err(|_| ConfigError::NonUnicode(name))?;
    let seconds: u64 = value.parse().map_err(|source| ConfigError::Number {
        name,
        value,
        source,
    })?;
    Ok(Duration::from_secs(seconds))
}

fn parse_flag(name: &'static str, default: bool) -> Result<bool, ConfigError> {
    let Some(value) = env::var_os(name) else {
        return Ok(default);
    };
    let value = value
        .into_string()
        .map_err(|_| ConfigError::NonUnicode(name))?;
    match value.as_str() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(ConfigError::Flag { name, value }),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is not valid Unicode")]
    NonUnicode(&'static str),
    #[error("{name} must be a positive integer, got {value}")]
    Number {
        name: &'static str,
        value: String,
        #[source]
        source: ParseIntError,
    },
    #[error("{LISTEN_ENV} must be an IP socket address, got {value}")]
    Listen {
        value: String,
        #[source]
        source: std::net::AddrParseError,
    },
    #[error("{0} must be greater than zero")]
    Zero(&'static str),
    #[error("{0} must be a non-zero duration")]
    ZeroDuration(&'static str),
    #[error("{name} exceeds the supported maximum of {maximum}")]
    TooLarge { name: &'static str, maximum: usize },
    #[error("max_queued_bytes_per_route must not exceed max_queued_bytes")]
    RouteBudgetExceedsGlobal,
    #[error("max_pending_per_source must not exceed max_pending")]
    SourceBudgetExceedsGlobal,
    #[error(
        "{WAITING_TIMEOUT_SECS_ENV} is {seconds}s but must be at least {minimum}s, or the relay \
         retires owner lanes before op_collab_relay_client::RelayLimits::owner_pair recycles them"
    )]
    WaitingTimeoutTooShort { seconds: u64, minimum: u64 },
    #[error("{WAITING_TIMEOUT_SECS_ENV} is {seconds}s but must not exceed {maximum}s")]
    WaitingTimeoutTooLong { seconds: u64, maximum: u64 },
    #[error("{name} must be 0 or 1, got {value}")]
    Flag { name: &'static str, value: String },
}
