//! Access and deployment classifications for the MCP catalog.

use super::{ToolAccess, ToolProfile, ToolSurface};

/// Every tool in `schemas::TOOL_SCHEMAS`, classified.
///
/// Kept in the same alphabetical order as the catalog so the two are
/// diffable side by side.
pub const TOOL_PROFILES: &[ToolProfile] = &[
    ToolProfile::new("ToolSearch", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("add_node_effect", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("add_comment", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("add_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("get_comment", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("list_comments", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("reply_comment", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_comment_status",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("align_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "apply_design_system",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("batch_design", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("create_generator", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "update_generator_params",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("get_generator", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("detach_generator", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("batch_get", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "brand_extract",
        ToolAccess::Write,
        // Fetches a caller-chosen website (or reads a caller-chosen image
        // path) from the daemon host; the network is the wider surface.
        ToolSurface::OutboundNetwork,
    ),
    ToolProfile::new("clear_selection", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "codegen_assemble",
        ToolAccess::Read,
        ToolSurface::ProcessGlobal,
    ),
    ToolProfile::new(
        "codegen_clean",
        ToolAccess::Read,
        ToolSurface::ProcessGlobal,
    ),
    ToolProfile::new("codegen_export", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("codegen_plan", ToolAccess::Read, ToolSurface::ProcessGlobal),
    ToolProfile::new(
        "codegen_submit_chunk",
        ToolAccess::Read,
        ToolSurface::ProcessGlobal,
    ),
    ToolProfile::new("conversion_status", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("copy_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("copy_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("count_nodes", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("create_component", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("create_variable", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("cut_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "cycle_active_axis_value",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "debug_logs_tail",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "debug_screenshot",
        ToolAccess::Read,
        ToolSurface::HostDiagnostics,
    ),
    ToolProfile::new(
        "debug_validation_report",
        ToolAccess::Read,
        ToolSurface::HostDiagnostics,
    ),
    ToolProfile::new("delete_component", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("delete_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("annotate_image", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("delete_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("delete_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("delete_variable", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("design_content", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("design_refine", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("design_skeleton", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "enrich_images",
        ToolAccess::Write,
        // Stock-photo search dials the product-constant Openverse / Wikimedia
        // hosts from the daemon — outbound network, like import_html_url.
        ToolSurface::OutboundNetwork,
    ),
    ToolProfile::new("finalize_design", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "run_design_agent",
        ToolAccess::Write,
        // The design loop dials the operator-configured LLM endpoint from
        // the daemon — outbound network, like enrich_images.
        ToolSurface::OutboundNetwork,
    ),
    ToolProfile::new("duplicate_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "duplicate_selected",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("export_design_md", ToolAccess::Read, ToolSurface::InMemory),
    // Writes a deck file at a caller-chosen path, so it is a local-filesystem
    // surface rather than an in-memory one: a hosted tenant must not be able
    // to place bytes anywhere on the daemon host.
    // Writes a directory of images at a caller-chosen path.
    ToolProfile::new(
        "export_frames",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("get_deck_boards", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "export_deck",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    // Reads the user's imported DESIGN.md files off the daemon host, not
    // just the embedded corpus, so it is not an in-memory surface.
    ToolProfile::new(
        "list_style_guides",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    // Mixed surface: every profile may list the immutable shipped catalogue;
    // registry construction injects whether the local-only, process-global
    // saved half may be included. The profile-aware snapshot is what keeps the
    // online result in-memory instead of hiding the whole discovery tool.
    ToolProfile::new(
        "list_scene_templates",
        ToolAccess::Read,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "use_scene_template",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("export_item", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("export_nodes", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("find_empty_space", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("find_node_by_name", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_active_theme", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_canvas_bounds", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_component", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "get_design_agent_prompt",
        ToolAccess::Read,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("get_design_md", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_design_prompt", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "get_design_quality",
        ToolAccess::Read,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("get_document_info", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_editor_state", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_guidelines", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_history_depth", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_node", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_node_children", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_node_parent", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_screenshot", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_selection", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_selection_set", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_style_guide", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "get_style_guide_tags",
        ToolAccess::Read,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("get_variables", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_viewport", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("group_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "import_html",
        ToolAccess::Write,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "import_html_url",
        ToolAccess::Write,
        ToolSurface::OutboundNetwork,
    ),
    ToolProfile::new(
        "import_svg",
        ToolAccess::Write,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "import_web_snapshot",
        ToolAccess::Write,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("insert_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "instantiate_component",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("lint_document", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("list_components", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("list_node_kinds", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("list_pages", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "list_theme_presets",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("list_ui_kits", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("list_variables", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "load_theme_preset",
        ToolAccess::Write,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("move_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("nudge_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "open_document",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("paste_clipboard", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("read_nodes", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("redo", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "remove_node_effect",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("remove_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("rename_component", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("rename_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("rename_variable", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("reorder_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("reorder_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "replace_all_matching_properties",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("replace_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "save_document",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "save_theme_preset",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "search_all_unique_properties",
        ToolAccess::Read,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_active_axis_value",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_active_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_active_tool", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_design_md", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_ellipse_arc", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_node_collapsed",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_corner_radius",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_fill_hex",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_node_flip", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_node_font_size",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_font_weight",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_node_hidden", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_node_locked", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_node_name", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_node_rotation",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_stroke_hex",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_stroke_side_width",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_stroke_width",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_node_text", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_selection", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_selection_set",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_themes", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_variable_boolean",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_variable_color",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_variable_number",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_variable_string",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_variables", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_viewport", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("snapshot_layout", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("spawn_agents", ToolAccess::Read, ToolSurface::HostProcess),
    ToolProfile::new(
        "toggle_node_selection",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("undo", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("ungroup_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("update_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("upsert_component", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("upsert_screen", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("upsert_variables", ToolAccess::Write, ToolSurface::InMemory),
];
