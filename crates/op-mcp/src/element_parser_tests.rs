use crate::parse_tool_call;

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
fn parse_tool_call_allows_structured_element_tool_args_for_ts_parity() {
    let line = r#"{"id":7,"method":"tools/call","params":{"name":"add_section_header_v0","arguments":{"title":"Recent","action":{"label":"See all","icon":"arrow-right"}}}}"#;
    let call = parse_tool_call(line).expect("element tools must accept TS-style nested args");
    assert_eq!(call.tool, "add_section_header_v0");
    assert_eq!(call.arguments.get("title"), Some(&"Recent".to_string()));
    assert_eq!(
        call.arguments.get("action"),
        Some(&r#"{"label":"See all","icon":"arrow-right"}"#.to_string())
    );
}
