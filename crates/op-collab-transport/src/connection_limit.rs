use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use crate::{ConfigError, ConnectionLimits, TransportConfig};

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionLimitError {
    #[error("the global pending-handshake limit is full")]
    PendingHandshakesFull,
    #[error("the per-address pending-handshake limit is full")]
    PendingHandshakesPerIpFull,
    #[error("the active-connection limit is full")]
    ActiveConnectionsFull,
    #[error("the connection limiter is unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionCounts {
    pub pending_handshakes: usize,
    pub active_connections: usize,
}

#[derive(Debug)]
struct LimiterState {
    pending: usize,
    pending_by_ip: HashMap<IpAddr, usize>,
    active: usize,
}

#[derive(Debug)]
struct LimiterInner {
    limits: ConnectionLimits,
    state: Mutex<LimiterState>,
}

/// Shared admission gate for an accept loop.
///
/// A host acquires a pending guard before starting any Noise work, then turns
/// it into an active guard only after ticket admission. Dropping either guard
/// releases its count, including on early-return error paths.
#[derive(Debug, Clone)]
pub struct ConnectionLimiter {
    inner: Arc<LimiterInner>,
}

impl ConnectionLimiter {
    pub fn new(limits: ConnectionLimits) -> Result<Self, ConfigError> {
        TransportConfig {
            connections: limits,
            ..TransportConfig::default()
        }
        .validate()?;
        Ok(Self {
            inner: Arc::new(LimiterInner {
                limits,
                state: Mutex::new(LimiterState {
                    pending: 0,
                    pending_by_ip: HashMap::new(),
                    active: 0,
                }),
            }),
        })
    }

    pub fn try_begin_handshake(
        &self,
        peer_ip: IpAddr,
    ) -> Result<PendingHandshakeGuard, ConnectionLimitError> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ConnectionLimitError::Unavailable)?;
        if state.pending >= self.inner.limits.max_pending_handshakes {
            return Err(ConnectionLimitError::PendingHandshakesFull);
        }
        let per_ip = state.pending_by_ip.get(&peer_ip).copied().unwrap_or(0);
        if per_ip >= self.inner.limits.max_pending_handshakes_per_ip {
            return Err(ConnectionLimitError::PendingHandshakesPerIpFull);
        }
        state.pending += 1;
        state.pending_by_ip.insert(peer_ip, per_ip + 1);
        drop(state);
        Ok(PendingHandshakeGuard {
            inner: Arc::clone(&self.inner),
            peer_ip,
            held: true,
        })
    }

    pub fn counts(&self) -> Result<ConnectionCounts, ConnectionLimitError> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| ConnectionLimitError::Unavailable)?;
        Ok(ConnectionCounts {
            pending_handshakes: state.pending,
            active_connections: state.active,
        })
    }
}

#[derive(Debug)]
pub struct PendingHandshakeGuard {
    inner: Arc<LimiterInner>,
    peer_ip: IpAddr,
    held: bool,
}

impl PendingHandshakeGuard {
    pub const fn peer_ip(&self) -> IpAddr {
        self.peer_ip
    }

    pub fn activate(mut self) -> Result<ActiveConnectionGuard, ConnectionLimitError> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ConnectionLimitError::Unavailable)?;
        if state.active >= self.inner.limits.max_active_connections {
            return Err(ConnectionLimitError::ActiveConnectionsFull);
        }
        release_pending(&mut state, self.peer_ip);
        state.active += 1;
        self.held = false;
        drop(state);
        Ok(ActiveConnectionGuard {
            inner: Arc::clone(&self.inner),
            peer_ip: self.peer_ip,
            held: true,
        })
    }
}

impl Drop for PendingHandshakeGuard {
    fn drop(&mut self) {
        if !self.held {
            return;
        }
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        release_pending(&mut state, self.peer_ip);
        self.held = false;
    }
}

#[derive(Debug)]
pub struct ActiveConnectionGuard {
    inner: Arc<LimiterInner>,
    peer_ip: IpAddr,
    held: bool,
}

impl ActiveConnectionGuard {
    pub const fn peer_ip(&self) -> IpAddr {
        self.peer_ip
    }
}

impl Drop for ActiveConnectionGuard {
    fn drop(&mut self) {
        if !self.held {
            return;
        }
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.active = state.active.saturating_sub(1);
        self.held = false;
    }
}

fn release_pending(state: &mut LimiterState, peer_ip: IpAddr) {
    state.pending = state.pending.saturating_sub(1);
    if let Some(per_ip) = state.pending_by_ip.get_mut(&peer_ip) {
        *per_ip = per_ip.saturating_sub(1);
        if *per_ip == 0 {
            state.pending_by_ip.remove(&peer_ip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn limits() -> ConnectionLimits {
        ConnectionLimits {
            max_pending_handshakes: 2,
            max_pending_handshakes_per_ip: 1,
            max_active_connections: 1,
            ..ConnectionLimits::default()
        }
    }

    #[test]
    fn pending_limits_are_exact_and_drop_releases_them() {
        let limiter = ConnectionLimiter::new(limits()).unwrap();
        let first_ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let second_ip = IpAddr::V6(Ipv6Addr::LOCALHOST);
        let first = limiter.try_begin_handshake(first_ip).unwrap();
        assert!(matches!(
            limiter.try_begin_handshake(first_ip),
            Err(ConnectionLimitError::PendingHandshakesPerIpFull)
        ));
        let second = limiter.try_begin_handshake(second_ip).unwrap();
        assert!(matches!(
            limiter.try_begin_handshake(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1))),
            Err(ConnectionLimitError::PendingHandshakesFull)
        ));
        drop(first);
        assert_eq!(limiter.counts().unwrap().pending_handshakes, 1);
        drop(second);
        assert_eq!(limiter.counts().unwrap().pending_handshakes, 0);
    }

    #[test]
    fn activation_is_bounded_and_raii_releases_both_phases() {
        let limiter = ConnectionLimiter::new(limits()).unwrap();
        let first = limiter
            .try_begin_handshake(IpAddr::V4(Ipv4Addr::LOCALHOST))
            .unwrap();
        let active = first.activate().unwrap();
        assert_eq!(
            limiter.counts().unwrap(),
            ConnectionCounts {
                pending_handshakes: 0,
                active_connections: 1,
            }
        );

        let pending = limiter
            .try_begin_handshake(IpAddr::V6(Ipv6Addr::LOCALHOST))
            .unwrap();
        assert!(matches!(
            pending.activate(),
            Err(ConnectionLimitError::ActiveConnectionsFull)
        ));
        assert_eq!(limiter.counts().unwrap().pending_handshakes, 0);
        drop(active);
        assert_eq!(limiter.counts().unwrap().active_connections, 0);
    }
}
