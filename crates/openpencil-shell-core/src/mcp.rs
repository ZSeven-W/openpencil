//! MCP (Model Context Protocol) request / response types.
//! Mirrors the wire shape `packages/pen-mcp` uses for its stdio +
//! HTTP server. v1 scope: protocol types + tool registry trait.
//! Real stdio listener + HTTP server land in `openpencil-desktop`
//! (or a dedicated `openpencil-mcp` binary) once the routing
//! decisions are made; the data shape here lets that work proceed
//! without redesign.

use std::collections::BTreeMap;

pub mod parser;
pub mod tools;
#[cfg(test)] mod tools_tests;
pub mod write_tools;
pub mod batch_design;
pub mod scalar_vars;
#[cfg(test)] mod write_tools_tests;
#[cfg(test)] mod copy_node_tests;
#[cfg(test)] mod replace_node_tests;
#[cfg(test)] mod batch_design_tests;
#[cfg(test)] mod scalar_vars_tests;

// Re-export the public surface of submodules so callers can keep
// using `mcp::parse_tool_call` / `mcp::GetDocumentInfo` after the
// split. Mirrors the `widgets::*` re-export pattern.
pub use parser::parse_tool_call;
pub use tools::{
    document_info_snapshot, get_active_theme_snapshot, get_node_snapshot,
    list_components_snapshot, list_pages_snapshot, list_variables_snapshot, selection_snapshot,
    GetActiveTheme, GetDocumentInfo, GetNode, GetSelection, ListComponents, ListPages,
    ListVariables, NodeRecord, VariableRecord,
};
pub use write_tools::{
    copy_node_snapshot, create_component_snapshot, delete_component_snapshot,
    delete_node_snapshot, insert_node_snapshot, instantiate_component_snapshot,
    move_node_snapshot, replace_node_snapshot, set_active_axis_value_snapshot,
    set_variable_color_snapshot, update_node_snapshot, CopyNode, CreateComponent,
    DeleteComponent, DeleteNode, InsertNode, InstantiateComponent, MoveNode, ReplaceNode,
    SetActiveAxisValue, SetVariableColor, UpdateNode,
};
pub use batch_design::{
    batch_design_snapshot, design_content_snapshot, design_refine_snapshot,
    design_skeleton_snapshot, BatchDesign, DesignContent, DesignRefine, DesignSkeleton,
};
pub use scalar_vars::{
    set_variable_boolean_snapshot, set_variable_number_snapshot,
    set_variable_string_snapshot, SetVariableBoolean, SetVariableNumber, SetVariableString,
};

/// JSON-RPC-style request id. Strings + integers both supported by
/// the spec; we accept either over the wire.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestId {
    Str(String),
    Num(i64),
}

/// Inbound tool invocation. `tool` is the registered tool name
/// (`insert_node`, `batch_design`, `design_skeleton`, etc); `arguments`
/// is the JSON object the tool expects.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: RequestId,
    pub tool: String,
    pub arguments: BTreeMap<String, String>,
}

/// Tool response — either a structured result object or an error.
/// Errors are typed enough for the LLM client to recover (e.g.
/// `MissingArgument` vs `InvalidArgument` vs `ToolFailed`).
#[derive(Debug, Clone, PartialEq)]
pub enum ToolResponse {
    Ok {
        id: RequestId,
        result: BTreeMap<String, String>,
        /// Optional mutation the host should apply to the live
        /// document. Write tools return one of these via
        /// `ToolOutcome::OkWithCommand`; read tools return None.
        /// The registry surfaces it so callers don't need to
        /// re-walk the tool list.
        command: Option<McpCommand>,
    },
    Err {
        id: RequestId,
        code: ToolErrorCode,
        message: String,
    },
}

/// Tool failure kind — matches JSON-RPC error categories. The MCP
/// server maps these to standard codes when serialising.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorCode {
    MissingArgument,
    InvalidArgument,
    ToolFailed,
    UnknownTool,
    Internal,
}

/// Result of a tool's work — content + payload only. The
/// `ToolRegistry::dispatch` wrapper attaches the originating
/// `RequestId` so a misbehaving tool literally can't mint a
/// wrong id (codex BLOCK: passing `&ToolCall` to tools left id
/// preservation as a convention only; this shape enforces it
/// structurally).
///
/// `OkWithCommand` carries a `McpCommand` the host applies AFTER
/// dispatch: the tool stays `&self` (so the registry doesn't
/// need `Arc<Mutex<Document>>`), but write tools can still
/// describe their intent + the host serializes the mutation
/// against the live document.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutcome {
    Ok(BTreeMap<String, String>),
    OkWithCommand(BTreeMap<String, String>, McpCommand),
    Err(ToolErrorCode, String),
}

/// Document mutation a tool wants the host to apply. The tool
/// validates its arguments (`call(&self, ...)`) and returns
/// `OkWithCommand(result, command)`; the host then calls
/// `Document::apply_mcp_command(command)`. This pattern keeps
/// the `McpTool` trait `&self` (so trait objects + the registry
/// stay simple) while still admitting write tools.
///
/// Variants extend as write tools land. Today:
/// - `SetVariableColor { name, hex }` — routes through
///   `VariableTable::set_color_hex` with its full correctness
///   chain (subset match / no default clobber / no other-axis
///   shadow / history snapshot).
/// - `SetActiveAxisValue { axis, value }` — pins a theme axis
///   directly (vs. `cycle_active_axis_value` which advances).
/// - `InsertNode { ... }` — creates a fresh node on the active
///   page with the supplied bounds + fill. The applier
///   allocates an id past `max_node_id()` so it can't collide
///   with existing nodes even after deletes.
#[derive(Debug, Clone, PartialEq)]
pub enum McpCommand {
    SetVariableColor {
        name: String,
        hex: String,
    },
    SetActiveAxisValue {
        axis: String,
        value: String,
    },
    InsertNode {
        kind: String,
        name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: Option<String>,
    },
    /// Patch fields on an existing node. Every field is optional;
    /// `None` leaves the live value unchanged. Bounds writes
    /// replace coordinates piecemeal — caller can move (x, y)
    /// without resizing or resize (w, h) without moving.
    UpdateNode {
        node_id: u64,
        x: Option<i32>,
        y: Option<i32>,
        width: Option<i32>,
        height: Option<i32>,
        name: Option<String>,
        fill_hex: Option<String>,
    },
    /// Remove a node + all its descendants from its parent. The
    /// applier walks pages to find the parent vec containing the
    /// id and `retain`s it out. Returns false when the id doesn't
    /// resolve OR points at a Page root (use page mutators for
    /// page deletion).
    DeleteNode { node_id: u64 },
    /// Reparent a node. `target_parent_id == 0` reparents to the
    /// page root of the currently-active page. Non-zero ids must
    /// resolve to an existing node; the applier rejects moves
    /// where the target would create a cycle (target is a
    /// descendant of the moved node).
    MoveNode {
        node_id: u64,
        target_parent_id: u64,
    },
    /// Deep-clone a node + every descendant under a new parent.
    /// `target_parent_id == 0` puts the copy at the active page
    /// root. Fresh ids are allocated by the applier starting past
    /// `max_node_id()` so the clone can't collide with any live
    /// node. The wire result is `{"wrote": "true"}` only — clone
    /// ids aren't surfaced today because `ToolOutcome` is built
    /// before apply runs; callers that need the new ids must
    /// re-query (e.g. `list_pages` + walk). A future patch may
    /// thread the allocator back to the tool to surface
    /// `new_root_id`, but the current contract is fire-and-forget.
    CopyNode {
        node_id: u64,
        target_parent_id: u64,
    },
    /// Replace an existing node with a freshly-built one at the
    /// same parent slot + same index. Captures the same shape as
    /// `InsertNode` (kind / name / bounds / fill_hex) plus the
    /// target `node_id` to swap. Useful when the LLM wants to
    /// change kind (rect → ellipse) or radically alter the node
    /// in one atomic op rather than via incremental `UpdateNode`
    /// patches. The new node gets a fresh id past `max_node_id()`
    /// so the wire response is still fire-and-forget (callers
    /// re-query to learn the new id).
    ///
    /// Bounded scope: today only the leaf-style fields land on
    /// the replacement node. A future patch may grow this to
    /// carry children / a full subtree once a JSON Node parser
    /// lives on the host. The current contract matches what TS
    /// `replace_node` accepts for primitives, minus children.
    ///
    /// **Destructive on containers**: replacing a Frame / Group
    /// / node-with-children drops every descendant of the old
    /// node. To prevent silent data loss the applier REFUSES the
    /// swap when the target has children unless `drop_children`
    /// is set to `true`. Callers that genuinely want to discard
    /// the subtree must opt in; otherwise use `update_node` to
    /// patch a container in place.
    ReplaceNode {
        node_id: u64,
        kind: String,
        name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: Option<String>,
        /// Required confirmation when the target has children.
        /// `false` (the default) makes the applier refuse a
        /// destructive swap; `true` is explicit consent.
        drop_children: bool,
    },
    /// Insert N leaf nodes on the active page in one atomic
    /// shot. Mirrors TS `batch_design` for the leaf subset
    /// (frame / group / rect / ellipse / polygon / line / text /
    /// path). Apply path validates EVERY descriptor before any
    /// mutation; a single bad entry rejects the whole batch so
    /// callers never see a partial design. Each emitted node
    /// gets a fresh non-colliding id from `next_node_id_seed`,
    /// advanced for the batch.
    BatchInsert {
        items: Vec<BatchInsertItem>,
    },
    /// Set a non-color scalar variable's value (Number / String /
    /// Boolean). Mirrors `SetVariableColor` for the other three
    /// `VariableKind`s. The applier routes through
    /// `VariableTable::set_scalar` which honors active-theme
    /// routing identically to set_color_hex.
    SetVariableScalar {
        name: String,
        scalar: VariableScalarPayload,
    },
    /// Instantiate a registered component on the active page. The
    /// applier deep-clones the component's root subtree with fresh
    /// ids past `max_node_id()` and appends it to the active page's
    /// top-level children. Mirrors TS's drag-from-Components-panel
    /// insertion. Returns false at apply time when the component
    /// id is unknown.
    InstantiateComponent {
        component_id: u64,
    },
    /// Promote an existing Frame / Group node to a registered
    /// component. `node_id` must resolve on the active page +
    /// must be a Frame or Group (the applier rejects other
    /// kinds). The component is keyed by the node's id; instances
    /// later spawn via `InstantiateComponent`.
    CreateComponent {
        node_id: u64,
        name: String,
    },
    /// Remove a component from the registry by id. Live instances
    /// already dropped on the page are NOT affected — they're
    /// independent clones at apply time.
    DeleteComponent {
        component_id: u64,
    },
}

/// Wire-friendly value payload for `McpCommand::SetVariableScalar`.
/// Mirrors `document::VariableScalar` but stays a plain enum so
/// the wire layer doesn't reach into shell-core's document API.
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScalarPayload {
    Number(f64),
    String(String),
    Boolean(bool),
}

/// Per-item descriptor for `McpCommand::BatchInsert`. Same shape
/// as `InsertNode`'s args; carried in a Vec so the applier can
/// validate-then-mutate the whole set atomically.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchInsertItem {
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub fill_hex: Option<String>,
}

/// Trait every MCP tool implements. The MCP server walks its
/// `ToolRegistry`, looks up the requested tool, and forwards the
/// arguments. Tools return a `ToolOutcome`; the registry wraps it
/// with the originating request id to produce a `ToolResponse`.
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome;
}

/// Registry — owned by the MCP server. v1 is a plain HashMap; a
/// future version may add priority / per-tool auth / rate limits.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn McpTool>>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: Box<dyn McpTool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }
    pub fn dispatch(&self, call: ToolCall) -> ToolResponse {
        // The registry — not the tool — stamps the response id. Tools
        // never see the id; their `ToolOutcome` is content-only. This
        // makes id mismatch structurally impossible (codex BLOCK:
        // passing the id to tools left enforcement as convention).
        let Some(tool) = self.tools.get(&call.tool) else {
            return ToolResponse::Err {
                id: call.id,
                code: ToolErrorCode::UnknownTool,
                message: format!("unknown tool: {}", call.tool),
            };
        };
        match tool.call(&call.arguments) {
            ToolOutcome::Ok(result) => ToolResponse::Ok {
                id: call.id,
                result,
                command: None,
            },
            ToolOutcome::OkWithCommand(result, command) => ToolResponse::Ok {
                id: call.id,
                result,
                command: Some(command),
            },
            ToolOutcome::Err(code, message) => ToolResponse::Err {
                id: call.id,
                code,
                message,
            },
        }
    }
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// JSON-RPC wire serialiser for `ToolResponse`. Manual emitter so
/// shell-core stays serde-free (no dep adds for wasm32). Produces
/// the standard `{"jsonrpc": "2.0", "id": ..., "result": ...}` /
/// `{"jsonrpc": "2.0", "id": ..., "error": {"code": ..., "message"
/// ...}}` shape any MCP client expects.
pub fn response_to_json(r: &ToolResponse) -> String {
    let (id_repr, body) = match r {
        ToolResponse::Ok { id, result, .. } => (
            id_to_json(id),
            format!(r#""result":{}"#, btree_to_json(result)),
        ),
        ToolResponse::Err { id, code, message } => (
            id_to_json(id),
            format!(
                r#""error":{{"code":{},"message":{}}}"#,
                error_code_to_int(*code),
                json_escape(message),
            ),
        ),
    };
    format!(r#"{{"jsonrpc":"2.0","id":{},{}}}"#, id_repr, body)
}

/// Read line-delimited JSON-RPC from `reader`, dispatch each request
/// through `registry`, write each response (followed by `\n`) to
/// `writer`. Loops until EOF or a write error. Pure stdlib I/O —
/// works on top of stdin/stdout, a TCP stream, or in-memory bufs
/// for tests. The actual `openpencil-mcp` binary just wraps this
/// with `BufReader::new(stdin())` + `stdout()`.
///
/// **Read-only path** — when a tool returns `ToolOutcome::
/// OkWithCommand`, this function REJECTS it as `ToolErrorCode::
/// Internal` so the client never sees a misleading success for
/// a mutation that was never applied (codex stop-gate: previously
/// the response was written without applying the command, so
/// clients saw "wrote: true" on a no-op). Hosts that need write
/// tools should use [`run_stdio_with_applier`] which threads in
/// a closure for command application.
pub fn run_stdio<R: std::io::BufRead, W: std::io::Write>(
    registry: &ToolRegistry,
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<()> {
    run_stdio_with_applier(registry, reader, writer, |_| {
        // No applier → write tools are unsupported on this path.
        // Returning false demotes ToolResponse::Ok to an Err so
        // the client doesn't see a fake success.
        false
    })
}

/// Variant of [`run_stdio`] that accepts an applier closure for
/// MCP write commands. The applier returns `true` when the
/// command actually mutated the document (caller may push undo
/// snapshots etc.) and `false` when it rejected — typically
/// because the document's state drifted between tool validation
/// (snapshotted) and command application (live).
///
/// Wire behaviour:
/// - read tools (no command) → response written verbatim.
/// - write tools, applier returns `true` → response written with
///   `result` carrying the tool's payload; the host has already
///   applied the mutation.
/// - write tools, applier returns `false` → demoted to
///   `ToolErrorCode::Internal` with message describing which
///   command rejected, so the client knows the mutation didn't
///   land.
pub fn run_stdio_with_applier<R, W, F>(
    registry: &ToolRegistry,
    reader: &mut R,
    writer: &mut W,
    mut apply: F,
) -> std::io::Result<()>
where
    R: std::io::BufRead,
    W: std::io::Write,
    F: FnMut(&McpCommand) -> bool,
{
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(()); // EOF
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(call) = parse_tool_call(trimmed) else {
            // Codex stop-gate: structured-arg rejection (or any
            // other parse failure) used to silently `continue`,
            // leaving JSON-RPC clients waiting on a correlated
            // response that never came. Surface a typed error
            // with the request id when we can recover it so the
            // client can fail fast. If even the id is missing
            // (e.g. wire-level malformed JSON, no id field at
            // all) we drop the line — there's nothing to
            // correlate against.
            if let Some(id) = parser::extract_request_id(trimmed) {
                let err = ToolResponse::Err {
                    id,
                    code: ToolErrorCode::InvalidArgument,
                    message: "malformed tool call: unparseable or structured arguments".into(),
                };
                writeln!(writer, "{}", response_to_json(&err))?;
                writer.flush()?;
            }
            continue;
        };
        let mut response = registry.dispatch(call);
        // Apply any queued command before reporting success. If
        // application fails (applier returns false), demote the
        // ToolResponse::Ok to a typed Err so the client sees the
        // mutation didn't land.
        if let ToolResponse::Ok {
            id,
            command: Some(cmd),
            ..
        } = &response
        {
            if !apply(cmd) {
                let id = id.clone();
                response = ToolResponse::Err {
                    id,
                    code: ToolErrorCode::Internal,
                    message: format!("host rejected command: {cmd:?}"),
                };
            }
        }
        writeln!(writer, "{}", response_to_json(&response))?;
        writer.flush()?;
    }
}

// Internal JSON serialisation helpers used by `response_to_json`.
// Kept private to this module — the wire parser sits in
// `mcp/parser.rs`; the first-party tools sit in `mcp/tools.rs`.
fn id_to_json(id: &RequestId) -> String {
    match id {
        RequestId::Str(s) => json_escape(s),
        RequestId::Num(n) => n.to_string(),
    }
}

fn error_code_to_int(code: ToolErrorCode) -> i32 {
    // JSON-RPC reserves -32600..-32603 for transport-level errors;
    // tool errors live in the application range (-32000..-32099).
    match code {
        ToolErrorCode::MissingArgument => -32_001,
        ToolErrorCode::InvalidArgument => -32_602,
        ToolErrorCode::ToolFailed => -32_002,
        ToolErrorCode::UnknownTool => -32_601,
        ToolErrorCode::Internal => -32_603,
    }
}

fn btree_to_json(m: &BTreeMap<String, String>) -> String {
    let mut out = String::from("{");
    let mut first = true;
    for (k, v) in m {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!("{}:{}", json_escape(k), json_escape(v)));
    }
    out.push('}');
    out
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
