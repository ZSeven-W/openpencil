use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::time::Instant;

use super::protocol::{establish_relay, RelayHandshake};
use super::BridgeHandle;
use crate::auth::{AuthMode, RelayAuthenticator, RelayCredentialProvider};
use crate::endpoint::RelayEndpoint;
use crate::error::{
    RelayBridgePhase, RelayBridgeStatus, RelayClientError, RelayFailureKind, RelayStopError,
    TunnelError,
};
use crate::limits::RelayLimits;
use crate::reauth_budget::ReauthBudget;
use crate::session::{cancelled, pump, ClientReauthContext};

/// A single-use loopback listener that connects an existing guest TCP driver
/// to one authenticated relay tunnel.
pub struct RelayGuestBridge {
    local_addr: SocketAddr,
    handle: BridgeHandle,
}

impl RelayGuestBridge {
    pub async fn start(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        authenticator: Arc<dyn RelayAuthenticator>,
    ) -> Result<Self, RelayClientError> {
        Self::start_inner(
            endpoint,
            route,
            AuthMode::ChallengeBound(authenticator),
            RelayLimits::default(),
        )
        .await
    }

    /// Starts the explicit reduced-assurance ticket-to-DH compatibility mode.
    ///
    /// This mode does not produce a challenge proof and must be paired only
    /// with a relay explicitly configured for ticket-binding-only admission.
    pub async fn start_ticket_binding_only(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        credentials: Arc<dyn RelayCredentialProvider>,
    ) -> Result<Self, RelayClientError> {
        Self::start_inner(
            endpoint,
            route,
            AuthMode::TicketBindingOnly(credentials),
            RelayLimits::default(),
        )
        .await
    }

    /// Starts an anonymous loopback relay for local development.
    ///
    /// This API is absent from release builds.
    #[cfg(any(test, debug_assertions))]
    pub async fn start_unauthenticated_for_development(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
    ) -> Result<Self, RelayClientError> {
        if !endpoint.is_numeric_loopback() {
            return Err(RelayClientError::DevelopmentEndpointNotLoopback);
        }
        Self::start_inner(
            endpoint,
            route,
            AuthMode::DevelopmentAnonymous,
            RelayLimits::default(),
        )
        .await
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn status(&self) -> RelayBridgeStatus {
        self.handle.status()
    }

    pub fn subscribe(&self) -> watch::Receiver<RelayBridgeStatus> {
        self.handle.subscribe()
    }

    /// Wait until the relay has paired this guest with an owner.
    ///
    /// The local TCP/Noise driver must not start its five-second handshake
    /// window before this gate opens: relay pairing has its own, much longer
    /// bounded budget and is not Noise progress.
    pub async fn wait_until_paired(
        &self,
        timeout: std::time::Duration,
    ) -> Result<(), RelayClientError> {
        let mut status = self.subscribe();
        let paired = async {
            loop {
                let current = *status.borrow();
                match current.phase {
                    RelayBridgePhase::Active => return Ok(()),
                    RelayBridgePhase::Failed => {
                        return Err(RelayClientError::GuestPairingFailed {
                            kind: current.last_error.unwrap_or(RelayFailureKind::Protocol),
                        });
                    }
                    RelayBridgePhase::Stopped => {
                        return Err(RelayClientError::StoppedBeforePaired);
                    }
                    RelayBridgePhase::Starting
                    | RelayBridgePhase::Waiting
                    | RelayBridgePhase::Degraded => {}
                }
                status
                    .changed()
                    .await
                    .map_err(|_| RelayClientError::StoppedBeforePaired)?;
            }
        };
        tokio::time::timeout(timeout, paired)
            .await
            .map_err(|_| RelayClientError::PairedTimeout)?
    }

    pub async fn stop(self) -> Result<(), RelayStopError> {
        self.handle.stop().await
    }

    async fn start_inner(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        auth: AuthMode,
        limits: RelayLimits,
    ) -> Result<Self, RelayClientError> {
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .map_err(|error| RelayClientError::BindLoopback { kind: error.kind() })?;
        let local_addr = listener
            .local_addr()
            .map_err(|error| RelayClientError::ReadLocalAddress { kind: error.kind() })?;
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let initial = RelayBridgeStatus {
            phase: RelayBridgePhase::Waiting,
            waiting_lanes: 1,
            active_tunnels: 0,
            last_error: None,
            relay_pairing_timeouts: 0,
        };
        let (status_tx, status_rx) = watch::channel(initial);
        let task = tokio::spawn(run_guest(
            endpoint, route, auth, listener, cancel_rx, status_tx, limits,
        ));
        Ok(Self {
            local_addr,
            handle: BridgeHandle::new(cancel_tx, status_rx, task, limits),
        })
    }

    #[cfg(test)]
    pub(crate) async fn start_test(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        limits: RelayLimits,
    ) -> Result<Self, RelayClientError> {
        Self::start_inner(endpoint, route, AuthMode::DevelopmentAnonymous, limits).await
    }
}

impl fmt::Debug for RelayGuestBridge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelayGuestBridge")
            .field("local_addr", &self.local_addr)
            .field("handle", &self.handle)
            .finish()
    }
}

async fn run_guest(
    endpoint: RelayEndpoint,
    route: RelayHandshake,
    auth: AuthMode,
    listener: TcpListener,
    mut cancel: watch::Receiver<bool>,
    status: watch::Sender<RelayBridgeStatus>,
    limits: RelayLimits,
) {
    let started_at = Instant::now();
    // One budget for the guest's single tunnel, spanning the handshake and the
    // pump so the bound covers the whole connection rather than one phase.
    let mut reauth_budget = ReauthBudget::new(limits);
    let socket = match establish_relay(
        &endpoint,
        &route,
        op_collab_relay_protocol::RelayRole::Guest,
        &auth,
        &mut reauth_budget,
        &mut cancel,
        started_at,
        limits,
    )
    .await
    {
        Ok(socket) => socket,
        Err(TunnelError::Cancelled) => {
            publish(&status, RelayBridgeStatus::stopped());
            return;
        }
        Err(TunnelError::Failure(kind)) => {
            publish_failed(&status, kind);
            return;
        }
    };

    if *cancel.borrow() {
        publish(&status, RelayBridgeStatus::stopped());
        return;
    }

    publish(
        &status,
        RelayBridgeStatus {
            phase: RelayBridgePhase::Active,
            waiting_lanes: 0,
            active_tunnels: 1,
            last_error: None,
            relay_pairing_timeouts: 0,
        },
    );
    // Pairing and the inner Noise handshake are separate bounded phases. Only
    // accept the loopback driver after the relay tunnel is paired, otherwise a
    // slow but healthy relay wait consumes Noise's five-second deadline.
    let remaining = match (started_at + limits.lifetime).checked_duration_since(Instant::now()) {
        Some(remaining) if !remaining.is_zero() => remaining,
        _ => {
            publish_failed(&status, RelayFailureKind::LifetimeExceeded);
            return;
        }
    };
    let accepted = tokio::select! {
        biased;
        _ = cancelled(&mut cancel) => {
            publish(&status, RelayBridgeStatus::stopped());
            return;
        }
        result = tokio::time::timeout(remaining, listener.accept()) => result,
    };
    let (mut local, _) = match accepted {
        Err(_) => {
            publish_failed(&status, RelayFailureKind::LifetimeExceeded);
            return;
        }
        Ok(Err(_)) => {
            publish_failed(&status, RelayFailureKind::LocalIo);
            return;
        }
        Ok(Ok(pair)) => pair,
    };
    if local.set_nodelay(true).is_err() {
        publish_failed(&status, RelayFailureKind::LocalIo);
        return;
    }
    match pump(
        socket,
        &mut local,
        ClientReauthContext {
            auth: &auth,
            role: op_collab_relay_protocol::RelayRole::Guest,
            route: route.route(),
        },
        &mut reauth_budget,
        &mut cancel,
        started_at,
        limits,
    )
    .await
    {
        Ok(()) | Err(TunnelError::Cancelled) => {
            publish(&status, RelayBridgeStatus::stopped());
        }
        Err(TunnelError::Failure(kind)) => publish_failed(&status, kind),
    }
}

fn publish_failed(status: &watch::Sender<RelayBridgeStatus>, kind: RelayFailureKind) {
    publish(
        status,
        RelayBridgeStatus {
            phase: RelayBridgePhase::Failed,
            waiting_lanes: 0,
            active_tunnels: 0,
            last_error: Some(kind),
            relay_pairing_timeouts: 0,
        },
    );
}

fn publish(status: &watch::Sender<RelayBridgeStatus>, value: RelayBridgeStatus) {
    status.send_replace(value);
}
