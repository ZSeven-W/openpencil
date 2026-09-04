use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use op_collab_relay_protocol::{
    RelayChallengeProofV2, RelayHelloAuthMode, RelayReauthChallengeV1, RelayReauthResponseV1,
    RELAY_CHALLENGE_PROOF_V2_BYTES,
};

use super::*;

const FRESH_BEARER: &[u8] = b"fresh-ticket";

struct RenewingAuthenticator {
    initial_ttl: u64,
    renewed_ttl: u64,
    initial_authentications: AtomicUsize,
    renewed_authentications: AtomicUsize,
}

impl RenewingAuthenticator {
    fn new(initial_ttl: u64, renewed_ttl: u64) -> Self {
        Self {
            initial_ttl,
            renewed_ttl,
            initial_authentications: AtomicUsize::new(0),
            renewed_authentications: AtomicUsize::new(0),
        }
    }
}

impl RelayAuthenticator for RenewingAuthenticator {
    fn challenge_key_id(&self) -> Result<Option<RelayChallengeKeyId>, RelayRejectCode> {
        Ok(Some(
            RelayChallengeKeyId::new("online-reauth-test-key").unwrap(),
        ))
    }

    fn authenticate(
        &self,
        hello: &RelayClientHello,
        credential: Option<&RelayBearerCredential>,
        challenge: Option<crate::RelayUpgradeChallenge>,
    ) -> Result<AuthenticatedRoute, RelayRejectCode> {
        if hello.auth_mode() != RelayHelloAuthMode::ChallengeBoundX25519V2
            || hello.auth_extension().possession_proof().is_none()
            || challenge.is_none()
        {
            return Err(RelayRejectCode::AuthenticationFailed);
        }
        let bearer = credential
            .ok_or(RelayRejectCode::AuthenticationRequired)?
            .as_bytes();
        let ttl = if bearer == TEST_BEARER {
            self.initial_authentications.fetch_add(1, Ordering::SeqCst);
            self.initial_ttl
        } else if bearer == FRESH_BEARER {
            self.renewed_authentications.fetch_add(1, Ordering::SeqCst);
            self.renewed_ttl
        } else {
            return Err(RelayRejectCode::AuthenticationFailed);
        };
        let now = unix_now();
        let verified = hello
            .verify_locator(&AcceptAllLocatorSignatures, now)
            .map_err(protocol_reject)?;
        Ok(AuthenticatedRoute::new(
            verified.route_map_key(),
            verified.role(),
            NonZeroU64::new(now + ttl).unwrap(),
        ))
    }
}

struct BlockingChallengeAuthenticator {
    calls: AtomicUsize,
    active: AtomicUsize,
    release: AtomicBool,
    block_from_call: usize,
    expiry_ttl: u64,
}

struct BlockingRenewalAuthenticator {
    active: AtomicUsize,
    release: AtomicBool,
}

struct ReleaseOnDrop<'a>(&'a AtomicBool);

impl Drop for ReleaseOnDrop<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl BlockingRenewalAuthenticator {
    fn new() -> Self {
        Self {
            active: AtomicUsize::new(0),
            release: AtomicBool::new(false),
        }
    }
}

impl RelayAuthenticator for BlockingRenewalAuthenticator {
    fn challenge_key_id(&self) -> Result<Option<RelayChallengeKeyId>, RelayRejectCode> {
        Ok(Some(
            RelayChallengeKeyId::new("blocking-renewal-key").unwrap(),
        ))
    }

    fn authenticate(
        &self,
        hello: &RelayClientHello,
        credential: Option<&RelayBearerCredential>,
        challenge: Option<crate::RelayUpgradeChallenge>,
    ) -> Result<AuthenticatedRoute, RelayRejectCode> {
        if hello.auth_mode() != RelayHelloAuthMode::ChallengeBoundX25519V2
            || hello.auth_extension().possession_proof().is_none()
            || challenge.is_none()
        {
            return Err(RelayRejectCode::AuthenticationFailed);
        }
        let bearer = credential
            .ok_or(RelayRejectCode::AuthenticationRequired)?
            .as_bytes();
        let ttl = match (bearer, hello.role()) {
            (TEST_BEARER, RelayRole::Owner) => 6,
            (TEST_BEARER, RelayRole::Guest) => 30,
            (FRESH_BEARER, RelayRole::Owner) => {
                self.active.fetch_add(1, Ordering::SeqCst);
                while !self.release.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(2));
                }
                self.active.fetch_sub(1, Ordering::SeqCst);
                30
            }
            (FRESH_BEARER, RelayRole::Guest) => 30,
            _ => return Err(RelayRejectCode::AuthenticationFailed),
        };
        let now = unix_now();
        let verified = hello
            .verify_locator(&AcceptAllLocatorSignatures, now)
            .map_err(protocol_reject)?;
        Ok(AuthenticatedRoute::new(
            verified.route_map_key(),
            verified.role(),
            NonZeroU64::new(now + ttl).unwrap(),
        ))
    }
}

impl BlockingChallengeAuthenticator {
    fn new(block_from_call: usize, expiry_ttl: u64) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            release: AtomicBool::new(false),
            block_from_call,
            expiry_ttl,
        }
    }
}

impl RelayAuthenticator for BlockingChallengeAuthenticator {
    fn challenge_key_id(&self) -> Result<Option<RelayChallengeKeyId>, RelayRejectCode> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call >= self.block_from_call {
            self.active.fetch_add(1, Ordering::SeqCst);
            while !self.release.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
            self.active.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(Some(RelayChallengeKeyId::new("blocking-test-key").unwrap()))
    }

    fn authenticate(
        &self,
        hello: &RelayClientHello,
        _credential: Option<&RelayBearerCredential>,
        challenge: Option<crate::RelayUpgradeChallenge>,
    ) -> Result<AuthenticatedRoute, RelayRejectCode> {
        if challenge.is_none() {
            return Err(RelayRejectCode::AuthenticationFailed);
        }
        let verified = hello
            .verify_locator(&AcceptAllLocatorSignatures, unix_now())
            .map_err(protocol_reject)?;
        Ok(AuthenticatedRoute::new(
            verified.route_map_key(),
            verified.role(),
            NonZeroU64::new(unix_now() + self.expiry_ttl).unwrap(),
        ))
    }
}

fn strict_hello(role: RelayRole, route_seed: u8, caller: u8) -> RelayClientHello {
    let v1 = make_valid_hello(role, route_seed);
    let verified = v1
        .verify_locator(&AcceptAllLocatorSignatures, unix_now())
        .unwrap();
    RelayClientHello::new_challenge_bound_v2(
        role,
        verified.route(),
        RelayAuthExtensionV1::new(
            CallerDeviceDhPublic::new([caller; 32]).unwrap(),
            Some(vec![2; RELAY_CHALLENGE_PROOF_V2_BYTES]),
        )
        .unwrap(),
    )
    .unwrap()
}

/// Read the next reauthentication challenge, answering waiting-lease pings on
/// the way the same as a real client's `next_binary_with_reauth` does.
async fn next_challenge(socket: &mut ClientSocket) -> RelayServerChallengeV1 {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let message = time::timeout_at(deadline, socket.next())
            .await
            .expect("reauthentication challenge arrives")
            .expect("socket remains open")
            .expect("challenge is a valid WebSocket message");
        match message {
            Message::Text(text) => {
                return RelayReauthChallengeV1::decode_text(&text)
                    .unwrap()
                    .into_challenge()
            }
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .await
                .expect("test client answers a lease ping"),
            Message::Pong(_) => {}
            other => panic!("expected reauthentication Text control, got {other:?}"),
        }
    }
}

async fn answer_challenge(
    socket: &mut ClientSocket,
    challenge: RelayServerChallengeV1,
    hello: RelayClientHello,
) -> String {
    let response = RelayReauthResponseV1::new(challenge, FRESH_BEARER, hello).unwrap();
    let text = response.encode_text().to_string();
    socket.send(Message::Text(text.clone())).await.unwrap();
    text
}

async fn connect_strict_ready(server: &TestServer, hello: &RelayClientHello) -> ClientSocket {
    connect_ready(server, hello).await
}

async fn next_data_or_answer(socket: &mut ClientSocket, hello: &RelayClientHello) -> Vec<u8> {
    loop {
        let message = time::timeout(Duration::from_secs(5), socket.next())
            .await
            .expect("relay frame arrives")
            .expect("socket remains open")
            .expect("valid WebSocket frame");
        match message {
            Message::Binary(bytes) => return bytes,
            Message::Text(text) => {
                let challenge = RelayReauthChallengeV1::decode_text(&text)
                    .unwrap()
                    .into_challenge();
                answer_challenge(socket, challenge, hello.clone()).await;
            }
            other => panic!("unexpected relay frame: {other:?}"),
        }
    }
}

async fn answer_reauth_control(
    socket: &mut ClientSocket,
    hello: &RelayClientHello,
    message: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
) {
    let message = message
        .expect("socket remains open during reauthentication")
        .expect("reauthentication control is a valid WebSocket message");
    match message {
        Message::Text(text) => {
            let challenge = RelayReauthChallengeV1::decode_text(&text)
                .expect("server sends a valid reauthentication challenge")
                .into_challenge();
            answer_challenge(socket, challenge, hello.clone()).await;
        }
        Message::Ping(payload) => {
            socket
                .send(Message::Pong(payload))
                .await
                .expect("reply to reauthentication ping");
        }
        Message::Pong(_) => {}
        other => panic!("unexpected frame while completing reauthentication: {other:?}"),
    }
}

async fn complete_paired_reauthentication(
    owner: &mut ClientSocket,
    guest: &mut ClientSocket,
    owner_hello: &RelayClientHello,
    guest_hello: &RelayClientHello,
    authenticator: &RenewingAuthenticator,
) {
    time::timeout(Duration::from_secs(5), async {
        while authenticator.renewed_authentications.load(Ordering::SeqCst) < 2 {
            tokio::select! {
                biased;
                message = owner.next() => {
                    answer_reauth_control(owner, owner_hello, message).await;
                }
                message = guest.next() => {
                    answer_reauth_control(guest, guest_hello, message).await;
                }
                _ = time::sleep(Duration::from_millis(1)) => {}
            }
        }
    })
    .await
    .expect("both peers complete online reauthentication");
}

async fn wait_for_atomic(value: &AtomicUsize, expected: usize) {
    time::timeout(Duration::from_secs(5), async {
        while value.load(Ordering::SeqCst) != expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("atomic state reaches expected value");
}

pub(super) async fn close_arrives_under_ping_flood(socket: ClientSocket) {
    let (mut sink, mut source) = socket.split();
    let flood = tokio::spawn(async move {
        loop {
            if sink.send(Message::Ping(Vec::new())).await.is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
    });
    time::timeout(Duration::from_secs(4), async {
        loop {
            match source.next().await {
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => {}
            }
        }
    })
    .await
    .expect("hard deadline closes despite saturated traffic");
    flood.abort();
    let _ = flood.await;
}

async fn exchange_bidirectional_burst(
    owner: &mut ClientSocket,
    guest: &mut ClientSocket,
    owner_hello: &RelayClientHello,
    guest_hello: &RelayClientHello,
) {
    const FRAMES: u8 = 12;
    for sequence in 0..FRAMES {
        owner
            .send(Message::Binary(vec![0x0a, sequence]))
            .await
            .unwrap();
        guest
            .send(Message::Binary(vec![0x0b, sequence]))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    for sequence in 0..FRAMES {
        assert_eq!(
            next_data_or_answer(guest, guest_hello).await,
            vec![0x0a, sequence]
        );
        assert_eq!(
            next_data_or_answer(owner, owner_hello).await,
            vec![0x0b, sequence]
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn paired_reauth_uses_fresh_challenge_and_preserves_racing_binary() {
    let mut config = test_config();
    config.handshake_timeout = Duration::from_millis(200);
    config.waiting_timeout = Duration::from_secs(5);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(10);
    let authenticator = Arc::new(RenewingAuthenticator::new(4, 30));
    let server = TestServer::start_with_authenticator(config, authenticator.clone()).await;
    let owner_hello = strict_hello(RelayRole::Owner, 21, 0x66);
    let guest_hello = strict_hello(RelayRole::Guest, 21, 0x66);
    let mut owner = connect_strict_ready(&server, &owner_hello).await;
    let mut guest = connect_strict_ready(&server, &guest_hello).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    let owner_challenge = next_challenge(&mut owner).await;
    owner
        .send(Message::Binary(b"binary-before-response".to_vec()))
        .await
        .unwrap();
    answer_challenge(&mut owner, owner_challenge, owner_hello.clone()).await;
    assert_eq!(
        next_data_or_answer(&mut guest, &guest_hello).await,
        b"binary-before-response"
    );

    guest
        .send(Message::Binary(b"binary-after-response".to_vec()))
        .await
        .unwrap();
    assert_eq!(
        next_data_or_answer(&mut owner, &owner_hello).await,
        b"binary-after-response"
    );
    complete_paired_reauthentication(
        &mut owner,
        &mut guest,
        &owner_hello,
        &guest_hello,
        authenticator.as_ref(),
    )
    .await;
    assert_eq!(
        authenticator.initial_authentications.load(Ordering::SeqCst),
        2
    );
    assert_eq!(
        authenticator.renewed_authentications.load(Ordering::SeqCst),
        2
    );

    let _ = owner.close(None).await;
    let _ = guest.close(None).await;
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn paired_reauth_pumps_saturated_bidirectional_traffic_while_authentication_blocks() {
    let mut config = test_config();
    config.handshake_timeout = Duration::from_secs(2);
    config.waiting_timeout = Duration::from_secs(8);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(12);
    config.relay_queue_capacity = 2;
    let authenticator = Arc::new(BlockingRenewalAuthenticator::new());
    let server = TestServer::start_with_authenticator(config, authenticator.clone()).await;
    let owner_hello = strict_hello(RelayRole::Owner, 30, 0x66);
    let guest_hello = strict_hello(RelayRole::Guest, 30, 0x66);
    let mut owner = connect_strict_ready(&server, &owner_hello).await;
    let mut guest = connect_strict_ready(&server, &guest_hello).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    let challenge = next_challenge(&mut owner).await;
    answer_challenge(&mut owner, challenge, owner_hello.clone()).await;
    wait_for_atomic(&authenticator.active, 1).await;
    let release = ReleaseOnDrop(&authenticator.release);
    let traffic = time::timeout(
        Duration::from_secs(1),
        exchange_bidirectional_burst(&mut owner, &mut guest, &owner_hello, &guest_hello),
    )
    .await;
    drop(release);
    wait_for_atomic(&authenticator.active, 0).await;
    traffic.expect("traffic continues while the authentication boundary is blocked");

    owner
        .send(Message::Binary(b"still-paired".to_vec()))
        .await
        .unwrap();
    assert_eq!(
        next_data_or_answer(&mut guest, &guest_hello).await,
        b"still-paired"
    );
    let _ = owner.close(None).await;
    let _ = guest.close(None).await;
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn waiting_owner_renews_past_initial_ticket_and_later_pairs() {
    let mut config = test_config();
    config.handshake_timeout = Duration::from_millis(200);
    config.waiting_timeout = Duration::from_secs(8);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(10);
    let server =
        TestServer::start_with_authenticator(config, Arc::new(RenewingAuthenticator::new(4, 30)))
            .await;
    let owner_hello = strict_hello(RelayRole::Owner, 22, 0x66);
    let guest_hello = strict_hello(RelayRole::Guest, 22, 0x66);
    let mut owner = connect_strict_ready(&server, &owner_hello).await;

    let challenge = next_challenge(&mut owner).await;
    answer_challenge(&mut owner, challenge, owner_hello.clone()).await;
    tokio::time::sleep(Duration::from_secs(3)).await;

    let mut guest = connect_strict_ready(&server, &guest_hello).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    let _ = owner.close(None).await;
    let _ = guest.close(None).await;
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stale_reauth_text_closes_without_peer_forwarding() {
    let mut config = test_config();
    config.handshake_timeout = Duration::from_millis(200);
    config.waiting_timeout = Duration::from_secs(5);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(10);
    let server =
        TestServer::start_with_authenticator(config, Arc::new(RenewingAuthenticator::new(4, 30)))
            .await;
    let owner_hello = strict_hello(RelayRole::Owner, 23, 0x66);
    let guest_hello = strict_hello(RelayRole::Guest, 23, 0x66);
    let mut owner = connect_strict_ready(&server, &owner_hello).await;
    let mut guest = connect_strict_ready(&server, &guest_hello).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    let challenge = next_challenge(&mut owner).await;
    let stale = RelayServerChallengeV1::new(challenge.key_id().clone(), [0xa5; 32]).unwrap();
    let stale_response = RelayReauthResponseV1::new(stale, FRESH_BEARER, owner_hello).unwrap();
    owner
        .send(Message::Text(stale_response.encode_text().to_string()))
        .await
        .unwrap();
    let closed = time::timeout(Duration::from_secs(2), owner.next())
        .await
        .expect("stale response closes connection");
    assert!(
        !matches!(closed, Some(Ok(Message::Binary(_)))),
        "control response must never become peer data"
    );
    if let Ok(Some(Ok(Message::Text(text)))) =
        time::timeout(Duration::from_millis(200), guest.next()).await
    {
        assert!(
            RelayReauthChallengeV1::decode_text(&text).is_ok(),
            "the only peer Text allowed is its own server challenge"
        );
        assert!(
            RelayReauthResponseV1::decode_text(&text).is_err(),
            "the stale owner response must never be forwarded to its peer"
        );
    }

    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn duplicate_successful_reauth_response_is_rejected_as_replay() {
    let mut config = test_config();
    config.handshake_timeout = Duration::from_millis(200);
    config.waiting_timeout = Duration::from_secs(5);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(10);
    let server =
        TestServer::start_with_authenticator(config, Arc::new(RenewingAuthenticator::new(4, 30)))
            .await;
    let owner_hello = strict_hello(RelayRole::Owner, 24, 0x66);
    let guest_hello = strict_hello(RelayRole::Guest, 24, 0x66);
    let mut owner = connect_strict_ready(&server, &owner_hello).await;
    let mut guest = connect_strict_ready(&server, &guest_hello).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    let challenge = next_challenge(&mut owner).await;
    let response = RelayReauthResponseV1::new(challenge, FRESH_BEARER, owner_hello).unwrap();
    let text = response.encode_text().to_string();
    owner.send(Message::Text(text.clone())).await.unwrap();
    owner.send(Message::Text(text)).await.unwrap();
    let result = time::timeout(Duration::from_secs(2), owner.next())
        .await
        .expect("duplicate response closes the connection");
    assert!(
        !matches!(result, Some(Ok(Message::Binary(_)))),
        "duplicate control response is never relayed as application data"
    );

    let _ = guest.close(None).await;
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn initial_challenge_timeout_keeps_its_auth_concurrency_permit() {
    let mut config = test_config();
    config.handshake_timeout = Duration::from_millis(100);
    config.max_auth_in_flight = 1;
    let authenticator = Arc::new(BlockingChallengeAuthenticator::new(0, 30));
    let server = TestServer::start_with_authenticator(config, authenticator.clone()).await;

    let first = tokio::spawn(connect(server.address, "/v1/tunnel"));
    wait_for_atomic(&authenticator.active, 1).await;
    let release = ReleaseOnDrop(&authenticator.release);
    assert!(
        first.await.unwrap().is_err(),
        "first handshake times out while the blocking task retains its permit"
    );
    let _ = connect(server.address, "/v1/tunnel").await;
    assert_eq!(
        authenticator.calls.load(Ordering::SeqCst),
        1,
        "a detached timed-out challenge cannot admit another blocking task"
    );

    drop(release);
    wait_for_atomic(&authenticator.active, 0).await;
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn online_challenge_timeout_keeps_its_auth_concurrency_permit() {
    let mut config = test_config();
    config.handshake_timeout = Duration::from_millis(200);
    config.max_auth_in_flight = 1;
    // Renewals are budgeted separately from initial authentication, so that an
    // unauthenticated flood cannot starve live tunnels into a policy close.
    // The permit hygiene this test covers is therefore a property of the
    // renewal budget: saturate that one to make a second online challenge
    // contend.
    config.max_reauth_in_flight = 1;
    config.waiting_timeout = Duration::from_secs(5);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(10);
    let authenticator = Arc::new(BlockingChallengeAuthenticator::new(2, 4));
    let server = TestServer::start_with_authenticator(config, authenticator.clone()).await;
    let owner_hello = strict_hello(RelayRole::Owner, 27, 0x66);
    let guest_hello = strict_hello(RelayRole::Guest, 27, 0x66);
    let mut owner = connect_strict_ready(&server, &owner_hello).await;
    let mut guest = connect_strict_ready(&server, &guest_hello).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);

    wait_for_atomic(&authenticator.active, 1).await;
    time::timeout(
        Duration::from_millis(300),
        exchange_bidirectional_burst(&mut owner, &mut guest, &owner_hello, &guest_hello),
    )
    .await
    .expect("permit and challenge-key waits keep pumping paired traffic");
    assert_eq!(
        authenticator.calls.load(Ordering::SeqCst),
        3,
        "the competing online challenge times out before spawning another blocking task"
    );

    authenticator.release.store(true, Ordering::SeqCst);
    wait_for_atomic(&authenticator.active, 0).await;
    let _ = owner.close(None).await;
    let _ = guest.close(None).await;
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn waiting_auth_expiry_is_not_starved_by_saturated_ping_traffic() {
    let mut config = test_config();
    config.waiting_timeout = Duration::from_secs(10);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(10);
    let server =
        TestServer::start_with_authenticator(config, Arc::new(ExpiringTestAuthenticator)).await;
    let owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 28)).await;
    close_arrives_under_ping_flood(owner).await;
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn paired_configured_lifetime_is_not_starved_by_saturated_ping_traffic() {
    let mut config = test_config();
    config.waiting_timeout = Duration::from_secs(5);
    config.idle_timeout = Duration::from_secs(10);
    config.tunnel_lifetime = Duration::from_secs(2);
    let server = TestServer::start(config).await;
    let mut owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 29)).await;
    let mut guest = connect_ready(&server, &make_valid_hello(RelayRole::Guest, 29)).await;
    assert_eq!(next_status(&mut owner).await, RelayServerStatus::Paired);
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);
    close_arrives_under_ping_flood(owner).await;
    let _ = guest.close(None).await;
    server.stop().await;
}

#[test]
fn reauth_identity_rejects_wrong_role_caller_locator_and_route_capability() {
    use crate::connection_reauth::RelaySessionIdentity;

    let original = strict_hello(RelayRole::Guest, 25, 0x66);
    let verified = original
        .verify_locator(&AcceptAllLocatorSignatures, unix_now())
        .unwrap();
    let route = verified.route_map_key();
    let identity = RelaySessionIdentity::new(route, RelayRole::Guest, &original).unwrap();

    let wrong_role = strict_hello(RelayRole::Owner, 25, 0x66);
    let wrong_caller = strict_hello(RelayRole::Guest, 25, 0x67);
    let wrong_locator = strict_hello(RelayRole::Guest, 26, 0x66);
    assert!(!identity.matches_hello(&wrong_role));
    assert!(!identity.matches_hello(&wrong_caller));
    assert!(!identity.matches_hello(&wrong_locator));

    let same_locator = original
        .locator()
        .clone()
        .verify(&AcceptAllLocatorSignatures, unix_now())
        .unwrap();
    let wrong_capability_route =
        VerifiedRelayRoute::new(same_locator, RouteCapability::new([0xfe; 32]).unwrap());
    let wrong_capability = RelayClientHello::new_challenge_bound_v2(
        RelayRole::Guest,
        &wrong_capability_route,
        RelayAuthExtensionV1::new(
            CallerDeviceDhPublic::new([0x66; 32]).unwrap(),
            Some(vec![2; RELAY_CHALLENGE_PROOF_V2_BYTES]),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(identity.matches_hello(&wrong_capability));
    let authenticated = AuthenticatedRoute::new(
        wrong_capability
            .verify_locator(&AcceptAllLocatorSignatures, unix_now())
            .unwrap()
            .route_map_key(),
        RelayRole::Guest,
        NonZeroU64::new(unix_now() + 30).unwrap(),
    );
    assert_eq!(identity.accepted_expiry(&authenticated), None);

    let authenticator = RenewingAuthenticator::new(4, 30);
    let wrong_bearer = RelayBearerCredential::new(b"wrong-ticket".to_vec());
    let challenge = crate::RelayUpgradeChallenge::generate(
        RelayChallengeKeyId::new("online-reauth-test-key").unwrap(),
        std::time::Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    assert!(matches!(
        authenticator.authenticate(&original, Some(&wrong_bearer), Some(challenge)),
        Err(RelayRejectCode::AuthenticationFailed)
    ));
}

#[test]
fn challenge_proof_wire_used_by_test_is_strict_v2() {
    let proof = vec![2; RELAY_CHALLENGE_PROOF_V2_BYTES];
    assert_eq!(
        RelayChallengeProofV2::decode(&proof).unwrap().as_bytes()[0],
        2
    );
}
