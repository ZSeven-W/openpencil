//! Pull-based, transport-agnostic code-generation state machine. Ported
//! from the TS `generateCode` orchestration (code-generation-pipeline.ts
//! lines 224-466).
//!
//! Unlike the TS async/await + `Promise.all` version, this machine never
//! blocks and never spawns threads. The HOST drives it: it calls `step()`
//! to learn which requests to run, runs them against its own transport,
//! and feeds streamed text back via `on_delta` / `on_complete` /
//! `on_error`. The same logic therefore works on desktop (worker threads)
//! and web (fetch) and is fully unit-testable with canned text.

use std::collections::{HashMap, HashSet};

use op_editor_core::codegen::{ChunkProgress, ChunkStatus, CodeGenProgress};
use serde_json::Value;

use crate::ai::assets::{collect_chunk_asset_hints, extract_codegen_assets};
use crate::ai::fallback_plan::fallback_plan_from_nodes_json;
use crate::ai::parse::{
    clean_code, compute_execution_order, extract_plan_json, parse_chunk_response, sanitize_name,
    validate_contract,
};
use crate::ai::pipeline_preflight::{
    self as preflight, assembly_status_label, is_pascal_case, MAX_USER_PROMPT_BYTES,
};
use crate::ai::prompts::{assembly_request, chunk_request, plan_request, ChunkPromptInput};
use crate::ai::types::{
    AssetFile, ChunkResult, CodePlan, CodegenInput, ExecutableChunk, PendingRequest, PipelineStep,
    RequestId, RequestKind,
};

/// Which overall phase the machine is in.
enum Phase {
    Planning,
    Chunks,
    Assembly,
    /// One-shot whole-document generation after chunking produced no code.
    Rescue,
    /// A terminal state: `step()` keeps returning this value.
    Terminal(PipelineStep),
}

/// State for the single in-flight request the host has been told to run.
/// We accumulate streamed deltas into `buffer`, keyed by `RequestId`.
struct InFlight {
    kind: RequestKind,
    buffer: String,
    /// Set by `on_error`; consumed when `step()` processes the failure.
    error: Option<String>,
    /// Set by `on_complete`; the buffer is now final and ready to parse.
    completed: bool,
}

/// Per-chunk bookkeeping during the chunk phase.
struct ChunkState {
    exec: ExecutableChunk,
    status: ChunkStatus,
    result: Option<ChunkResult>,
    /// True once we've already retried this chunk after a failure.
    retried: bool,
    /// The `RequestId` currently dispatched for this chunk (None when idle).
    in_flight: Option<RequestId>,
}

pub struct CodegenPipeline {
    input: CodegenInput,
    /// Sanitized node JSON (asset data-URLs swapped for `./assets/...`).
    sanitized_nodes_json: String,
    /// One owned sanitized forest plus an id set. Chunk hydration borrows
    /// subtrees from this value on demand, avoiding the old O(N²) clone of
    /// every descendant subtree into an id → Value map.
    sanitized_nodes_value: Value,
    node_ids: HashSet<String>,
    assets: Vec<AssetFile>,

    phase: Phase,
    next_id: u64,
    /// Deltas for requests the host has been told to run but hasn't finished.
    in_flight: HashMap<RequestId, InFlight>,

    // ── Planning ──
    /// True once the strict-retry plan request has been issued.
    planning_retried: bool,
    /// Set when a planning attempt fails; the next `step()` re-dispatches.
    planning_retry_pending: bool,
    planning_done: Option<bool>,
    plan: Option<CodePlan>,

    // ── Chunks ──
    /// Ordered by execution order, then plan order.
    chunks: Vec<ChunkState>,

    // ── Assembly ──
    assembly_retried: bool,
    assembly_done: Option<bool>,

    /// Ordered failure trail retained across retries and included if the
    /// one-shot rescue also fails.
    failures: Vec<String>,
}

impl CodegenPipeline {
    pub fn new(input: CodegenInput) -> Self {
        // Pull embedded image assets out ONCE so prompts ship paths, not
        // base64 (TS: `extractCodegenAssets` at the top of generateCode).
        let (sanitized_nodes_json, assets) = extract_codegen_assets(&input.nodes_json);
        let sanitized_nodes_value =
            serde_json::from_str(&sanitized_nodes_json).unwrap_or(Value::Null);
        let mut node_ids = HashSet::new();
        preflight::index_node_ids(&sanitized_nodes_value, &mut node_ids);

        Self {
            input,
            sanitized_nodes_json,
            sanitized_nodes_value,
            node_ids,
            assets,
            phase: Phase::Planning,
            next_id: 0,
            in_flight: HashMap::new(),
            planning_retried: false,
            planning_retry_pending: false,
            planning_done: None,
            plan: None,
            chunks: Vec::new(),
            assembly_retried: false,
            assembly_done: None,
            failures: Vec::new(),
        }
    }

    fn alloc_id(&mut self) -> RequestId {
        let id = RequestId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Allocate a fresh id and register an empty in-flight buffer for `kind`.
    fn register_inflight(&mut self, kind: RequestKind) -> RequestId {
        let id = self.alloc_id();
        self.in_flight.insert(
            id,
            InFlight {
                kind,
                buffer: String::new(),
                error: None,
                completed: false,
            },
        );
        id
    }

    /// Ask the machine what to do next. Pure read of internal state plus the
    /// allocation of request ids; never blocks.
    pub fn step(&mut self) -> PipelineStep {
        match &self.phase {
            Phase::Terminal(step) => step.clone(),
            Phase::Planning => self.step_planning(),
            Phase::Chunks => self.step_chunks(),
            Phase::Assembly => self.step_assembly(),
            Phase::Rescue => self.step_rescue(),
        }
    }

    // ── Phase: Planning ──────────────────────────────────────────────────

    fn step_planning(&mut self) -> PipelineStep {
        // (a) A prior planning attempt failed and we still have a retry left.
        if self.planning_retry_pending {
            self.planning_retry_pending = false;
            self.planning_retried = true;
            let id = self.register_inflight(RequestKind::Planning);
            let req = plan_request(id, &self.input, true);
            if req.user_message.len() > MAX_USER_PROMPT_BYTES {
                self.in_flight.remove(&id);
                return self.handle_oversized_planning_prompt(req.user_message.len());
            }
            self.planning_done = Some(false);
            return PipelineStep::Dispatch(vec![req]);
        }

        // (b) A planning request is in flight — process its terminal signal.
        if let Some((id, flight)) = self.take_settled_inflight() {
            return self.resolve_planning(id, flight);
        }
        if self.has_inflight() {
            return PipelineStep::Waiting;
        }

        // (c) First entry: deterministic targets finish here; everything
        // else dispatches the non-strict plan request.
        if self.input.framework.is_deterministic() {
            return self.finish_deterministic_target();
        }
        let id = self.register_inflight(RequestKind::Planning);
        let req = plan_request(id, &self.input, false);
        if req.user_message.len() > MAX_USER_PROMPT_BYTES {
            self.in_flight.remove(&id);
            return self.handle_oversized_planning_prompt(req.user_message.len());
        }
        self.planning_done = Some(false);
        PipelineStep::Dispatch(vec![req])
    }

    fn handle_oversized_planning_prompt(&mut self, bytes: usize) -> PipelineStep {
        self.planning_done = Some(false);
        self.record_failure(
            "planning",
            format!(
                "prompt is {bytes} bytes (limit {MAX_USER_PROMPT_BYTES}); using hierarchical fallback plan"
            ),
        );
        match fallback_plan_from_nodes_json(&self.sanitized_nodes_json) {
            Some(plan) => self.apply_plan(plan),
            None => {
                self.phase = Phase::Rescue;
                self.step_rescue()
            }
        }
    }

    fn resolve_planning(&mut self, _id: RequestId, flight: InFlight) -> PipelineStep {
        // Error path: retry once, then fail terminally.
        if let Some(message) = flight.error {
            return self.handle_planning_failure(message);
        }

        // Completed: parse the buffered text into a CodePlan.
        let parsed = extract_plan_json(&flight.buffer)
            .and_then(|json| serde_json::from_str::<CodePlan>(&json).ok());

        let Some(plan) = parsed else {
            return self.handle_planning_parse_failure();
        };

        self.apply_plan(plan)
    }

    fn apply_plan(&mut self, plan: CodePlan) -> PipelineStep {
        let mut plan = plan;
        let structure_issue = preflight::plan_structure_issue(&plan.chunks);
        let mut exec_chunks = if structure_issue.is_none() {
            self.hydrate_plan(&plan)
        } else {
            Vec::new()
        };
        if structure_issue.is_none() && exec_chunks.is_empty() {
            self.planning_done = Some(false);
            self.record_failure("planning", "produced no valid chunks");
            self.phase = Phase::Rescue;
            return self.step_rescue();
        }
        if let Some(issue) = structure_issue.or_else(|| self.plan_dispatch_issue(&exec_chunks)) {
            self.record_failure("planning", format!("unsafe model plan: {issue}"));
            let fallback = fallback_plan_from_nodes_json(&self.sanitized_nodes_json);
            if let Some(candidate) = fallback {
                let candidate_structure_issue = preflight::plan_structure_issue(&candidate.chunks);
                let candidate_chunks = if candidate_structure_issue.is_none() {
                    self.hydrate_plan(&candidate)
                } else {
                    Vec::new()
                };
                if candidate_structure_issue.is_none()
                    && !candidate_chunks.is_empty()
                    && self.plan_dispatch_issue(&candidate_chunks).is_none()
                {
                    plan = candidate;
                    exec_chunks = candidate_chunks;
                } else {
                    self.record_failure("planning fallback", "still contains overlapping chunks");
                    self.planning_done = Some(false);
                    self.phase = Phase::Rescue;
                    return self.step_rescue();
                }
            } else {
                self.record_failure("planning fallback", "could not derive a safe chunk plan");
                self.planning_done = Some(false);
                self.phase = Phase::Rescue;
                return self.step_rescue();
            }
        }

        self.plan = Some(plan);
        self.planning_done = Some(true);
        self.chunks = exec_chunks
            .into_iter()
            .map(|exec| ChunkState {
                exec,
                status: ChunkStatus::Pending,
                result: None,
                retried: false,
                in_flight: None,
            })
            .collect();
        self.phase = Phase::Chunks;
        self.step_chunks()
    }

    fn handle_planning_parse_failure(&mut self) -> PipelineStep {
        if !self.planning_retried {
            return self.handle_planning_failure("response was not a valid code plan".to_string());
        }
        self.record_failure(
            "planning",
            "attempt 2: response was not a valid code plan; using deterministic plan fallback",
        );
        match fallback_plan_from_nodes_json(&self.sanitized_nodes_json) {
            Some(plan) => self.apply_plan(plan),
            None => {
                self.planning_done = Some(false);
                self.phase = Phase::Rescue;
                self.step_rescue()
            }
        }
    }

    fn handle_planning_failure(&mut self, message: String) -> PipelineStep {
        let attempt = if self.planning_retried { 2 } else { 1 };
        self.record_failure("planning", format!("attempt {attempt}: {message}"));
        if self.planning_retried {
            // Chunk planning is unavailable; bypass it with the one-shot
            // whole-document generator rather than terminating empty-handed.
            self.planning_done = Some(false);
            self.phase = Phase::Rescue;
            self.step_rescue()
        } else {
            // Dispatch the strict-prompt retry immediately (TS retries inline
            // within the same planning step).
            self.planning_retry_pending = true;
            self.step_planning()
        }
    }

    /// Port of hydratePlan (pipeline.ts:47-80): drop chunks whose nodeIds
    /// resolve to nothing in the input tree, compute execution order, and
    /// return the survivors sorted by (order, plan position).
    fn hydrate_plan(&self, plan: &CodePlan) -> Vec<ExecutableChunk> {
        let orders = compute_execution_order(&plan.chunks);
        let mut execs: Vec<ExecutableChunk> = plan
            .chunks
            .iter()
            .filter(|chunk| chunk.node_ids.iter().any(|id| self.node_ids.contains(id)))
            .map(|chunk| ExecutableChunk {
                plan: chunk.clone(),
                order: orders.get(&chunk.id).copied().unwrap_or(0),
            })
            .collect();
        // Stable sort by order keeps original plan order within a group.
        execs.sort_by_key(|e| e.order);
        execs
    }

    fn plan_dispatch_issue(&self, chunks: &[ExecutableChunk]) -> Option<String> {
        preflight::plan_dispatch_issue(chunks, &self.sanitized_nodes_value)
    }

    // ── Phase: Chunks ────────────────────────────────────────────────────

    fn step_chunks(&mut self) -> PipelineStep {
        // Drain any settled in-flight chunk first.
        while let Some((id, flight)) = self.take_settled_inflight() {
            self.resolve_chunk(id, flight);
        }

        loop {
            // All chunks terminal → advance to assembly.
            if self.all_chunks_terminal() {
                self.phase = Phase::Assembly;
                return self.step_assembly();
            }

            // Lowest incomplete order-group gates dispatch (TS batches by order).
            let Some(active_order) = self.lowest_incomplete_order() else {
                return PipelineStep::Waiting;
            };

            let mut dispatch: Vec<PendingRequest> = Vec::new();
            let mut progressed_without_dispatch = false;
            let mut idx = 0;
            while idx < self.chunks.len() {
                if self.chunks[idx].order() != active_order {
                    idx += 1;
                    continue;
                }
                // Skip chunks already settled or in flight.
                if self.chunks[idx].is_terminal() || self.chunks[idx].in_flight.is_some() {
                    idx += 1;
                    continue;
                }

                let chunk_id = self.chunks[idx].exec.plan.id.clone();
                let deps = self.chunks[idx].exec.plan.dependencies.clone();

                let blocked_deps = deps
                    .iter()
                    .filter(|dep| {
                        matches!(
                            self.dep_status(dep),
                            Some(ChunkStatus::Failed | ChunkStatus::Skipped)
                        )
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if !blocked_deps.is_empty() {
                    self.chunks[idx].status = ChunkStatus::Skipped;
                    self.record_failure(
                        format!("chunk {chunk_id}"),
                        format!(
                            "skipped because dependencies failed or were skipped: {}",
                            blocked_deps.join(", ")
                        ),
                    );
                    progressed_without_dispatch = true;
                    idx += 1;
                    continue;
                }

                // Collect dependency contracts (only deps with a component name).
                let dep_contracts = self.collect_dep_contracts(&deps);

                // Derive the chunk's node JSON + its asset hints.
                let chunk_nodes_json = self.chunk_nodes_json(&self.chunks[idx].exec.plan.node_ids);
                let ancestor_context = preflight::ancestor_context_json(
                    &self.sanitized_nodes_value,
                    &self.chunks[idx].exec.plan.node_ids,
                );
                let asset_hints = collect_chunk_asset_hints(&chunk_nodes_json, &self.assets);
                let suggested = self.chunks[idx].exec.plan.suggested_component_name.clone();

                let id = self.register_inflight(RequestKind::Chunk {
                    chunk_id: chunk_id.clone(),
                });
                let req = chunk_request(
                    id,
                    ChunkPromptInput::new(
                        &chunk_id,
                        &chunk_nodes_json,
                        &suggested,
                        ancestor_context.as_deref(),
                        &dep_contracts,
                        &asset_hints,
                    ),
                    &self.input,
                );
                if req.user_message.len() > MAX_USER_PROMPT_BYTES {
                    self.in_flight.remove(&id);
                    self.chunks[idx].status = ChunkStatus::Failed;
                    self.record_failure(
                        format!("chunk {chunk_id}"),
                        format!(
                            "prompt is {} bytes (limit {MAX_USER_PROMPT_BYTES}); request was not dispatched",
                            req.user_message.len()
                        ),
                    );
                    progressed_without_dispatch = true;
                    idx += 1;
                    continue;
                }
                self.chunks[idx].in_flight = Some(id);
                self.chunks[idx].status = ChunkStatus::Running;
                dispatch.push(req);
                idx += 1;
            }

            if !dispatch.is_empty() {
                return PipelineStep::Dispatch(dispatch);
            }
            if !progressed_without_dispatch {
                return PipelineStep::Waiting;
            }
        }
    }

    fn resolve_chunk(&mut self, _id: RequestId, flight: InFlight) {
        let RequestKind::Chunk { chunk_id } = &flight.kind else {
            return;
        };
        let Some(idx) = self.chunks.iter().position(|c| c.exec.plan.id == *chunk_id) else {
            return;
        };
        self.chunks[idx].in_flight = None;

        // Transport failure path: retry once, then mark Failed.
        if let Some(message) = flight.error {
            let attempt = if self.chunks[idx].retried { 2 } else { 1 };
            self.record_failure(
                format!("chunk {chunk_id}"),
                format!("attempt {attempt}: {message}"),
            );
            self.fail_or_retry_chunk(idx);
            return;
        }

        // Parse the buffered chunk response.
        let mut result = parse_chunk_response(&flight.buffer, chunk_id);

        // A completed stream with no source is semantically the same as a
        // failed request. Retry it once; never let an empty result masquerade
        // as a degraded-but-usable chunk.
        if result.code.trim().is_empty() {
            let attempt = if self.chunks[idx].retried { 2 } else { 1 };
            self.record_failure(
                format!("chunk {chunk_id}"),
                format!("attempt {attempt}: model returned empty code"),
            );
            self.fail_or_retry_chunk(idx);
            return;
        }

        // Force a valid PascalCase component name from the suggested label
        // when the model returned an empty / non-PascalCase one.
        if result.contract.component_name.is_empty()
            || !is_pascal_case(&result.contract.component_name)
        {
            result.contract.component_name =
                sanitize_name(&self.chunks[idx].exec.plan.suggested_component_name);
        }

        let (valid, _issues) = validate_contract(&result);
        self.chunks[idx].status = if valid {
            ChunkStatus::Done
        } else {
            ChunkStatus::Degraded
        };
        self.chunks[idx].result = Some(result);
    }

    fn fail_or_retry_chunk(&mut self, idx: usize) {
        if self.chunks[idx].retried {
            self.chunks[idx].status = ChunkStatus::Failed;
        } else {
            // Re-dispatch on the next `step()` by leaving it non-terminal +
            // not in flight; mark that we've consumed the one retry.
            self.chunks[idx].retried = true;
            self.chunks[idx].status = ChunkStatus::Pending;
        }
    }

    // ── Phase: Assembly ──────────────────────────────────────────────────

    fn step_assembly(&mut self) -> PipelineStep {
        // Process a settled assembly request first.
        if let Some((_id, flight)) = self.take_settled_inflight() {
            return self.resolve_assembly(flight);
        }
        if self.has_inflight() {
            return PipelineStep::Waiting;
        }

        let chunk_blocks = self.build_chunk_blocks();

        if self
            .chunks
            .iter()
            .all(|c| self.chunk_code(c).trim().is_empty())
        {
            self.assembly_done = Some(false);
            self.record_failure("assembly", "all chunks failed; no code to assemble");
            self.phase = Phase::Rescue;
            return self.step_rescue();
        }

        let plan_summary = self.build_plan_summary();
        let asset_paths: Vec<String> = self
            .assets
            .iter()
            .map(|a| a.relative_path.clone())
            .collect();

        let id = self.register_inflight(RequestKind::Assembly);
        let req = assembly_request(id, &chunk_blocks, &plan_summary, &self.input, &asset_paths);
        if req.user_message.len() > MAX_USER_PROMPT_BYTES {
            self.in_flight.remove(&id);
            self.record_failure(
                "assembly",
                format!(
                    "prompt is {} bytes (limit {MAX_USER_PROMPT_BYTES}); using whole-document rescue",
                    req.user_message.len()
                ),
            );
            self.phase = Phase::Rescue;
            return self.step_rescue();
        }
        self.assembly_done = Some(false);
        PipelineStep::Dispatch(vec![req])
    }

    fn resolve_assembly(&mut self, flight: InFlight) -> PipelineStep {
        let degraded = self.any_chunk_degraded_or_worse();
        let code = clean_code(&flight.buffer);
        let failure = flight
            .error
            .or_else(|| preflight::model_output_failure(self.input.framework, &code));

        if let Some(message) = failure {
            let attempt = if self.assembly_retried { 2 } else { 1 };
            self.record_failure("assembly", format!("attempt {attempt}: {message}"));
            if self.assembly_retried {
                self.phase = Phase::Rescue;
                return self.step_rescue();
            }
            // First failure → re-dispatch immediately (TS retries inline).
            self.assembly_retried = true;
            self.assembly_done = Some(false);
            return self.step_assembly();
        }

        self.assembly_done = Some(true);
        let step = PipelineStep::Done {
            code,
            degraded,
            assets: self.assets.clone(),
        };
        self.phase = Phase::Terminal(step.clone());
        step
    }

    /// Per-chunk block sent to the assembly AI. Carries the status header,
    /// the chunk code, and — for non-failed chunks — the contract detail
    /// (TS `chunksSection`, codegen-prompts.ts:142-153): successful chunks
    /// emit `Contract: {json}`; degraded chunks emit the "infer from code"
    /// NOTE; failed chunks contribute an empty code block.
    fn build_chunk_blocks(&self) -> String {
        self.chunks
            .iter()
            .map(|c| {
                let name = if c.exec.plan.name.is_empty() {
                    c.exec.plan.id.as_str()
                } else {
                    c.exec.plan.name.as_str()
                };
                let status = assembly_status_label(c.status);
                let code = self.chunk_code(c);
                let detail = match status {
                    "successful" => c
                        .result
                        .as_ref()
                        .and_then(|r| serde_json::to_string(&r.contract).ok())
                        .map(|json| format!("\nContract: {json}"))
                        .unwrap_or_default(),
                    "degraded" => "\n*NOTE: No contract available. Infer component name and \
                                    imports from the code.*"
                        .to_string(),
                    _ => String::new(),
                };
                format!("// ── {name} ({status}) ──\n\n{code}{detail}")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Full rootLayout + sharedStyles for the assembly prompt (TS
    /// codegen-prompts.ts:170-171 sends `Root layout: {json}` and
    /// `Shared styles: {json}`), so the assembler has direction / gap /
    /// responsive + every shared-style name available.
    fn build_plan_summary(&self) -> String {
        let root_layout_json = self
            .plan
            .as_ref()
            .and_then(|p| serde_json::to_string(&p.root_layout).ok())
            .unwrap_or_else(|| "{}".to_string());
        let shared_styles_json = self
            .plan
            .as_ref()
            .and_then(|p| serde_json::to_string(&p.shared_styles).ok())
            .unwrap_or_else(|| "[]".to_string());
        let node_ids = self
            .chunks
            .iter()
            .flat_map(|chunk| chunk.exec.plan.node_ids.iter().cloned())
            .collect::<Vec<_>>();
        let ancestor_context =
            preflight::ancestor_context_json(&self.sanitized_nodes_value, &node_ids)
                .unwrap_or_else(|| "[]".to_string());
        format!(
            "Root layout: {root_layout_json}\nShared styles: {shared_styles_json}\n\
             Ancestor wrapper context: {ancestor_context}"
        )
    }

    fn chunk_code<'a>(&self, c: &'a ChunkState) -> &'a str {
        c.result.as_ref().map(|r| r.code.as_str()).unwrap_or("")
    }

    fn any_chunk_degraded_or_worse(&self) -> bool {
        self.chunks.iter().any(|c| c.status != ChunkStatus::Done)
    }

    // ── In-flight delta handling ─────────────────────────────────────────

    pub fn on_delta(&mut self, id: RequestId, delta: &str) {
        if let Some(flight) = self.in_flight.get_mut(&id) {
            flight.buffer.push_str(delta);
        }
    }

    pub fn on_complete(&mut self, id: RequestId) {
        if let Some(flight) = self.in_flight.get_mut(&id) {
            flight.completed = true;
        }
    }

    pub fn on_error(&mut self, id: RequestId, message: String) {
        if let Some(flight) = self.in_flight.get_mut(&id) {
            flight.error = Some(message);
            flight.completed = true;
        }
    }

    pub fn cancel(&mut self) {
        self.phase = Phase::Terminal(PipelineStep::Failed {
            message: "Aborted".to_string(),
        });
    }

    /// True if any request is dispatched but not yet settled.
    fn has_inflight(&self) -> bool {
        !self.in_flight.is_empty()
    }

    /// Remove and return the first in-flight request that has settled
    /// (completed or errored). Deterministic order is not required because
    /// at most one request per phase is in flight (planning / assembly),
    /// and chunks are resolved one at a time in a drain loop.
    fn take_settled_inflight(&mut self) -> Option<(RequestId, InFlight)> {
        let settled = self
            .in_flight
            .iter()
            .find(|(_, f)| f.completed)
            .map(|(id, _)| *id)?;
        self.in_flight.remove(&settled).map(|f| (settled, f))
    }

    // ── Progress snapshot ────────────────────────────────────────────────

    pub fn progress(&self) -> CodeGenProgress {
        CodeGenProgress {
            planning_done: self.planning_done,
            chunks: self
                .chunks
                .iter()
                .map(|c| ChunkProgress {
                    chunk_id: c.exec.plan.id.clone(),
                    name: if c.exec.plan.name.is_empty() {
                        c.exec.plan.id.clone()
                    } else {
                        c.exec.plan.name.clone()
                    },
                    status: c.status,
                })
                .collect(),
            assembly_done: self.assembly_done,
        }
    }
}

impl ChunkState {
    fn order(&self) -> usize {
        self.exec.order
    }

    fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            ChunkStatus::Done | ChunkStatus::Degraded | ChunkStatus::Failed | ChunkStatus::Skipped
        )
    }
}

#[path = "pipeline_chunk_helpers.rs"]
mod chunk_helpers;

#[path = "pipeline_rescue.rs"]
mod rescue;

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod tests;
