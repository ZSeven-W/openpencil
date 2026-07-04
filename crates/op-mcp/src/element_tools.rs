//! UIKit element tools — one `insert_<comp>` MCP tool per built-in
//! [`UIKit`] component, so an LLM client can drop a Primary Button
//! (etc.) onto the canvas without first having to learn the kit-id
//! / component-id pair. The TS counterpart is `pen-mcp`'s ~100
//! `add_card_v0` / `add_toast_v0` element tools.
//!
//! Each tool returns `ToolOutcome::OkWithCommand(map,
//! EditorCommand::InstantiateKitComponent { … })` so the host's
//! applier runs the same `EditorState::instantiate_kit_component`
//! path the Component-Browser panel uses — deep-clone, fresh-id,
//! subtree-translate, select, history-snapshot.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, EditorState, NodeId, UIKit};

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// Sanitize a `kit-id` / `component-id` for embedding in a tool name
/// — MCP tool names are `[a-zA-Z0-9_]+`, so dashes become underscores.
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The MCP tool name for a kit component: `insert_<comp_sanitized>`.
/// v1 ships one starter kit so the kit prefix is dropped for terseness;
/// a future imported-kits surface can fold the kit id back in once
/// collisions are possible.
pub fn element_tool_name(component_id: &str) -> String {
    format!("insert_{}", sanitize(component_id))
}

/// `insert_<comp>` MCP tool — instantiates one UIKit component onto
/// the active page through the editor command bus.
pub struct InsertKitComponent {
    name: String,
    kit_id: String,
    component_id: String,
}

impl InsertKitComponent {
    pub fn new(kit_id: impl Into<String>, component_id: impl Into<String>) -> Self {
        let component_id = component_id.into();
        Self {
            name: element_tool_name(&component_id),
            kit_id: kit_id.into(),
            component_id,
        }
    }
}

impl McpTool for InsertKitComponent {
    fn name(&self) -> &str {
        &self.name
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        // `x` / `y` are optional doc-px floats; omitted slots default
        // to 0.0 at apply time.
        let doc_x = match parse_optional_f64(args, "x") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let doc_y = match parse_optional_f64(args, "y") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut result = BTreeMap::new();
        result.insert("kit_id".into(), self.kit_id.clone());
        result.insert("component_id".into(), self.component_id.clone());
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InstantiateKitComponent {
                kit_id: self.kit_id.clone(),
                component_id: self.component_id.clone(),
                doc_x,
                doc_y,
                target_parent: NodeId::NONE,
                page_id: None,
                overrides_json: None,
            },
        )
    }
}

/// Parse a number arg as `Option<f64>`. An absent slot returns `None`
/// (the command falls back to 0.0); a malformed slot is a hard error
/// so the LLM client retries with a valid value.
///
/// The `Err` variant carries a full `ToolOutcome::Err` — the call site
/// returns it verbatim, so the large size is intentional.
#[allow(clippy::result_large_err)]
fn parse_optional_f64(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<f64>, ToolOutcome> {
    match args.get(key) {
        None => Ok(None),
        Some(v) if v.is_empty() => Ok(None),
        Some(v) => v.parse::<f64>().map(Some).map_err(|_| {
            ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("{key} must be a number"),
            )
        }),
    }
}

/// Walk every loaded kit and emit one [`InsertKitComponent`] per
/// component. The host's `rebuild_registry` chains this into the live
/// `ToolRegistry`.
pub fn insert_kit_component_tools(state: &EditorState) -> Vec<InsertKitComponent> {
    state
        .ui_kits
        .iter()
        .flat_map(|kit: &UIKit| {
            kit.components
                .iter()
                .map(|c| InsertKitComponent::new(kit.id.clone(), c.id.clone()))
        })
        .collect()
}

/// JSON-encoded `tools/list` schema for one element tool. The host
/// concatenates this into the `tools/list` response next to the static
/// `TOOL_SCHEMAS`.
pub fn element_tool_schema(component_name: &str, component_id: &str) -> String {
    let tool = element_tool_name(component_id);
    canonical_element_tool_schema(&tool, component_name)
}

fn canonical_element_tool_schema(tool: &str, component_name: &str) -> String {
    format!(
        r#"{{"name":"{tool}","description":"Insert a {component_name} from the built-in UIKit onto the active page. Optional x/y doc-px floats place the top-left; defaults to (0, 0).","inputSchema":{{"type":"object","properties":{{"x":{{"type":"string","description":"top-left doc-px (float)"}},"y":{{"type":"string","description":"top-left doc-px (float)"}}}}}}}}"#
    )
}

/// JSON-encoded schemas for every element tool the live state has —
/// matches the iterator order of [`insert_kit_component_tools`] so
/// counts agree.
pub fn element_tool_schemas(state: &EditorState) -> Vec<String> {
    state
        .ui_kits
        .iter()
        .flat_map(|kit| {
            kit.components
                .iter()
                .map(|c| element_tool_schema(&c.name, &c.id))
        })
        .collect()
}
