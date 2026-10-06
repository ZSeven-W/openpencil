//! Owner relay-lane pool behaviour: how the pool recycles idle lanes on its
//! own budget, replaces a lane a guest has paired with, and refuses to
//! over-provision once a tunnel ends. Split out of `bridge/tests.rs` at the
//! 800-line cap; pure code motion.

use super::*;

/// Poll `probe` until it reads `want`, or fail saying what was awaited and
/// what was last seen.
///
/// Replaces the "loop while != want OR deadline, then assert_eq!(want)" shape
/// this file used in six places. That shape has two exits and only one of them
/// is checked: on a loaded runner it leaves on the DEADLINE and the assertion
/// then reports a bare `0 vs 1`, which reads as a product defect rather than
/// as "it had not settled yet". Measured 2026-08-28 on the macos-aarch64 CI
/// leg, where `a_finished_tunnel_does_not_leave_the_pool_over_provisioned`
/// failed exactly that way.
///
/// The budget is deliberately generous: these are convergence waits, not
/// latency assertions, so a slow runner should make the test slower, never
/// red. Anything genuinely stuck still fails, with a message that says so.
async fn settles_to<T>(label: &str, want: T, mut probe: impl FnMut() -> T)
where
    T: PartialEq + std::fmt::Debug,
{
    let deadline = StdInstant::now() + Duration::from_secs(10);
    loop {
        let seen = probe();
        if seen == want {
            return;
        }
        assert!(
            StdInstant::now() < deadline,
            "{label}: waited for {want:?}, last saw {seen:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_finished_tunnel_does_not_leave_the_pool_over_provisioned() {
    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let owner_driver = tokio::spawn(async move {
        // Accept the tunnel's local end, then drop it to end the tunnel.
        let (stream, _) = owner_listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        drop(stream);
        tokio::time::sleep(Duration::from_secs(3)).await;
    });
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    // Concurrently open relay connections. The relay caps waiting peers per
    // route, so a pool that replaces both the paired lane AND its own
    // replacement would be rejected in production; here it shows up as an
    // over-count.
    let open = Arc::new(AtomicUsize::new(0));
    let server_open = Arc::clone(&open);
    let server = tokio::spawn(async move {
        let mut first = true;
        while let Ok((stream, _)) = relay_listener.accept().await {
            let pair = std::mem::replace(&mut first, false);
            let open = Arc::clone(&server_open);
            tokio::spawn(async move {
                let Ok(mut socket) = accept_async(stream).await else {
                    return;
                };
                receive_hello(&mut socket, RelayRole::Owner).await;
                if pair {
                    mark_ready_and_paired(&mut socket).await;
                } else if socket
                    .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
                    .await
                    .is_err()
                {
                    return;
                }
                open.fetch_add(1, Ordering::SeqCst);
                while socket.next().await.is_some() {}
                open.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });

    let mut limits = test_limits();
    limits.idle = Duration::from_secs(6);
    limits.lifetime = Duration::from_secs(12);
    limits.owner_pair = Duration::from_secs(6);
    let bridge =
        RelayOwnerBridge::start_test(endpoint(relay_addr), handshake(6), owner_addr, 1, limits)
            .await
            .unwrap();

    // Wait for the tunnel to come up and then finish.
    settles_to("tunnel comes up", 1, || bridge.status().active_tunnels).await;
    settles_to("tunnel finishes", 0, || bridge.status().active_tunnels).await;

    // The pool settles back to exactly one waiting lane: the tunnel's slot was
    // already replaced when it paired, so its end owes nothing further.
    settles_to(
        "the pool holds exactly `lane_count` unpaired lanes once the tunnel ends",
        1,
        || open.load(Ordering::SeqCst),
    )
    .await;
    settles_to("waiting lanes", 1, || bridge.status().waiting_lanes).await;

    bridge.stop().await.unwrap();
    server.abort();
    owner_driver.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_paired_owner_lane_is_replaced_so_the_waiting_queue_never_empties() {
    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let owner_driver = tokio::spawn(async move {
        // Hold the tunnel's local end open for the duration of the assertion.
        let (stream, _) = owner_listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(3)).await;
        drop(stream);
    });
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    // The first lane is paired with a guest; every later lane is admitted and
    // left waiting.
    let server = tokio::spawn(async move {
        let mut first = true;
        while let Ok((stream, _)) = relay_listener.accept().await {
            let pair = std::mem::replace(&mut first, false);
            tokio::spawn(async move {
                let Ok(mut socket) = accept_async(stream).await else {
                    return;
                };
                receive_hello(&mut socket, RelayRole::Owner).await;
                if pair {
                    mark_ready_and_paired(&mut socket).await;
                } else if socket
                    .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
                    .await
                    .is_err()
                {
                    return;
                }
                while socket.next().await.is_some() {}
            });
        }
    });

    let mut limits = test_limits();
    limits.idle = Duration::from_secs(6);
    limits.lifetime = Duration::from_secs(12);
    limits.owner_pair = Duration::from_secs(6);
    let bridge =
        RelayOwnerBridge::start_test(endpoint(relay_addr), handshake(6), owner_addr, 1, limits)
            .await
            .unwrap();

    let deadline = StdInstant::now() + Duration::from_secs(3);
    let mut status = bridge.status();
    while (status.active_tunnels == 0 || status.waiting_lanes == 0) && StdInstant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(20)).await;
        status = bridge.status();
    }
    assert_eq!(
        status.active_tunnels, 1,
        "the guest lane must be tunnelling"
    );
    assert!(
        status.waiting_lanes >= 1,
        "a paired lane must be replaced, or the next guest has nothing to pair with"
    );

    bridge.stop().await.unwrap();
    server.abort();
    owner_driver.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_lanes_recycle_on_the_client_budget_and_keep_the_pool_populated() {
    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    // A relay that admits owner lanes and never pairs them — an idle session
    // waiting for its first guest.
    let server_accepted = Arc::clone(&accepted);
    let server = tokio::spawn(async move {
        while let Ok((stream, _)) = relay_listener.accept().await {
            let accepted = Arc::clone(&server_accepted);
            tokio::spawn(async move {
                let Ok(mut socket) = accept_async(stream).await else {
                    return;
                };
                receive_hello(&mut socket, RelayRole::Owner).await;
                if socket
                    .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
                    .await
                    .is_err()
                {
                    return;
                }
                accepted.fetch_add(1, Ordering::SeqCst);
                while socket.next().await.is_some() {}
            });
        }
    });

    let mut limits = test_limits();
    limits.owner_pair = Duration::from_millis(120);
    // Long enough that only an undelayed re-dial can produce another lane
    // inside this test: a recycle must not serve the failure backoff.
    limits.retry = Duration::from_secs(30);
    let bridge =
        RelayOwnerBridge::start_test(endpoint(relay_addr), handshake(6), owner_addr, 2, limits)
            .await
            .unwrap();
    bridge
        .wait_until_ready(Duration::from_secs(2))
        .await
        .unwrap();

    // Staggered first budgets (60 ms and 120 ms) plus a further full cycle.
    let deadline = StdInstant::now() + Duration::from_secs(3);
    while accepted.load(Ordering::SeqCst) <= 2 && StdInstant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
        // A recycle is never a fault, so the pool must not go Degraded and
        // must keep at least one lane in the relay's waiting queue.
        let status = bridge.status();
        assert_eq!(status.last_error, None, "recycling is not a relay failure");
        assert_ne!(status.phase, RelayBridgePhase::Degraded);
    }
    assert!(
        accepted.load(Ordering::SeqCst) > 2,
        "owner lanes must re-dial on the client budget, not wait for the relay"
    );
    // The loop above leaves as soon as the third dial is ACCEPTED, which is
    // before the replacement lane has finished announcing itself — the pool
    // is legitimately `Starting` for that moment. Settle to the resting
    // state instead of sampling mid-recycle (macos-aarch64 CI, 2026-08-29).
    settles_to("pool returns to Waiting", RelayBridgePhase::Waiting, || {
        bridge.status().phase
    })
    .await;
    settles_to("a lane waits in the relay queue", true, || {
        bridge.status().waiting_lanes > 0
    })
    .await;

    bridge.stop().await.unwrap();
    server.abort();
    drop(owner_listener);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_relay_pairing_timeout_recycles_the_lane_without_degrading_the_pool() {
    use op_collab_relay_protocol::RelayRejectCode;
    use tokio_tungstenite::tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};

    // The relay retires every lane the way a real one does when its own
    // waiting window expires first, and it does so with the close frame alone:
    // the status frame is exactly what a connection reset used to swallow, so
    // the client must recover the reason from the closing handshake.
    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        while let Ok((stream, _)) = relay_listener.accept().await {
            tokio::spawn(async move {
                let Ok(mut socket) = accept_async(stream).await else {
                    return;
                };
                receive_hello(&mut socket, RelayRole::Owner).await;
                if socket
                    .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
                    .await
                    .is_err()
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
                let _ = socket
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::Policy,
                        reason: RelayRejectCode::PairingTimeout.close_reason().into(),
                    })))
                    .await;
                while socket.next().await.is_some() {}
            });
        }
    });

    let mut limits = test_limits();
    limits.owner_pair = Duration::from_secs(5);
    let bridge =
        RelayOwnerBridge::start_test(endpoint(relay_addr), handshake(11), owner_addr, 1, limits)
            .await
            .unwrap();

    let deadline = StdInstant::now() + Duration::from_secs(5);
    while bridge.status().relay_pairing_timeouts == 0 && StdInstant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let status = bridge.status();
    assert!(
        status.relay_pairing_timeouts >= 1,
        "a relay-retired lane must be counted so an inverted pairing contract is visible"
    );
    assert_eq!(
        status.last_error, None,
        "a relay recycle is not a pool failure and must not advertise a broken relay"
    );
    assert_ne!(status.phase, RelayBridgePhase::Degraded);
    assert_ne!(status.phase, RelayBridgePhase::Failed);

    bridge.stop().await.unwrap();
    server.abort();
    drop(owner_listener);
}

/// Peak-tracking counter of connections the relay has accepted and not paired.
///
/// This is the number the relay's `max_waiting_per_route` actually bounds, and
/// it is NOT the same as the pool's `unpaired_lanes`: that counts lane tasks,
/// including ones still dialling, which have no slot in the relay's queue yet.
#[derive(Default)]
struct WaitingWatch {
    open: AtomicUsize,
    peak: AtomicUsize,
}

impl WaitingWatch {
    fn enter(&self) {
        let open = self.open.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(open, Ordering::SeqCst);
    }

    fn leave(&self) {
        self.open.fetch_sub(1, Ordering::SeqCst);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_pool_never_exceeds_its_lane_count_in_the_relays_waiting_queue() {
    // `unpaired_lanes` counts lane tasks, not registrations, so a slow dial, a
    // refused dial, and a lane leaving the queue to pair all move the two
    // counts apart. The relay caps waiting peers per route, so the count that
    // matters is how many connections are simultaneously accepted-and-unpaired
    // — never more than `lane_count`, or a production relay would start
    // refusing the pool's own lanes with `Capacity`.
    const LANE_COUNT: usize = 2;

    let owner_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let owner_addr = owner_listener.local_addr().unwrap();
    let owner_driver = tokio::spawn(async move {
        // Hold the paired tunnel's local end open for the whole assertion.
        while let Ok((stream, _)) = owner_listener.accept().await {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(6)).await;
                drop(stream);
            });
        }
    });

    let relay_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay_addr = relay_listener.local_addr().unwrap();
    let watch = Arc::new(WaitingWatch::default());
    let relay_watch = Arc::clone(&watch);
    let server = tokio::spawn(async move {
        let mut nth = 0_usize;
        while let Ok((stream, _)) = relay_listener.accept().await {
            nth += 1;
            let watch = Arc::clone(&relay_watch);
            tokio::spawn(async move {
                // A dial refused outright before the upgrade, the way a relay
                // at capacity or a half-open proxy behaves.
                if nth == 2 {
                    drop(stream);
                    return;
                }
                // A dial whose upgrade is slow, the way a loaded TLS
                // terminator behaves: the lane task exists and counts as
                // unpaired long before the relay has a slot for it.
                if nth == 3 {
                    tokio::time::sleep(Duration::from_millis(400)).await;
                }
                let Ok(mut socket) = accept_async(stream).await else {
                    return;
                };
                receive_hello(&mut socket, RelayRole::Owner).await;
                // Only accepted-and-unpaired connections occupy a queue slot.
                watch.enter();
                if socket
                    .send(Message::Binary(RelayServerStatus::Ready.encode().to_vec()))
                    .await
                    .is_err()
                {
                    watch.leave();
                    return;
                }
                if nth == 3 {
                    // A guest arrives for the slow lane once it is settled in
                    // the queue. It leaves the queue to tunnel, and owes the
                    // pool a replacement dial.
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    if socket
                        .send(Message::Binary(RelayServerStatus::Paired.encode().to_vec()))
                        .await
                        .is_err()
                    {
                        watch.leave();
                        return;
                    }
                    watch.leave();
                    while socket.next().await.is_some() {}
                    return;
                }
                while socket.next().await.is_some() {}
                watch.leave();
            });
        }
    });

    let mut limits = test_limits();
    // Long enough that nothing recycles on schedule during the assertion: the
    // churn under test is the pool's own replacement logic, not the clock.
    limits.owner_pair = Duration::from_secs(30);
    limits.pair = Duration::from_secs(30);
    limits.idle = Duration::from_secs(30);
    limits.lifetime = Duration::from_secs(60);
    let bridge = RelayOwnerBridge::start_test(
        endpoint(relay_addr),
        handshake(12),
        owner_addr,
        LANE_COUNT,
        limits,
    )
    .await
    .unwrap();

    // Wait for the refused dial to retry, the slow upgrade to land, and the
    // pair to settle — by POLLING for the settled state, not by sleeping a
    // fixed span and hoping it was enough. The previous shape waited only for
    // the lower bound (`open >= LANE_COUNT`), slept 500ms, then asserted
    // EXACT equality; on a loaded runner the replacement dial had not always
    // landed inside that window, which is how this test failed on CI while
    // passing locally.
    //
    // The peak bound is an INVARIANT, not an end-state, so it is checked on
    // every poll rather than once at the end: exceeding the lane count for a
    // single instant is the bug, and a check that only looks after the fact
    // can miss it entirely.
    let deadline = StdInstant::now() + Duration::from_secs(10);
    loop {
        let peak = watch.peak.load(Ordering::SeqCst);
        assert!(
            peak <= LANE_COUNT,
            "the pool held {peak} unpaired registrations at once, above its lane count of              {LANE_COUNT}"
        );
        // The settled state is BOTH counts at rest: `lane_count` lanes waiting
        // AND the guest's tunnel up. Polling only the queue would let the
        // assertion run while the pair was still completing.
        if watch.open.load(Ordering::SeqCst) == LANE_COUNT && bridge.status().active_tunnels == 1 {
            break;
        }
        assert!(
            StdInstant::now() < deadline,
            "the pool never settled: {} waiting lanes (want {LANE_COUNT}), {} active tunnels \
             (want 1)",
            watch.open.load(Ordering::SeqCst),
            bridge.status().active_tunnels
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Settled — now confirm it STAYS settled rather than having been passed
    // through on the way somewhere else.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        watch.peak.load(Ordering::SeqCst) <= LANE_COUNT,
        "the pool held {} unpaired registrations at once, above its lane count of {LANE_COUNT}",
        watch.peak.load(Ordering::SeqCst)
    );
    assert_eq!(
        watch.open.load(Ordering::SeqCst),
        LANE_COUNT,
        "the pool must settle back to exactly `lane_count` waiting lanes"
    );
    assert_eq!(bridge.status().active_tunnels, 1);

    bridge.stop().await.unwrap();
    server.abort();
    owner_driver.abort();
}
