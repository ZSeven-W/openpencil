//! The shared program→forest executor used by script-gen and the
//! `batch_design` bridge.
//!
//! `run_program_to_forest` runs a `batch_design` DSL PROGRAM —
//! `name=I(parent, {...})` insert operations with shared bindings — against a
//! FRESH empty document and returns the produced section forest. Because a
//! child is nested by passing its parent's *binding* (a captured variable),
//! the two weak-model structural failures a flat `_parent` list suffers
//! become near-inexpressible:
//!   - a table/list cell cannot be emitted as a full-width SIBLING of its row
//!     (the row id is a captured variable, not a string the model re-types), and
//!   - a "header but zero rows" table cannot happen when each row is its own
//!     `I(table, ...)` line authored alongside the data.
//!
//! `script_gen` is the only caller today: the sub-agent writes a real
//! JavaScript program (`op_mcp::script_runner`), and the recorded
//! `batch_design` program it produces is handed to
//! [`run_program_to_forest`] here to build the section forest.
//!
//! ## Why this returns a `StateSchema` alongside the forest
//!
//! `op-mcp`'s generation-insert path (`batch_design.rs::hoist_generation_state`)
//! hoists any node-level `state` block into a doc-root `MergeAppState` command
//! BEFORE the insert command, tagged with the "unplanned" priority — it has no
//! orchestrator plan index to stamp. That `MergeAppState` lands on whatever
//! document it is applied to. Here that document is a SCRATCH `EditorState::new()`
//! that exists only to run the program and get a forest back — its `doc.state`
//! is discarded once this function returns the forest alone. The orchestrator
//! (`subagent.rs`) is the one caller that actually knows the subtask's real
//! `plan_idx`, so it needs the hoisted state handed back separately in order to
//! re-tag it with that index (rather than the unplanned one baked in here) before
//! merging it into the live document. Returning `(forest, program_state)` lets the
//! caller do exactly that instead of silently losing every `$app.*` key a
//! script-gen'd subtask declared.
use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::state::StateSchema;
use op_editor_core::EditorState;

/// Run a `batch_design` DSL PROGRAM string against a FRESH empty document and
/// return the produced section forest plus any doc-root `state` the program's
/// nodes hoisted (see the module doc for why the latter matters). The caller
/// (`script_gen`) hands in the DSL a JS engine emitted by calling the bound
/// `I`/`C`/… functions. The executor collects per-line errors and applies the
/// surviving lines (best-effort); a program that builds nothing is an error.
pub fn run_program_to_forest(program: &str) -> Result<(Vec<PenNode>, StateSchema), String> {
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
    // The scratch document starts empty, so anything sitting in `doc.state`
    // now came from the program's own hoisted `MergeAppState` — drain it so
    // the caller can re-tag it with the subtask's real plan_idx.
    let program_state = state.doc.state.take().unwrap_or_default();
    Ok((nodes, program_state))
}

/// Echo the executor's per-line `errors[]` (if any) to stderr — visibility
/// without failing the section.
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

#[cfg(test)]
#[path = "program_gen_tests.rs"]
mod tests;
