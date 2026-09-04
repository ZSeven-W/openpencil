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

use crate::chat_modify_sanitize::sanitize_modify_replacement;
use crate::chat_tool_result::{rolled_back_transaction_message, structured_error_result};
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
pub fn execute_with_registry(
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
            let data = match json {
                Some(raw) => serde_json::from_str::<serde_json::Value>(&raw)
                    .unwrap_or(serde_json::Value::String(raw)),
                None => serde_json::to_value(&result).unwrap_or(serde_json::Value::Null),
            };
            // `batch_design` reports transactional rollbacks as structured
            // JSON so callers retain the failing lines and resend hint. That
            // is still a tool failure: no command landed and the agent loop
            // must receive `is_error`, otherwise the transcript paints a
            // green success card for an unchanged document.
            if let Some(message) = rolled_back_transaction_message(&data) {
                return (structured_error_result(message, data), false);
            }
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

/// Apply a DESIGN_MODIFY result inside the Frames selected when the turn
/// started. Existing nodes may only be replaced inside those subtrees;
/// explicit parents must also be in scope. Unknown or id-less nodes can use
/// an implicit parent only when exactly one target Frame was captured.
/// Inserts and replacements dispatch through MCP tool validation before
/// applying commands. Returns `(applied_count, mutated)`.
///
/// Documented divergence: TS wraps the loop in one history batch;
/// here every node is its own undo step — the same granularity the
/// Rust design pipeline has until host batch mode lands
/// (design_session.rs `BeginUndoBatch` TODO).
pub fn apply_design_modification(
    state: &mut EditorState,
    nodes: &[DesignModificationOp],
    target_frame_ids: &[String],
) -> (usize, bool) {
    if !valid_modify_scope(state, target_frame_ids) {
        return (0, false);
    }
    let implicit_parent = (target_frame_ids.len() == 1).then(|| target_frame_ids[0].as_str());
    let mut count = 0usize;
    let mut mutated = false;
    for (parent, node) in nodes {
        let id = node.get("id").and_then(|v| v.as_str());
        let explicit_parent = parent != "null";
        let parent_in_scope = explicit_parent
            && node_exists(state, parent)
            && node_is_in_modify_scope(state, target_frame_ids, parent);
        let existing_id = id.filter(|id| node_exists(state, id));
        let (applied, did_mutate) = if parent_in_scope {
            insert_modify_subtree(state, node, Some(parent.as_str()))
        } else if explicit_parent {
            (0, false)
        } else if let Some(existing_id) = existing_id {
            if node_is_in_modify_scope(state, target_frame_ids, existing_id) {
                replace_modify_subtree(state, node, existing_id)
            } else {
                (0, false)
            }
        } else if let Some(parent) = implicit_parent {
            insert_modify_subtree(state, node, Some(parent))
        } else {
            (0, false)
        };
        count += applied;
        mutated |= did_mutate;
    }
    (count, mutated)
}

fn valid_modify_scope(state: &EditorState, target_frame_ids: &[String]) -> bool {
    !target_frame_ids.is_empty()
        && target_frame_ids.iter().all(|id| {
            op_editor_core::walkers::find_node(
                state.active_children(),
                &op_editor_core::NodeId::new(id),
            )
            .is_some_and(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
        })
}

fn subtree_contains_id(node: &jian_ops_schema::node::PenNode, target_id: &str) -> bool {
    node.id_str() == target_id
        || node.children().is_some_and(|children| {
            children
                .iter()
                .any(|child| subtree_contains_id(child, target_id))
        })
}

fn node_is_in_modify_scope(
    state: &EditorState,
    target_frame_ids: &[String],
    node_id: &str,
) -> bool {
    target_frame_ids.iter().any(|frame_id| {
        op_editor_core::walkers::find_node(
            state.active_children(),
            &op_editor_core::NodeId::new(frame_id),
        )
        .is_some_and(|frame| subtree_contains_id(frame, node_id))
    })
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

pub fn parse_design_modification_target_frame_ids_arg(args_json: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(args_json)
        .ok()
        .and_then(|args| args.get("targetFrameIds").cloned())
        .and_then(|ids| serde_json::from_value(ids).ok())
        .unwrap_or_default()
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
    let existing_json = if let Some(existing) =
        op_editor_core::walkers::find_node(state.active_children(), &target_id)
    {
        collect_subtree_ids(existing, &mut preserved_ids);
        serde_json::to_value(existing).ok()
    } else {
        None
    };

    let mut incoming = node.clone();
    backfill_placeholder_image_srcs(&mut incoming, state);
    if let Some(existing) = existing_json.as_ref() {
        sanitize_modify_replacement(&mut incoming, existing);
    }
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
    // Direct Modify's `I(parent, node)` protocol is append-only. Mobile
    // status bars have a structural first-child contract, so repair that one
    // unambiguous case here while leaving every ordinary ADD append-only.
    let status_bar_parent = parent_id
        .filter(|parent| is_mobile_vertical_root(state, parent))
        .filter(|_| status_bar_like_json(node));
    let existing_status_bar = status_bar_parent.and_then(|parent| {
        op_editor_core::walkers::find_node(
            state.active_children(),
            &op_editor_core::NodeId::new(parent),
        )
        .and_then(|root| root.children())
        .and_then(|children| {
            children
                .iter()
                .enumerate()
                .find(|(_, child)| status_bar_like_node(child))
                .map(|(index, child)| {
                    (
                        child.id_str().to_string(),
                        index,
                        child.base().role.as_deref() != Some("status-bar"),
                    )
                })
        })
    });
    if let (Some(parent), Some((node_id, index, role_missing))) =
        (status_bar_parent, existing_status_bar)
    {
        let mut mutated = false;
        if role_missing {
            mutated |= state.apply(op_editor_core::EditorCommand::PatchNodeData {
                node_id: op_editor_core::NodeId::new(node_id.clone()),
                patch_json: r#"{"role":"status-bar"}"#.to_string(),
                page_id: None,
            });
        }
        if index > 0 {
            mutated |= state.apply(op_editor_core::EditorCommand::MoveNode {
                node_id: op_editor_core::NodeId::new(node_id),
                target_parent: op_editor_core::NodeId::new(parent),
                page_id: None,
                index: Some(0),
            });
        }
        return (1, mutated);
    }
    let mut incoming = node.clone();
    if status_bar_parent.is_some() {
        if let Some(object) = incoming.as_object_mut() {
            object.insert(
                "role".into(),
                serde_json::Value::String("status-bar".into()),
            );
        }
    }
    let Some(parent) = parent_id else {
        return (0, false);
    };
    let mut args = serde_json::json!({ "data": incoming });
    args["parent"] = serde_json::Value::String(parent.to_string());
    let (result, mut mutated) = execute_chat_tool(state, "insert_node", &args.to_string());
    if !result.is_error {
        if let Some(parent) = status_bar_parent {
            let inserted_status_bar = op_editor_core::walkers::find_node(
                state.active_children(),
                &op_editor_core::NodeId::new(parent),
            )
            .and_then(|root| root.children())
            .and_then(|children| {
                children
                    .iter()
                    .find(|child| status_bar_like_node(child))
                    .map(|child| child.id_str().to_string())
            });
            if let Some(node_id) = inserted_status_bar {
                mutated |= state.apply(op_editor_core::EditorCommand::MoveNode {
                    node_id: op_editor_core::NodeId::new(node_id),
                    target_parent: op_editor_core::NodeId::new(parent),
                    page_id: None,
                    index: Some(0),
                });
            }
        }
        return (1, mutated);
    }
    eprintln!(
        "[AI] design modification insert_node failed: {}",
        result.content
    );
    (0, mutated)
}

fn is_mobile_vertical_root(state: &EditorState, id: &str) -> bool {
    state.active_children().iter().any(|node| {
        node.id_str() == id
            && node
                .width_px()
                .is_some_and(|width| (300.0..=480.0).contains(&width))
            && matches!(
                node,
                jian_ops_schema::node::PenNode::Frame(frame)
                    if frame.container.layout
                        == Some(jian_ops_schema::node::container::LayoutMode::Vertical)
            )
    })
}

fn status_bar_like_json(node: &serde_json::Value) -> bool {
    node.get("role").and_then(serde_json::Value::as_str) == Some("status-bar")
        || ["name", "id"].iter().any(|key| {
            node.get(*key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(status_bar_like_text)
        })
}

fn status_bar_like_node(node: &jian_ops_schema::node::PenNode) -> bool {
    node.base().role.as_deref() == Some("status-bar")
        || node
            .base()
            .name
            .as_deref()
            .is_some_and(status_bar_like_text)
        || status_bar_like_text(node.id_str())
}

fn status_bar_like_text(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase().replace(['_', '-'], " ");
    normalized.contains("status bar")
        || normalized.contains("statusbar")
        || normalized.contains("状态栏")
        || normalized.contains("系统栏")
}

fn node_exists(state: &EditorState, id: &str) -> bool {
    op_editor_core::walkers::find_node(state.active_children(), &op_editor_core::NodeId::new(id))
        .is_some()
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
#[path = "chat_canvas_tools_tests.rs"]
mod tests;
