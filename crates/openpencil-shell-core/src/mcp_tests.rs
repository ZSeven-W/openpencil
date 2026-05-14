//! Tests for `mcp.rs`. Moved to a sibling file to keep the spine
//! under the 800-line cap.

use super::mcp::*;
use std::collections::BTreeMap;


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
fn run_stdio_emits_error_when_parse_fails_so_clients_dont_hang() {
    // Codex stop-time gate: when `parse_tool_call` returned None
    // (e.g. structured-arg rejection), the dispatcher silently
    // continued. JSON-RPC clients waiting for a response keyed
    // off the request id never got one — the call hung.
    //
    // Now any line that has an extractable `id` but otherwise
    // fails parsing emits a typed error response so the client
    // can correlate + recover.
    use std::io::{BufReader, Cursor};
    let mut r = ToolRegistry::default();
    r.register(Box::new(EchoTool));
    // Structured `drop_children` (object) — fails parse but has
    // a valid id. The dispatcher MUST emit a response.
    let bad = br#"{"jsonrpc":"2.0","id":42,"method":"replace_node","params":{"node_id":"1","drop_children":{}}}"#;
    let mut input: Vec<u8> = bad.to_vec();
    input.push(b'\n');
    let mut reader = BufReader::new(Cursor::new(input));
    let mut writer: Vec<u8> = Vec::new();
    run_stdio(&r, &mut reader, &mut writer).unwrap();
    let out = String::from_utf8(writer).unwrap();
    assert!(out.contains(r#""id":42"#), "response must echo the request id: {out}");
    assert!(out.contains(r#""error""#), "response must be a typed error: {out}");
    assert!(out.contains("malformed tool call"), "error message must name the cause: {out}");
}

#[test]
fn run_stdio_skips_lines_without_an_id() {
    // A line that lacks an id field at all has nothing to
    // correlate against. The dispatcher drops it silently —
    // emitting an unaddressed error would only confuse the
    // client. Existing skip-malformed behavior unchanged.
    use std::io::{BufReader, Cursor};
    let mut r = ToolRegistry::default();
    r.register(Box::new(EchoTool));
    let input = b"garbage\n{\"method\":\"x\",\"params\":{}}\n";
    let mut reader = BufReader::new(Cursor::new(input.as_ref()));
    let mut writer: Vec<u8> = Vec::new();
    run_stdio(&r, &mut reader, &mut writer).unwrap();
    let out = String::from_utf8(writer).unwrap();
    assert!(out.is_empty(), "id-less lines must be dropped silently: {out:?}");
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
    use crate::mcp::write_tools::set_variable_color_snapshot;
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
    use crate::mcp::write_tools::set_variable_color_snapshot;
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
    use crate::mcp::write_tools::set_variable_color_snapshot;
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
fn parse_tool_call_rejects_structured_arg_values() {
    // Nested objects/arrays don't appear in tool args today.
    // The parser refuses to build a ToolCall when ANY value is
    // structured — surfacing a sentinel string instead would
    // either collide with a legitimate user-supplied scalar
    // (codex flagged "{...}" as plausibly matching a variable
    // name) or leave string-accepting tools unable to tell wire
    // malformed input from real data. Reject at the wire layer
    // so no tool sees a structured value as a scalar.
    let with_obj = r#"{"id":1,"method":"x","params":{"keep":"yes","nested":{"a":1}}}"#;
    assert!(parse_tool_call(with_obj).is_none(), "object value must reject the parse");
    let with_arr = r#"{"id":1,"method":"x","params":{"keep":"yes","arr":[1,2]}}"#;
    assert!(parse_tool_call(with_arr).is_none(), "array value must reject the parse");
    // All-scalar still parses.
    let ok = r#"{"id":1,"method":"x","params":{"keep":"yes","also":"ok"}}"#;
    let call = parse_tool_call(ok).expect("scalar-only must parse");
    assert_eq!(call.arguments.get("keep"), Some(&"yes".to_string()));
    assert_eq!(call.arguments.get("also"), Some(&"ok".to_string()));
}

#[test]
fn parse_tool_call_rejects_structured_values_in_mcp_tools_call_shape() {
    // Same rejection contract applies on the MCP `tools/call`
    // path (codex stop-gate: the previous structured-rejection
    // tests only exercised the legacy/direct shape).
    let with_obj = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"42","nested":{"a":1}}}}"#;
    assert!(parse_tool_call(with_obj).is_none(), "object value inside arguments must reject");
    let with_arr = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"42","arr":[1]}}}"#;
    assert!(parse_tool_call(with_arr).is_none(), "array value inside arguments must reject");
    // Scalar-only still parses.
    let ok = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"42"}}}"#;
    let call = parse_tool_call(ok).expect("scalar-only must parse");
    assert_eq!(call.tool, "get_node");
    assert_eq!(call.arguments.get("node_id"), Some(&"42".to_string()));
}

#[test]
fn parse_tool_call_rejects_non_object_arguments_field() {
    // Codex stop-gate: `arguments` present but not an object
    // (e.g. `"arguments":"oops"` or `"arguments":42`) used to
    // downgrade to empty args via `extract_object_body` returning
    // None. Now it rejects the parse so a wire-shape error
    // doesn't silently land at the dispatcher as a no-arg call.
    let str_args = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":"oops"}}"#;
    assert!(parse_tool_call(str_args).is_none(), "string `arguments` must reject");
    let num_args = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":42}}"#;
    assert!(parse_tool_call(num_args).is_none(), "number `arguments` must reject");
    let arr_args = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":[]}}"#;
    assert!(parse_tool_call(arr_args).is_none(), "array `arguments` must reject");
    // `arguments` legitimately omitted → empty args, parse OK.
    let no_args = r#"{"id":1,"method":"tools/call","params":{"name":"list_pages"}}"#;
    let call = parse_tool_call(no_args).expect("missing `arguments` is legit");
    assert_eq!(call.tool, "list_pages");
    assert!(call.arguments.is_empty());
}

#[test]
fn parse_tool_call_arguments_lookup_is_top_level_only() {
    // Codex stop-gate (twice): `body.find("\"arguments\"")` was a
    // substring scan that matched nested keys OR string values
    // containing the literal `"arguments"`. Fixed with a real
    // top-level walker. Three repros below — all valid JSON,
    // none containing a real top-level `arguments` object:
    //
    // 1. Nested `meta.arguments` object — the substring scan
    //    would have hit it and downgraded a sibling
    //    `"arguments":"oops"` to Body silently. Now the walker
    //    sees the top-level `arguments` is a string → Malformed.
    let shadow = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","meta":{"arguments":{}},"arguments":"oops"}}"#;
    assert!(
        parse_tool_call(shadow).is_none(),
        "nested meta.arguments must not shadow the real top-level arguments"
    );
    // 2. A sibling whose STRING value is literally "arguments".
    //    Real `arguments` is missing → Missing → empty args, parse OK.
    let str_collide = r#"{"id":1,"method":"tools/call","params":{"name":"arguments"}}"#;
    let call = parse_tool_call(str_collide).expect("name=\"arguments\" must not false-positive");
    assert_eq!(call.tool, "arguments");
    assert!(call.arguments.is_empty());
    // 3. A nested `arguments` deep inside something else, with no
    //    top-level `arguments` at all. Still empty args, parse OK.
    let deep = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","other":{"x":{"arguments":42}}}}"#;
    let call = parse_tool_call(deep).expect("deeply nested arguments key must not surface");
    assert_eq!(call.tool, "get_node");
    assert!(call.arguments.is_empty());
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
