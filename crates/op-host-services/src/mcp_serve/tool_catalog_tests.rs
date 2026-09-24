//! Tests for catalog parsing and the catalog's interaction with the
//! deployment/scope profile.

use super::super::schemas::TOOL_SCHEMAS;
use super::super::tool_profile::{
    profile_for, schema_name, McpAccessProfile, McpScopes, ToolRefusal,
};
use super::*;

fn strings(items: &[&str]) -> impl Iterator<Item = String> {
    items
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .into_iter()
}

#[test]
fn every_lean_tool_is_a_classified_catalog_tool() {
    for tool in LEAN_TOOLS {
        assert!(
            TOOL_SCHEMAS
                .iter()
                .any(|schema| schema_name(schema).as_deref() == Some(*tool)),
            "{tool} must exist in the full catalog the lean one delegates to"
        );
        assert!(profile_for(tool).is_some(), "{tool} must be classified");
    }
    assert_eq!(LEAN_TOOLS.len(), 6);
}

#[test]
fn catalog_names_parse_case_insensitively() {
    assert_eq!("lean".parse::<McpToolCatalog>(), Ok(McpToolCatalog::Lean));
    assert_eq!(" LEAN ".parse::<McpToolCatalog>(), Ok(McpToolCatalog::Lean));
    assert_eq!("full".parse::<McpToolCatalog>(), Ok(McpToolCatalog::Full));
    assert_eq!(
        "default".parse::<McpToolCatalog>(),
        Ok(McpToolCatalog::Full)
    );
    let error = "tiny".parse::<McpToolCatalog>().unwrap_err();
    assert_eq!(
        error.to_string(),
        r#"unknown MCP tool profile "tiny" from --mcp-profile; expected one of: full, lean"#
    );
}

#[test]
fn the_flag_wins_over_the_environment() {
    assert_eq!(resolve_catalog(None, None), Ok(McpToolCatalog::Full));
    assert_eq!(resolve_catalog(None, Some("  ")), Ok(McpToolCatalog::Full));
    assert_eq!(
        resolve_catalog(None, Some("lean")),
        Ok(McpToolCatalog::Lean)
    );
    assert_eq!(
        resolve_catalog(Some("full"), Some("lean")),
        Ok(McpToolCatalog::Full)
    );
    assert_eq!(
        resolve_catalog(Some("lean"), None),
        Ok(McpToolCatalog::Lean)
    );
    let env_error = resolve_catalog(None, Some("mini")).unwrap_err();
    assert!(
        env_error.to_string().contains(MCP_PROFILE_ENV),
        "{env_error}"
    );
}

#[test]
fn the_profile_flag_is_extracted_from_anywhere_in_argv() {
    let (positional, flag) =
        extract_profile_flag(strings(&["doc.op", "--mcp-profile", "lean"])).unwrap();
    assert_eq!(positional, ["doc.op"]);
    assert_eq!(flag.as_deref(), Some("lean"));

    let (positional, flag) =
        extract_profile_flag(strings(&["--mcp-profile=lean", "3100", "doc.op"])).unwrap();
    assert_eq!(positional, ["3100", "doc.op"]);
    assert_eq!(flag.as_deref(), Some("lean"));

    let (positional, flag) = extract_profile_flag(strings(&["doc.op"])).unwrap();
    assert_eq!(positional, ["doc.op"]);
    assert_eq!(flag, None);

    assert_eq!(
        extract_profile_flag(strings(&["doc.op", "--mcp-profile"])),
        Err(McpProfileError::MissingValue)
    );
    assert_eq!(
        extract_profile_flag(strings(&["--mcp-profile=", "doc.op"])),
        Err(McpProfileError::MissingValue)
    );
}

#[test]
fn http_paths_select_a_catalog() {
    assert_eq!(
        McpToolCatalog::for_http_path("/mcp"),
        Some(McpToolCatalog::Full)
    );
    assert_eq!(
        McpToolCatalog::for_http_path("/"),
        Some(McpToolCatalog::Full)
    );
    assert_eq!(
        McpToolCatalog::for_http_path("/mcp/lean"),
        Some(McpToolCatalog::Lean)
    );
    assert_eq!(McpToolCatalog::for_http_path("/mcp/other"), None);
    assert_eq!(McpToolCatalog::Lean.http_path(), LEAN_MCP_PATH);
}

#[test]
fn the_lean_catalog_narrows_listing_and_calls() {
    let lean = McpAccessProfile::LEAN;
    assert!(lean.lists("batch_design"));
    assert!(!lean.lists("delete_node"));
    assert!(!lean.lists("insert_btn_primary"), "kit tools stay out");
    assert_eq!(lean.refuse("batch_design"), None);
    assert_eq!(
        lean.refuse("delete_node"),
        Some(ToolRefusal::NotInCatalog(McpToolCatalog::Lean))
    );
    assert_eq!(McpAccessProfile::UNRESTRICTED.refuse("delete_node"), None);
}

#[test]
fn the_catalog_never_widens_deployment_or_scope_decisions() {
    // A read-only credential on the lean catalog still cannot write.
    let read_only =
        McpAccessProfile::online(McpScopes::READ_ONLY).with_catalog(McpToolCatalog::Lean);
    assert_eq!(
        read_only.refuse("batch_design"),
        Some(ToolRefusal::ScopeInsufficient)
    );
    assert_eq!(read_only.refuse("get_editor_state"), None);
    // An online deployment on the lean catalog still hides host tools, and
    // the catalog refusal (the more useful hint) ranks first.
    assert_eq!(
        read_only.refuse("save_document"),
        Some(ToolRefusal::NotInCatalog(McpToolCatalog::Lean))
    );
}

#[test]
fn the_refusal_message_names_the_way_forward() {
    let message = ToolRefusal::NotInCatalog(McpToolCatalog::Lean).message("delete_node");
    assert!(message.starts_with("tool-not-in-profile: the tool 'delete_node'"));
    for tool in LEAN_TOOLS {
        assert!(message.contains(tool), "{message}");
    }
    assert!(message.contains("/mcp"), "{message}");
}
