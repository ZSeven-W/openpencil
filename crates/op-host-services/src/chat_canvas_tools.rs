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
use op_editor_core::EditorState;
pub use op_editor_host_core::chat::{chat_tool_channel, ChatToolRequest, UiChatToolExecutor};
use op_mcp::{ToolRegistry, ToolResponse};

/// TS `maxTurns` for the chat agent loop (`ai-chat-handlers.ts:254`).
pub const MAX_TOOL_TURNS: usize = 20;

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
    let Some(_level) = chat_tool_level(name) else {
        return (
            error_result(format!("tool not available in chat: {name}")),
            false,
        );
    };
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
    let registry = chat_tool_registry(state, name);
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

/// Apply a DESIGN_MODIFY result to the live document — port of TS
/// `extractAndApplyDesignModification` (design-canvas-ops.ts:589-618):
/// nodes whose `id` already exists are updated in place; unknown ids
/// are inserted under the active page's primary frame (TS
/// `getActivePagePrimaryFrameId`, design-canvas-ops.ts:86-94), or the
/// page root when the page has no frame. Each node dispatches through
/// [`execute_chat_tool`] (`update_node` / `insert_node`) so validation
/// matches the MCP path. Returns `(applied_count, mutated)`.
///
/// Documented divergence: TS wraps the loop in one history batch;
/// here every node is its own undo step — the same granularity the
/// Rust design pipeline has until host batch mode lands
/// (design_session.rs `BeginUndoBatch` TODO).
pub fn apply_design_modification(
    state: &mut EditorState,
    nodes: &[serde_json::Value],
) -> (usize, bool) {
    use op_editor_core::pen_node_ext::PenNodeExt;

    let mut count = 0usize;
    let mut mutated = false;
    for node in nodes {
        let id = node.get("id").and_then(|v| v.as_str()).map(str::to_string);
        let exists = id.as_deref().is_some_and(|id| {
            op_editor_core::walkers::find_node(
                state.active_children(),
                &op_editor_core::NodeId::new(id),
            )
            .is_some()
        });
        let (tool, args) = if exists {
            (
                "update_node",
                serde_json::json!({ "nodeId": id, "data": node }),
            )
        } else {
            // TS: parent the implied-new node to the active page's
            // primary frame; null parent falls to the page root.
            let parent = state
                .active_children()
                .iter()
                .find(|n| matches!(n, jian_ops_schema::node::PenNode::Frame(_)))
                .map(|n| n.id_str().to_string());
            let mut args = serde_json::json!({ "data": node });
            if let Some(parent) = parent {
                args["parent"] = serde_json::Value::String(parent);
            }
            ("insert_node", args)
        };
        let (result, did_mutate) = execute_chat_tool(state, tool, &args.to_string());
        if did_mutate {
            mutated = true;
        }
        if !result.is_error {
            count += 1;
        } else {
            // Best-effort apply (TS loop never aborts): log the
            // per-node failure for diagnosis and continue.
            eprintln!("[AI] design modification {tool} failed: {}", result.content);
        }
    }
    (count, mutated)
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
