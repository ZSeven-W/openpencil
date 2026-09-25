//! Tool-dispatch tests for the stdio/HTTP MCP server: live-document tools
//! (`find_empty_space`, `read_nodes`, theme / design.md mutations), the online
//! scene-template gate, and `export_frames`. Split out of `mcp_serve/tests.rs`
//! at the 800-line cap; nested under that module so `use super::*` still
//! reaches its helpers and `mcp_serve`'s own items.

use super::*;

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
    let response = process_message_with_applier(&mut state, line, |_, _, _| false)
        .expect("dispatch")
        .expect("response");
    assert!(response.contains(r#""id":9"#), "{response}");
    let result = crate::mcp_serve::tool_text(&response);
    assert!(result.contains(r#""x":"240""#), "{result}");
    assert!(result.contains(r#""y":"20""#), "{result}");
}

#[test]
fn file_mcp_applier_normalizes_mobile_document_before_save() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "openpencil-mcp-mobile-normalize-{}-{suffix}.op",
        std::process::id()
    ));
    let mut state = op_editor_core::EditorState::new();
    let operations = r##"root=I(null,{"type":"frame","name":"Screen","width":375,"height":"fit_content","fill":[{"type":"solid","color":"#f7f8fa"}],"children":[{"type":"frame","name":"Status Bar","width":"fill_container","height":62,"children":[{"type":"text","content":"9:41"},{"type":"text","content":"signal wifi battery"}]},{"type":"path","name":"ChevronDownIcon","role":"icon","d":"M6 9l6 6 6-6","width":14,"height":14,"stroke":{"thickness":2.2,"fill":[{"type":"solid","color":"#1A1614"}]}}]})"##;
    let line = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "batch_design", "arguments": {"operations": operations}}
    })
    .to_string();

    let response = process_message(&mut state, &path, &line)
        .expect("dispatch")
        .expect("response");
    assert!(!response.contains(r#""isError":true"#), "{response}");

    let saved = load_editor_state(&path).expect("saved document");
    let root = &saved.active_children()[0];
    assert_eq!(root.width_px(), Some(375.0));
    assert_eq!(root.height_px(), Some(812.0));
    let status_bar = &root.children().expect("root children")[0];
    assert_eq!(status_bar.base().role.as_deref(), Some("status-bar"));
    assert!(status_bar.children().is_some_and(|children| {
        children
            .iter()
            .any(|child| child.base().name.as_deref() == Some("Levels"))
    }));
    let icon = &root.children().expect("root children")[1];
    let jian_ops_schema::node::PenNode::IconFont(icon) = icon else {
        panic!("hand-drawn chevron should be normalized before save")
    };
    assert_eq!(icon.icon_font_name, "chevron-down");
    assert_eq!(icon.icon_font_family.as_deref(), Some("lucide"));
    assert_eq!(
        icon.fill
            .as_ref()
            .and_then(|fills| fills.first())
            .and_then(|fill| {
                let jian_ops_schema::style::PenFill::Solid(body) = fill else {
                    return None;
                };
                Some(body.color.as_str())
            }),
        Some("#1A1614")
    );

    let _ = std::fs::remove_file(&path);
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
    let response = process_message_with_applier(&mut state, &line, |_, _, _| false)
        .expect("dispatch")
        .expect("response");
    assert!(response.contains(r#""id":11"#), "{response}");
    let result = crate::mcp_serve::tool_text(&response);
    // TS read-nodes: { nodes, variables?, themes? } — native, no `count`.
    assert!(
        result.contains(r#""nodes""#) && !result.contains(r#""count""#),
        "{result}"
    );
    assert!(result.contains(&node_id), "{result}");
}

#[test]
fn full_design_context_tools_dispatch_as_read_only_nested_json() {
    let mut state = op_editor_core::EditorState::new();
    let calls = [
        (
            "get_design_agent_prompt",
            r#"{"userMessage":"Design an analytics dashboard","verifyProtocol":"layout"}"#,
            &["prompt", "verifyProtocol", "Product-Design Depth"][..],
        ),
        (
            "list_ui_kits",
            r#"{"kitId":"openpencil-starter","limit":2}"#,
            &["kits", "scriptRef", "starter/btn-primary"][..],
        ),
        (
            "get_design_quality",
            r#"{}"#,
            &["geometryIssues", "contrastIssues", "navIssues"][..],
        ),
    ];
    for (index, (tool, arguments, expected)) in calls.iter().enumerate() {
        let line = format!(
            r#"{{"jsonrpc":"2.0","id":{},"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#,
            index + 30
        );
        let mut applied = false;
        let response = process_message_with_applier(&mut state, &line, |_, _, _| {
            applied = true;
            true
        })
        .expect("dispatch")
        .expect("response");
        assert!(!applied, "read tool {tool} must emit no editor command");
        let result = crate::mcp_serve::tool_text(&response);
        for needle in *expected {
            assert!(result.contains(needle), "{tool} missing {needle}: {result}");
        }
    }
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
        process_message_with_applier(&mut state, &line, |_, state, cmd| state.apply(cmd.clone()))
            .expect("dispatch")
            .expect("response");
    assert!(response.contains(r#""id":12"#), "{response}");
    assert!(
        crate::mcp_serve::tool_text(&response).contains(r#""wrote":"true""#),
        "{response}"
    );
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
        process_message_with_applier(&mut state, &line, |_, state, cmd| state.apply(cmd.clone()))
            .expect("dispatch")
            .expect("response");
    assert!(response.contains(r#""id":13"#), "{response}");
    assert!(
        crate::mcp_serve::tool_text(&response).contains(r#""wrote":"true""#),
        "{response}"
    );
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
        process_message_with_applier(&mut state, line, |_, state, cmd| state.apply(cmd.clone()))
            .expect("dispatch")
            .expect("response");
    assert!(response.contains(r#""id":10"#), "{response}");
    assert!(
        crate::mcp_serve::tool_text(&response).contains(r#""wrote":"true""#),
        "{response}"
    );
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

#[test]
fn online_dispatch_refuses_user_scene_templates_before_the_tool_runs() {
    let mut state = op_editor_core::EditorState::new();
    let line = r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"use_scene_template","arguments":{"templateId":"user:private-deck"}}}"#;
    let mut applied = false;
    let response = process_message_with_applier_profiled(
        &mut state,
        line,
        tool_profile::McpAccessProfile::online(tool_profile::McpScopes::FULL),
        |_, _, _| {
            applied = true;
            true
        },
    )
    .expect("dispatch")
    .expect("response");

    assert!(response.contains(r#""isError":true"#), "{response}");
    assert!(
        response.contains("user-template-not-available"),
        "the profile must reject the user half explicitly: {response}"
    );
    assert!(!applied, "a refused user template must emit no command");
}

#[test]
fn online_dispatch_keeps_shipped_scene_templates_available() {
    let mut state = op_editor_core::EditorState::new();
    let line = r#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"use_scene_template","arguments":{"templateId":"slide-deck"}}}"#;
    let mut applied_id = None;
    let response = process_message_with_applier_profiled(
        &mut state,
        line,
        tool_profile::McpAccessProfile::online(tool_profile::McpScopes::FULL),
        |_, _, command| {
            let EditorCommand::AdoptSceneTemplate { template_id } = command else {
                return false;
            };
            applied_id = Some(template_id.clone());
            true
        },
    )
    .expect("dispatch")
    .expect("response");

    assert!(!response.contains(r#""isError""#), "{response}");
    assert_eq!(applied_id.as_deref(), Some("slide-deck"));
}

#[test]
fn scene_template_listing_is_shipped_only_online_and_two_source_locally() {
    let _guard = scene_template_tools::exclusive_user_template_registry_for_tests();
    op_editor_core::user_scene_templates::load_user_scene_template(
        op_editor_core::user_scene_templates::UserSceneTemplate {
            id: "user:private-deck".to_string(),
            name: "Private Deck".to_string(),
            frames: 1,
            frame_width: 1920,
            frame_height: 1080,
            document: r#"{"version":"1.0.0","children":[]}"#.to_string(),
            preview_jpeg: Vec::new(),
        },
    )
    .expect("register user template fixture");
    let line = r#"{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"list_scene_templates","arguments":{}}}"#;

    let online = process_message_with_applier_profiled(
        &mut op_editor_core::EditorState::new(),
        line,
        tool_profile::McpAccessProfile::online(tool_profile::McpScopes::FULL),
        |_, _, _| false,
    )
    .expect("online dispatch")
    .expect("online response");
    let online_text = crate::mcp_serve::tool_text(&online);
    assert!(online_text.contains("slide-deck"), "{online_text}");
    assert!(!online_text.contains("user:private-deck"), "{online_text}");

    let local =
        process_message_with_applier(&mut op_editor_core::EditorState::new(), line, |_, _, _| {
            false
        })
        .expect("local dispatch")
        .expect("local response");
    let local_text = crate::mcp_serve::tool_text(&local);
    assert!(local_text.contains("slide-deck"), "{local_text}");
    assert!(local_text.contains("user:private-deck"), "{local_text}");
}

#[test]
fn local_tool_search_keeps_local_resource_descriptors() {
    let line = r#"{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"ToolSearch","arguments":{"query":"select:save_document,get_node","max_results":11}}}"#;
    let response =
        process_message_with_applier(&mut op_editor_core::EditorState::new(), line, |_, _, _| {
            false
        })
        .expect("local dispatch")
        .expect("local response");
    let result: serde_json::Value = serde_json::from_str(&crate::mcp_serve::tool_text(&response))
        .expect("ToolSearch result JSON");
    let names: Vec<&str> = result["results"]
        .as_array()
        .expect("results array")
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert_eq!(names, ["save_document", "get_node"], "{result}");
}

#[test]
fn export_frames_writes_one_image_per_top_level_frame() {
    use op_mcp::{McpTool as _, ToolOutcome};
    use std::collections::BTreeMap;

    let mut state = op_editor_core::EditorState::new();
    for (index, name) in ["Cover", "Agenda"].iter().enumerate() {
        state.apply(op_editor_core::EditorCommand::InsertNode {
            kind: "frame".into(),
            name: (*name).into(),
            x: (index as i32) * 400,
            y: 0,
            width: 320,
            height: 180,
            // A frame with no fill paints nothing and the exporter refuses
            // it, which is the behaviour the failure branch below covers.
            fill_hex: Some("#ffffff".into()),
            target_parent: op_editor_core::NodeId::NONE,
            page_id: None,
        });
    }

    let directory = std::env::temp_dir().join("op-mcp-export-frames-test");
    let _ = std::fs::remove_dir_all(&directory);
    let mut args = BTreeMap::new();
    args.insert("outputDir".to_string(), directory.display().to_string());

    let outcome = super::export_frames_tool::export_frames_snapshot(&state).call(&args);
    let ToolOutcome::OkJson(json) = outcome else {
        panic!("unexpected outcome: {outcome:?}");
    };
    let report: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(report["attempted"].as_u64(), Some(2), "{json}");
    assert!(
        report["failed"].as_array().is_some_and(Vec::is_empty),
        "{json}"
    );

    // The report is only a claim until the files are on disk.
    let written = report["written"].as_array().expect("written array");
    assert_eq!(written.len(), 2);
    for entry in written {
        let path = directory.join(entry.as_str().expect("file name"));
        assert!(
            path.is_file(),
            "{} was reported but not written",
            path.display()
        );
        assert!(std::fs::metadata(&path).expect("stat").len() > 0);
    }
    let _ = std::fs::remove_dir_all(&directory);
}
