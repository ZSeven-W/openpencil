//! Canvas tool surface for the chat agent loop.
//!
//! Mirrors the TS chat CRUD tool registry
//! (`apps/web/src/services/ai/agent-tools.ts::getCrudToolDefs` +
//! `TOOL_AUTH_MAP`) and the client-side dispatch in
//! `agent-tool-executor.ts`: the model calls canvas tools, the HOST
//! executes them against the live document. Execution reuses the MCP
//! stack end-to-end — the same `op_mcp` tools, the same wire-parse
//! discipline (`parse_tool_call`), and the same
//! `EditorState::apply(EditorCommand)` path `mcp_serve.rs` uses — so
//! a chat tool call behaves byte-for-byte like the MCP server's.
//!
//! Threading mirrors `design_session::RemoteDocSink`: the agent-loop
//! worker calls [`UiChatToolExecutor::execute`], which forwards a
//! [`ChatToolRequest`] over a channel and blocks on the ack; the UI
//! event loop drains requests each frame (`chat_session::pump`) and
//! executes via [`execute_chat_tool`] against the canonical state.

use op_ai::chat_provider::{ChatToolDef, ChatToolResult};
use op_editor_core::{EditorState, PenNodeExt};
pub use op_editor_host_core::chat::{chat_tool_channel, ChatToolRequest, UiChatToolExecutor};
use op_mcp::{ToolRegistry, ToolResponse};
use std::collections::HashSet;

/// TS `maxTurns` for the chat agent loop (`ai-chat-handlers.ts:254`).
pub const MAX_TOOL_TURNS: usize = 20;

pub type DesignModificationOp = (String, serde_json::Value);

/// The chat tool subset — the TS CRUD set (`getCrudToolDefs`) with the
/// TS `TOOL_AUTH_MAP` auth levels. Design-pipeline tools
/// (`generate_design` / `plan_layout` / `batch_insert`) are excluded:
/// design intent routes to the orchestrator pipeline in this shell,
/// so the plan_layout session guard is not ported (see module docs).
const CHAT_TOOLS: &[(&str, &str)] = &[
    ("batch_get", "read"),
    ("snapshot_layout", "read"),
    ("get_selection", "read"),
    ("insert_node", "create"),
    ("update_node", "modify"),
    ("move_node", "modify"),
    ("delete_node", "delete"),
];

/// Auth level for a chat tool name (`None` = not in the chat set).
pub fn chat_tool_level(name: &str) -> Option<&'static str> {
    CHAT_TOOLS
        .iter()
        .find(|(tool, _)| *tool == name)
        .map(|(_, level)| *level)
}

/// Build the tool definitions the agent loop advertises to the model.
/// Descriptions follow the TS `getCrudToolDefs` text; schemas match
/// the Rust `op_mcp` tools' argument surface (the executor dispatches
/// into those tools verbatim).
pub fn chat_tool_defs() -> Vec<ChatToolDef> {
    let def = |name: &str, description: &str, schema: &str| ChatToolDef {
        name: name.to_string(),
        description: description.to_string(),
        level: chat_tool_level(name).unwrap_or("read").to_string(),
        input_schema_json: schema.to_string(),
    };
    vec![
        def(
            "batch_get",
            "Search and read nodes from the document. ALWAYS call this first before update_node or delete_node to find the correct node IDs. With no arguments, returns top-level children (current page structure). Search by type/name patterns or read specific IDs.",
            r#"{"type":"object","properties":{"nodeIds":{"type":"array","items":{"type":"string"},"description":"Node IDs to retrieve"},"patterns":{"type":"array","items":{"type":"object","properties":{"type":{"type":"string"},"name":{"type":"string"}}},"description":"Search patterns to match"},"parentId":{"type":"string"},"readDepth":{"type":"number"},"pageId":{"type":"string"}}}"#,
        ),
        def(
            "snapshot_layout",
            "Get a compact layout snapshot of the current page showing node positions and sizes. Use it as a text-only screenshot-replacement to diagnose visual bugs like stacked badges or overlapping text.",
            r#"{"type":"object","properties":{"parentId":{"type":"string"},"maxDepth":{"type":"string","description":"depth limit, default 1"},"pageId":{"type":"string"}}}"#,
        ),
        def(
            "get_selection",
            "Get the currently selected nodes on the canvas with their full data",
            r#"{"type":"object","properties":{"readDepth":{"type":"number","description":"How deep to include children in selected nodes; default 2"}}}"#,
        ),
        def(
            "insert_node",
            "Insert a new node into the document tree. Always call snapshot_layout or batch_get first. Pass the node as a \"data\" object (type, name, width, height, fill, children, etc.) plus an optional \"parent\" node id for explicit placement.",
            r#"{"type":"object","properties":{"parent":{"type":"string","description":"Parent node id; omit for page root"},"data":{"type":"object","description":"PenNode data (type, name, width, height, fill, children, etc.)"},"pageId":{"type":"string","description":"Target page ID (optional, defaults to active page)"}},"required":["data"]}"#,
        ),
        def(
            "update_node",
            "Update properties of an existing node by ID",
            r#"{"type":"object","properties":{"nodeId":{"type":"string","description":"Node ID to update"},"data":{"type":"object","description":"Properties to update"},"fill_hex":{"type":"string","description":"Shortcut: set the solid fill color (#rrggbb)"},"name":{"type":"string"},"x":{"type":"string"},"y":{"type":"string"},"width":{"type":"string"},"height":{"type":"string"}},"required":["nodeId"]}"#,
        ),
        def(
            "move_node",
            "Move a node to a different parent container. Use when you need to reparent a node (e.g. move an element into a frame). The node is placed at the end of the parent's children list by default.",
            r#"{"type":"object","properties":{"nodeId":{"type":"string","description":"Node ID to move"},"parent":{"type":"string","description":"New parent node ID"},"index":{"type":"string","description":"Position index within parent (optional)"}},"required":["nodeId","parent"]}"#,
        ),
        def(
            "delete_node",
            "Delete a node (and all its children) from the document. Use when the user asks to remove, delete, or clear elements. Always call batch_get first to find the correct node ID before deleting.",
            r#"{"type":"object","properties":{"nodeId":{"type":"string","description":"Node ID to delete"}},"required":["nodeId"]}"#,
        ),
    ]
}

/// Execute one chat tool call against the live editor state. Returns
/// the TS-shaped tool result (`{"success":…}`) plus whether the call
/// mutated the document (caller marks the redraw dirty).
///
/// Mirrors `mcp_serve.rs`'s per-call lifecycle: rebuild the registry
/// against the live document (read-tool snapshots see prior writes),
/// dispatch through the wire parser's argument discipline, then apply
/// any returned `EditorCommand` via `EditorState::apply` — the same
/// pre-validate-then-mutate path the MCP server uses.
pub fn execute_chat_tool(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
) -> (ChatToolResult, bool) {
    let registry = if name == "replace_node" {
        let mut registry = ToolRegistry::default();
        registry.register(Box::new(op_mcp::replace_node_snapshot()));
        registry
    } else {
        let Some(_level) = chat_tool_level(name) else {
            return (
                error_result(format!("tool not available in chat: {name}")),
                false,
            );
        };
        chat_tool_registry(state, name)
    };
    execute_with_registry(state, name, args_json, registry)
}

/// Core dispatch+apply body shared by chat and design tool executors.
///
/// Normalises `args_json`, builds the JSON-RPC wire line, dispatches
/// through `op_mcp::parse_tool_call` + `registry.dispatch`, and applies
/// any returned `EditorCommand` via `EditorState::apply`. Returns the
/// TS-shaped result envelope plus a mutation flag.
pub(crate) fn execute_with_registry(
    state: &mut EditorState,
    name: &str,
    args_json: &str,
    registry: ToolRegistry,
) -> (ChatToolResult, bool) {
    // Ride the real wire parser so structured-argument discipline
    // (which keys may carry objects/arrays) matches the MCP server.
    let args = if args_json.trim().is_empty() {
        "{}".to_string()
    } else {
        args_json.trim().to_string()
    };
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":{},"arguments":{args}}}}}"#,
        serde_json::Value::String(name.to_string())
    );
    let Some(call) = op_mcp::parse_tool_call(&line) else {
        return (
            error_result(format!(
                "malformed tool call for {name}: arguments must be a JSON object of scalar values (plus the documented object-valued keys)"
            )),
            false,
        );
    };
    let response = registry.dispatch(call);
    match response {
        ToolResponse::Ok {
            result,
            command,
            json,
            ..
        } => {
            let mut mutated = false;
            if let Some(cmd) = command {
                if state.apply(cmd.clone()) {
                    mutated = true;
                } else {
                    return (
                        error_result(format!("host rejected command for {name}")),
                        false,
                    );
                }
            }
            let data = match json {
                Some(raw) => serde_json::from_str::<serde_json::Value>(&raw)
                    .unwrap_or(serde_json::Value::String(raw)),
                None => serde_json::to_value(&result).unwrap_or(serde_json::Value::Null),
            };
            let envelope = serde_json::json!({ "success": true, "data": data });
            (
                ChatToolResult {
                    content: envelope.to_string(),
                    is_error: false,
                },
                mutated,
            )
        }
        ToolResponse::Err { message, .. } => (error_result(message), false),
    }
}

fn error_result(message: String) -> ChatToolResult {
    let envelope = serde_json::json!({ "success": false, "error": message });
    ChatToolResult {
        content: envelope.to_string(),
        is_error: true,
    }
}

/// Apply a DESIGN_MODIFY result to the live document. Top-level nodes
/// whose `id` already exists replace that whole subtree; unknown or
/// id-less nodes insert as new top-level elements under the active
/// page's primary frame (TS `getActivePagePrimaryFrameId`,
/// design-canvas-ops.ts:86-94). Inserts and replacements dispatch
/// through MCP tool validation before applying commands. Returns
/// `(applied_count, mutated)`.
///
/// Documented divergence: TS wraps the loop in one history batch;
/// here every node is its own undo step — the same granularity the
/// Rust design pipeline has until host batch mode lands
/// (design_session.rs `BeginUndoBatch` TODO).
pub fn apply_design_modification(
    state: &mut EditorState,
    nodes: &[DesignModificationOp],
) -> (usize, bool) {
    let mut count = 0usize;
    let mut mutated = false;
    for (parent, node) in nodes {
        let id = node.get("id").and_then(|v| v.as_str());
        let parent_exists = parent != "null" && node_exists(state, parent);
        let (applied, did_mutate) = if parent_exists {
            insert_modify_subtree(state, node, Some(parent.as_str()))
        } else if parent == "null" && id.is_some_and(|id| node_exists(state, id)) {
            replace_modify_subtree(state, node, id.expect("checked above"))
        } else {
            insert_modify_subtree(state, node, None)
        };
        count += applied;
        mutated |= did_mutate;
    }
    (count, mutated)
}

pub fn parse_design_modification_ops_arg(args_json: &str) -> Vec<DesignModificationOp> {
    let Ok(args) = serde_json::from_str::<serde_json::Value>(args_json) else {
        return Vec::new();
    };
    let Some(nodes) = args.get("nodes") else {
        return Vec::new();
    };
    serde_json::from_value::<Vec<DesignModificationOp>>(nodes.clone()).unwrap_or_else(|_| {
        nodes
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .cloned()
                    .map(|node| ("null".to_string(), node))
                    .collect()
            })
            .unwrap_or_default()
    })
}

fn replace_modify_subtree(
    state: &mut EditorState,
    node: &serde_json::Value,
    node_id: &str,
) -> (usize, bool) {
    let target_id = op_editor_core::NodeId::new(node_id);
    let Some(path) = node_index_path(state.active_children(), &target_id) else {
        return (0, false);
    };
    let mut preserved_ids = HashSet::new();
    if let Some(existing) = op_editor_core::walkers::find_node(state.active_children(), &target_id)
    {
        collect_subtree_ids(existing, &mut preserved_ids);
    }

    let mut incoming = node.clone();
    backfill_placeholder_image_srcs(&mut incoming, state);
    let args = serde_json::json!({
        "nodeId": node_id,
        "data": incoming,
        "drop_children": true
    });
    let (result, mutated) = execute_chat_tool(state, "replace_node", &args.to_string());
    if !result.is_error {
        if let Some(replaced) = node_mut_at_path(state.active_children_mut(), &path) {
            let mut used = HashSet::new();
            restore_existing_subtree_ids(replaced, node, &preserved_ids, &mut used);
        }
        return (1, mutated);
    }
    eprintln!(
        "[AI] design modification replace_node failed: {}",
        result.content
    );
    (0, mutated)
}

fn node_index_path(
    nodes: &[jian_ops_schema::node::PenNode],
    target: &op_editor_core::NodeId,
) -> Option<Vec<usize>> {
    fn walk(
        nodes: &[jian_ops_schema::node::PenNode],
        target: &op_editor_core::NodeId,
        path: &mut Vec<usize>,
    ) -> bool {
        for (idx, node) in nodes.iter().enumerate() {
            path.push(idx);
            if node.id_str() == target.as_str() {
                return true;
            }
            if let Some(children) = node.children() {
                if walk(children, target, path) {
                    return true;
                }
            }
            path.pop();
        }
        false
    }

    let mut path = Vec::new();
    walk(nodes, target, &mut path).then_some(path)
}

fn node_mut_at_path<'a>(
    nodes: &'a mut [jian_ops_schema::node::PenNode],
    path: &[usize],
) -> Option<&'a mut jian_ops_schema::node::PenNode> {
    let (idx, rest) = path.split_first()?;
    let node = nodes.get_mut(*idx)?;
    if rest.is_empty() {
        return Some(node);
    }
    node.children_mut()
        .and_then(|children| node_mut_at_path(children, rest))
}

fn collect_subtree_ids(node: &jian_ops_schema::node::PenNode, ids: &mut HashSet<String>) {
    ids.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            collect_subtree_ids(child, ids);
        }
    }
}

fn restore_existing_subtree_ids(
    node: &mut jian_ops_schema::node::PenNode,
    incoming: &serde_json::Value,
    preserved_ids: &HashSet<String>,
    used: &mut HashSet<String>,
) {
    if let Some(id) = incoming
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|id| preserved_ids.contains(*id) && used.insert((*id).to_string()))
    {
        node.base_mut().id = id.to_string();
    }

    let incoming_children = incoming.get("children").and_then(|v| v.as_array());
    if let (Some(children), Some(incoming_children)) = (node.children_mut(), incoming_children) {
        for (child, incoming_child) in children.iter_mut().zip(incoming_children) {
            restore_existing_subtree_ids(child, incoming_child, preserved_ids, used);
        }
    }
}

fn backfill_placeholder_image_srcs(incoming: &mut serde_json::Value, state: &EditorState) {
    match incoming {
        serde_json::Value::Object(obj) => {
            let replacement = obj
                .get("src")
                .and_then(|src| (src.as_str() == Some("<image>")).then_some(()))
                .and_then(|_| obj.get("id").and_then(|id| id.as_str()))
                .and_then(|id| existing_real_image_src(state, id));
            if let Some(src) = replacement {
                obj.insert("src".into(), serde_json::Value::String(src));
            }
            for value in obj.values_mut() {
                backfill_placeholder_image_srcs(value, state);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                backfill_placeholder_image_srcs(item, state);
            }
        }
        _ => {}
    }
}

fn existing_real_image_src(state: &EditorState, id: &str) -> Option<String> {
    let node = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new(id),
    )?;
    let jian_ops_schema::node::PenNode::Image(image) = node else {
        return None;
    };
    let src = image.src.as_str();
    is_real_image_src(src).then(|| src.to_string())
}

fn is_real_image_src(src: &str) -> bool {
    src.starts_with("data:") || src.starts_with("http://") || src.starts_with("https://")
}

fn insert_modify_subtree(
    state: &mut EditorState,
    node: &serde_json::Value,
    parent_id: Option<&str>,
) -> (usize, bool) {
    let mut args = serde_json::json!({ "data": node });
    if let Some(parent) = parent_id
        .map(str::to_string)
        .or_else(|| primary_frame_id(state))
    {
        args["parent"] = serde_json::Value::String(parent);
    }
    let (result, mutated) = execute_chat_tool(state, "insert_node", &args.to_string());
    if !result.is_error {
        return (1, mutated);
    }
    eprintln!(
        "[AI] design modification insert_node failed: {}",
        result.content
    );
    (0, mutated)
}

fn node_exists(state: &EditorState, id: &str) -> bool {
    op_editor_core::walkers::find_node(state.active_children(), &op_editor_core::NodeId::new(id))
        .is_some()
}

fn primary_frame_id(state: &EditorState) -> Option<String> {
    use op_editor_core::PenNodeExt;

    state
        .active_children()
        .iter()
        .find(|n| matches!(n, jian_ops_schema::node::PenNode::Frame(_)))
        .map(|n| n.id_str().to_string())
}

/// Build a registry carrying only the requested chat tool — the same
/// snapshot-per-call discipline as `mcp_serve::rebuild_registry`, but
/// scoped to the chat subset.
fn chat_tool_registry(state: &EditorState, requested: &str) -> ToolRegistry {
    let mut r = ToolRegistry::default();
    match requested {
        "batch_get" => r.register(Box::new(op_mcp::batch_get_snapshot(state))),
        "snapshot_layout" => r.register(Box::new(op_mcp::snapshot_layout_snapshot(state))),
        "get_selection" => r.register(Box::new(op_mcp::selection_snapshot(state))),
        "insert_node" => r.register(Box::new(op_mcp::insert_node_snapshot())),
        "update_node" => r.register(Box::new(op_mcp::update_node_snapshot())),
        "move_node" => r.register(Box::new(op_mcp::move_node_snapshot())),
        "delete_node" => r.register(Box::new(op_mcp::delete_node_snapshot())),
        _ => {}
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::chat_provider::ChatToolExecutor;

    #[test]
    fn chat_tool_defs_match_ts_crud_subset_and_auth_levels() {
        let defs = chat_tool_defs();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "batch_get",
                "snapshot_layout",
                "get_selection",
                "insert_node",
                "update_node",
                "move_node",
                "delete_node",
            ],
            "chat tool set mirrors TS getCrudToolDefs"
        );
        // TS TOOL_AUTH_MAP parity.
        assert_eq!(chat_tool_level("batch_get"), Some("read"));
        assert_eq!(chat_tool_level("insert_node"), Some("create"));
        assert_eq!(chat_tool_level("update_node"), Some("modify"));
        assert_eq!(chat_tool_level("move_node"), Some("modify"));
        assert_eq!(chat_tool_level("delete_node"), Some("delete"));
        // Design-pipeline tools stay excluded from chat v1.
        assert_eq!(chat_tool_level("plan_layout"), None);
        assert_eq!(chat_tool_level("generate_design"), None);
        // Every schema is valid JSON.
        for d in &defs {
            serde_json::from_str::<serde_json::Value>(&d.input_schema_json)
                .unwrap_or_else(|e| panic!("schema for {} unparseable: {e}", d.name));
        }
    }

    #[test]
    fn execute_rejects_tools_outside_the_chat_set() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_chat_tool(&mut state, "delete_page", "{}");
        assert!(result.is_error);
        assert!(!mutated);
        assert!(result.content.contains("not available in chat"));
    }

    #[test]
    fn execute_read_tool_returns_success_envelope() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_chat_tool(&mut state, "get_selection", "{}");
        assert!(!result.is_error, "got {}", result.content);
        assert!(!mutated, "read tools never mutate");
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(v["success"], serde_json::Value::Bool(true));
    }

    #[test]
    fn execute_insert_then_update_recolors_node_via_apply_path() {
        // End-to-end-ish: a scripted insert_node creates a rect through
        // the EditorCommand apply path, then update_node recolors it —
        // the GAP #32 acceptance scenario ("make the title red").
        let mut state = EditorState::new();
        let (insert, mutated) = execute_chat_tool(
            &mut state,
            "insert_node",
            r##"{"kind":"rect","name":"Title","x":"10","y":"10","width":"100","height":"40","fill_hex":"#112233"}"##,
        );
        assert!(!insert.is_error, "insert failed: {}", insert.content);
        assert!(mutated, "insert must mutate the document");
        // insert_node's wire result is `{wrote:true}` — the applier
        // allocates the id, so read it back off the live document the
        // way a follow-up batch_get would see it.
        use op_editor_core::PenNodeExt;
        let node_id = state
            .active_children()
            .last()
            .map(|n| n.id_str().to_string())
            .expect("inserted node present on the active page");

        let (update, mutated) = execute_chat_tool(
            &mut state,
            "update_node",
            &format!(r##"{{"nodeId":"{node_id}","fill_hex":"#ff0000"}}"##),
        );
        assert!(!update.is_error, "update failed: {}", update.content);
        assert!(mutated, "update must mutate the document");

        let doc_json = serde_json::to_string(&state.doc).unwrap().to_lowercase();
        assert!(
            doc_json.contains("#ff0000"),
            "node fill must be recolored to #ff0000 via the apply path"
        );
    }

    #[test]
    fn execute_update_unknown_node_reports_tool_error() {
        let mut state = EditorState::new();
        let (result, mutated) = execute_chat_tool(
            &mut state,
            "update_node",
            r##"{"nodeId":"nope","fill_hex":"#ff0000"}"##,
        );
        assert!(result.is_error, "got {}", result.content);
        assert!(!mutated);
        let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(v["success"], serde_json::Value::Bool(false));
    }

    #[test]
    fn apply_modification_replaces_existing_tree_and_backfills_image_src() {
        use op_editor_core::{walkers::find_node, NodeId, PenNodeExt};

        let mut state = EditorState::new();
        state.active_children_mut().clear();
        state.active_children_mut().push(
            serde_json::from_value(serde_json::json!({
                "type": "frame",
                "id": "n100",
                "name": "Before",
                "x": -100.0,
                "y": 0.0,
                "width": 80.0,
                "height": 80.0,
                "children": []
            }))
            .expect("valid before node"),
        );
        state.active_children_mut().push(
            serde_json::from_value(serde_json::json!({
                "type": "frame",
                "id": "n217",
                "name": "Mini Player",
                "x": 0.0,
                "y": 0.0,
                "width": 320.0,
                "height": 180.0,
                "children": [
                    {
                        "type": "image",
                        "id": "n218",
                        "name": "Cover Image",
                        "src": "data:image/png;base64,REALIMAGE",
                        "x": 0.0,
                        "y": 0.0,
                        "width": 80.0,
                        "height": 80.0
                    },
                    {
                        "type": "text",
                        "id": "n220",
                        "name": "Song Title",
                        "content": "Original Title",
                        "x": 90.0,
                        "y": 0.0,
                        "width": 180.0,
                        "height": 24.0
                    }
                ]
            }))
            .expect("valid frame node"),
        );
        state.active_children_mut().push(
            serde_json::from_value(serde_json::json!({
                "type": "frame",
                "id": "n300",
                "name": "After",
                "x": 400.0,
                "y": 0.0,
                "width": 80.0,
                "height": 80.0,
                "children": []
            }))
            .expect("valid after node"),
        );

        let nodes = vec![(
            "null".to_string(),
            serde_json::json!({
                "type": "frame",
                "id": "n217",
                "name": "Mini Player Rewritten",
                "children": [
                    {
                        "type": "image",
                        "id": "n218",
                        "name": "Cover Image Rewritten",
                        "src": "<image>",
                        "width": 10.0,
                        "height": 10.0
                    },
                    {
                        "type": "text",
                        "id": "n220",
                        "name": "Song Title Rewritten",
                        "content": "B"
                    },
                    {
                        "type": "frame",
                        "name": "Progress Bar",
                        "width": 220.0,
                        "height": 8.0,
                        "children": []
                    }
                ]
            }),
        )];
        let (count, mutated) = apply_design_modification(&mut state, &nodes);

        assert_eq!(count, 1);
        assert!(mutated);
        assert_eq!(state.active_children()[0].id_str(), "n100");
        assert_eq!(state.active_children()[1].id_str(), "n217");
        assert_eq!(state.active_children()[2].id_str(), "n300");
        let mini_player = find_node(state.active_children(), &NodeId::new("n217"))
            .expect("existing mini player remains");
        let mini_json = serde_json::to_value(mini_player).expect("mini player serializes");
        assert_eq!(
            mini_json["name"],
            serde_json::json!("Mini Player Rewritten")
        );
        let children = mini_player.children().expect("mini player children");
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].id_str(), "n218");
        assert_eq!(children[1].id_str(), "n220");
        assert_eq!(children[2].base().name.as_deref(), Some("Progress Bar"));

        let image =
            find_node(state.active_children(), &NodeId::new("n218")).expect("cover image remains");
        let image_json = serde_json::to_value(image).expect("image serializes");
        assert_eq!(
            image_json["name"],
            serde_json::json!("Cover Image Rewritten")
        );
        assert_eq!(
            image_json["src"],
            serde_json::json!("data:image/png;base64,REALIMAGE")
        );
        assert_eq!(image_json["width"], serde_json::json!(10.0));

        let title =
            find_node(state.active_children(), &NodeId::new("n220")).expect("title remains");
        let title_json = serde_json::to_value(title).expect("title serializes");
        assert_eq!(
            title_json["name"],
            serde_json::json!("Song Title Rewritten")
        );
        assert_eq!(title_json["content"], serde_json::json!("B"));

        fn count_id(nodes: &[jian_ops_schema::node::PenNode], id: &str) -> usize {
            nodes
                .iter()
                .map(|node| {
                    usize::from(node.id_str() == id)
                        + node.children().map(|kids| count_id(kids, id)).unwrap_or(0)
                })
                .sum()
        }
        assert_eq!(count_id(state.active_children(), "n217"), 1);
        assert_eq!(count_id(state.active_children(), "n218"), 1);
        assert_eq!(count_id(state.active_children(), "n220"), 1);
    }

    #[test]
    fn ui_executor_round_trips_through_the_channel() {
        // Worker side blocks on the ack while the "UI thread" (this
        // test) drains the request and executes against live state —
        // the full pending/apply channel discipline minus winit.
        let (executor, rx) = chat_tool_channel();
        let worker = std::thread::spawn(move || executor.execute("get_selection", "{}"));
        let req = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("worker forwards the tool request");
        assert_eq!(req.name, "get_selection");
        let mut state = EditorState::new();
        let (result, _) = execute_chat_tool(&mut state, &req.name, &req.args_json);
        req.ack.send(result).unwrap();
        let got = worker.join().unwrap();
        assert!(!got.is_error);
        assert!(got.content.contains("\"success\":true"));
    }

    #[test]
    fn ui_executor_reports_abort_when_session_dropped() {
        let (executor, rx) = chat_tool_channel();
        drop(rx); // session went away
        let result = executor.execute("batch_get", "{}");
        assert!(result.is_error);
        assert!(result.content.contains("aborted"));
    }
}
