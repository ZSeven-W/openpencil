use std::{
    num::NonZeroU64,
    sync::{Arc, Mutex},
};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use op_collab_relay_protocol::{
    is_valid_relay_bearer_token, RelayClientHello, RelayProtocolError, RelayRejectCode,
    RelayServerStatus, MAX_RELAY_BEARER_BYTES, MAX_RELAY_REAUTH_RESPONSE_TEXT_BYTES,
    RELAY_CHALLENGE_HEADER_NAME, RELAY_WAITING_HEADER_NAME,
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch, OwnedSemaphorePermit, Semaphore},
    time::{self, Instant, Sleep},
};
use tokio_tungstenite::{
    accept_hdr_async_with_config,
    tungstenite::{
        handshake::server::{ErrorResponse, Request, Response},
        http::HeaderValue,
        protocol::WebSocketConfig,
        Message,
    },
    WebSocketStream,
};
use zeroize::Zeroizing;

use crate::{
    auth::{AuthenticatedRoute, RelayAuthenticator, RelayBearerCredential, RelayUpgradeChallenge},
    close::{close, close_and_linger, reject_and_close, send_status, RelayCloseReason},
    config::RelayConfig,
    connection_reauth::{
        perform_reauthentication, ReauthOutcome, ReauthRelayTraffic, RelayAuthState,
        RelaySessionIdentity,
    },
    connection_wait::wait_for_pair,
    observe::{ConnectionTrace, HandshakeStage, RelayCensus},
    peer_quota::PeerQuotaPermit,
    registry::{PairNotice, QueuedPayload, Registration, Registry},
};

type WebSocket = WebSocketStream<TcpStream>;
pub(crate) type WebSocketSink = SplitSink<WebSocket, Message>;
pub(crate) type WebSocketSource = SplitStream<WebSocket>;

// The bearer ceiling must admit every credential shape the relay accepts.
// This is a relation, not an equality: while dual-accept is on, a bearer may
// still be a full collaboration ticket, and once clients have moved it will
// be the much smaller claim-minimized relay token.
//
// Do NOT shrink `MAX_RELAY_BEARER_BYTES` to the relay-token bound here. It
// feeds `MAX_RELAY_REAUTH_RESPONSE_TEXT_BYTES`, which sizes `max_message_size`
// on BOTH ends of the socket, so a one-sided shrink breaks online
// reauthentication with an opaque capacity error. Retiring the larger ceiling
// is a separate, coordinated change.
const _: () = assert!(MAX_RELAY_BEARER_BYTES >= op_auth_bridge::MAX_COLLAB_TICKET_BYTES);
const _: () = assert!(MAX_RELAY_BEARER_BYTES >= op_auth_bridge::MAX_COLLAB_RELAY_TOKEN_BYTES);

pub(crate) struct ConnectionServices {
    pub(crate) config: Arc<RelayConfig>,
    pub(crate) registry: Registry,
    pub(crate) authenticator: Arc<dyn RelayAuthenticator>,
    pub(crate) auth_in_flight: Arc<Semaphore>,
    pub(crate) reauth_in_flight: Arc<Semaphore>,
    pub(crate) queued_bytes: Arc<Semaphore>,
    pub(crate) census: Arc<RelayCensus>,
}

/// The capacity a connection occupies until it pairs.
///
/// Both permits cover the same span — the pre-pairing phase — so they are
/// released together. Once paired, the connection's cost is bounded by
/// `max_active_pairs` and the queue budgets instead.
pub(crate) struct AdmissionPermit {
    _pending: OwnedSemaphorePermit,
    _source: PeerQuotaPermit,
}

impl AdmissionPermit {
    pub(crate) fn new(pending: OwnedSemaphorePermit, source: PeerQuotaPermit) -> Self {
        Self {
            _pending: pending,
            _source: source,
        }
    }
}

pub(crate) async fn serve_connection(
    stream: TcpStream,
    services: ConnectionServices,
    mut shutdown: watch::Receiver<bool>,
    admission_permit: AdmissionPermit,
) {
    let ConnectionServices {
        config,
        registry,
        authenticator,
        auth_in_flight,
        reauth_in_flight,
        queued_bytes,
        census,
    } = services;
    let mut trace = ConnectionTrace::new(&census);
    trace.accepted();
    let started_at = Instant::now();
    let handshake_deadline = started_at + config.handshake_timeout;
    let Ok(auth_permit) = Arc::clone(&auth_in_flight).try_acquire_owned() else {
        trace.handshake_failed(HandshakeStage::AuthConcurrency);
        return;
    };
    let challenge_authenticator = Arc::clone(&authenticator);
    let challenge_key = tokio::task::spawn_blocking(move || {
        (challenge_authenticator.challenge_key_id(), auth_permit)
    });
    let (challenge_key, auth_permit) =
        match time::timeout_at(handshake_deadline, challenge_key).await {
            Ok(Ok((Ok(key_id), permit))) => (key_id, permit),
            Ok(Ok((Err(_), _))) | Ok(Err(_)) | Err(_) => {
                trace.handshake_failed(HandshakeStage::ChallengeKey);
                return;
            }
        };
    let challenge = match challenge_key {
        Some(key_id) => {
            match RelayUpgradeChallenge::generate(key_id, handshake_deadline.into_std()) {
                Ok(challenge) => Some(challenge),
                Err(_) => {
                    trace.handshake_failed(HandshakeStage::ChallengeGeneration);
                    return;
                }
            }
        }
        None => None,
    };
    let strict_reauth = challenge.is_some();
    let challenge_header = challenge
        .as_ref()
        .map(|challenge| challenge.challenge().encode_header());
    let (websocket, credential) = match time::timeout_at(
        handshake_deadline,
        accept_tunnel(
            stream,
            config.max_message_bytes,
            config.unauthenticated_dev(),
            challenge_header,
            config.waiting_advertisement().encode_header(),
        ),
    )
    .await
    {
        Ok(Ok(websocket)) => websocket,
        Ok(Err(_)) => {
            trace.handshake_failed(HandshakeStage::Upgrade);
            return;
        }
        Err(_) => {
            trace.handshake_failed(HandshakeStage::UpgradeTimeout);
            return;
        }
    };

    let (mut sink, mut source) = websocket.split();
    let first = match time::timeout_at(handshake_deadline, source.next()).await {
        Ok(Some(Ok(Message::Binary(payload)))) => Zeroizing::new(payload),
        Ok(Some(Ok(_))) => {
            reject(
                &mut trace,
                &mut sink,
                &mut source,
                RelayRejectCode::MalformedHello,
            )
            .await;
            return;
        }
        Ok(Some(Err(_))) | Ok(None) | Err(_) => {
            trace.handshake_failed(HandshakeStage::Hello);
            return;
        }
    };
    let hello = match RelayClientHello::decode(&first) {
        Ok(hello) => hello,
        Err(error) => {
            reject(
                &mut trace,
                &mut sink,
                &mut source,
                hello_decode_reject_code(&error),
            )
            .await;
            return;
        }
    };
    drop(first);
    let session_hello = hello.clone();
    let initial_authenticator = Arc::clone(&authenticator);
    let authentication = tokio::task::spawn_blocking(move || {
        let _auth_permit = auth_permit;
        initial_authenticator.authenticate(&hello, credential.as_ref(), challenge)
    });
    let authentication = match time::timeout_at(handshake_deadline, authentication).await {
        Ok(Ok(authentication)) => authentication,
        Ok(Err(_)) | Err(_) => {
            reject(
                &mut trace,
                &mut sink,
                &mut source,
                RelayRejectCode::AuthenticationFailed,
            )
            .await;
            return;
        }
    };
    let AuthenticatedRoute {
        route,
        role,
        expires_at_unix,
    } = match authentication {
        Ok(route) => route,
        Err(code) => {
            reject(&mut trace, &mut sink, &mut source, code).await;
            return;
        }
    };
    let Some(expires_at_unix) =
        NonZeroU64::new(expires_at_unix.get().min(session_hello.expires_at_unix()))
    else {
        reject(
            &mut trace,
            &mut sink,
            &mut source,
            RelayRejectCode::AuthenticationFailed,
        )
        .await;
        return;
    };
    let Some(mut auth_state) = RelayAuthState::new(expires_at_unix) else {
        reject(
            &mut trace,
            &mut sink,
            &mut source,
            RelayRejectCode::AuthenticationFailed,
        )
        .await;
        return;
    };
    let Some(identity) = RelaySessionIdentity::new(route, role, &session_hello) else {
        reject(
            &mut trace,
            &mut sink,
            &mut source,
            RelayRejectCode::AuthenticationFailed,
        )
        .await;
        return;
    };
    trace.identify(role, &route);
    let configured_deadline = started_at
        .checked_add(config.tunnel_lifetime)
        .unwrap_or(auth_state.deadline());

    let (relay_tx, relay_rx) = mpsc::channel(config.relay_queue_capacity);
    let registration = match registry.register(route, role, relay_tx).await {
        Ok(registration) => registration,
        Err(_) => {
            reject(
                &mut trace,
                &mut sink,
                &mut source,
                RelayRejectCode::Capacity,
            )
            .await;
            return;
        }
    };
    if !send_status(&mut sink, RelayServerStatus::Ready).await {
        match &registration {
            Registration::Waiting(waiting) => registry.unregister_waiter(waiting).await,
            // The counterpart only reclaims the slot if it reads its pairing
            // notice, and its wait loop can time out in the same wake without
            // ever reading it. Release the pair here so the entry cannot leak.
            Registration::Paired(pair) => registry.close_pair(pair.pair_id).await,
        }
        return;
    }

    let mut pair = match registration {
        Registration::Paired(pair) => {
            drop(admission_permit);
            pair
        }
        Registration::Waiting(mut waiting) => {
            trace.registered(waiting.waiting_on_route);
            let pair = wait_for_pair(
                &mut trace,
                &mut sink,
                &mut source,
                &mut waiting,
                &config,
                &mut auth_state,
                configured_deadline,
                strict_reauth,
                Arc::clone(&authenticator),
                Arc::clone(&reauth_in_flight),
                &identity,
                &mut shutdown,
            )
            .await;
            registry.unregister_waiter(&waiting).await;
            drop(admission_permit);
            let Some(pair) = pair else {
                return;
            };
            pair
        }
    };

    if !send_status(&mut sink, RelayServerStatus::Paired).await {
        registry.close_pair(pair.pair_id).await;
        return;
    }
    trace.paired();
    let synchronize_timeout = auth_state
        .effective_deadline(configured_deadline)
        .saturating_duration_since(Instant::now())
        .min(config.handshake_timeout);
    if synchronize_timeout.is_zero() || !pair.synchronize(synchronize_timeout).await {
        close_out(
            &mut trace,
            &mut sink,
            &mut source,
            RelayCloseReason::PeerDisconnected,
        )
        .await;
        registry.close_pair(pair.pair_id).await;
        return;
    }
    relay_paired(
        &mut trace,
        sink,
        source,
        relay_rx,
        pair,
        queued_bytes,
        &registry,
        &config,
        auth_state,
        configured_deadline,
        strict_reauth,
        authenticator,
        reauth_in_flight,
        identity,
        &mut shutdown,
    )
    .await;
}

/// Reject a peer: record it, then deliver the reason and drain the close.
pub(crate) async fn reject(
    trace: &mut ConnectionTrace<'_>,
    sink: &mut WebSocketSink,
    source: &mut WebSocketSource,
    code: RelayRejectCode,
) {
    trace.rejected(code);
    reject_and_close(sink, source, code).await;
}

/// Close a peer: record it, then deliver the reason and drain the close.
pub(crate) async fn close_out(
    trace: &mut ConnectionTrace<'_>,
    sink: &mut WebSocketSink,
    source: &mut WebSocketSource,
    reason: RelayCloseReason,
) {
    trace.closed(reason);
    close_and_linger(sink, source, reason).await;
}

// Tungstenite fixes the handshake callback's error to a full HTTP response.
#[allow(clippy::result_large_err)]
async fn accept_tunnel(
    stream: TcpStream,
    max_message_bytes: usize,
    allow_anonymous_dev: bool,
    challenge_header: Option<String>,
    waiting_header: String,
) -> Result<(WebSocket, Option<RelayBearerCredential>), tokio_tungstenite::tungstenite::Error> {
    let credential = Arc::new(Mutex::new(None));
    let callback_credential = Arc::clone(&credential);
    let callback = move |request: &Request, mut response: Response| {
        if request.uri().path_and_query().map(|uri| uri.as_str()) != Some("/v1/tunnel") {
            return Err(http_error_response(404, "not found"));
        }
        // The waiting capability is not a secret and carries no identity, so
        // it rides every accepted upgrade — including the development path
        // that has no challenge to attach.
        let advertise = |response: &mut Response| {
            response.headers_mut().insert(
                RELAY_WAITING_HEADER_NAME,
                HeaderValue::from_str(&waiting_header)
                    .expect("waiting advertisement encoding is a valid HTTP header"),
            );
        };
        match parse_bearer(request) {
            Ok(Some(value)) => {
                *callback_credential
                    .lock()
                    .expect("relay credential mutex is not poisoned") = Some(value);
                if let Some(value) = challenge_header.as_deref() {
                    response.headers_mut().insert(
                        RELAY_CHALLENGE_HEADER_NAME,
                        HeaderValue::from_str(value)
                            .expect("protocol challenge encoding is a valid HTTP header"),
                    );
                }
                advertise(&mut response);
                Ok(response)
            }
            Ok(None) if allow_anonymous_dev => {
                debug_assert!(challenge_header.is_none());
                advertise(&mut response);
                Ok(response)
            }
            Ok(None) | Err(()) => Err(http_error_response(401, "unauthorized")),
        }
    };
    let max_wire_message_bytes = max_message_bytes.max(MAX_RELAY_REAUTH_RESPONSE_TEXT_BYTES);
    let config = WebSocketConfig {
        write_buffer_size: 8 * 1024,
        max_write_buffer_size: max_wire_message_bytes.saturating_mul(2) + 8 * 1024,
        max_message_size: Some(max_wire_message_bytes),
        max_frame_size: Some(max_wire_message_bytes),
        ..WebSocketConfig::default()
    };
    let websocket = accept_hdr_async_with_config(stream, callback, Some(config)).await?;
    let credential = credential
        .lock()
        .expect("relay credential mutex is not poisoned")
        .take();
    Ok((websocket, credential))
}

fn parse_bearer(request: &Request) -> Result<Option<RelayBearerCredential>, ()> {
    const PREFIX: &[u8] = b"Bearer ";

    let mut values = request.headers().get_all("authorization").iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(());
    }
    let value = value.as_bytes();
    let token = value.strip_prefix(PREFIX).ok_or(())?;
    if token.len() > MAX_RELAY_BEARER_BYTES || !is_rfc6750_b64token(token) {
        return Err(());
    }
    Ok(Some(RelayBearerCredential::new(token.to_vec())))
}

pub(crate) fn is_rfc6750_b64token(token: &[u8]) -> bool {
    is_valid_relay_bearer_token(token)
}

fn http_error_response(status: u16, message: &'static str) -> ErrorResponse {
    Response::builder()
        .status(status)
        .body(Some(message.to_string()))
        .expect("static HTTP rejection response is valid")
}

#[allow(clippy::too_many_arguments)]
async fn relay_paired(
    trace: &mut ConnectionTrace<'_>,
    mut sink: WebSocketSink,
    mut source: WebSocketSource,
    mut relay_rx: mpsc::Receiver<QueuedPayload>,
    pair: PairNotice,
    queued_bytes: Arc<Semaphore>,
    registry: &Registry,
    config: &RelayConfig,
    mut auth_state: RelayAuthState,
    configured_deadline: Instant,
    strict_reauth: bool,
    authenticator: Arc<dyn RelayAuthenticator>,
    reauth_in_flight: Arc<Semaphore>,
    identity: RelaySessionIdentity,
    shutdown: &mut watch::Receiver<bool>,
) {
    let mut idle = sleep_until(Instant::now() + config.idle_timeout);
    let mut lifetime = sleep_until(auth_state.effective_deadline(configured_deadline));
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
                    close_out(trace, &mut sink, &mut source, RelayCloseReason::Shutdown).await;
                    break;
                }
            }
            () = &mut lifetime => {
                close_out(
                    trace,
                    &mut sink,
                    &mut source,
                    RelayCloseReason::TunnelLifetime,
                ).await;
                break;
            }
            () = &mut reauth, if reauth_at.is_some() => {
                match perform_reauthentication(
                    &mut sink,
                    &mut source,
                    Arc::clone(&authenticator),
                    Arc::clone(&reauth_in_flight),
                    &identity,
                    auth_state,
                    configured_deadline,
                    config.handshake_timeout,
                    Some(&mut ReauthRelayTraffic {
                        relay_rx: &mut relay_rx,
                        peer_tx: &pair.peer_tx,
                        route_budget: &pair.route_budget,
                        queued_bytes: &queued_bytes,
                        max_message_bytes: config.max_message_bytes,
                    }),
                ).await {
                    Ok(ReauthOutcome::Extended(extended)) => {
                        auth_state = extended;
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
                            &mut sink,
                            &mut source,
                            RelayCloseReason::ReauthenticationFailed,
                        ).await;
                        break;
                    }
                }
            }
            () = &mut idle => {
                close_out(trace, &mut sink, &mut source, RelayCloseReason::IdleTimeout).await;
                break;
            }
            message = source.next() => {
                match message {
                    Some(Ok(Message::Binary(payload))) => {
                        if payload.len() > config.max_message_bytes {
                            trace.closed(RelayCloseReason::MessageTooLarge);
                            close(&mut sink, RelayCloseReason::MessageTooLarge).await;
                            break;
                        }
                        let permits = u32::try_from(payload.len())
                            .expect("payload length is bounded to 64 KiB");
                        let Ok(global_budget) = Arc::clone(&queued_bytes)
                            .try_acquire_many_owned(permits)
                        else {
                            trace.closed(RelayCloseReason::Backpressure);
                            close(&mut sink, RelayCloseReason::Backpressure).await;
                            break;
                        };
                        let Ok(route_budget) = Arc::clone(&pair.route_budget)
                            .try_acquire_many_owned(permits)
                        else {
                            trace.closed(RelayCloseReason::Backpressure);
                            close(&mut sink, RelayCloseReason::Backpressure).await;
                            break;
                        };
                        let queued =
                            QueuedPayload::new(payload, global_budget, route_budget);
                        if pair.peer_tx.try_send(queued).is_err() {
                            trace.closed(RelayCloseReason::Backpressure);
                            close(&mut sink, RelayCloseReason::Backpressure).await;
                            break;
                        }
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Some(Ok(Message::Text(_))) => {
                        trace.closed(RelayCloseReason::TextUnsupported);
                        close(&mut sink, RelayCloseReason::TextUnsupported).await;
                        break;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if sink.send(Message::Pong(payload)).await.is_err() {
                            trace.closed(RelayCloseReason::PeerReset);
                            break;
                        }
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Some(Ok(Message::Pong(_))) => {
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Some(Ok(Message::Close(frame))) => {
                        trace.closed(RelayCloseReason::PeerClosed);
                        let _ = sink.send(Message::Close(frame)).await;
                        break;
                    }
                    Some(Ok(Message::Frame(_))) => {
                        trace.closed(RelayCloseReason::UnexpectedFrame);
                        close(&mut sink, RelayCloseReason::UnexpectedFrame).await;
                        break;
                    }
                    Some(Err(error)) => {
                        trace.closed(transport_end_reason(Some(&error)));
                        break;
                    }
                    None => {
                        trace.closed(RelayCloseReason::PeerEof);
                        break;
                    }
                }
            }
            queued = relay_rx.recv() => {
                let Some(queued) = queued else {
                    trace.closed(RelayCloseReason::PeerDisconnected);
                    close(&mut sink, RelayCloseReason::PeerDisconnected).await;
                    break;
                };
                let (payload, _global_budget, _route_budget) =
                    queued.into_parts();
                if sink.send(Message::Binary(payload)).await.is_err() {
                    trace.closed(RelayCloseReason::PeerReset);
                    break;
                }
                reset_idle(&mut idle, config.idle_timeout);
            }
        }
    }

    registry.close_pair(pair.pair_id).await;
}

pub(crate) fn sleep_until(deadline: Instant) -> std::pin::Pin<Box<Sleep>> {
    Box::pin(time::sleep_until(deadline))
}

pub(crate) fn reset_idle(idle: &mut std::pin::Pin<Box<Sleep>>, timeout: std::time::Duration) {
    idle.as_mut().reset(Instant::now() + timeout);
}

/// Classify how a peer's transport ended.
///
/// A reset and an orderly end are both "the socket is gone", but only one of
/// them says something is severing tunnels.
pub(crate) fn transport_end_reason(
    error: Option<&tokio_tungstenite::tungstenite::Error>,
) -> RelayCloseReason {
    match error {
        None | Some(tokio_tungstenite::tungstenite::Error::ConnectionClosed) => {
            RelayCloseReason::PeerEof
        }
        Some(_) => RelayCloseReason::PeerReset,
    }
}

fn hello_decode_reject_code(error: &RelayProtocolError) -> RelayRejectCode {
    match error {
        RelayProtocolError::UnsupportedVersion { .. } => RelayRejectCode::UnsupportedVersion,
        _ => RelayRejectCode::MalformedHello,
    }
}
