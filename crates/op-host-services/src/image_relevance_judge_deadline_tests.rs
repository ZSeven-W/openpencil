use super::*;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
struct WorkState {
    requests: Mutex<Vec<ChatRequest>>,
    active: AtomicUsize,
    exited: AtomicUsize,
    peak: AtomicUsize,
    released_early: AtomicBool,
}

struct JoinedProvider {
    state: Arc<WorkState>,
}

struct ActiveWork(Arc<WorkState>);

impl Drop for ActiveWork {
    fn drop(&mut self) {
        // Cleanup happens in the actual async transport task, not in the
        // consumer. A returned judge with exited < started leaked work.
        if JUDGE_REQUEST_PERMITS.available_permits() == MAX_CONCURRENT_CALLS {
            self.0.released_early.store(true, Ordering::SeqCst);
        }
        self.0.active.fetch_sub(1, Ordering::SeqCst);
        self.0.exited.fetch_add(1, Ordering::SeqCst);
    }
}

impl ChatProvider for JoinedProvider {
    fn provider_label(&self) -> &str {
        "joined-judge-test"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        panic!("a joined-capable judge must not call legacy send")
    }

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn supports_joined_cancellable_send(&self) -> bool {
        true
    }

    fn send_cancellable_joined(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Option<Box<dyn Iterator<Item = ChatDelta> + Send>> {
        assert!(!cancel.load(Ordering::Acquire));
        let seed = *request.attachments[0].data.last().unwrap();
        self.state.requests.lock().unwrap().push(request);
        let (tx, rx) = tokio::sync::mpsc::channel(2);
        let state = Arc::clone(&self.state);
        let task = crate::chat_runtime::shared_runtime().spawn(async move {
            let active = state.active.fetch_add(1, Ordering::SeqCst) + 1;
            state.peak.fetch_max(active, Ordering::SeqCst);
            let _work = ActiveWork(state);
            if seed == 1 {
                let _ = tx
                    .send(ChatDelta::TextDelta(valid_reply("weak", "soft light")))
                    .await;
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    })
                    .await;
            } else {
                if seed == 2 {
                    // Syntactically complete text is still unverified when
                    // the transport remains unfinished until cancellation.
                    let _ = tx
                        .send(ChatDelta::TextDelta(valid_reply("on", "match")))
                        .await;
                }
                std::future::pending::<()>().await;
            }
        });
        Some(Box::new(
            crate::chat_runtime::BlockingRecvIter::cancellable_joined(rx, cancel, task),
        ))
    }
}

fn joined_judge() -> (ChatVisionJudge, Arc<WorkState>) {
    let state = Arc::new(WorkState::default());
    let provider = Arc::new(JoinedProvider {
        state: Arc::clone(&state),
    });
    (ChatVisionJudge::new(provider), state)
}

#[test]
fn joined_chat_uses_shared_contract_model_and_real_image_format() {
    let _transport = transport_lock();
    let (judge, state) = joined_judge();
    let judge = judge.with_model(Some("vision-test".into()));
    assert!(judge.supports_deadline_judging());
    let result = judge.judge_before(
        "original subject",
        "original appearance contract",
        &[fake_jpeg(1)],
        Instant::now() + Duration::from_secs(1),
    );
    assert_eq!(result[0].verdict, RelevanceVerdict::Weak);
    let requests = state.requests.lock().unwrap();
    let request = &requests[0];
    assert_eq!(
        request.user_message,
        judge_prompt("original subject", "original appearance contract")
    );
    assert!(
        !request.system_prompt.is_empty(),
        "do not inject unrelated generation skills"
    );
    assert_eq!(request.model.as_deref(), Some("vision-test"));
    assert_eq!(request.attachments[0].media_type, "image/jpeg");
    assert_eq!(request.attachments[0].data, fake_jpeg(1));
    assert_eq!(
        request.thinking,
        op_ai::chat_provider::ThinkingMode::Disabled
    );
    assert_eq!(request.effort, op_ai::chat_provider::EffortLevel::Low);
    assert_eq!(request.max_output_tokens, MAX_TOKENS);
    assert_eq!(state.exited.load(Ordering::SeqCst), 1);
}

#[test]
fn joined_chat_keeps_completed_weak_but_discards_partial_and_silent_responses() {
    let _transport = transport_lock();
    let (judge, state) = joined_judge();
    let started = Instant::now();
    let result = judge.judge_before(
        "q",
        "i",
        &[fake_jpeg(1), fake_jpeg(2), fake_jpeg(3), fake_jpeg(4)],
        started + Duration::from_millis(120),
    );
    assert_eq!(result.len(), 3, "expired next wave must not start");
    assert_eq!(result[0].verdict, RelevanceVerdict::Weak);
    for candidate in &result[1..] {
        assert_eq!(candidate.verdict, RelevanceVerdict::Off);
        assert_eq!(candidate.reason, "judge unavailable");
    }
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(state.requests.lock().unwrap().len(), 3);
    assert_eq!(
        state.exited.load(Ordering::SeqCst),
        3,
        "all actual transport tasks must exit"
    );
    assert_eq!(state.active.load(Ordering::SeqCst), 0);
    assert!(!state.released_early.load(Ordering::SeqCst));
    assert_eq!(
        JUDGE_REQUEST_PERMITS.available_permits(),
        MAX_CONCURRENT_CALLS
    );
}

#[test]
fn multiple_joined_chat_slots_never_exceed_transport_capacity() {
    let _transport = transport_lock();
    let state = Arc::new(WorkState::default());
    let barrier = std::sync::Barrier::new(3);
    let deadline = Instant::now() + Duration::from_millis(160);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..3)
            .map(|_| {
                let judge = ChatVisionJudge::new(Arc::new(JoinedProvider {
                    state: Arc::clone(&state),
                }));
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    judge.judge_before(
                        "q",
                        "i",
                        &[fake_jpeg(3), fake_jpeg(3), fake_jpeg(3)],
                        deadline,
                    )
                })
            })
            .collect();
        for handle in handles {
            assert!(handle
                .join()
                .unwrap()
                .iter()
                .all(|candidate| candidate.verdict == RelevanceVerdict::Off));
        }
    });
    assert_eq!(state.peak.load(Ordering::SeqCst), MAX_CONCURRENT_CALLS);
    assert_eq!(state.active.load(Ordering::SeqCst), 0);
    assert_eq!(state.requests.lock().unwrap().len(), MAX_CONCURRENT_CALLS);
    assert_eq!(state.exited.load(Ordering::SeqCst), MAX_CONCURRENT_CALLS);
    assert!(!state.released_early.load(Ordering::SeqCst));
    assert_eq!(
        JUDGE_REQUEST_PERMITS.available_permits(),
        MAX_CONCURRENT_CALLS
    );
}

#[test]
fn joined_chat_expired_and_capacity_queued_work_never_starts_provider() {
    let _transport = transport_lock();
    let (judge, state) = joined_judge();
    assert!(judge
        .judge_before("q", "i", &[fake_jpeg(1)], Instant::now())
        .is_empty());
    let permits = crate::chat_runtime::block_on_anywhere(async {
        JUDGE_REQUEST_PERMITS
            .acquire_many(MAX_CONCURRENT_CALLS as u32)
            .await
            .unwrap()
    });
    let result = judge.judge_before(
        "q",
        "i",
        &[fake_jpeg(1)],
        Instant::now() + Duration::from_millis(40),
    );
    assert_eq!(result[0].verdict, RelevanceVerdict::Off);
    assert!(state.requests.lock().unwrap().is_empty());
    drop(permits);
    // Normal judge also chooses the joined adapter, with its standard budget.
    assert_eq!(
        judge.judge("q", "i", &[fake_jpeg(1)])[0].verdict,
        RelevanceVerdict::Weak
    );
}

#[test]
fn cancellation_flag_alone_does_not_claim_joined_deadline_support() {
    struct CancelOnlyProvider;
    impl ChatProvider for CancelOnlyProvider {
        fn provider_label(&self) -> &str {
            "cancel-only"
        }
        fn send(&self, _: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            Box::new(vec![ChatDelta::TextDelta(valid_reply("on", "match"))].into_iter())
        }
        fn supports_cancellable_send(&self) -> bool {
            true
        }
    }
    let judge = ChatVisionJudge::new(Arc::new(CancelOnlyProvider));
    assert!(!judge.supports_deadline_judging());
    assert!(judge
        .judge_before(
            "q",
            "i",
            &[fake_jpeg(1)],
            Instant::now() + Duration::from_secs(1)
        )
        .is_empty());
    assert_eq!(
        judge.judge("q", "i", &[fake_jpeg(1)])[0].verdict,
        RelevanceVerdict::On
    );
}
