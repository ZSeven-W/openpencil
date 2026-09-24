//! Lean-profile tests: the exact `tools/list`, one real dispatch per lean
//! tool against a fixture document, and refusal of everything else.

use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use serde_json::{json, Value};

use super::super::tool_catalog::LEAN_TOOLS;
use super::super::tool_profile::McpAccessProfile;
use super::super::{
    process_message_with_applier_profiled, tool_text, tools_list_response, TOOL_SCHEMAS,
};

/// A page with one filled 320×200 frame; returns the state and the frame id.
fn fixture() -> (EditorState, String) {
    let mut state = EditorState::new();
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "frame".into(),
        name: "Card".into(),
        x: 0,
        y: 0,
        width: 320,
        height: 200,
        fill_hex: Some("#ffffff".into()),
        target_parent: NodeId::NONE,
        page_id: None,
    }));
    let id = state.active_children()[0].id_str().to_string();
    (state, id)
}

fn tools_list(profile: McpAccessProfile, debug: bool) -> Vec<Value> {
    let response = tools_list_response("1", &EditorState::new(), debug, profile);
    let value: Value = serde_json::from_str(&response).expect("tools/list JSON");
    value["result"]["tools"]
        .as_array()
        .expect("tools array")
        .clone()
}

/// Dispatch one lean `tools/call`; returns the raw response and whether any
/// command was applied.
fn call(state: &mut EditorState, tool: &str, arguments: Value) -> (String, bool) {
    let line = json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "tools/call",
        "params": {"name": tool, "arguments": arguments},
    })
    .to_string();
    let mut applied = false;
    let response =
        process_message_with_applier_profiled(state, &line, McpAccessProfile::LEAN, |_, s, cmd| {
            let ok = s.apply(cmd.clone());
            applied |= ok;
            ok
        })
        .expect("dispatch")
        .expect("response");
    (response, applied)
}

fn is_error(response: &str) -> bool {
    response.contains(r#""isError":true"#)
}

fn text_json(response: &str) -> Value {
    serde_json::from_str(&tool_text(response)).expect("tool result JSON")
}

#[test]
fn lean_tools_list_is_exactly_the_six_workflow_tools() {
    for debug in [false, true] {
        let tools = tools_list(McpAccessProfile::LEAN, debug);
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().expect("name"))
            .collect();
        assert_eq!(
            names,
            [
                "get_editor_state",
                "get_guidelines",
                "batch_design",
                "snapshot_layout",
                "get_screenshot",
                "finalize_design",
            ],
            "lean tools/list must be exactly the six tools in workflow order"
        );
        assert_eq!(names, LEAN_TOOLS);
    }
}

#[test]
fn full_tools_list_is_unchanged_by_the_lean_profile() {
    let state = EditorState::new();
    let tools = tools_list(McpAccessProfile::UNRESTRICTED, false);
    assert_eq!(
        tools.len(),
        TOOL_SCHEMAS.len() + op_mcp::element_tools::element_tool_schemas(&state).len(),
        "the full catalog keeps every static schema plus the kit insert tools"
    );
}

#[test]
fn lean_schemas_document_the_workflow_and_stay_valid() {
    let tools = tools_list(McpAccessProfile::LEAN, false);
    let lean_text: usize = tools.iter().map(|tool| tool.to_string().len()).sum();
    let full_text: usize = TOOL_SCHEMAS.iter().map(|schema| schema.len()).sum();
    assert!(
        lean_text * 5 < full_text,
        "the lean catalog must be a small fraction of the full one ({lean_text} vs {full_text})"
    );
    for tool in &tools {
        let name = tool["name"].as_str().expect("name");
        let description = tool["description"].as_str().expect("description");
        assert!(
            description.contains("lean profile") && description.contains("STEP"),
            "{name} must state its workflow step: {description}"
        );
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{name} properties"));
        assert!(
            !properties.contains_key("filePath"),
            "{name}: lean sessions never retarget files"
        );
        for (key, property) in properties {
            if property["type"] == "array" {
                assert!(property.get("items").is_some(), "{name}.{key} needs items");
            }
        }
    }
    let editor_state = &tools[0]["inputSchema"];
    assert!(editor_state["properties"].get("nodeIds").is_some());
    assert!(editor_state.get("additionalProperties").is_none());
    let screenshot = &tools[4]["inputSchema"]["properties"];
    assert!(screenshot.get("format").is_some() && screenshot.get("nodeId").is_some());
    let batch = &tools[2]["inputSchema"]["properties"];
    for key in ["script", "operations", "nodes_json"] {
        assert!(batch.get(key).is_some(), "batch_design keeps {key}");
    }
    let finalize = tools[5]["description"]
        .as_str()
        .expect("finalize description");
    assert!(finalize.contains("MUST NOT present the design as finished while complete=false"));
}

#[test]
fn get_editor_state_merges_doc_info_pages_variables_and_selection() {
    let (mut state, frame) = fixture();
    state.set_single_selection(NodeId::new(&frame));
    let (response, applied) = call(&mut state, "get_editor_state", json!({}));
    assert!(!is_error(&response), "{response}");
    assert!(!applied);
    let result = text_json(&response);
    for key in ["editor", "document", "pages", "variables", "theme"] {
        assert!(result.get(key).is_some(), "missing {key}: {result}");
    }
    assert!(
        result.get("nodes").is_none(),
        "nodes only on request: {result}"
    );
    assert_eq!(result["editor"]["selection_ids"], frame.as_str());
    assert!(result["editor"]["top_level_nodes"]
        .as_str()
        .is_some_and(|nodes| nodes.contains("Card")));
    assert_eq!(result["document"]["total_nodes"], "1", "{result}");
}

#[test]
fn get_editor_state_reads_nodes_through_read_nodes() {
    let (mut state, frame) = fixture();
    let (response, _) = call(
        &mut state,
        "get_editor_state",
        json!({"nodeIds": [frame.clone()], "depth": 0}),
    );
    assert!(!is_error(&response), "{response}");
    let nodes = text_json(&response)["nodes"].clone();
    assert_eq!(nodes[0]["id"], frame.as_str(), "{nodes}");
    assert_eq!(nodes[0]["name"], "Card", "{nodes}");

    let (bad, _) = call(&mut state, "get_editor_state", json!({"depth": "deep"}));
    assert!(is_error(&bad), "read_nodes' own validation surfaces: {bad}");
}

#[test]
fn get_guidelines_serves_a_topic() {
    let (mut state, _) = fixture();
    let (response, _) = call(&mut state, "get_guidelines", json!({"topic": "mobile"}));
    assert!(!is_error(&response), "{response}");
    assert!(tool_text(&response).len() > 200, "{response}");
}

#[test]
fn batch_design_edits_existing_nodes_and_creates_new_ones() {
    let (mut state, frame) = fixture();
    let update = format!(r#"U("{frame}",{{"name":"Hero"}})"#);
    let (response, applied) = call(&mut state, "batch_design", json!({"operations": update}));
    assert!(!is_error(&response), "{response}");
    assert!(applied, "an update must reach the applier");
    assert_eq!(
        state.active_children()[0].base().name.as_deref(),
        Some("Hero")
    );

    let script = format!(
        r##"I("{frame}", {{"type":"rectangle","name":"Badge","width":40,"height":20,"fill":"#ff0000"}})"##
    );
    let (response, applied) = call(&mut state, "batch_design", json!({"script": script}));
    assert!(!is_error(&response), "{response}");
    assert!(applied);
    let children = state.active_children()[0]
        .children()
        .expect("frame children");
    assert!(children
        .iter()
        .any(|child| child.base().name.as_deref() == Some("Badge")));

    let delete = format!(r#"D("{frame}")"#);
    let (response, applied) = call(&mut state, "batch_design", json!({"operations": delete}));
    assert!(!is_error(&response), "{response}");
    assert!(applied);
    assert!(
        state.active_children().is_empty(),
        "D() deletes the subtree"
    );
}

#[test]
fn snapshot_layout_reports_the_fixture_frame() {
    let (mut state, frame) = fixture();
    let (response, _) = call(&mut state, "snapshot_layout", json!({}));
    assert!(!is_error(&response), "{response}");
    assert!(tool_text(&response).contains(&frame), "{response}");
}

#[test]
fn get_screenshot_renders_an_image_and_exports_on_request() {
    let (mut state, frame) = fixture();
    let (response, _) = call(&mut state, "get_screenshot", json!({"nodeId": "root"}));
    assert!(!is_error(&response), "{response}");
    assert!(response.contains(r#""type":"image""#), "{response}");

    let (response, _) = call(
        &mut state,
        "get_screenshot",
        json!({"nodeId": frame, "format": "jpeg", "scale": 2}),
    );
    assert!(!is_error(&response), "{response}");
    let export = text_json(&response);
    let file = &export["files"][0];
    assert!(
        file["bytes_base64"].as_str().is_some_and(|b| !b.is_empty()),
        "{export}"
    );

    let (missing, _) = call(&mut state, "get_screenshot", json!({"format": "png"}));
    assert!(is_error(&missing), "{missing}");
}

#[test]
fn finalize_design_runs_the_repair_passes() {
    let (mut state, _) = fixture();
    let (response, _) = call(&mut state, "finalize_design", json!({}));
    assert!(!is_error(&response), "{response}");
    assert!(tool_text(&response).contains("complete"), "{response}");
}

#[test]
fn out_of_profile_tools_are_refused_before_they_run() {
    for (tool, arguments) in [
        ("delete_node", json!({"node_id": "x"})),
        ("insert_btn_primary", json!({})),
        ("ToolSearch", json!({"query": "select:delete_node"})),
        ("read_nodes", json!({})),
        ("no_such_tool", json!({})),
    ] {
        let (mut state, _) = fixture();
        let revision = state.document_revision();
        let (response, applied) = call(&mut state, tool, arguments);
        assert!(is_error(&response), "{tool}: {response}");
        assert!(
            response.contains("tool-not-in-profile") && response.contains("get_editor_state"),
            "{tool}: the refusal must name the code and the available tools: {response}"
        );
        assert!(response.contains(r#""id":9"#), "{response}");
        assert!(!applied, "{tool} must not reach the applier");
        assert_eq!(state.document_revision(), revision);
    }
}

struct HttpMock {
    input: std::io::Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl std::io::Read for HttpMock {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.input, buf)
    }
}

impl std::io::Write for HttpMock {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn mcp_http_tools(path: &str, profile: McpAccessProfile) -> usize {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = HttpMock {
        input: std::io::Cursor::new(request.into_bytes()),
        output: Vec::new(),
    };
    let mut state = EditorState::new();
    super::super::auto_finalize::serve_http_connection(
        &mut stream,
        &mut state,
        std::path::Path::new("/tmp/unused-lean.op"),
        None,
        profile,
    )
    .expect("serve");
    let response = String::from_utf8(stream.output).expect("utf8");
    let body = response.split_once("\r\n\r\n").expect("body").1;
    let value: Value = serde_json::from_str(body).expect("json");
    value["result"]["tools"].as_array().expect("tools").len()
}

#[test]
fn mcp_http_serves_the_configured_catalog_and_lean_on_its_own_path() {
    assert_eq!(mcp_http_tools("/mcp", McpAccessProfile::LEAN), 6);
    assert_eq!(
        mcp_http_tools("/mcp/lean", McpAccessProfile::UNRESTRICTED),
        6
    );
    assert!(mcp_http_tools("/mcp", McpAccessProfile::UNRESTRICTED) > 100);
}
