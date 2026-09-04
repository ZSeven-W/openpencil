use std::{
    num::NonZeroU64,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_util::{SinkExt, StreamExt};
use op_collab_relay_protocol::{
    CallerDeviceDhPublic, ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic,
    RelayAuthExtensionV1, RelayChallengeKeyId, RelayClientHello, RelayLocatorVerifier,
    RelayProtocolError, RelayRegion, RelayRejectCode, RelayRole, RelayServerChallengeV1,
    RelayServerStatus, RouteCapability, RouteId, UnsignedRelayLocatorV1, VerifiedRelayRoute,
    MAX_PAIRING_LIFETIME_SECS, MAX_RELAY_BEARER_BYTES, RELAY_CHALLENGE_HEADER_NAME,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
    time,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest,
        http::{header::AUTHORIZATION, HeaderValue, StatusCode},
        Error as WebSocketError, Message,
    },
    MaybeTlsStream, WebSocketStream,
};

use crate::{
    connection::is_rfc6750_b64token, server::serve_listener, AuthenticatedRoute,
    RelayAuthenticator, RelayBearerCredential, RelayConfig, RelayServerError,
};

type ClientSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const TEST_BEARER: &[u8] = b"test-ticket";
struct AcceptAllLocatorSignatures;

impl RelayLocatorVerifier for AcceptAllLocatorSignatures {
    fn verify(
        &self,
        _key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        true
    }
}

struct TestAuthenticator;

impl RelayAuthenticator for TestAuthenticator {
    fn authenticate(
        &self,
        hello: &RelayClientHello,
        credential: Option<&RelayBearerCredential>,
        _challenge: Option<crate::RelayUpgradeChallenge>,
    ) -> Result<AuthenticatedRoute, RelayRejectCode> {
        let credential = credential.ok_or(RelayRejectCode::AuthenticationRequired)?;
        if credential.as_bytes() != TEST_BEARER {
            return Err(RelayRejectCode::AuthenticationFailed);
        }
        let now = unix_now();
        if hello.expires_at_unix().saturating_sub(now) > MAX_PAIRING_LIFETIME_SECS {
            return Err(RelayRejectCode::ExpiryTooFarFuture);
        }
        let verified = hello
            .verify_locator(&AcceptAllLocatorSignatures, now)
            .map_err(protocol_reject)?;
        Ok(AuthenticatedRoute::new(
            verified.route_map_key(),
            verified.role(),
            NonZeroU64::new(verified.route().locator().claims().expires_at_unix())
                .expect("test locator expiry is non-zero"),
        ))
    }
}

struct MaximumBearerAuthenticator;

impl RelayAuthenticator for MaximumBearerAuthenticator {
    fn authenticate(
        &self,
        hello: &RelayClientHello,
        credential: Option<&RelayBearerCredential>,
        _challenge: Option<crate::RelayUpgradeChallenge>,
    ) -> Result<AuthenticatedRoute, RelayRejectCode> {
        if credential
            .map(RelayBearerCredential::as_bytes)
            .is_none_or(|bearer| bearer.len() != MAX_RELAY_BEARER_BYTES)
        {
            return Err(RelayRejectCode::AuthenticationFailed);
        }
        let now = unix_now();
        let verified = hello
            .verify_locator(&AcceptAllLocatorSignatures, now)
            .map_err(protocol_reject)?;
        Ok(AuthenticatedRoute::new(
            verified.route_map_key(),
            verified.role(),
            NonZeroU64::new(verified.route().locator().claims().expires_at_unix())
                .expect("test locator expiry is non-zero"),
        ))
    }
}

struct ExpiringTestAuthenticator;

impl RelayAuthenticator for ExpiringTestAuthenticator {
    fn authenticate(
        &self,
        hello: &RelayClientHello,
        credential: Option<&RelayBearerCredential>,
        challenge: Option<crate::RelayUpgradeChallenge>,
    ) -> Result<AuthenticatedRoute, RelayRejectCode> {
        let authenticated = TestAuthenticator.authenticate(hello, credential, challenge)?;
        Ok(AuthenticatedRoute::new(
            authenticated.route,
            authenticated.role(),
            NonZeroU64::new(unix_now() + 2).expect("short test expiry is non-zero"),
        ))
    }
}

struct OverlongCustomAuthenticator;

impl RelayAuthenticator for OverlongCustomAuthenticator {
    fn authenticate(
        &self,
        hello: &RelayClientHello,
        credential: Option<&RelayBearerCredential>,
        challenge: Option<crate::RelayUpgradeChallenge>,
    ) -> Result<AuthenticatedRoute, RelayRejectCode> {
        let authenticated = TestAuthenticator.authenticate(hello, credential, challenge)?;
        Ok(AuthenticatedRoute::new(
            authenticated.route,
            authenticated.role(),
            NonZeroU64::new(unix_now() + 30).expect("test expiry is non-zero"),
        ))
    }
}

struct ChallengeHeaderAuthenticator;

impl RelayAuthenticator for ChallengeHeaderAuthenticator {
    fn challenge_key_id(&self) -> Result<Option<RelayChallengeKeyId>, RelayRejectCode> {
        Ok(Some(
            RelayChallengeKeyId::new("test-relay-key").expect("test challenge key id"),
        ))
    }

    fn authenticate(
        &self,
        _hello: &RelayClientHello,
        _credential: Option<&RelayBearerCredential>,
        _challenge: Option<crate::RelayUpgradeChallenge>,
    ) -> Result<AuthenticatedRoute, RelayRejectCode> {
        Err(RelayRejectCode::AuthenticationFailed)
    }
}

struct TestServer {
    address: std::net::SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), RelayServerError>>,
}

impl TestServer {
    async fn start(config: RelayConfig) -> Self {
        Self::start_with_authenticator(config, Arc::new(TestAuthenticator)).await
    }

    async fn start_with_authenticator(
        mut config: RelayConfig,
        authenticator: Arc<dyn RelayAuthenticator>,
    ) -> Self {
        config.listen = "127.0.0.1:0".parse().expect("loopback address");
        let listener = TcpListener::bind(config.listen)
            .await
            .expect("bind relay test listener");
        let address = listener.local_addr().expect("relay test address");
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(serve_listener(
            listener,
            config,
            authenticator,
            async move {
                let _ = shutdown_rx.await;
            },
        ));
        Self {
            address,
            shutdown,
            task,
        }
    }

    async fn stop(self) {
        let _ = self.shutdown.send(());
        self.task
            .await
            .expect("relay task joins")
            .expect("relay stops cleanly");
    }
}

fn test_config() -> RelayConfig {
    let mut config = RelayConfig::default();
    config.handshake_timeout = Duration::from_secs(1);
    config.waiting_timeout = Duration::from_millis(200);
    config.idle_timeout = Duration::from_secs(2);
    config.tunnel_lifetime = Duration::from_secs(5);
    config
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock is after Unix epoch")
        .as_secs()
}

fn make_hello(
    role: RelayRole,
    route_seed: u8,
    not_before: u64,
    expires_at: u64,
    verify_at: u64,
) -> RelayClientHello {
    let unsigned = UnsignedRelayLocatorV1::new(
        RelayRegion::Cn,
        RouteId::new([route_seed; 16]).expect("non-zero route id"),
        NonZeroU64::new(1).expect("non-zero generation"),
        OwnerNoiseStatic::new([0x44; 32]).expect("non-zero owner static"),
        ExpectedDiscoveryId::new("relay-server-test").expect("discovery id"),
        not_before,
        expires_at,
        LocatorKeyId::new("test-locator-key").expect("key id"),
    )
    .expect("locator claims");
    let locator = unsigned
        .attach_signature(LocatorSignature::new([0x55; 64]).expect("non-zero fake signature"));
    let locator = locator
        .verify(&AcceptAllLocatorSignatures, verify_at)
        .expect("test locator verifies at construction time");
    let route = VerifiedRelayRoute::new(
        locator,
        RouteCapability::new([route_seed.wrapping_add(1); 32]).expect("non-zero route capability"),
    );
    RelayClientHello::new(
        role,
        &route,
        RelayAuthExtensionV1::without_possession_proof(
            CallerDeviceDhPublic::new([0x66; 32]).expect("caller DH public"),
        ),
    )
}

fn make_valid_hello(role: RelayRole, route_seed: u8) -> RelayClientHello {
    let now = unix_now();
    make_hello(role, route_seed, now.saturating_sub(5), now + 600, now)
}

fn make_too_far_future_hello(role: RelayRole, route_seed: u8) -> RelayClientHello {
    let now = unix_now();
    make_hello(role, route_seed, now + 60, now + 3_660, now + 60)
}

async fn connect(
    address: std::net::SocketAddr,
    path: &str,
) -> Result<ClientSocket, WebSocketError> {
    connect_with_authorization_values(address, path, &["Bearer test-ticket".to_owned()]).await
}

/// Connect and keep the upgrade response, for capability-header assertions.
async fn connect_with_response(
    address: std::net::SocketAddr,
) -> (
    ClientSocket,
    tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
) {
    let mut request = format!("ws://{address}/v1/tunnel")
        .into_client_request()
        .expect("test WebSocket request");
    request.headers_mut().append(
        AUTHORIZATION,
        HeaderValue::from_static("Bearer test-ticket"),
    );
    connect_async(request).await.expect("connect relay")
}

async fn connect_with_authorization_values(
    address: std::net::SocketAddr,
    path: &str,
    authorization_values: &[String],
) -> Result<ClientSocket, WebSocketError> {
    let mut request = format!("ws://{address}{path}")
        .into_client_request()
        .expect("test WebSocket request");
    for value in authorization_values {
        request.headers_mut().append(
            AUTHORIZATION,
            HeaderValue::try_from(value.as_str()).expect("test Authorization value"),
        );
    }
    connect_async(request).await.map(|(socket, _)| socket)
}

async fn send_hello(socket: &mut ClientSocket, hello: &RelayClientHello) {
    socket
        .send(Message::Binary(hello.encode().to_vec()))
        .await
        .expect("send relay hello");
}

async fn next_status(socket: &mut ClientSocket) -> RelayServerStatus {
    next_status_within(socket, Duration::from_secs(1)).await
}

/// Read the next status frame, answering the relay's waiting-lease pings the
/// way a real client's `next_binary_with_reauth` does.
async fn next_status_within(socket: &mut ClientSocket, timeout: Duration) -> RelayServerStatus {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let message = time::timeout_at(deadline, socket.next())
            .await
            .expect("status arrives before timeout")
            .expect("relay socket remains open")
            .expect("relay status frame is valid WebSocket");
        match message {
            Message::Binary(raw) => {
                return RelayServerStatus::decode(&raw).expect("strict relay status")
            }
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .await
                .expect("test client answers a lease ping"),
            Message::Pong(_) => {}
            other => panic!("expected binary relay status, got {other:?}"),
        }
    }
}

async fn connect_ready(server: &TestServer, hello: &RelayClientHello) -> ClientSocket {
    let mut socket = connect(server.address, "/v1/tunnel")
        .await
        .expect("connect relay");
    send_hello(&mut socket, hello).await;
    assert_eq!(next_status(&mut socket).await, RelayServerStatus::Ready);
    socket
}

#[tokio::test]
async fn successful_upgrades_emit_distinct_canonical_challenges() {
    let server =
        TestServer::start_with_authenticator(test_config(), Arc::new(ChallengeHeaderAuthenticator))
            .await;
    let mut challenges = Vec::new();
    for _ in 0..2 {
        let mut request = format!("ws://{}/v1/tunnel", server.address)
            .into_client_request()
            .expect("test WebSocket request");
        request.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer test-ticket"),
        );
        let (mut socket, response) = connect_async(request).await.expect("connect relay");
        let values: Vec<_> = response
            .headers()
            .get_all(RELAY_CHALLENGE_HEADER_NAME)
            .iter()
            .collect();
        assert_eq!(values.len(), 1);
        let encoded = values[0].to_str().expect("ASCII challenge header");
        let challenge =
            RelayServerChallengeV1::decode_header(encoded).expect("canonical challenge header");
        assert_eq!(challenge.key_id().as_str(), "test-relay-key");
        challenges.push(*challenge.nonce());
        let _ = socket.close(None).await;
    }
    assert_ne!(challenges[0], challenges[1]);
    server.stop().await;
}

#[tokio::test]
async fn owner_and_guest_relay_binary_in_both_directions() {
    let server = TestServer::start(test_config()).await;
    let owner_hello = make_valid_hello(RelayRole::Owner, 1);
    let guest_hello = make_valid_hello(RelayRole::Guest, 1);
    let mut owner = connect_ready(&server, &owner_hello).await;
    let mut guest = connect_ready(&server, &guest_hello).await;

    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    owner
        .send(Message::Binary(vec![1, 2, 3, 4]))
        .await
        .expect("owner sends opaque bytes");
    assert_eq!(
        guest
            .next()
            .await
            .expect("guest frame")
            .expect("guest read"),
        Message::Binary(vec![1, 2, 3, 4])
    );
    guest
        .send(Message::Binary(vec![9, 8, 7]))
        .await
        .expect("guest sends opaque bytes");
    assert_eq!(
        owner
            .next()
            .await
            .expect("owner frame")
            .expect("owner read"),
        Message::Binary(vec![9, 8, 7])
    );

    let _ = owner.close(None).await;
    let _ = guest.close(None).await;
    server.stop().await;
}

#[tokio::test]
async fn two_owners_on_the_same_route_never_pair() {
    let mut config = test_config();
    config.waiting_timeout = Duration::from_millis(100);
    let server = TestServer::start(config).await;
    let hello = make_valid_hello(RelayRole::Owner, 2);
    let mut first = connect_ready(&server, &hello).await;
    let mut second = connect_ready(&server, &hello).await;

    assert_eq!(
        next_status(&mut first).await,
        RelayServerStatus::Rejected(RelayRejectCode::PairingTimeout)
    );
    assert_eq!(
        next_status(&mut second).await,
        RelayServerStatus::Rejected(RelayRejectCode::PairingTimeout)
    );

    server.stop().await;
}

#[tokio::test]
async fn expired_locator_is_rejected_before_ready() {
    let server = TestServer::start(test_config()).await;
    let now = unix_now();
    let hello = make_hello(
        RelayRole::Guest,
        3,
        now.saturating_sub(120),
        now.saturating_sub(60),
        now.saturating_sub(90),
    );
    let mut socket = connect(server.address, "/v1/tunnel")
        .await
        .expect("connect relay");
    send_hello(&mut socket, &hello).await;
    assert_eq!(
        next_status(&mut socket).await,
        RelayServerStatus::Rejected(RelayRejectCode::LocatorExpired)
    );
    server.stop().await;
}

#[tokio::test]
async fn locator_expiring_too_far_in_the_future_is_rejected() {
    let server = TestServer::start(test_config()).await;
    let mut socket = connect(server.address, "/v1/tunnel")
        .await
        .expect("connect relay");
    socket
        .send(Message::Binary(
            make_too_far_future_hello(RelayRole::Guest, 4)
                .encode()
                .to_vec(),
        ))
        .await
        .expect("send future hello");
    assert_eq!(
        next_status(&mut socket).await,
        RelayServerStatus::Rejected(RelayRejectCode::ExpiryTooFarFuture)
    );
    server.stop().await;
}

#[tokio::test]
async fn authentication_expiry_ends_a_waiting_connection() {
    let mut config = test_config();
    config.waiting_timeout = Duration::from_secs(10);
    config.idle_timeout = Duration::from_secs(10);
    let server =
        TestServer::start_with_authenticator(config, Arc::new(ExpiringTestAuthenticator)).await;
    let mut owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 6)).await;

    assert_eq!(
        next_status_within(&mut owner, Duration::from_secs(4)).await,
        RelayServerStatus::Rejected(RelayRejectCode::AuthenticationFailed)
    );

    server.stop().await;
}

#[tokio::test]
async fn authentication_expiry_ends_a_paired_forwarder() {
    let server =
        TestServer::start_with_authenticator(test_config(), Arc::new(ExpiringTestAuthenticator))
            .await;
    let mut owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 7)).await;
    let mut guest = connect_ready(&server, &make_valid_hello(RelayRole::Guest, 7)).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    let message = time::timeout(Duration::from_secs(4), owner.next())
        .await
        .expect("authorization expiry closes the paired socket")
        .expect("server sends a close frame")
        .expect("close frame is valid");
    assert!(matches!(message, Message::Close(_)));

    server.stop().await;
}

#[tokio::test]
async fn wrong_websocket_path_is_rejected_during_upgrade() {
    let server = TestServer::start(test_config()).await;
    let error = connect(server.address, "/wrong")
        .await
        .expect_err("wrong relay path must fail");
    match error {
        WebSocketError::Http(response) => {
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
        other => panic!("expected HTTP upgrade rejection, got {other:?}"),
    }
    server.stop().await;
}

#[tokio::test]
async fn malformed_or_missing_bearer_headers_are_rejected_during_upgrade() {
    let server = TestServer::start(test_config()).await;
    let cases = [
        Vec::new(),
        vec!["Bearer first".to_owned(), "Bearer second".to_owned()],
        vec!["bearer abc".to_owned()],
        vec!["Bearer  abc".to_owned()],
        vec!["Bearer".to_owned()],
        vec!["Bearer ".to_owned()],
        vec!["Bearer =".to_owned()],
        vec!["Bearer abc=def".to_owned()],
        vec!["Bearer\tabc".to_owned()],
        vec![format!("Bearer {}", "a".repeat(MAX_RELAY_BEARER_BYTES + 1))],
    ];

    for authorization_values in cases {
        let error =
            connect_with_authorization_values(server.address, "/v1/tunnel", &authorization_values)
                .await
                .expect_err("invalid Authorization must fail the upgrade");
        assert!(
            matches!(error, WebSocketError::Http(ref response) if response.status() == StatusCode::UNAUTHORIZED),
            "unexpected upgrade error: {error:?}"
        );
    }

    server.stop().await;
}

#[tokio::test]
async fn maximum_auth_bridge_sized_bearer_is_accepted_during_upgrade() {
    assert_eq!(
        MAX_RELAY_BEARER_BYTES,
        op_auth_bridge::MAX_COLLAB_TICKET_BYTES
    );
    let server =
        TestServer::start_with_authenticator(test_config(), Arc::new(MaximumBearerAuthenticator))
            .await;
    let authorization = format!("Bearer {}", "a".repeat(MAX_RELAY_BEARER_BYTES));
    let mut socket =
        connect_with_authorization_values(server.address, "/v1/tunnel", &[authorization])
            .await
            .expect("maximum legal ticket-sized Authorization upgrades");
    send_hello(&mut socket, &make_valid_hello(RelayRole::Owner, 0xf1)).await;
    assert_eq!(next_status(&mut socket).await, RelayServerStatus::Ready);
    let _ = socket.close(None).await;
    server.stop().await;
}

#[tokio::test]
async fn custom_authenticator_expiry_is_clamped_to_the_signed_locator() {
    let mut config = test_config();
    config.waiting_timeout = Duration::from_secs(10);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(10);
    let server =
        TestServer::start_with_authenticator(config, Arc::new(OverlongCustomAuthenticator)).await;
    let now = unix_now();
    let hello = make_hello(RelayRole::Owner, 0xf2, now.saturating_sub(1), now + 2, now);
    let owner = connect_ready(&server, &hello).await;
    reauth_tests::close_arrives_under_ping_flood(owner).await;
    server.stop().await;
}

#[tokio::test]
async fn default_server_entrypoint_fails_closed_without_an_authenticator() {
    assert!(matches!(
        crate::run_until(test_config(), async {}).await,
        Err(RelayServerError::AuthenticationRequired)
    ));
}

#[test]
fn authentication_concurrency_must_be_non_zero() {
    let mut config = test_config();
    config.max_auth_in_flight = 0;
    assert!(matches!(
        config.validate(),
        Err(crate::ConfigError::Zero("max_auth_in_flight"))
    ));
}

#[test]
fn reauthentication_concurrency_must_be_non_zero() {
    let mut config = test_config();
    config.max_reauth_in_flight = 0;
    assert!(matches!(
        config.validate(),
        Err(crate::ConfigError::Zero("max_reauth_in_flight"))
    ));
}

#[test]
fn source_budget_must_not_exceed_the_global_pending_ceiling() {
    let mut config = test_config();
    config.max_pending = 4;
    config.max_pending_per_source = 5;
    assert!(matches!(
        config.validate(),
        Err(crate::ConfigError::SourceBudgetExceedsGlobal)
    ));
}

#[tokio::test]
async fn one_source_cannot_hold_more_than_its_share_of_pending_slots() {
    let mut config = test_config();
    config.waiting_timeout = Duration::from_secs(5);
    config.max_pending_per_source = 1;
    let server = TestServer::start(config).await;

    // An un-paired connection holds its admission slot for the whole wait, so
    // the second connection from the same source must be refused outright.
    let waiting = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 41)).await;
    assert!(connect(server.address, "/v1/tunnel").await.is_err());

    // Releasing the first connection returns the slot to that source.
    drop(waiting);
    let mut admitted = None;
    for _ in 0..50 {
        if let Ok(socket) = connect(server.address, "/v1/tunnel").await {
            admitted = Some(socket);
            break;
        }
        time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        admitted.is_some(),
        "a released source slot admits the next connection"
    );
    drop(admitted);
    server.stop().await;
}

#[tokio::test]
async fn pairing_releases_the_source_budget() {
    let mut config = test_config();
    config.waiting_timeout = Duration::from_secs(5);
    config.max_pending_per_source = 2;
    let server = TestServer::start(config).await;

    let owner_hello = make_valid_hello(RelayRole::Owner, 42);
    let guest_hello = make_valid_hello(RelayRole::Guest, 42);
    let mut owner = connect_ready(&server, &owner_hello).await;
    let mut guest = connect_ready(&server, &guest_hello).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    // Both peers are paired, so neither still charges the pre-pairing budget
    // and two fresh connections from the same source fit again.
    let mut admitted = 0;
    for _ in 0..50 {
        if connect(server.address, "/v1/tunnel").await.is_ok() {
            admitted += 1;
            if admitted == 2 {
                break;
            }
        }
        time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(admitted, 2, "paired peers release their source slots");
    server.stop().await;
}

#[test]
fn bearer_token_requires_data_before_optional_padding() {
    for invalid in [
        b"=".as_slice(),
        b"==".as_slice(),
        b"abc=def".as_slice(),
        b"abc def".as_slice(),
    ] {
        assert!(!is_rfc6750_b64token(invalid));
    }
    for valid in [
        b"abc".as_slice(),
        b"abc=".as_slice(),
        b"a.b_c-1~+/==".as_slice(),
    ] {
        assert!(is_rfc6750_b64token(valid));
    }
}

#[tokio::test]
async fn oversized_binary_is_never_forwarded() {
    let server = TestServer::start(test_config()).await;
    let owner_hello = make_valid_hello(RelayRole::Owner, 5);
    let guest_hello = make_valid_hello(RelayRole::Guest, 5);
    let mut owner = connect_ready(&server, &owner_hello).await;
    let mut guest = connect_ready(&server, &guest_hello).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    owner
        .send(Message::Binary(vec![0x7f; 64 * 1024 + 1]))
        .await
        .expect("client can write oversized test frame");
    let guest_result = time::timeout(Duration::from_secs(1), guest.next())
        .await
        .expect("relay closes peer after oversized frame");
    assert!(
        !matches!(guest_result, Some(Ok(Message::Binary(payload))) if payload.len() > 64 * 1024),
        "oversized payload must never reach the peer"
    );

    server.stop().await;
}

fn protocol_reject(error: RelayProtocolError) -> RelayRejectCode {
    match error {
        RelayProtocolError::NotYetValid => RelayRejectCode::LocatorNotYetValid,
        RelayProtocolError::Expired => RelayRejectCode::LocatorExpired,
        RelayProtocolError::ExpiryTooFarFuture | RelayProtocolError::ValidityWindowTooLong => {
            RelayRejectCode::ExpiryTooFarFuture
        }
        RelayProtocolError::UnsupportedVersion { .. } => RelayRejectCode::UnsupportedVersion,
        _ => RelayRejectCode::AuthenticationFailed,
    }
}

mod lease_tests;
mod reauth_tests;
mod reject_delivery_tests;
