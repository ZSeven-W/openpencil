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

use std::collections::HashMap;

use op_editor_core::codegen::{ChunkProgress, ChunkStatus, CodeGenProgress};
use serde_json::Value;

use crate::ai::assets::{collect_chunk_asset_hints, extract_codegen_assets};
use crate::ai::fallback_plan::fallback_plan_from_nodes_json;
use crate::ai::parse::{
    clean_code, compute_execution_order, extract_plan_json, parse_chunk_response, sanitize_name,
    validate_contract,
};
use crate::ai::prompts::{assembly_request, chunk_request, plan_request};
use crate::ai::types::{
    AssetFile, ChunkResult, CodePlan, CodegenInput, ExecutableChunk, PendingRequest, PipelineStep,
    RequestId, RequestKind,
};

/// Which overall phase the machine is in.
enum Phase {
    Planning,
    Chunks,
    Assembly,
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
    /// Recursive id → node Value map, deep-walking `children`. Built ONCE so
    /// hydration (id presence) and per-chunk node serialization both resolve
    /// nested ids, not just top-level ones (TS `indexNodes` / `hydratePlan`).
    node_map: HashMap<String, Value>,
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
}

impl CodegenPipeline {
    pub fn new(input: CodegenInput) -> Self {
        // Pull embedded image assets out ONCE so prompts ship paths, not
        // base64 (TS: `extractCodegenAssets` at the top of generateCode).
        let (sanitized_nodes_json, assets) = extract_codegen_assets(&input.nodes_json);
        let sanitized_nodes_value =
            serde_json::from_str(&sanitized_nodes_json).unwrap_or(Value::Null);
        // Deep-walk the sanitized tree ONCE into an id → node map (TS
        // `indexNodes`), so both hydration and chunk-node slicing resolve
        // nested ids.
        let mut node_map = HashMap::new();
        index_nodes(&sanitized_nodes_value, &mut node_map);

        Self {
            input,
            sanitized_nodes_json,
            node_map,
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

        // (c) First entry: dispatch the non-strict plan request.
        let id = self.register_inflight(RequestKind::Planning);
        let req = plan_request(id, &self.input, false);
        self.planning_done = Some(false);
        PipelineStep::Dispatch(vec![req])
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
        // Hydrate the plan against the real node tree.
        let exec_chunks = self.hydrate_plan(&plan);
        if exec_chunks.is_empty() {
            // TS: `Planning produced no valid chunks` is terminal (no retry).
            self.planning_done = Some(false);
            let step = PipelineStep::Failed {
                message: "Planning produced no valid chunks".to_string(),
            };
            self.phase = Phase::Terminal(step.clone());
            return step;
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
            return self.handle_planning_failure("Planning failed".to_string());
        }
        match fallback_plan_from_nodes_json(&self.sanitized_nodes_json) {
            Some(plan) => self.apply_plan(plan),
            None => self.handle_planning_failure("Planning failed".to_string()),
        }
    }

    fn handle_planning_failure(&mut self, message: String) -> PipelineStep {
        if self.planning_retried {
            // Already used the one strict retry — fail terminally.
            self.planning_done = Some(false);
            let step = PipelineStep::Failed { message };
            self.phase = Phase::Terminal(step.clone());
            step
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
            .filter(|chunk| {
                chunk
                    .node_ids
                    .iter()
                    .any(|id| self.node_map.contains_key(id))
            })
            .map(|chunk| ExecutableChunk {
                plan: chunk.clone(),
                order: orders.get(&chunk.id).copied().unwrap_or(0),
            })
            .collect();
        // Stable sort by order keeps original plan order within a group.
        execs.sort_by_key(|e| e.order);
        execs
    }

    // ── Phase: Chunks ────────────────────────────────────────────────────

    fn step_chunks(&mut self) -> PipelineStep {
        // Drain any settled in-flight chunk first.
        while let Some((id, flight)) = self.take_settled_inflight() {
            self.resolve_chunk(id, flight);
        }

        // All chunks terminal → advance to assembly.
        if self.all_chunks_terminal() {
            self.phase = Phase::Assembly;
            return self.step_assembly();
        }

        // Lowest incomplete order-group gates dispatch (TS batches by order).
        let Some(active_order) = self.lowest_incomplete_order() else {
            // Nothing incomplete but not all terminal — only happens with
            // in-flight chunks; wait for their deltas.
            return PipelineStep::Waiting;
        };

        let mut dispatch: Vec<PendingRequest> = Vec::new();
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

            // If any dependency failed, this chunk is skipped (no dispatch).
            if deps
                .iter()
                .any(|d| self.dep_status(d) == Some(ChunkStatus::Failed))
            {
                self.chunks[idx].status = ChunkStatus::Skipped;
                idx += 1;
                continue;
            }

            // Collect dependency contracts (only deps with a component name).
            let dep_contracts = self.collect_dep_contracts(&deps);

            // Derive the chunk's node JSON + its asset hints.
            let chunk_nodes_json = self.chunk_nodes_json(&self.chunks[idx].exec.plan.node_ids);
            let asset_hints = collect_chunk_asset_hints(&chunk_nodes_json, &self.assets);
            let suggested = self.chunks[idx].exec.plan.suggested_component_name.clone();

            let id = self.register_inflight(RequestKind::Chunk {
                chunk_id: chunk_id.clone(),
            });
            let req = chunk_request(
                id,
                &chunk_id,
                &chunk_nodes_json,
                &suggested,
                &dep_contracts,
                &asset_hints,
                &self.input,
            );
            self.chunks[idx].in_flight = Some(id);
            self.chunks[idx].status = ChunkStatus::Running;
            dispatch.push(req);
            idx += 1;
        }

        if dispatch.is_empty() {
            PipelineStep::Waiting
        } else {
            PipelineStep::Dispatch(dispatch)
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

        // Failure path: retry once, then mark Failed.
        if flight.error.is_some() {
            self.fail_or_retry_chunk(idx);
            return;
        }

        // Parse the buffered chunk response.
        let mut result = parse_chunk_response(&flight.buffer, chunk_id);

        // TS (code-generation-pipeline.ts:296-342) only retries a chunk on a
        // THROWN stream error (our `on_error`). A successfully-parsed result —
        // even one with empty/poor code — flows through `validate_contract`
        // and ends Done (valid) or Degraded (invalid); it is never failed by
        // empty code. Retry-once lives ONLY on the `on_error` path.

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

    fn collect_dep_contracts(&self, deps: &[String]) -> Vec<crate::ai::types::ChunkContract> {
        deps.iter()
            .filter_map(|dep_id| {
                self.chunks
                    .iter()
                    .find(|c| c.exec.plan.id == *dep_id)
                    .and_then(|c| c.result.as_ref())
                    .map(|r| r.contract.clone())
                    .filter(|c| !c.component_name.is_empty())
            })
            .collect()
    }

    fn dep_status(&self, dep_id: &str) -> Option<ChunkStatus> {
        self.chunks
            .iter()
            .find(|c| c.exec.plan.id == dep_id)
            .map(|c| c.status)
    }

    /// Serialize exactly the nodes whose `id` is in `node_ids`, in `node_ids`
    /// order, resolved from the RECURSIVE id map (TS `hydratePlan` resolves
    /// each id from `nodeMap`, so a nested child id yields just that child —
    /// not the whole tree). Falls back to the whole sanitized tree only when
    /// none of the ids resolve.
    fn chunk_nodes_json(&self, node_ids: &[String]) -> String {
        let subset: Vec<&Value> = node_ids
            .iter()
            .filter_map(|id| self.node_map.get(id))
            .collect();
        if !subset.is_empty() {
            return serde_json::to_string(&subset)
                .unwrap_or_else(|_| self.sanitized_nodes_json.clone());
        }
        // Fallback: pass the whole sanitized tree (still valid input for the
        // chunk prompt; hints just won't narrow).
        self.sanitized_nodes_json.clone()
    }

    fn all_chunks_terminal(&self) -> bool {
        self.chunks.iter().all(|c| c.is_terminal())
    }

    fn lowest_incomplete_order(&self) -> Option<usize> {
        self.chunks
            .iter()
            .filter(|c| !c.is_terminal())
            .map(|c| c.order())
            .min()
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

        // No chunk produced any code → terminal failure (TS: throw).
        if self.chunks.iter().all(|c| self.chunk_code(c).is_empty()) {
            self.assembly_done = Some(false);
            let step = PipelineStep::Failed {
                message: "All chunks failed — no code to assemble".to_string(),
            };
            self.phase = Phase::Terminal(step.clone());
            return step;
        }

        let plan_summary = self.build_plan_summary();
        let asset_paths: Vec<String> = self
            .assets
            .iter()
            .map(|a| a.relative_path.clone())
            .collect();

        let id = self.register_inflight(RequestKind::Assembly);
        let req = assembly_request(id, &chunk_blocks, &plan_summary, &self.input, &asset_paths);
        self.assembly_done = Some(false);
        PipelineStep::Dispatch(vec![req])
    }

    fn resolve_assembly(&mut self, flight: InFlight) -> PipelineStep {
        let degraded = self.any_chunk_degraded_or_worse();

        if flight.error.is_some() {
            if self.assembly_retried {
                // Second failure → best-effort fallback to concatenation of
                // ONLY the chunks that produced code (TS filters empty ones).
                self.assembly_done = Some(false);
                let code = self.build_fallback_code();
                let step = PipelineStep::Done {
                    code,
                    degraded: true,
                    assets: self.assets.clone(),
                };
                self.phase = Phase::Terminal(step.clone());
                return step;
            }
            // First failure → re-dispatch immediately (TS retries inline).
            self.assembly_retried = true;
            self.assembly_done = Some(false);
            return self.step_assembly();
        }

        let code = clean_code(&flight.buffer);
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

    /// Best-effort fallback when assembly fails twice: concatenate ONLY the
    /// chunks that actually produced code (TS code-generation-pipeline.ts:
    /// 451-454 filters to `c.code` first), joined by the status header. An
    /// empty failed/skipped chunk must NOT contribute a header-only section.
    fn build_fallback_code(&self) -> String {
        self.chunks
            .iter()
            .filter(|c| !self.chunk_code(c).is_empty())
            .map(|c| {
                let name = if c.exec.plan.name.is_empty() {
                    c.exec.plan.id.as_str()
                } else {
                    c.exec.plan.name.as_str()
                };
                let status = assembly_status_label(c.status);
                let code = self.chunk_code(c);
                format!("// ── {name} ({status}) ──\n\n{code}")
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
        format!("Root layout: {root_layout_json}\nShared styles: {shared_styles_json}")
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

/// True PascalCase check — port of /^[A-Z][a-zA-Z0-9]*$/. Mirrors the
/// private helper in `parse.rs` (not re-exported there).
fn is_pascal_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// Map a chunk status to the TS assembly-block label
/// (`done → successful`, `degraded → degraded`, everything else → failed).
fn assembly_status_label(status: ChunkStatus) -> &'static str {
    match status {
        ChunkStatus::Done => "successful",
        ChunkStatus::Degraded => "degraded",
        _ => "failed",
    }
}

/// Deep-walk the node JSON tree into an id → node Value map (TS
/// `indexNodes`, code-generation-pipeline.ts:49-55). Recurses through
/// `children` so nested ids resolve to their own subtree node. Used by both
/// hydration (id presence) and per-chunk node serialization. The first node
/// seen for an id wins (matching the TS Map insert order — top-level before
/// descendants).
fn index_nodes(value: &Value, out: &mut HashMap<String, Value>) {
    match value {
        Value::Array(items) => {
            for item in items {
                index_nodes(item, out);
            }
        }
        Value::Object(map) => {
            if let Some(id) = map.get("id").and_then(Value::as_str) {
                out.entry(id.to_string()).or_insert_with(|| value.clone());
            }
            if let Some(children) = map.get("children") {
                index_nodes(children, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::{CodegenInput, PipelineStep, RequestKind};
    use op_ai::chat_provider::{EffortLevel, ThinkingMode};
    use op_editor_core::codegen::Framework;

    fn input() -> CodegenInput {
        CodegenInput {
            nodes_json: "[{\"type\":\"frame\",\"id\":\"n1\",\"children\":[]}]".into(),
            framework: Framework::React,
            variables_json: None,
            max_output_tokens: 3000,
            thinking: ThinkingMode::Adaptive,
            effort: EffortLevel::Low,
        }
    }

    #[test]
    fn single_chunk_run_reaches_done() {
        let mut p = CodegenPipeline::new(input());
        let reqs = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("{other:?}"),
        };
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].kind, RequestKind::Planning);
        let plan_id = reqs[0].id;
        p.on_delta(plan_id, "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"root\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}");
        p.on_complete(plan_id);
        let reqs = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("{other:?}"),
        };
        assert_eq!(reqs.len(), 1);
        let chunk_id = reqs[0].id;
        p.on_delta(
            chunk_id,
            "export default function Root(){ return null }\n---CONTRACT---\n{\"componentName\":\"Root\"}",
        );
        p.on_complete(chunk_id);
        let reqs = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("{other:?}"),
        };
        let asm_id = reqs[0].id;
        p.on_delta(asm_id, "export default function App(){ return <Root/> }");
        p.on_complete(asm_id);
        match p.step() {
            PipelineStep::Done { code, degraded, .. } => {
                assert!(code.contains("function App"));
                assert!(!degraded);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn planning_parse_failure_retries_once_then_uses_fallback_plan() {
        let mut p = CodegenPipeline::new(CodegenInput {
            nodes_json: r#"[{"type":"frame","id":"hero","name":"Hero","children":[]}]"#.into(),
            ..input()
        });
        let id1 = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            _ => panic!(),
        };
        p.on_delta(id1, "not json at all");
        p.on_complete(id1);
        let reqs = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("{other:?}"),
        };
        assert!(reqs[0].user_message.contains("ONLY valid JSON"));
        let id2 = reqs[0].id;
        p.on_delta(id2, "still not json");
        p.on_complete(id2);
        let reqs = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("expected fallback chunk dispatch, got {other:?}"),
        };
        assert_eq!(reqs.len(), 1);
        assert_eq!(
            reqs[0].kind,
            RequestKind::Chunk {
                chunk_id: "chunk-1".into()
            }
        );
        assert!(
            reqs[0].user_message.contains("Hero"),
            "fallback plan should preserve the selected node name in the chunk prompt"
        );
    }

    #[test]
    fn assembly_failure_falls_back_to_concatenation() {
        let mut p = CodegenPipeline::new(input());
        let id = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            _ => panic!(),
        };
        p.on_delta(id, "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}");
        p.on_complete(id);
        let cid = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            _ => panic!(),
        };
        p.on_delta(
            cid,
            "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}",
        );
        p.on_complete(cid);
        let a1 = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            _ => panic!(),
        };
        p.on_error(a1, "boom".into());
        let a2 = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            _ => panic!(),
        };
        p.on_error(a2, "boom again".into());
        match p.step() {
            PipelineStep::Done { code, degraded, .. } => {
                assert!(degraded);
                assert!(code.contains("Root"));
            }
            other => panic!("expected degraded Done, got {other:?}"),
        }
    }

    /// Drive `p` through planning with `plan_json`, returning the dispatched
    /// chunk request ids (in order).
    fn run_planning(p: &mut CodegenPipeline, plan_json: &str) -> Vec<RequestId> {
        let id = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            other => panic!("expected planning dispatch, got {other:?}"),
        };
        p.on_delta(id, plan_json);
        p.on_complete(id);
        match p.step() {
            PipelineStep::Dispatch(r) => r.iter().map(|q| q.id).collect(),
            other => panic!("expected chunk dispatch, got {other:?}"),
        }
    }

    // ── FIX 3: parsed-but-poor chunk is Degraded, not Failed ──────────────

    #[test]
    fn chunk_with_code_but_no_contract_ends_degraded_not_failed() {
        let mut p = CodegenPipeline::new(input());
        let chunk_ids = run_planning(
            &mut p,
            "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
        );
        let cid = chunk_ids[0];
        // Non-empty code with NO contract and NO matching component name in
        // the body → validation fails → Degraded (never retried/Failed).
        p.on_delta(cid, "const x = 1; // no component, no contract");
        p.on_complete(cid);
        // Advancing to assembly proves the chunk settled (Degraded) and the
        // pipeline proceeded rather than retrying.
        let asm = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("expected assembly dispatch, got {other:?}"),
        };
        assert_eq!(asm[0].kind, RequestKind::Assembly);
        let prog = p.progress();
        assert_eq!(prog.chunks[0].status, ChunkStatus::Degraded);
    }

    #[test]
    fn chunk_on_error_retries_once_then_fails() {
        let mut p = CodegenPipeline::new(input());
        let chunk_ids = run_planning(
            &mut p,
            "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
        );
        // First on_error → retry (re-dispatch the same chunk).
        p.on_error(chunk_ids[0], "stream broke".into());
        let retry = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("expected retry dispatch, got {other:?}"),
        };
        assert!(matches!(retry[0].kind, RequestKind::Chunk { .. }));
        // Second on_error → Failed. The only chunk failed → assembly has no
        // code → terminal Failed.
        p.on_error(retry[0].id, "stream broke again".into());
        match p.step() {
            PipelineStep::Failed { message } => {
                assert!(message.contains("no code to assemble"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    // ── FIX 4: recursive id resolution selects the nested child ───────────

    #[test]
    fn chunk_node_json_resolves_nested_child_not_whole_tree() {
        let mut input = input();
        // A frame whose child is the chunk's only node_id. The child carries a
        // distinctive name/type so we can confirm slicing (the chunk request's
        // `compact_nodes` strips the raw `id` field, so we assert on content).
        input.nodes_json =
            "[{\"type\":\"frame\",\"id\":\"root\",\"name\":\"RootFrame\",\"children\":[{\"type\":\"text\",\"id\":\"label\",\"name\":\"HelloChild\"}]}]"
                .into();
        let mut p = CodegenPipeline::new(input);
        // Capture the chunk dispatch directly so we can inspect its user msg.
        let plan_id = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            other => panic!("expected planning dispatch, got {other:?}"),
        };
        p.on_delta(plan_id, "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Label\",\"nodeIds\":[\"label\"],\"role\":\"r\",\"suggestedComponentName\":\"Label\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}");
        p.on_complete(plan_id);
        let chunk_reqs = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("expected chunk dispatch, got {other:?}"),
        };
        // The chunk request's user message must carry the CHILD node, not the
        // whole frame tree.
        let msg = &chunk_reqs[0].user_message;
        assert!(msg.contains("HelloChild"));
        assert!(msg.contains("text"));
        // The parent frame must NOT be embedded (we sliced just the child).
        assert!(!msg.contains("RootFrame"));
        assert!(!msg.contains("frame"));
    }

    // ── FIX 5: assembly carries full rootLayout + sharedStyles ────────────

    #[test]
    fn assembly_request_includes_layout_gap_and_shared_style_name() {
        let mut p = CodegenPipeline::new(input());
        let chunk_ids = run_planning(
            &mut p,
            "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[{\"name\":\"brandPrimary\",\"description\":\"main\"}],\"rootLayout\":{\"direction\":\"row\",\"gap\":24,\"responsive\":true}}",
        );
        p.on_delta(
            chunk_ids[0],
            "export default function Root(){ return null }\n---CONTRACT---\n{\"componentName\":\"Root\"}",
        );
        p.on_complete(chunk_ids[0]);
        let asm = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("expected assembly dispatch, got {other:?}"),
        };
        let msg = &asm[0].user_message;
        assert!(msg.contains("\"gap\":24"));
        assert!(msg.contains("\"responsive\":true"));
        assert!(msg.contains("brandPrimary"));
    }

    // ── FIX 6: fallback filters empty-code chunks ─────────────────────────

    #[test]
    fn assembly_fallback_skips_empty_code_chunks() {
        let mut input = input();
        input.nodes_json =
            "[{\"type\":\"frame\",\"id\":\"a\",\"children\":[]},{\"type\":\"frame\",\"id\":\"b\",\"children\":[]}]"
                .into();
        let mut p = CodegenPipeline::new(input);
        // Two independent chunks. `c1` (Good) produces code; `c2` (Bad) errors
        // twice → Failed (empty code).
        let chunk_ids = run_planning(
            &mut p,
            "{\"chunks\":[\
                {\"id\":\"c1\",\"name\":\"Good\",\"nodeIds\":[\"a\"],\"role\":\"r\",\"suggestedComponentName\":\"Good\",\"dependencies\":[]},\
                {\"id\":\"c2\",\"name\":\"Bad\",\"nodeIds\":[\"b\"],\"role\":\"r\",\"suggestedComponentName\":\"Bad\",\"dependencies\":[]}\
             ],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
        );
        assert_eq!(chunk_ids.len(), 2);
        // Chunks dispatch in plan order: chunk_ids[0] = c1 (Good),
        // chunk_ids[1] = c2 (Bad).
        let good_id = chunk_ids[0];
        let bad_id = chunk_ids[1];
        // Resolve Good with code.
        p.on_delta(
            good_id,
            "export default function Good(){}\n---CONTRACT---\n{\"componentName\":\"Good\"}",
        );
        p.on_complete(good_id);
        // Error Bad twice → Failed.
        p.on_error(bad_id, "boom".into());
        // step to re-dispatch Bad's retry
        let retry = match p.step() {
            PipelineStep::Dispatch(r) => r,
            other => panic!("expected Bad retry dispatch, got {other:?}"),
        };
        p.on_error(retry[0].id, "boom2".into());
        // Now assembly dispatches; fail it twice.
        let a1 = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            other => panic!("expected assembly dispatch, got {other:?}"),
        };
        p.on_error(a1, "asm boom".into());
        let a2 = match p.step() {
            PipelineStep::Dispatch(r) => r[0].id,
            other => panic!("expected assembly retry dispatch, got {other:?}"),
        };
        p.on_error(a2, "asm boom2".into());
        match p.step() {
            PipelineStep::Done { code, degraded, .. } => {
                assert!(degraded);
                // The good chunk's code is present...
                assert!(code.contains("function Good"));
                assert!(code.contains("Good (successful)"));
                // ...and the failed chunk contributes NO header-only section.
                assert!(!code.contains("Bad (failed)"));
                assert!(!code.contains("Bad"));
            }
            other => panic!("expected degraded Done, got {other:?}"),
        }
    }
}
