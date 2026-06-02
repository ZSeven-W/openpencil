//! TS-compatible codegen MCP tool tests.

use std::collections::BTreeMap;

use super::{
    codegen_assemble_snapshot, codegen_clean_snapshot, codegen_plan_snapshot,
    codegen_submit_chunk_snapshot, McpTool, ToolErrorCode, ToolOutcome,
};

fn arg(key: &str, value: &str) -> BTreeMap<String, String> {
    let mut args = BTreeMap::new();
    args.insert(key.into(), value.into());
    args
}

#[test]
fn codegen_plan_reports_live_canvas_requirement_in_file_backed_rust_mcp() {
    let tool = codegen_plan_snapshot();
    let args = arg(
        "plan",
        r#"{"chunks":[{"chunkId":"hero","nodeIds":["n1"],"dependsOn":[]}],"sharedStyles":[],"rootLayout":{"nodeId":"n1"}}"#,
    );

    match tool.call(&args) {
        ToolOutcome::Err(code, message) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert_eq!(
                message,
                "codegen_plan requires a running App (live canvas mode). Pipeline state is stored in App memory."
            );
        }
        other => panic!("expected live-canvas ToolFailed, got {other:?}"),
    }
}

#[test]
fn codegen_submit_chunk_reports_live_canvas_requirement() {
    let tool = codegen_submit_chunk_snapshot();
    let mut args = arg("planId", "plan-1");
    args.insert(
        "result".into(),
        r#"{"chunkId":"hero","code":"export function Hero() { return null; }","contract":{"provides":[],"requires":[]}}"#
            .into(),
    );
    args.insert("status".into(), "skipped".into());

    match tool.call(&args) {
        ToolOutcome::Err(code, message) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert_eq!(
                message,
                "codegen_submit_chunk requires a running App (live canvas mode)"
            );
        }
        other => panic!("expected live-canvas ToolFailed, got {other:?}"),
    }
}

#[test]
fn codegen_assemble_reports_live_canvas_requirement() {
    let tool = codegen_assemble_snapshot();
    let mut args = arg("planId", "plan-1");
    args.insert("framework".into(), "react".into());

    match tool.call(&args) {
        ToolOutcome::Err(code, message) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert_eq!(
                message,
                "codegen_assemble requires a running App (live canvas mode)"
            );
        }
        other => panic!("expected live-canvas ToolFailed, got {other:?}"),
    }
}

#[test]
fn codegen_clean_is_idempotent_without_live_canvas_state() {
    let tool = codegen_clean_snapshot();
    let args = arg("planId", "plan-1");

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("ok").map(String::as_str), Some("true"));
            assert_eq!(out.get("deleted").map(String::as_str), Some("false"));
        }
        other => panic!("expected idempotent clean response, got {other:?}"),
    }
}
