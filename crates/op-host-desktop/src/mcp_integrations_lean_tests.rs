//! The lean-profile endpoint across every integration format. All writes
//! go to a per-test temp home — never the user's real client configs.

use super::*;

fn temp_home(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("openpencil-mcp-lean-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("create temp home");
    path
}

const LEAN_URL: &str = "http://127.0.0.1:3100/mcp/lean";
const FULL_URL: &str = "http://127.0.0.1:3100/mcp";

#[test]
fn endpoint_url_selects_the_profile_path() {
    assert_eq!(endpoint_url(3100, false), FULL_URL);
    assert_eq!(endpoint_url(3100, true), LEAN_URL);
}

#[test]
fn every_cli_format_installs_the_lean_endpoint_and_switches_back() {
    let home = temp_home("formats");
    for cli in McpCli::ALL {
        let path = set_cli_profile_at_home(cli, true, 3100, true, &home)
            .unwrap_or_else(|e| panic!("{} lean install: {e}", cli.label()));
        let text = fs::read_to_string(&path).expect("config written");
        assert!(
            text.contains(LEAN_URL),
            "{} must point at the lean endpoint:\n{text}",
            cli.label()
        );
        assert!(
            cli_config_has_openpencil(cli, &path) || cli == McpCli::Antigravity,
            "{} stays detected as enabled",
            cli.label()
        );

        set_cli_profile_at_home(cli, true, 3100, false, &home)
            .unwrap_or_else(|e| panic!("{} full install: {e}", cli.label()));
        let text = fs::read_to_string(&path).expect("config rewritten");
        assert!(
            text.contains(FULL_URL) && !text.contains(LEAN_URL),
            "{} must switch back to the full catalog in place:\n{text}",
            cli.label()
        );
    }
    let _ = fs::remove_dir_all(home);
}

#[test]
fn the_full_catalog_wrapper_keeps_the_historic_endpoint() {
    let home = temp_home("wrapper");
    let path = set_cli_enabled_at_home(McpCli::ClaudeCode, true, 3100, &home).expect("install");
    let text = fs::read_to_string(path).expect("config");
    assert!(
        text.contains(FULL_URL) && !text.contains("/mcp/lean"),
        "{text}"
    );
    let _ = fs::remove_dir_all(home);
}
