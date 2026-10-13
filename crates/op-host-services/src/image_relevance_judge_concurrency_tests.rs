use super::*;

struct SlowChatProvider {
    concurrency: Arc<StubConcurrency>,
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl ChatProvider for SlowChatProvider {
    fn provider_label(&self) -> &str {
        "slow-judge"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let concurrency = Arc::clone(&self.concurrency);
        let calls = Arc::clone(&self.calls);
        Box::new(std::iter::once_with(move || {
            use std::sync::atomic::Ordering;
            calls.fetch_add(1, Ordering::SeqCst);
            let active = concurrency.active.fetch_add(1, Ordering::SeqCst) + 1;
            concurrency.peak.fetch_max(active, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(60));
            concurrency.active.fetch_sub(1, Ordering::SeqCst);
            ChatDelta::TextDelta(valid_reply("on", "match"))
        }))
    }
}

#[test]
fn chat_instances_and_mixed_transports_share_the_same_request_budget() {
    let _transport = transport_lock();
    for env_count in [0, 1] {
        let (port, seen, concurrency) = start_controlled_judge_stub(
            "200 OK",
            &valid_reply("on", "match"),
            Duration::from_millis(60),
        );
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut judges: Vec<Box<dyn ImageRelevanceJudge>> = Vec::new();
        for _ in 0..env_count {
            judges.push(Box::new(OpenAiCompatVisionJudge {
                client: reqwest::Client::builder()
                    .use_rustls_tls()
                    .no_proxy()
                    .build()
                    .unwrap(),
                endpoint: format!("http://127.0.0.1:{port}/v1/chat/completions"),
                api_key: "test-key".into(),
                model: "vision-model".into(),
            }));
        }
        for _ in env_count..3 {
            judges.push(Box::new(ChatVisionJudge::new(Arc::new(SlowChatProvider {
                concurrency: Arc::clone(&concurrency),
                calls: Arc::clone(&calls),
            }))));
        }
        let barrier = std::sync::Barrier::new(judges.len());
        std::thread::scope(|scope| {
            let handles: Vec<_> = judges
                .into_iter()
                .map(|judge| {
                    let barrier = &barrier;
                    scope.spawn(move || {
                        barrier.wait();
                        judge.judge("q", "i", &[fake_jpeg(1), fake_jpeg(2), fake_jpeg(3)])
                    })
                })
                .collect();
            for handle in handles {
                let judged = handle.join().unwrap();
                assert_eq!(judged.len(), 3);
                assert!(judged.iter().all(|c| c.verdict == RelevanceVerdict::On));
            }
        });
        assert_eq!(seen.lock().unwrap().len(), env_count * 3);
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            (3 - env_count) * 3
        );
        let peak = concurrency.peak.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            peak > 0 && peak <= MAX_CONCURRENT_CALLS,
            "env={env_count} peak={peak}"
        );
    }
}

#[test]
fn chat_capacity_deadline_cancels_queue_before_provider_is_called() {
    let _transport = transport_lock();
    let (judge, seen) = scripted_judge(&valid_reply("on", "match"));
    let permits = op_image_enrich::net::block_on_image_runtime(async {
        JUDGE_REQUEST_PERMITS
            .acquire_many(MAX_CONCURRENT_CALLS as u32)
            .await
            .unwrap()
    });
    let started = Instant::now();
    let candidate = judge.judge_one(
        0,
        ("q", ""),
        &fake_jpeg(1),
        started + Duration::from_millis(40),
        &JudgeTraceBatch::default(),
    );
    let elapsed = started.elapsed();
    assert_eq!(candidate.verdict, RelevanceVerdict::Off);
    assert_eq!(candidate.reason, "judge unavailable");
    assert!(elapsed < Duration::from_secs(1), "elapsed={elapsed:?}");
    assert!(seen.lock().unwrap().is_empty());
    // Unsupported bounded judging must not enqueue any work, even while all
    // transport capacity is occupied. The synchronous provider is unchanged.
    assert!(judge
        .judge_before("q", "i", &[fake_jpeg(1)], Instant::now())
        .is_empty());
    drop(permits);
    assert_eq!(
        judge.judge("q", "i", &[fake_jpeg(1)])[0].verdict,
        RelevanceVerdict::On
    );
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn concurrent_slots_share_one_http_budget_across_clones_and_instances() {
    let _transport = transport_lock();
    let (port, seen, concurrency) = start_controlled_judge_stub(
        "200 OK",
        &valid_reply("on", "match"),
        Duration::from_millis(60),
    );
    let make_judge = || OpenAiCompatVisionJudge {
        client: reqwest::Client::builder()
            .use_rustls_tls()
            .no_proxy()
            .build()
            .unwrap(),
        endpoint: format!("http://127.0.0.1:{port}/v1/chat/completions"),
        api_key: "test-key".into(),
        model: "vision-model".into(),
    };
    let judge = make_judge();
    let judges = [judge.clone(), judge, make_judge()];
    let barrier = std::sync::Barrier::new(judges.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = judges
            .into_iter()
            .map(|judge| {
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    judge.judge("q", "i", &[fake_jpeg(1), fake_jpeg(2), fake_jpeg(3)])
                })
            })
            .collect();
        for handle in handles {
            let judged = handle.join().unwrap();
            assert_eq!(judged.len(), 3);
            assert!(judged.iter().all(|c| c.verdict == RelevanceVerdict::On));
        }
    });
    assert_eq!(seen.lock().unwrap().len(), 9);
    let peak = concurrency.peak.load(std::sync::atomic::Ordering::SeqCst);
    assert!(peak > 0 && peak <= MAX_CONCURRENT_CALLS, "peak={peak}");
}

#[test]
fn deadline_cancels_queued_http_without_opening_a_request() {
    let _transport = transport_lock();
    let (port, seen) = start_verdict_stub(&valid_reply("on", "match"));
    let judge = OpenAiCompatVisionJudge {
        client: reqwest::Client::builder()
            .use_rustls_tls()
            .no_proxy()
            .build()
            .unwrap(),
        endpoint: format!("http://127.0.0.1:{port}/v1/chat/completions"),
        api_key: "test-key".into(),
        model: "vision-model".into(),
    };
    let permits = op_image_enrich::net::block_on_image_runtime(async {
        JUDGE_REQUEST_PERMITS
            .acquire_many(MAX_CONCURRENT_CALLS as u32)
            .await
            .unwrap()
    });
    let started = Instant::now();
    let judged = judge.judge_before(
        "q",
        "i",
        &[fake_jpeg(1)],
        started + Duration::from_millis(40),
    );
    let elapsed = started.elapsed();
    drop(permits);
    assert!(judged.is_empty());
    assert!(elapsed < Duration::from_secs(1), "elapsed={elapsed:?}");
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn env_judge_deadline_keeps_completed_verdicts_and_cancels_a_silent_candidate() {
    let _transport = transport_lock();
    use std::io::{Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let (stop_tx, stop_rx) = std::sync::mpsc::channel();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let server_calls = Arc::clone(&calls);
    let server = std::thread::spawn(move || {
        let mut sockets = Vec::new();
        let server_deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < server_deadline && stop_rx.try_recv().is_err() {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    socket
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .unwrap();
                    let mut head = Vec::new();
                    let mut byte = [0_u8];
                    while !head.ends_with(b"\r\n\r\n") && head.len() < 64 * 1024 {
                        if socket.read(&mut byte).unwrap_or(0) == 0 {
                            break;
                        }
                        head.push(byte[0]);
                    }
                    let len = String::from_utf8_lossy(&head)
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    let mut body = vec![0; len];
                    let _ = socket.read_exact(&mut body);
                    let index = server_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if index == 0 {
                        let payload = serde_json::json!({
                            "choices": [{"message": {"content": valid_reply("on", "match")}}]
                        })
                        .to_string();
                        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", payload.len(), payload);
                        socket.write_all(response.as_bytes()).unwrap();
                    } else {
                        // Hold the second request silent. The client must
                        // cancel this future without waiting for 120s retries.
                        sockets.push(socket);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("stub accept: {error}"),
            }
        }
    });
    let judge = OpenAiCompatVisionJudge {
        client: reqwest::Client::builder()
            .use_rustls_tls()
            .no_proxy()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap(),
        endpoint,
        api_key: "test-key".into(),
        model: "vision-model".into(),
    };
    assert!(judge.supports_deadline_judging());
    let started = Instant::now();
    let judged = op_image_enrich::net::block_on_image_runtime(async {
        judge.judge_before(
            "serum",
            "product",
            &[fake_jpeg(1), fake_jpeg(2)],
            started + Duration::from_millis(250),
        )
    });
    let elapsed = started.elapsed();
    let _ = stop_tx.send(());
    server.join().unwrap();
    assert!(
        elapsed < Duration::from_secs(1),
        "sync bridge must respect the batch deadline: {elapsed:?}"
    );
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "expired work must not retry"
    );
    assert_eq!(judged.len(), 1, "retain the completed result at timeout");
    assert_eq!(judged[0].verdict, RelevanceVerdict::On);
}
