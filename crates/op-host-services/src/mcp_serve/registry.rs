//! Per-request MCP tool-registry construction.
//!
//! Each snapshot detaches the tool from the borrowed editor state before the
//! caller applies a command. A direct tool call narrows construction to that
//! tool; discovery builds the complete registry.

use super::*;

pub(super) fn rebuild_registry(
    doc: &EditorState,
    requested_tool: Option<&str>,
    profile: tool_profile::McpAccessProfile,
) -> ToolRegistry {
    let mut r = ToolRegistry::default();

    macro_rules! register_tool {
        ($name:literal, $tool:expr) => {
            if should_register(requested_tool, $name) {
                r.register(Box::new($tool));
            }
        };
    }

    if requested_tool
        .map(|name| name.starts_with("add_"))
        .unwrap_or(true)
    {
        for tool in op_mcp::element_tools::insert_kit_component_tools(doc) {
            if should_register(requested_tool, tool.name()) {
                r.register(Box::new(tool));
            }
        }
    }
    register_tool!("open_document", open_document_snapshot(doc));
    register_tool!("save_document", save_document_snapshot(doc));
    register_tool!("get_document_info", document_info_snapshot(doc));
    register_tool!("get_selection", selection_snapshot(doc));
    register_tool!("get_node", get_node_snapshot(doc));
    register_tool!("list_pages", list_pages_snapshot(doc));
    register_tool!("list_variables", list_variables_snapshot(doc));
    register_tool!("get_variables", get_variables_snapshot(doc));
    register_tool!("upsert_variables", upsert_variables_snapshot());
    register_tool!("upsert_component", upsert_component_snapshot());
    register_tool!("upsert_screen", upsert_screen_snapshot());
    register_tool!("conversion_status", conversion_status_snapshot(doc));
    register_tool!("lint_document", lint_document_snapshot(doc));
    register_tool!("save_theme_preset", save_theme_preset_snapshot(doc));
    register_tool!("load_theme_preset", load_theme_preset_snapshot());
    register_tool!("list_theme_presets", list_theme_presets_snapshot());
    register_tool!("get_design_md", get_design_md_snapshot(doc));
    register_tool!("set_design_md", set_design_md_snapshot(doc));
    register_tool!("export_design_md", export_design_md_snapshot(doc));
    register_tool!("get_style_guide_tags", get_style_guide_tags_snapshot());
    register_tool!("get_style_guide", get_style_guide_snapshot());
    register_tool!("list_style_guides", list_style_guides_snapshot());
    register_tool!("get_guidelines", get_guidelines_snapshot());
    // Phase 0: always register spawn_agents (validates + returns request result).
    // Actual parallel execution is deferred to Phase 3 (Task 3.1).
    register_tool!("spawn_agents", spawn_agents_snapshot());
    register_tool!(
        "ToolSearch",
        tool_search_snapshot(tool_profile::tool_search_schemas(profile))
    );
    register_tool!("get_screenshot", get_screenshot_snapshot(doc));
    register_tool!("export_item", export_item_snapshot(doc));
    register_tool!("export_nodes", export_nodes_snapshot(doc));
    register_tool!("export_deck", export_deck_snapshot(doc));
    register_tool!("export_frames", export_frames_snapshot(doc));
    register_tool!("get_deck_boards", get_deck_boards_snapshot(doc));
    register_tool!(
        "list_scene_templates",
        list_scene_templates_snapshot(profile.includes_user_scene_templates())
    );
    register_tool!(
        "use_scene_template",
        use_scene_template_snapshot(profile.includes_user_scene_templates())
    );
    register_tool!("get_active_theme", get_active_theme_snapshot(doc));
    register_tool!("list_components", list_components_snapshot(doc));
    register_tool!("get_component", get_component_snapshot(doc));
    register_tool!("batch_get", batch_get_snapshot(doc));
    register_tool!("read_nodes", read_nodes_snapshot(doc));
    register_tool!("codegen_plan", codegen_plan_snapshot(doc));
    register_tool!("codegen_submit_chunk", codegen_submit_chunk_snapshot());
    register_tool!("codegen_assemble", codegen_assemble_snapshot());
    register_tool!("codegen_clean", codegen_clean_snapshot());
    register_tool!(
        "search_all_unique_properties",
        search_all_unique_properties_snapshot(doc)
    );
    register_tool!(
        "replace_all_matching_properties",
        replace_all_matching_properties_snapshot(doc)
    );
    register_tool!("snapshot_layout", snapshot_layout_snapshot(doc));
    register_tool!("find_empty_space", find_empty_space_snapshot(doc));
    register_tool!("get_canvas_bounds", get_canvas_bounds_snapshot(doc));
    register_tool!("find_node_by_name", find_node_by_name_snapshot(doc));
    register_tool!("get_node_parent", get_node_parent_snapshot(doc));
    register_tool!("get_node_children", get_node_children_snapshot(doc));
    register_tool!("count_nodes", count_nodes_snapshot(doc));
    register_tool!("list_node_kinds", list_node_kinds_snapshot(doc));
    register_tool!("get_history_depth", get_history_depth_snapshot(doc));
    register_tool!("get_viewport", get_viewport_snapshot(doc));
    register_tool!("get_selection_set", get_selection_set_snapshot(doc));
    register_tool!("get_editor_state", get_editor_state_snapshot(doc));
    #[cfg(feature = "mcp-debug-tools")]
    if debug_tools_enabled() {
        register_tool!(
            "debug_validation_report",
            debug_validation_report_snapshot(doc)
        );
        register_tool!("debug_logs_tail", debug_logs_tail_snapshot());
        register_tool!("debug_screenshot", debug_screenshot_snapshot());
    }
    register_tool!("clear_selection", clear_selection_snapshot());
    register_tool!("set_selection", set_selection_snapshot());
    register_tool!("set_viewport", set_viewport_snapshot());
    register_tool!("set_node_hidden", set_node_hidden_snapshot());
    register_tool!("set_node_locked", set_node_locked_snapshot());
    register_tool!("set_node_collapsed", set_node_collapsed_snapshot());
    register_tool!("set_active_tool", set_active_tool_snapshot());
    register_tool!("undo", undo_snapshot());
    register_tool!("redo", redo_snapshot());
    register_tool!("duplicate_selected", duplicate_selected_snapshot());
    register_tool!("delete_selected", delete_selected_snapshot());
    register_tool!("nudge_selected", nudge_selected_snapshot());
    register_tool!("group_selected", group_selected_snapshot());
    register_tool!("ungroup_selected", ungroup_selected_snapshot());
    register_tool!("reorder_selected", reorder_selected_snapshot());
    register_tool!("set_node_rotation", set_node_rotation_snapshot());
    register_tool!("set_node_text", set_node_text_snapshot());
    register_tool!("set_node_corner_radius", set_node_corner_radius_snapshot());
    register_tool!("set_node_font_size", set_node_font_size_snapshot());
    register_tool!("set_node_font_weight", set_node_font_weight_snapshot());
    register_tool!("set_node_stroke_hex", set_node_stroke_hex_snapshot());
    register_tool!("set_node_stroke_width", set_node_stroke_width_snapshot());
    register_tool!(
        "set_node_stroke_side_width",
        set_node_stroke_side_width_snapshot()
    );
    register_tool!("align_selected", align_selected_snapshot());
    register_tool!("set_node_fill_hex", set_node_fill_hex_snapshot());
    register_tool!("set_node_flip", set_node_flip_snapshot());
    register_tool!("set_ellipse_arc", set_ellipse_arc_snapshot());
    register_tool!("add_node_effect", add_node_effect_snapshot());
    register_tool!("remove_node_effect", remove_node_effect_snapshot());
    register_tool!("set_node_name", set_node_name_snapshot());
    register_tool!("set_selection_set", set_selection_set_snapshot());
    register_tool!("toggle_node_selection", toggle_node_selection_snapshot());
    register_tool!(
        "cycle_active_axis_value",
        cycle_active_axis_value_snapshot(doc)
    );
    register_tool!("copy_selected", copy_selected_snapshot());
    register_tool!("cut_selected", cut_selected_snapshot());
    register_tool!("paste_clipboard", paste_clipboard_snapshot());
    register_tool!("set_variable_color", set_variable_color_snapshot(doc));
    register_tool!("set_active_axis_value", set_active_axis_value_snapshot(doc));
    register_tool!("insert_node", insert_node_snapshot());
    register_tool!("import_svg", import_svg_snapshot());
    register_tool!("import_html", import_html_snapshot());
    register_tool!("import_html_url", import_html_url_snapshot());
    register_tool!("import_web_snapshot", import_web_snapshot_tool());
    register_tool!("update_node", update_node_snapshot());
    register_tool!("delete_node", delete_node_snapshot());
    register_tool!("move_node", move_node_snapshot());
    register_tool!("copy_node", copy_node_snapshot());
    register_tool!("replace_node", replace_node_snapshot());
    register_tool!("batch_design", batch_design_snapshot(doc));
    register_tool!("get_design_prompt", get_design_prompt_snapshot(doc));
    register_tool!("design_skeleton", design_skeleton_snapshot());
    register_tool!("design_content", design_content_snapshot());
    register_tool!("design_refine", design_refine_snapshot(doc));
    register_tool!("finalize_design", finalize_design_snapshot(doc));
    register_tool!("enrich_images", enrich_images_snapshot(doc));
    register_tool!("set_variable_number", set_variable_number_snapshot(doc));
    register_tool!("set_variable_string", set_variable_string_snapshot(doc));
    register_tool!("set_variable_boolean", set_variable_boolean_snapshot(doc));
    register_tool!("set_variables", set_variables_snapshot());
    register_tool!("set_themes", set_themes_snapshot());
    register_tool!("apply_design_system", apply_design_system_snapshot());
    register_tool!("create_variable", create_variable_snapshot(doc));
    register_tool!("delete_variable", delete_variable_snapshot(doc));
    register_tool!("rename_variable", rename_variable_snapshot(doc));
    register_tool!("instantiate_component", instantiate_component_snapshot());
    register_tool!("create_component", create_component_snapshot());
    register_tool!("delete_component", delete_component_snapshot());
    register_tool!("rename_component", rename_component_snapshot());
    register_tool!("set_active_page", set_active_page_snapshot());
    register_tool!("add_page", add_page_snapshot());
    register_tool!("rename_page", rename_page_snapshot(doc));
    register_tool!("delete_page", delete_page_snapshot(doc));
    register_tool!("remove_page", remove_page_snapshot(doc));
    register_tool!("duplicate_page", duplicate_page_snapshot(doc));
    register_tool!("reorder_page", reorder_page_snapshot(doc));
    r
}

fn should_register(requested_tool: Option<&str>, tool_name: &str) -> bool {
    requested_tool
        .map(|requested| requested == tool_name)
        .unwrap_or(true)
}
