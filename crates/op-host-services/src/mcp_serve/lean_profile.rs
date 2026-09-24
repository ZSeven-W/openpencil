//! The lean MCP profile: its `tools/list` schemas and its registry.
//!
//! Six tools cover the whole design loop (see `tool_catalog::LEAN_TOOLS`).
//! Four are the full-catalog tools unchanged; `get_editor_state` and
//! `get_screenshot` are the composing wrappers in `lean_tools`.
//!
//! The schemas are DERIVED from the full catalog at first use rather than
//! written out again: each keeps its full-catalog `inputSchema` (minus the
//! per-call `filePath` retarget, which a lean session never needs) and swaps
//! in a shorter description that states the workflow step. Only the two
//! composing wrappers extend their input schema, with the arguments they
//! forward to the tools they delegate to.

use std::sync::OnceLock;

use op_editor_core::EditorState;
use op_mcp::{
    batch_design_snapshot, get_guidelines_snapshot, snapshot_layout_snapshot, ToolRegistry,
};
use serde_json::{json, Value};

use super::finalize_tool::finalize_design_snapshot;
use super::lean_tools::{LeanEditorState, LeanScreenshot};
use super::schemas::TOOL_SCHEMAS;
use super::tool_catalog::LEAN_TOOLS;
use super::tool_profile::schema_name;

/// The loop every lean description is anchored to.
const WORKFLOW: &str = "OpenPencil lean profile — loop: get_editor_state → get_guidelines → \
batch_design → snapshot_layout / get_screenshot → finalize_design.";

fn lean_description(tool: &str) -> String {
    let step = match tool {
        "get_editor_state" => {
            "STEP 1 · READ. Call first. One JSON snapshot: editor (active page, selection, \
             top-level nodes, components), document (page count, node total), pages, \
             variables (name/kind/value) and theme axes. Pass nodeIds and/or depth to also \
             return full node JSON under `nodes` (no nodeIds = the page's top-level nodes; \
             depth 0=node only, 1=children, -1=full subtree, the default). Read a node before \
             editing it with batch_design operations."
        }
        "get_guidelines" => {
            "STEP 2 · LEARN (once per task). Product-design guidelines: category=guide with a \
             topic (web-app, mobile, landing-page, dashboard, slides, card, …), or \
             category=style to resolve a palette/typography style. Read the matching topic \
             before building a new screen."
        }
        "batch_design" => {
            "STEP 3 · WRITE — the only write tool here. New UI: script, a JavaScript program \
             calling I(parent,node) or K(kitRef,parent,overrides) with loops and data arrays. \
             Existing nodes: operations, one per line — U(id,{…}) update, D(id) delete, \
             M(id,parent[,index]) move, C(id,parent,{…}) copy, R(id,{…}) replace, G(…) image \
             fill. Send exactly one of script / operations / nodes_json. Transactional: any \
             failing line applies nothing and errors[] lists every failure — fix and resend. \
             Keep each call to <=25 operations; build large screens section by section."
        }
        "snapshot_layout" => {
            "STEP 4a · VERIFY GEOMETRY. Resolved bounding-box tree of the active page (or of \
             parentId), maxDepth levels deep (default 1). Cheap — run after every batch_design \
             to catch overlap, overflow and zero-size nodes."
        }
        "get_screenshot" => {
            "STEP 4b · VERIFY VISUALLY. Render nodeId (or \"root\", the page's first top-level \
             frame) to an inline PNG image. Pass format (png|jpeg|webp|pdf) and optional scale \
             to EXPORT instead: returns base64 file bytes rather than an image."
        }
        "finalize_design" => {
            "STEP 5 · FINISH. Run OpenPencil's deterministic post-generation repair passes \
             (layout, contrast, overflow, chrome dedupe, image slots, …) and save. Call ONCE \
             after your batch_design calls succeed and BEFORE presenting the result. Returns a \
             repair summary, complete, blockingAdvisoryCount and advisories. You MUST NOT \
             present the design as finished while complete=false: fix every advisory with \
             batch_design operations, then call finalize_design again. root_ids optionally \
             scopes the passes."
        }
        _ => "",
    };
    format!("{step} {WORKFLOW}")
}

/// Input-schema properties a composing wrapper adds on top of its
/// full-catalog schema — exactly the arguments it forwards.
fn extra_properties(tool: &str) -> Option<Value> {
    match tool {
        "get_editor_state" => Some(json!({
            "nodeIds": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Node ids to return as full JSON under `nodes`"
            },
            "depth": {
                "type": "number",
                "description": "Child depth for node reads: 0=node only, 1=direct children, -1=full subtree (default)"
            },
            "pageId": {
                "type": "string",
                "description": "Page to read nodes from (default: the active page)"
            }
        })),
        "get_screenshot" => Some(json!({
            "format": {
                "type": "string",
                "enum": ["png", "jpeg", "webp", "pdf"],
                "description": "Export instead of screenshot: return base64 file bytes in this format"
            },
            "scale": {
                "type": "number",
                "description": "Export scale factor (default 1)"
            }
        })),
        _ => None,
    }
}

fn lean_schema(tool: &str) -> String {
    let full = TOOL_SCHEMAS
        .iter()
        .find(|schema| schema_name(schema).as_deref() == Some(tool))
        .unwrap_or_else(|| panic!("lean tool {tool} has no full-catalog schema"));
    let mut schema: Value =
        serde_json::from_str(full).unwrap_or_else(|e| panic!("{tool} schema JSON: {e}"));
    schema["description"] = Value::String(lean_description(tool));
    let input = schema["inputSchema"]
        .as_object_mut()
        .unwrap_or_else(|| panic!("{tool} schema has no inputSchema"));
    if let Some(properties) = input.get_mut("properties").and_then(Value::as_object_mut) {
        properties.remove("filePath");
    }
    if let Some(Value::Object(extra)) = extra_properties(tool) {
        // A wrapper that now takes arguments can no longer forbid them.
        input.remove("additionalProperties");
        let properties = input
            .entry("properties")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap_or_else(|| panic!("{tool} inputSchema.properties is not an object"));
        properties.extend(extra);
    }
    schema.to_string()
}

/// The lean `tools/list` entries, in workflow order. Built once.
pub(crate) fn lean_tool_schemas() -> &'static [String] {
    static SCHEMAS: OnceLock<Vec<String>> = OnceLock::new();
    SCHEMAS.get_or_init(|| LEAN_TOOLS.iter().map(|tool| lean_schema(tool)).collect())
}

/// The lean registry. Like the full one it snapshots per call and, for a
/// direct call, builds only the requested tool.
pub(super) fn lean_registry(doc: &EditorState, requested_tool: Option<&str>) -> ToolRegistry {
    let mut registry = ToolRegistry::default();
    let wants = |name: &str| requested_tool.is_none_or(|requested| requested == name);
    if wants("get_editor_state") {
        registry.register(Box::new(LeanEditorState::snapshot(doc)));
    }
    if wants("get_guidelines") {
        registry.register(Box::new(get_guidelines_snapshot()));
    }
    if wants("batch_design") {
        registry.register(Box::new(batch_design_snapshot(doc)));
    }
    if wants("snapshot_layout") {
        registry.register(Box::new(snapshot_layout_snapshot(doc)));
    }
    if wants("get_screenshot") {
        registry.register(Box::new(LeanScreenshot::snapshot(doc)));
    }
    if wants("finalize_design") {
        registry.register(Box::new(finalize_design_snapshot(doc)));
    }
    registry
}

#[cfg(test)]
#[path = "lean_profile_tests.rs"]
mod tests;
