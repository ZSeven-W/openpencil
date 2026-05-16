//! Tests for `mcp.rs` — cross-cutting stdio dispatch + parser
//! invariants + a few read-tool registry round-trips.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`.

use crate::test_fixtures::{add_variable, sample, state_with};
use crate::*;
use jian_ops_schema::variable::{VariableKind, VariableScalar};
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
/// response id. Under the v2 trait it CAN'T — the registry stamps
/// the id.
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
            assert_eq!(id, RequestId::Str("req-1".into()));
            assert_eq!(result.get("k"), Some(&"v".to_string()));
        }
        _ => panic!("expected Ok"),
    }
}

#[test]
fn registry_forces_id_on_response_regardless_of_tool() {
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
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains(r#""id":7"#));
}

#[test]
fn run_stdio_emits_error_when_parse_fails_so_clients_dont_hang() {
    use std::io::{BufReader, Cursor};
    let mut r = ToolRegistry::default();
    r.register(Box::new(EchoTool));
    let bad = br#"{"jsonrpc":"2.0","id":42,"method":"replace_node","params":{"node_id":"1","drop_children":{}}}"#;
    let mut input: Vec<u8> = bad.to_vec();
    input.push(b'\n');
    let mut reader = BufReader::new(Cursor::new(input));
    let mut writer: Vec<u8> = Vec::new();
    run_stdio(&r, &mut reader, &mut writer).unwrap();
    let out = String::from_utf8(writer).unwrap();
    assert!(
        out.contains(r#""id":42"#),
        "response must echo the request id: {out}"
    );
    assert!(
        out.contains(r#""error""#),
        "response must be a typed error: {out}"
    );
    assert!(
        out.contains("malformed tool call"),
        "error message names the cause: {out}"
    );
}

#[test]
fn run_stdio_skips_lines_without_an_id() {
    use std::io::{BufReader, Cursor};
    let mut r = ToolRegistry::default();
    r.register(Box::new(EchoTool));
    let input = b"garbage\n{\"method\":\"x\",\"params\":{}}\n";
    let mut reader = BufReader::new(Cursor::new(input.as_ref()));
    let mut writer: Vec<u8> = Vec::new();
    run_stdio(&r, &mut reader, &mut writer).unwrap();
    let out = String::from_utf8(writer).unwrap();
    assert!(
        out.is_empty(),
        "id-less lines must be dropped silently: {out:?}"
    );
}

#[test]
fn run_stdio_demotes_write_tool_response_to_error_without_applier() {
    // The read-only `run_stdio` path demotes any `OkWithCommand`
    // response to `Internal` so a misleading "wrote: true" can't
    // reach the client.
    use std::io::{BufReader, Cursor};
    let mut s = state_with(vec![]);
    add_variable(
        &mut s,
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff8800".into()),
    );
    let mut r = ToolRegistry::default();
    r.register(Box::new(set_variable_color_snapshot(&s)));
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
    use std::io::{BufReader, Cursor};
    let mut s = state_with(vec![]);
    add_variable(
        &mut s,
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff8800".into()),
    );
    let mut r = ToolRegistry::default();
    r.register(Box::new(set_variable_color_snapshot(&s)));
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"set_variable_color\",\"params\":{\"name\":\"brand\",\"hex\":\"#00ff00\"}}\n";
    let mut reader = BufReader::new(Cursor::new(input.as_ref()));
    let mut writer: Vec<u8> = Vec::new();
    let mut applied: Vec<EditorCommand> = Vec::new();
    run_stdio_with_applier(&r, &mut reader, &mut writer, |cmd| {
        applied.push(cmd.clone());
        true
    })
    .unwrap();
    assert_eq!(applied.len(), 1);
    assert!(matches!(
        applied[0],
        EditorCommand::SetVariableColor { ref name, ref hex }
        if name == "brand" && hex == "#00ff00"
    ));
    let out = String::from_utf8(writer).unwrap();
    assert!(out.contains(r#""id":1"#));
    assert!(out.contains(r#""wrote":"true""#));
    assert!(!out.contains(r#""code":"#), "no error code: {out}");
}

#[test]
fn run_stdio_with_applier_demotes_when_applier_rejects() {
    use std::io::{BufReader, Cursor};
    let mut s = state_with(vec![]);
    add_variable(
        &mut s,
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff8800".into()),
    );
    let mut r = ToolRegistry::default();
    r.register(Box::new(set_variable_color_snapshot(&s)));
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
    use crate::test_fixtures::frame;
    let f = frame(
        "n10",
        "F",
        0.0,
        0.0,
        200.0,
        100.0,
        vec![
            crate::test_fixtures::rect("n11", "a", 0.0, 0.0, 10.0, 10.0),
            crate::test_fixtures::rect("n12", "b", 20.0, 0.0, 10.0, 10.0),
        ],
    );
    let s = state_with(vec![f]);
    let info = document_info_snapshot(&s);
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
    let mut s = sample();
    s.clear_selection();
    let snap = selection_snapshot(&s);
    assert_eq!(snap.selected_id, "");
    assert_eq!(snap.kind, "none");
}

#[test]
fn get_selection_reports_selected_node_bounds_and_kind() {
    let mut s = sample();
    s.set_single_selection(op_editor_core::NodeId::new("n10"));
    let snap = selection_snapshot(&s);
    assert_eq!(snap.selected_id, "n10");
    assert_eq!(snap.kind, "frame");
    assert!(snap.width > 0);
    assert!(snap.height > 0);
}

#[test]
fn list_pages_reports_count_and_names() {
    let s = sample();
    let snap = list_pages_snapshot(&s);
    // The sample single-page fixture reports the fallback page.
    assert_eq!(snap.page_count, 1);
    assert_eq!(snap.active_page_index, 0);
    assert!(!snap.names.is_empty(), "page name must serialize");
}

#[test]
fn get_node_returns_record_for_known_id() {
    let s = sample();
    let tool = get_node_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n10".into());
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
    let s = sample();
    let tool = get_node_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n99999".into());
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
    let s = sample();
    let tool = get_node_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!("expected MissingArgument"),
    }
}

#[test]
fn get_node_errors_on_unknown_string_id() {
    let s = sample();
    let tool = get_node_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "not-a-known-id".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::ToolFailed),
        _ => panic!("expected ToolFailed"),
    }
}

#[test]
fn parse_tool_call_extracts_string_params() {
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
    let line = r#"{"id":1,"method":"list_pages"}"#;
    let call = parse_tool_call(line).expect("must parse");
    assert_eq!(call.tool, "list_pages");
    assert!(call.arguments.is_empty());
}

#[test]
fn parse_tool_call_rejects_structured_arg_values() {
    let with_obj = r#"{"id":1,"method":"x","params":{"keep":"yes","nested":{"a":1}}}"#;
    assert!(
        parse_tool_call(with_obj).is_none(),
        "object value must reject the parse"
    );
    let with_arr = r#"{"id":1,"method":"x","params":{"keep":"yes","arr":[1,2]}}"#;
    assert!(
        parse_tool_call(with_arr).is_none(),
        "array value must reject the parse"
    );
    let ok = r#"{"id":1,"method":"x","params":{"keep":"yes","also":"ok"}}"#;
    let call = parse_tool_call(ok).expect("scalar-only must parse");
    assert_eq!(call.arguments.get("keep"), Some(&"yes".to_string()));
    assert_eq!(call.arguments.get("also"), Some(&"ok".to_string()));
}

#[test]
fn parse_tool_call_rejects_structured_values_in_mcp_tools_call_shape() {
    let with_obj = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"42","nested":{"a":1}}}}"#;
    assert!(
        parse_tool_call(with_obj).is_none(),
        "object value inside arguments must reject"
    );
    let with_arr = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"42","arr":[1]}}}"#;
    assert!(
        parse_tool_call(with_arr).is_none(),
        "array value inside arguments must reject"
    );
    let ok = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"42"}}}"#;
    let call = parse_tool_call(ok).expect("scalar-only must parse");
    assert_eq!(call.tool, "get_node");
    assert_eq!(call.arguments.get("node_id"), Some(&"42".to_string()));
}

#[test]
fn parse_tool_call_rejects_non_object_arguments_field() {
    let str_args =
        r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":"oops"}}"#;
    assert!(
        parse_tool_call(str_args).is_none(),
        "string `arguments` must reject"
    );
    let num_args = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":42}}"#;
    assert!(
        parse_tool_call(num_args).is_none(),
        "number `arguments` must reject"
    );
    let arr_args = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","arguments":[]}}"#;
    assert!(
        parse_tool_call(arr_args).is_none(),
        "array `arguments` must reject"
    );
    let no_args = r#"{"id":1,"method":"tools/call","params":{"name":"list_pages"}}"#;
    let call = parse_tool_call(no_args).expect("missing `arguments` is legit");
    assert_eq!(call.tool, "list_pages");
    assert!(call.arguments.is_empty());
}

#[test]
fn parse_tool_call_arguments_lookup_is_top_level_only() {
    let shadow = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","meta":{"arguments":{}},"arguments":"oops"}}"#;
    assert!(
        parse_tool_call(shadow).is_none(),
        "nested meta.arguments must not shadow the real top-level arguments"
    );
    let str_collide = r#"{"id":1,"method":"tools/call","params":{"name":"arguments"}}"#;
    let call = parse_tool_call(str_collide).expect("name=\"arguments\" must not false-positive");
    assert_eq!(call.tool, "arguments");
    assert!(call.arguments.is_empty());
    let deep = r#"{"id":1,"method":"tools/call","params":{"name":"get_node","other":{"x":{"arguments":42}}}}"#;
    let call = parse_tool_call(deep).expect("deeply nested arguments key must not surface");
    assert_eq!(call.tool, "get_node");
    assert!(call.arguments.is_empty());
}

#[test]
fn get_node_reachable_through_stdio_path() {
    let s = sample();
    let mut r = ToolRegistry::default();
    r.register(Box::new(get_node_snapshot(&s)));
    let line = r#"{"id":1,"method":"get_node","params":{"node_id":"n10"}}"#;
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
    let line = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"42"}}}"#;
    let call = parse_tool_call(line).expect("parse");
    assert_eq!(call.tool, "get_node");
    assert_eq!(call.arguments.get("node_id"), Some(&"42".to_string()));
}

#[test]
fn parse_tool_call_mcp_shape_with_no_arguments() {
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
    let s = sample();
    let mut r = ToolRegistry::default();
    r.register(Box::new(get_node_snapshot(&s)));
    let line = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"get_node","arguments":{"node_id":"n10"}}}"#;
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
