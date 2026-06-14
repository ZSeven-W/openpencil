//! S3b-2 Tasks A1 + B1 + B2: concurrency decision + screen grouping + worker
//! + concurrent executor.
//!
//! Port of `orchestrator.ts:780-810` (Task A1),
//! `orchestrator-sub-agent.ts:248-312` (Task B1 worker body), and
//! `orchestrator-sub-agent.ts:227-327` (Task B2 concurrent executor).
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
//!   document. `run_subtask` reads `sink.state()` to bind generated colours to
//!   the seeded design variables, so each buffer carries a pre-concurrent
//!   `EditorState` snapshot taken after variable seed + scaffold apply.
//! - `run_screen_group_worker` — async fn that runs one screen group's subtasks
//!   in order using the 2-attempt concurrent retry ladder (vs the sequential
//!   path's 3-attempt tier-gated ladder in `run.rs`).
//!
//! ## Task B2
//! - `run_concurrent` — the concurrent executor:
//!   - a shared `Arc<tokio::sync::Semaphore>` (permits = `effective_concurrency`)
//!     caps in-flight subtask LLM calls; RAII permit drop replaces TS `releaseSlot`
//!   - N worker futures (one per screen group, each owning its own `BufferDocSink`)
//!     driven together by `futures::future::join_all` on a SINGLE task (no
//!     `tokio::spawn` — avoids `Send` bounds; LLM I/O is offloaded in
//!     `LlmClient::call`).  `join_all` polls every worker on each wake, so
//!     different groups' LLM calls genuinely overlap — the concurrent equivalent
//!     of JS `Promise.all(workers)`.
//!   - progress fan-in: workers send `Progress` on an mpsc channel; driver drains
//!     + forwards to `on_progress` after `join_all`
//!   - serialized replay: after `join_all`, replays every worker's buffered
//!     commands into the real `&mut DocSink` in subtask-plan-index order
//!     (deterministic)
//!
use crate::plan::{OrchestratorPlan, Subtask};
use crate::retry::is_non_retryable;
use crate::subagent::{apply_command_with_reveal, reveal_now_millis, run_subtask};
use crate::types::{AbortFlag, DesignRequest, DocSink, LlmClient, Progress, SubtaskOutcome};
use futures::future::join_all;
use op_editor_core::{EditorCommand, EditorState};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

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
/// **Design choice (spec §5 implementer note):** the buffer is still a plain
/// command collector; it does not re-execute worker commands locally. The
/// pre-concurrent snapshot is read-only context for generation post-processing
/// such as design-variable colour binding. The snapshot should be taken just
/// before the concurrent phase starts (after variable seed + N-root scaffold
/// apply) — that is Task B2's responsibility.
///
/// After all workers finish the orchestrator replays `commands` into the real
/// `DocSink` in subtask-plan-index order (Task B2: serialized replay).
pub(crate) struct BufferDocSink {
    /// Snapshot of `EditorState` taken before the concurrent phase.
    /// Returned by `state()` unchanged for read-only generation context.
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

// ── Task B1/B2: concurrent screen-group worker fn ─────────────────────────────

/// The output of one screen-group worker: its private command buffer plus the
/// per-subtask outcomes (each tagged with its plan index for serialized replay).
pub(crate) struct WorkerResult {
    /// The worker's private buffer — replayed into the real sink after `join_all`.
    pub buffer: BufferDocSink,
    /// `(plan_index, outcome)` for every subtask the worker ran, in `indices` order.
    pub outcomes: Vec<(usize, SubtaskOutcome)>,
}

/// Runs all subtasks for one screen group using the **2-attempt concurrent
/// retry ladder** (vs the sequential path's 3-attempt tier-gated ladder).
///
/// Port of `orchestrator-sub-agent.ts:248-312`.
///
/// ## Owned buffer (genuine concurrency)
/// The worker **owns** its [`BufferDocSink`] by value (passed in, returned in
/// [`WorkerResult`]).  `run_subtask` writes into this stack-local buffer via
/// `&mut` — the `&mut` is to the worker's *own* buffer, never a shared `Vec`
/// slot.  This is what lets [`run_concurrent`] drive N worker futures with
/// `futures::future::join_all` without the borrow checker rejecting aliased
/// `&mut` borrows: each future owns its buffer, so there is no shared mutable
/// state.
///
/// ## Retry ladder
/// | Attempt | `reduced_complexity` | `minimal_skills` |
/// |---------|----------------------|------------------|
/// | 1       | `false`              | `false`          |
/// | 2       | `true`               | `true`           |
///
/// Skips the sequential path's tier-gated middle attempt.
///
/// ## Semaphore
/// Each subtask acquires one permit from `semaphore` before the LLM call.
/// RAII permit drop (when the block completes) replaces TS `releaseSlot`.
///
/// ## Retryable-failure gate (identical to sequential path)
/// `error.is_some() && node_count == 0 && !abort.is_set() && !is_non_retryable(&err)`
///
/// Partial results (`node_count > 0`) are **never** retried.
///
/// ## Progress
/// Progress events are sent via `progress_tx` (mpsc channel).
/// The `run_concurrent` driver drains the receiver and forwards to `on_progress`.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_screen_group_worker(
    group: &ScreenGroup,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    mut buffer: BufferDocSink,
    semaphore: Arc<Semaphore>,
    progress_tx: mpsc::UnboundedSender<Progress>,
) -> WorkerResult {
    let mut outcomes: Vec<(usize, SubtaskOutcome)> = Vec::with_capacity(group.indices.len());

    for &idx in &group.indices {
        // Abort check at the top of every iteration (spec §4.4).
        if abort.is_set() {
            break;
        }

        let subtask = &plan.subtasks[idx];

        let _ = progress_tx.send(Progress::SubtaskStarted {
            id: subtask.id.clone(),
            label: subtask.label.clone(),
        });

        // Acquire a semaphore permit before the LLM call.
        // RAII: permit drops when the block completes (= releaseSlot in TS).
        let _permit = semaphore
            .acquire()
            .await
            .expect("semaphore should not be closed");

        // --- Attempt 1: full complexity ---
        let outcome1 = run_subtask(
            subtask,
            plan,
            request,
            llm,
            &mut buffer,
            abort,
            false,
            false,
        )
        .await;

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
            run_subtask(subtask, plan, request, llm, &mut buffer, abort, true, true).await
        } else {
            outcome1
        };

        // Emit progress.
        let zero = final_outcome.node_count == 0;
        let node_count = final_outcome.node_count;
        let err_msg = final_outcome.error.clone();

        if zero {
            let _ = progress_tx.send(Progress::SubtaskFailed {
                id: subtask.id.clone(),
                error: err_msg.unwrap_or_default(),
            });
        } else {
            let _ = progress_tx.send(Progress::SubtaskDone {
                id: subtask.id.clone(),
                node_count,
            });
        }

        outcomes.push((idx, final_outcome));
    }

    WorkerResult { buffer, outcomes }
}

// ── Task B2: run_concurrent ────────────────────────────────────────────────────

/// Concurrent executor — semaphore-gated screen-group workers + serialized replay.
///
/// Port of `orchestrator-sub-agent.ts:227-327` (semaphore + workers + collection).
///
/// ## Genuine concurrency via `join_all`
/// One **worker future per screen group** is built; all N are driven together
/// with [`futures::future::join_all`] on the SINGLE current task (no
/// `tokio::spawn` — avoids `Send` bounds; the LLM I/O is already offloaded
/// inside `LlmClient::call`).  `join_all` polls every worker on each wake: when
/// worker A awaits an LLM stream chunk, `join_all` polls worker B, which
/// proceeds with *its* LLM call — the calls genuinely overlap in wall-clock
/// time.  This is the concurrent equivalent of JS `Promise.all(workers)`.
///
/// The borrow checker is satisfied because **each worker future owns its own
/// `BufferDocSink`** (constructed here, moved in, returned in [`WorkerResult`])
/// — there is no shared `&mut`.  The genuinely-shared values are all shared
/// `&` (`&dyn LlmClient`, `&AbortFlag`, `&plan`, `&request`, `Arc<Semaphore>`);
/// the `mpsc` sender is cloned per worker.
///
/// ## Semaphore
/// A shared `Arc<Semaphore>` (permits = `effective_concurrency(concurrency,
/// groups.len())`) caps the total in-flight subtask LLM calls across all
/// genuinely-overlapping workers.
///
/// ## Progress fan-in
/// Workers send `Progress` events on cloned `mpsc::UnboundedSender<Progress>`s;
/// after `join_all` the driver drains the receiver and forwards to
/// `on_progress`.  The `on_progress` consumer must tolerate interleaved
/// `SubtaskStarted` events (multiple started before corresponding done).
///
/// ## Serialized replay (spec §5)
/// `join_all` yields `Vec<WorkerResult>`; the driver then replays every
/// worker's buffered `EditorCommand`s into the real `&mut DocSink` in
/// ascending plan-index order — deterministic, matches the TS indexed-results
/// array, never torn.
///
/// Sort-by-min-index: within each group subtasks already ran in ascending plan
/// index order, so the buffer contains commands in ascending plan-index order.
/// Cross-group: sorting groups by min(indices) ensures the group with the
/// lower-numbered subtask replays first.
///
/// ## Return value
/// `Vec<Option<SubtaskOutcome>>` indexed by plan order (0..plan.subtasks.len()).
/// A slot is `None` if the worker was aborted before reaching that subtask.
/// The all-zero-nodes failure verdict (Task C1) is NOT checked here.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_concurrent(
    groups: &[ScreenGroup],
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    snapshot: EditorState,
    real_sink: &mut dyn DocSink,
    on_progress: &mut dyn FnMut(Progress),
    host_epoch: Option<u64>,
) -> Vec<Option<SubtaskOutcome>> {
    let n = plan.subtasks.len();
    let ec = effective_concurrency(request.concurrency, groups.len());
    let semaphore = Arc::new(Semaphore::new(ec as usize));

    // Progress channel: workers send; we drain after `join_all`.
    // The master sender is dropped after building the per-worker clones so the
    // channel closes once every worker (and thus every clone) has finished.
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<Progress>();

    // Build one worker future per screen group.  Each future OWNS its own
    // `BufferDocSink` (moved in) — no shared `&mut`, so `join_all` is legal.
    let worker_futures: Vec<_> = groups
        .iter()
        .map(|group| {
            let buffer = BufferDocSink::new(snapshot.clone());
            let sem = Arc::clone(&semaphore);
            let ptx = progress_tx.clone();
            run_screen_group_worker(group, plan, request, llm, abort, buffer, sem, ptx)
        })
        .collect();

    // Drop the master sender so the channel closes when all worker clones drop.
    drop(progress_tx);

    // Drive ALL workers concurrently on the current task.  `join_all` polls
    // each worker on every wake — LLM calls from different groups genuinely
    // overlap.  Returns one `WorkerResult` per group, in `groups` order.
    let worker_results: Vec<WorkerResult> = join_all(worker_futures).await;

    // Drain the progress channel and forward to `on_progress`.
    while let Ok(event) = progress_rx.try_recv() {
        on_progress(event);
    }

    // Collect per-subtask outcomes indexed by plan order.
    let mut per_subtask: Vec<Option<SubtaskOutcome>> = vec![None; n];
    for result in &worker_results {
        for (plan_idx, outcome) in &result.outcomes {
            per_subtask[*plan_idx] = Some(outcome.clone());
        }
    }

    // Serialized replay in plan-index order.
    // `worker_results` is in `groups` order; sort it by each group's minimum
    // plan index so the group containing the lowest-indexed subtask replays
    // first.  Within a group subtasks already ran in ascending index order.
    let mut replay_order: Vec<usize> = (0..groups.len()).collect();
    replay_order.sort_by_key(|&g| {
        groups[g]
            .indices
            .iter()
            .copied()
            .min()
            .unwrap_or(usize::MAX)
    });
    let mut worker_results = worker_results;
    for g_idx in replay_order {
        for cmd in worker_results[g_idx].buffer.commands.drain(..) {
            apply_command_with_reveal(real_sink, cmd, host_epoch, reveal_now_millis());
        }
    }

    per_subtask
}

// Tests are split across two sibling files to honor the 800-line cap:
// `concurrent_tests.rs`    — Task A1 + B1 (grouping / decision / worker / sink)
// `concurrent_tests_b2.rs` — Task B2 (`run_concurrent` concurrent executor)
#[cfg(test)]
#[path = "concurrent_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "concurrent_tests_b2.rs"]
mod tests_b2;
