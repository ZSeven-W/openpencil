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
//! This is the DEFAULT generation protocol for the open / Chinese reasoning
//! models (see [`program_gen_enabled_for_model`]); the executable-JS script
//! path is now an env-gated opt-in, because its all-or-nothing parse flattened
//! tables whenever a weak model truncated the script mid-op.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::EditorState;

/// Whether the sub-agent should emit a `batch_design` DSL PROGRAM for `model`.
///
/// Resolution order:
///   1. `OPENPENCIL_PROGRAM_GEN` set → honor it verbatim (force on/off for A-B).
///   2. Another protocol explicitly opted into (`OPENPENCIL_SCRIPT_GEN` /
///      `OPENPENCIL_MANIFEST`) → defer to it (program stays off).
///   3. Otherwise DEFAULT ON for the open / Chinese reasoning models and OFF for
///      Claude / GPT / Gemini / o-series (which emit clean flat JSONL natively).
///
/// This is the weak-model default because it is both structurally safer and
/// more truncation-resilient than the alternatives — see the module docs and
/// [`crate::script_gen::script_gen_enabled_for_model`].
pub fn program_gen_enabled_for_model(model: &str) -> bool {
    if let Ok(v) = std::env::var("OPENPENCIL_PROGRAM_GEN") {
        return matches!(v.trim(), "1" | "true" | "TRUE" | "on");
    }
    // A different generation protocol was explicitly requested — don't shadow it.
    if std::env::var("OPENPENCIL_SCRIPT_GEN").is_ok()
        || std::env::var("OPENPENCIL_MANIFEST").is_ok()
    {
        return false;
    }
    default_program_gen_for_model(model)
}

/// Family default (no env override): ON for the open / Chinese reasoning models,
/// OFF for Claude / GPT / Gemini / o-series (which handle flat JSONL natively)
/// and for an empty/unknown-but-flat-native id.
fn default_program_gen_for_model(model: &str) -> bool {
    let normalized = match model.find('/') {
        Some(i) => &model[i + 1..],
        None => model,
    };
    let lower = normalized.to_lowercase();
    if lower.is_empty() {
        return false;
    }
    let flat_jsonl_native = lower.contains("claude")
        || lower.contains("gpt-")
        || lower.contains("gpt4")
        || lower.contains("gemini")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4");
    !flat_jsonl_native
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
    run_program_to_forest(&program)
}

/// Run a `batch_design` DSL PROGRAM string against a FRESH empty document and
/// return the produced section forest. Shared by [`parse_program`] (the model
/// authored the DSL directly) and `script_gen` (a JS engine emitted the DSL by
/// calling the bound `I`/`C`/… functions). The executor collects per-line errors
/// and applies the surviving lines (best-effort); a program that builds nothing
/// is an error.
pub fn run_program_to_forest(program: &str) -> Result<Vec<PenNode>, String> {
    let mut state = EditorState::new();
    let mut args: BTreeMap<String, String> = BTreeMap::new();
    args.insert("operations".to_string(), program.to_string());

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
