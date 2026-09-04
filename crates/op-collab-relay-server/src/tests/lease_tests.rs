//! The authenticated waiting lease and the capability that advertises it.
//!
//! Without a lease, `waiting_timeout` is a hard countdown from registration, so
//! the relay retires healthy owner lanes on a fixed cadence and the client has
//! to out-race it by re-dialling. With a lease the countdown restarts on every
//! pong, so a live peer simply stays in the queue.

use op_collab_relay_protocol::{RelayWaitingAdvertisementV1, RELAY_WAITING_HEADER_NAME};

use super::*;

/// A waiting window short enough to test, with a lease ping inside it.
fn lease_config(waiting: Duration, idle: Duration) -> RelayConfig {
    let mut config = test_config();
    config.waiting_timeout = waiting;
    config.idle_timeout = idle;
    config.tunnel_lifetime = Duration::from_secs(60);
    config
}

/// Answer the relay's lease pings for `duration`, failing if it says anything
/// else. Returns once the window has elapsed with the connection still open.
async fn pong_for(socket: &mut ClientSocket, duration: Duration) {
    let deadline = tokio::time::Instant::now() + duration;
    loop {
        let Ok(message) = time::timeout_at(deadline, socket.next()).await else {
            return;
        };
        match message
            .expect("relay keeps a ponging peer's connection open")
            .expect("relay frame is valid WebSocket")
        {
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .await
                .expect("client answers a lease ping"),
            Message::Pong(_) => {}
            other => panic!("a leased waiting peer must not be retired, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn a_ponging_peer_keeps_its_waiting_slot_past_the_window() {
    let config = lease_config(Duration::from_millis(1_500), Duration::from_secs(30));
    assert!(config.waiting_lease);
    let server = TestServer::start(config).await;
    let mut owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 0x61)).await;

    // Twice the un-leased window: without renewal this is a guaranteed
    // `Rejected(PairingTimeout)`.
    pong_for(&mut owner, Duration::from_secs(3)).await;

    // And the lane is still a real member of the queue, not just an open
    // socket: a guest arriving now pairs with it.
    let mut guest = connect_ready(&server, &make_valid_hello(RelayRole::Guest, 0x61)).await;
    assert_eq!(next_status(&mut guest).await, RelayServerStatus::Paired);
    assert_eq!(
        next_status_within(&mut owner, Duration::from_secs(2)).await,
        RelayServerStatus::Paired
    );

    server.stop().await;
}

#[tokio::test]
async fn a_silent_peer_is_still_reaped_by_the_idle_window() {
    // The lease renews on proof of liveness only, so a dead-but-open socket —
    // the NAT-held flow the lease is most at risk of hoarding — is still
    // collected by `idle_timeout`.
    //
    // "Silent" has to mean *not reading*: tungstenite answers a ping as a side
    // effect of reading one, which is exactly why the lease works for clients
    // that predate it, and exactly why a test cannot simulate a dead peer by
    // reading and ignoring.
    let config = lease_config(Duration::from_secs(30), Duration::from_secs(1));
    let server = TestServer::start(config).await;
    let mut owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 0x62)).await;

    time::sleep(Duration::from_secs(3)).await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let closed = loop {
        match time::timeout_at(deadline, owner.next())
            .await
            .expect("the idle reaper fires")
            .expect("relay socket delivers the close")
            .expect("relay close frame is valid WebSocket")
        {
            // Buffered while the peer was not reading; never answered in time.
            Message::Ping(_) => {}
            Message::Close(frame) => break frame,
            other => panic!("expected an idle close, got {other:?}"),
        }
    };
    assert_eq!(
        closed.expect("idle close carries a reason").reason.as_ref(),
        "idle timeout"
    );

    server.stop().await;
}

#[tokio::test]
async fn disabling_the_lease_restores_the_fixed_countdown() {
    let mut config = lease_config(Duration::from_millis(400), Duration::from_secs(30));
    config.waiting_lease = false;
    let server = TestServer::start(config).await;
    let mut owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 0x63)).await;

    assert_eq!(
        next_status_within(&mut owner, Duration::from_secs(3)).await,
        RelayServerStatus::Rejected(RelayRejectCode::PairingTimeout)
    );

    server.stop().await;
}

#[tokio::test]
async fn the_upgrade_response_advertises_the_waiting_capability() {
    let config = lease_config(Duration::from_secs(30), Duration::from_secs(30));
    let server = TestServer::start(config).await;
    let (socket, response) = connect_with_response(server.address).await;
    let advertised = RelayWaitingAdvertisementV1::decode_header(
        response
            .headers()
            .get(RELAY_WAITING_HEADER_NAME)
            .expect("relay advertises its waiting capability")
            .to_str()
            .expect("advertisement is ASCII"),
    )
    .expect("advertisement decodes");
    assert!(advertised.renewable());
    // With a lease the advertised window is the ceiling a healthy peer may
    // reach, not the per-lease countdown.
    assert_eq!(advertised.window_secs(), 60);
    drop(socket);
    server.stop().await;

    let mut config = lease_config(Duration::from_secs(30), Duration::from_secs(30));
    config.waiting_lease = false;
    let server = TestServer::start(config).await;
    let (socket, response) = connect_with_response(server.address).await;
    let advertised = RelayWaitingAdvertisementV1::decode_header(
        response
            .headers()
            .get(RELAY_WAITING_HEADER_NAME)
            .expect("relay advertises its waiting capability")
            .to_str()
            .expect("advertisement is ASCII"),
    )
    .expect("advertisement decodes");
    assert!(!advertised.renewable());
    assert_eq!(advertised.window_secs(), 30);
    drop(socket);
    server.stop().await;
}
