//! In-memory codegen pipeline plan store.
//!
//! Port of the TS live-app store the `/api/mcp/codegen/*` routes share:
//! - `apps/web/server/utils/codegen-plan-store.ts` (createPlan /
//!   submitChunkResult / assemblePlan / cleanPlan, validation, topo sort,
//!   chunk hydration, 30-minute TTL, module-global `plans` map)
//! - `packages/pen-mcp/src/utils/validate-contract.ts` (validateContract)
//!
//! State is process-global (`PLANS`), mirroring TS's module-level
//! `const plans = new Map()` living in the app server process: a plan
//! created by one `codegen_plan` call is visible to later
//! `codegen_submit_chunk` / `codegen_assemble` calls on the same server.
//!
//! ## Documented divergences from TS
//!
//! - **Plan ids**: TS mints `crypto.randomUUID()`. Rust emits the same
//!   UUID-v4 wire format but derives the bits from time + pid + an atomic
//!   counter (splitmix64-mixed). Ids only need uniqueness within this
//!   process's store — same rationale as `mcp_live::make_live_token`.
//! - **Malformed-shape errors**: where TS would crash with a `TypeError`
//!   (e.g. a chunk without a `dependencies` array hits
//!   `chunk.dependencies.length`, a result without `code` hits
//!   `code.includes`, `contract.componentName` on a missing contract),
//!   Rust returns a clean error message instead of mirroring the V8
//!   TypeError text (codegen-plan-store.ts:66, validate-contract.ts:5-14).
//! - **`chunkId` required on submit**: TS `state.results.set(result.chunkId,…)`
//!   silently stores under the key `undefined` when `chunkId` is missing
//!   (codegen-plan-store.ts:254); Rust rejects with a clean error.
//! - **componentName checks are string-only**: TS regex-tests whatever
//!   truthy value `contract.componentName` holds (coercing non-strings);
//!   Rust only checks string names (validate-contract.ts:8).

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

/// TS `TTL_MS = 30 * 60 * 1000` (codegen-plan-store.ts:31).
const TTL_MS: u64 = 30 * 60 * 1000;

/// TS `PlanState` (codegen-plan-store.ts:20-28). `chunks` keeps the raw
/// client chunk objects (key order preserved via serde_json's
/// `preserve_order`) so hydration spreads them back out like TS
/// `{...chunk}` does.
struct PlanState {
    chunks: Vec<Value>,
    nodes: HashMap<String, Value>,
    order: HashMap<String, usize>,
    results: HashMap<String, Value>,
    statuses: HashMap<String, String>,
    last_activity_ms: u64,
}

/// TS module-global `const plans = new Map<string, PlanState>()`.
static PLANS: LazyLock<Mutex<HashMap<String, PlanState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static PLAN_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn lock_plans() -> std::sync::MutexGuard<'static, HashMap<String, PlanState>> {
    PLANS.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// TS `cleanExpired()` — drop plans idle past the TTL. Called from
/// `create_plan` only, like TS (codegen-plan-store.ts:33-38,176).
fn clean_expired(plans: &mut HashMap<String, PlanState>, now: u64) {
    plans.retain(|_, state| now.saturating_sub(state.last_activity_ms) <= TTL_MS);
}

/// splitmix64 step — deterministic 64-bit mixer.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Mint a UUID-v4-formatted plan id (TS: `randomUUID()`, see module
/// docs for the entropy divergence).
fn mint_plan_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let counter = PLAN_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut seed = nanos ^ (u64::from(std::process::id()) << 32) ^ counter.rotate_left(17);
    let hi = splitmix64(&mut seed);
    let lo = splitmix64(&mut seed);
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&hi.to_be_bytes());
    bytes[8..].copy_from_slice(&lo.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

// --- Chunk field access -------------------------------------------------

fn chunk_id(chunk: &Value) -> Result<&str, String> {
    chunk
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "every plan chunk needs a string id".to_string())
}

fn chunk_dependencies(chunk: &Value) -> Result<&Vec<Value>, String> {
    chunk
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "Chunk {} needs a dependencies array",
                chunk.get("id").and_then(Value::as_str).unwrap_or("?")
            )
        })
}

/// `chunk.nodeIds` as a slice; missing / non-array reads as empty (the
/// "has no nodeIds" validation error then fires — see `validate_plan`).
fn chunk_node_ids(chunk: &Value) -> &[Value] {
    chunk
        .get("nodeIds")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Render a nodeId entry the way a JS template literal would for the
/// common string case; non-strings keep their JSON form.
fn node_id_repr(value: &Value) -> String {
    match value.as_str() {
        Some(s) => s.to_string(),
        None => value.to_string(),
    }
}

// --- Validation (TS validatePlan, codegen-plan-store.ts:42-105) ----------

fn validate_plan(chunks: &[Value], node_index: &HashMap<String, Value>) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    let mut ids: HashSet<&str> = HashSet::new();

    // Duplicate chunkId — and like TS, return early on any duplicates.
    for chunk in chunks {
        let id = chunk_id(chunk).unwrap_or("?");
        if !ids.insert(id) {
            errors.push(format!("Duplicate chunkId: {id}"));
        }
    }
    if !errors.is_empty() {
        return errors;
    }

    for chunk in chunks {
        let id = chunk_id(chunk).unwrap_or("?");
        let node_ids = chunk_node_ids(chunk);
        if node_ids.is_empty() {
            errors.push(format!("Chunk {id} has no nodeIds"));
        }
        for dep in chunk_dependencies(chunk).map(Vec::as_slice).unwrap_or(&[]) {
            let dep_id = dep.as_str().unwrap_or("");
            if !ids.contains(dep_id) {
                errors.push(format!(
                    "Chunk {id} depends on unknown chunk {}",
                    node_id_repr(dep)
                ));
            }
        }
        for node_id in node_ids {
            let found = node_id
                .as_str()
                .is_some_and(|key| node_index.contains_key(key));
            if !found {
                errors.push(format!(
                    "Chunk {id}: node {} not found in document",
                    node_id_repr(node_id)
                ));
            }
        }
    }

    // Circular dependency detection via topological sort (TS :74-103).
    if errors.is_empty() {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for chunk in chunks {
            let id = chunk_id(chunk).unwrap_or("?");
            let deps = chunk_dependencies(chunk).map(Vec::as_slice).unwrap_or(&[]);
            in_degree.insert(id, deps.len());
            for dep in deps {
                adj.entry(dep.as_str().unwrap_or("")).or_default().push(id);
            }
        }
        let mut queue: VecDeque<&str> = chunks
            .iter()
            .filter(|c| {
                chunk_dependencies(c)
                    .map(|deps| deps.is_empty())
                    .unwrap_or(false)
            })
            .filter_map(|c| chunk_id(c).ok())
            .collect();
        let mut processed = 0usize;
        while let Some(id) = queue.pop_front() {
            processed += 1;
            for next in adj.get(id).map(Vec::as_slice).unwrap_or(&[]) {
                let deg = in_degree.get(next).copied().unwrap_or(1).saturating_sub(1);
                in_degree.insert(next, deg);
                if deg == 0 {
                    queue.push_back(next);
                }
            }
        }
        if processed < chunks.len() {
            let cycle_ids: Vec<&str> = chunks
                .iter()
                .filter_map(|c| chunk_id(c).ok())
                .filter(|id| in_degree.get(id).copied().unwrap_or(0) > 0)
                .collect();
            errors.push(format!("Circular dependency: {}", cycle_ids.join(" → ")));
        }
    }

    errors
}

/// TS `detectWarnings` (codegen-plan-store.ts:107-125): warn for any node
/// claimed by more than one chunk, in node-first-seen order.
fn detect_warnings(chunks: &[Value]) -> Vec<String> {
    let mut node_order: Vec<String> = Vec::new();
    let mut node_to_chunks: HashMap<String, Vec<&str>> = HashMap::new();
    for chunk in chunks {
        let id = chunk_id(chunk).unwrap_or("?");
        for node_id in chunk_node_ids(chunk) {
            let key = node_id_repr(node_id);
            let entry = node_to_chunks.entry(key.clone()).or_insert_with(|| {
                node_order.push(key);
                Vec::new()
            });
            entry.push(id);
        }
    }
    node_order
        .iter()
        .filter_map(|node_id| {
            let chunk_ids = node_to_chunks.get(node_id)?;
            (chunk_ids.len() > 1)
                .then(|| format!("Node {node_id} claimed by chunks: {}", chunk_ids.join(", ")))
        })
        .collect()
}

// --- Node indexing (TS indexNodes, codegen-plan-store.ts:129-141) --------

fn index_nodes(nodes: &[Value]) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    fn walk(list: &[Value], map: &mut HashMap<String, Value>) {
        for node in list {
            if let Some(id) = node.get("id").and_then(Value::as_str) {
                map.insert(id.to_string(), node.clone());
            }
            if let Some(children) = node.get("children").and_then(Value::as_array) {
                walk(children, map);
            }
        }
    }
    walk(nodes, &mut map);
    map
}

// --- Topological sort (TS topoSort, codegen-plan-store.ts:145-171) -------

fn topo_sort(chunks: &[Value]) -> Vec<&Value> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut by_id: HashMap<&str, &Value> = HashMap::new();

    for chunk in chunks {
        let id = chunk_id(chunk).unwrap_or("?");
        let deps = chunk_dependencies(chunk).map(Vec::as_slice).unwrap_or(&[]);
        by_id.insert(id, chunk);
        in_degree.insert(id, deps.len());
        for dep in deps {
            adj.entry(dep.as_str().unwrap_or("")).or_default().push(id);
        }
    }

    let mut result: Vec<&Value> = Vec::new();
    let mut queue: VecDeque<&str> = chunks
        .iter()
        .filter(|c| {
            chunk_dependencies(c)
                .map(|deps| deps.is_empty())
                .unwrap_or(false)
        })
        .filter_map(|c| chunk_id(c).ok())
        .collect();

    while let Some(id) = queue.pop_front() {
        if let Some(chunk) = by_id.get(id) {
            result.push(chunk);
        }
        for next in adj.get(id).map(Vec::as_slice).unwrap_or(&[]) {
            let deg = in_degree.get(next).copied().unwrap_or(1).saturating_sub(1);
            in_degree.insert(next, deg);
            if deg == 0 {
                queue.push_back(next);
            }
        }
    }
    result
}

// --- Chunk hydration (TS hydrateChunk, codegen-plan-store.ts:175-196) ----

/// `{...chunk, nodes, order, depContracts}` — non-full hydration replaces
/// `children` with the literal `"..."` on every node (even leaves), like
/// the TS spread `{ ...n, children: '...' }`.
fn hydrate_chunk(
    chunk: &Value,
    order: usize,
    node_index: &HashMap<String, Value>,
    dep_contracts: Vec<Value>,
    full_hydrate: bool,
) -> Value {
    let nodes: Vec<Value> = chunk_node_ids(chunk)
        .iter()
        .filter_map(|id| id.as_str().and_then(|key| node_index.get(key)))
        .map(|node| {
            if full_hydrate {
                node.clone()
            } else {
                let mut snap = node.as_object().cloned().unwrap_or_default();
                snap.insert("children".into(), json!("..."));
                Value::Object(snap)
            }
        })
        .collect();

    let mut payload = chunk.as_object().cloned().unwrap_or_default();
    payload.insert("nodes".into(), Value::Array(nodes));
    payload.insert("order".into(), json!(order));
    payload.insert("depContracts".into(), Value::Array(dep_contracts));
    Value::Object(payload)
}

// --- Contract validation (TS validateContract) ----------------------------

/// `^[A-Z][a-zA-Z0-9]*$` (validate-contract.ts:8) without pulling regex in.
fn is_pascal_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// Port of `validateContract(result)` → `{valid, issues}`. `Err` covers
/// the result shapes where TS would crash with a TypeError (see module
/// docs).
pub(crate) fn validate_contract(result: &Value) -> Result<Value, String> {
    if !result.is_object() {
        return Err("result must be an object".into());
    }
    let Some(code) = result.get("code").and_then(Value::as_str) else {
        return Err("result.code must be a string".into());
    };
    let Some(contract) = result.get("contract").filter(|c| c.is_object()) else {
        return Err("result.contract must be an object".into());
    };

    let mut issues: Vec<String> = Vec::new();
    let component_name = contract
        .get("componentName")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty());

    // 1. componentName must be a valid PascalCase identifier (if provided).
    if let Some(name) = component_name {
        if !is_pascal_case(name) {
            issues.push(format!(
                "componentName \"{name}\" is not a valid PascalCase identifier"
            ));
        }
    }

    // 2. componentName should appear in code (skip for SFC frameworks
    //    where the name is implicit).
    let is_sfc = code.contains("<script") || code.contains("<template") || code.contains("<style");
    if let Some(name) = component_name {
        if !is_sfc && !code.contains(name) {
            issues.push(format!(
                "componentName \"{name}\" not found in generated code"
            ));
        }
    }

    Ok(json!({ "valid": issues.is_empty(), "issues": issues }))
}

// --- Public API ------------------------------------------------------------

/// TS `createPlan(plan, allNodes)` → `{planId, executionPlan, warnings}`.
/// `Err` carries the validation errors joined with `"; "` (the message the
/// TS route 400s with).
pub(crate) fn create_plan(plan: &Value, all_nodes: &[Value]) -> Result<Value, String> {
    {
        let mut plans = lock_plans();
        clean_expired(&mut plans, now_ms());
    }

    let chunks = plan
        .get("chunks")
        .and_then(Value::as_array)
        .ok_or_else(|| "plan.chunks must be an array".to_string())?;
    // Structural normalization Rust needs up front (TS would TypeError
    // mid-validation on these shapes — see module docs).
    for chunk in chunks {
        chunk_id(chunk)?;
        chunk_dependencies(chunk)?;
    }

    let node_index = index_nodes(all_nodes);
    let errors = validate_plan(chunks, &node_index);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    let warnings = detect_warnings(chunks);
    let sorted = topo_sort(chunks);
    let mut order_map: HashMap<String, usize> = HashMap::new();
    for (i, chunk) in sorted.iter().enumerate() {
        order_map.insert(chunk_id(chunk).unwrap_or("?").to_string(), i);
    }

    let plan_id = mint_plan_id();
    // Hydrate before moving the index into the store. Only the first chunk
    // in topo order is fully hydrated (TS: `i === 0`); the rest get the
    // `children: "..."` snapshots.
    let execution_plan: Vec<Value> = sorted
        .iter()
        .enumerate()
        .map(|(i, chunk)| hydrate_chunk(chunk, i, &node_index, Vec::new(), i == 0))
        .collect();

    let mut statuses: HashMap<String, String> = HashMap::new();
    for chunk in chunks {
        statuses.insert(chunk_id(chunk).unwrap_or("?").to_string(), "pending".into());
    }
    let state = PlanState {
        chunks: chunks.clone(),
        nodes: node_index,
        order: order_map,
        results: HashMap::new(),
        statuses,
        last_activity_ms: now_ms(),
    };
    lock_plans().insert(plan_id.clone(), state);

    Ok(json!({
        "planId": plan_id,
        "executionPlan": execution_plan,
        "warnings": warnings,
    }))
}

/// TS `submitChunkResult(planId, result, statusOverride)` →
/// `{validation, progress, nextChunk?}`.
pub(crate) fn submit_chunk_result(
    plan_id: &str,
    result: &Value,
    status_override: Option<&str>,
) -> Result<Value, String> {
    let mut plans = lock_plans();
    let state = plans
        .get_mut(plan_id)
        .ok_or_else(|| format!("Plan {plan_id} not found"))?;
    state.last_activity_ms = now_ms(); // TS touch(planId)

    let validation = validate_contract(result)?;
    let valid = validation
        .get("valid")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let result_chunk_id = result
        .get("chunkId")
        .and_then(Value::as_str)
        .ok_or_else(|| "result.chunkId must be a string".to_string())?;

    let status = match status_override {
        Some("failed") => "failed",
        Some("skipped") => "skipped",
        _ if valid => "done",
        _ => "degraded",
    };
    state
        .results
        .insert(result_chunk_id.to_string(), result.clone());
    state
        .statuses
        .insert(result_chunk_id.to_string(), status.to_string());

    // progress over the ORIGINAL chunk order (TS maps state.plan.chunks).
    let progress: Vec<Value> = state
        .chunks
        .iter()
        .map(|c| {
            let id = chunk_id(c).unwrap_or("?");
            let mut entry = Map::new();
            entry.insert("step".into(), json!("chunk"));
            entry.insert("chunkId".into(), json!(id));
            // JSON.stringify drops `name: undefined`; an explicit null is kept.
            if let Some(name) = c.get("name") {
                entry.insert("name".into(), name.clone());
            }
            entry.insert(
                "status".into(),
                json!(state
                    .statuses
                    .get(id)
                    .map(String::as_str)
                    .unwrap_or("pending")),
            );
            if let Some(result) = state.results.get(id) {
                entry.insert("result".into(), result.clone());
            }
            Value::Object(entry)
        })
        .collect();

    // nextChunk: first pending chunk (in topo order) whose deps all settled.
    let mut next_chunk: Option<Value> = None;
    for chunk in topo_sort(&state.chunks) {
        let id = chunk_id(chunk).unwrap_or("?");
        if state.statuses.get(id).map(String::as_str) != Some("pending") {
            continue;
        }
        let deps = chunk_dependencies(chunk).map(Vec::as_slice).unwrap_or(&[]);
        let deps_ready = deps.iter().all(|dep| {
            matches!(
                state
                    .statuses
                    .get(dep.as_str().unwrap_or(""))
                    .map(String::as_str),
                Some("done") | Some("degraded") | Some("failed") | Some("skipped")
            )
        });
        if deps_ready {
            let dep_contracts: Vec<Value> = deps
                .iter()
                .map(|dep| {
                    let dep_id = dep.as_str().unwrap_or("");
                    let dep_status = state.statuses.get(dep_id).map(String::as_str);
                    if matches!(dep_status, Some("failed") | Some("skipped")) {
                        return Value::Null;
                    }
                    state
                        .results
                        .get(dep_id)
                        .and_then(|r| r.get("contract"))
                        .cloned()
                        .unwrap_or(Value::Null)
                })
                .collect();
            let order = state.order.get(id).copied().unwrap_or(0);
            next_chunk = Some(hydrate_chunk(
                chunk,
                order,
                &state.nodes,
                dep_contracts,
                true,
            ));
            break;
        }
    }

    let mut out = Map::new();
    out.insert("validation".into(), validation);
    out.insert("progress".into(), Value::Array(progress));
    if let Some(next) = next_chunk {
        out.insert("nextChunk".into(), next); // dropped when undefined in TS
    }
    Ok(Value::Object(out))
}

/// TS `assemblePlan(planId, framework)` → `{chunks, contracts,
/// dependencyGraph, degraded}`. Terminal — removes the plan from the store.
/// `framework` is accepted but unused, mirroring TS `_framework`.
pub(crate) fn assemble_plan(plan_id: &str, _framework: &str) -> Result<Value, String> {
    let mut plans = lock_plans();
    let state = plans
        .remove(plan_id)
        .ok_or_else(|| format!("Plan {plan_id} not found"))?;

    let mut chunks_out: Vec<Value> = Vec::new();
    let mut contracts: Vec<Value> = Vec::new();
    let mut dependency_graph = Map::new();
    let mut degraded = false;

    for chunk in topo_sort(&state.chunks) {
        let id = chunk_id(chunk).unwrap_or("?");
        if let Some(result) = state.results.get(id) {
            chunks_out.push(result.clone());
            // TS pushes `result.contract` even when absent — JSON null.
            contracts.push(result.get("contract").cloned().unwrap_or(Value::Null));
        }
        dependency_graph.insert(
            id.to_string(),
            chunk.get("dependencies").cloned().unwrap_or(json!([])),
        );
        if state.statuses.get(id).map(String::as_str) != Some("done") {
            degraded = true;
        }
    }

    Ok(json!({
        "chunks": chunks_out,
        "contracts": contracts,
        "dependencyGraph": dependency_graph,
        "degraded": degraded,
    }))
}

/// TS `cleanPlan(planId)` → `{ok, deleted}`. Idempotent.
pub(crate) fn clean_plan(plan_id: &str) -> Value {
    let existed = lock_plans().remove(plan_id).is_some();
    json!({ "ok": true, "deleted": existed })
}
