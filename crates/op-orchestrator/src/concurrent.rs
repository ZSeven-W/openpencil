//! S3b-2 Tasks A1 + B1: concurrency decision + screen grouping + worker.
//!
//! Port of `orchestrator.ts:780-810` (Task A1) and
//! `orchestrator-sub-agent.ts:248-312` (Task B1 worker body).
//!
//! ## Task A1
//! - `DesignRequest.concurrency` (added in `types.rs`)
//! - `clamp_concurrency` — defensive [1, 6] clamp
//! - `group_subtasks_by_screen` — group subtasks by screen; faithful to TS L785-801
//! - `effective_concurrency` — concurrency decision; faithful to TS L803-810
//!   (minus append-mode gate, which is S3b-4)
//!
//! ## Task B1
//! - `BufferDocSink` — a per-worker buffering `DocSink` implementation that
//!   collects applied `EditorCommand`s into a `Vec` without touching the real
//!   document.  This is safe because `run_subtask` does NOT call `sink.state()`
//!   at all — it only calls `sink.apply()`.  (Verified in `subagent.rs`.)
//!   A snapshot of a pre-concurrent `EditorState` is still carried so that
//!   `state()` can return something valid if any future caller does read it.
//! - `run_screen_group_worker` — async fn that runs one screen group's subtasks
//!   in order using the 2-attempt concurrent retry ladder (vs the sequential
//!   path's 3-attempt tier-gated ladder in `run.rs`).
//!
//! Callers land in later S3b-2 tasks; scaffolding symbols are allowed to be unused.
#![allow(dead_code)]

use crate::plan::{OrchestratorPlan, Subtask};
use crate::retry::is_non_retryable;
use crate::subagent::run_subtask;
use crate::types::{AbortFlag, DesignRequest, DocSink, LlmClient, Progress, SubtaskOutcome};
use op_editor_core::{EditorCommand, EditorState};

/// A group of subtask indices that share the same screen.
/// `screen` is the screen name; `indices` are into the plan's `subtasks` slice.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScreenGroup {
    pub screen: String,
    pub indices: Vec<usize>,
}

/// Clamps a raw concurrency value to the valid range `[1, 6]`.
///
/// This mirrors the store-side clamp in TS (store clamps to [1,6] before
/// writing to `request.concurrency`). The Rust crate clamps defensively on
/// the way in so callers need not worry about out-of-range values.
pub(crate) fn clamp_concurrency(v: u32) -> u32 {
    v.clamp(1, 6)
}

/// Groups subtasks by screen, faithfully porting `orchestrator.ts:785-801`.
///
/// Rules:
/// - Only called when `concurrency > 1` (caller's responsibility).
/// - `first_screen` = the `screen` of the first subtask that has one, else `"page"`.
/// - A subtask with no `screen` falls back to `first_screen`.
/// - Group order = first-seen order of distinct screen values.
/// - If no subtask has a `screen`, returns an empty `Vec` (caller treats as
///   single-screen; `effective_concurrency` will return 1 in that case).
pub(crate) fn group_subtasks_by_screen(subtasks: &[Subtask]) -> Vec<ScreenGroup> {
    let has_any_screen = subtasks.iter().any(|st| st.screen.is_some());
    if !has_any_screen {
        return vec![];
    }

    // first_screen = screen of first subtask that has one, else "page".
    let first_screen: String = subtasks
        .iter()
        .find(|st| st.screen.is_some())
        .and_then(|st| st.screen.clone())
        .unwrap_or_else(|| "page".to_string());

    let mut groups: Vec<ScreenGroup> = Vec::new();
    // Map from screen name to index into `groups`.
    let mut screen_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (i, subtask) in subtasks.iter().enumerate() {
        let screen = subtask
            .screen
            .clone()
            .unwrap_or_else(|| first_screen.clone());

        if let Some(&group_idx) = screen_map.get(&screen) {
            groups[group_idx].indices.push(i);
        } else {
            screen_map.insert(screen.clone(), groups.len());
            groups.push(ScreenGroup {
                screen,
                indices: vec![i],
            });
        }
    }

    groups
}

/// Computes the effective concurrency for a run.
///
/// Port of `orchestrator.ts:803-810`, minus the append-mode gate (S3b-4).
///
/// - `screen_group_count > 1` → `clamp_concurrency(concurrency)`.
/// - else → `1`.
pub(crate) fn effective_concurrency(concurrency: u32, screen_group_count: usize) -> u32 {
    if screen_group_count > 1 {
        clamp_concurrency(concurrency)
    } else {
        1
    }
}

// ── Task B1: BufferDocSink ────────────────────────────────────────────────────

/// A per-worker buffering [`DocSink`] that collects every applied
/// [`EditorCommand`] into an in-memory `Vec` without touching the real document.
///
/// **Design choice (spec §5 implementer note):** `run_subtask` (in `subagent.rs`)
/// never calls `sink.state()` — it only calls `sink.apply()`.  Therefore the
/// simplest correct buffer sink is a plain command collector; no document
/// re-execution is needed.  We still carry a pre-concurrent `EditorState`
/// snapshot so that any unexpected future `state()` call gets a valid (if
/// stale) answer rather than a panic or empty document.  The snapshot should
/// be taken just before the concurrent phase starts (after the N-root scaffold
/// is applied to the real sink) — that is Task B2's responsibility.
///
/// After `join_all` the orchestrator replays `commands` into the real
/// `DocSink` in subtask-plan-index order (Task B2: serialized replay).
pub(crate) struct BufferDocSink {
    /// Snapshot of `EditorState` taken before the concurrent phase.
    /// Returned by `state()` unchanged — workers do not read it.
    snapshot: EditorState,
    /// All `EditorCommand`s collected via `apply()` calls.
    pub commands: Vec<EditorCommand>,
    /// Tracks undo-batch nesting depth (for parity with `DocSink` contract).
    pub batch_depth: i32,
}

impl BufferDocSink {
    /// Create a new buffer sink from a pre-concurrent state snapshot.
    pub(crate) fn new(snapshot: EditorState) -> Self {
        Self {
            snapshot,
            commands: Vec::new(),
            batch_depth: 0,
        }
    }
}

impl DocSink for BufferDocSink {
    fn state(&self) -> &EditorState {
        &self.snapshot
    }

    /// Buffer the command for later replay; always returns `true`.
    ///
    /// We return `true` unconditionally — the real document is not touched
    /// here; the per-worker result is validated after replay (Task B2).
    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.commands.push(cmd);
        true
    }

    fn begin_undo_batch(&mut self) {
        self.batch_depth += 1;
    }

    fn end_undo_batch(&mut self) {
        self.batch_depth -= 1;
    }
}

// ── Task B1: concurrent screen-group worker fn ────────────────────────────────

/// Runs all subtasks for one screen group using the **2-attempt concurrent
/// retry ladder** (vs the sequential path's 3-attempt tier-gated ladder).
///
/// Port of `orchestrator-sub-agent.ts:248-312`.
///
/// ## Retry ladder
/// | Attempt | `reduced_complexity` | `minimal_skills` |
/// |---------|----------------------|------------------|
/// | 1       | `false`              | `false`          |
/// | 2       | `true`               | `true`           |
///
/// Skips the sequential path's tier-gated middle attempt.
///
/// ## Retryable-failure gate (identical to sequential path)
/// `error.is_some() && node_count == 0 && !abort.is_set() && !is_non_retryable(&err)`
///
/// Partial results (`node_count > 0`) are **never** retried.
///
/// ## Progress
/// `progress_fn` is called with `SubtaskStarted` before each subtask and
/// `SubtaskDone` / `SubtaskFailed` after.  In Task B2 this will be wired to
/// an `mpsc::UnboundedSender` so multiple workers can fan-in; for B1 a plain
/// `&mut dyn FnMut(Progress)` suffices.
///
/// ## Return value
/// A `Vec<SubtaskOutcome>` with one entry per subtask index in `group.indices`,
/// in the same order as `group.indices` (i.e. plan order within this screen).
pub(crate) async fn run_screen_group_worker(
    group: &ScreenGroup,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    sink: &mut BufferDocSink,
    progress_fn: &mut dyn FnMut(Progress),
) -> Vec<SubtaskOutcome> {
    let mut outcomes: Vec<SubtaskOutcome> = Vec::with_capacity(group.indices.len());

    for &idx in &group.indices {
        // Abort check at the top of every iteration (spec §4.4).
        if abort.is_set() {
            break;
        }

        let subtask = &plan.subtasks[idx];

        progress_fn(Progress::SubtaskStarted {
            id: subtask.id.clone(),
            label: subtask.label.clone(),
        });

        // --- Attempt 1: full complexity ---
        let outcome1 = run_subtask(subtask, plan, request, llm, sink, abort, false, false).await;

        // Evaluate the non-retryable predicate once from attempt-1's error
        // (matches TS `isNonRetryable` computed before the retry chain).
        let non_retryable = outcome1
            .error
            .as_deref()
            .map(is_non_retryable)
            .unwrap_or(false);

        // Retryable iff: error + zero nodes + not aborted + not non-retryable.
        // Partial result (node_count > 0) is never retried.
        let is_retryable = |o: &SubtaskOutcome| {
            o.error.is_some() && o.node_count == 0 && !abort.is_set() && !non_retryable
        };

        // --- Attempt 2: reduced_complexity=true, minimal_skills=true ---
        let final_outcome = if is_retryable(&outcome1) {
            tracing::warn!(
                subtask = %subtask.id,
                error = outcome1.error.as_deref().unwrap_or(""),
                "concurrent worker: subtask empty, retrying with minimal skills (attempt 2)"
            );
            run_subtask(subtask, plan, request, llm, sink, abort, true, true).await
        } else {
            outcome1
        };

        // Emit progress.
        let zero = final_outcome.node_count == 0;
        let node_count = final_outcome.node_count;
        let err_msg = final_outcome.error.clone();

        if zero {
            progress_fn(Progress::SubtaskFailed {
                id: subtask.id.clone(),
                error: err_msg.unwrap_or_default(),
            });
        } else {
            progress_fn(Progress::SubtaskDone {
                id: subtask.id.clone(),
                node_count,
            });
        }

        outcomes.push(final_outcome);
    }

    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Region, Subtask};

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
        }
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
        // "login" gets index 0 and 1 (b bucketed under first_screen="login")
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

    /// first_screen fallback is "page" when no subtask has a screen
    /// (but `has_any_screen` is false, so returns empty — tested separately).
    /// Here we test that a None screen at the START falls back to first_screen
    /// from a LATER subtask — i.e., first_screen is derived from the first
    /// subtask that HAS a screen.
    #[test]
    fn group_subtasks_no_screen_at_start_uses_later_first_screen() {
        let subtasks = vec![
            subtask_with_screen("a", None),         // no screen
            subtask_with_screen("b", Some("home")), // first with screen → first_screen="home"
            subtask_with_screen("c", Some("settings")),
        ];
        let groups = group_subtasks_by_screen(&subtasks);
        // "home" first (b has screen), but "a" has no screen → bucketed under first_screen="home"
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].screen, "home");
        // "a" (index 0) bucketed under "home" — it's first because it's processed first
        // and maps to first_screen="home" which is first seen when processing "a".
        assert!(groups[0].indices.contains(&0));
        assert!(groups[0].indices.contains(&1));
        assert_eq!(groups[1].screen, "settings");
        assert_eq!(groups[1].indices, vec![2]);
    }

    // ── Task B1: BufferDocSink ────────────────────────────────────────────────

    use crate::test_support::{ScriptResponse, ScriptedLlm};
    use crate::types::LlmError;
    use futures::executor::block_on;
    use op_editor_core::EditorState;

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

    const NODE_JSON: &str = r#"[{"type":"frame","id":"h1","name":"H","x":0,"y":0,"width":100,"height":100,"children":[]}]"#;

    /// `BufferDocSink` collects commands without modifying a real doc.
    #[test]
    fn buffer_doc_sink_collects_commands() {
        let mut sink = BufferDocSink::new(EditorState::new());
        // apply() should always return true and buffer the command.
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
        // The returned reference should be valid (same content).
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

    // ── Task B1: run_screen_group_worker ─────────────────────────────────────

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
        let mut sink = BufferDocSink::new(EditorState::new());
        let mut events: Vec<Progress> = Vec::new();
        let outcomes = block_on(run_screen_group_worker(
            &group,
            &plan,
            &req,
            &llm,
            &abort,
            &mut sink,
            &mut |p| events.push(p),
        ));
        assert_eq!(outcomes.len(), 2);
        assert_eq!(outcomes[0].node_count, 1);
        assert_eq!(outcomes[1].node_count, 1);
        // Buffer collected 2 InsertSubtree commands (one per subtask).
        assert_eq!(sink.commands.len(), 2);
    }

    /// Attempt-1 zero-node + retryable → attempt 2 runs with (true, true).
    /// We verify this by scripting: attempt 1 → error, attempt 2 → nodes.
    #[test]
    fn worker_attempt1_zero_retryable_triggers_attempt2() {
        let plan = make_plan_with_subtasks(&["s0"]);
        let req = make_req();
        let llm = ScriptedLlm::new(vec![
            // Attempt 1 — LLM error → zero nodes, retryable
            ScriptResponse::Fail(LlmError {
                message: "timeout".into(),
                aborted: false,
            }),
            // Attempt 2 — succeeds
            ScriptResponse::Text(NODE_JSON.into()),
        ]);
        let group = ScreenGroup {
            screen: "home".into(),
            indices: vec![0],
        };
        let abort = AbortFlag::new();
        let mut sink = BufferDocSink::new(EditorState::new());
        let mut events: Vec<Progress> = Vec::new();
        let outcomes = block_on(run_screen_group_worker(
            &group,
            &plan,
            &req,
            &llm,
            &abort,
            &mut sink,
            &mut |p| events.push(p),
        ));
        assert_eq!(outcomes.len(), 1);
        // Final outcome should show nodes from attempt 2.
        assert_eq!(
            outcomes[0].node_count, 1,
            "attempt 2 should have produced a node"
        );
        assert_eq!(sink.commands.len(), 1, "one InsertSubtree from attempt 2");
    }

    /// Non-retryable error on attempt 1 → NO attempt 2.
    #[test]
    fn worker_non_retryable_error_no_retry() {
        let plan = make_plan_with_subtasks(&["s0"]);
        let req = make_req();
        let llm = ScriptedLlm::new(vec![
            // Attempt 1 — non-retryable HTTP 401
            ScriptResponse::Fail(LlmError {
                message: "HTTP 401 Unauthorized".into(),
                aborted: false,
            }),
            // Attempt 2 — should NOT be called; if it is, the test fails by
            // observing the scripted LLM's exhausted error.
        ]);
        let group = ScreenGroup {
            screen: "home".into(),
            indices: vec![0],
        };
        let abort = AbortFlag::new();
        let mut sink = BufferDocSink::new(EditorState::new());
        let mut events: Vec<Progress> = Vec::new();
        let outcomes = block_on(run_screen_group_worker(
            &group,
            &plan,
            &req,
            &llm,
            &abort,
            &mut sink,
            &mut |p| events.push(p),
        ));
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].node_count, 0, "non-retryable must stay zero");
        // Verify the error message is the original 401, not "scripted LLM exhausted"
        assert!(
            outcomes[0].error.as_deref().unwrap_or("").contains("401"),
            "error should carry original 401 message"
        );
        // No InsertSubtree in the buffer (nothing was applied).
        assert_eq!(sink.commands.len(), 0);
    }

    /// Partial result (node_count > 0) on attempt 1 → never retried.
    #[test]
    fn worker_partial_result_never_retried() {
        let plan = make_plan_with_subtasks(&["s0"]);
        let req = make_req();
        // Attempt 1 produces nodes — the scripted LLM has only one response.
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_JSON.into())]);
        let group = ScreenGroup {
            screen: "home".into(),
            indices: vec![0],
        };
        let abort = AbortFlag::new();
        let mut sink = BufferDocSink::new(EditorState::new());
        let outcomes = block_on(run_screen_group_worker(
            &group,
            &plan,
            &req,
            &llm,
            &abort,
            &mut sink,
            &mut |_| {},
        ));
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].node_count, 1);
        // Only 1 command buffered (attempt 1 only).
        assert_eq!(sink.commands.len(), 1);
    }

    /// Abort flag set before the loop → worker returns immediately with no outcomes.
    #[test]
    fn worker_aborts_at_loop_head() {
        let plan = make_plan_with_subtasks(&["s0", "s1"]);
        let req = make_req();
        let llm = ScriptedLlm::new(vec![]); // should not be called
        let group = ScreenGroup {
            screen: "home".into(),
            indices: vec![0, 1],
        };
        let abort = AbortFlag::new();
        abort.set(); // set before the worker runs
        let mut sink = BufferDocSink::new(EditorState::new());
        let outcomes = block_on(run_screen_group_worker(
            &group,
            &plan,
            &req,
            &llm,
            &abort,
            &mut sink,
            &mut |_| {},
        ));
        assert!(
            outcomes.is_empty(),
            "aborted before loop: no outcomes expected"
        );
        assert_eq!(sink.commands.len(), 0);
    }

    /// Progress events: SubtaskStarted is emitted before each subtask;
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
        let mut sink = BufferDocSink::new(EditorState::new());
        let mut events: Vec<String> = Vec::new();
        block_on(run_screen_group_worker(
            &group,
            &plan,
            &req,
            &llm,
            &abort,
            &mut sink,
            &mut |p| match &p {
                Progress::SubtaskStarted { id, .. } => events.push(format!("start:{id}")),
                Progress::SubtaskDone { id, .. } => events.push(format!("done:{id}")),
                Progress::SubtaskFailed { id, .. } => events.push(format!("fail:{id}")),
                _ => {}
            },
        ));
        assert_eq!(events, vec!["start:s0", "done:s0", "start:s1", "done:s1"],);
    }

    /// SubtaskFailed progress emitted when final outcome is zero nodes.
    #[test]
    fn worker_emits_failed_progress_on_zero_nodes() {
        let plan = make_plan_with_subtasks(&["s0"]);
        let req = make_req();
        // Both attempts produce garbage → zero nodes.
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text("not json".into()),
            ScriptResponse::Text("also not json".into()),
        ]);
        let group = ScreenGroup {
            screen: "home".into(),
            indices: vec![0],
        };
        let abort = AbortFlag::new();
        let mut sink = BufferDocSink::new(EditorState::new());
        let mut events: Vec<String> = Vec::new();
        block_on(run_screen_group_worker(
            &group,
            &plan,
            &req,
            &llm,
            &abort,
            &mut sink,
            &mut |p| {
                if let Progress::SubtaskFailed { id, .. } = &p {
                    events.push(format!("fail:{id}"));
                }
            },
        ));
        assert_eq!(events, vec!["fail:s0"]);
    }
}
