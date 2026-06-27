//! Real parallel sub-agent generation for the agentic-loop `spawn_agents`
//! tool (Track-1 Step 5).
//!
//! When the gated agentic design loop calls the `spawn_agents` tool, the
//! model hands us a list of independent subtask specs (one prompt + one
//! target container per agent). This module runs those subtasks
//! **concurrently** by REUSING the orchestrator's per-subtask runner
//! ([`crate::subagent::run_subtask`]) and its concurrency machinery
//! ([`crate::concurrent::BufferDocSink`] + [`crate::concurrent::clamp_concurrency`]
//! with a shared `tokio::Semaphore` and `futures::future::join_all`) — the SAME
//! pieces the orchestrator's default concurrent path
//! ([`crate::concurrent::run_concurrent`]) is built from.
//!
//! ## What is reused vs. what is new
//!
//! - **Reused, unmodified:** [`crate::subagent::run_subtask`] (the full
//!   per-subtask generate → parse → Class-A passes → `InsertSubtree` unit),
//!   [`crate::concurrent::BufferDocSink`] (per-worker command buffer),
//!   [`crate::concurrent::clamp_concurrency`] (the `[1,6]` cap), and the
//!   buffer-replay-in-deterministic-order discipline.
//! - **New (additive):** the spec→subtask mapping and a worker-per-spec
//!   fan-out that drives N `run_subtask` calls under one `Semaphore`. The
//!   orchestrator's own `run_concurrent` is NOT touched — this is a parallel
//!   entry point for the loop bridge, not an edit to the default path.
//!
//! ## Why not call `run_concurrent` directly
//!
//! `run_concurrent` assumes the orchestrator's screen-grouping + N-root
//! scaffold + variable-seed phases already ran (it groups subtasks by
//! `screen`, replays per-group). The loop's `spawn_agents` specs are flat,
//! pre-scoped (each spec names its own container node) and target a live,
//! already-assembled document — there is no screen grouping or scaffold to
//! build. So we reuse the SAME building blocks at one level down
//! (`run_subtask` + `Semaphore` + `BufferDocSink`) rather than the
//! screen-group orchestration on top of them.

use std::sync::Arc;

use futures::future::join_all;
use tokio::sync::Semaphore;

use crate::concurrent::{clamp_concurrency, BufferDocSink};
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::subagent::{
    apply_command_with_reveal, apply_insert_subtree_with_reveal, reveal_now_millis, run_subtask,
};
use crate::types::{AbortFlag, DesignRequest, DocSink, LlmClient, SubtaskOutcome};
use op_editor_core::{EditorCommand, PenNodeExt};

/// One spec the model handed `spawn_agents`: a focused design prompt plus
/// the container node the produced subtree must land under.
///
/// This is the orchestrator-side shape (decoupled from
/// `op_mcp::spawn_agents_tool::SpawnSpec` so `op-orchestrator` does not
/// depend on `op-mcp`). The loop bridge maps the MCP spec — after resolving
/// styleguide / guideline NAMES into the prompt text — onto this struct.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnAgentSpec {
    /// Stable id for this agent (used as the subtask id + id-prefix so
    /// generated node ids don't collide across concurrent workers).
    pub id: String,
    /// Human label surfaced on the subtask-progress UI.
    pub label: String,
    /// The full design prompt for this agent (the loop bridge has already
    /// inlined the resolved styleguide + guideline content into it).
    pub prompt: String,
    /// The container node the produced subtree is inserted under. `None`
    /// inserts at the active page root (same semantics as a subtask with
    /// no `parent_frame_id`).
    pub parent_frame_id: Option<String>,
}

/// The result of one spawned agent — what it created, for the structured
/// tool result the loop returns to the model.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnAgentResult {
    /// The agent's id (echoes [`SpawnAgentSpec::id`]).
    pub id: String,
    /// Number of top-level nodes the agent produced (0 = failed).
    pub node_count: usize,
    /// Post-remap root ids the agent inserted into the live document.
    pub inserted_root_ids: Vec<String>,
    /// Failure message when the agent produced nothing; `None` on success.
    pub error: Option<String>,
}

impl From<SubtaskOutcome> for SpawnAgentResult {
    fn from(o: SubtaskOutcome) -> Self {
        SpawnAgentResult {
            id: o.id,
            node_count: o.node_count,
            inserted_root_ids: o.inserted_root_ids,
            error: o.error,
        }
    }
}

/// Build a minimal [`OrchestratorPlan`] from the live document's root frame
/// so [`run_subtask`]'s Class-A passes (theme detection, canvas-width-aware
/// role resolution, page-bg heuristics) have the same context they get on
/// the orchestrator path.
///
/// `subtasks` is filled from the specs so each `run_subtask` can resolve its
/// own `plan_idx` (used by `hoist_app_state`).
fn plan_from_state(sink: &dyn DocSink, specs: &[SpawnAgentSpec]) -> OrchestratorPlan {
    // Root frame = the first top-level frame of the active page when present;
    // else a neutral full-canvas spec. This mirrors how the orchestrator's
    // own plan carries the page root frame (width + fill drive theme).
    let (width, height, fill) = sink
        .state()
        .active_children()
        .first()
        .map(|n| {
            (
                n.width_px().unwrap_or(1200.0),
                n.height_px().unwrap_or(800.0),
                op_editor_core::fills::first_solid_fill_hex(n).map(|hex| {
                    vec![crate::plan::PlanFill {
                        kind: "solid".into(),
                        color: hex.to_string(),
                    }]
                }),
            )
        })
        .unwrap_or((1200.0, 800.0, None));

    let subtasks = specs
        .iter()
        .map(|spec| Subtask {
            id: spec.id.clone(),
            label: spec.label.clone(),
            region: Region { width, height },
            id_prefix: spec.id.clone(),
            parent_frame_id: spec.parent_frame_id.clone(),
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
        })
        .collect();

    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "spawn-root".into(),
            name: "Page".into(),
            width,
            height,
            layout: None,
            gap: None,
            padding: None,
            fill,
        },
        subtasks,
        style_guide_name: None,
    }
}

/// Run N spawned agents CONCURRENTLY, reusing the orchestrator's per-subtask
/// runner and concurrency cap.
///
/// ## Concurrency (genuine, bounded)
/// A shared `Arc<Semaphore>` with `clamp_concurrency(concurrency)` permits
/// caps the number of in-flight subagent LLM calls — the exact same cap and
/// the exact same RAII-permit-drop discipline [`crate::concurrent::run_concurrent`]
/// uses. One worker future per spec is built; all are driven together with
/// [`futures::future::join_all`] on the current task (no `tokio::spawn`, to
/// avoid `Send` bounds — the LLM I/O is already offloaded inside
/// `LlmClient::call`). `join_all` polls every worker on each wake, so
/// different specs' LLM calls genuinely overlap.
///
/// ## Isolation + deterministic merge
/// Each worker owns its own [`BufferDocSink`] (a clone of the pre-spawn
/// `EditorState` snapshot) — no shared `&mut`, which is what lets `join_all`
/// drive them. After `join_all`, the buffered commands are replayed into the
/// real `sink` in **spec order** (deterministic), via the same
/// `apply_command_with_reveal` path the orchestrator uses.
///
/// ## Return
/// One [`SpawnAgentResult`] per spec, in spec order — what each agent
/// created, for the structured tool result the loop hands back to the model.
pub async fn run_spawned_agents_concurrent(
    specs: &[SpawnAgentSpec],
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    concurrency: u32,
    host_epoch: Option<u64>,
) -> Vec<SpawnAgentResult> {
    if specs.is_empty() {
        return Vec::new();
    }

    let plan = plan_from_state(sink, specs);
    // Pre-spawn read-only snapshot — each worker's BufferDocSink reads it for
    // generation context (theme / prior-accent / variable binding) exactly as
    // the orchestrator's concurrent workers do.
    let snapshot = sink.state().clone();

    let permits = clamp_concurrency(concurrency) as usize;
    let semaphore = Arc::new(Semaphore::new(permits));

    // Shared `&plan` for every worker future (none of them mutate it). Binding
    // a reference here — rather than letting `async move` move `plan` into the
    // first future — is what lets all N workers borrow it concurrently, exactly
    // as `run_concurrent` shares `&plan` across its screen-group workers.
    let plan_ref = &plan;

    // One worker future per spec. Each OWNS its own BufferDocSink (no shared
    // &mut) so join_all is legal and the calls genuinely overlap.
    let worker_futures = (0..plan.subtasks.len()).map(|idx| {
        let mut buffer = BufferDocSink::new(snapshot.clone());
        let sem = Arc::clone(&semaphore);
        async move {
            let subtask = &plan_ref.subtasks[idx];
            // Abort before acquiring a permit so a cancelled run does no LLM work.
            if abort.is_set() {
                return (
                    idx,
                    buffer,
                    SubtaskOutcome {
                        id: subtask.id.clone(),
                        node_count: 0,
                        error: Some("aborted".into()),
                        inserted_root_ids: Vec::new(),
                    },
                );
            }
            // Acquire one permit before the LLM call. RAII drop = releaseSlot.
            let _permit = sem
                .acquire()
                .await
                .expect("spawn semaphore should not be closed");
            let outcome = run_subtask(
                subtask,
                plan_ref,
                request,
                llm,
                &mut buffer,
                abort,
                false,
                false,
            )
            .await;
            (idx, buffer, outcome)
        }
    });

    // Drive ALL workers concurrently — LLM calls from different specs overlap.
    let mut worker_results: Vec<(usize, BufferDocSink, SubtaskOutcome)> =
        join_all(worker_futures).await;

    // Deterministic merge: replay each worker's buffered commands into the
    // real sink in spec order (ascending idx). Same reveal bookkeeping the
    // orchestrator uses.
    //
    // The per-worker `BufferDocSink` cannot mint post-remap ids (its state is a
    // read-only snapshot), so each `SubtaskOutcome.inserted_root_ids` is empty.
    // We capture the REAL post-remap root ids HERE — at the point the subtree
    // lands in the live document — by routing each `InsertSubtree` through
    // `insert_subtree_returning_root_ids` so the structured result the loop
    // returns to the model names the nodes that actually exist.
    worker_results.sort_by_key(|(idx, _, _)| *idx);
    let mut results: Vec<SpawnAgentResult> = Vec::with_capacity(worker_results.len());
    for (_, mut buffer, outcome) in worker_results {
        let mut inserted_root_ids: Vec<String> = Vec::new();
        for cmd in buffer.commands.drain(..) {
            match cmd {
                EditorCommand::InsertSubtree {
                    nodes, parent_id, ..
                } => {
                    if let Some(ids) = apply_insert_subtree_with_reveal(
                        sink,
                        nodes,
                        parent_id,
                        host_epoch,
                        reveal_now_millis(),
                    ) {
                        inserted_root_ids.extend(ids);
                    }
                }
                other => {
                    apply_command_with_reveal(sink, other, host_epoch, reveal_now_millis());
                }
            }
        }
        let mut result: SpawnAgentResult = outcome.into();
        // Prefer the real post-remap ids captured at replay over the empty
        // buffer-sink ids; keep `node_count` / `error` from the runner.
        if !inserted_root_ids.is_empty() {
            result.inserted_root_ids = inserted_root_ids;
        }
        results.push(result);
    }

    results
}

#[cfg(test)]
#[path = "spawn_concurrent_tests.rs"]
mod tests;
