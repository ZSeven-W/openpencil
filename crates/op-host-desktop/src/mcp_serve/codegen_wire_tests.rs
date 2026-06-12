//! End-to-end wire tests for the codegen MCP workflow
//! (`codegen_plan` → `codegen_submit_chunk` → `codegen_assemble` →
//! `codegen_clean`) through the desktop host's `process_message`
//! dispatcher — covering registration, the structured-arg wire parse,
//! the TS `content[].text` envelope (pretty-printed JSON, per
//! `packages/pen-mcp/src/tools/codegen-plan.ts:33` `JSON.stringify(…,
//! null, 2)`), and the in-process plan store integration.
//!
//! Split out of `tests.rs` (already at the 800-line cap).

#![cfg(test)]

use super::*;

/// Editor state with a Text leaf `n1` and a Frame `n2` carrying one
/// child — enough to observe both hydration modes (full vs
/// `children: "..."`).
fn codegen_state() -> EditorState {
    let doc = r##"{
      "version": "1.0.0",
      "children": [
        {
          "id": "n1",
          "type": "text",
          "name": "Title",
          "x": 0,
          "y": 0,
          "width": 200,
          "height": 40,
          "content": "Hello"
        },
        {
          "id": "n2",
          "type": "frame",
          "name": "Button",
          "x": 0,
          "y": 60,
          "width": 120,
          "height": 40,
          "children": [
            {
              "id": "n21",
              "type": "rectangle",
              "name": "ButtonBg",
              "x": 0,
              "y": 0,
              "width": 120,
              "height": 40,
              "fill": [{ "type": "solid", "color": "#3B82F6" }]
            }
          ]
        }
      ]
    }"##;
    let loaded = jian_ops_schema::load_str(doc).expect("parse codegen fixture");
    EditorState::from_document(loaded.value)
}

/// Dispatch one `tools/call` line; codegen tools are read-only so the
/// applier must never fire.
fn dispatch(state: &mut EditorState, line: &str) -> String {
    process_message_with_applier(state, line, |_, _| {
        panic!("codegen tools must not emit editor commands")
    })
    .expect("dispatch")
    .expect("response")
}

fn chunk_result_args(chunk_id: &str, component: &str) -> String {
    format!(
        r#"{{"chunkId":"{chunk_id}","code":"export function {component}() {{ return null; }}","contract":{{"chunkId":"{chunk_id}","componentName":"{component}","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}}}}"#
    )
}

#[test]
fn codegen_workflow_round_trips_over_the_tools_call_wire() {
    let mut state = codegen_state();

    // --- codegen_plan: structured `plan` arg rides the wire ---
    let plan_line = r#"{"jsonrpc":"2.0","id":61,"method":"tools/call","params":{"name":"codegen_plan","arguments":{"plan":{"chunks":[{"id":"b","name":"Button","nodeIds":["n2"],"dependencies":["a"]},{"id":"a","name":"Title","nodeIds":["n1"],"dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":8,"responsive":true}}}}}"#;
    let response = dispatch(&mut state, plan_line);
    assert!(response.contains(r#""id":61"#), "{response}");
    let text = crate::tool_text(&response);
    // TS tool layer pretty-prints with JSON.stringify(result, null, 2).
    assert!(text.contains("\n  \"planId\""), "pretty-printed: {text}");
    let plan_out: serde_json::Value = serde_json::from_str(&text).expect("plan json");
    let plan_id = plan_out["planId"].as_str().expect("planId").to_string();
    let execution_plan = plan_out["executionPlan"].as_array().expect("executionPlan");
    // Topo order: dependency-free `a` first; later chunks hydrate with
    // `children: "..."` snapshots (TS `{ ...n, children: '...' }`).
    assert_eq!(execution_plan[0]["id"], "a");
    assert_eq!(execution_plan[1]["id"], "b");
    assert_eq!(execution_plan[1]["nodes"][0]["id"], "n2");
    assert_eq!(execution_plan[1]["nodes"][0]["children"], "...");
    assert_eq!(plan_out["warnings"], serde_json::json!([]));

    // --- codegen_submit_chunk: a settles, b becomes nextChunk ---
    let submit_line = format!(
        r#"{{"jsonrpc":"2.0","id":62,"method":"tools/call","params":{{"name":"codegen_submit_chunk","arguments":{{"planId":"{plan_id}","result":{}}}}}}}"#,
        chunk_result_args("a", "TitleChunk")
    );
    let submit_out: serde_json::Value =
        serde_json::from_str(&crate::tool_text(&dispatch(&mut state, &submit_line)))
            .expect("submit json");
    assert_eq!(
        submit_out["validation"],
        serde_json::json!({ "valid": true, "issues": [] })
    );
    assert_eq!(submit_out["nextChunk"]["id"], "b");
    assert_eq!(
        submit_out["nextChunk"]["depContracts"][0]["componentName"],
        "TitleChunk"
    );
    // nextChunk is fully hydrated: the Frame keeps its real children.
    assert!(
        submit_out["nextChunk"]["nodes"][0]["children"].is_array(),
        "{submit_out}"
    );

    let submit_b_line = format!(
        r#"{{"jsonrpc":"2.0","id":63,"method":"tools/call","params":{{"name":"codegen_submit_chunk","arguments":{{"planId":"{plan_id}","result":{}}}}}}}"#,
        chunk_result_args("b", "ButtonChunk")
    );
    let _ = dispatch(&mut state, &submit_b_line);

    // --- codegen_assemble: topo results, terminal ---
    let assemble_line = format!(
        r#"{{"jsonrpc":"2.0","id":64,"method":"tools/call","params":{{"name":"codegen_assemble","arguments":{{"planId":"{plan_id}","framework":"react"}}}}}}"#
    );
    let assemble_out: serde_json::Value =
        serde_json::from_str(&crate::tool_text(&dispatch(&mut state, &assemble_line)))
            .expect("assemble json");
    assert_eq!(assemble_out["chunks"][0]["chunkId"], "a");
    assert_eq!(assemble_out["chunks"][1]["chunkId"], "b");
    assert_eq!(
        assemble_out["dependencyGraph"],
        serde_json::json!({ "a": [], "b": ["a"] })
    );
    assert_eq!(assemble_out["degraded"], serde_json::json!(false));

    // Terminal: a second assemble reports the TS error through the MCP
    // `isError` envelope (NOT a JSON-RPC `error`).
    let response = dispatch(&mut state, &assemble_line);
    assert!(response.contains(r#""isError":true"#), "{response}");
    assert_eq!(
        crate::tool_text(&response),
        format!("Error: codegen_assemble failed: Plan {plan_id} not found")
    );

    // --- codegen_clean: idempotent ---
    let clean_line = format!(
        r#"{{"jsonrpc":"2.0","id":65,"method":"tools/call","params":{{"name":"codegen_clean","arguments":{{"planId":"{plan_id}"}}}}}}"#
    );
    let clean_out: serde_json::Value =
        serde_json::from_str(&crate::tool_text(&dispatch(&mut state, &clean_line)))
            .expect("clean json");
    assert_eq!(
        clean_out,
        serde_json::json!({ "ok": true, "deleted": false })
    );
}

#[test]
fn codegen_plan_validation_failure_uses_ts_error_envelope() {
    let mut state = codegen_state();
    // Unknown node id → the TS plan route 400s with the validation
    // message; the tool layer prefixes `codegen_plan failed: `.
    let line = r#"{"jsonrpc":"2.0","id":66,"method":"tools/call","params":{"name":"codegen_plan","arguments":{"plan":{"chunks":[{"id":"a","nodeIds":["ghost"],"dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}}}}"#;
    let response = dispatch(&mut state, line);
    assert!(response.contains(r#""isError":true"#), "{response}");
    assert_eq!(
        crate::tool_text(&response),
        "Error: codegen_plan failed: Chunk a: node ghost not found in document"
    );
}
