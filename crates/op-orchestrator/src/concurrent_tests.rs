//! `concurrent` tests, part 1 — Task A1 (`effective_concurrency`,
//! `group_subtasks_by_screen`) + Task B1 (`BufferDocSink`,
//! `run_screen_group_worker`).
//!
//! Task B2 (`run_concurrent`) tests live in the sibling `concurrent_tests_b2.rs`
//! — split so both test files stay under the 800-line cap.
//!
//! Wired as `#[path = "concurrent_tests.rs"] mod tests;` inside `concurrent.rs`,
//! so this stays a child module of `concurrent` and `use super::*` resolves to
//! `concurrent`.

use super::*;
use crate::plan::{Region, Subtask};
use crate::test_support::{ScriptResponse, ScriptedLlm};
use crate::types::LlmError;
use futures::executor::block_on;
use op_editor_core::EditorState;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

// ── Helpers ────────────────────────────────────────────────────────────────

fn subtask_with_screen(id: &str, screen: Option<&str>) -> Subtask {
    Subtask {
        id: id.into(),
        label: id.into(),
        id_prefix: id.into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        parent_frame_id: None,
        elements: None,
        screen: screen.map(|s| s.to_string()),
        generated_root_id: None,
    }
}

fn make_plan_with_subtasks(subtask_ids: &[&str]) -> crate::plan::OrchestratorPlan {
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec};
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: subtask_ids
            .iter()
            .map(|id| crate::plan::Subtask {
                id: id.to_string(),
                label: id.to_string(),
                region: Region {
                    width: 1200.0,
                    height: 400.0,
                },
                id_prefix: id.to_string(),
                parent_frame_id: Some("root".into()),
                elements: None,
                screen: None,
                generated_root_id: None,
            })
            .collect(),
        style_guide_name: None,
    }
}

fn make_req() -> crate::types::DesignRequest {
    crate::types::DesignRequest {
        prompt: "test".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 2,
    }
}

const NODE_JSON: &str =
    r#"[{"type":"frame","id":"h1","name":"H","x":0,"y":0,"width":100,"height":100,"children":[]}]"#;

/// Build a default semaphore (2 permits) and an mpsc sender for B1 worker tests.
/// Returns `(semaphore, sender, receiver)`.
fn make_worker_channel() -> (
    Arc<Semaphore>,
    mpsc::UnboundedSender<Progress>,
    mpsc::UnboundedReceiver<Progress>,
) {
    let sem = Arc::new(Semaphore::new(2));
    let (tx, rx) = mpsc::unbounded_channel::<Progress>();
    (sem, tx, rx)
}

/// Drain all progress events from a receiver into a Vec.
fn drain_events(mut rx: mpsc::UnboundedReceiver<Progress>) -> Vec<Progress> {
    let mut events = Vec::new();
    while let Ok(e) = rx.try_recv() {
        events.push(e);
    }
    events
}

// ── effective_concurrency ──────────────────────────────────────────────────

/// (concurrency=1, 3 screens) → 1  (single-threaded even with many screens)
#[test]
fn effective_concurrency_one_concurrency_three_screens_gives_one() {
    assert_eq!(effective_concurrency(1, 3), 1);
}

/// (concurrency=4, 1 screen) → 1  (only one group → sequential)
#[test]
fn effective_concurrency_four_concurrency_one_screen_gives_one() {
    assert_eq!(effective_concurrency(4, 1), 1);
}

/// (concurrency=4, 3 screens) → 4
#[test]
fn effective_concurrency_four_concurrency_three_screens_gives_four() {
    assert_eq!(effective_concurrency(4, 3), 4);
}

/// Clamp: (concurrency=99, 3 screens) → 6
#[test]
fn effective_concurrency_clamps_to_six() {
    assert_eq!(effective_concurrency(99, 3), 6);
}

// ── group_subtasks_by_screen ───────────────────────────────────────────────

/// Basic grouping: [login, home, login] → 2 groups {login:[0,2], home:[1]}
#[test]
fn group_subtasks_three_entries_two_screens() {
    let subtasks = vec![
        subtask_with_screen("a", Some("login")),
        subtask_with_screen("b", Some("home")),
        subtask_with_screen("c", Some("login")),
    ];
    let groups = group_subtasks_by_screen(&subtasks);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].screen, "login");
    assert_eq!(groups[0].indices, vec![0, 2]);
    assert_eq!(groups[1].screen, "home");
    assert_eq!(groups[1].indices, vec![1]);
}

/// A subtask with no screen falls back to first_screen.
#[test]
fn group_subtasks_no_screen_falls_back_to_first_screen() {
    let subtasks = vec![
        subtask_with_screen("a", Some("login")),
        subtask_with_screen("b", None), // no screen → "login"
        subtask_with_screen("c", Some("home")),
    ];
    let groups = group_subtasks_by_screen(&subtasks);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].screen, "login");
    assert_eq!(groups[0].indices, vec![0, 1]);
    assert_eq!(groups[1].screen, "home");
    assert_eq!(groups[1].indices, vec![2]);
}

/// All subtasks have no screen → empty result (no groups).
#[test]
fn group_subtasks_all_no_screen_returns_empty() {
    let subtasks = vec![
        subtask_with_screen("a", None),
        subtask_with_screen("b", None),
    ];
    let groups = group_subtasks_by_screen(&subtasks);
    assert!(groups.is_empty());
}

/// Empty subtask slice → empty result.
#[test]
fn group_subtasks_empty_slice_returns_empty() {
    let groups = group_subtasks_by_screen(&[]);
    assert!(groups.is_empty());
}

/// Group order is first-seen order of distinct screen values.
#[test]
fn group_subtasks_preserves_first_seen_order() {
    let subtasks = vec![
        subtask_with_screen("a", Some("profile")),
        subtask_with_screen("b", Some("settings")),
        subtask_with_screen("c", Some("profile")),
        subtask_with_screen("d", Some("dashboard")),
    ];
    let groups = group_subtasks_by_screen(&subtasks);
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].screen, "profile");
    assert_eq!(groups[1].screen, "settings");
    assert_eq!(groups[2].screen, "dashboard");
}

/// A None screen at the START falls back to first_screen from a LATER subtask.
#[test]
fn group_subtasks_no_screen_at_start_uses_later_first_screen() {
    let subtasks = vec![
        subtask_with_screen("a", None),         // no screen
        subtask_with_screen("b", Some("home")), // first with screen → first_screen="home"
        subtask_with_screen("c", Some("settings")),
    ];
    let groups = group_subtasks_by_screen(&subtasks);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].screen, "home");
    assert!(groups[0].indices.contains(&0));
    assert!(groups[0].indices.contains(&1));
    assert_eq!(groups[1].screen, "settings");
    assert_eq!(groups[1].indices, vec![2]);
}

// ── Task B1: BufferDocSink ────────────────────────────────────────────────

/// `BufferDocSink` collects commands without modifying a real doc.
#[test]
fn buffer_doc_sink_collects_commands() {
    let mut sink = BufferDocSink::new(EditorState::new());
    let applied = sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![],
        parent_id: op_editor_core::NodeId::NONE,
    });
    assert!(applied, "BufferDocSink.apply must always return true");
    assert_eq!(sink.commands.len(), 1);
}

/// `state()` on `BufferDocSink` returns the snapshot passed at construction.
#[test]
fn buffer_doc_sink_state_returns_snapshot() {
    let state = EditorState::new();
    let sink = BufferDocSink::new(state.clone());
    let _ = sink.state();
}

/// `BufferDocSink` tracks undo-batch depth correctly.
#[test]
fn buffer_doc_sink_undo_batch_depth() {
    let mut sink = BufferDocSink::new(EditorState::new());
    assert_eq!(sink.batch_depth, 0);
    sink.begin_undo_batch();
    assert_eq!(sink.batch_depth, 1);
    sink.end_undo_batch();
    assert_eq!(sink.batch_depth, 0);
}

// ── Task B1/B2: run_screen_group_worker ──────────────────────────────────

/// Happy path: both subtasks succeed on attempt 1.
#[test]
fn worker_happy_path_both_succeed() {
    let plan = make_plan_with_subtasks(&["s0", "s1"]);
    let req = make_req();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(NODE_JSON.into()), // s0 attempt 1
        ScriptResponse::Text(NODE_JSON.into()), // s1 attempt 1
    ]);
    let group = ScreenGroup {
        screen: "home".into(),
        indices: vec![0, 1],
    };
    let abort = AbortFlag::new();
    let buffer = BufferDocSink::new(EditorState::new());
    let (sem, tx, rx) = make_worker_channel();
    let result = block_on(run_screen_group_worker(
        &group, &plan, &req, &llm, &abort, buffer, sem, tx,
    ));
    let events = drain_events(rx);
    assert_eq!(result.outcomes.len(), 2);
    assert_eq!(result.outcomes[0].1.node_count, 1);
    assert_eq!(result.outcomes[1].1.node_count, 1);
    // Buffer collected 2 InsertSubtree commands (one per subtask).
    assert_eq!(result.buffer.commands.len(), 2);
    // Progress events: started + done for each subtask.
    assert_eq!(events.len(), 4);
}

/// Attempt-1 zero-node + retryable → attempt 2 runs with (true, true).
#[test]
fn worker_attempt1_zero_retryable_triggers_attempt2() {
    let plan = make_plan_with_subtasks(&["s0"]);
    let req = make_req();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Fail(LlmError {
            message: "timeout".into(),
            aborted: false,
        }),
        ScriptResponse::Text(NODE_JSON.into()),
    ]);
    let group = ScreenGroup {
        screen: "home".into(),
        indices: vec![0],
    };
    let abort = AbortFlag::new();
    let buffer = BufferDocSink::new(EditorState::new());
    let (sem, tx, _rx) = make_worker_channel();
    let result = block_on(run_screen_group_worker(
        &group, &plan, &req, &llm, &abort, buffer, sem, tx,
    ));
    assert_eq!(result.outcomes.len(), 1);
    assert_eq!(
        result.outcomes[0].1.node_count, 1,
        "attempt 2 should have produced a node"
    );
    assert_eq!(
        result.buffer.commands.len(),
        1,
        "one InsertSubtree from attempt 2"
    );
}

/// Non-retryable error on attempt 1 → NO attempt 2.
#[test]
fn worker_non_retryable_error_no_retry() {
    let plan = make_plan_with_subtasks(&["s0"]);
    let req = make_req();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
        message: "HTTP 401 Unauthorized".into(),
        aborted: false,
    })]);
    let group = ScreenGroup {
        screen: "home".into(),
        indices: vec![0],
    };
    let abort = AbortFlag::new();
    let buffer = BufferDocSink::new(EditorState::new());
    let (sem, tx, _rx) = make_worker_channel();
    let result = block_on(run_screen_group_worker(
        &group, &plan, &req, &llm, &abort, buffer, sem, tx,
    ));
    assert_eq!(result.outcomes.len(), 1);
    assert_eq!(
        result.outcomes[0].1.node_count, 0,
        "non-retryable must stay zero"
    );
    assert!(
        result.outcomes[0]
            .1
            .error
            .as_deref()
            .unwrap_or("")
            .contains("401"),
        "error should carry original 401 message"
    );
    assert_eq!(result.buffer.commands.len(), 0);
}

/// Partial result (node_count > 0) on attempt 1 → never retried.
#[test]
fn worker_partial_result_never_retried() {
    let plan = make_plan_with_subtasks(&["s0"]);
    let req = make_req();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_JSON.into())]);
    let group = ScreenGroup {
        screen: "home".into(),
        indices: vec![0],
    };
    let abort = AbortFlag::new();
    let buffer = BufferDocSink::new(EditorState::new());
    let (sem, tx, _rx) = make_worker_channel();
    let result = block_on(run_screen_group_worker(
        &group, &plan, &req, &llm, &abort, buffer, sem, tx,
    ));
    assert_eq!(result.outcomes.len(), 1);
    assert_eq!(result.outcomes[0].1.node_count, 1);
    assert_eq!(result.buffer.commands.len(), 1);
}

/// Abort flag set before the loop → worker returns immediately with no outcomes.
#[test]
fn worker_aborts_at_loop_head() {
    let plan = make_plan_with_subtasks(&["s0", "s1"]);
    let req = make_req();
    let llm = ScriptedLlm::new(vec![]);
    let group = ScreenGroup {
        screen: "home".into(),
        indices: vec![0, 1],
    };
    let abort = AbortFlag::new();
    abort.set();
    let buffer = BufferDocSink::new(EditorState::new());
    let (sem, tx, _rx) = make_worker_channel();
    let result = block_on(run_screen_group_worker(
        &group, &plan, &req, &llm, &abort, buffer, sem, tx,
    ));
    assert!(
        result.outcomes.is_empty(),
        "aborted before loop: no outcomes expected"
    );
    assert_eq!(result.buffer.commands.len(), 0);
}

/// Progress events: SubtaskStarted emitted before each subtask;
/// SubtaskDone emitted for successful subtasks.
#[test]
fn worker_emits_started_and_done_progress() {
    let plan = make_plan_with_subtasks(&["s0", "s1"]);
    let req = make_req();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(NODE_JSON.into()),
        ScriptResponse::Text(NODE_JSON.into()),
    ]);
    let group = ScreenGroup {
        screen: "home".into(),
        indices: vec![0, 1],
    };
    let abort = AbortFlag::new();
    let buffer = BufferDocSink::new(EditorState::new());
    let (sem, tx, rx) = make_worker_channel();
    block_on(run_screen_group_worker(
        &group, &plan, &req, &llm, &abort, buffer, sem, tx,
    ));
    let events: Vec<String> = drain_events(rx)
        .into_iter()
        .filter_map(|p| match &p {
            Progress::SubtaskStarted { id, .. } => Some(format!("start:{id}")),
            Progress::SubtaskDone { id, .. } => Some(format!("done:{id}")),
            Progress::SubtaskFailed { id, .. } => Some(format!("fail:{id}")),
            _ => None,
        })
        .collect();
    assert_eq!(events, vec!["start:s0", "done:s0", "start:s1", "done:s1"]);
}

/// SubtaskFailed progress emitted when final outcome is zero nodes.
#[test]
fn worker_emits_failed_progress_on_zero_nodes() {
    let plan = make_plan_with_subtasks(&["s0"]);
    let req = make_req();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text("not json".into()),
        ScriptResponse::Text("also not json".into()),
    ]);
    let group = ScreenGroup {
        screen: "home".into(),
        indices: vec![0],
    };
    let abort = AbortFlag::new();
    let buffer = BufferDocSink::new(EditorState::new());
    let (sem, tx, rx) = make_worker_channel();
    block_on(run_screen_group_worker(
        &group, &plan, &req, &llm, &abort, buffer, sem, tx,
    ));
    let fail_events: Vec<String> = drain_events(rx)
        .into_iter()
        .filter_map(|p| {
            if let Progress::SubtaskFailed { id, .. } = &p {
                Some(format!("fail:{id}"))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(fail_events, vec!["fail:s0"]);
}
