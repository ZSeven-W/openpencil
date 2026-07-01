//! In-process design tool surface for the AI design agent loop.
//!
//! Mirrors `chat_canvas_tools.rs` for the 14-tool design toolset (vs the
//! 7-tool CRUD set). Schema definitions are derived from
//! `mcp_serve::schemas::TOOL_SCHEMAS` — the same source the MCP server
//! advertises — so the in-process and MCP surfaces stay byte-equal as JSON.
//!
//! This module provides the defs + registry + executor that Task 2.3 will
//! wire into the design agent tool-loop. It does NOT touch `op-orchestrator`
//! and does NOT wire routing (that is Task 2.3).

use op_ai::chat_provider::{ChatToolDef, ChatToolResult};
use op_editor_core::EditorState;
use op_mcp::ToolRegistry;

use crate::chat_canvas_tools::{execute_chat_tool, execute_with_registry};
use crate::mcp_serve::schemas;

/// The 14-tool design toolset with auth levels.
/// Reads = "read"; batch_design / set_variables / spawn_agents / export_nodes = "create".
pub const DESIGN_TOOLS: &[(&str, &str)] = &[
    ("get_editor_state", "read"),
    ("get_guidelines", "read"),
    ("get_style_guide_tags", "read"),
    ("get_style_guide", "read"),
    ("get_variables", "read"),
    ("set_variables", "create"),
    ("batch_get", "read"),
    ("snapshot_layout", "read"),
    ("find_empty_space", "read"),
    ("batch_design", "create"),
    ("get_screenshot", "read"),
    ("export_nodes", "create"),
    ("spawn_agents", "create"),
    ("ToolSearch", "read"),
];

/// Auth level for a design tool name (`None` = not in the design set).
pub fn design_tool_level(name: &str) -> Option<&'static str> {
    DESIGN_TOOLS
        .iter()
        .find(|(tool, _)| *tool == name)
        .map(|(_, level)| *level)
}

/// Build tool definitions for the design agent by deriving them from
/// `schemas::TOOL_SCHEMAS` — so the in-process schema is byte-equal to
/// what the MCP server advertises (parity guarantee).
pub fn design_tool_defs() -> Vec<ChatToolDef> {
    DESIGN_TOOLS
        .iter()
        .map(|(name, _)| {
            let (description, input_schema_json) = extract_from_schemas(name)
                .unwrap_or_else(|| panic!("design tool {name} not found in TOOL_SCHEMAS"));
            ChatToolDef {
                name: name.to_string(),
                description,
                level: design_tool_level(name).unwrap_or("read").to_string(),
                input_schema_json,
            }
        })
        .collect()
}

/// Execute one design tool call against the live editor state. Returns
/// the TS-shaped tool result plus whether the call mutated the document.
///
/// Reuses `execute_with_registry` from `chat_canvas_tools` so the
/// dispatch+apply discipline (parse_tool_call → registry.dispatch →
/// state.apply → envelope) is not duplicated.
pub fn execute_design_tool(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
) -> (ChatToolResult, bool) {
    let Some(_level) = design_tool_level(name) else {
        let envelope = serde_json::json!({ "success": false, "error": format!("tool not available in design agent: {name}") });
        return (
            ChatToolResult {
                content: envelope.to_string(),
                is_error: true,
            },
            false,
        );
    };
    let registry = design_tool_registry(state, name);
    execute_with_registry(state, name, args_json, registry)
}

/// Unified executor for the design agent pump: design-surface tools
/// route to [`execute_design_tool`]; everything else falls through to
/// [`execute_chat_tool`] (the CRUD surface). Only tools the provider
/// ADVERTISES are ever called by the model, so CRUD tools never see
/// design-only names and vice versa — this router is purely defensive.
///
/// This is the single call-site in `chat_session.rs::drain_tool_requests`
/// once the design-loop flag is ON. When the flag is OFF the pump still
/// calls `execute_chat_tool` directly, so the CRUD path is unaffected.
pub fn execute_agent_tool(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
) -> (ChatToolResult, bool) {
    if design_tool_level(name).is_some() {
        execute_design_tool(state, name, args_json)
    } else {
        execute_chat_tool(state, name, args_json)
    }
}

/// Build a registry carrying only the requested design tool — snapshot
/// registered against the live state so read tools see prior writes.
fn design_tool_registry(state: &EditorState, requested: &str) -> ToolRegistry {
    use crate::mcp_serve::export_tool::export_nodes_snapshot;
    use crate::mcp_serve::screenshot_tool::get_screenshot_snapshot;

    let mut r = ToolRegistry::default();
    match requested {
        "get_editor_state" => r.register(Box::new(op_mcp::get_editor_state_snapshot(state))),
        "get_guidelines" => r.register(Box::new(op_mcp::get_guidelines_snapshot())),
        "get_style_guide_tags" => r.register(Box::new(op_mcp::get_style_guide_tags_snapshot())),
        "get_style_guide" => r.register(Box::new(op_mcp::get_style_guide_snapshot())),
        "get_variables" => r.register(Box::new(op_mcp::get_variables_snapshot(state))),
        "set_variables" => r.register(Box::new(op_mcp::set_variables_snapshot())),
        "batch_get" => r.register(Box::new(op_mcp::batch_get_snapshot(state))),
        "snapshot_layout" => r.register(Box::new(op_mcp::snapshot_layout_snapshot(state))),
        "find_empty_space" => r.register(Box::new(op_mcp::find_empty_space_snapshot(state))),
        "batch_design" => r.register(Box::new(op_mcp::batch_design_snapshot(state))),
        "get_screenshot" => r.register(Box::new(get_screenshot_snapshot(state))),
        "export_nodes" => r.register(Box::new(export_nodes_snapshot(state))),
        "spawn_agents" => r.register(Box::new(op_mcp::spawn_agents_snapshot())),
        "ToolSearch" => r.register(Box::new(op_mcp::tool_search_snapshot(
            schemas::TOOL_SCHEMAS,
        ))),
        _ => {}
    }
    r
}

/// Extract `description` and `inputSchema` JSON string from `TOOL_SCHEMAS`
/// for the given tool name. Returns `None` when no entry matches.
///
/// Parses the raw JSON descriptor using `serde_json` so the extracted
/// `inputSchema` is round-tripped through `serde_json::Value` — ensuring
/// the parity test can compare it by value rather than string equality.
fn extract_from_schemas(name: &str) -> Option<(String, String)> {
    for entry in schemas::TOOL_SCHEMAS {
        let v: serde_json::Value = serde_json::from_str(entry).ok()?;
        if v.get("name").and_then(|n| n.as_str()) == Some(name) {
            let description = v.get("description")?.as_str()?.to_string();
            let input_schema = v.get("inputSchema")?.clone();
            let input_schema_json = input_schema.to_string();
            return Some((description, input_schema_json));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_tool_defs_cover_all_14_tools_with_schema_parity() {
        let defs = design_tool_defs();

        // All 14 tools are present.
        assert_eq!(defs.len(), 14, "expected 14 design tool defs");
        for (name, _) in DESIGN_TOOLS {
            assert!(
                defs.iter().any(|d| d.name == *name),
                "missing design tool def for {name}"
            );
        }

        // PARITY: for each tool, the input_schema_json in the def must
        // equal the inputSchema value from TOOL_SCHEMAS (as parsed JSON).
        // This ensures in-process defs stay byte-equal to the MCP server.
        for def in &defs {
            // Find the matching TOOL_SCHEMAS entry.
            let schema_entry = schemas::TOOL_SCHEMAS
                .iter()
                .find(|entry| {
                    let v: serde_json::Value = serde_json::from_str(entry).unwrap();
                    v.get("name").and_then(|n| n.as_str()) == Some(def.name.as_str())
                })
                .unwrap_or_else(|| panic!("design tool {} not found in TOOL_SCHEMAS", def.name));

            // Extract the canonical inputSchema from TOOL_SCHEMAS.
            let canonical: serde_json::Value = serde_json::from_str(schema_entry).unwrap();
            let canonical_schema = canonical.get("inputSchema").unwrap_or_else(|| {
                panic!("TOOL_SCHEMAS entry for {} missing inputSchema", def.name)
            });

            // Parse the def's input_schema_json and compare as Value.
            let def_schema: serde_json::Value = serde_json::from_str(&def.input_schema_json)
                .unwrap_or_else(|e| {
                    panic!("def.input_schema_json for {} unparseable: {e}", def.name)
                });

            assert_eq!(
                def_schema, *canonical_schema,
                "inputSchema mismatch for {}: in-process def != TOOL_SCHEMAS",
                def.name
            );
        }

        // Every DESIGN_TOOLS entry must exist in TOOL_SCHEMAS (no orphans).
        for (name, _) in DESIGN_TOOLS {
            let found = schemas::TOOL_SCHEMAS.iter().any(|entry| {
                let v: serde_json::Value = serde_json::from_str(entry).unwrap();
                v.get("name").and_then(|n| n.as_str()) == Some(*name)
            });
            assert!(found, "design tool {name} is not in TOOL_SCHEMAS — orphan!");
        }
    }

    #[test]
    fn execute_design_rejects_tools_outside_the_design_set() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(&mut state, "delete_page", "{}");
        assert!(result.is_error);
        assert!(!mutated);
        assert!(result.content.contains("not available in design agent"));
    }

    #[test]
    fn execute_design_read_tool_returns_success_envelope() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(&mut state, "get_editor_state", "{}");
        assert!(!result.is_error, "got {}", result.content);
        assert!(!mutated, "read tools never mutate");
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(v["success"], serde_json::Value::Bool(true));
    }

    #[test]
    fn execute_design_batch_design_inserts_frame_and_mutates() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',width:120,height:80})"}"#,
        );
        assert!(!result.is_error, "batch_design failed: {}", result.content);
        assert!(mutated, "batch_design must mutate the document");

        // The active page must now have at least one child (the inserted frame).
        assert!(
            !state.active_children().is_empty(),
            "doc must have a frame after batch_design"
        );
    }

    // --- execute_agent_tool tests ---

    #[test]
    fn execute_agent_tool_routes_design_tool_to_design_surface() {
        // batch_design is a design-only tool — it must execute and mutate
        // via the design surface, not the CRUD surface.
        let mut state = EditorState::new();
        let (result, mutated) = execute_agent_tool(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',width:80,height:60})"}"#,
        );
        assert!(
            !result.is_error,
            "batch_design via agent router failed: {}",
            result.content
        );
        assert!(mutated, "batch_design must mutate via the design surface");
        assert!(
            !state.active_children().is_empty(),
            "a frame must exist after batch_design via execute_agent_tool"
        );
    }

    #[test]
    fn execute_agent_tool_routes_crud_tool_to_chat_surface() {
        // delete_node is a CRUD-only tool — it must route to execute_chat_tool.
        // With an unknown nodeId the chat surface returns an error (node not found),
        // which proves the CRUD path was taken rather than the design path that
        // would have returned "not available in design agent".
        let mut state = EditorState::new();
        let (result, mutated) =
            execute_agent_tool(&mut state, "delete_node", r#"{"nodeId":"nope"}"#);
        // The CRUD surface returns an error for an unknown node — NOT "not available in design agent".
        assert!(result.is_error, "unknown node delete must error");
        assert!(!mutated);
        assert!(
            !result.content.contains("not available in design agent"),
            "must have taken the CRUD path, not the design path"
        );
    }

    #[test]
    fn execute_agent_tool_unknown_name_returns_not_available_error() {
        // A name outside both sets falls through to execute_chat_tool
        // which returns "not available in chat".
        let mut state = EditorState::new();
        let (result, mutated) = execute_agent_tool(&mut state, "delete_page", "{}");
        assert!(result.is_error);
        assert!(!mutated);
        assert!(
            result.content.contains("not available in chat"),
            "unknown tools should report the CRUD surface's 'not available in chat' error, got: {}",
            result.content
        );
    }
}
