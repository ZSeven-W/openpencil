//! `mcp_serve` tests — split out of `mcp_serve.rs` to keep that
//! file under the 800-line cap.

#![cfg(test)]

use super::*;
use op_editor_core::pen_node_ext::PenNodeExt;

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
        106,
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
        194,
        "starter kit ships 6 canonical + 188 TS-compatible element tools — update this if the TS catalog changes"
    );
    for name in [
        "insert_btn_primary",
        "insert_input_text",
        "insert_card_basic",
        "insert_nav_bar",
        "insert_divider",
        "insert_badge",
        "add_text_button_v0",
        "add_text_button_v1",
        "add_form_field_v0",
        "add_form_field_v1",
        "add_stat_card_v0",
        "add_stat_card_v1",
        "add_top_nav_bar_v0",
        "add_top_nav_bar_v1",
        "add_divider_v0",
        "add_divider_v1",
        "add_badge_v0",
        "add_badge_v1",
    ] {
        assert!(
            r.contains(name),
            "tools/list must include element tool {name}"
        );
    }
    for name in [
        "get_document_info",
        "open_document",
        "save_document",
        "get_selection",
        "get_node",
        "list_pages",
        "list_variables",
        "get_variables",
        "save_theme_preset",
        "load_theme_preset",
        "list_theme_presets",
        "get_design_md",
        "set_design_md",
        "export_design_md",
        "get_style_guide_tags",
        "get_style_guide",
        "get_active_theme",
        "list_components",
        "get_component",
        "batch_get",
        "read_nodes",
        "codegen_plan",
        "codegen_submit_chunk",
        "codegen_assemble",
        "codegen_clean",
        "search_all_unique_properties",
        "replace_all_matching_properties",
        "snapshot_layout",
        "find_empty_space",
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
        "get_design_prompt",
        "set_variable_number",
        "set_variable_string",
        "set_variable_boolean",
        "set_variables",
        "set_themes",
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
    for name in [
        "debug_validation_report",
        "debug_logs_tail",
        "debug_screenshot",
    ] {
        assert!(
            r_debug.contains(name),
            "debug tools/list must advertise {name}: {r_debug}"
        );
    }
    std::env::remove_var("OPENPENCIL_DEBUG_TOOLS");
}

#[test]
fn tools_list_design_content_schema_advertises_ts_layered_args() {
    std::env::remove_var("OPENPENCIL_DEBUG_TOOLS");
    let state = op_editor_core::EditorState::new();
    let response: serde_json::Value = serde_json::from_str(&tools_list_response("3", &state))
        .expect("tools/list response should be JSON");
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools/list result should contain tools");
    let design_content = tools
        .iter()
        .find(|tool| tool.get("name").and_then(|name| name.as_str()) == Some("design_content"))
        .expect("design_content schema");
    let properties = design_content["inputSchema"]["properties"]
        .as_object()
        .expect("design_content properties");

    for key in [
        "sectionId",
        "children",
        "postProcess",
        "canvasWidth",
        "pageId",
    ] {
        assert!(
            properties.contains_key(key),
            "design_content schema should advertise {key}: {design_content}"
        );
    }
    assert_eq!(
        design_content["inputSchema"]["required"]
            .as_array()
            .map(|items| items
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()),
        Some(vec!["sectionId", "children"])
    );
}

#[test]
fn tools_list_schemas_advertise_ts_file_path_args() {
    std::env::remove_var("OPENPENCIL_DEBUG_TOOLS");
    let state = op_editor_core::EditorState::new();
    let response: serde_json::Value = serde_json::from_str(&tools_list_response("3", &state))
        .expect("tools/list response should be JSON");
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools/list result should contain tools");

    for (tool_name, expected) in [
        ("get_selection", vec!["filePath", "readDepth"]),
        ("batch_get", vec!["filePath", "readDepth", "searchDepth"]),
        ("read_nodes", vec!["filePath", "nodeIds", "depth"]),
        ("snapshot_layout", vec!["filePath", "parentId", "maxDepth"]),
        ("find_empty_space", vec!["filePath", "width", "height"]),
        ("add_page", vec!["filePath", "name", "children"]),
        (
            "insert_node",
            vec!["filePath", "data", "postProcess", "canvasWidth", "pageId"],
        ),
        (
            "update_node",
            vec!["filePath", "data", "postProcess", "canvasWidth", "pageId"],
        ),
        (
            "replace_node",
            vec!["filePath", "data", "postProcess", "canvasWidth", "pageId"],
        ),
        (
            "import_svg",
            vec![
                "filePath",
                "svgPath",
                "maxDim",
                "postProcess",
                "canvasWidth",
            ],
        ),
        ("set_variables", vec!["filePath", "variables", "replace"]),
        ("get_design_prompt", vec!["section", "filePath"]),
        ("get_design_md", vec!["filePath"]),
        ("set_design_md", vec!["filePath", "markdown", "autoExtract"]),
        ("export_design_md", vec!["filePath"]),
        ("design_content", vec!["filePath", "sectionId", "children"]),
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool.get("name").and_then(|name| name.as_str()) == Some(tool_name))
            .unwrap_or_else(|| panic!("missing {tool_name} schema"));
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{tool_name} properties should be an object"));
        for key in expected {
            assert!(
                properties.contains_key(key),
                "{tool_name} schema should advertise {key}: {tool}"
            );
        }
    }
}

#[test]
fn find_empty_space_returns_padded_position_from_active_page_bounds() {
    let mut state = op_editor_core::EditorState::new();
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Left".into(),
        x: 10,
        y: 20,
        width: 100,
        height: 50,
        fill_hex: None,
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    }));
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Right".into(),
        x: 140,
        y: 30,
        width: 50,
        height: 40,
        fill_hex: None,
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    }));
    let line = r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"find_empty_space","arguments":{"direction":"right","width":"320","height":"240"}}}"#;
    let response = process_message_with_applier(&mut state, line, |_, _| false)
        .expect("dispatch")
        .expect("response");
    assert!(response.contains(r#""id":9"#), "{response}");
    assert!(response.contains(r#""x":"240""#), "{response}");
    assert!(response.contains(r#""y":"20""#), "{response}");
}

#[test]
fn read_nodes_accepts_structured_ids_over_mcp() {
    let mut state = op_editor_core::EditorState::new();
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Card".into(),
        x: 10,
        y: 20,
        width: 100,
        height: 50,
        fill_hex: None,
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    }));
    let node_id = state.active_children()[0].base().id.clone();
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{{"name":"read_nodes","arguments":{{"nodeIds":["{node_id}"],"depth":0}}}}}}"#
    );
    let response = process_message_with_applier(&mut state, &line, |_, _| false)
        .expect("dispatch")
        .expect("response");
    assert!(response.contains(r#""id":11"#), "{response}");
    assert!(response.contains(r#""count":"1""#), "{response}");
    assert!(response.contains(&node_id), "{response}");
}

#[test]
fn load_theme_preset_merges_live_doc_over_mcp() {
    let mut state = op_editor_core::EditorState::new();
    let dir = std::env::temp_dir().join(format!(
        "openpencil-theme-preset-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let preset_path = dir.join("dark.optheme");
    std::fs::write(
        &preset_path,
        r##"{
  "type": "openpencil-theme-preset",
  "version": "1.0.0",
  "name": "Dark",
  "themes": { "Mode": ["Light", "Dark"] },
  "variables": { "brand": { "type": "color", "value": "#101010" } }
}"##,
    )
    .expect("preset file");

    let preset_path_json =
        serde_json::to_string(&preset_path.to_string_lossy()).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{{"name":"load_theme_preset","arguments":{{"presetPath":{preset_path_json}}}}}}}"#
    );
    let response =
        process_message_with_applier(&mut state, &line, |state, cmd| state.apply(cmd.clone()))
            .expect("dispatch")
            .expect("response");
    assert!(response.contains(r#""id":12"#), "{response}");
    assert!(response.contains(r#""wrote":"true""#), "{response}");
    assert_eq!(
        state
            .doc
            .themes
            .as_ref()
            .and_then(|themes| themes.get("Mode"))
            .cloned(),
        Some(vec!["Light".to_string(), "Dark".to_string()])
    );
    assert!(state
        .doc
        .variables
        .as_ref()
        .is_some_and(|variables| variables.contains_key("brand")));

    let _ = std::fs::remove_file(&preset_path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn set_design_md_mutates_live_doc_over_mcp() {
    let mut state = op_editor_core::EditorState::new();
    let markdown = serde_json::to_string("# Design System: Aurora\n\n## Visual Theme\nCalm.")
        .expect("markdown json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{{"name":"set_design_md","arguments":{{"markdown":{markdown}}}}}}}"#
    );
    let response =
        process_message_with_applier(&mut state, &line, |state, cmd| state.apply(cmd.clone()))
            .expect("dispatch")
            .expect("response");
    assert!(response.contains(r#""id":13"#), "{response}");
    assert!(response.contains(r#""wrote":"true""#), "{response}");
    assert_eq!(
        state
            .doc
            .design_md
            .as_ref()
            .and_then(|spec| spec.project_name.as_deref()),
        Some("Aurora")
    );
}

#[test]
fn set_themes_accepts_structured_mcp_arguments_and_mutates_state() {
    let mut state = op_editor_core::EditorState::new();
    let line = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"set_themes","arguments":{"themes":{"Mode":["Light","Dark"]},"replace":true}}}"#;
    let response =
        process_message_with_applier(&mut state, line, |state, cmd| state.apply(cmd.clone()))
            .expect("dispatch")
            .expect("response");
    assert!(response.contains(r#""id":10"#), "{response}");
    assert!(response.contains(r#""wrote":"true""#), "{response}");
    assert_eq!(
        state
            .doc
            .themes
            .as_ref()
            .and_then(|themes| themes.get("Mode"))
            .cloned(),
        Some(vec!["Light".to_string(), "Dark".to_string()])
    );
}

fn temp_doc_paths(test_name: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "openpencil-mcp-filepath-{test_name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let primary = dir.join("primary.op");
    let alternate = dir.join("alternate.op");
    (dir, primary, alternate)
}

fn write_named_doc(path: &std::path::Path, node_id: &str, name: &str) {
    std::fs::write(
        path,
        format!(
            r##"{{
  "version": "1.0.0",
  "children": [
    {{
      "id": "{node_id}",
      "type": "rectangle",
      "name": "{name}",
      "x": 0,
      "y": 0,
      "width": 100,
      "height": 60,
      "fill": [{{ "type": "solid", "color": "#FFFFFF" }}]
    }}
  ]
}}"##
        ),
    )
    .expect("write doc");
}

#[test]
fn process_message_reads_document_from_ts_file_path_arg() {
    let (dir, primary_path, alternate_path) = temp_doc_paths("read");
    write_named_doc(&primary_path, "n1", "Primary");
    write_named_doc(&alternate_path, "n2", "Alternate");
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let file_path_json =
        serde_json::to_string(&alternate_path.to_string_lossy()).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{{"name":"batch_get","arguments":{{"filePath":{file_path_json},"readDepth":1}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert!(response.contains("Alternate"), "{response}");
    assert!(!response.contains("Primary"), "{response}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn process_message_open_document_reports_ts_file_path_target() {
    let (dir, primary_path, alternate_path) = temp_doc_paths("open");
    write_named_doc(&primary_path, "n1", "Primary");
    write_named_doc(&alternate_path, "n2", "Alternate");
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let alternate = alternate_path.to_string_lossy().to_string();
    let file_path_json = serde_json::to_string(&alternate).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{{"name":"open_document","arguments":{{"filePath":{file_path_json}}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert!(response.contains(&alternate), "{response}");
    assert!(!response.contains("warning"), "{response}");
    assert!(!response.contains("does not reopen files"), "{response}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn process_message_open_document_creates_missing_ts_file_path_target() {
    let (dir, primary_path, alternate_path) = temp_doc_paths("open-create");
    write_named_doc(&primary_path, "n1", "Primary");
    assert!(!alternate_path.exists());
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let alternate = alternate_path.to_string_lossy().to_string();
    let file_path_json = serde_json::to_string(&alternate).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":24,"method":"tools/call","params":{{"name":"open_document","arguments":{{"filePath":{file_path_json}}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert!(response.contains(&alternate), "{response}");
    assert!(
        alternate_path.exists(),
        "open_document should create the target .op file"
    );
    let created = std::fs::read_to_string(&alternate_path).expect("created document");
    assert!(created.contains(r#""version""#), "{created}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn process_message_writes_document_to_ts_file_path_arg() {
    let (dir, primary_path, alternate_path) = temp_doc_paths("write");
    write_named_doc(&primary_path, "n1", "Primary");
    write_named_doc(&alternate_path, "n2", "Alternate");
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let file_path_json =
        serde_json::to_string(&alternate_path.to_string_lossy()).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{{"name":"add_page","arguments":{{"filePath":{file_path_json},"name":"FromFilePath"}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert!(response.contains(r#""wrote":"true""#), "{response}");
    let primary_text = std::fs::read_to_string(&primary_path).expect("primary doc");
    let alternate_text = std::fs::read_to_string(&alternate_path).expect("alternate doc");
    assert!(!primary_text.contains("FromFilePath"), "{primary_text}");
    assert!(alternate_text.contains("FromFilePath"), "{alternate_text}");
    let _ = std::fs::remove_dir_all(dir);
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
