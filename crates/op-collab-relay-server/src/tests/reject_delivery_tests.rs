//! Delivery of a relay rejection: the status frame, the reason repeated in the
//! close frame, and the bounded linger that keeps a reset from eating either.

use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};

use super::*;

async fn next_close_frame(socket: &mut ClientSocket) -> CloseFrame<'static> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        match time::timeout_at(deadline, socket.next())
            .await
            .expect("close arrives before timeout")
            .expect("relay socket delivers the close")
            .expect("relay close frame is valid WebSocket")
        {
            Message::Close(Some(frame)) => return frame,
            Message::Ping(_) | Message::Pong(_) => {}
            other => panic!("expected a close frame with a reason, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn a_pairing_timeout_reject_survives_the_close_and_ends_the_stream_cleanly() {
    let mut config = test_config();
    config.waiting_timeout = Duration::from_millis(100);
    let server = TestServer::start(config).await;
    let mut owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 0x5a)).await;

    assert_eq!(
        next_status(&mut owner).await,
        RelayServerStatus::Rejected(RelayRejectCode::PairingTimeout)
    );

    // The close frame repeats the reason, so a peer that only observes the
    // closing handshake still learns why its lane was retired.
    let frame = next_close_frame(&mut owner).await;
    assert_eq!(frame.code, CloseCode::Policy);
    assert_eq!(
        RelayRejectCode::from_close_reason(frame.reason.as_ref()),
        Some(RelayRejectCode::PairingTimeout)
    );

    // And the relay lingers for the closing handshake instead of dropping the
    // socket underneath it: the stream ends, it does not reset. A reset would
    // surface here as `Protocol(ResetWithoutClosingHandshake)` — exactly the
    // generic transport fault that used to replace the reject reason.
    assert!(
        time::timeout(Duration::from_secs(2), owner.next())
            .await
            .expect("the stream ends before the timeout")
            .is_none(),
        "a rejected peer must see a graceful close, never a reset"
    );

    server.stop().await;
}

#[tokio::test]
async fn every_pre_pairing_rejection_carries_its_reason_in_the_close_frame() {
    let server = TestServer::start(test_config()).await;
    let now = unix_now();
    let expired = make_hello(
        RelayRole::Guest,
        0x5b,
        now.saturating_sub(120),
        now.saturating_sub(60),
        now.saturating_sub(90),
    );
    let mut socket = connect(server.address, "/v1/tunnel")
        .await
        .expect("connect relay");
    send_hello(&mut socket, &expired).await;

    assert_eq!(
        next_status(&mut socket).await,
        RelayServerStatus::Rejected(RelayRejectCode::LocatorExpired)
    );
    assert_eq!(
        RelayRejectCode::from_close_reason(next_close_frame(&mut socket).await.reason.as_ref()),
        Some(RelayRejectCode::LocatorExpired)
    );

    server.stop().await;
}

#[tokio::test]
async fn an_old_client_that_ignores_the_close_reason_still_gets_the_status_frame() {
    // The status frame is the contract; the close reason is a fallback for
    // peers that only ever observe the closing handshake. A client built
    // before the fallback existed reads exactly one binary frame and hangs up,
    // and must still learn why it was retired.
    let mut config = test_config();
    config.waiting_timeout = Duration::from_millis(100);
    config.waiting_lease = false;
    let server = TestServer::start(config).await;
    let mut owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 0x5c)).await;

    assert_eq!(
        next_status(&mut owner).await,
        RelayServerStatus::Rejected(RelayRejectCode::PairingTimeout)
    );
    // Never reads the close frame; drops the socket the way an old client's
    // lane task does when it returns.
    drop(owner);

    server.stop().await;
}

#[tokio::test]
async fn a_reject_survives_a_peer_that_is_writing_when_the_window_expires() {
    // The RST condition. A socket closed while unread bytes sit in its receive
    // queue is closed with RST, and an RST that overtakes the frames already
    // in flight discards them from the peer's buffer — which is how the reject
    // used to turn into `Protocol(ResetWithoutClosingHandshake)`.
    //
    // Here the peer keeps writing right through the rejection and only then
    // reads, so the relay is guaranteed to have unread inbound data at the
    // moment it rejects.
    let mut config = test_config();
    config.waiting_timeout = Duration::from_millis(300);
    config.waiting_lease = false;
    let server = TestServer::start(config).await;
    let owner = connect_ready(&server, &make_valid_hello(RelayRole::Owner, 0x5d)).await;
    let (mut sink, source) = owner.split();

    let writer = tokio::spawn(async move {
        // Pings, because they are the only frame a waiting peer may legally
        // send; anything else would be rejected as a protocol fault instead.
        for _ in 0..40 {
            if sink.send(Message::Ping(vec![7; 32])).await.is_err() {
                break;
            }
            time::sleep(Duration::from_millis(20)).await;
        }
        sink
    });

    let mut source = source;
    let status = loop {
        match time::timeout(Duration::from_secs(3), source.next())
            .await
            .expect("a status arrives before the timeout")
            .expect("the relay does not reset the connection")
            .expect("relay frames stay valid WebSocket")
        {
            Message::Binary(raw) => break RelayServerStatus::decode(&raw).expect("strict status"),
            Message::Pong(_) | Message::Ping(_) => {}
            other => panic!("expected the reject status, got {other:?}"),
        }
    };
    assert_eq!(
        status,
        RelayServerStatus::Rejected(RelayRejectCode::PairingTimeout),
        "a reject must reach a peer that was mid-write when the window expired"
    );

    let _ = writer.await;
    server.stop().await;
}
