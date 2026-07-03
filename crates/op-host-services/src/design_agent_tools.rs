//! In-process design tool surface for the AI design agent loop.
//!
//! Mirrors `chat_canvas_tools.rs` for the 14-tool design toolset (vs the
//! 7-tool CRUD set). Schema definitions for every tool are derived from
//! `mcp_serve::schemas::TOOL_SCHEMAS` — the same source the MCP server
//! advertises — so the in-process and MCP surfaces stay byte-equal as JSON.
//!
//! The loop's design surface is `batch_design`, which accepts a sandboxed-JS
//! `script` input (see `op_mcp::script_runner`) in addition to the `operations`
//! DSL — giving the loop loops/data-driven emission without a separate
//! element-builder tool.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use jian_ops_schema::node::{ContainerProps, PenNode, TextContent};
use op_ai::chat_provider::{ChatToolDef, ChatToolResult};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;
use op_mcp::ToolRegistry;

use crate::chat_canvas_tools::{execute_chat_tool, execute_with_registry};
use crate::mcp_serve::schemas;

/// The 14-tool design toolset with auth levels.
/// Reads = "read"; batch_design / set_variables / spawn_agents /
/// export_nodes = "create".
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
            // Every design tool is sourced from TOOL_SCHEMAS for byte-equal
            // MCP parity.
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
    execute_design_tool_with_reveals(
        state,
        name,
        args_json,
        op_editor_core::agent_indicators::active_epoch(),
    )
}

/// Execute one design tool call and register entrance reveals for nodes inserted
/// by write batches when the host has an active indicator epoch.
pub fn execute_design_tool_with_reveals(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
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
    let reveal_started_ms = reveal_now_millis();
    let ids_before = should_register_batch_reveals(name, indicator_epoch)
        .then(|| collect_active_node_ids(state));
    let registry = design_tool_registry(state, name);
    let (mut result, mutated) = execute_with_registry(state, name, args_json, registry);
    if mutated && !result.is_error {
        if let Some(ids_before) = ids_before.as_ref() {
            register_new_node_reveals(ids_before, state, indicator_epoch, reveal_started_ms);
        }
    }
    // Per-batch layout feedback: after every WRITE batch, attach what the real
    // layout proves wrong (collapses / table overflow / text overflow) so the
    // model sees each batch's geometric consequences immediately and repairs
    // them in-process, instead of piling defects up for the loop-end finalize.
    // Deterministic analogue of Pencil's per-batch snapshot_layout feedback.
    if mutated && name == "batch_design" && !result.is_error {
        let issues = op_orchestrator::geometry_validation::geometry_diagnostics(state);
        if !issues.is_empty() {
            if let Ok(mut envelope) = serde_json::from_str::<serde_json::Value>(&result.content) {
                if let Some(obj) = envelope.as_object_mut() {
                    obj.insert("layoutIssues".into(), serde_json::json!(issues));
                    obj.insert(
                        "layoutHint".into(),
                        serde_json::json!(
                            "The resolved layout has the issues above. Fix them with a follow-up batch_design before building the next section."
                        ),
                    );
                    result.content = envelope.to_string();
                }
            }
        }
    }
    (result, mutated)
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
    execute_agent_tool_with_reveals(
        state,
        name,
        args_json,
        op_editor_core::agent_indicators::active_epoch(),
    )
}

/// Host-facing tool router with an explicit indicator epoch for tests and
/// desktop paths that already know the active design-loop epoch.
pub fn execute_agent_tool_with_reveals(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    indicator_epoch: Option<u64>,
) -> (ChatToolResult, bool) {
    if design_tool_level(name).is_some() {
        execute_design_tool_with_reveals(state, name, args_json, indicator_epoch)
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

fn should_register_batch_reveals(name: &str, indicator_epoch: Option<u64>) -> bool {
    indicator_epoch.is_some() && name == "batch_design"
}

fn collect_active_node_ids(state: &EditorState) -> HashSet<String> {
    let mut out = HashSet::new();
    for node in state.active_children() {
        collect_node_ids(node, &mut out);
    }
    out
}

fn collect_node_ids(node: &PenNode, out: &mut HashSet<String>) {
    out.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            collect_node_ids(child, out);
        }
    }
}

fn register_new_node_reveals(
    ids_before: &HashSet<String>,
    state: &EditorState,
    indicator_epoch: Option<u64>,
    reveal_started_ms: u64,
) {
    let Some(epoch) = indicator_epoch else {
        return;
    };
    let mut stream = RevealStream {
        index: 0,
        next_start_ms: reveal_started_ms,
    };
    for node in state.active_children() {
        register_node_reveals(
            node,
            ids_before,
            epoch,
            reveal_started_ms,
            0,
            None,
            &mut stream,
        );
    }
}

struct RevealStream {
    index: u64,
    next_start_ms: u64,
}

fn register_node_reveals(
    node: &PenNode,
    ids_before: &HashSet<String>,
    epoch: u64,
    reveal_started_ms: u64,
    depth: u64,
    parent_reveal_start_ms: Option<u64>,
    stream: &mut RevealStream,
) {
    let id = node.id_str();
    let mut own_reveal_start_ms = parent_reveal_start_ms;
    if !ids_before.contains(id) && should_reveal_node(node, depth) {
        let own_stream_index = stream.index;
        stream.index += 1;
        let base_start = reveal_started_ms
            + op_editor_core::agent_indicators::reveal_offset_ms(depth, own_stream_index);
        let child_runway_start = parent_reveal_start_ms
            .map(|started_at| {
                started_at.saturating_add(op_editor_core::agent_indicators::REVEAL_CHILD_RUNWAY_MS)
            })
            .unwrap_or(reveal_started_ms);
        let started_at = base_start.max(child_runway_start).max(stream.next_start_ms);
        op_editor_core::agent_indicators::add_reveal(epoch, id, started_at);
        stream.next_start_ms =
            started_at.saturating_add(op_editor_core::agent_indicators::REVEAL_STAGGER_MS);
        own_reveal_start_ms = Some(started_at);
    }
    if let Some(children) = node.children() {
        for child in children {
            register_node_reveals(
                child,
                ids_before,
                epoch,
                reveal_started_ms,
                depth + 1,
                own_reveal_start_ms,
                stream,
            );
        }
    }
}

fn should_reveal_node(node: &PenNode, depth: u64) -> bool {
    depth == 0 || node_has_own_visual(node) || node_is_named_structure(node)
}

fn node_has_own_visual(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => {
            container_has_own_visual(&n.container) || n.image_search_query.is_some()
        }
        PenNode::Group(n) => container_has_own_visual(&n.container),
        PenNode::Rectangle(n) => container_has_own_visual(&n.container),
        PenNode::Ref(_) => false,
        PenNode::Text(n) => match &n.content {
            TextContent::Plain(s) => !s.is_empty(),
            TextContent::Styled(segments) => !segments.is_empty(),
        },
        _ => true,
    }
}

fn container_has_own_visual(container: &ContainerProps) -> bool {
    container
        .fill
        .as_ref()
        .is_some_and(|fills| !fills.is_empty())
        || container.stroke.is_some()
        || container
            .effects
            .as_ref()
            .is_some_and(|effects| !effects.is_empty())
}

fn node_is_named_structure(node: &PenNode) -> bool {
    if !node.is_container() {
        return false;
    }
    let base = node.base();
    base.role.as_deref().is_some_and(|role| !role.is_empty())
        || base.name.as_deref().is_some_and(|name| !name.is_empty())
}

fn reveal_now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
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
            return extract_from_schema_entry(entry);
        }
    }
    None
}

/// Parse one tool descriptor JSON string into `(description, inputSchema)`.
/// Used for `TOOL_SCHEMAS` entries.
fn extract_from_schema_entry(entry: &str) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(entry).ok()?;
    let description = v.get("description")?.as_str()?.to_string();
    let input_schema = v.get("inputSchema")?.clone();
    Some((description, input_schema.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_tool_defs_cover_all_14_tools_with_schema_parity() {
        let defs = design_tool_defs();

        // All 14 tools are present, every one MCP-sourced.
        assert_eq!(defs.len(), 14, "expected 14 design tool defs");
        for (name, _) in DESIGN_TOOLS {
            assert!(
                defs.iter().any(|d| d.name == *name),
                "missing design tool def for {name}"
            );
        }

        // PARITY: for each tool, the input_schema_json in the def must equal
        // the inputSchema value from TOOL_SCHEMAS (as parsed JSON), so
        // in-process defs stay byte-equal to the MCP server.
        for def in defs.iter() {
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

    #[test]
    fn execute_design_batch_design_registers_reveals_when_epoch_is_set() {
        use op_editor_core::agent_indicators;

        agent_indicators::clear();
        let epoch = agent_indicators::begin();
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool_with_reveals(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',name:'Root',width:120,height:80})\ntitle=I(root,{type:'text',name:'Title',content:'Hello',width:80,height:20})"}"#,
            Some(epoch),
        );
        assert!(!result.is_error, "batch_design failed: {}", result.content);
        assert!(mutated, "batch_design must mutate the document");

        let ids: Vec<String> = collect_active_node_ids(&state).into_iter().collect();
        assert!(ids.len() >= 2, "batch inserted a subtree, got {ids:?}");
        let snapshot = agent_indicators::snapshot();
        for id in ids {
            assert!(
                snapshot.reveals.contains_key(&id),
                "newly inserted node {id} should have a reveal: {:?}",
                snapshot.reveals
            );
        }
        agent_indicators::end_if_epoch(epoch);
        agent_indicators::clear();
    }

    #[test]
    fn execute_design_batch_design_attaches_per_batch_layout_feedback() {
        // A batch that lands an OVERFLOWING table (5×240 fixed columns in a
        // 600px root) must come back with `layoutIssues` — the per-batch
        // geometry feedback the model repairs in-process.
        let mut state = EditorState::new();
        let ops = r#"{"operations":"root=I(null,{\"type\":\"frame\",\"name\":\"Page\",\"width\":600,\"height\":\"fit_content\",\"layout\":\"vertical\",\"children\":[{\"type\":\"frame\",\"name\":\"Client Table\",\"layout\":\"vertical\",\"width\":\"fill_container\",\"children\":[{\"type\":\"frame\",\"name\":\"Row\",\"layout\":\"horizontal\",\"gap\":16,\"width\":\"fill_container\",\"height\":24,\"children\":[{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20}]},{\"type\":\"frame\",\"name\":\"Row\",\"layout\":\"horizontal\",\"gap\":16,\"width\":\"fill_container\",\"height\":24,\"children\":[{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20},{\"type\":\"frame\",\"name\":\"C\",\"width\":240,\"height\":20}]}]}]})"}"#;
        let (result, mutated) = execute_design_tool(&mut state, "batch_design", ops);
        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        let issues = v["layoutIssues"].as_array().expect("layoutIssues attached");
        assert!(
            issues
                .iter()
                .any(|i| i.as_str().unwrap_or("").contains("column widths")),
            "table overflow reported, got {issues:?}"
        );
        assert!(v["layoutHint"].is_string(), "actionable hint attached");
    }

    #[test]
    fn execute_design_clean_batch_attaches_no_layout_feedback() {
        // A geometrically clean batch must NOT carry layoutIssues noise.
        let mut state = EditorState::new();
        let (result, mutated) = execute_design_tool(
            &mut state,
            "batch_design",
            r#"{"operations":"root=I(null,{type:'frame',width:400,height:300})"}"#,
        );
        assert!(!result.is_error, "batch failed: {}", result.content);
        assert!(mutated);
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(
            v.get("layoutIssues").is_none(),
            "clean layout must not attach issues: {}",
            result.content
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
