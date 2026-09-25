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
        141,
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
        "get_design_agent_prompt",
        "get_design_quality",
        "spawn_agents",
        "ToolSearch",
        "get_screenshot",
        "export_item",
        "export_nodes",
        "get_active_theme",
        "list_components",
        "list_ui_kits",
        "get_component",
        "batch_get",
        "read_nodes",
        "codegen_plan",
        "codegen_submit_chunk",
        "codegen_assemble",
        "codegen_clean",
        "codegen_export",
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
        "brand_extract",
        "import_web_snapshot",
        "update_node",
        "delete_node",
        "move_node",
        "copy_node",
        "replace_node",
        "batch_design",
        "create_generator",
        "update_generator_params",
        "get_generator",
        "detach_generator",
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
        (
            "codegen_export",
            vec!["filePath", "framework", "nodeIds", "pageId"],
        ),
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

#[path = "tests_transport.rs"]
mod transport;

#[path = "tests_dispatch.rs"]
mod dispatch;
