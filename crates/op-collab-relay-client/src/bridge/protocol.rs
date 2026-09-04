use std::fmt;
use std::time::Duration;

use op_collab_relay_protocol::{
    RelayAuthExtensionV1, RelayClientHello, RelayRole, RelayServerStatus, VerifiedRelayRoute,
};
use tokio::sync::watch;
use tokio::time::Instant;

use crate::auth::AuthMode;
use crate::endpoint::RelayEndpoint;
use crate::error::{RelayFailureKind, TunnelError};
use crate::limits::{PairBudget, RelayLimits};
use crate::reauth_budget::ReauthBudget;
use crate::session::{
    connect_socket, next_binary_with_reauth, send_binary, ClientReauthContext, RelaySocket,
    RelayUpgrade,
};

/// Caller-verified routing material plus this device's authentication
/// extension.
///
/// The desktop verifies and retains invite identity fields. The bridge only
/// uses this value to encode the fixed relay hello.
#[derive(Clone)]
pub struct RelayHandshake {
    route: VerifiedRelayRoute,
    auth: RelayAuthExtensionV1,
}

impl RelayHandshake {
    pub fn new(route: VerifiedRelayRoute, auth: RelayAuthExtensionV1) -> Self {
        Self { route, auth }
    }

    pub(crate) fn route(&self) -> &VerifiedRelayRoute {
        &self.route
    }
}

impl fmt::Debug for RelayHandshake {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RelayHandshake([REDACTED])")
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn establish_relay(
    endpoint: &RelayEndpoint,
    handshake: &RelayHandshake,
    role: RelayRole,
    auth: &AuthMode,
    reauth_budget: &mut ReauthBudget,
    cancel: &mut watch::Receiver<bool>,
    started_at: Instant,
    limits: RelayLimits,
) -> Result<RelaySocket, TunnelError> {
    establish_relay_with_ready_hook(
        endpoint,
        handshake,
        role,
        auth,
        reauth_budget,
        cancel,
        started_at,
        limits,
        PairBudget::Fixed(limits.pair),
        || {},
    )
    .await
}

/// `pair_budget` is how long this connection may wait for its counterpart
/// after the relay accepted it. Guests use the long [`RelayLimits::pair`]
/// window; owner lanes pass a policy that is resolved against the relay's
/// advertised waiting capability once the upgrade response is in hand, so the
/// client, not the relay, decides when an idle lane is retired.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn establish_relay_with_ready_hook<F>(
    endpoint: &RelayEndpoint,
    handshake: &RelayHandshake,
    role: RelayRole,
    auth: &AuthMode,
    reauth_budget: &mut ReauthBudget,
    cancel: &mut watch::Receiver<bool>,
    started_at: Instant,
    limits: RelayLimits,
    pair_budget: PairBudget,
    on_ready: F,
) -> Result<RelaySocket, TunnelError>
where
    F: FnOnce(),
{
    let (connect_timeout, connect_kind) = phase_timeout(
        started_at,
        limits,
        limits.connect,
        RelayFailureKind::ConnectTimeout,
    )?;
    let mut connect_limits = limits;
    connect_limits.connect = connect_timeout;
    let RelayUpgrade {
        mut socket,
        challenge_attempt,
        waiting,
    } = connect_socket(endpoint, auth, cancel, connect_limits)
        .await
        .map_err(|error| remap_timeout(error, connect_kind))?;
    let pair_budget = pair_budget.resolve(waiting);

    let hello = match challenge_attempt {
        Some((attempt, challenge)) => attempt
            .finish(&challenge, role, &handshake.route)
            .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))?
            .into_hello(),
        None => RelayClientHello::new(role, &handshake.route, handshake.auth.clone()),
    };
    let (hello_timeout, hello_kind) = phase_timeout(
        started_at,
        limits,
        limits.hello,
        RelayFailureKind::HelloTimeout,
    )?;
    send_binary(
        &mut socket,
        hello.encode().to_vec(),
        cancel,
        hello_timeout,
        hello_kind,
    )
    .await?;
    let first = next_status(
        &mut socket,
        ClientReauthContext {
            auth,
            role,
            route: &handshake.route,
        },
        reauth_budget,
        cancel,
        hello_timeout,
        hello_kind,
        limits.max_binary_bytes,
    )
    .await?;
    match first {
        RelayServerStatus::Ready => on_ready(),
        RelayServerStatus::Rejected(code) => {
            return Err(TunnelError::Failure(RelayFailureKind::from_reject(code)));
        }
        RelayServerStatus::Paired => {
            return Err(TunnelError::Failure(RelayFailureKind::Protocol));
        }
    }

    let (pair_timeout, pair_kind) = phase_timeout(
        started_at,
        limits,
        pair_budget,
        RelayFailureKind::PairTimeout,
    )?;
    let second = next_status(
        &mut socket,
        ClientReauthContext {
            auth,
            role,
            route: &handshake.route,
        },
        reauth_budget,
        cancel,
        pair_timeout,
        pair_kind,
        limits.max_binary_bytes,
    )
    .await?;
    match second {
        RelayServerStatus::Paired => Ok(socket),
        RelayServerStatus::Rejected(code) => {
            Err(TunnelError::Failure(RelayFailureKind::from_reject(code)))
        }
        RelayServerStatus::Ready => Err(TunnelError::Failure(RelayFailureKind::Protocol)),
    }
}

#[allow(clippy::too_many_arguments)]
async fn next_status(
    socket: &mut RelaySocket,
    reauth: ClientReauthContext<'_>,
    reauth_budget: &mut ReauthBudget,
    cancel: &mut watch::Receiver<bool>,
    timeout: Duration,
    timeout_kind: RelayFailureKind,
    max_binary_bytes: usize,
) -> Result<RelayServerStatus, TunnelError> {
    let bytes = next_binary_with_reauth(
        socket,
        reauth,
        reauth_budget,
        cancel,
        timeout,
        timeout_kind,
        max_binary_bytes,
    )
    .await?;
    RelayServerStatus::decode(&bytes).map_err(|_| TunnelError::Failure(RelayFailureKind::Protocol))
}

fn phase_timeout(
    started_at: Instant,
    limits: RelayLimits,
    phase_limit: Duration,
    phase_kind: RelayFailureKind,
) -> Result<(Duration, RelayFailureKind), TunnelError> {
    let remaining = (started_at + limits.lifetime)
        .checked_duration_since(Instant::now())
        .ok_or(TunnelError::Failure(RelayFailureKind::LifetimeExceeded))?;
    if remaining <= phase_limit {
        Ok((remaining, RelayFailureKind::LifetimeExceeded))
    } else {
        Ok((phase_limit, phase_kind))
    }
}

fn remap_timeout(error: TunnelError, timeout_kind: RelayFailureKind) -> TunnelError {
    match error {
        TunnelError::Failure(RelayFailureKind::ConnectTimeout) => {
            TunnelError::Failure(timeout_kind)
        }
        other => other,
    }
}
