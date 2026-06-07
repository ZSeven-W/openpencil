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
    /// Parsed Value of the sanitized nodes, for per-chunk subset slicing.
    sanitized_nodes_value: Value,
    /// Set of node ids present in the sanitized tree (for hydration).
    node_id_index: std::collections::HashSet<String>,
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
        let mut node_id_index = std::collections::HashSet::new();
        collect_node_ids(&sanitized_nodes_value, &mut node_id_index);

        Self {
            input,
            sanitized_nodes_json,
            sanitized_nodes_value,
            node_id_index,
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
            return self.handle_planning_failure("Planning failed".to_string());
        };

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
                    .any(|id| self.node_id_index.contains(id))
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

        // Empty code is treated like a failure (TS retries an empty/throwing
        // call). If retry already used, the chunk fails.
        if result.code.is_empty() {
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

    /// Serialize the subset of top-level sanitized nodes whose `id` is in
    /// `node_ids` (the simple approach the task spec calls for).
    fn chunk_nodes_json(&self, node_ids: &[String]) -> String {
        let wanted: std::collections::HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
        if let Value::Array(items) = &self.sanitized_nodes_value {
            let subset: Vec<&Value> = items
                .iter()
                .filter(|item| {
                    item.get("id")
                        .and_then(Value::as_str)
                        .map(|id| wanted.contains(id))
                        .unwrap_or(false)
                })
                .collect();
            if !subset.is_empty() {
                return serde_json::to_string(&subset)
                    .unwrap_or_else(|_| self.sanitized_nodes_json.clone());
            }
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
                // Second failure → best-effort fallback to concatenation.
                self.assembly_done = Some(false);
                let code = self.build_chunk_blocks();
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

    /// `"// ── {name} ({status}) ──\n\n{code}"` per chunk, joined by "\n\n".
    /// Skipped / failed chunks contribute empty code. Mirrors the TS
    /// assembly fallback concatenation block.
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
                format!("// ── {name} ({status}) ──\n\n{code}")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn build_plan_summary(&self) -> String {
        let direction = self
            .plan
            .as_ref()
            .map(|p| p.root_layout.direction.as_str())
            .filter(|d| !d.is_empty())
            .unwrap_or("column");
        format!(
            "Root layout direction: {direction}. Chunk count: {}.",
            self.chunks.len()
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

/// Walk the node JSON Value tree collecting every `id` field, so hydration
/// can drop planned chunks that reference no real node (TS: `indexNodes`).
fn collect_node_ids(value: &Value, out: &mut std::collections::HashSet<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_node_ids(item, out);
            }
        }
        Value::Object(map) => {
            if let Some(id) = map.get("id").and_then(Value::as_str) {
                out.insert(id.to_string());
            }
            for v in map.values() {
                collect_node_ids(v, out);
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
    fn planning_parse_failure_retries_once_then_fails() {
        let mut p = CodegenPipeline::new(input());
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
        assert!(matches!(p.step(), PipelineStep::Failed { .. }));
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
}
