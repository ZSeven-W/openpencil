//! The un-paired waiting phase.
//!
//! Split out of `connection.rs` at the 800-line cap. This is where an
//! authenticated peer sits in the relay's waiting queue until a counterpart
//! arrives, and where the waiting lease is renewed.

use std::{pin::Pin, sync::Arc};

use futures_util::{SinkExt, StreamExt};
use op_collab_relay_protocol::RelayRejectCode;
use tokio::{
    sync::{watch, Semaphore},
    time::{Instant, Sleep},
};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    auth::RelayAuthenticator,
    close::RelayCloseReason,
    config::RelayConfig,
    connection::{
        close_out, reject, reset_idle, sleep_until, transport_end_reason, WebSocketSink,
        WebSocketSource,
    },
    connection_reauth::{
        perform_reauthentication, ReauthOutcome, RelayAuthState, RelaySessionIdentity,
    },
    observe::ConnectionTrace,
    registry::{PairNotice, WaitingRegistration},
};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn wait_for_pair(
    trace: &mut ConnectionTrace<'_>,
    sink: &mut WebSocketSink,
    source: &mut WebSocketSource,
    waiting: &mut WaitingRegistration,
    config: &RelayConfig,
    auth_state: &mut RelayAuthState,
    configured_deadline: Instant,
    strict_reauth: bool,
    authenticator: Arc<dyn RelayAuthenticator>,
    reauth_in_flight: Arc<Semaphore>,
    identity: &RelaySessionIdentity,
    shutdown: &mut watch::Receiver<bool>,
) -> Option<PairNotice> {
    let waiting_deadline = Instant::now() + config.waiting_timeout;
    let mut idle = sleep_until(Instant::now() + config.idle_timeout);
    let mut waiting_timeout = sleep_until(waiting_deadline);
    let mut lifetime = sleep_until(auth_state.effective_deadline(configured_deadline));
    // The waiting lease. `lease_ping` asks the peer to prove it is still there;
    // the pong it sends back renews `waiting_timeout`. Disabled by
    // configuration, the timers below simply never fire and the waiting window
    // stays the fixed countdown it has always been.
    let lease_interval = config.lease_ping_interval();
    let mut lease_ping = sleep_until(Instant::now() + lease_interval);
    let mut reauth_at = strict_reauth
        .then(|| {
            auth_state.reauth_at(
                Instant::now(),
                configured_deadline,
                identity.locator_expiry_unix(),
            )
        })
        .flatten();
    let mut reauth = sleep_until(
        reauth_at.unwrap_or_else(|| auth_state.effective_deadline(configured_deadline)),
    );

    loop {
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                if changed.is_ok() && *shutdown.borrow() {
                    close_out(trace, sink, source, RelayCloseReason::Shutdown).await;
                    return None;
                }
            }
            () = &mut lifetime => {
                reject(trace, sink, source, RelayRejectCode::AuthenticationFailed).await;
                return None;
            }
            () = &mut reauth, if reauth_at.is_some() => {
                match perform_reauthentication(
                    sink,
                    source,
                    Arc::clone(&authenticator),
                    Arc::clone(&reauth_in_flight),
                    identity,
                    *auth_state,
                    configured_deadline,
                    config.handshake_timeout,
                    None,
                ).await {
                    Ok(ReauthOutcome::Extended(extended)) => {
                        *auth_state = extended;
                        lifetime.as_mut().reset(
                            auth_state.effective_deadline(configured_deadline),
                        );
                        reauth_at = auth_state.reauth_at(
                            Instant::now(),
                            configured_deadline,
                            identity.locator_expiry_unix(),
                        );
                        reauth.as_mut().reset(
                            reauth_at.unwrap_or_else(|| {
                                auth_state.effective_deadline(configured_deadline)
                            }),
                        );
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Ok(ReauthOutcome::StillCurrent) => {
                        reauth_at = auth_state.reauth_at(
                            Instant::now(),
                            configured_deadline,
                            identity.locator_expiry_unix(),
                        );
                        reauth.as_mut().reset(
                            reauth_at.unwrap_or_else(|| {
                                auth_state.effective_deadline(configured_deadline)
                            }),
                        );
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Err(()) => {
                        close_out(
                            trace,
                            sink,
                            source,
                            RelayCloseReason::ReauthenticationFailed,
                        ).await;
                        return None;
                    }
                }
            }
            () = &mut lease_ping, if config.waiting_lease => {
                if sink.send(Message::Ping(Vec::new())).await.is_err() {
                    trace.closed(RelayCloseReason::PeerReset);
                    return None;
                }
                lease_ping.as_mut().reset(Instant::now() + lease_interval);
            }
            () = &mut waiting_timeout => {
                reject(trace, sink, source, RelayRejectCode::PairingTimeout).await;
                return None;
            }
            () = &mut idle => {
                close_out(trace, sink, source, RelayCloseReason::IdleTimeout).await;
                return None;
            }
            result = &mut waiting.pair_rx => {
                return result.ok();
            }
            message = source.next() => {
                match message {
                    Some(Ok(Message::Ping(payload))) => {
                        if sink.send(Message::Pong(payload)).await.is_err() {
                            trace.closed(RelayCloseReason::PeerReset);
                            return None;
                        }
                        reset_idle(&mut idle, config.idle_timeout);
                        // A peer keeping itself alive is proving liveness just
                        // as well as one answering our ping.
                        renew_lease(config, &mut waiting_timeout);
                    }
                    Some(Ok(Message::Pong(_))) => {
                        reset_idle(&mut idle, config.idle_timeout);
                        renew_lease(config, &mut waiting_timeout);
                    }
                    Some(Ok(Message::Close(frame))) => {
                        trace.closed(RelayCloseReason::PeerClosed);
                        let _ = sink.send(Message::Close(frame)).await;
                        return None;
                    }
                    Some(Ok(Message::Binary(_) | Message::Text(_) | Message::Frame(_))) => {
                        // No application payload is accepted before both peers
                        // have received Paired.
                        reject(trace, sink, source, RelayRejectCode::MalformedHello).await;
                        return None;
                    }
                    Some(Err(error)) => {
                        trace.closed(transport_end_reason(Some(&error)));
                        return None;
                    }
                    None => {
                        trace.closed(RelayCloseReason::PeerEof);
                        return None;
                    }
                }
            }
        }
    }
}

/// Restart the waiting countdown for a peer that has just proved it is alive.
///
/// The renewal is deliberately NOT capped here. The `lifetime` arm above
/// already ends the connection at `RelayAuthState::effective_deadline`, which
/// folds in both the authentication expiry and `tunnel_lifetime`, so a lease
/// cannot outlive the credential that opened it — and letting that arm win
/// keeps the reported reason honest about which clock actually expired.
fn renew_lease(config: &RelayConfig, waiting_timeout: &mut Pin<Box<Sleep>>) {
    if !config.waiting_lease {
        return;
    }
    waiting_timeout
        .as_mut()
        .reset(Instant::now() + config.waiting_timeout);
}
