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

// Re-export the public surface of submodules so callers can keep
// using `mcp::parse_tool_call` / `mcp::GetDocumentInfo` after the
// split. Mirrors the `widgets::*` re-export pattern.
pub use parser::parse_tool_call;
pub use tools::{
    document_info_snapshot, get_active_theme_snapshot, get_node_snapshot, list_pages_snapshot,
    list_variables_snapshot, selection_snapshot, set_variable_color_snapshot, GetActiveTheme,
    GetDocumentInfo, GetNode, GetSelection, ListPages, ListVariables, NodeRecord,
    SetVariableColor, VariableRecord,
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
#[derive(Debug, Clone, PartialEq)]
pub enum McpCommand {
    SetVariableColor { name: String, hex: String },
    SetActiveAxisValue { axis: String, value: String },
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
#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;
    impl McpTool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
            ToolOutcome::Ok(args.clone())
        }
    }

    /// Deliberately badly-behaved tool: tries to invent a different
    /// response id. Under the v2 trait it CAN'T — `call` returns
    /// outcome only; the registry stamps the id. Used by the
    /// `registry_forces_id_on_misbehaving_tool` regression.
    struct LyingTool;
    impl McpTool for LyingTool {
        fn name(&self) -> &str {
            "lie"
        }
        fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
            ToolOutcome::Ok(BTreeMap::new())
        }
    }

    #[test]
    fn registry_starts_empty() {
        let r = ToolRegistry::default();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert!(r.names().is_empty());
    }

    #[test]
    fn registry_dispatches_to_registered_tool() {
        let mut r = ToolRegistry::default();
        r.register(Box::new(EchoTool));
        let mut args = BTreeMap::new();
        args.insert("k".into(), "v".into());
        let call = ToolCall {
            id: RequestId::Str("req-1".into()),
            tool: "echo".into(),
            arguments: args.clone(),
        };
        match r.dispatch(call) {
            ToolResponse::Ok { id, result, .. } => {
                // Codex BLOCK: the request id MUST round-trip via the
                // tool — JSON-RPC matches responses by id.
                assert_eq!(id, RequestId::Str("req-1".into()));
                assert_eq!(result.get("k"), Some(&"v".to_string()));
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn registry_forces_id_on_response_regardless_of_tool() {
        // Codex BLOCK round 2: id preservation must be enforced
        // structurally, not by convention. The trait now returns
        // a content-only `ToolOutcome`; the registry stamps the id.
        // Verify any tool's response carries the registry-supplied
        // id even when the tool itself has no access to it.
        let mut r = ToolRegistry::default();
        r.register(Box::new(LyingTool));
        let call = ToolCall {
            id: RequestId::Str("req-honest".into()),
            tool: "lie".into(),
            arguments: BTreeMap::new(),
        };
        match r.dispatch(call) {
            ToolResponse::Ok { id, .. } => {
                assert_eq!(id, RequestId::Str("req-honest".into()));
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn response_to_json_ok_payload() {
        let mut result = BTreeMap::new();
        result.insert("k".into(), "v".into());
        let r = ToolResponse::Ok {
            id: RequestId::Num(7),
            result,
            command: None,
        };
        let j = response_to_json(&r);
        assert!(j.contains(r#""jsonrpc":"2.0""#));
        assert!(j.contains(r#""id":7"#));
        assert!(j.contains(r#""result":"#));
        assert!(j.contains(r#""k":"v""#));
    }

    #[test]
    fn response_to_json_err_payload() {
        let r = ToolResponse::Err {
            id: RequestId::Str("req".into()),
            code: ToolErrorCode::UnknownTool,
            message: "no such tool".into(),
        };
        let j = response_to_json(&r);
        assert!(j.contains(r#""id":"req""#));
        assert!(j.contains(r#""code":-32601"#));
        assert!(j.contains(r#""message":"no such tool""#));
    }

    #[test]
    fn parse_tool_call_round_trips_through_registry() {
        let line = r#"{"jsonrpc":"2.0","id":42,"method":"echo","params":{}}"#;
        let call = parse_tool_call(line).expect("parse");
        assert_eq!(call.id, RequestId::Num(42));
        assert_eq!(call.tool, "echo");
        let mut r = ToolRegistry::default();
        r.register(Box::new(EchoTool));
        match r.dispatch(call) {
            ToolResponse::Ok { id, .. } => assert_eq!(id, RequestId::Num(42)),
            _ => panic!(),
        }
    }

    #[test]
    fn run_stdio_dispatches_multi_line_stream() {
        use std::io::{BufReader, Cursor};
        let mut r = ToolRegistry::default();
        r.register(Box::new(EchoTool));
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"echo\",\"params\":{}}\n\
                      {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"echo\",\"params\":{}}\n\
                      {\"jsonrpc\":\"2.0\",\"id\":\"x\",\"method\":\"unknown\",\"params\":{}}\n";
        let mut reader = BufReader::new(Cursor::new(input.as_ref()));
        let mut writer: Vec<u8> = Vec::new();
        run_stdio(&r, &mut reader, &mut writer).unwrap();
        let out = String::from_utf8(writer).unwrap();
        // 3 input lines → 3 response lines.
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains(r#""id":1"#));
        assert!(lines[1].contains(r#""id":2"#));
        assert!(lines[2].contains(r#""id":"x""#));
        assert!(lines[2].contains(r#""code":-32601"#));
    }

    #[test]
    fn run_stdio_skips_malformed_lines() {
        use std::io::{BufReader, Cursor};
        let mut r = ToolRegistry::default();
        r.register(Box::new(EchoTool));
        let input = b"garbage\n\n{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"echo\",\"params\":{}}\n";
        let mut reader = BufReader::new(Cursor::new(input.as_ref()));
        let mut writer: Vec<u8> = Vec::new();
        run_stdio(&r, &mut reader, &mut writer).unwrap();
        let out = String::from_utf8(writer).unwrap();
        // Only the valid 3rd line produces output.
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains(r#""id":7"#));
    }

    #[test]
    fn run_stdio_demotes_write_tool_response_to_error_without_applier() {
        // Codex stop-gate: a write tool returning OkWithCommand on
        // the read-only `run_stdio` path was being written as
        // success — clients saw "wrote: true" even though no
        // command had been applied. Now the read-only path demotes
        // the response to ToolErrorCode::Internal so the misleading
        // success can't reach the client.
        use std::io::{BufReader, Cursor};
        use crate::mcp::tools::set_variable_color_snapshot;
        use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
        let mut doc = Document::empty();
        doc.var_table.variables.push(Variable {
            name: "brand".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff8800".into())),
        });
        let mut r = ToolRegistry::default();
        r.register(Box::new(set_variable_color_snapshot(&doc)));
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"set_variable_color\",\"params\":{\"name\":\"brand\",\"hex\":\"#00ff00\"}}\n";
        let mut reader = BufReader::new(Cursor::new(input.as_ref()));
        let mut writer: Vec<u8> = Vec::new();
        run_stdio(&r, &mut reader, &mut writer).unwrap();
        let out = String::from_utf8(writer).unwrap();
        assert!(
            out.contains(r#""code":-32603"#),
            "read-only run_stdio must demote write OkWithCommand to Internal; got {out}"
        );
        assert!(out.contains("host rejected command"));
    }

    #[test]
    fn run_stdio_with_applier_applies_write_command_then_writes_success() {
        // The companion path: with an applier the write tool's
        // OkWithCommand is honored — the closure receives the
        // command, returns true, and the client sees a normal Ok
        // response. Mirrors what a real MCP server would do
        // (Document::apply_mcp_command threaded in as the closure).
        use std::io::{BufReader, Cursor};
        use crate::mcp::tools::set_variable_color_snapshot;
        use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
        let mut doc = Document::empty();
        doc.var_table.variables.push(Variable {
            name: "brand".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff8800".into())),
        });
        let mut r = ToolRegistry::default();
        r.register(Box::new(set_variable_color_snapshot(&doc)));
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"set_variable_color\",\"params\":{\"name\":\"brand\",\"hex\":\"#00ff00\"}}\n";
        let mut reader = BufReader::new(Cursor::new(input.as_ref()));
        let mut writer: Vec<u8> = Vec::new();
        let mut applied_commands: Vec<McpCommand> = Vec::new();
        let result = run_stdio_with_applier(&r, &mut reader, &mut writer, |cmd| {
            applied_commands.push(cmd.clone());
            true
        });
        result.unwrap();
        // The applier saw the command exactly once + the response
        // is a normal Ok (no error code).
        assert_eq!(applied_commands.len(), 1);
        assert!(matches!(
            applied_commands[0],
            McpCommand::SetVariableColor { ref name, ref hex }
            if name == "brand" && hex == "#00ff00"
        ));
        let out = String::from_utf8(writer).unwrap();
        assert!(out.contains(r#""id":1"#));
        assert!(out.contains(r#""wrote":"true""#));
        assert!(!out.contains(r#""code":"#), "no error code: {out}");
    }

    #[test]
    fn run_stdio_with_applier_demotes_when_applier_rejects() {
        // Applier returns false (e.g. host's document state drifted
        // between tool registration + dispatch; the variable was
        // deleted mid-flight). Wire response becomes Internal so
        // the client knows the mutation didn't land.
        use std::io::{BufReader, Cursor};
        use crate::mcp::tools::set_variable_color_snapshot;
        use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
        let mut doc = Document::empty();
        doc.var_table.variables.push(Variable {
            name: "brand".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff8800".into())),
        });
        let mut r = ToolRegistry::default();
        r.register(Box::new(set_variable_color_snapshot(&doc)));
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"set_variable_color\",\"params\":{\"name\":\"brand\",\"hex\":\"#00ff00\"}}\n";
        let mut reader = BufReader::new(Cursor::new(input.as_ref()));
        let mut writer: Vec<u8> = Vec::new();
        run_stdio_with_applier(&r, &mut reader, &mut writer, |_| false).unwrap();
        let out = String::from_utf8(writer).unwrap();
        assert!(out.contains(r#""code":-32603"#));
        assert!(out.contains("host rejected"));
    }

    #[test]
    fn json_escape_handles_special_chars() {
        let r = ToolResponse::Err {
            id: RequestId::Str("x\"y".into()),
            code: ToolErrorCode::Internal,
            message: "line1\nline2".into(),
        };
        let j = response_to_json(&r);
        assert!(j.contains(r#""x\"y""#));
        assert!(j.contains(r#""line1\nline2""#));
    }

    #[test]
    fn get_document_info_reports_snapshot_via_registry() {
        use crate::document::{Document, Node, NodeKind};
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        page.children.push(Node::with_children(
            10,
            NodeKind::Frame,
            "F",
            vec![
                Node::leaf(11, NodeKind::Rect, "a"),
                Node::leaf(12, NodeKind::Rect, "b"),
            ],
        ));
        let info = document_info_snapshot(&doc);
        // Frame + 2 children = 3 nodes total.
        assert_eq!(info.total_nodes, 3);
        let mut r = ToolRegistry::default();
        r.register(Box::new(info));
        let call = ToolCall {
            id: RequestId::Num(1),
            tool: "get_document_info".into(),
            arguments: BTreeMap::new(),
        };
        match r.dispatch(call) {
            ToolResponse::Ok { result, .. } => {
                assert_eq!(result.get("total_nodes"), Some(&"3".to_string()));
                assert_eq!(result.get("page_count"), Some(&"1".to_string()));
                assert_eq!(result.get("active_page_index"), Some(&"0".to_string()));
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn registry_errors_on_unknown_tool() {
        let r = ToolRegistry::default();
        let call = ToolCall {
            id: RequestId::Num(7),
            tool: "nope".into(),
            arguments: BTreeMap::new(),
        };
        match r.dispatch(call) {
            ToolResponse::Err { code, message, .. } => {
                assert_eq!(code, ToolErrorCode::UnknownTool);
                assert!(message.contains("nope"));
            }
            _ => panic!("expected Err"),
        }
    }

    #[test]
    fn get_selection_reports_no_selection_when_none() {
        let mut doc = crate::document::Document::sample();
        doc.set_single_selection(crate::document::NodeId::NONE);
        let snap = selection_snapshot(&doc);
        assert_eq!(snap.selected_id, 0);
        assert_eq!(snap.kind, "none");
    }

    #[test]
    fn get_selection_reports_selected_node_bounds_and_kind() {
        let mut doc = crate::document::Document::sample();
        // Sample doc's Frame is id 10 with NodeKind::Frame.
        doc.set_single_selection(crate::document::NodeId::new(10));
        let snap = selection_snapshot(&doc);
        assert_eq!(snap.selected_id, 10);
        assert_eq!(snap.kind, "frame");
        // Sample frame bounds are positive (mutators.rs::sample).
        assert!(snap.width > 0);
        assert!(snap.height > 0);
    }

    #[test]
    fn list_pages_reports_count_and_names() {
        let doc = crate::document::Document::sample();
        let snap = list_pages_snapshot(&doc);
        // Sample document has 1 page.
        assert_eq!(snap.page_count, 1);
        assert_eq!(snap.active_page_index, 0);
        assert!(!snap.names.is_empty(), "page name must serialize");
    }

    #[test]
    fn get_node_returns_record_for_known_id() {
        let doc = crate::document::Document::sample();
        let tool = get_node_snapshot(&doc);
        let mut args = BTreeMap::new();
        args.insert("node_id".into(), "10".into());
        match tool.call(&args) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("kind"), Some(&"frame".to_string()));
                assert!(out.get("name").map(|n| !n.is_empty()).unwrap_or(false));
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_node_errors_on_unknown_id() {
        let doc = crate::document::Document::sample();
        let tool = get_node_snapshot(&doc);
        let mut args = BTreeMap::new();
        args.insert("node_id".into(), "99999".into());
        match tool.call(&args) {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::ToolFailed);
                assert!(msg.contains("99999"));
            }
            _ => panic!("expected Err for unknown id"),
        }
    }

    #[test]
    fn get_node_errors_on_missing_arg() {
        let doc = crate::document::Document::sample();
        let tool = get_node_snapshot(&doc);
        match tool.call(&BTreeMap::new()) {
            ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
            _ => panic!("expected MissingArgument"),
        }
    }

    #[test]
    fn get_node_surfaces_fill_and_stroke_refs() {
        // When a node's fill / stroke colour follows a variable
        // (`var_table.fill_refs[id] = "color-primary"`), the
        // `get_node` payload must expose the variable name so LLM
        // clients can pick "bump the variable" vs "override the
        // node's literal colour" without re-walking the document.
        use crate::document::{Document, NodeId};
        let mut doc = Document::sample();
        doc.var_table
            .fill_refs
            .insert(NodeId::new(11), "color-primary".into());
        doc.var_table
            .stroke_refs
            .insert(NodeId::new(11), "color-accent".into());
        let tool = get_node_snapshot(&doc);
        let mut args = BTreeMap::new();
        args.insert("node_id".into(), "11".into());
        match tool.call(&args) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("fill_ref"), Some(&"color-primary".to_string()));
                assert_eq!(out.get("stroke_ref"), Some(&"color-accent".to_string()));
            }
            other => panic!("expected Ok, got {other:?}"),
        }
        // Nodes WITHOUT a ref-mapping must return empty strings
        // (not omit the field) so clients can probe with a single
        // `out.get("fill_ref") == Some("")` instead of branching
        // on Option<String> AND Option<&str>.
        let mut args2 = BTreeMap::new();
        args2.insert("node_id".into(), "12".into());
        match tool.call(&args2) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("fill_ref"), Some(&String::new()));
                assert_eq!(out.get("stroke_ref"), Some(&String::new()));
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_node_errors_on_non_numeric_arg() {
        let doc = crate::document::Document::sample();
        let tool = get_node_snapshot(&doc);
        let mut args = BTreeMap::new();
        args.insert("node_id".into(), "not-a-number".into());
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
            _ => panic!("expected InvalidArgument"),
        }
    }

    #[test]
    fn parse_tool_call_extracts_string_params() {
        // The wire-format parser must surface `node_id` so
        // `get_node` is reachable through stdio (not just the
        // direct registry-dispatch path).
        let line = r#"{"jsonrpc":"2.0","id":3,"method":"get_node","params":{"node_id":"42"}}"#;
        let call = parse_tool_call(line).expect("must parse");
        assert_eq!(call.tool, "get_node");
        assert_eq!(call.arguments.get("node_id"), Some(&"42".to_string()));
    }

    #[test]
    fn parse_tool_call_extracts_numeric_and_bool_params() {
        let line = r#"{"id":7,"method":"x","params":{"page":1,"active":true}}"#;
        let call = parse_tool_call(line).expect("must parse");
        assert_eq!(call.arguments.get("page"), Some(&"1".to_string()));
        assert_eq!(call.arguments.get("active"), Some(&"true".to_string()));
    }

    #[test]
    fn parse_tool_call_handles_missing_params() {
        // Tools that take no arguments shouldn't fail to parse.
        let line = r#"{"id":1,"method":"list_pages"}"#;
        let call = parse_tool_call(line).expect("must parse");
        assert_eq!(call.tool, "list_pages");
        assert!(call.arguments.is_empty());
    }

    #[test]
    fn parse_tool_call_skips_nested_object_values() {
        // Nested objects/arrays don't appear in tool args today.
        // The parser must skip them without poisoning the map.
        let line = r#"{"id":1,"method":"x","params":{"keep":"yes","nested":{"a":1},"also":"ok"}}"#;
        let call = parse_tool_call(line).expect("must parse");
        assert_eq!(call.arguments.get("keep"), Some(&"yes".to_string()));
        assert_eq!(call.arguments.get("also"), Some(&"ok".to_string()));
        // `nested` is intentionally skipped.
        assert!(call.arguments.get("nested").is_none());
    }

    #[test]
    fn get_node_reachable_through_stdio_path() {
        // End-to-end: the wire `params` parser feeds the tool's
        // required `node_id`, dispatch returns Ok with kind=frame.
        // This is the regression test for codex BLOCK:
        // `get_node is not usable through the stdio MCP path`.
        let doc = crate::document::Document::sample();
        let mut r = ToolRegistry::default();
        r.register(Box::new(get_node_snapshot(&doc)));
        let line = r#"{"id":1,"method":"get_node","params":{"node_id":"10"}}"#;
        let call = parse_tool_call(line).expect("parse");
        match r.dispatch(call) {
            ToolResponse::Ok { result, .. } => {
                assert_eq!(result.get("kind"), Some(&"frame".to_string()));
            }
            ToolResponse::Err { code, message, .. } => {
                panic!("expected Ok, got Err({code:?}, {message})")
            }
        }
    }

    #[test]
    fn parse_tool_call_real_mcp_tools_call_shape() {
        // Real MCP wire shape — `method:"tools/call"`, tool name +
        // arguments nested under params. This is what Claude Code /
        // Codex / Gemini etc. send when invoking a tool. The codex
        // stop-gate flagged that the parser previously only
        // recognized the flat `method == toolname` shape.
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"42"}}}"#;
        let call = parse_tool_call(line).expect("parse");
        assert_eq!(call.tool, "get_node");
        assert_eq!(call.arguments.get("node_id"), Some(&"42".to_string()));
    }

    #[test]
    fn parse_tool_call_mcp_shape_with_no_arguments() {
        // Tools that take no args still use the `tools/call` envelope.
        let line = r#"{"id":2,"method":"tools/call","params":{"name":"list_pages","arguments":{}}}"#;
        let call = parse_tool_call(line).expect("parse");
        assert_eq!(call.tool, "list_pages");
        assert!(call.arguments.is_empty());
    }

    #[test]
    fn parse_tool_call_mcp_shape_with_numeric_arg() {
        let line = r#"{"id":3,"method":"tools/call","params":{"name":"x","arguments":{"limit":5,"enabled":true}}}"#;
        let call = parse_tool_call(line).expect("parse");
        assert_eq!(call.arguments.get("limit"), Some(&"5".to_string()));
        assert_eq!(call.arguments.get("enabled"), Some(&"true".to_string()));
    }

    #[test]
    fn get_node_reachable_through_real_mcp_envelope() {
        // The regression test that closes codex BLOCK #2 ("parser
        // still doesn't handle the real MCP tool-call shape"):
        // a real MCP-style request must route to the tool.
        let doc = crate::document::Document::sample();
        let mut r = ToolRegistry::default();
        r.register(Box::new(get_node_snapshot(&doc)));
        let line = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"10"}}}"#;
        let call = parse_tool_call(line).expect("parse");
        match r.dispatch(call) {
            ToolResponse::Ok { result, id, .. } => {
                assert!(matches!(id, RequestId::Num(7)));
                assert_eq!(result.get("kind"), Some(&"frame".to_string()));
            }
            ToolResponse::Err { code, message, .. } => {
                panic!("expected Ok, got Err({code:?}, {message})")
            }
        }
    }
}
