use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{header::AUTHORIZATION, HeaderMap};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};

use op_collab_relay_protocol::{
    RelayReauthChallengeV1, RelayRejectCode, RelayRole, RelayServerChallengeV1,
    RelayWaitingAdvertisementV1, VerifiedRelayRoute, MAX_RELAY_REAUTH_RESPONSE_TEXT_BYTES,
    RELAY_CHALLENGE_HEADER_NAME, RELAY_WAITING_HEADER_NAME,
};

use crate::auth::{AuthMode, RelayAuthAttempt, RelayCredential};
use crate::endpoint::RelayEndpoint;
use crate::error::{RelayFailureKind, TunnelError};
use crate::limits::RelayLimits;
use crate::reauth_budget::ReauthBudget;

pub(crate) type RelaySocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn rustls_connector() -> Connector {
    let mut roots = rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    let _ = roots.add_parsable_certificates(native.certs);
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("ring supports default TLS protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    Connector::Rustls(Arc::new(config))
}

#[derive(Clone, Copy)]
pub(crate) struct ClientReauthContext<'a> {
    pub(crate) auth: &'a AuthMode,
    pub(crate) role: RelayRole,
    pub(crate) route: &'a VerifiedRelayRoute,
}

pub(crate) struct RelayUpgrade {
    pub(crate) socket: RelaySocket,
    pub(crate) challenge_attempt: Option<(RelayAuthAttempt, RelayServerChallengeV1)>,
    /// The relay's waiting-window capability, when it advertised one.
    ///
    /// Optional by design: a relay that predates the capability, or an
    /// intermediary that strips unknown headers, simply leaves the client on
    /// its compiled-in fallback rather than breaking the connection.
    pub(crate) waiting: Option<RelayWaitingAdvertisementV1>,
}

pub(crate) async fn connect_socket(
    endpoint: &RelayEndpoint,
    auth: &AuthMode,
    cancel: &mut watch::Receiver<bool>,
    limits: RelayLimits,
) -> Result<RelayUpgrade, TunnelError> {
    let mut request = endpoint
        .as_str()
        .into_client_request()
        .map_err(|_| TunnelError::Failure(RelayFailureKind::Protocol))?;
    let mut challenge_attempt = None;
    let mut reduced_credential: Option<RelayCredential> = None;
    match auth {
        AuthMode::ChallengeBound(authenticator) => {
            let attempt = authenticator
                .begin_attempt()
                .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))?;
            let value = attempt
                .authorization_header()
                .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))?;
            request.headers_mut().insert(AUTHORIZATION, value);
            challenge_attempt = Some(attempt);
        }
        AuthMode::TicketBindingOnly(provider) => {
            let credential = provider
                .credential()
                .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))?;
            let value = credential
                .authorization_header()
                .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))?;
            request.headers_mut().insert(AUTHORIZATION, value);
            reduced_credential = Some(credential);
        }
        #[cfg(any(test, debug_assertions))]
        AuthMode::DevelopmentAnonymous => {}
    }

    let max_wire_message_bytes = limits
        .max_binary_bytes
        .max(MAX_RELAY_REAUTH_RESPONSE_TEXT_BYTES);
    let config = WebSocketConfig {
        write_buffer_size: limits.max_binary_bytes,
        max_write_buffer_size: max_wire_message_bytes.saturating_mul(3),
        max_message_size: Some(max_wire_message_bytes),
        max_frame_size: Some(max_wire_message_bytes),
        ..WebSocketConfig::default()
    };

    let connect =
        connect_async_tls_with_config(request, Some(config), true, Some(rustls_connector()));
    tokio::select! {
        _ = cancelled(cancel) => Err(TunnelError::Cancelled),
        result = tokio::time::timeout(limits.connect, connect) => {
            match result {
                Err(_) => Err(TunnelError::Failure(RelayFailureKind::ConnectTimeout)),
                Ok(Err(_)) => Err(TunnelError::Failure(RelayFailureKind::Connect)),
                Ok(Ok((socket, response))) => {
                    let challenge_attempt = match challenge_attempt {
                        Some(attempt) => {
                            let challenge = parse_challenge(response.headers())?;
                            Some((attempt, challenge))
                        }
                        None => None,
                    };
                    drop(reduced_credential);
                    Ok(RelayUpgrade {
                        socket,
                        challenge_attempt,
                        waiting: parse_waiting_advertisement(response.headers()),
                    })
                }
            }
        }
    }
}

fn parse_challenge(headers: &HeaderMap) -> Result<RelayServerChallengeV1, TunnelError> {
    let mut values = headers.get_all(RELAY_CHALLENGE_HEADER_NAME).iter();
    let value = values
        .next()
        .ok_or(TunnelError::Failure(RelayFailureKind::Authentication))?;
    if values.next().is_some() {
        return Err(TunnelError::Failure(RelayFailureKind::Authentication));
    }
    let value = value
        .to_str()
        .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))?;
    RelayServerChallengeV1::decode_header(value)
        .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))
}

/// Read the relay's waiting capability from the upgrade response.
///
/// Never fails the connection. A missing, duplicated, or malformed value is
/// treated exactly like an absent one, because the capability is an
/// optimisation and the fallback is always safe.
fn parse_waiting_advertisement(headers: &HeaderMap) -> Option<RelayWaitingAdvertisementV1> {
    let mut values = headers.get_all(RELAY_WAITING_HEADER_NAME).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    RelayWaitingAdvertisementV1::decode_header(value.to_str().ok()?).ok()
}

pub(crate) async fn send_binary(
    socket: &mut RelaySocket,
    bytes: Vec<u8>,
    cancel: &mut watch::Receiver<bool>,
    timeout: Duration,
    timeout_kind: RelayFailureKind,
) -> Result<(), TunnelError> {
    if bytes.is_empty() {
        return Err(TunnelError::Failure(RelayFailureKind::Protocol));
    }
    tokio::select! {
        _ = cancelled(cancel) => Err(TunnelError::Cancelled),
        result = tokio::time::timeout(timeout, socket.send(Message::Binary(bytes))) => {
            match result {
                Err(_) => Err(TunnelError::Failure(timeout_kind)),
                Ok(Err(error)) => Err(map_websocket_error(error)),
                Ok(Ok(())) => Ok(()),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn pump(
    mut socket: RelaySocket,
    local: &mut TcpStream,
    reauth: ClientReauthContext<'_>,
    reauth_budget: &mut ReauthBudget,
    cancel: &mut watch::Receiver<bool>,
    started_at: Instant,
    limits: RelayLimits,
) -> Result<(), TunnelError> {
    let lifetime_deadline = started_at + limits.lifetime;
    let now = Instant::now();
    let mut idle_deadline = now + limits.idle;
    let mut keepalive_deadline = now + limits.keepalive;
    // A successful Ping write proves only that the local socket still accepts
    // bytes. Keep one fixed response deadline until any peer frame arrives, so
    // repeatedly writable buffers cannot mask a dead relay path forever.
    let mut pong_deadline = None;
    let mut buffer = vec![0_u8; limits.max_binary_bytes];
    let mut transferred = 0_u64;

    loop {
        if *cancel.borrow() {
            return Err(TunnelError::Cancelled);
        }
        let now = Instant::now();
        if now >= lifetime_deadline {
            return Err(TunnelError::Failure(RelayFailureKind::LifetimeExceeded));
        }
        if now >= idle_deadline || pong_deadline.is_some_and(|deadline| now >= deadline) {
            return Err(TunnelError::Failure(RelayFailureKind::IdleTimeout));
        }
        if pong_deadline.is_none() && now >= keepalive_deadline {
            send_before_deadlines(
                &mut socket,
                Message::Ping(Vec::new()),
                cancel,
                idle_deadline,
                lifetime_deadline,
            )
            .await?;
            let now = Instant::now();
            idle_deadline = now + limits.idle;
            keepalive_deadline = now + limits.keepalive;
            pong_deadline = Some(now + limits.idle);
            continue;
        }
        let next_deadline = lifetime_deadline
            .min(idle_deadline)
            .min(pong_deadline.unwrap_or(keepalive_deadline));
        tokio::select! {
            _ = cancelled(cancel) => return Err(TunnelError::Cancelled),
            _ = tokio::time::sleep_until(next_deadline) => {}
            read = local.read(&mut buffer) => {
                let read = read.map_err(|_| TunnelError::Failure(RelayFailureKind::LocalIo))?;
                if read == 0 {
                    close_socket(&mut socket, cancel, limits.idle).await;
                    return Ok(());
                }
                transferred = add_transferred(transferred, read, limits.max_connection_bytes)?;
                send_before_deadlines(
                    &mut socket,
                    Message::Binary(buffer[..read].to_vec()),
                    cancel,
                    idle_deadline,
                    lifetime_deadline,
                ).await?;
                let now = Instant::now();
                idle_deadline = now + limits.idle;
            }
            message = socket.next() => {
                match message {
                    Some(Ok(Message::Binary(bytes))) => {
                        if bytes.is_empty() {
                            return Err(TunnelError::Failure(RelayFailureKind::Protocol));
                        }
                        if bytes.len() > limits.max_binary_bytes {
                            return Err(TunnelError::Failure(RelayFailureKind::BinaryFrameTooLarge));
                        }
                        transferred = add_transferred(
                            transferred,
                            bytes.len(),
                            limits.max_connection_bytes,
                        )?;
                        write_before_deadlines(
                            local,
                            &bytes,
                            cancel,
                            idle_deadline,
                            lifetime_deadline,
                        ).await?;
                        note_peer_activity(
                            &mut idle_deadline,
                            &mut keepalive_deadline,
                            &mut pong_deadline,
                            limits,
                        );
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        send_before_deadlines(
                            &mut socket,
                            Message::Pong(bytes),
                            cancel,
                            idle_deadline,
                            lifetime_deadline,
                        ).await?;
                        note_peer_activity(
                            &mut idle_deadline,
                            &mut keepalive_deadline,
                            &mut pong_deadline,
                            limits,
                        );
                    }
                    Some(Ok(Message::Pong(_))) => {
                        note_peer_activity(
                            &mut idle_deadline,
                            &mut keepalive_deadline,
                            &mut pong_deadline,
                            limits,
                        );
                    }
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Ok(Message::Text(text))) => {
                        answer_reauth_challenge(
                            &mut socket,
                            &text,
                            reauth,
                            reauth_budget,
                            cancel,
                            idle_deadline,
                            lifetime_deadline,
                        ).await?;
                        note_peer_activity(
                            &mut idle_deadline,
                            &mut keepalive_deadline,
                            &mut pong_deadline,
                            limits,
                        );
                    }
                    Some(Ok(Message::Frame(_))) => {
                        return Err(TunnelError::Failure(RelayFailureKind::Protocol));
                    }
                    Some(Err(error)) => return Err(map_websocket_error(error)),
                }
            }
        }
    }
}

fn note_peer_activity(
    idle_deadline: &mut Instant,
    keepalive_deadline: &mut Instant,
    pong_deadline: &mut Option<Instant>,
    limits: RelayLimits,
) {
    let now = Instant::now();
    *idle_deadline = now + limits.idle;
    *keepalive_deadline = now + limits.keepalive;
    *pong_deadline = None;
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn next_binary_with_reauth(
    socket: &mut RelaySocket,
    reauth: ClientReauthContext<'_>,
    reauth_budget: &mut ReauthBudget,
    cancel: &mut watch::Receiver<bool>,
    timeout: Duration,
    timeout_kind: RelayFailureKind,
    max_binary_bytes: usize,
) -> Result<Vec<u8>, TunnelError> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(TunnelError::Failure(timeout_kind))?;
        let message = tokio::select! {
            _ = cancelled(cancel) => return Err(TunnelError::Cancelled),
            result = tokio::time::timeout(remaining, socket.next()) => {
                result.map_err(|_| TunnelError::Failure(timeout_kind))?
            }
        };
        match message {
            Some(Ok(Message::Binary(bytes))) => {
                if bytes.is_empty() {
                    return Err(TunnelError::Failure(RelayFailureKind::Protocol));
                }
                if bytes.len() > max_binary_bytes {
                    return Err(TunnelError::Failure(RelayFailureKind::BinaryFrameTooLarge));
                }
                return Ok(bytes);
            }
            Some(Ok(Message::Text(text))) => {
                answer_reauth_challenge(
                    socket,
                    &text,
                    reauth,
                    reauth_budget,
                    cancel,
                    deadline,
                    deadline,
                )
                .await?;
            }
            Some(Ok(Message::Ping(bytes))) => {
                send_before_deadlines(socket, Message::Pong(bytes), cancel, deadline, deadline)
                    .await?;
            }
            Some(Ok(Message::Pong(_))) => {}
            Some(Ok(Message::Close(frame))) => return Err(close_error(frame.as_ref())),
            None => return Err(TunnelError::Failure(RelayFailureKind::Closed)),
            Some(Ok(Message::Frame(_))) => {
                return Err(TunnelError::Failure(RelayFailureKind::Protocol));
            }
            Some(Err(error)) => return Err(map_websocket_error(error)),
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn answer_reauth_challenge(
    socket: &mut RelaySocket,
    text: &str,
    reauth: ClientReauthContext<'_>,
    reauth_budget: &mut ReauthBudget,
    cancel: &mut watch::Receiver<bool>,
    idle_deadline: Instant,
    lifetime_deadline: Instant,
) -> Result<(), TunnelError> {
    let AuthMode::ChallengeBound(authenticator) = reauth.auth else {
        return Err(TunnelError::Failure(RelayFailureKind::TextFrame));
    };
    let challenge = RelayReauthChallengeV1::decode_text(text)
        .map_err(|_| TunnelError::Failure(RelayFailureKind::TextFrame))?
        .into_challenge();
    // The budget is charged only on the path that actually mints a bearer, so
    // a relay cannot spend it with frames that never reach the credential
    // provider, and the reduced-assurance modes keep their existing rejection.
    reauth_budget.admit(Instant::now())?;
    let attempt = authenticator
        .begin_attempt()
        .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))?;
    let response = attempt
        .finish(&challenge, reauth.role, reauth.route)
        .and_then(|response| response.into_reauth_response(challenge))
        .map_err(|_| TunnelError::Failure(RelayFailureKind::Authentication))?;
    let encoded = response.encode_text();
    send_before_deadlines(
        socket,
        Message::Text(encoded.to_string()),
        cancel,
        idle_deadline,
        lifetime_deadline,
    )
    .await
}

async fn send_before_deadlines(
    socket: &mut RelaySocket,
    message: Message,
    cancel: &mut watch::Receiver<bool>,
    idle_deadline: Instant,
    lifetime_deadline: Instant,
) -> Result<(), TunnelError> {
    tokio::select! {
        _ = cancelled(cancel) => Err(TunnelError::Cancelled),
        _ = tokio::time::sleep_until(lifetime_deadline) => {
            Err(TunnelError::Failure(RelayFailureKind::LifetimeExceeded))
        }
        _ = tokio::time::sleep_until(idle_deadline) => {
            Err(TunnelError::Failure(RelayFailureKind::IdleTimeout))
        }
        result = socket.send(message) => result.map_err(map_websocket_error),
    }
}

async fn write_before_deadlines(
    local: &mut TcpStream,
    bytes: &[u8],
    cancel: &mut watch::Receiver<bool>,
    idle_deadline: Instant,
    lifetime_deadline: Instant,
) -> Result<(), TunnelError> {
    tokio::select! {
        _ = cancelled(cancel) => Err(TunnelError::Cancelled),
        _ = tokio::time::sleep_until(lifetime_deadline) => {
            Err(TunnelError::Failure(RelayFailureKind::LifetimeExceeded))
        }
        _ = tokio::time::sleep_until(idle_deadline) => {
            Err(TunnelError::Failure(RelayFailureKind::IdleTimeout))
        }
        result = local.write_all(bytes) => {
            result.map_err(|_| TunnelError::Failure(RelayFailureKind::LocalIo))
        }
    }
}

async fn close_socket(
    socket: &mut RelaySocket,
    cancel: &mut watch::Receiver<bool>,
    timeout: Duration,
) {
    tokio::select! {
        _ = cancelled(cancel) => {}
        _ = tokio::time::sleep(timeout) => {}
        _ = socket.close(None) => {}
    }
}

fn add_transferred(current: u64, next: usize, maximum: u64) -> Result<u64, TunnelError> {
    let total = current
        .checked_add(next as u64)
        .ok_or(TunnelError::Failure(RelayFailureKind::ByteLimitExceeded))?;
    if total > maximum {
        return Err(TunnelError::Failure(RelayFailureKind::ByteLimitExceeded));
    }
    Ok(total)
}

pub(crate) async fn cancelled(cancel: &mut watch::Receiver<bool>) {
    if *cancel.borrow() {
        return;
    }
    while cancel.changed().await.is_ok() {
        if *cancel.borrow() {
            return;
        }
    }
}

pub(crate) async fn sleep_or_cancel(
    duration: Duration,
    cancel: &mut watch::Receiver<bool>,
) -> bool {
    tokio::select! {
        _ = cancelled(cancel) => false,
        _ = tokio::time::sleep(duration) => true,
    }
}

/// Classify a close frame the relay sent while this end was still waiting for
/// a status frame.
///
/// The relay repeats a rejection's reason inside the close frame, so a close
/// that carries a `relay-reject:` token is a rejection with a known cause, not
/// an anonymous disconnect. Anything else stays
/// [`RelayFailureKind::Closed`].
fn close_error(frame: Option<&CloseFrame<'_>>) -> TunnelError {
    match frame.and_then(|frame| RelayRejectCode::from_close_reason(frame.reason.as_ref())) {
        Some(code) => TunnelError::Failure(RelayFailureKind::from_reject(code)),
        None => TunnelError::Failure(RelayFailureKind::Closed),
    }
}

pub(crate) fn map_websocket_error(error: WebSocketError) -> TunnelError {
    match error {
        WebSocketError::Capacity(_) => TunnelError::Failure(RelayFailureKind::BinaryFrameTooLarge),
        WebSocketError::Protocol(_) | WebSocketError::Utf8 => {
            TunnelError::Failure(RelayFailureKind::Protocol)
        }
        WebSocketError::ConnectionClosed | WebSocketError::AlreadyClosed => {
            TunnelError::Failure(RelayFailureKind::Closed)
        }
        _ => TunnelError::Failure(RelayFailureKind::RelayIo),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use op_collab_relay_protocol::RELAY_REJECT_CODES;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

    use super::*;

    #[test]
    fn relay_connector_stays_on_rustls_when_native_tls_is_also_linked() {
        assert!(matches!(rustls_connector(), Connector::Rustls(_)));
    }

    fn frame(reason: &'static str) -> CloseFrame<'static> {
        CloseFrame {
            code: CloseCode::Policy,
            reason: Cow::Borrowed(reason),
        }
    }

    #[test]
    fn a_relay_close_reason_recovers_the_reject_code_it_carries() {
        for code in RELAY_REJECT_CODES {
            let expected = RelayFailureKind::from_reject(code);
            assert_eq!(
                close_error(Some(&frame(code.close_reason()))),
                TunnelError::Failure(expected)
            );
        }
        // A pairing timeout is the one the client must never confuse with a
        // real rejection or with a generic protocol fault.
        assert_eq!(
            close_error(Some(&frame(RelayRejectCode::PairingTimeout.close_reason()))),
            TunnelError::Failure(RelayFailureKind::RejectedPairingTimeout)
        );
    }

    #[test]
    fn an_ordinary_close_stays_an_anonymous_disconnect() {
        assert_eq!(
            close_error(None),
            TunnelError::Failure(RelayFailureKind::Closed)
        );
        for reason in ["idle timeout", "relay shutdown", "", "relay-reject:"] {
            assert_eq!(
                close_error(Some(&CloseFrame {
                    code: CloseCode::Away,
                    reason: Cow::Borrowed(reason),
                })),
                TunnelError::Failure(RelayFailureKind::Closed)
            );
        }
    }
}
