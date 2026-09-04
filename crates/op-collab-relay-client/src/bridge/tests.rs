use std::net::SocketAddr;
use std::num::NonZeroU64;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant as StdInstant};

use futures_util::{SinkExt, StreamExt};
use op_collab_relay_protocol::{
    CallerDeviceDhPublic, ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic,
    RelayAuthExtensionV1, RelayChallengeKeyId, RelayChallengeProofV2, RelayClientHello,
    RelayHelloAuthMode, RelayLocatorVerifier, RelayReauthChallengeV1, RelayReauthResponseV1,
    RelayRegion, RelayRole, RelayServerChallengeV1, RelayServerStatus, RouteCapability, RouteId,
    UnsignedRelayLocatorV1, VerifiedRelayRoute, RELAY_CHALLENGE_HEADER_NAME,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{header::AUTHORIZATION, HeaderValue};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, accept_hdr_async, WebSocketStream};

use super::{RelayGuestBridge, RelayHandshake, RelayOwnerBridge};
use crate::auth::{
    ChallengeBoundRelayAuthenticator, PinnedRelayX25519Keys, RelayAuthError, RelayAuthenticator,
    RelayClientX25519Agreement, RelayCredential, RelayCredentialProvider,
    RelayServerX25519PublicKey,
};
use crate::endpoint::RelayEndpoint;
use crate::error::{RelayBridgePhase, RelayBridgeStatus, RelayClientError, RelayFailureKind};
use crate::limits::{RelayLimits, MAX_RELAY_BINARY_BYTES, MAX_RELAY_CONNECTION_BYTES};
use zeroize::Zeroizing;

const TEST_SHARED_SECRET: [u8; 32] = [0x77; 32];
const TEST_RELAY_PUBLIC: [u8; 32] = [0x78; 32];

struct FixedClientAgreement {
    caller_public: CallerDeviceDhPublic,
}

impl RelayClientX25519Agreement for FixedClientAgreement {
    fn caller_public_key(&self) -> Result<CallerDeviceDhPublic, RelayAuthError> {
        Ok(self.caller_public)
    }

    fn agree(&self, relay_public_key: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>, RelayAuthError> {
        if relay_public_key != &TEST_RELAY_PUBLIC {
            return Err(RelayAuthError::KeyAgreement);
        }
        Ok(Zeroizing::new(TEST_SHARED_SECRET))
    }
}

struct AcceptAllSignatures;

impl RelayLocatorVerifier for AcceptAllSignatures {
    fn verify(
        &self,
        _key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        true
    }
}

fn handshake(caller_byte: u8) -> RelayHandshake {
    let claims = UnsignedRelayLocatorV1::new(
        RelayRegion::Cn,
        RouteId::new([1; 16]).unwrap(),
        NonZeroU64::new(7).unwrap(),
        OwnerNoiseStatic::new([2; 32]).unwrap(),
        ExpectedDiscoveryId::new("session-test").unwrap(),
        100,
        200,
        LocatorKeyId::new("test-key").unwrap(),
    )
    .unwrap();
    let locator = claims
        .attach_signature(LocatorSignature::new([3; 64]).unwrap())
        .verify(&AcceptAllSignatures, 150)
        .unwrap();
    let route = VerifiedRelayRoute::new(locator, RouteCapability::new([4; 32]).unwrap());
    let auth = RelayAuthExtensionV1::without_possession_proof(
        CallerDeviceDhPublic::new([caller_byte; 32]).unwrap(),
    );
    RelayHandshake::new(route, auth)
}

fn test_limits() -> RelayLimits {
    RelayLimits {
        connect: Duration::from_secs(1),
        hello: Duration::from_secs(1),
        pair: Duration::from_secs(1),
        owner_pair: Duration::from_secs(1),
        keepalive: Duration::from_millis(250),
        idle: Duration::from_secs(2),
        lifetime: Duration::from_secs(5),
        retry: Duration::from_millis(10),
        // Stop is a convergence wait, not a latency assertion: no test asserts
        // `RelayStopError::Timeout`, so this ceiling only decides how loaded a
        // machine may be before an orderly shutdown gets misreported as a
        // timeout. Measured 2026-08-28: at 1s, three suite instances racing a
        // concurrent cargo build tripped it in whole-round bursts.
        stop: Duration::from_secs(10),
        max_binary_bytes: MAX_RELAY_BINARY_BYTES,
        max_connection_bytes: MAX_RELAY_CONNECTION_BYTES,
    }
}

fn endpoint(address: SocketAddr) -> RelayEndpoint {
    RelayEndpoint::parse(&format!("ws://{address}/v1/tunnel")).unwrap()
}

#[tokio::test]
async fn anonymous_development_bridges_reject_remote_endpoints_before_connecting() {
    let remote = || RelayEndpoint::parse("wss://relay.example/v1/tunnel").unwrap();
    let guest = RelayGuestBridge::start_unauthenticated_for_development(remote(), handshake(7))
        .await
        .unwrap_err();
    assert!(matches!(
        guest,
        RelayClientError::DevelopmentEndpointNotLoopback
    ));

    let owner = RelayOwnerBridge::start_unauthenticated_for_development(
        remote(),
        handshake(8),
        "127.0.0.1:41234".parse().unwrap(),
        1,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        owner,
        RelayClientError::DevelopmentEndpointNotLoopback
    ));
}

async fn receive_hello(socket: &mut WebSocketStream<TcpStream>, role: RelayRole) {
    let hello = receive_decoded_hello(socket).await;
    assert_eq!(hello.role(), role);
}

async fn receive_decoded_hello(socket: &mut WebSocketStream<TcpStream>) -> RelayClientHello {
    let message = socket.next().await.unwrap().unwrap();
    let Message::Binary(bytes) = message else {
        panic!("client hello must be binary");
    };
    RelayClientHello::decode(&bytes).unwrap()
}

async fn mark_ready_and_paired(socket: &mut WebSocketStream<TcpStream>) {
    socket
        .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
        .await
        .unwrap();
    socket
        .send(Message::Binary(RelayServerStatus::Paired.encode().to_vec()))
        .await
        .unwrap();
}

#[allow(clippy::result_large_err)]
fn assert_authorization(request: &Request, response: Response) -> Result<Response, ErrorResponse> {
    assert_eq!(
        request
            .headers()
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
        Some("Bearer admission-secret")
    );
    Ok(response)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guest_bridges_bytes_both_ways_and_preserves_authorization() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_hdr_async(stream, assert_authorization)
            .await
            .unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        mark_ready_and_paired(&mut socket).await;

        let message = socket.next().await.unwrap().unwrap();
        assert_eq!(message, Message::Binary(b"guest-to-relay".to_vec()));
        socket
            .send(Message::Binary(b"relay-to-guest".to_vec()))
            .await
            .unwrap();
        socket.close(None).await.unwrap();
    });

    let credentials: Arc<dyn RelayCredentialProvider> =
        Arc::new(|| RelayCredential::bearer("admission-secret"));
    let bridge = RelayGuestBridge::start_ticket_binding_only(
        endpoint(relay_addr),
        handshake(5),
        credentials,
    )
    .await
    .unwrap();
    let mut local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    local.write_all(b"guest-to-relay").await.unwrap();
    let mut inbound = [0_u8; 14];
    local.read_exact(&mut inbound).await.unwrap();
    assert_eq!(&inbound, b"relay-to-guest");

    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
    bridge.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::result_large_err)]
async fn strict_guest_reauthenticates_without_forwarding_text_or_losing_binary() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let relay_server = tokio::spawn(async move {
        let key_id = RelayChallengeKeyId::new("relay-pop-key").unwrap();
        let initial_challenge = RelayServerChallengeV1::new(key_id.clone(), [1; 32]).unwrap();
        let online_challenge = RelayServerChallengeV1::new(key_id, [2; 32]).unwrap();
        let initial_header = initial_challenge.encode_header();
        let (stream, _) = relay_listener.accept().await.unwrap();
        let callback = move |request: &Request, mut response: Response| {
            assert_eq!(
                request
                    .headers()
                    .get(AUTHORIZATION)
                    .and_then(|value| value.to_str().ok()),
                Some("Bearer attempt-1")
            );
            response.headers_mut().insert(
                RELAY_CHALLENGE_HEADER_NAME,
                HeaderValue::from_str(&initial_header).unwrap(),
            );
            Ok(response)
        };
        let mut socket = accept_hdr_async(stream, callback).await.unwrap();
        let initial_hello = receive_decoded_hello(&mut socket).await;
        RelayChallengeProofV2::decode(
            initial_hello
                .auth_extension()
                .possession_proof()
                .expect("initial strict proof"),
        )
        .unwrap()
        .verify(
            &TEST_SHARED_SECRET,
            &initial_challenge,
            b"attempt-1",
            &initial_hello,
        )
        .unwrap();
        mark_ready_and_paired(&mut socket).await;
        socket
            .send(Message::Text(
                RelayReauthChallengeV1::new(online_challenge.clone()).encode_text(),
            ))
            .await
            .unwrap();

        let mut saw_binary = false;
        let mut saw_response = false;
        while !saw_binary || !saw_response {
            match socket.next().await.unwrap().unwrap() {
                Message::Binary(bytes) => {
                    assert_eq!(bytes, b"during-reauth");
                    saw_binary = true;
                }
                Message::Text(text) => {
                    let response = RelayReauthResponseV1::decode_text(&text).unwrap();
                    assert_eq!(response.challenge(), &online_challenge);
                    assert_eq!(response.bearer(), b"attempt-2");
                    RelayChallengeProofV2::decode(
                        response
                            .hello()
                            .auth_extension()
                            .possession_proof()
                            .expect("online strict proof"),
                    )
                    .unwrap()
                    .verify(
                        &TEST_SHARED_SECRET,
                        &online_challenge,
                        b"attempt-2",
                        response.hello(),
                    )
                    .unwrap();
                    saw_response = true;
                }
                other => panic!("unexpected client frame during reauth: {other:?}"),
            }
        }
        socket
            .send(Message::Binary(b"peer-after-reauth".to_vec()))
            .await
            .unwrap();
    });

    let attempts = Arc::new(AtomicUsize::new(0));
    let credentials: Arc<dyn RelayCredentialProvider> = {
        let attempts = Arc::clone(&attempts);
        Arc::new(move || {
            let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
            RelayCredential::bearer(format!("attempt-{attempt}"))
        })
    };
    let relay_keys = Arc::new(
        PinnedRelayX25519Keys::new([RelayServerX25519PublicKey::new(
            RelayChallengeKeyId::new("relay-pop-key").unwrap(),
            TEST_RELAY_PUBLIC,
        )
        .unwrap()])
        .unwrap(),
    );
    let authenticator: Arc<dyn RelayAuthenticator> =
        Arc::new(ChallengeBoundRelayAuthenticator::new(
            credentials,
            Arc::new(FixedClientAgreement {
                caller_public: CallerDeviceDhPublic::new([6; 32]).unwrap(),
            }),
            relay_keys,
        ));
    let bridge = RelayGuestBridge::start(endpoint(relay_addr), handshake(6), authenticator)
        .await
        .unwrap();
    let mut local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    let mut statuses = bridge.subscribe();
    let active = wait_for_phase(&mut statuses, RelayBridgePhase::Active).await;
    assert_eq!(active.active_tunnels, 1);
    local.write_all(b"during-reauth").await.unwrap();
    let mut received = [0_u8; 17];
    local.read_exact(&mut received).await.unwrap();
    assert_eq!(&received, b"peer-after-reauth");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);

    relay_server.await.unwrap();
    bridge.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reduced_assurance_client_never_answers_strict_reauth_control() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_hdr_async(stream, assert_authorization)
            .await
            .unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        mark_ready_and_paired(&mut socket).await;
        let challenge =
            RelayServerChallengeV1::new(RelayChallengeKeyId::new("strict-key").unwrap(), [9; 32])
                .unwrap();
        socket
            .send(Message::Text(
                RelayReauthChallengeV1::new(challenge).encode_text(),
            ))
            .await
            .unwrap();
        assert!(
            !matches!(
                tokio::time::timeout(Duration::from_secs(1), socket.next()).await,
                Ok(Some(Ok(Message::Text(_))))
            ),
            "reduced client must not answer a strict control challenge"
        );
    });

    let credentials: Arc<dyn RelayCredentialProvider> =
        Arc::new(|| RelayCredential::bearer("admission-secret"));
    let bridge = RelayGuestBridge::start_ticket_binding_only(
        endpoint(relay_addr),
        handshake(5),
        credentials,
    )
    .await
    .unwrap();
    let _local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    let mut statuses = bridge.subscribe();
    let failed = wait_for_phase(&mut statuses, RelayBridgePhase::Failed).await;
    assert_eq!(failed.last_error, Some(RelayFailureKind::TextFrame));

    server.await.unwrap();
    bridge.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::result_large_err)]
async fn owner_refresh_attempts_use_fresh_bearers_challenges_and_proofs() {
    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let owner_driver = tokio::spawn(async move {
        for value in [b"first".as_slice(), b"second".as_slice()] {
            let (mut stream, _) = owner_listener.accept().await.unwrap();
            stream.write_all(value).await.unwrap();
        }
    });

    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let relay_server = tokio::spawn(async move {
        let key_id = RelayChallengeKeyId::new("relay-pop-key").unwrap();
        let mut previous_proof = None;
        for attempt in 1_u8..=2 {
            let expected_bearer = format!("attempt-{attempt}");
            let challenge = RelayServerChallengeV1::new(key_id.clone(), [attempt; 32]).unwrap();
            let challenge_header = challenge.encode_header();
            let (stream, _) = relay_listener.accept().await.unwrap();
            let callback = move |request: &Request, mut response: Response| {
                let expected = format!("Bearer {expected_bearer}");
                assert_eq!(
                    request
                        .headers()
                        .get(AUTHORIZATION)
                        .and_then(|value| value.to_str().ok()),
                    Some(expected.as_str())
                );
                response.headers_mut().insert(
                    RELAY_CHALLENGE_HEADER_NAME,
                    HeaderValue::from_str(&challenge_header).unwrap(),
                );
                Ok(response)
            };
            let mut socket = accept_hdr_async(stream, callback).await.unwrap();
            let hello = receive_decoded_hello(&mut socket).await;
            assert_eq!(hello.role(), RelayRole::Owner);
            assert_eq!(
                hello.auth_mode(),
                RelayHelloAuthMode::ChallengeBoundX25519V2
            );
            let proof = RelayChallengeProofV2::decode(
                hello
                    .auth_extension()
                    .possession_proof()
                    .expect("strict client proof"),
            )
            .unwrap();
            proof
                .verify(
                    &TEST_SHARED_SECRET,
                    &challenge,
                    format!("attempt-{attempt}").as_bytes(),
                    &hello,
                )
                .unwrap();
            if let Some(previous) = &previous_proof {
                assert_ne!(previous, proof.as_bytes());
            }
            previous_proof = Some(*proof.as_bytes());
            mark_ready_and_paired(&mut socket).await;
            let _ = socket.next().await;
            socket.close(None).await.unwrap();
        }
    });

    let attempts = Arc::new(AtomicUsize::new(0));
    let credentials: Arc<dyn RelayCredentialProvider> = {
        let attempts = Arc::clone(&attempts);
        Arc::new(move || {
            let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
            RelayCredential::bearer(format!("attempt-{attempt}"))
        })
    };
    let key_id = RelayChallengeKeyId::new("relay-pop-key").unwrap();
    let relay_keys = Arc::new(
        PinnedRelayX25519Keys::new([
            RelayServerX25519PublicKey::new(key_id, TEST_RELAY_PUBLIC).unwrap()
        ])
        .unwrap(),
    );
    let key_agreement: Arc<dyn RelayClientX25519Agreement> = Arc::new(FixedClientAgreement {
        caller_public: CallerDeviceDhPublic::new([6; 32]).unwrap(),
    });
    let authenticator: Arc<dyn RelayAuthenticator> = Arc::new(
        ChallengeBoundRelayAuthenticator::new(credentials, key_agreement, relay_keys),
    );
    let bridge = RelayOwnerBridge::start(
        endpoint(relay_addr),
        handshake(6),
        owner_addr,
        1,
        authenticator,
    )
    .await
    .unwrap();

    tokio::time::timeout(Duration::from_secs(8), relay_server)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(8), owner_driver)
        .await
        .unwrap()
        .unwrap();
    assert!(attempts.load(Ordering::SeqCst) >= 2);
    bridge.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_replenishes_lane_after_each_tunnel_and_bridges_both_ways() {
    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let owner_driver = tokio::spawn(async move {
        for (outbound, inbound) in [
            (&b"owner-one"[..], &b"relay-one"[..]),
            (&b"owner-two"[..], &b"relay-two"[..]),
        ] {
            let (mut stream, _) = owner_listener.accept().await.unwrap();
            stream.write_all(outbound).await.unwrap();
            let mut received = vec![0_u8; inbound.len()];
            stream.read_exact(&mut received).await.unwrap();
            assert_eq!(received, inbound);
        }
    });

    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let relay_server = tokio::spawn(async move {
        for (outbound, inbound) in [
            (&b"owner-one"[..], &b"relay-one"[..]),
            (&b"owner-two"[..], &b"relay-two"[..]),
        ] {
            let (stream, _) = relay_listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            receive_hello(&mut socket, RelayRole::Owner).await;
            mark_ready_and_paired(&mut socket).await;
            let message = socket.next().await.unwrap().unwrap();
            assert_eq!(message, Message::Binary(outbound.to_vec()));
            socket
                .send(Message::Binary(inbound.to_vec()))
                .await
                .unwrap();
            socket.close(None).await.unwrap();
        }
    });

    let bridge = RelayOwnerBridge::start_test(
        endpoint(relay_addr),
        handshake(6),
        owner_addr,
        1,
        test_limits(),
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(4), relay_server)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(4), owner_driver)
        .await
        .unwrap()
        .unwrap();
    bridge.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_readiness_waits_for_relay_acceptance_not_pairing() {
    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Owner).await;
        socket
            .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
            .await
            .unwrap();
        while socket.next().await.is_some() {}
    });

    let bridge = RelayOwnerBridge::start_test(
        endpoint(relay_addr),
        handshake(6),
        owner_addr,
        1,
        test_limits(),
    )
    .await
    .unwrap();
    bridge
        // Generous on purpose: readiness here is a settled state the relay
        // WILL reach, so a loaded runner should make the wait longer, never
        // red. The timeout contract has its own test below with a budget it
        // expects to blow.
        .wait_until_ready(Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(bridge.status().phase, RelayBridgePhase::Waiting);
    assert_eq!(bridge.status().waiting_lanes, 1);

    bridge.stop().await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_readiness_times_out_when_relay_cannot_be_reached() {
    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let unused_relay = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = unused_relay.local_addr().unwrap();
    drop(unused_relay);

    let bridge = RelayOwnerBridge::start_test(
        endpoint(relay_addr),
        handshake(6),
        owner_addr,
        1,
        test_limits(),
    )
    .await
    .unwrap();
    let error = bridge
        .wait_until_ready(Duration::from_millis(80))
        .await
        .unwrap_err();
    assert!(matches!(error, RelayClientError::ReadyTimeout));
    bridge.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_cancels_a_guest_waiting_for_pairing_and_joins() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        socket
            .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
            .await
            .unwrap();
        let _ = ready_tx.send(());
        while socket.next().await.is_some() {}
    });

    let bridge = RelayGuestBridge::start_test(endpoint(relay_addr), handshake(7), test_limits())
        .await
        .unwrap();
    let _local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    ready_rx.await.unwrap();
    let started = StdInstant::now();
    bridge.stop().await.unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn text_application_frame_becomes_a_redacted_error_status() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        socket
            .send(Message::Text("not-binary".to_owned()))
            .await
            .unwrap();
    });

    let bridge = RelayGuestBridge::start_test(endpoint(relay_addr), handshake(8), test_limits())
        .await
        .unwrap();
    let _local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    let mut statuses = bridge.subscribe();
    let failed = wait_for_phase(&mut statuses, RelayBridgePhase::Failed).await;
    assert_eq!(failed.last_error, Some(RelayFailureKind::TextFrame));
    assert!(!format!("{failed:?}").contains("not-binary"));

    server.await.unwrap();
    bridge.stop().await.unwrap();
}

#[test]
fn handshake_debug_redacts_locator_capability_and_device_proof() {
    let debug = format!("{:?}", handshake(9));
    assert_eq!(debug, "RelayHandshake([REDACTED])");
    assert!(!debug.contains("session-test"));
}

async fn wait_for_phase(
    statuses: &mut watch::Receiver<RelayBridgeStatus>,
    phase: RelayBridgePhase,
) -> RelayBridgeStatus {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let status = *statuses.borrow_and_update();
            if status.phase == phase {
                return status;
            }
            statuses.changed().await.unwrap();
        }
    })
    .await
    .unwrap()
}

#[path = "owner_pool_tests.rs"]
mod owner_pool;

#[path = "guest_recovery_tests.rs"]
mod guest_recovery;
