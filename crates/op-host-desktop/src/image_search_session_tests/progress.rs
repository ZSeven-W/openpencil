//! Session-level diagnostics use frozen memo results, never external requests.

use super::super::progress::ProgressEventKind;
use super::super::*;
use super::*;

fn state_with_slots(ids: &[&str]) -> EditorState {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    for id in ids {
        state
            .active_children_mut()
            .push(image_node(id, "", Some("ceramic vase")));
    }
    state
}

fn pending_memo(session: &mut ImageSearchSession, state: &EditorState) -> SearchIntentKey {
    let target = collect_targets(state, &HashSet::new()).remove(0);
    let key = search_intent_key(
        &target.query,
        target.aspect_ratio,
        target.prompt.as_deref(),
        target.judge_view,
    );
    session.search_memo.lock().unwrap().insert(
        key.clone(),
        SearchMemoEntry::Pending {
            request_id: 42,
            waiters: Vec::new(),
        },
    );
    key
}

fn drained_count(session: &ImageSearchSession) -> usize {
    session
        .progress_events
        .iter()
        .filter(|event| event.kind == ProgressEventKind::Drained)
        .count()
}

#[test]
fn shared_memo_fetch_still_accounts_for_two_applied_slots_and_drains_once() {
    let mut state = state_with_slots(&["a", "b"]);
    let mut session = ImageSearchSession::new();
    let key = pending_memo(&mut session, &state);
    assert!(session.enqueue_missing(&state));
    assert_eq!(session.progress_snapshot().enqueued, 2);
    assert_eq!(session.progress_snapshot().pending, 2);
    assert_eq!(drained_count(&session), 0);
    let memo = session.search_memo.lock().unwrap();
    let Some(SearchMemoEntry::Pending { waiters, .. }) = memo.get(&key) else {
        panic!("the shared request stays pending")
    };
    assert_eq!(waiters.len(), 2);
    drop(memo);
    assert!(publish_search_result(
        &session.search_memo,
        key,
        42,
        Some("memo.jpg".into())
    ));
    assert!(session.poll_into(&mut state));
    assert!(!session.is_pending());
    assert_eq!(session.progress_snapshot().applied, 2);
    assert_eq!(session.progress_snapshot().pending, 0);
    assert_eq!(drained_count(&session), 1);
    assert!(!session.poll_into(&mut state));
    assert!(!session.enqueue_missing(&state));
    assert_eq!(drained_count(&session), 1);
}

#[test]
fn missing_photo_counts_only_after_the_fallback_is_written() {
    let mut state = state_with_slots(&["a"]);
    let mut session = ImageSearchSession::new();
    let key = pending_memo(&mut session, &state);
    assert!(session.enqueue_missing(&state));
    assert_eq!(session.progress_snapshot().fallback, 0);
    assert!(publish_search_result(&session.search_memo, key, 42, None));
    assert_eq!(session.progress_snapshot().pending, 1);
    assert_eq!(drained_count(&session), 0);
    assert!(session.poll_into(&mut state));
    assert!(is_image_fallback(&state.active_children()[0]));
    assert_eq!(session.progress_snapshot().fallback, 1);
    assert_eq!(session.progress_snapshot().applied, 0);
    assert_eq!(drained_count(&session), 1);
}

#[test]
fn an_explicit_retry_after_drain_starts_a_new_batch() {
    let mut state = state_with_slots(&["a"]);
    let mut session = ImageSearchSession::new();
    let key = pending_memo(&mut session, &state);
    assert!(session.enqueue_missing(&state));
    assert!(publish_search_result(&session.search_memo, key, 42, None));
    assert!(session.poll_into(&mut state));
    let session_id = session.progress_snapshot().session_id;
    assert_eq!(session.progress_snapshot().batch_id, 1);

    // A caller restores the failed slot before retry_search_failures, as the
    // production retry contract requires. Keep the replacement lookup local.
    state.active_children_mut()[0] = image_node("a", "", Some("ceramic vase"));
    state.mark_document_changed();
    let key = pending_memo(&mut session, &state);
    session.retry_search_failures(&HashSet::from([NodeId::new("a")]));
    assert!(session.enqueue_missing(&state));
    assert_eq!(session.progress_snapshot().session_id, session_id);
    assert_eq!(session.progress_snapshot().batch_id, 2);
    assert_eq!(session.progress_snapshot().enqueued, 1);
    assert_eq!(session.progress_snapshot().fallback, 0);
    assert!(publish_search_result(
        &session.search_memo,
        key,
        42,
        Some("retry.jpg".into())
    ));
    assert!(session.poll_into(&mut state));
    assert_eq!(session.progress_snapshot().applied, 1);
    assert_eq!(drained_count(&session), 2);
}

#[test]
fn collaboration_rejection_is_discarded_after_the_result_reaches_the_sink() {
    let mut state = state_with_slots(&["a"]);
    let mut session = ImageSearchSession::new();
    let key = pending_memo(&mut session, &state);
    assert!(session.enqueue_missing(&state));
    assert!(state.editor_ui.collab.set_authenticated_session(
        op_editor_core::CollabConnectionPhase::Active,
        op_editor_core::AuthenticatedCollabSession {
            session_name: "Shared design".into(),
            role: op_editor_core::CollabUiRole::Editor,
            share_endpoint: None,
        },
        Vec::new(),
    ));
    assert!(publish_search_result(
        &session.search_memo,
        key,
        42,
        Some("blocked.jpg".into())
    ));
    assert!(session.poll_into(&mut state));
    assert_eq!(session.progress_snapshot().discarded, 1);
    assert_eq!(session.progress_snapshot().applied, 0);
    assert_eq!(drained_count(&session), 1);
}

#[test]
fn stale_and_disconnected_results_are_terminal_without_claiming_a_photo_applied() {
    let mut state = state_with_slots(&["a", "b"]);
    let mut session = ImageSearchSession::new();
    let key = pending_memo(&mut session, &state);
    assert!(session.enqueue_missing(&state));
    let PenNode::Image(image) = &mut state.active_children_mut()[0] else {
        panic!("image")
    };
    image.image_search_query = Some("different subject".into());
    state.mark_document_changed();
    let Some(SearchMemoEntry::Pending { waiters, .. }) =
        session.search_memo.lock().unwrap().remove(&key)
    else {
        panic!("pending")
    };
    waiters[0]
        .send(JobOutcome::Search(Some("stale.jpg".into())))
        .unwrap();
    drop(waiters);
    assert!(!session.poll_into(&mut state));
    assert_eq!(session.progress_snapshot().discarded, 1);
    assert_eq!(session.progress_snapshot().disconnected, 1);
    assert_eq!(session.progress_snapshot().applied, 0);
    assert_eq!(drained_count(&session), 1);
}

#[test]
fn reset_cancels_the_old_batch_and_same_ids_start_a_new_session() {
    let mut state = state_with_slots(&["a"]);
    let mut session = ImageSearchSession::new();
    let old_key = pending_memo(&mut session, &state);
    let old_memo = Arc::clone(&session.search_memo);
    assert!(session.enqueue_missing(&state));
    let old_session = session.progress_snapshot().session_id;
    session.reset();
    let event = session.progress_events.last().unwrap();
    assert_eq!(event.kind, ProgressEventKind::Cancelled);
    assert_eq!(event.snapshot.session_id, old_session);
    assert_eq!(event.snapshot.cancelled, 1);
    assert_eq!(event.snapshot.pending, 0);
    assert_eq!(drained_count(&session), 0);
    let key = pending_memo(&mut session, &state);
    assert!(session.enqueue_missing(&state));
    assert_ne!(session.progress_snapshot().session_id, old_session);
    assert_eq!(session.progress_snapshot().batch_id, 1);
    assert!(publish_search_result(
        &old_memo,
        old_key,
        42,
        Some("old.jpg".into())
    ));
    assert!(!session.poll_into(&mut state));
    assert_eq!(session.progress_snapshot().pending, 1);
    assert!(publish_search_result(
        &session.search_memo,
        key,
        42,
        Some("new.jpg".into())
    ));
    assert!(session.poll_into(&mut state));
    assert_eq!(session.progress_snapshot().applied, 1);
    assert_eq!(drained_count(&session), 1);
}

#[test]
fn generation_to_stock_keeps_the_same_logical_target_pending() {
    let mut state = state_with_slots(&["a"]);
    let PenNode::Image(image) = &mut state.active_children_mut()[0] else {
        panic!("image")
    };
    image.image_prompt = Some("ceramic vase photograph".into());
    image.image_search_query = None;
    let target = collect_targets(&state, &HashSet::new()).remove(0);
    let mut session = ImageSearchSession::new();
    let key = pending_memo(&mut session, &state);
    let (tx, rx) = mpsc::channel();
    tx.send(JobOutcome::Generate(Err(ImageGenerateError::Busy {
        provider: "fixture",
    })))
    .unwrap();
    session.in_flight.insert("a".into());
    session.progress.enqueue("a");
    session.jobs.push(ImageSearchJob {
        node_id: NodeId::new("a"),
        deadline: None,
        intent: Some(intent_fingerprint(&target, None)),
        stock_view: None,
        fallback_target: Some(target),
        rx,
    });
    assert!(!session.poll_into(&mut state));
    assert!(session.is_pending());
    assert_eq!(session.stats().fell_back, 1);
    assert_eq!(session.progress_snapshot().enqueued, 1);
    assert_eq!(session.progress_snapshot().pending, 1);
    assert_eq!(session.progress_snapshot().fallback, 0);
    assert_eq!(drained_count(&session), 0);
    assert!(publish_search_result(
        &session.search_memo,
        key,
        42,
        Some("stock.jpg".into())
    ));
    assert!(session.poll_into(&mut state));
    assert_eq!(session.progress_snapshot().enqueued, 1);
    assert_eq!(session.progress_snapshot().applied, 1);
    assert_eq!(session.progress_snapshot().pending, 0);
    assert_eq!(drained_count(&session), 1);
}
