//! Read-tool registry round-trip tests. Split out of `mcp_tests.rs` to
//! keep each test file under the 800-line cap.

use crate::test_fixtures::{sample, state_with};
use crate::*;
use std::collections::BTreeMap;

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
fn snapshot_layout_emits_ts_json_tree_not_string() {
    use crate::test_fixtures::{frame, rect};
    // Parent at a NON-ZERO position so the parent-relative→absolute
    // accumulation is exercised: a child authored at relative (50,50) under a
    // frame at (100,50) must report absolute (150,100), matching TS
    // computeLayoutTree's `absX = parentX + bounds.x`.
    let f = frame(
        "n10",
        "F",
        100.0,
        50.0,
        200.0,
        100.0,
        vec![
            rect("n11", "a", 50.0, 50.0, 10.0, 10.0),
            rect("n12", "b", 70.0, 50.0, 10.0, 10.0),
        ],
    );
    let s = state_with(vec![f]);
    let mut r = ToolRegistry::default();
    r.register(Box::new(snapshot_layout_snapshot(&s)));
    let mut args = BTreeMap::new();
    args.insert("maxDepth".into(), "1".into());
    match r.dispatch(ToolCall {
        id: RequestId::Num(1),
        tool: "snapshot_layout".into(),
        arguments: args,
    }) {
        ToolResponse::Ok {
            json: Some(raw), ..
        } => {
            let value: serde_json::Value = serde_json::from_str(&raw).expect("layout json");
            let layout = value["layout"].as_array().expect("layout is a JSON array");
            assert_eq!(layout.len(), 1, "one root frame");
            let root = &layout[0];
            assert_eq!(root["id"], "n10");
            assert_eq!(root["name"], "F");
            assert_eq!(root["type"], "frame");
            // Top-level frame: absolute == its authored position.
            assert_eq!(root["x"], 100, "root x absolute: {root}");
            assert_eq!(root["y"], 50, "root y absolute: {root}");
            assert!(root["width"].is_number(), "width must be numeric: {root}");
            // Children nest as a JSON array (maxDepth=1 includes the 2 rects)
            // with ABSOLUTE coords = parent + relative (matching TS).
            let kids = root["children"].as_array().expect("children array");
            assert_eq!(kids.len(), 2);
            assert_eq!(kids[0]["id"], "n11");
            assert_eq!(
                kids[0]["x"], 150,
                "child abs x = parent 100 + relative 50: {}",
                kids[0]
            );
            assert_eq!(
                kids[0]["y"], 100,
                "child abs y = parent 50 + relative 50: {}",
                kids[0]
            );
            assert_eq!(kids[1]["type"], "rectangle");
            assert_eq!(kids[1]["x"], 170, "{}", kids[1]);
        }
        other => panic!("expected OkJson layout, got {other:?}"),
    }
}

#[test]
fn snapshot_layout_parent_id_returns_parent_relative_coords() {
    use crate::test_fixtures::{frame, rect};
    // TS calls computeLayoutTree(parent.children, …) with parentX/Y=0, so a
    // `parentId` query returns coords RELATIVE to that parent — NOT
    // document-absolute. Frame at (100,50); children authored at relative
    // (50,50)/(70,50) must report exactly (50,50)/(70,50) under parentId=n10.
    let f = frame(
        "n10",
        "F",
        100.0,
        50.0,
        200.0,
        100.0,
        vec![
            rect("n11", "a", 50.0, 50.0, 10.0, 10.0),
            rect("n12", "b", 70.0, 50.0, 10.0, 10.0),
        ],
    );
    let s = state_with(vec![f]);
    let mut r = ToolRegistry::default();
    r.register(Box::new(snapshot_layout_snapshot(&s)));
    let mut args = BTreeMap::new();
    args.insert("parentId".into(), "n10".into());
    args.insert("maxDepth".into(), "1".into());
    match r.dispatch(ToolCall {
        id: RequestId::Num(1),
        tool: "snapshot_layout".into(),
        arguments: args,
    }) {
        ToolResponse::Ok {
            json: Some(raw), ..
        } => {
            let value: serde_json::Value = serde_json::from_str(&raw).expect("layout json");
            let layout = value["layout"].as_array().expect("layout array");
            assert_eq!(layout.len(), 2, "roots = n10's children");
            assert_eq!(layout[0]["id"], "n11");
            assert_eq!(
                layout[0]["x"], 50,
                "parentId coords are parent-relative: {}",
                layout[0]
            );
            assert_eq!(layout[0]["y"], 50, "{}", layout[0]);
            assert_eq!(layout[1]["x"], 70, "{}", layout[1]);
        }
        other => panic!("expected OkJson layout, got {other:?}"),
    }
}

#[test]
fn snapshot_layout_unsized_node_uses_scene_aggregate_bounds() {
    use crate::test_fixtures::{group, rect};
    // `snapshot_layout` reports the same bounds the live canvas selection
    // overlay uses. A bounds-less group resolves to the union of its children
    // instead of the old TS `w || 100` placeholder.
    let g = group("g1", "G", vec![rect("r1", "a", 10.0, 20.0, 30.0, 40.0)]);
    let s = state_with(vec![g]);
    let mut r = ToolRegistry::default();
    r.register(Box::new(snapshot_layout_snapshot(&s)));
    let mut args = BTreeMap::new();
    args.insert("maxDepth".into(), "1".into());
    match r.dispatch(ToolCall {
        id: RequestId::Num(1),
        tool: "snapshot_layout".into(),
        arguments: args,
    }) {
        ToolResponse::Ok {
            json: Some(raw), ..
        } => {
            let v: serde_json::Value = serde_json::from_str(&raw).expect("layout json");
            let root = &v["layout"][0];
            assert_eq!(root["id"], "g1");
            assert_eq!(root["type"], "group");
            assert_eq!(root["x"], 10, "aggregate x follows child bounds: {root}");
            assert_eq!(root["y"], 20, "aggregate y follows child bounds: {root}");
            assert_eq!(
                root["width"], 30,
                "aggregate width follows child bounds: {root}"
            );
            assert_eq!(
                root["height"], 40,
                "aggregate height follows child bounds: {root}"
            );
        }
        other => panic!("expected OkJson layout, got {other:?}"),
    }
}

#[test]
#[cfg_attr(
    target_os = "windows",
    ignore = "Windows CI aborts in DirectWrite/Skia text measurement for fit-content layout"
)]
fn snapshot_layout_resolves_fit_content_text_to_measured_bounds() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"text","id":"t1","name":"Label","x":10,"y":20,
         "width":"fit_content","height":"fit_content",
         "content":"N490-测试-9","fontSize":14}
      ]}],"children":[]
    }"##;
    let parsed = jian_ops_schema::load_str(src).expect("parse .op fixture");
    let s = op_editor_core::EditorState::from_document(parsed.value);
    let mut r = ToolRegistry::default();
    r.register(Box::new(snapshot_layout_snapshot(&s)));
    let mut args = BTreeMap::new();
    args.insert("maxDepth".into(), "0".into());
    match r.dispatch(ToolCall {
        id: RequestId::Num(1),
        tool: "snapshot_layout".into(),
        arguments: args,
    }) {
        ToolResponse::Ok {
            json: Some(raw), ..
        } => {
            let v: serde_json::Value = serde_json::from_str(&raw).expect("layout json");
            let text = &v["layout"][0];
            assert_eq!(text["id"], "t1");
            assert_eq!(text["x"], 10);
            assert_eq!(text["y"], 20);
            let width = text["width"].as_f64().expect("numeric width");
            let height = text["height"].as_f64().expect("numeric height");
            assert!(
                width > op_editor_core::DEFAULT_TEXT_NODE_WIDTH as f64,
                "fit_content text should use measured content width, got {width}: {text}"
            );
            assert_ne!(width, 100.0, "fit_content text must not use 100 fallback");
            assert!(height >= 14.0, "height should fit the font, got {height}");
            assert_ne!(
                height, 100.0,
                "fit_content text height must not use 100 fallback"
            );
        }
        other => panic!("expected OkJson layout, got {other:?}"),
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
    assert!(!snap.pages[0].1.is_empty(), "page name must serialize");
}

#[test]
fn list_pages_emits_json_pages_array() {
    let mut s = state_with(vec![]);
    assert!(s.add_page_with_name(Some("Second".into())).is_some());
    let snap = list_pages_snapshot(&s);
    let pages = s.doc.pages.as_ref().expect("multi-page doc");
    match snap.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            let v: serde_json::Value = serde_json::from_str(&json).expect("pages json");
            assert_eq!(v["pageCount"], 2);
            let arr = v["pages"].as_array().expect("pages array");
            assert_eq!(arr[0]["id"], pages[0].id.as_str());
            assert_eq!(arr[1]["id"], pages[1].id.as_str());
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
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
fn parse_tool_call_unescapes_string_params() {
    let line = r#"{"id":1,"method":"set_node_text","params":{"node_id":"n1","text":"a\"b\\c"}}"#;
    let call = parse_tool_call(line).expect("must parse");
    assert_eq!(call.arguments.get("text"), Some(&r#"a"b\c"#.to_string()));
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
fn get_guidelines_style_flat_params_parses() {
    let line = r#"{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"get_guidelines","arguments":{"category":"style","name":"Atlas Grid","colorPalette":"Alloy Blue","roundness":"medium","elevation":"low","headings":"Inter","body":"Inter","captions":"Inter","data":"IBM Plex Mono","decorativeImagery":"product diagrams only when they clarify state"}}}"#;
    let call = parse_tool_call(line).expect("flat scalar style args must parse");
    assert_eq!(call.tool, "get_guidelines");
    assert_eq!(
        call.arguments.get("colorPalette"),
        Some(&"Alloy Blue".to_string())
    );
    assert_eq!(
        call.arguments.get("data"),
        Some(&"IBM Plex Mono".to_string())
    );

    let mut registry = ToolRegistry::default();
    registry.register(Box::new(get_guidelines_snapshot()));
    match registry.dispatch(call) {
        ToolResponse::Ok { result, .. } => {
            assert_eq!(result.get("category").map(String::as_str), Some("style"));
            let content = result.get("content").expect("content field");
            assert!(
                content.contains("Atlas Grid is a practical workspace style"),
                "style guideline must dispatch through registry: {content}"
            );
            assert!(
                content.contains("on-surface.primary"),
                "computed on-* tokens must be serialized: {content}"
            );
        }
        other => panic!("expected Ok dispatch, got {other:?}"),
    }
}

#[test]
fn parse_tool_call_allows_structured_variable_payloads_for_ts_parity() {
    let vars = r##"{"id":1,"method":"tools/call","params":{"name":"set_variables","arguments":{"variables":{"brand":{"type":"color","value":"#ff0000"}},"replace":true}}}"##;
    let call = parse_tool_call(vars).expect("set_variables must accept TS-style object args");
    assert_eq!(call.tool, "set_variables");
    assert_eq!(
        call.arguments.get("variables"),
        Some(&r##"{"brand":{"type":"color","value":"#ff0000"}}"##.to_string())
    );
    assert_eq!(call.arguments.get("replace"), Some(&"true".to_string()));

    let themes = r#"{"id":2,"method":"tools/call","params":{"name":"set_themes","arguments":{"themes":{"Mode":["Light","Dark"]}}}}"#;
    let call = parse_tool_call(themes).expect("set_themes must accept TS-style object args");
    assert_eq!(call.tool, "set_themes");
    assert_eq!(
        call.arguments.get("themes"),
        Some(&r#"{"Mode":["Light","Dark"]}"#.to_string())
    );
}

#[test]
fn parse_tool_call_allows_structured_read_nodes_ids_for_ts_parity() {
    let line = r#"{"id":3,"method":"tools/call","params":{"name":"read_nodes","arguments":{"nodeIds":["n10","n11"],"depth":0}}}"#;
    let call = parse_tool_call(line).expect("read_nodes must accept TS-style nodeIds array");
    assert_eq!(call.tool, "read_nodes");
    assert_eq!(
        call.arguments.get("nodeIds"),
        Some(&r#"["n10","n11"]"#.to_string())
    );
    assert_eq!(call.arguments.get("depth"), Some(&"0".to_string()));
}

#[test]
fn parse_tool_call_allows_structured_style_ops_args_for_ts_parity() {
    let line = r#"{"id":5,"method":"tools/call","params":{"name":"search_all_unique_properties","arguments":{"parents":["n1"],"properties":["fillColor","textColor"]}}}"#;
    let call = parse_tool_call(line).expect("style ops must accept TS-style array arguments");
    assert_eq!(call.tool, "search_all_unique_properties");
    assert_eq!(
        call.arguments.get("parents").map(String::as_str),
        Some(r#"["n1"]"#)
    );
    assert_eq!(
        call.arguments.get("properties").map(String::as_str),
        Some(r#"["fillColor","textColor"]"#)
    );
}

#[test]
fn parse_tool_call_allows_structured_replace_style_ops_args_for_ts_parity() {
    let line = r##"{"id":6,"method":"tools/call","params":{"name":"replace_all_matching_properties","arguments":{"parents":["n1"],"properties":{"fillColor":[{"from":"#fff","to":"#000"}]}}}}"##;
    let call = parse_tool_call(line)
        .expect("replace style ops must accept TS-style array/object arguments");
    assert_eq!(call.tool, "replace_all_matching_properties");
    assert_eq!(
        call.arguments.get("parents").map(String::as_str),
        Some(r#"["n1"]"#)
    );
    assert_eq!(
        call.arguments.get("properties").map(String::as_str),
        Some(r##"{"fillColor":[{"from":"#fff","to":"#000"}]}"##)
    );
}

#[test]
fn parse_tool_call_allows_structured_batch_get_args_for_ts_parity() {
    let line = r#"{"id":4,"method":"tools/call","params":{"name":"batch_get","arguments":{"patterns":[{"name":"Button"}],"nodeIds":["n11"],"readDepth":0}}}"#;
    let call = parse_tool_call(line).expect("batch_get must accept TS-style structured args");
    assert_eq!(call.tool, "batch_get");
    assert_eq!(
        call.arguments.get("patterns"),
        Some(&r#"[{"name":"Button"}]"#.to_string())
    );
    assert_eq!(
        call.arguments.get("nodeIds"),
        Some(&r#"["n11"]"#.to_string())
    );
    assert_eq!(call.arguments.get("readDepth"), Some(&"0".to_string()));
}

#[test]
fn parse_tool_call_allows_structured_insert_node_data_for_ts_parity() {
    let line = r##"{"id":4,"method":"tools/call","params":{"name":"insert_node","arguments":{"parent":null,"data":{"type":"rectangle","name":"Card","x":1,"y":2,"width":100,"height":50,"fill":[{"type":"solid","color":"#112233"}]},"pageId":"page-2"}}}"##;
    let call = parse_tool_call(line).expect("insert_node must accept TS-style data object");
    assert_eq!(call.tool, "insert_node");
    assert_eq!(call.arguments.get("parent"), Some(&"null".to_string()));
    assert_eq!(
        call.arguments.get("data"),
        Some(
            &r##"{"type":"rectangle","name":"Card","x":1,"y":2,"width":100,"height":50,"fill":[{"type":"solid","color":"#112233"}]}"##
                .to_string()
        )
    );
    assert_eq!(call.arguments.get("pageId"), Some(&"page-2".to_string()));
}

#[test]
fn parse_tool_call_allows_structured_update_node_data_for_ts_parity() {
    let line = r##"{"id":5,"method":"tools/call","params":{"name":"update_node","arguments":{"nodeId":"n10","data":{"name":"Updated","x":5,"fill":[{"type":"solid","color":"#123456"}]},"pageId":"page-2"}}}"##;
    let call = parse_tool_call(line).expect("update_node must accept TS-style data object");
    assert_eq!(call.tool, "update_node");
    assert_eq!(call.arguments.get("nodeId"), Some(&"n10".to_string()));
    assert_eq!(
        call.arguments.get("data"),
        Some(
            &r##"{"name":"Updated","x":5,"fill":[{"type":"solid","color":"#123456"}]}"##
                .to_string()
        )
    );
    assert_eq!(call.arguments.get("pageId"), Some(&"page-2".to_string()));
}

#[test]
fn parse_tool_call_allows_structured_copy_node_overrides_for_ts_parity() {
    let line = r##"{"id":7,"method":"tools/call","params":{"name":"copy_node","arguments":{"sourceId":"n10","parent":null,"overrides":{"name":"Copy","x":24,"id":"ignored"},"pageId":"page-2"}}}"##;
    let call = parse_tool_call(line).expect("copy_node must accept TS-style overrides object");
    assert_eq!(call.tool, "copy_node");
    assert_eq!(call.arguments.get("sourceId"), Some(&"n10".to_string()));
    assert_eq!(call.arguments.get("parent"), Some(&"null".to_string()));
    assert_eq!(
        call.arguments.get("overrides"),
        Some(&r#"{"name":"Copy","x":24,"id":"ignored"}"#.to_string())
    );
    assert_eq!(call.arguments.get("pageId"), Some(&"page-2".to_string()));
}

#[test]
fn parse_tool_call_allows_structured_replace_node_data_for_ts_parity() {
    let line = r##"{"id":6,"method":"tools/call","params":{"name":"replace_node","arguments":{"nodeId":"n10","data":{"type":"rectangle","name":"Replacement","x":5,"width":100,"height":50,"fill":[{"type":"solid","color":"#123456"}]},"pageId":"page-2"}}}"##;
    let call = parse_tool_call(line).expect("replace_node must accept TS-style data object");
    assert_eq!(call.tool, "replace_node");
    assert_eq!(call.arguments.get("nodeId"), Some(&"n10".to_string()));
    assert_eq!(
        call.arguments.get("data"),
        Some(
            &r##"{"type":"rectangle","name":"Replacement","x":5,"width":100,"height":50,"fill":[{"type":"solid","color":"#123456"}]}"##
                .to_string()
        )
    );
    assert_eq!(call.arguments.get("pageId"), Some(&"page-2".to_string()));
}

#[test]
fn parse_tool_call_allows_structured_style_guide_tags_for_ts_parity() {
    let line = r#"{"id":4,"method":"tools/call","params":{"name":"get_style_guide","arguments":{"tags":["light-mode","clean"],"platform":"webapp"}}}"#;
    let call = parse_tool_call(line).expect("get_style_guide must accept TS-style tags array");
    assert_eq!(call.tool, "get_style_guide");
    assert_eq!(
        call.arguments.get("tags"),
        Some(&r#"["light-mode","clean"]"#.to_string())
    );
    assert_eq!(call.arguments.get("platform"), Some(&"webapp".to_string()));
}

#[test]
fn parse_tool_call_allows_structured_codegen_args_for_ts_parity() {
    let plan_line = r#"{"id":5,"method":"tools/call","params":{"name":"codegen_plan","arguments":{"plan":{"chunks":[{"chunkId":"hero","nodeIds":["n1"],"dependsOn":[]}],"sharedStyles":[],"rootLayout":{"nodeId":"n1"}},"pageId":"page-1"}}}"#;
    let plan = parse_tool_call(plan_line).expect("codegen_plan must accept TS-style plan object");
    assert_eq!(plan.tool, "codegen_plan");
    assert_eq!(
        plan.arguments.get("plan"),
        Some(
            &r#"{"chunks":[{"chunkId":"hero","nodeIds":["n1"],"dependsOn":[]}],"sharedStyles":[],"rootLayout":{"nodeId":"n1"}}"#
                .to_string()
        )
    );
    assert_eq!(plan.arguments.get("pageId"), Some(&"page-1".to_string()));

    let submit_line = r#"{"id":6,"method":"tools/call","params":{"name":"codegen_submit_chunk","arguments":{"planId":"plan-1","result":{"chunkId":"hero","code":"export const Hero = () => null;","contract":{"provides":[],"requires":[]}},"status":"failed"}}}"#;
    let submit =
        parse_tool_call(submit_line).expect("codegen_submit_chunk must accept result object");
    assert_eq!(submit.tool, "codegen_submit_chunk");
    assert_eq!(
        submit.arguments.get("result"),
        Some(
            &r#"{"chunkId":"hero","code":"export const Hero = () => null;","contract":{"provides":[],"requires":[]}}"#
                .to_string()
        )
    );
    assert_eq!(submit.arguments.get("status"), Some(&"failed".to_string()));
}

#[test]
fn parse_tool_call_allows_structured_design_content_children_for_ts_parity() {
    let line = r#"{"id":7,"method":"tools/call","params":{"name":"design_content","arguments":{"sectionId":"section-1","children":[{"type":"text","name":"Title","content":"Hello","width":120,"height":24}]}}}"#;
    let call = parse_tool_call(line).expect("design_content must accept TS-style children array");
    assert_eq!(call.tool, "design_content");
    assert_eq!(
        call.arguments.get("sectionId"),
        Some(&"section-1".to_string())
    );
    assert_eq!(
        call.arguments.get("children"),
        Some(
            &r#"[{"type":"text","name":"Title","content":"Hello","width":120,"height":24}]"#
                .to_string()
        )
    );
}

#[test]
fn parse_tool_call_allows_structured_add_page_children_for_ts_parity() {
    let line = r#"{"id":8,"method":"tools/call","params":{"name":"add_page","arguments":{"name":"Landing","children":[{"type":"frame","name":"Hero","width":1200,"height":640,"children":[]}]}}}"#;
    let call = parse_tool_call(line).expect("add_page must accept TS-style children array");
    assert_eq!(call.tool, "add_page");
    assert_eq!(call.arguments.get("name"), Some(&"Landing".to_string()));
    assert_eq!(
        call.arguments.get("children"),
        Some(
            &r#"[{"type":"frame","name":"Hero","width":1200,"height":640,"children":[]}]"#
                .to_string()
        )
    );
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
