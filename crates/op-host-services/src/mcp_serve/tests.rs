//! `mcp_serve` tests — split out of `mcp_serve.rs` to keep that
//! file under the 800-line cap.

#![cfg(test)]

use super::*;
use op_editor_core::pen_node_ext::PenNodeExt;

#[test]
fn tools_list_response_includes_all_registered_tools() {
    // Debug gating is passed explicitly (no process-global env mutation,
    // so this test can't race other tests' env access).
    let state = op_editor_core::EditorState::new();
    let r = tools_list_response(
        "3",
        &state,
        false,
        tool_profile::McpAccessProfile::UNRESTRICTED,
    );
    // The production catalog excludes debug tools. Exact-count
    // assertion: any tool added without updating this test trips
    // the count first. Codex stop-gate: previous `contains`-only
    // checks would have silently passed if a new tool slipped into
    // TOOL_SCHEMAS without being added to the list below.
    assert_eq!(
        TOOL_SCHEMAS.len(),
        131,
        "tools/list catalog count must match the registered tools — add the new tool to this test"
    );
    // Production catalog excludes debug tools (we removed the
    // env var above to ensure deterministic gate-off behaviour).
    assert!(
        !r.contains("debug_validation_report"),
        "production tools/list must not advertise the debug tool: {r}"
    );
    #[cfg(not(feature = "mcp-debug-tools"))]
    {
        let r_forced_debug = tools_list_response(
            "3",
            &state,
            true,
            tool_profile::McpAccessProfile::UNRESTRICTED,
        );
        for name in [
            "debug_validation_report",
            "debug_logs_tail",
            "debug_screenshot",
        ] {
            assert!(
                !r_forced_debug.contains(name),
                "formal release catalog must exclude {name} even if debug listing is requested: {r_forced_debug}"
            );
        }
    }
    // UIKit element tools are appended dynamically — one per
    // built-in starter-kit component (6) — and ride alongside
    // the static schemas in the tools/list response.
    assert_eq!(
        op_mcp::element_tools::element_tool_schemas(&state).len(),
        37,
        "builtin kits ship 37 canonical element tools (6 starter + 31 shadcn)"
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
        "open_document",
        "save_document",
        "get_selection",
        "get_node",
        "list_pages",
        "list_variables",
        "get_variables",
        "upsert_variables",
        "upsert_component",
        "upsert_screen",
        "conversion_status",
        "lint_document",
        "save_theme_preset",
        "load_theme_preset",
        "list_theme_presets",
        "get_design_md",
        "set_design_md",
        "export_design_md",
        "get_style_guide_tags",
        "get_style_guide",
        "get_guidelines",
        "spawn_agents",
        "ToolSearch",
        "get_screenshot",
        "export_item",
        "export_nodes",
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
        "get_editor_state",
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
        "set_node_stroke_side_width",
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
        "import_html",
        "import_html_url",
        "import_web_snapshot",
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
        "apply_design_system",
        "create_variable",
        "delete_variable",
        "rename_variable",
        "design_skeleton",
        "design_content",
        "design_refine",
        "finalize_design",
        "enrich_images",
    ] {
        assert!(r.contains(name), "tools/list must include {name}: {r}");
    }

    // Gate open (debug_enabled = true) — internal debug builds can opt in
    // to the debug tools catalog.
    let r_debug = tools_list_response(
        "3",
        &state,
        true,
        tool_profile::McpAccessProfile::UNRESTRICTED,
    );
    #[cfg(feature = "mcp-debug-tools")]
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
    #[cfg(not(feature = "mcp-debug-tools"))]
    assert!(
        !r_debug.contains("debug_validation_report"),
        "default release feature set must not include debug tools: {r_debug}"
    );
}

#[test]
fn tools_list_design_content_schema_advertises_ts_layered_args() {
    let state = op_editor_core::EditorState::new();
    let response: serde_json::Value = serde_json::from_str(&tools_list_response(
        "3",
        &state,
        false,
        tool_profile::McpAccessProfile::UNRESTRICTED,
    ))
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
    let state = op_editor_core::EditorState::new();
    let response: serde_json::Value = serde_json::from_str(&tools_list_response(
        "3",
        &state,
        false,
        tool_profile::McpAccessProfile::UNRESTRICTED,
    ))
    .expect("tools/list response should be JSON");
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools/list result should contain tools");

    for (tool_name, expected) in [
        ("save_document", vec!["filePath", "sourceFilePath"]),
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
            "import_html",
            vec!["filePath", "htmlPath", "parent", "pageId"],
        ),
        (
            "import_html_url",
            vec!["filePath", "url", "parent", "pageId"],
        ),
        (
            "import_web_snapshot",
            vec!["filePath", "snapshot", "snapshotPath", "parent", "pageId"],
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
    let response = process_message_with_applier(&mut state, line, |_, _, _| false)
        .expect("dispatch")
        .expect("response");
    assert!(response.contains(r#""id":9"#), "{response}");
    let result = crate::mcp_serve::tool_text(&response);
    assert!(result.contains(r#""x":"240""#), "{result}");
    assert!(result.contains(r#""y":"20""#), "{result}");
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

#[path = "tests_transport.rs"]
mod transport;

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
