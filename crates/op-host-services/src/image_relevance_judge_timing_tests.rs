use super::*;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};

#[test]
fn chat_trace_distinguishes_completed_evidence_from_parse_failure() {
    let _transport = transport_lock();
    for (reply, expected_stage) in [
        (valid_reply("off", "different subject"), None),
        ("invalid evidence".to_string(), Some("parse")),
    ] {
        let (judge, seen) = scripted_judge(&reply);
        let (trace, record) = JudgeTraceBatch::capture_for_test();
        let candidate = judge.judge_one(
            0,
            ("q", "i"),
            &fake_jpeg(1),
            Instant::now() + Duration::from_secs(1),
            &trace,
        );
        assert_eq!(candidate.verdict, RelevanceVerdict::Off);
        assert_eq!(seen.lock().unwrap().len(), 1);
        let record = record();
        assert_eq!(record["failure_stage"].as_str(), expected_stage);
        assert_eq!(record["phase_timing"], "available");
        assert!(record["queue_ms"].is_u64());
        assert!(record["provider_ms"].is_u64());
        assert!(
            record["budget_after_queue_ms"].as_u64().unwrap()
                <= record["budget_before_queue_ms"].as_u64().unwrap()
        );
    }
}

#[test]
fn preflight_failures_have_no_provider_duration() {
    struct MustNotStart;
    impl ChatProvider for MustNotStart {
        fn provider_label(&self) -> &str {
            "must-not-start-timing"
        }
        fn send(&self, _: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            panic!("invalid requests must not enter provider.send")
        }
        fn supports_cancellable_send(&self) -> bool {
            true
        }
        fn supports_joined_cancellable_send(&self) -> bool {
            true
        }
        fn send_cancellable_joined(
            &self,
            _: ChatRequest,
            _: Arc<AtomicBool>,
        ) -> Option<Box<dyn Iterator<Item = ChatDelta> + Send>> {
            panic!("preflight failure must not enter provider.send_cancellable_joined")
        }
    }
    let client = ChatVisionLlmClient::new(Arc::new(MustNotStart));
    let request = || VisionCallRequest {
        system: String::new(),
        message: "q".to_string(),
        image_base64: "not valid base64!".to_string(),
        model: None,
        provider: None,
        timeout: Duration::from_secs(1),
    };
    let results = [
        client.validate_measured(request()),
        client.validate_before_measured(request(), Instant::now(), MAX_TOKENS),
        client.validate_before_measured(
            request(),
            Instant::now() + Duration::from_secs(1),
            MAX_TOKENS,
        ),
    ];
    for (response, provider_elapsed) in results {
        assert!(matches!(response, VisionResponse::Skipped { .. }));
        assert!(provider_elapsed.is_none());
    }
}

struct GatedProvider {
    entered: Sender<()>,
    release: Arc<Mutex<Receiver<()>>>,
    concurrency: Arc<StubConcurrency>,
}

impl ChatProvider for GatedProvider {
    fn provider_label(&self) -> &str {
        "gated-judge-timing"
    }

    fn send(&self, _: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let entered = self.entered.clone();
        let release = Arc::clone(&self.release);
        let concurrency = Arc::clone(&self.concurrency);
        Box::new(std::iter::once_with(move || {
            let active = concurrency.active.fetch_add(1, Ordering::SeqCst) + 1;
            concurrency.peak.fetch_max(active, Ordering::SeqCst);
            entered.send(()).unwrap();
            release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(2))
                .expect("test releases the provider");
            concurrency.active.fetch_sub(1, Ordering::SeqCst);
            ChatDelta::TextDelta(valid_reply("on", "match"))
        }))
    }
}

#[test]
fn six_chat_slots_keep_three_transport_permits_and_separate_phase_records() {
    let _transport = transport_lock();
    let (entered_tx, entered_rx) = channel();
    let (release_tx, release_rx) = channel();
    let concurrency = Arc::new(StubConcurrency::default());
    let provider = Arc::new(GatedProvider {
        entered: entered_tx,
        release: Arc::new(Mutex::new(release_rx)),
        concurrency: Arc::clone(&concurrency),
    });
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..6)
            .map(|index| {
                let provider = provider.clone();
                scope.spawn(move || {
                    let judge = ChatVisionJudge::new(provider);
                    let (trace, record) = JudgeTraceBatch::capture_for_test();
                    let candidate = judge.judge_one(
                        index,
                        ("q", "i"),
                        &fake_jpeg(index as u8),
                        Instant::now() + Duration::from_secs(5),
                        &trace,
                    );
                    (candidate, record())
                })
            })
            .collect();
        for _ in 0..3 {
            entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        assert_eq!(concurrency.active.load(Ordering::SeqCst), 3);
        assert_eq!(JUDGE_REQUEST_PERMITS.available_permits(), 0);
        assert!(entered_rx.try_recv().is_err());
        for _ in 0..3 {
            release_tx.send(()).unwrap();
            entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        for _ in 0..3 {
            release_tx.send(()).unwrap();
        }
        for handle in handles {
            let (candidate, record) = handle.join().unwrap();
            assert_eq!(candidate.verdict, RelevanceVerdict::On);
            assert_eq!(record["candidate_index"], candidate.index);
            assert_eq!(record["phase_timing"], "available");
            assert!(record["failure_stage"].is_null());
            let queue = record["queue_ms"].as_u64().unwrap();
            let provider = record["provider_ms"].as_u64().unwrap();
            assert!(record["elapsed_ms"].as_u64().unwrap() >= queue + provider);
        }
    });
    assert_eq!(concurrency.peak.load(Ordering::SeqCst), 3);
    assert_eq!(concurrency.active.load(Ordering::SeqCst), 0);
    assert_eq!(JUDGE_REQUEST_PERMITS.available_permits(), 3);
}

struct CleanupWork {
    started: Instant,
    entered: Sender<()>,
    release: Arc<Mutex<Receiver<()>>>,
    minimum_provider_ms: Arc<AtomicU64>,
}

impl Drop for CleanupWork {
    fn drop(&mut self) {
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(2))
            .expect("test releases joined cancellation cleanup");
        self.minimum_provider_ms
            .store(self.started.elapsed().as_millis() as u64, Ordering::SeqCst);
    }
}

struct JoinedTimingProvider {
    cleanup: Mutex<Option<CleanupWork>>,
}

impl ChatProvider for JoinedTimingProvider {
    fn provider_label(&self) -> &str {
        "joined-judge-timing"
    }
    fn send(&self, _: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        panic!("joined provider must not call legacy send")
    }
    fn supports_cancellable_send(&self) -> bool {
        true
    }
    fn supports_joined_cancellable_send(&self) -> bool {
        true
    }
    fn send_cancellable_joined(
        &self,
        _: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Option<Box<dyn Iterator<Item = ChatDelta> + Send>> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let mut work = self.cleanup.lock().unwrap().take().unwrap();
        work.started = Instant::now();
        let task = crate::chat_runtime::shared_runtime().spawn(async move {
            let _work = work;
            let _tx = tx;
            std::future::pending::<()>().await;
        });
        Some(Box::new(
            crate::chat_runtime::BlockingRecvIter::cancellable_joined(rx, cancel, task),
        ))
    }
}

#[test]
fn provider_timing_and_permit_include_joined_cancellation_cleanup() {
    let _transport = transport_lock();
    let (entered_tx, entered_rx) = channel();
    let (release_tx, release_rx) = channel();
    let minimum_provider_ms = Arc::new(AtomicU64::new(0));
    let judge = ChatVisionJudge::new(Arc::new(JoinedTimingProvider {
        cleanup: Mutex::new(Some(CleanupWork {
            started: Instant::now(),
            entered: entered_tx,
            release: Arc::new(Mutex::new(release_rx)),
            minimum_provider_ms: Arc::clone(&minimum_provider_ms),
        })),
    }));
    let (done_tx, done_rx) = channel();
    let handle = std::thread::spawn(move || {
        let (trace, record) = JudgeTraceBatch::capture_for_test();
        let candidate = judge.judge_one(
            0,
            ("q", "i"),
            &fake_jpeg(1),
            Instant::now() + Duration::from_millis(120),
            &trace,
        );
        done_tx.send((candidate, record())).unwrap();
    });
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(done_rx.try_recv().is_err(), "cleanup has not joined yet");
    assert_eq!(JUDGE_REQUEST_PERMITS.available_permits(), 2);
    release_tx.send(()).unwrap();
    let (candidate, record) = done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    handle.join().unwrap();
    assert_eq!(candidate.reason, "judge unavailable");
    assert_eq!(record["failure_stage"], "provider");
    assert!(record["provider_ms"].as_u64().unwrap() >= minimum_provider_ms.load(Ordering::SeqCst));
    assert_eq!(JUDGE_REQUEST_PERMITS.available_permits(), 3);
}
