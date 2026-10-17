use super::*;

#[test]
fn a_batch_drains_once_only_after_every_admitted_slot_has_a_terminal_outcome() {
    let mut progress = ImageBatchProgress::default();
    assert!(progress
        .finish("unknown", TerminalOutcome::Applied)
        .is_none());
    for id in ["photo", "missing", "stale", "broken"] {
        progress.enqueue(id);
    }
    progress.enqueue("photo");
    assert_eq!(progress.snapshot().enqueued, 4);
    assert_eq!(progress.snapshot().pending, 4);
    assert!(progress.finish("photo", TerminalOutcome::Applied).is_none());
    assert!(progress
        .finish("missing", TerminalOutcome::Fallback)
        .is_none());
    assert!(progress
        .finish("stale", TerminalOutcome::Discarded)
        .is_none());
    let event = progress
        .finish("broken", TerminalOutcome::Disconnected)
        .unwrap();
    assert_eq!(event.kind, ProgressEventKind::Drained);
    assert_eq!(event.snapshot.pending, 0);
    assert_eq!(event.snapshot.terminal_count(), 4);
    assert_eq!(event.snapshot.applied, 1);
    assert_eq!(event.snapshot.fallback, 1);
    assert_eq!(event.snapshot.discarded, 1);
    assert_eq!(event.snapshot.disconnected, 1);
    assert!(progress
        .finish("broken", TerminalOutcome::Disconnected)
        .is_none());
    assert!(event
        .to_string()
        .contains("batch_drained scope=logical_target_batch session_id="));
}

#[test]
fn a_retried_slot_joins_the_current_batch_or_starts_the_next_after_drain() {
    let mut progress = ImageBatchProgress::default();
    progress.enqueue("a");
    progress.enqueue("b");
    let session = progress.snapshot().session_id;
    assert!(progress.finish("a", TerminalOutcome::Discarded).is_none());
    progress.enqueue("a");
    assert_eq!(progress.snapshot().batch_id, 1);
    assert_eq!(progress.snapshot().enqueued, 3);
    assert_eq!(progress.snapshot().pending, 2);
    assert!(progress.finish("a", TerminalOutcome::Applied).is_none());
    assert!(progress.finish("b", TerminalOutcome::Applied).is_some());
    progress.enqueue("a");
    assert_eq!(progress.snapshot().session_id, session);
    assert_eq!(progress.snapshot().batch_id, 2);
    assert_eq!(progress.snapshot().enqueued, 1);
    assert_eq!(progress.snapshot().applied, 0);
    assert_eq!(progress.snapshot().discarded, 0);
}

#[test]
fn reset_cancels_pending_targets_without_emitting_a_successful_drain() {
    let mut progress = ImageBatchProgress::default();
    progress.enqueue("a");
    progress.enqueue("b");
    assert!(progress.finish("a", TerminalOutcome::Applied).is_none());
    let old_session = progress.snapshot().session_id;
    let event = progress.reset().unwrap();
    assert_eq!(event.kind, ProgressEventKind::Cancelled);
    assert_eq!(event.snapshot.session_id, old_session);
    assert_eq!(event.snapshot.enqueued, 2);
    assert_eq!(event.snapshot.applied, 1);
    assert_eq!(event.snapshot.cancelled, 1);
    assert_eq!(event.snapshot.pending, 0);
    assert_eq!(event.snapshot.terminal_count(), event.snapshot.enqueued);
    assert_ne!(progress.snapshot().session_id, old_session);
    assert_eq!(progress.snapshot().batch_id, 0);
    assert!(progress.finish("b", TerminalOutcome::Applied).is_none());
    assert!(progress.reset().is_none());
}

#[test]
fn reset_after_drain_does_not_relabel_the_completed_batch_as_cancelled() {
    let mut progress = ImageBatchProgress::default();
    progress.enqueue("a");
    assert!(progress.finish("a", TerminalOutcome::Fallback).is_some());
    assert!(progress.reset().is_none());
    progress.enqueue("a");
    assert_eq!(progress.snapshot().batch_id, 1);
    assert_eq!(progress.snapshot().fallback, 0);
}
