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

use op_editor_core::{EditorCommand, EditorState, UIKit};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_sanitizes_dashes() {
        assert_eq!(element_tool_name("btn-primary"), "insert_btn_primary");
        assert_eq!(element_tool_name("nav-bar"), "insert_nav_bar");
    }

    #[test]
    fn tool_emits_instantiate_command() {
        let tool = InsertKitComponent::new("openpencil-starter", "btn-primary");
        assert_eq!(tool.name(), "insert_btn_primary");
        let mut args = BTreeMap::new();
        args.insert("x".to_string(), "120".to_string());
        args.insert("y".to_string(), "80".to_string());
        match tool.call(&args) {
            ToolOutcome::OkWithCommand(_, cmd) => match cmd {
                EditorCommand::InstantiateKitComponent {
                    kit_id,
                    component_id,
                    doc_x,
                    doc_y,
                } => {
                    assert_eq!(kit_id, "openpencil-starter");
                    assert_eq!(component_id, "btn-primary");
                    assert_eq!(doc_x, Some(120.0));
                    assert_eq!(doc_y, Some(80.0));
                }
                _ => panic!("expected InstantiateKitComponent"),
            },
            other => panic!("expected OkWithCommand, got {other:?}"),
        }
    }

    #[test]
    fn missing_x_y_default_to_none() {
        let tool = InsertKitComponent::new("openpencil-starter", "badge");
        match tool.call(&BTreeMap::new()) {
            ToolOutcome::OkWithCommand(
                _,
                EditorCommand::InstantiateKitComponent { doc_x, doc_y, .. },
            ) => {
                assert_eq!(doc_x, None);
                assert_eq!(doc_y, None);
            }
            other => panic!("expected OkWithCommand, got {other:?}"),
        }
    }

    #[test]
    fn malformed_x_is_a_hard_error() {
        let tool = InsertKitComponent::new("openpencil-starter", "badge");
        let mut args = BTreeMap::new();
        args.insert("x".to_string(), "not-a-number".to_string());
        match tool.call(&args) {
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _) => {}
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[test]
    fn registry_covers_every_starter_kit_component() {
        let state = EditorState::new();
        let tools = insert_kit_component_tools(&state);
        let schemas = element_tool_schemas(&state);
        assert_eq!(tools.len(), 6, "starter kit ships 6 components");
        assert_eq!(schemas.len(), tools.len(), "schema + tool counts agree");
        // Each tool name appears verbatim in its schema.
        for tool in &tools {
            assert!(
                schemas
                    .iter()
                    .any(|s| s.contains(&format!("\"name\":\"{}\"", tool.name()))),
                "schema set must include {}",
                tool.name(),
            );
        }
    }
}
