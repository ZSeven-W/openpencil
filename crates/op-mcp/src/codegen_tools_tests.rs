//! TS-compatible codegen MCP tool tests — mirrors the behavior of the TS
//! plan store (`apps/web/server/utils/codegen-plan-store.ts`), the codegen
//! routes, and `packages/pen-mcp/src/__tests__/validate-contract.test.ts`.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::codegen_plan_store::validate_contract;
use super::test_fixtures::{rect, sample, state_with};
use super::{
    codegen_assemble_snapshot, codegen_clean_snapshot, codegen_plan_snapshot,
    codegen_submit_chunk_snapshot, McpTool, ToolErrorCode, ToolOutcome,
};

fn args_with(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// A valid two-chunk plan over the `sample()` fixture: `b` (Button → n12)
/// depends on `a` (Title → n11). Listed b-first so topo sorting is visible.
fn sample_plan_json() -> String {
    json!({
        "chunks": [
            {
                "id": "b",
                "name": "Button",
                "nodeIds": ["n12"],
                "role": "button",
                "suggestedComponentName": "ButtonChunk",
                "dependencies": ["a"]
            },
            {
                "id": "a",
                "name": "Title",
                "nodeIds": ["n11"],
                "role": "title",
                "suggestedComponentName": "TitleChunk",
                "dependencies": []
            }
        ],
        "sharedStyles": [],
        "rootLayout": { "direction": "column", "gap": 8, "responsive": true }
    })
    .to_string()
}

fn chunk_result_json(chunk_id: &str, component: &str) -> String {
    json!({
        "chunkId": chunk_id,
        "code": format!("export function {component}() {{ return null; }}"),
        "contract": {
            "chunkId": chunk_id,
            "componentName": component,
            "exportedProps": [],
            "slots": [],
            "cssClasses": [],
            "cssVariables": [],
            "imports": []
        }
    })
    .to_string()
}

fn expect_json(outcome: ToolOutcome) -> Value {
    match outcome {
        ToolOutcome::OkJson(raw) => serde_json::from_str(&raw).expect("tool emitted valid JSON"),
        other => panic!("expected OkJson, got {other:?}"),
    }
}

fn expect_err(outcome: ToolOutcome) -> (ToolErrorCode, String) {
    match outcome {
        ToolOutcome::Err(code, message) => (code, message),
        other => panic!("expected Err, got {other:?}"),
    }
}

/// Create a plan over `sample()` and return its planId.
fn create_sample_plan() -> String {
    let state = sample();
    let tool = codegen_plan_snapshot(&state);
    let out = expect_json(tool.call(&args_with(&[("plan", &sample_plan_json())])));
    out["planId"].as_str().expect("planId").to_string()
}

// --- codegen_plan ----------------------------------------------------------

#[test]
fn codegen_plan_returns_topo_sorted_execution_plan_with_hydration() {
    let state = sample();
    let tool = codegen_plan_snapshot(&state);
    match tool.call(&args_with(&[("plan", &sample_plan_json())])) {
        ToolOutcome::OkJson(raw) => {
            // TS pretty-prints with JSON.stringify(result, null, 2).
            assert!(raw.contains("\n  \"planId\""), "pretty-printed: {raw}");
            let out: Value = serde_json::from_str(&raw).expect("json");
            // randomUUID wire format: 8-4-4-4-12.
            let plan_id = out["planId"].as_str().expect("planId");
            assert_eq!(plan_id.len(), 36, "{plan_id}");
            assert!(
                plan_id.as_bytes()[8] == b'-'
                    && plan_id.as_bytes()[13] == b'-'
                    && plan_id.as_bytes()[18] == b'-'
                    && plan_id.as_bytes()[23] == b'-',
                "{plan_id}"
            );
            assert_eq!(out["warnings"], json!([]));
            let plan = out["executionPlan"].as_array().expect("executionPlan");
            assert_eq!(plan.len(), 2);
            // Topo order: dependency-free `a` first even though the client
            // listed `b` first.
            assert_eq!(plan[0]["id"], "a");
            assert_eq!(plan[0]["order"], json!(0));
            assert_eq!(plan[0]["depContracts"], json!([]));
            assert_eq!(plan[1]["id"], "b");
            assert_eq!(plan[1]["order"], json!(1));
            // First chunk in topo order is fully hydrated: the Text leaf
            // keeps its natural shape (no synthetic children marker).
            let a_node = &plan[0]["nodes"][0];
            assert_eq!(a_node["id"], "n11");
            assert!(a_node.get("children").is_none(), "{a_node}");
            // Later chunks get `children: "..."` snapshots — even though
            // the Group really has children (TS `{ ...n, children: '...' }`).
            let b_node = &plan[1]["nodes"][0];
            assert_eq!(b_node["id"], "n12");
            assert_eq!(b_node["children"], json!("..."));
            // Chunk fields ride into the payload (TS spread).
            assert_eq!(plan[1]["suggestedComponentName"], "ButtonChunk");
            assert_eq!(plan[1]["dependencies"], json!(["a"]));
            // Cleanup the process-global store.
            let _ = codegen_clean_snapshot().call(&args_with(&[("planId", plan_id)]));
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn codegen_plan_rejects_duplicate_chunk_ids_before_other_checks() {
    let state = sample();
    let tool = codegen_plan_snapshot(&state);
    // The duplicated plan ALSO has an unknown dependency; TS returns after
    // the duplicate check so only the duplicate error surfaces.
    let plan = json!({
        "chunks": [
            { "id": "a", "nodeIds": ["n11"], "dependencies": ["ghost"] },
            { "id": "a", "nodeIds": ["n12"], "dependencies": [] }
        ],
        "sharedStyles": [],
        "rootLayout": { "direction": "column", "gap": 0, "responsive": false }
    })
    .to_string();
    let (code, message) = expect_err(tool.call(&args_with(&[("plan", &plan)])));
    assert_eq!(code, ToolErrorCode::ToolFailed);
    assert_eq!(message, "codegen_plan failed: Duplicate chunkId: a");
}

#[test]
fn codegen_plan_collects_node_ids_dependency_and_missing_node_errors() {
    let state = sample();
    let tool = codegen_plan_snapshot(&state);
    let plan = json!({
        "chunks": [
            { "id": "a", "nodeIds": [], "dependencies": ["ghost"] },
            { "id": "b", "nodeIds": ["nope"], "dependencies": [] }
        ],
        "sharedStyles": [],
        "rootLayout": { "direction": "column", "gap": 0, "responsive": false }
    })
    .to_string();
    let (_, message) = expect_err(tool.call(&args_with(&[("plan", &plan)])));
    assert_eq!(
        message,
        "codegen_plan failed: Chunk a has no nodeIds; \
         Chunk a depends on unknown chunk ghost; \
         Chunk b: node nope not found in document"
    );
}

#[test]
fn codegen_plan_rejects_circular_dependencies() {
    let state = sample();
    let tool = codegen_plan_snapshot(&state);
    let plan = json!({
        "chunks": [
            { "id": "a", "nodeIds": ["n11"], "dependencies": ["b"] },
            { "id": "b", "nodeIds": ["n12"], "dependencies": ["a"] }
        ],
        "sharedStyles": [],
        "rootLayout": { "direction": "column", "gap": 0, "responsive": false }
    })
    .to_string();
    let (_, message) = expect_err(tool.call(&args_with(&[("plan", &plan)])));
    assert_eq!(message, "codegen_plan failed: Circular dependency: a → b");
}

#[test]
fn codegen_plan_warns_when_chunks_share_node_ids() {
    let state = sample();
    let tool = codegen_plan_snapshot(&state);
    let plan = json!({
        "chunks": [
            { "id": "a", "nodeIds": ["n11"], "dependencies": [] },
            { "id": "b", "nodeIds": ["n11"], "dependencies": [] }
        ],
        "sharedStyles": [],
        "rootLayout": { "direction": "column", "gap": 0, "responsive": false }
    })
    .to_string();
    let out = expect_json(tool.call(&args_with(&[("plan", &plan)])));
    assert_eq!(out["warnings"], json!(["Node n11 claimed by chunks: a, b"]));
    let plan_id = out["planId"].as_str().expect("planId");
    let _ = codegen_clean_snapshot().call(&args_with(&[("planId", plan_id)]));
}

#[test]
fn codegen_plan_missing_plan_reports_route_message() {
    let state = sample();
    let tool = codegen_plan_snapshot(&state);
    let (code, message) = expect_err(tool.call(&BTreeMap::new()));
    assert_eq!(code, ToolErrorCode::ToolFailed);
    assert_eq!(message, "codegen_plan failed: Missing plan in request body");
}

#[test]
fn codegen_plan_defaults_to_first_page_and_honors_page_id() {
    use jian_ops_schema::page::PenPage;
    let mut state = state_with(vec![]);
    state.doc.pages = Some(vec![
        PenPage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![rect("p1n1", "First", 0.0, 0.0, 10.0, 10.0)],
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "page-2".into(),
            name: "Page 2".into(),
            children: vec![rect("p2n1", "Second", 0.0, 0.0, 10.0, 10.0)],
            state: None,
            lifecycle: None,
        },
    ]);
    // Active page is page-2, but the TS plan route resolves
    // `getActivePageChildren(doc, pageId ?? null)` → FIRST page by default.
    state.ui.active_page_index = 1;
    let tool = codegen_plan_snapshot(&state);

    let plan_for = |node_id: &str| {
        json!({
            "chunks": [ { "id": "a", "nodeIds": [node_id], "dependencies": [] } ],
            "sharedStyles": [],
            "rootLayout": { "direction": "column", "gap": 0, "responsive": false }
        })
        .to_string()
    };

    // Default page = first page → p1n1 resolves, p2n1 does not.
    let out = expect_json(tool.call(&args_with(&[("plan", &plan_for("p1n1"))])));
    let plan_id = out["planId"].as_str().expect("planId").to_string();
    let _ = codegen_clean_snapshot().call(&args_with(&[("planId", &plan_id)]));
    let (_, message) = expect_err(tool.call(&args_with(&[("plan", &plan_for("p2n1"))])));
    assert_eq!(
        message,
        "codegen_plan failed: Chunk a: node p2n1 not found in document"
    );

    // Explicit pageId targets that page.
    let out = expect_json(tool.call(&args_with(&[
        ("plan", &plan_for("p2n1")),
        ("pageId", "page-2"),
    ])));
    let plan_id = out["planId"].as_str().expect("planId").to_string();
    let _ = codegen_clean_snapshot().call(&args_with(&[("planId", &plan_id)]));
}

// --- codegen_submit_chunk --------------------------------------------------

#[test]
fn codegen_submit_chunk_returns_validation_progress_and_next_chunk() {
    let plan_id = create_sample_plan();
    let tool = codegen_submit_chunk_snapshot();
    let out = expect_json(tool.call(&args_with(&[
        ("planId", &plan_id),
        ("result", &chunk_result_json("a", "TitleChunk")),
    ])));

    assert_eq!(out["validation"], json!({ "valid": true, "issues": [] }));

    // Progress follows the ORIGINAL plan.chunks order (b first).
    let progress = out["progress"].as_array().expect("progress");
    assert_eq!(progress.len(), 2);
    assert_eq!(progress[0]["step"], "chunk");
    assert_eq!(progress[0]["chunkId"], "b");
    assert_eq!(progress[0]["name"], "Button");
    assert_eq!(progress[0]["status"], "pending");
    assert!(progress[0].get("result").is_none(), "{}", progress[0]);
    assert_eq!(progress[1]["chunkId"], "a");
    assert_eq!(progress[1]["status"], "done");
    assert_eq!(progress[1]["result"]["chunkId"], "a");

    // nextChunk: b is now ready, fully hydrated, with a's contract.
    let next = &out["nextChunk"];
    assert_eq!(next["id"], "b");
    assert_eq!(next["order"], json!(1));
    assert_eq!(next["depContracts"][0]["componentName"], "TitleChunk");
    // Full hydration: the Group node carries its REAL children.
    assert!(next["nodes"][0]["children"].is_array(), "{next}");

    let _ = codegen_clean_snapshot().call(&args_with(&[("planId", &plan_id)]));
}

#[test]
fn codegen_submit_chunk_degraded_when_component_name_missing_from_code() {
    let plan_id = create_sample_plan();
    let tool = codegen_submit_chunk_snapshot();
    let bad = json!({
        "chunkId": "a",
        "code": "export default function () { return null; }",
        "contract": { "componentName": "TitleChunk" }
    })
    .to_string();
    let out = expect_json(tool.call(&args_with(&[("planId", &plan_id), ("result", &bad)])));
    assert_eq!(out["validation"]["valid"], json!(false));
    assert_eq!(
        out["validation"]["issues"][0],
        "componentName \"TitleChunk\" not found in generated code"
    );
    // Invalid contract → status degraded (still settles the dependency).
    let progress = out["progress"].as_array().expect("progress");
    assert_eq!(progress[1]["chunkId"], "a");
    assert_eq!(progress[1]["status"], "degraded");
    let _ = codegen_clean_snapshot().call(&args_with(&[("planId", &plan_id)]));
}

#[test]
fn codegen_submit_chunk_status_override_passes_null_contract_downstream() {
    let plan_id = create_sample_plan();
    let tool = codegen_submit_chunk_snapshot();
    let out = expect_json(tool.call(&args_with(&[
        ("planId", &plan_id),
        ("result", &chunk_result_json("a", "TitleChunk")),
        ("status", "failed"),
    ])));
    let progress = out["progress"].as_array().expect("progress");
    assert_eq!(progress[1]["status"], "failed");
    // Failed dep → depContracts entry is null (TS ResolvedDepContract).
    assert_eq!(out["nextChunk"]["id"], "b");
    assert_eq!(out["nextChunk"]["depContracts"], json!([null]));
    let _ = codegen_clean_snapshot().call(&args_with(&[("planId", &plan_id)]));
}

#[test]
fn codegen_submit_chunk_unknown_plan_reports_not_found() {
    let tool = codegen_submit_chunk_snapshot();
    let (code, message) = expect_err(tool.call(&args_with(&[
        ("planId", "nope"),
        ("result", &chunk_result_json("a", "TitleChunk")),
    ])));
    assert_eq!(code, ToolErrorCode::ToolFailed);
    assert_eq!(message, "codegen_submit_chunk failed: Plan nope not found");
}

#[test]
fn codegen_submit_chunk_missing_args_report_route_message() {
    let tool = codegen_submit_chunk_snapshot();
    let (_, message) = expect_err(tool.call(&args_with(&[("planId", "p")])));
    assert_eq!(
        message,
        "codegen_submit_chunk failed: Missing planId or result"
    );
}

// --- codegen_assemble ------------------------------------------------------

#[test]
fn codegen_assemble_returns_topo_results_and_clears_the_plan() {
    let plan_id = create_sample_plan();
    let submit = codegen_submit_chunk_snapshot();
    let _ = submit.call(&args_with(&[
        ("planId", &plan_id),
        ("result", &chunk_result_json("a", "TitleChunk")),
    ]));
    let _ = submit.call(&args_with(&[
        ("planId", &plan_id),
        ("result", &chunk_result_json("b", "ButtonChunk")),
    ]));

    let tool = codegen_assemble_snapshot();
    let out = expect_json(tool.call(&args_with(&[("planId", &plan_id), ("framework", "react")])));
    let chunks = out["chunks"].as_array().expect("chunks");
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0]["chunkId"], "a"); // topological order
    assert_eq!(chunks[1]["chunkId"], "b");
    assert_eq!(out["contracts"][0]["componentName"], "TitleChunk");
    assert_eq!(out["contracts"][1]["componentName"], "ButtonChunk");
    assert_eq!(out["dependencyGraph"], json!({ "a": [], "b": ["a"] }));
    assert_eq!(out["degraded"], json!(false));

    // Terminal operation — the plan cache is cleared.
    let (_, message) =
        expect_err(tool.call(&args_with(&[("planId", &plan_id), ("framework", "react")])));
    assert_eq!(
        message,
        format!("codegen_assemble failed: Plan {plan_id} not found")
    );
}

#[test]
fn codegen_assemble_marks_degraded_when_chunks_are_not_done() {
    let plan_id = create_sample_plan();
    let submit = codegen_submit_chunk_snapshot();
    let _ = submit.call(&args_with(&[
        ("planId", &plan_id),
        ("result", &chunk_result_json("a", "TitleChunk")),
    ]));
    // b never submitted → pending → degraded; framework omitted defaults
    // to react (TS route: `query.framework || 'react'`).
    let out = expect_json(codegen_assemble_snapshot().call(&args_with(&[("planId", &plan_id)])));
    assert_eq!(out["degraded"], json!(true));
    assert_eq!(out["chunks"].as_array().expect("chunks").len(), 1);
    // dependencyGraph still lists every chunk.
    assert_eq!(out["dependencyGraph"]["b"], json!(["a"]));
}

// --- codegen_clean ---------------------------------------------------------

#[test]
fn codegen_clean_reports_deleted_flag_and_is_idempotent() {
    let plan_id = create_sample_plan();
    let tool = codegen_clean_snapshot();
    let out = expect_json(tool.call(&args_with(&[("planId", &plan_id)])));
    assert_eq!(out, json!({ "ok": true, "deleted": true }));
    let out = expect_json(tool.call(&args_with(&[("planId", &plan_id)])));
    assert_eq!(out, json!({ "ok": true, "deleted": false }));
}

// --- validateContract (packages/pen-mcp/src/__tests__/validate-contract.test.ts)

fn make_result(code: &str, component_name: &str) -> Value {
    json!({
        "chunkId": "c1",
        "code": code,
        "contract": {
            "chunkId": "c1",
            "componentName": component_name,
            "exportedProps": [],
            "slots": [],
            "cssClasses": [],
            "cssVariables": [],
            "imports": []
        }
    })
}

#[test]
fn validate_contract_passes_valid_contract() {
    let result = make_result("function HeroSection() { return <div />; }", "HeroSection");
    let out = validate_contract(&result).expect("validates");
    assert_eq!(out, json!({ "valid": true, "issues": [] }));
}

#[test]
fn validate_contract_rejects_invalid_pascal_case_component_name() {
    let result = make_result("function HeroSection() { return <div />; }", "hero-section");
    let out = validate_contract(&result).expect("validates");
    assert_eq!(out["valid"], json!(false));
    assert_eq!(
        out["issues"][0],
        "componentName \"hero-section\" is not a valid PascalCase identifier"
    );
}

#[test]
fn validate_contract_warns_when_component_name_not_in_code() {
    let result = make_result("export default function() {}", "HeroSection");
    let out = validate_contract(&result).expect("validates");
    assert_eq!(out["valid"], json!(false));
    assert_eq!(
        out["issues"][0],
        "componentName \"HeroSection\" not found in generated code"
    );
}

#[test]
fn validate_contract_skips_component_name_check_for_sfc_frameworks() {
    let result = make_result(
        "<template><div></div></template><script>export default {}</script>",
        "HeroSection",
    );
    let out = validate_contract(&result).expect("validates");
    assert_eq!(out["valid"], json!(true));
}
