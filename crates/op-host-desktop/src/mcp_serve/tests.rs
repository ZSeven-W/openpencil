//! `mcp_serve` tests — split out of `mcp_serve.rs` to keep that
//! file under the 800-line cap.

#![cfg(test)]

use super::*;

#[test]
fn sniff_method_walks_top_level() {
    assert_eq!(
        sniff_method(r#"{"id":1,"method":"initialize","params":{}}"#),
        Some("initialize".into())
    );
    assert_eq!(
        sniff_method(r#"{"id":1,"method":"tools/call","params":{"name":"x"}}"#),
        Some("tools/call".into())
    );
    // Nested `method` keys must not shadow the real one.
    assert_eq!(
        sniff_method(r#"{"id":1,"method":"tools/list","params":{"method":"fake"}}"#),
        Some("tools/list".into())
    );
    assert_eq!(sniff_method("not even json"), None);
}

#[test]
fn sniff_id_raw_preserves_type() {
    assert_eq!(sniff_id_raw(r#"{"id":42,"method":"x"}"#), Some("42".into()));
    assert_eq!(
        sniff_id_raw(r#"{"id":"abc","method":"x"}"#),
        Some(r#""abc""#.into())
    );
}

#[test]
fn initialize_response_includes_protocol_and_capabilities() {
    let r = initialize_response("7");
    assert!(r.contains(r#""id":7"#));
    assert!(r.contains(r#""protocolVersion""#));
    assert!(r.contains(r#""tools""#));
    assert!(r.contains(r#""serverInfo""#));
}

#[test]
fn tools_list_response_includes_all_registered_tools() {
    // The debug isolation flag is process-global; remove it
    // explicitly so this test is deterministic under cargo's
    // parallel runner (an earlier test may have set it).
    std::env::remove_var("OPENPENCIL_DEBUG_TOOLS");
    let state = op_editor_core::EditorState::new();
    let r = tools_list_response("3", &state);
    // The production catalog excludes debug tools. Exact-count
    // assertion: any tool added without updating this test trips
    // the count first. Codex stop-gate: previous `contains`-only
    // checks would have silently passed if a new tool slipped into
    // TOOL_SCHEMAS without being added to the list below.
    assert_eq!(
        TOOL_SCHEMAS.len(),
        83,
        "tools/list catalog count must match the registered tools — add the new tool to this test"
    );
    // Production catalog excludes debug tools (we removed the
    // env var above to ensure deterministic gate-off behaviour).
    assert!(
        !r.contains("debug_validation_report"),
        "production tools/list must not advertise the debug tool: {r}"
    );
    // UIKit element tools are appended dynamically — one per
    // built-in starter-kit component (6) — and ride alongside
    // the static schemas in the tools/list response.
    assert_eq!(
        op_mcp::element_tools::element_tool_schemas(&state).len(),
        6,
        "starter kit ships 6 element tools — update this if the kit grows"
    );
    for name in [
        "insert_btn_primary",
        "insert_input_text",
        "insert_card_basic",
        "insert_nav_bar",
        "insert_divider",
        "insert_badge",
    ] {
        assert!(
            r.contains(name),
            "tools/list must include element tool {name}"
        );
    }
    for name in [
        "get_document_info",
        "get_selection",
        "get_node",
        "list_pages",
        "list_variables",
        "get_active_theme",
        "list_components",
        "get_component",
        "snapshot_layout",
        "get_canvas_bounds",
        "find_node_by_name",
        "get_node_parent",
        "get_node_children",
        "count_nodes",
        "list_node_kinds",
        "get_history_depth",
        "get_viewport",
        "get_selection_set",
        "clear_selection",
        "set_selection",
        "set_viewport",
        "set_node_hidden",
        "set_node_locked",
        "set_node_collapsed",
        "set_active_tool",
        "undo",
        "redo",
        "duplicate_selected",
        "delete_selected",
        "nudge_selected",
        "group_selected",
        "ungroup_selected",
        "reorder_selected",
        "set_node_rotation",
        "set_node_text",
        "set_node_corner_radius",
        "set_node_font_size",
        "set_node_font_weight",
        "set_node_stroke_hex",
        "set_node_stroke_width",
        "align_selected",
        "set_node_fill_hex",
        "set_node_flip",
        "set_ellipse_arc",
        "add_node_effect",
        "remove_node_effect",
        "set_node_name",
        "set_selection_set",
        "toggle_node_selection",
        "cycle_active_axis_value",
        "copy_selected",
        "cut_selected",
        "paste_clipboard",
        "instantiate_component",
        "create_component",
        "delete_component",
        "rename_component",
        "set_active_page",
        "add_page",
        "rename_page",
        "delete_page",
        "remove_page",
        "duplicate_page",
        "reorder_page",
        "set_variable_color",
        "set_active_axis_value",
        "insert_node",
        "import_svg",
        "update_node",
        "delete_node",
        "move_node",
        "copy_node",
        "replace_node",
        "batch_design",
        "set_variable_number",
        "set_variable_string",
        "set_variable_boolean",
        "create_variable",
        "delete_variable",
        "rename_variable",
        "design_skeleton",
        "design_content",
        "design_refine",
    ] {
        assert!(r.contains(name), "tools/list must include {name}: {r}");
    }

    // Gate open — the debug tool joins the catalog.
    std::env::set_var("OPENPENCIL_DEBUG_TOOLS", "1");
    let r_debug = tools_list_response("3", &state);
    assert!(
        r_debug.contains("debug_validation_report"),
        "debug tools/list must advertise debug_validation_report: {r_debug}"
    );
    std::env::remove_var("OPENPENCIL_DEBUG_TOOLS");
}

/// In-memory `Read + Write` stand-in for a `TcpStream` so the HTTP
/// transport can be exercised without a real socket.
struct MockStream {
    input: std::io::Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl std::io::Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.input.read(buf)
    }
}

impl std::io::Write for MockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn http_request_body_reads_exactly_content_length() {
    // Trailing bytes past Content-Length must NOT leak into the body.
    let body = r#"{"method":"ping"}"#;
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}TRAILING-IGNORED",
        body.len()
    );
    let mut cur = std::io::Cursor::new(request.into_bytes());
    assert_eq!(read_http_request_body(&mut cur).unwrap(), body);
}

#[test]
fn http_transport_serves_initialize() {
    let rpc = r#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{}}"#;
    let request = format!(
        "POST / HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{rpc}",
        rpc.len()
    );
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.into_bytes()),
        output: Vec::new(),
    };
    let mut state = EditorState::new();
    serve_http_connection(
        &mut stream,
        &mut state,
        std::path::Path::new("/tmp/unused.op"),
    )
    .expect("serve_http_connection");
    let resp = String::from_utf8(stream.output).unwrap();
    assert!(resp.starts_with("HTTP/1.1 200 OK"), "status line: {resp}");
    assert!(resp.contains("Content-Type: application/json"));
    assert!(resp.contains("mcp-session-id: openpencil"));
    assert!(resp.contains("Access-Control-Allow-Origin: *"));
    // The JSON-RPC initialize reply carries the protocol handshake +
    // the request id, proving the body round-tripped over HTTP.
    assert!(resp.contains(r#""protocolVersion""#), "body: {resp}");
    assert!(resp.contains(r#""id":7"#), "body: {resp}");
}

#[test]
fn http_transport_serves_options_preflight() {
    let request = "OPTIONS /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n";
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.as_bytes().to_vec()),
        output: Vec::new(),
    };
    let mut state = EditorState::new();
    serve_http_connection(
        &mut stream,
        &mut state,
        std::path::Path::new("/tmp/unused.op"),
    )
    .expect("serve_http_connection");
    let resp = String::from_utf8(stream.output).unwrap();
    assert!(resp.starts_with("HTTP/1.1 204 No Content"), "{resp}");
    assert!(resp.contains("Access-Control-Allow-Methods"));
}
