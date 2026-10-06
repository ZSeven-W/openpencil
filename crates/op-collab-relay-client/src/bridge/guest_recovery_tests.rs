use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guest_pair_gate_keeps_slow_relay_wait_outside_the_local_transport_deadline() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        socket
            .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(5_200)).await;
        socket
            .send(Message::Binary(RelayServerStatus::Paired.encode().to_vec()))
            .await
            .unwrap();
        socket
            .send(Message::Binary(b"noise-may-start-now".to_vec()))
            .await
            .unwrap();
        let _ = socket.next().await;
    });

    let mut limits = test_limits();
    limits.pair = Duration::from_secs(7);
    limits.lifetime = Duration::from_secs(10);
    let bridge = RelayGuestBridge::start_test(endpoint(relay_addr), handshake(5), limits)
        .await
        .unwrap();
    let started = tokio::time::Instant::now();
    bridge
        .wait_until_paired(Duration::from_secs(7))
        .await
        .unwrap();
    assert!(tokio::time::Instant::now().duration_since(started) > Duration::from_secs(5));

    // The inner transport connects only after pairing, so its own handshake
    // clock starts here rather than six seconds ago.
    let mut local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    let mut inbound = [0_u8; 19];
    local.read_exact(&mut inbound).await.unwrap();
    assert_eq!(&inbound, b"noise-may-start-now");

    bridge.stop().await.unwrap();
    server.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guest_pair_gate_preserves_an_authentication_rejection() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        socket
            .send(Message::Binary(
                RelayServerStatus::Rejected(
                    op_collab_relay_protocol::RelayRejectCode::AuthenticationFailed,
                )
                .encode()
                .to_vec(),
            ))
            .await
            .unwrap();
    });

    let bridge = RelayGuestBridge::start_test(endpoint(relay_addr), handshake(5), test_limits())
        .await
        .unwrap();
    let error = bridge
        // The rejection WILL arrive; the budget only bounds how loaded a
        // machine may be before the wait is misreported as a timeout, and
        // this test asserts WHICH error came back. At one second a busy
        // runner returned PairTimeout instead of the rejection it was
        // checking for. Its siblings below already use 3-7s.
        .wait_until_paired(Duration::from_secs(10))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        RelayClientError::GuestPairingFailed {
            kind: RelayFailureKind::Rejected(
                op_collab_relay_protocol::RelayRejectCode::AuthenticationFailed
            )
        }
    ));

    server.await.unwrap();
    bridge.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_guest_ping_pong_survives_multiple_idle_windows_without_local_tcp_traffic() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let (observed_tx, observed_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        mark_ready_and_paired(&mut socket).await;
        let mut pings = 0_usize;
        let mut observed_tx = Some(observed_tx);
        while let Some(message) = socket.next().await {
            let Ok(message) = message else {
                break;
            };
            match message {
                Message::Ping(payload) => {
                    pings += 1;
                    socket.send(Message::Pong(payload)).await.unwrap();
                    if pings == 6 {
                        let _ = observed_tx.take().unwrap().send(());
                    }
                }
                Message::Close(_) => break,
                other => panic!("idle local TCP must emit only keepalive frames: {other:?}"),
            }
        }
    });

    let mut limits = test_limits();
    limits.connect = Duration::from_secs(3);
    limits.hello = Duration::from_secs(3);
    limits.pair = Duration::from_secs(3);
    limits.keepalive = Duration::from_millis(20);
    limits.idle = Duration::from_millis(70);
    limits.lifetime = Duration::from_secs(5);
    limits.stop = Duration::from_secs(2);
    let bridge = RelayGuestBridge::start_test(endpoint(relay_addr), handshake(5), limits)
        .await
        .unwrap();
    bridge
        .wait_until_paired(Duration::from_secs(3))
        .await
        .unwrap();
    let _idle_local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), observed_rx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bridge.status().phase, RelayBridgePhase::Active);

    bridge.stop().await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_guest_processes_pongs_while_local_tcp_writes_continuously() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let (observed_tx, observed_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        mark_ready_and_paired(&mut socket).await;
        let mut pings = 0_usize;
        let mut observed_tx = Some(observed_tx);
        while let Some(message) = socket.next().await {
            let Ok(message) = message else {
                break;
            };
            match message {
                Message::Ping(payload) => {
                    pings += 1;
                    socket.send(Message::Pong(payload)).await.unwrap();
                    if pings == 6 {
                        let _ = observed_tx.take().unwrap().send(());
                    }
                }
                Message::Binary(_) => {}
                Message::Close(_) => break,
                other => panic!("unexpected active tunnel frame: {other:?}"),
            }
        }
    });

    let mut limits = test_limits();
    limits.connect = Duration::from_secs(3);
    limits.hello = Duration::from_secs(3);
    limits.pair = Duration::from_secs(3);
    limits.keepalive = Duration::from_millis(20);
    limits.idle = Duration::from_millis(70);
    limits.lifetime = Duration::from_secs(5);
    limits.stop = Duration::from_secs(2);
    let bridge = RelayGuestBridge::start_test(endpoint(relay_addr), handshake(5), limits)
        .await
        .unwrap();
    bridge
        .wait_until_paired(Duration::from_secs(3))
        .await
        .unwrap();
    let mut local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    let writer = tokio::spawn(async move {
        loop {
            if local.write_all(b"one-way-noise").await.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });
    tokio::time::timeout(Duration::from_secs(1), observed_rx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bridge.status().phase, RelayBridgePhase::Active);

    writer.abort();
    let _ = writer.await;
    bridge.stop().await.unwrap();
    // This test proves Pong processing, not how quickly a simulated relay
    // observes an abruptly dropped client socket during teardown.
    server.abort();
    let _ = server.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_guest_without_a_pong_fails_on_the_bounded_idle_watchdog() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        mark_ready_and_paired(&mut socket).await;
        // Do not poll the WebSocket: tungstenite therefore cannot synthesize a
        // Pong, which models a black-holed relay path with a writable local TCP
        // buffer.
        tokio::time::sleep(Duration::from_millis(250)).await;
        drop(socket);
    });

    let mut limits = test_limits();
    limits.connect = Duration::from_secs(3);
    limits.hello = Duration::from_secs(3);
    limits.pair = Duration::from_secs(3);
    limits.keepalive = Duration::from_millis(20);
    limits.idle = Duration::from_millis(70);
    limits.lifetime = Duration::from_secs(5);
    limits.stop = Duration::from_secs(2);
    let bridge = RelayGuestBridge::start_test(endpoint(relay_addr), handshake(5), limits)
        .await
        .unwrap();
    bridge
        .wait_until_paired(Duration::from_secs(3))
        .await
        .unwrap();
    let mut idle_local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    let mut eof = [0_u8; 1];
    let _ = tokio::time::timeout(Duration::from_secs(1), idle_local.read(&mut eof))
        .await
        .expect("relay failure closes the loopback");
    let failed = bridge.status();
    assert_eq!(failed.phase, RelayBridgePhase::Failed);
    assert_eq!(failed.last_error, Some(RelayFailureKind::IdleTimeout));

    server.await.unwrap();
    bridge.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn continuous_local_writes_cannot_starve_the_unanswered_ping_watchdog() {
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = relay_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        receive_hello(&mut socket, RelayRole::Guest).await;
        mark_ready_and_paired(&mut socket).await;
        // Leave the WebSocket writable but never read the Ping or return any
        // peer activity. Local Noise traffic must not postpone its watchdog.
        tokio::time::sleep(Duration::from_millis(500)).await;
        drop(socket);
    });

    let mut limits = test_limits();
    limits.connect = Duration::from_secs(3);
    limits.hello = Duration::from_secs(3);
    limits.pair = Duration::from_secs(3);
    limits.keepalive = Duration::from_millis(20);
    limits.idle = Duration::from_millis(70);
    limits.lifetime = Duration::from_secs(5);
    limits.stop = Duration::from_secs(2);
    let bridge = RelayGuestBridge::start_test(endpoint(relay_addr), handshake(5), limits)
        .await
        .unwrap();
    bridge
        .wait_until_paired(Duration::from_secs(3))
        .await
        .unwrap();
    let mut local = TcpStream::connect(bridge.local_addr()).await.unwrap();
    let writer = tokio::spawn(async move {
        loop {
            if local.write_all(b"one-way-noise").await.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });
    let mut statuses = bridge.subscribe();
    let failed = wait_for_phase(&mut statuses, RelayBridgePhase::Failed).await;
    assert_eq!(failed.last_error, Some(RelayFailureKind::IdleTimeout));

    writer.abort();
    server.await.unwrap();
    bridge.stop().await.unwrap();
}
