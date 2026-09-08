//! Concurrency primitives shared by the agent-team fan-out (`spawn_concurrent`)
//! and — since the multiscreen-fanout-break fix's item D-lite (2026-07-17) —
//! by the classic Orchestrator's INTER-screen-group executor.
//!
//! ## History
//! The orchestrator's former multi-screen concurrent path (screen grouping +
//! N-root scaffold + semaphore/`join_all` worker fan-in) was collapsed into
//! the sequential path by `aca0d3a0` (2026-07-02): its M3 quality gate only
//! ever measured concurrency WITHIN one screen (parallel sections of the
//! SAME root) against sequential — found byte-identical output, only
//! differing in wall-clock time, so the executor was deleted as unpaid-for
//! complexity. Item A (2026-07-17) revived the PURE partitioning half
//! (`screen_groups::group_subtasks_by_screen` + N-root scaffold,
//! `run_screen_groups.rs`) but kept every group's subtasks running
//! sequentially — the structural bug (`plan_normalize` folding every subtask
//! onto one root) was the actual break; concurrency was orthogonal to it.
//!
//! Item D-lite (this module's new half) revives GENUINE concurrency, but
//! scoped ONLY to DISTINCT screen groups — never same-screen section
//! parallelism, which is exactly what `aca0d3a0`'s data verdict evaluated
//! and found not worth the complexity. That verdict does not transfer here:
//! N independent screens (N independent root subtrees, N independent
//! design-agent turns with no shared mutable state) is a different question
//! from "does splitting ONE screen's sections across workers help" — the
//! former is the ⚡Nx "team size" UI setting's entire premise (`agent_team_size`
//! → `DesignRequest.concurrency`), which had been dead for the classic path
//! since the retirement; this module makes it live again.
//!
//! What's here:
//! - [`clamp_concurrency`] — the defensive `[1, 6]` permit cap.
//! - [`effective_concurrency`] — `min(clamp(request.concurrency), groups.len())`,
//!   forced to `1` when there's at most one group (no parallelism possible or
//!   meaningful) — `run.rs` takes the untouched sequential path whenever this
//!   returns `1`, so single-screen / single-group plans are a byte-identical
//!   regression lock.
//! - [`BufferDocSink`] — isolated command buffering with a local staging state;
//!   successful commands are replayed atomically into the real sink.
//! - [`run_subtask_retry_ladder`] — the 3-attempt tier-gated retry ladder,
//!   extracted from `run.rs`'s sequential loop so BOTH the sequential path
//!   and each screen-group worker share the IDENTICAL retry semantics —
//!   parallelizing groups must never let a group's retry behavior drift from
//!   what the M3 quality gate was measured against.
//! - [`run_screen_groups_concurrent`] — the executor: one worker future per
//!   screen group, driven together on a [`futures::stream::FuturesUnordered`]
//!   (no `tokio::spawn`, no `Send` bounds beyond what `LlmClient`/`DocSink`
//!   already require), gated by a shared `tokio::sync::Semaphore` sized to
//!   `effective_concurrency`. A `tokio::select!` loop polls the progress
//!   channel and the worker set TOGETHER (visibility fix, 2026-07-17): each
//!   `Progress` event reaches the caller the moment it's sent, and each
//!   successful subtask's buffered commands replay atomically into the REAL
//!   sink as soon as that subtask finishes. Replay follows subtask completion
//!   order, can interleave across disjoint screen roots, and still has exactly
//!   one real-document writer.

use crate::agent_identity::AgentIdentity;
use crate::model_profile::ModelTier;
use crate::plan::{OrchestratorPlan, Subtask};
use crate::retry::{is_non_retryable, is_self_check_rejection};
use crate::screen_groups::ScreenGroup;
use crate::subagent::{reveal_now_millis, run_subtask_with_reveal_at_and_outcomes};
use crate::types::{
    AbortFlag, DesignRequest, DocSink, GeometryEchoBudget, LlmClient, Progress, SubtaskOutcome,
};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use op_editor_core::{EditorCommand, EditorState};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

#[path = "concurrent_replay.rs"]
mod replay;
use replay::{apply_worker_event, run_screen_group_worker, WorkerSignal};

/// Clamps a raw concurrency value to the valid range `[1, 6]`.
///
/// This mirrors the store-side clamp in TS (store clamps to [1,6] before
/// writing to `request.concurrency`). The Rust crate clamps defensively on
/// the way in so callers need not worry about out-of-range values.
pub(crate) fn clamp_concurrency(v: u32) -> u32 {
    v.clamp(1, 6)
}

/// The effective worker count for the screen-group executor:
/// `min(clamp_concurrency(concurrency), group_count)`, or `1` when there is
/// at most one group — a single group has nothing to run alongside, so
/// `run.rs` takes the untouched sequential path (byte-identical regression
/// lock for single-screen plans, append mode, and any plan whose subtasks
/// all share one `screen` tag).
pub(crate) fn effective_concurrency(concurrency: u32, group_count: usize) -> u32 {
    if group_count > 1 {
        clamp_concurrency(concurrency).min(group_count as u32)
    } else {
        1
    }
}

// ── BufferDocSink ─────────────────────────────────────────────────────────────

/// An isolated command buffer with a staging snapshot for structural gates.
/// Failed attempts reset it; successful commands are replayed atomically.
pub(crate) struct BufferDocSink {
    /// Group-local staging state taken before this buffered subtask.
    /// It mirrors buffered commands so local structural gates can inspect them.
    snapshot: EditorState,
    initial_snapshot: EditorState,
    /// All `EditorCommand`s collected via `apply()` calls.
    pub commands: Vec<EditorCommand>,
    /// Tracks undo-batch nesting depth (for parity with `DocSink` contract).
    pub batch_depth: i32,
}

impl BufferDocSink {
    /// Create an isolated buffer sink from one group-local snapshot.
    pub(crate) fn new(snapshot: EditorState) -> Self {
        Self {
            snapshot: snapshot.clone(),
            initial_snapshot: snapshot,
            commands: Vec::new(),
            batch_depth: 0,
        }
    }
}

impl DocSink for BufferDocSink {
    fn state(&self) -> &EditorState {
        &self.snapshot
    }

    /// Buffer the command and mirror it into staging; real replay is later.
    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.commands.push(cmd.clone());
        let _ = self.snapshot.apply(cmd);
        true
    }

    fn begin_undo_batch(&mut self) {
        self.batch_depth += 1;
    }

    fn end_undo_batch(&mut self) {
        self.batch_depth -= 1;
    }

    fn rollback_inserted_roots(&mut self, _root_ids: &[String]) {
        self.commands.clear();
        self.snapshot = self.initial_snapshot.clone();
    }

    fn is_buffered(&self) -> bool {
        true
    }
}

// ── Shared retry ladder ─────────────────────────────────────────────────────

/// Run the 3-attempt tier-gated retry ladder for ONE subtask — attempt 1 full
/// complexity, attempt 2 `reduced_complexity` iff `tier == ModelTier::Basic`,
/// attempt 3 `reduced_complexity + minimal_skills` — emitting `SubtaskStarted`
/// / `SubtaskRetry` progress events via `on_progress`, exactly as `run.rs`'s
/// sequential loop always has.
///
/// Extracted so [`run_screen_group_worker`] and the sequential path in
/// `run.rs` share the IDENTICAL retry semantics: parallelizing groups must
/// never let a group's retry behavior drift from the ladder the M3 quality
/// gate was measured against.
///
/// Does not decide `SubtaskDone` / `SubtaskFailed` emission or salvage
/// eligibility — the caller does that from the returned [`SubtaskOutcome`],
/// since that bookkeeping differs slightly between the sequential loop's
/// flat plan-index accounting and a worker's group-relative accounting.
///
/// `agent_indicator_epoch` should be `None` when `sink` is a [`BufferDocSink`]
/// (buffered inserts have not actually landed in the document yet — wiring
/// the canvas reveal/indicator overlay to them here would race the replay
/// that makes them real) and `Some(epoch)` for the sequential path, which
/// writes directly to the real sink.
///
/// After the ladder settles on a winning outcome, one more thing can
/// happen before it's returned: the `geometry_echo` step
/// ([`maybe_geometry_echo`]) — see that function's doc for the full
/// contract (budget, buffered-sink no-op, replace-on-success semantics).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_subtask_retry_ladder(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    tier: ModelTier,
    agent_indicator_epoch: Option<u64>,
    geometry_echo_budget: &GeometryEchoBudget,
    on_progress: &mut dyn FnMut(Progress),
) -> SubtaskOutcome {
    run_subtask_retry_ladder_with_outcomes(
        subtask,
        plan,
        request,
        llm,
        sink,
        abort,
        tier,
        agent_indicator_epoch,
        geometry_echo_budget,
        on_progress,
        &[],
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_subtask_retry_ladder_with_outcomes(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    tier: ModelTier,
    agent_indicator_epoch: Option<u64>,
    geometry_echo_budget: &GeometryEchoBudget,
    on_progress: &mut dyn FnMut(Progress),
    prior_outcomes: &[SubtaskOutcome],
) -> SubtaskOutcome {
    on_progress(Progress::SubtaskStarted {
        id: subtask.id.clone(),
        label: subtask.label.clone(),
    });

    // Attempt 1 — full complexity. Forwards the progress sink so the
    // per-subtask SkillLoadReport reaches the chat UI.
    let outcome1 = run_subtask_with_reveal_at_and_outcomes(
        subtask,
        plan,
        request,
        llm,
        sink,
        abort,
        false,
        false,
        agent_indicator_epoch,
        reveal_now_millis(),
        Some(&mut *on_progress),
        prior_outcomes,
    )
    .await;
    let completeness1 = if abort.is_set() {
        None
    } else {
        crate::subtask_completeness::reject_incomplete_attempt(sink, subtask, &outcome1)
    };

    // Evaluate the non-retryable predicate once from attempt-1's error
    // (faithful to the sequential path: computed before the retry chain and
    // reused for both the attempt-2 and attempt-3 guards).
    let non_retryable = outcome1
        .error
        .as_deref()
        .map(is_non_retryable)
        .unwrap_or(false);

    let retryable = |o: &SubtaskOutcome, incomplete: bool| {
        (incomplete || (o.error.is_some() && o.node_count == 0 && !non_retryable))
            && !abort.is_set()
    };

    // A self-check quality rejection (`orchestration_self_check` fatally
    // rejected otherwise-real, otherwise-parsed content) is not evidence the
    // model needs a narrower skill set — the content was fine except for the
    // one flagged issue, so throwing skills away on attempt 2 only makes the
    // REST of the design worse while doing nothing to fix that issue. Skill
    // downgrade stays reserved for attempt 1 failures that actually suggest
    // the model is struggling with the full prompt (stream errors, parse
    // failures, blank output). Instead, attempt 2 retries at the SAME
    // complexity/skill tier with the rejection reason echoed into the
    // prompt (`prompt.rs`'s `retry_feedback` block) so the model can fix
    // exactly that issue.
    let attempt1_self_check_rejection = outcome1
        .error
        .as_deref()
        .is_some_and(is_self_check_rejection);
    let attempt2_subtask = if let Some(failure) = completeness1.as_ref() {
        Subtask {
            retry_feedback: Some(crate::plan::RetryFeedback::Completeness(
                failure.feedback.clone(),
            )),
            ..subtask.clone()
        }
    } else if attempt1_self_check_rejection {
        Subtask {
            retry_feedback: outcome1
                .error
                .clone()
                .map(crate::plan::RetryFeedback::SelfCheck),
            ..subtask.clone()
        }
    } else {
        subtask.clone()
    };

    // Attempt 2 — reduced_complexity iff Basic tier AND attempt 1 wasn't a
    // self-check quality rejection.
    let outcome2 = if retryable(&outcome1, completeness1.is_some()) {
        tracing::warn!(
            subtask = %subtask.id,
            error = outcome1.error.as_deref().unwrap_or(""),
            "subtask failed, retrying (attempt 2)"
        );
        on_progress(Progress::SubtaskRetry {
            id: subtask.id.clone(),
            attempt: 2,
            reason: completeness1
                .as_ref()
                .map(|failure| failure.feedback.clone())
                .or_else(|| outcome1.error.clone())
                .unwrap_or_else(|| "zero nodes generated".into()),
        });
        Some(
            run_subtask_with_reveal_at_and_outcomes(
                &attempt2_subtask,
                plan,
                request,
                llm,
                sink,
                abort,
                tier == ModelTier::Basic && !attempt1_self_check_rejection,
                false,
                agent_indicator_epoch,
                reveal_now_millis(),
                None,
                prior_outcomes,
            )
            .await,
        )
    } else {
        None
    };
    let completeness2 = if abort.is_set() {
        None
    } else {
        outcome2.as_ref().and_then(|outcome| {
            crate::subtask_completeness::reject_incomplete_attempt(sink, subtask, outcome)
        })
    };

    let outcome_after2 = outcome2.as_ref().unwrap_or(&outcome1);
    let attempt3_subtask = if let Some(failure) = completeness2.as_ref() {
        Subtask {
            retry_feedback: Some(crate::plan::RetryFeedback::Completeness(
                failure.feedback.clone(),
            )),
            ..subtask.clone()
        }
    } else if outcome_after2
        .error
        .as_deref()
        .is_some_and(is_self_check_rejection)
    {
        Subtask {
            retry_feedback: outcome_after2
                .error
                .clone()
                .map(crate::plan::RetryFeedback::SelfCheck),
            ..subtask.clone()
        }
    } else if attempt2_subtask.retry_feedback.is_some() {
        // A transient attempt-2 parse/transport failure must not erase the
        // actionable self-check feedback learned on attempt 1.
        attempt2_subtask.clone()
    } else {
        subtask.clone()
    };

    // Attempt 3 — minimal skills (last-ditch fallback).
    let outcome3 = if retryable(outcome_after2, completeness2.is_some()) {
        tracing::warn!(
            subtask = %subtask.id,
            error = outcome_after2.error.as_deref().unwrap_or(""),
            "subtask still empty after retry, falling back to minimal skills (attempt 3)"
        );
        on_progress(Progress::SubtaskRetry {
            id: subtask.id.clone(),
            attempt: 3,
            reason: completeness2
                .as_ref()
                .map(|failure| failure.feedback.clone())
                .or_else(|| outcome_after2.error.clone())
                .unwrap_or_else(|| "zero nodes generated".into()),
        });
        Some(
            run_subtask_with_reveal_at_and_outcomes(
                &attempt3_subtask,
                plan,
                request,
                llm,
                sink,
                abort,
                true,
                true,
                agent_indicator_epoch,
                reveal_now_millis(),
                None,
                prior_outcomes,
            )
            .await,
        )
    } else {
        None
    };
    let completeness3 = if abort.is_set() {
        None
    } else {
        outcome3.as_ref().and_then(|outcome| {
            crate::subtask_completeness::incomplete_attempt(sink, subtask, outcome)
        })
    };

    // Whichever attempt actually won, carry along the (reduced_complexity,
    // minimal_skills) it used — the geometry_echo retry below reuses the
    // SAME tier, never escalating or de-escalating.
    let (mut outcome, reduced_complexity, minimal_skills, final_completeness) =
        if let Some(o3) = outcome3 {
            (o3, true, true, completeness3)
        } else if let Some(o2) = outcome2 {
            (
                o2,
                tier == ModelTier::Basic && !attempt1_self_check_rejection,
                false,
                completeness2,
            )
        } else {
            (outcome1, false, false, completeness1)
        };

    if let Some(failure) = final_completeness {
        crate::subtask_completeness::retain_incomplete_outcome(&mut outcome, &failure);
        on_progress(Progress::SubtaskIncomplete {
            id: subtask.id.clone(),
            expected: failure.expected,
            delivered: failure.delivered,
        });
        return outcome;
    }

    maybe_geometry_echo_with_outcomes(
        subtask,
        plan,
        request,
        llm,
        sink,
        abort,
        reduced_complexity,
        minimal_skills,
        agent_indicator_epoch,
        geometry_echo_budget,
        on_progress,
        prior_outcomes,
        outcome,
    )
    .await
}

/// One in-loop self-correction round for real resolved-layout violations.
///
/// A no-op (zero extra LLM calls) whenever:
/// - `outcome.node_count == 0` — nothing landed, nothing to check;
/// - `outcome.inserted_root_ids` is empty — a sink that cannot surface
///   post-insert ids has nothing live to lay out or address for a replace.
/// - the diagnostics come back empty — the common case, zero cost;
/// - the run-wide [`GeometryEchoBudget`] is exhausted.
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
async fn maybe_geometry_echo(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    agent_indicator_epoch: Option<u64>,
    budget: &GeometryEchoBudget,
    on_progress: &mut dyn FnMut(Progress),
    outcome: SubtaskOutcome,
) -> SubtaskOutcome {
    maybe_geometry_echo_with_outcomes(
        subtask,
        plan,
        request,
        llm,
        sink,
        abort,
        reduced_complexity,
        minimal_skills,
        agent_indicator_epoch,
        budget,
        on_progress,
        &[],
        outcome,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn maybe_geometry_echo_with_outcomes(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    agent_indicator_epoch: Option<u64>,
    budget: &GeometryEchoBudget,
    on_progress: &mut dyn FnMut(Progress),
    prior_outcomes: &[SubtaskOutcome],
    outcome: SubtaskOutcome,
) -> SubtaskOutcome {
    if outcome.node_count == 0
        || outcome.inserted_root_ids.is_empty()
        || abort.is_set()
        || sink.is_buffered()
    {
        return outcome;
    }
    let issues = crate::geometry_validation::geometry_diagnostics_for_roots(
        sink.state(),
        &outcome.inserted_root_ids,
    );
    if issues.is_empty() {
        return outcome;
    }
    if !budget.try_consume() {
        return outcome;
    }

    on_progress(Progress::GeometryEcho {
        id: subtask.id.clone(),
        issue_count: issues.len(),
    });
    tracing::info!(
        subtask = %subtask.id,
        issue_count = issues.len(),
        "geometry echo: resolved-layout violation(s) found, retrying in-loop"
    );

    let echo_subtask = Subtask {
        retry_feedback: Some(crate::plan::RetryFeedback::Geometry(issues.join("\n"))),
        ..subtask.clone()
    };
    let retried = run_subtask_with_reveal_at_and_outcomes(
        &echo_subtask,
        plan,
        request,
        llm,
        sink,
        abort,
        reduced_complexity,
        minimal_skills,
        agent_indicator_epoch,
        reveal_now_millis(),
        None,
        prior_outcomes,
    )
    .await;

    if retried.node_count == 0 {
        // The echo retry itself failed (LLM error, parse failure, or a
        // fresh self-check rejection) — keep the ORIGINAL, still-real
        // content rather than lose it; the deterministic net in
        // `cleanup.rs` picks up whatever geometry issues remain.
        return outcome;
    }

    // Adopt the corrected content: drop the original insert now that a
    // real replacement has landed. Delete-then-keep-the-new-insert rather
    // than a literal `EditorCommand::ReplaceSubtree` (which is 1-old-root-
    // to-1-new-node only) because a subtask can produce N top-level roots
    // on either side — this generalizes to N-old/M-new without assuming a
    // 1:1 shape.
    crate::subtask_completeness::rollback_inserted_roots(sink, &outcome.inserted_root_ids);
    retried
}

// ── Screen-group concurrent executor ────────────────────────────────────────

/// Result of the concurrent screen-group phase, in the same shape `run.rs`'s
/// sequential loop produces locally — so the downstream salvage /
/// zero-content / cleanup / `RunSummary` code is UNCHANGED regardless of
/// which phase ran.
pub(crate) struct ConcurrentPhaseResult {
    /// Every subtask's outcome that actually ran, in PLAN order — a subtask
    /// never reached because `abort` fired is simply absent, the same
    /// convention the sequential loop's early `break` already leaves.
    pub outcomes: Vec<SubtaskOutcome>,
    pub aborted_mid: bool,
    pub zero_node_failure: bool,
    pub incomplete_subtask_failure: bool,
    /// `(plan_index, outcomes_index)` for every zero-node outcome — `run.rs`'s
    /// end-of-run salvage pass retries exactly these, unchanged.
    pub salvage: Vec<(usize, usize)>,
}

/// Drives one worker future per screen group on a [`FuturesUnordered`] —
/// genuine concurrency (LLM calls from different groups overlap in
/// wall-clock time) with no `tokio::spawn`/extra `Send` bounds and no shared
/// `&mut` (each worker owns one fresh [`BufferDocSink`] per subtask). A shared
/// [`Semaphore`] caps in-flight subtask LLM calls at `effective_concurrency`.
///
/// **Visibility (three-piece fix, 2026-07-17)** — the earlier `join_all`
/// version drove every worker to completion, drained `on_progress` in one
/// batch AFTER all of them finished, then replayed every buffer in
/// ascending-plan-index order. That made a concurrent run LOOK sequential:
/// the progress checklist sat frozen until the very end, then every root
/// popped in at once. This version instead runs a `tokio::select!` loop
/// polling the progress channel and the `FuturesUnordered` TOGETHER:
/// - Every `Progress` event reaches `on_progress` the moment its worker
///   sends it — mid-run, not after `join_all` resolves.
/// - Each successful subtask's buffered commands replay into the REAL `sink`
///   the instant that subtask settles, without waiting for the rest of its
///   group. A failed subtask drops its unopened buffer. This is safe because
///   every group's root was already scaffolded (fixed id + position) before
///   this phase started (see `run_screen_groups::insert_screen_group_roots`),
///   so subtask replays from sibling groups target disjoint subtrees.
///
/// The REAL indicator epoch is applied only at replay time — see
/// `run_subtask_retry_ladder`'s doc for why buffered/worker-phase inserts
/// never touch `agent_indicators`. Per-group visual identity (distinct
/// colour/name per root) is NOT decided here — the caller already tagged
/// each group's scaffold root via `agent_indicators::add_frame` before this
/// phase runs, and the canvas paint's ownership walk inherits that tag down
/// into whatever this function replays, however it interleaves.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_screen_groups_concurrent(
    groups: &[ScreenGroup],
    group_identities: &[AgentIdentity],
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    tier: ModelTier,
    effective_concurrency: u32,
    agent_indicator_epoch: Option<u64>,
    geometry_echo_budget: &GeometryEchoBudget,
    on_progress: &mut dyn FnMut(Progress),
) -> ConcurrentPhaseResult {
    assert_eq!(
        groups.len(),
        group_identities.len(),
        "every concurrent screen group must have one stable identity"
    );
    // Item 4: a fact-line up front makes ⚡Nx's effect legible instead of
    // silent — before this, nothing distinguished a concurrent run from a
    // sequential one in the progress panel until content started landing.
    on_progress(Progress::ConcurrentGroupsStarted {
        group_count: groups.len(),
        workers: effective_concurrency,
    });
    // Publish the complete checklist for each screen before any worker can
    // finish. The desktop can now create stable per-agent bubbles without
    // guessing group ownership from interleaved subtask events.
    for (group_idx, (group, identity)) in groups.iter().zip(group_identities).enumerate() {
        let subtasks = group
            .indices
            .iter()
            .map(|&idx| {
                let subtask = &plan.subtasks[idx];
                (subtask.id.clone(), subtask.label.clone())
            })
            .collect();
        on_progress(Progress::worker_scoped(
            group_idx,
            group.screen.clone(),
            identity.clone(),
            Progress::Planned { subtasks },
        ));
    }

    let snapshot = sink.state().clone();
    let semaphore = Arc::new(Semaphore::new(effective_concurrency as usize));
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<WorkerSignal>();

    // Build one worker future per screen group, each future OWNS its own
    // group-local snapshot and constructs a fresh `BufferDocSink` per subtask —
    // no shared `&mut`. Successful commits update only that group's mirror, so
    // later same-group prompts see prior sections without absorbing sibling
    // completion order. Tagged with its group index for completion accounting.
    let mut worker_futures: FuturesUnordered<_> = groups
        .iter()
        .enumerate()
        .map(|(g_idx, group)| {
            let fut = run_screen_group_worker(
                g_idx,
                group,
                plan,
                request,
                llm,
                abort,
                tier,
                snapshot.clone(),
                Arc::clone(&semaphore),
                geometry_echo_budget,
                event_tx.clone(),
            );
            async move { (g_idx, fut.await) }
        })
        .collect();
    // Drop the master sender so the channel closes once every worker (and
    // thus every clone) has finished — `event_rx.recv()` then observes
    // `None` instead of hanging forever.
    drop(event_tx);

    let mut per_subtask: Vec<Option<(SubtaskOutcome, bool)>> = vec![None; plan.subtasks.len()];
    let mut aborted_mid = abort.is_set();
    let mut remaining = groups.len();
    // Poll the event channel and worker set TOGETHER. This loop is the ONE
    // real-document writer: it drains one successful subtask atomically,
    // then acks that worker so the same group may proceed to its next subtask.
    while remaining > 0 {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                apply_worker_event(
                    event,
                    groups,
                    group_identities,
                    plan,
                    sink,
                    agent_indicator_epoch,
                    &mut per_subtask,
                    on_progress,
                );
            }
            Some((_g_idx, result)) = worker_futures.next() => {
                remaining -= 1;
                aborted_mid |= result.aborted;
            }
        }
    }
    // An ordinary progress event and the LAST worker's completion can both be
    // ready in the same poll — drain whatever remains after the worker loop.
    // A SubtaskSettled event cannot normally remain here because its worker
    // waits for the event's ack before it can finish, but handling it keeps the
    // invariant fail-safe if worker shutdown changes later.
    while let Ok(event) = event_rx.try_recv() {
        apply_worker_event(
            event,
            groups,
            group_identities,
            plan,
            sink,
            agent_indicator_epoch,
            &mut per_subtask,
            on_progress,
        );
    }

    let mut outcomes = Vec::new();
    let mut salvage = Vec::new();
    let mut zero_node_failure = false;
    for (plan_idx, slot) in per_subtask.into_iter().enumerate() {
        if let Some((outcome, is_zero)) = slot {
            outcomes.push(outcome);
            if is_zero {
                zero_node_failure = true;
                salvage.push((plan_idx, outcomes.len() - 1));
            }
        }
    }

    let incomplete_subtask_failure = outcomes
        .iter()
        .any(crate::subtask_completeness::is_incomplete_outcome);
    ConcurrentPhaseResult {
        outcomes,
        aborted_mid,
        zero_node_failure,
        incomplete_subtask_failure,
        salvage,
    }
}

#[cfg(test)]
#[path = "concurrent_tests.rs"]
mod tests;
