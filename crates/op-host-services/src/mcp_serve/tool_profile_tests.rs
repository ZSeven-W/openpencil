//! Tests for the MCP capability profile.
//!
//! The parity tests are the load-bearing ones: they are what turns "someone
//! added a tool and forgot to classify it" from a silent security hole into a
//! build failure.

use super::*;

#[test]
fn every_catalog_tool_is_classified() {
    let missing: Vec<String> = catalog_tool_names()
        .into_iter()
        .filter(|name| profile_for(name).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "these tools are in TOOL_SCHEMAS but carry no capability profile, so a \
         deployment would fall back to a default instead of a decision: {missing:?}"
    );
}

#[test]
fn every_classified_tool_is_in_the_catalog() {
    let catalog = catalog_tool_names();
    let stale: Vec<&str> = TOOL_PROFILES
        .iter()
        .map(|profile| profile.name)
        // The debug tools are classified in every build but only present in
        // the catalog of a debug-tool build; keeping their classification
        // unconditional is what stops a feature flag from losing the deny.
        .filter(|name| !is_debug_only_tool(name))
        .filter(|name| !catalog.iter().any(|entry| entry == name))
        .collect();
    assert!(
        stale.is_empty(),
        "these profiles name tools the catalog no longer has: {stale:?}"
    );
}

#[test]
fn the_table_has_no_duplicate_entries() {
    let mut names: Vec<&str> = TOOL_PROFILES.iter().map(|profile| profile.name).collect();
    let before = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(before, names.len(), "duplicate profile entries");
}

#[test]
fn the_denied_set_is_exactly_the_reviewed_list() {
    // Pinned deliberately: widening this set is a product decision and
    // narrowing it is a security decision. Either way it should be a diff a
    // reviewer sees, not a side effect of editing a tool.
    let mut denied = denied_tool_names();
    denied.sort_unstable();
    assert_eq!(
        denied,
        vec![
            "codegen_assemble",
            "codegen_clean",
            "codegen_plan",
            "codegen_submit_chunk",
            "debug_logs_tail",
            "debug_screenshot",
            "debug_validation_report",
            // Stock-photo search dials public providers from the daemon host —
            // outbound network, denied online for the same reason
            // import_html_url is.
            "enrich_images",
            // Writes a deck file at a caller-chosen path — denied online for the
            // same reason save_document is.
            "export_deck",
            "export_frames",
            "import_html",
            "import_html_url",
            "import_svg",
            "import_web_snapshot",
            // Reads the host's imported DESIGN.md files, which carry no tenant
            // dimension — denied online for the same reason list_theme_presets is.
            "list_style_guides",
            "list_theme_presets",
            "load_theme_preset",
            "open_document",
            "save_document",
            "save_theme_preset",
            "spawn_agents",
        ]
    );
}

#[test]
fn the_unrestricted_profile_refuses_nothing_and_lists_everything() {
    let profile = McpAccessProfile::UNRESTRICTED;
    for name in catalog_tool_names() {
        assert!(profile.lists(&name), "{name} must stay listed");
        assert_eq!(profile.refuse(&name), None, "{name} must stay callable");
    }
}

#[test]
fn the_online_profile_refuses_every_unshareable_tool() {
    let profile = McpAccessProfile::online(McpScopes::FULL);
    for name in denied_tool_names() {
        assert!(!profile.lists(name), "{name} must not be advertised");
        assert_eq!(
            profile.refuse(name),
            Some(ToolRefusal::LocalResourceDenied),
            "{name} must be refused even when asked for by name"
        );
    }
}

#[test]
fn the_online_profile_still_serves_the_in_memory_catalog() {
    let profile = McpAccessProfile::online(McpScopes::FULL);
    for name in [
        "add_page",
        "insert_node",
        "get_node",
        "batch_design",
        "list_scene_templates",
        "undo",
        "use_scene_template",
    ] {
        assert!(profile.lists(name), "{name}");
        assert_eq!(profile.refuse(name), None, "{name}");
    }
}

#[test]
fn tool_search_catalog_uses_the_same_online_surface_filter() {
    let local = tool_search_schemas(McpAccessProfile::UNRESTRICTED);
    assert_eq!(
        local, TOOL_SCHEMAS,
        "local discovery keeps the full catalog"
    );

    let online = tool_search_schemas(McpAccessProfile::online(McpScopes::FULL));
    let names: Vec<String> = online
        .iter()
        .filter_map(|schema| schema_name(schema))
        .collect();
    assert!(names.iter().any(|name| name == "get_node"));
    for denied in denied_tool_names() {
        assert!(
            !names.iter().any(|name| name == denied),
            "{denied} must be hidden from ToolSearch as well as tools/list"
        );
    }
}

#[test]
fn online_scene_template_calls_keep_shipped_ids_but_refuse_user_ids() {
    let online = McpAccessProfile::online(McpScopes::FULL);
    assert!(!online.includes_user_scene_templates());
    assert!(McpAccessProfile::UNRESTRICTED.includes_user_scene_templates());
    let shipped =
        std::collections::BTreeMap::from([("templateId".to_string(), "slide-deck".to_string())]);
    assert_eq!(online.refuse_call("use_scene_template", &shipped), None);

    let user = std::collections::BTreeMap::from([(
        "templateId".to_string(),
        "  user:private-deck  ".to_string(),
    )]);
    assert_eq!(
        online.refuse_call("use_scene_template", &user),
        Some(ToolRefusal::UserTemplateDenied)
    );
    assert_eq!(
        McpAccessProfile::UNRESTRICTED.refuse_call("use_scene_template", &user),
        None,
        "the local single-user daemon keeps access to its own saved templates"
    );
}

#[test]
fn a_read_only_credential_may_read_but_not_write() {
    let profile = McpAccessProfile::online(McpScopes::READ_ONLY);
    for read_tool in ["get_node", "list_pages", "snapshot_layout", "lint_document"] {
        assert_eq!(profile.refuse(read_tool), None, "{read_tool}");
    }
    for write_tool in [
        "add_page",
        "insert_node",
        "delete_node",
        "undo",
        "batch_design",
    ] {
        assert_eq!(
            profile.refuse(write_tool),
            Some(ToolRefusal::ScopeInsufficient),
            "{write_tool}"
        );
    }
}

#[test]
fn a_read_only_credential_still_sees_the_write_tools_it_cannot_call() {
    // Hiding them would read as "this deployment has no write tools" rather
    // than "your token cannot use them", which is a worse diagnostic.
    let profile = McpAccessProfile::online(McpScopes::READ_ONLY);
    assert!(profile.lists("add_page"));
}

#[test]
fn a_denied_tool_reports_denial_rather_than_a_scope_problem() {
    // `save_document` is a document READ that writes the filesystem, so a
    // scope-first check would wave it through for a read-only token.
    let read_only = McpAccessProfile::online(McpScopes::READ_ONLY);
    assert_eq!(
        read_only.refuse("save_document"),
        Some(ToolRefusal::LocalResourceDenied),
        "a bigger token must not look like the fix"
    );
    assert_eq!(
        profile_for("save_document").map(|profile| profile.access),
        Some(ToolAccess::Read),
        "this is the trap the ordering exists for"
    );
}

#[test]
fn an_unclassified_tool_is_assumed_to_write() {
    // The dynamic `add_*` insert tools are generated per document and are
    // never in the static catalog. They are writes, and an unclassified name
    // must not be reachable by a read-only credential.
    let profile = McpAccessProfile::online(McpScopes::READ_ONLY);
    assert_eq!(
        profile.refuse("add_some_kit_component"),
        Some(ToolRefusal::ScopeInsufficient)
    );
    // …but it is still in-memory, so a full-scope online credential may use it.
    assert_eq!(
        McpAccessProfile::online(McpScopes::FULL).refuse("add_some_kit_component"),
        None
    );
}

#[test]
fn scopes_are_derived_from_the_hub_scope_list() {
    assert_eq!(
        McpScopes::from_scope_list(&["mcp:read"]),
        McpScopes::READ_ONLY
    );
    assert!(McpScopes::from_scope_list(&["mcp:read", "mcp:write"]).can_write());
    // Write without read is honoured as written rather than normalised.
    assert!(McpScopes::from_scope_list(&["mcp:write"]).can_write());
    assert!(!McpScopes::from_scope_list(&["mcp:write"]).allows(ToolAccess::Read));
    // Fail-closed: a token that names no mcp scope may do nothing. op-hub
    // MUST issue explicit `mcp:read` / `mcp:write` on any token intended to
    // drive the canvas — one without them is inert by design.
    assert_eq!(
        McpScopes::from_scope_list(&["billing:read"]),
        McpScopes::NONE
    );
    assert_eq!(McpScopes::from_scope_list::<&str>(&[]), McpScopes::NONE);
    // An unknown mcp scope does NOT grant anything.
    assert_eq!(
        McpScopes::from_scope_list(&["mcp:admin"]),
        McpScopes {
            read: false,
            write: false
        }
    );
}

#[test]
fn a_refusal_message_leads_with_its_machine_code() {
    let denied = ToolRefusal::LocalResourceDenied.message("save_document");
    assert!(denied.starts_with("tool-not-available:"), "{denied}");
    assert!(denied.contains("save_document"), "{denied}");

    let user_template = ToolRefusal::UserTemplateDenied.message("use_scene_template");
    assert!(
        user_template.starts_with("user-template-not-available:"),
        "{user_template}"
    );

    let scoped = ToolRefusal::ScopeInsufficient.message("add_page");
    assert!(scoped.starts_with("scope-insufficient:"), "{scoped}");
    assert!(scoped.contains("mcp:write"), "{scoped}");
}

#[test]
fn schema_names_are_read_out_of_the_catalog_entries() {
    assert_eq!(
        schema_name(r#"{"name":"get_node","description":"x"}"#).as_deref(),
        Some("get_node")
    );
    assert_eq!(schema_name("{}"), None);
}

#[test]
fn every_tool_that_writes_the_local_filesystem_is_denied() {
    // Independent of the pinned list above: these are the tools the audit
    // found reaching the host filesystem, asserted by surface rather than by
    // name so a reclassification cannot quietly re-expose one.
    for name in [
        "save_document",
        "save_theme_preset",
        "load_theme_preset",
        "list_theme_presets",
        "import_html",
        "import_svg",
        "import_web_snapshot",
        "debug_logs_tail",
        "open_document",
    ] {
        assert_eq!(
            profile_for(name).map(|profile| profile.surface),
            Some(ToolSurface::LocalFilesystem),
            "{name}"
        );
        assert!(!ToolSurface::LocalFilesystem.is_shareable());
    }
    assert_eq!(
        profile_for("import_html_url").map(|profile| profile.surface),
        Some(ToolSurface::OutboundNetwork)
    );
}
