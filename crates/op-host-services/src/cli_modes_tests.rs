//! Argv tests for the MCP server modes' tool-catalog selection. The
//! environment value is passed in, so nothing here mutates process env.

use super::*;

fn argv(items: &[&str]) -> impl Iterator<Item = String> {
    items
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .into_iter()
}

#[test]
fn mcp_args_default_to_the_full_catalog() {
    let (positional, catalog) = parse_mcp_mode_args(argv(&["doc.op"]), None).unwrap();
    assert_eq!(positional, ["doc.op"]);
    assert_eq!(catalog, McpToolCatalog::Full);
}

#[test]
fn mcp_args_accept_the_profile_flag_after_the_path() {
    let (positional, catalog) =
        parse_mcp_mode_args(argv(&["doc.op", "--mcp-profile", "lean"]), None).unwrap();
    assert_eq!(positional, ["doc.op"]);
    assert_eq!(catalog, McpToolCatalog::Lean);
}

#[test]
fn mcp_http_args_accept_the_equals_form_before_the_port() {
    let (positional, catalog) =
        parse_mcp_mode_args(argv(&["--mcp-profile=lean", "3100", "doc.op"]), None).unwrap();
    assert_eq!(positional, ["3100", "doc.op"]);
    assert_eq!(catalog, McpToolCatalog::Lean);
}

#[test]
fn the_environment_selects_lean_and_the_flag_overrides_it() {
    let (_, from_env) = parse_mcp_mode_args(argv(&["doc.op"]), Some("lean".into())).unwrap();
    assert_eq!(from_env, McpToolCatalog::Lean);
    let (_, overridden) = parse_mcp_mode_args(
        argv(&["doc.op", "--mcp-profile", "full"]),
        Some("lean".into()),
    )
    .unwrap();
    assert_eq!(overridden, McpToolCatalog::Full);
}

#[test]
fn a_bad_profile_is_a_usage_error_before_any_server_starts() {
    assert!(parse_mcp_mode_args(argv(&["doc.op", "--mcp-profile", "tiny"]), None).is_err());
    assert!(parse_mcp_mode_args(argv(&["doc.op"]), Some("tiny".into())).is_err());
    // Exit code 2 = malformed invocation; returning at all proves no server
    // was started (both would block serving stdin / a socket).
    assert_eq!(
        run_cli_mode(
            "op-test",
            "--mcp",
            argv(&["doc.op", "--mcp-profile", "tiny"])
        ),
        Some(2)
    );
    assert_eq!(
        run_cli_mode(
            "op-test",
            "--mcp-http",
            argv(&["3100", "doc.op", "--mcp-profile"])
        ),
        Some(2)
    );
}
