//! The operator-configurable waiting window and the pairing contract it must
//! stay inside.

#![cfg(test)]

use std::{
    sync::{Mutex, MutexGuard, OnceLock},
    time::Duration,
};

use op_collab_relay_protocol::{
    DEFAULT_RELAY_WAITING_TIMEOUT_SECS, MAX_RELAY_WAITING_TIMEOUT_SECS,
    MIN_RELAY_WAITING_TIMEOUT_SECS, RELAY_OWNER_LANE_RECYCLE_SECS,
};

use crate::{ConfigError, RelayConfig};

const WAITING_TIMEOUT_SECS_ENV: &str = "OPENPENCIL_COLLAB_RELAY_WAITING_TIMEOUT_SECS";

/// Serialises the tests that mutate the process environment.
fn env_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn the_default_waiting_window_matches_the_shared_pairing_contract() {
    assert_eq!(
        RelayConfig::default().waiting_timeout,
        Duration::from_secs(DEFAULT_RELAY_WAITING_TIMEOUT_SECS)
    );
    assert!(RelayConfig::default().validate().is_ok());
}

#[test]
fn a_waiting_window_under_the_client_recycle_budget_is_refused() {
    let mut config = RelayConfig::default();
    config.waiting_timeout = Duration::from_secs(MIN_RELAY_WAITING_TIMEOUT_SECS - 1);
    assert!(matches!(
        config.validate(),
        Err(ConfigError::WaitingTimeoutTooShort { minimum, .. })
            if minimum == MIN_RELAY_WAITING_TIMEOUT_SECS
    ));

    // A relay set to the client's own recycle budget is the exact race the
    // bound exists to prevent, so it is refused too.
    config.waiting_timeout = Duration::from_secs(RELAY_OWNER_LANE_RECYCLE_SECS);
    assert!(matches!(
        config.validate(),
        Err(ConfigError::WaitingTimeoutTooShort { .. })
    ));
}

#[test]
fn the_waiting_window_bounds_are_inclusive_at_both_ends() {
    let mut config = RelayConfig::default();
    config.waiting_timeout = Duration::from_secs(MIN_RELAY_WAITING_TIMEOUT_SECS);
    assert!(config.validate().is_ok());
    config.waiting_timeout = Duration::from_secs(MAX_RELAY_WAITING_TIMEOUT_SECS);
    assert!(config.validate().is_ok());
}

#[test]
fn an_unbounded_waiting_window_is_refused() {
    let mut config = RelayConfig::default();
    config.waiting_timeout = Duration::from_secs(MAX_RELAY_WAITING_TIMEOUT_SECS + 1);
    assert!(matches!(
        config.validate(),
        Err(ConfigError::WaitingTimeoutTooLong { maximum, .. })
            if maximum == MAX_RELAY_WAITING_TIMEOUT_SECS
    ));
}

#[test]
fn a_zero_waiting_window_is_refused_before_the_range_check() {
    let mut config = RelayConfig::default();
    config.waiting_timeout = Duration::ZERO;
    assert!(matches!(
        config.validate(),
        Err(ConfigError::ZeroDuration("waiting_timeout"))
    ));
}

#[test]
fn the_environment_override_is_parsed_validated_and_optional() {
    let _guard = env_guard();

    std::env::remove_var(WAITING_TIMEOUT_SECS_ENV);
    assert_eq!(
        RelayConfig::from_env()
            .expect("default configuration")
            .waiting_timeout,
        Duration::from_secs(DEFAULT_RELAY_WAITING_TIMEOUT_SECS)
    );

    std::env::set_var(WAITING_TIMEOUT_SECS_ENV, "120");
    assert_eq!(
        RelayConfig::from_env()
            .expect("configured waiting window")
            .waiting_timeout,
        Duration::from_secs(120)
    );

    std::env::set_var(WAITING_TIMEOUT_SECS_ENV, "30");
    assert!(matches!(
        RelayConfig::from_env(),
        Err(ConfigError::WaitingTimeoutTooShort { seconds: 30, .. })
    ));

    std::env::set_var(WAITING_TIMEOUT_SECS_ENV, "not-a-number");
    assert!(matches!(
        RelayConfig::from_env(),
        Err(ConfigError::Number { name, .. }) if name == WAITING_TIMEOUT_SECS_ENV
    ));

    std::env::remove_var(WAITING_TIMEOUT_SECS_ENV);
}

#[test]
fn the_refusal_names_both_ends_of_the_contract() {
    let mut config = RelayConfig::default();
    config.waiting_timeout = Duration::from_secs(1);
    let message = config
        .validate()
        .expect_err("a one-second waiting window is refused")
        .to_string();
    assert!(message.contains(WAITING_TIMEOUT_SECS_ENV));
    assert!(message.contains("RelayLimits::owner_pair"));
}

#[test]
fn the_lease_is_on_by_default_and_has_a_strict_operator_switch() {
    let _guard = env_guard();
    const LEASE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_WAITING_LEASE";

    std::env::remove_var(LEASE_ENV);
    assert!(RelayConfig::default().waiting_lease);
    assert!(
        RelayConfig::from_env()
            .expect("default configuration")
            .waiting_lease
    );

    std::env::set_var(LEASE_ENV, "0");
    assert!(
        !RelayConfig::from_env()
            .expect("lease disabled")
            .waiting_lease
    );
    std::env::set_var(LEASE_ENV, "1");
    assert!(
        RelayConfig::from_env()
            .expect("lease enabled")
            .waiting_lease
    );

    for invalid in ["", "true", "yes", "01"] {
        std::env::set_var(LEASE_ENV, invalid);
        assert!(matches!(
            RelayConfig::from_env(),
            Err(ConfigError::Flag { name, .. }) if name == LEASE_ENV
        ));
    }
    std::env::remove_var(LEASE_ENV);
}

#[test]
fn the_lease_ping_always_fits_inside_both_the_waiting_and_idle_windows() {
    // A ping slower than the idle window would have the relay reaping the very
    // peers the lease exists to hold.
    let mut config = RelayConfig::default();
    for (waiting, idle) in [(55_u64, 90_u64), (60, 90), (900, 90), (900, 900), (55, 10)] {
        config.waiting_timeout = Duration::from_secs(waiting);
        config.idle_timeout = Duration::from_secs(idle);
        let interval = config.lease_ping_interval();
        assert!(interval * 2 <= config.waiting_timeout, "waiting {waiting}");
        assert!(interval * 2 <= config.idle_timeout, "idle {idle}");
        assert!(!interval.is_zero());
    }
}

#[test]
fn the_advertisement_tells_a_client_which_regime_the_relay_runs() {
    let mut config = RelayConfig::default();
    config.tunnel_lifetime = Duration::from_secs(12 * 60 * 60);

    let leased = config.waiting_advertisement();
    assert!(leased.renewable());
    assert_eq!(leased.window_secs(), 12 * 60 * 60);

    config.waiting_lease = false;
    let countdown = config.waiting_advertisement();
    assert!(!countdown.renewable());
    assert_eq!(countdown.window_secs(), config.waiting_timeout.as_secs());

    // Never advertises more than the protocol's ceiling, whatever the lifetime.
    config.waiting_lease = true;
    config.tunnel_lifetime = Duration::from_secs(10 * 24 * 60 * 60);
    assert_eq!(
        config.waiting_advertisement().window_secs(),
        op_collab_relay_protocol::MAX_ADVERTISED_WAITING_SECS
    );
}
