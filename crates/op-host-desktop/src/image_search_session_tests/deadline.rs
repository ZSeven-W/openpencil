//! CLI deadlines survive queueing and generation-to-search fallback.

use super::super::*;
use super::*;

#[test]
fn queued_searches_keep_the_run_deadline_and_reset_clears_it() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("photo", "", Some("forest trail")));
    let mut session = ImageSearchSession::new();
    let deadline = Instant::now() + Duration::from_secs(120);
    session.set_deadline(deadline);
    // Keep the worker pending without network so its stored deadline is
    // observable before the UI-thread sink consumes the result.
    let target = collect_targets(&state, &HashSet::new()).remove(0);
    session.search_memo.lock().unwrap().insert(
        search_intent_key(&target.query, target.aspect_ratio, target.prompt.as_deref()),
        SearchMemoEntry::Pending {
            request_id: 42,
            waiters: Vec::new(),
        },
    );

    assert!(session.enqueue_missing(&state));
    assert_eq!(session.jobs.len(), 1);
    assert_eq!(session.jobs[0].deadline, Some(deadline));

    session.reset();
    assert!(session.deadline.is_none());
    assert!(!session.is_pending());
}

#[test]
fn generation_fallback_keeps_its_queued_deadline() {
    let mut state = EditorState::default();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(image_node("photo", "", Some("forest trail")));
    let target = collect_targets(&state, &HashSet::new()).remove(0);
    let original_deadline = Instant::now() + Duration::from_secs(120);
    let (tx, rx) = mpsc::channel();
    tx.send(JobOutcome::Generate(Err(ImageGenerateError::Busy {
        provider: "test",
    })))
    .unwrap();
    let mut session = ImageSearchSession {
        jobs: vec![ImageSearchJob {
            node_id: target.node_id.clone(),
            deadline: Some(original_deadline),
            intent: Some(intent_fingerprint(&target, None)),
            fallback_target: Some(target.clone()),
            rx,
        }],
        ..Default::default()
    };
    session.search_memo.lock().unwrap().insert(
        search_intent_key(&target.query, target.aspect_ratio, target.prompt.as_deref()),
        SearchMemoEntry::Pending {
            request_id: 42,
            waiters: Vec::new(),
        },
    );
    // Even a later session setting cannot extend an already-queued job.
    session.set_deadline(original_deadline + Duration::from_secs(120));

    session.poll_into(&mut state);

    assert_eq!(session.jobs.len(), 1);
    assert_eq!(session.stats().fell_back, 1);
    assert_eq!(session.jobs[0].deadline, Some(original_deadline));
    assert!(session.jobs[0].fallback_target.is_none());
}
