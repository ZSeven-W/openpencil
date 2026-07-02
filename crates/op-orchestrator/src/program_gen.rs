//! Program-DSL generation path (`OPENPENCIL_PROGRAM_GEN`).
//!
//! The sub-agent emits a `batch_design` DSL PROGRAM — `name=I(parent, {...})`
//! insert operations with shared bindings — instead of flat `_parent` JSONL.
//! Because a child is nested by passing its parent's *binding* (a captured
//! variable), the two weak-model structural failures the flat path suffers
//! become near-inexpressible:
//!   - a table/list cell cannot be emitted as a full-width SIBLING of its row
//!     (the row id is a captured variable, not a string the model re-types), and
//!   - a "header but zero rows" table cannot happen when each row is its own
//!     `I(table, ...)` line authored alongside the data.
//!
//! Validated 2026-06-29 across glm-5.2 / minimax-m3 / deepseek on dashboard +
//! e-commerce + mobile screens (see openpencil-docs/pencil-generation-way).
//! This is gated OFF by default so it can be A/B'd against the JSONL path on the
//! corpus before becoming the default generation protocol.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::EditorState;

/// Env gate, mirroring [`crate::manifest::manifest_enabled_for_model`]. The
/// `model` is accepted for symmetry / future per-model rollout; today the
/// process-global `OPENPENCIL_PROGRAM_GEN` decides for every model.
pub fn program_gen_enabled_for_model(_model: &str) -> bool {
    std::env::var("OPENPENCIL_PROGRAM_GEN")
        .map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "on"))
        .unwrap_or(false)
}

/// Run the emitted `batch_design` program against a FRESH empty document and
/// return the produced section forest (the program's `I(null, ...)` root inserts
/// become the top-level children). The executor collects per-line errors and
/// applies the surviving lines (TS `runBatchDesignDsl` best-effort semantics), so
/// a single malformed line never sinks the whole section. Errors are surfaced as
/// warnings; a program that builds nothing is an error.
pub fn parse_program(text: &str) -> Result<Vec<PenNode>, String> {
    let program = extract_program(text);
    if program.trim().is_empty() {
        return Err("program is empty after stripping prose/fences".into());
    }
    let mut state = EditorState::new();
    let mut args: BTreeMap<String, String> = BTreeMap::new();
    args.insert("operations".to_string(), program);

    let cmd = {
        let tool = op_mcp::batch_design_snapshot(&state);
        match op_mcp::McpTool::call(&tool, &args) {
            op_mcp::ToolOutcome::OkJsonWithCommand(json, cmd) => {
                surface_program_warnings(&json);
                cmd
            }
            op_mcp::ToolOutcome::OkJson(json) => {
                return Err(format!("program produced no command: {json}"));
            }
            other => return Err(format!("unexpected batch_design outcome: {other:?}")),
        }
    };
    if !state.apply(cmd) {
        return Err("program command rejected by document".into());
    }
    let nodes = state.active_children().to_vec();
    if nodes.is_empty() {
        return Err("program produced no nodes".into());
    }
    Ok(nodes)
}

/// Echo the executor's per-line `errors[]` (if any) to stderr, like the manifest
/// path echoes its warnings — visibility without failing the section.
fn surface_program_warnings(envelope_json: &str) {
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(envelope_json) {
        if let Some(serde_json::Value::Array(errors)) = map.get("errors") {
            for err in errors {
                if let Some(msg) = err.get("error").and_then(|e| e.as_str()) {
                    eprintln!("[program-gen] dropped line: {msg}");
                }
            }
        }
    }
}

/// Strip an accidental ```fence``` wrapper and any leading/trailing prose,
/// keeping the program body. The line-by-line executor tolerates stray prose
/// lines (they become per-line errors), so this only needs to remove the common
/// markdown-fence wrapper a chat model sometimes adds despite instructions.
fn extract_program(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // drop the opening fence's language tag line, and the closing fence.
        let body = rest.split_once('\n').map(|x| x.1).unwrap_or(rest);
        let body = body.rsplit_once("```").map(|x| x.0).unwrap_or(body);
        return body.trim().to_string();
    }
    trimmed.to_string()
}

#[cfg(test)]
#[path = "program_gen_tests.rs"]
mod tests;
