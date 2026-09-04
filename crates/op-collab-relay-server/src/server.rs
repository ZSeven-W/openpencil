use std::{
    future::Future,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use tokio::{
    net::TcpListener,
    sync::{watch, Semaphore},
    task::JoinSet,
};

use crate::{
    auth::{RelayAuthenticator, UnauthenticatedDevAuthenticator},
    config::RelayConfig,
    connection::{serve_connection, AdmissionPermit, ConnectionServices},
    error::RelayServerError,
    observe::{census_interval, RelayCensus},
    peer_quota::PeerQuota,
    registry::Registry,
};

pub async fn run(config: RelayConfig) -> Result<(), RelayServerError> {
    run_until(config, async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl-C handler");
        }
    })
    .await
}

pub async fn run_with_authenticator(
    config: RelayConfig,
    authenticator: Arc<dyn RelayAuthenticator>,
) -> Result<(), RelayServerError> {
    run_with_authenticator_until(config, authenticator, async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl-C handler");
        }
    })
    .await
}

pub async fn run_until<F>(config: RelayConfig, shutdown: F) -> Result<(), RelayServerError>
where
    F: Future<Output = ()>,
{
    if !config.unauthenticated_dev() {
        return Err(RelayServerError::AuthenticationRequired);
    }
    run_with_authenticator_until(
        config,
        Arc::new(UnauthenticatedDevAuthenticator::new(SystemTime::now)),
        shutdown,
    )
    .await
}

pub async fn run_with_authenticator_until<F>(
    config: RelayConfig,
    authenticator: Arc<dyn RelayAuthenticator>,
    shutdown: F,
) -> Result<(), RelayServerError>
where
    F: Future<Output = ()>,
{
    config.validate()?;
    let listener =
        TcpListener::bind(config.listen)
            .await
            .map_err(|source| RelayServerError::Bind {
                address: config.listen,
                source,
            })?;
    serve_listener(listener, config, authenticator, shutdown).await
}

pub(crate) async fn serve_listener<F>(
    listener: TcpListener,
    config: RelayConfig,
    authenticator: Arc<dyn RelayAuthenticator>,
    shutdown: F,
) -> Result<(), RelayServerError>
where
    F: Future<Output = ()>,
{
    let local_addr = listener.local_addr().map_err(RelayServerError::Accept)?;
    tracing::info!(address = %local_addr, "collaboration relay listening");

    let config = Arc::new(config);
    let registry = Registry::new(
        config.max_active_pairs,
        config.max_waiting_per_route,
        config.max_queued_bytes_per_route,
    );
    let pending = Arc::new(Semaphore::new(config.max_pending));
    let peer_quota = PeerQuota::new(config.max_pending_per_source);
    let auth_in_flight = Arc::new(Semaphore::new(config.max_auth_in_flight));
    let reauth_in_flight = Arc::new(Semaphore::new(config.max_reauth_in_flight));
    let queued_bytes = Arc::new(Semaphore::new(config.max_queued_bytes));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut connections = JoinSet::new();
    let mut rejections = RejectionLog::default();
    let census = Arc::new(RelayCensus::default());
    // Driven from the accept loop so the gauge keeps ticking even while the
    // relay is not accepting anything — an outage is precisely the case where
    // no new connection arrives to trigger a lazy summary.
    let mut census_tick = tokio::time::interval(census_interval());
    census_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // The first tick resolves immediately; spend it so the relay does not log
    // an empty census the instant it starts.
    census_tick.tick().await;
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            biased;
            () = &mut shutdown => break,
            _ = census_tick.tick() => {
                census.emit(registry.queue_census().await);
            }
            accepted = listener.accept() => {
                let (stream, peer_addr) = accepted.map_err(RelayServerError::Accept)?;
                let Ok(pending_permit) = Arc::clone(&pending).try_acquire_owned() else {
                    census.record_admission_refused();
                    rejections.record_pending_exhausted();
                    continue;
                };
                let Some(source_permit) = peer_quota.try_acquire(peer_addr.ip()) else {
                    census.record_admission_refused();
                    rejections.record_source_exhausted();
                    continue;
                };
                let config = Arc::clone(&config);
                let registry = registry.clone();
                let authenticator = Arc::clone(&authenticator);
                let auth_in_flight = Arc::clone(&auth_in_flight);
                let reauth_in_flight = Arc::clone(&reauth_in_flight);
                let queued_bytes = Arc::clone(&queued_bytes);
                let census = Arc::clone(&census);
                let shutdown = shutdown_rx.clone();
                connections.spawn(async move {
                    serve_connection(
                        stream,
                        ConnectionServices {
                            config,
                            registry,
                            authenticator,
                            auth_in_flight,
                            reauth_in_flight,
                            queued_bytes,
                            census,
                        },
                        shutdown,
                        AdmissionPermit::new(pending_permit, source_permit),
                    )
                    .await;
                });
            }
            Some(result) = connections.join_next(), if !connections.is_empty() => {
                if let Err(error) = result {
                    tracing::warn!(%error, "relay connection task failed");
                }
            }
        }
    }

    rejections.flush_pending_summary();
    census.emit(registry.queue_census().await);
    tracing::info!("relay shutdown requested");
    let _ = shutdown_tx.send(true);
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            tracing::warn!(%error, "relay connection task failed during shutdown");
        }
    }
    Ok(())
}

/// Interval between capacity-rejection summaries.
const REJECTION_SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

/// Rate-limits capacity-rejection logging.
///
/// A refused connection is precisely the event a flood generates at line rate,
/// so emitting one line per rejection turns the flood into unbounded log-write
/// amplification on the shared relay. Rejections are counted instead and
/// summarised at most once per interval; the first rejection after a quiet
/// period still logs immediately so operators are not left without a signal.
struct RejectionLog {
    summary_started: Instant,
    pending_exhausted: u64,
    source_exhausted: u64,
}

impl Default for RejectionLog {
    fn default() -> Self {
        Self {
            summary_started: Instant::now()
                .checked_sub(REJECTION_SUMMARY_INTERVAL)
                .unwrap_or_else(Instant::now),
            pending_exhausted: 0,
            source_exhausted: 0,
        }
    }
}

impl RejectionLog {
    fn record_pending_exhausted(&mut self) {
        self.pending_exhausted = self.pending_exhausted.saturating_add(1);
        self.summarize_if_due();
    }

    fn record_source_exhausted(&mut self) {
        self.source_exhausted = self.source_exhausted.saturating_add(1);
        self.summarize_if_due();
    }

    fn summarize_if_due(&mut self) {
        let now = Instant::now();
        if now.saturating_duration_since(self.summary_started) < REJECTION_SUMMARY_INTERVAL {
            return;
        }
        self.summary_started = now;
        self.emit();
    }

    fn flush_pending_summary(&mut self) {
        if self.pending_exhausted == 0 && self.source_exhausted == 0 {
            return;
        }
        self.emit();
    }

    fn emit(&mut self) {
        tracing::warn!(
            pending_exhausted = self.pending_exhausted,
            source_exhausted = self.source_exhausted,
            "relay refused connections at capacity"
        );
        self.pending_exhausted = 0;
        self.source_exhausted = 0;
    }
}
