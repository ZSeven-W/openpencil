//! Tests for the Cline CLI MCP integration: settings-path resolution, the
//! nested `streamableHttp` entry shape, ownership refusals, and detection.

use std::fs;
use std::path::{Path, PathBuf};

use op_editor_core::agent_settings::McpCli;
use serde_json::{json, Value};

use super::{cline_settings_path, ClineEnv};
use crate::mcp_config_error::McpConfigError;
use crate::mcp_integrations::{config_path, detect_enabled_clis_at_home, set_cli_enabled_at_home};

fn temp_home(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "openpencil-mcp-cline-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("create temp home");
    path
}

fn settings(home: &Path) -> PathBuf {
    home.join(".cline/data/settings/cline_mcp_settings.json")
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read settings")).expect("json")
}

fn seed(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().expect("parent")).expect("settings dir");
    fs::write(path, serde_json::to_string_pretty(value).expect("json")).expect("seed");
}

fn detected(home: &Path) -> bool {
    detect_enabled_clis_at_home(home)[McpCli::Cline.index()]
}

#[test]
fn cline_is_appended_without_shifting_existing_slots() {
    assert_eq!(McpCli::Cline.index(), 13);
    assert_eq!(McpCli::Dsh.index(), 12);
}

#[test]
fn settings_path_follows_cline_resolver_precedence() {
    let home = Path::new("/h");
    let default = ClineEnv::default();
    assert_eq!(cline_settings_path(home, &default), settings(home));
    assert_eq!(config_path(McpCli::Cline, home, false), settings(home));

    let cline_dir = ClineEnv {
        cline_dir: Some("/opt/cline".into()),
        ..ClineEnv::default()
    };
    assert_eq!(
        cline_settings_path(home, &cline_dir),
        PathBuf::from("/opt/cline/data/settings/cline_mcp_settings.json")
    );

    let data_dir = ClineEnv {
        data_dir: Some("/data".into()),
        ..cline_dir.clone()
    };
    assert_eq!(
        cline_settings_path(home, &data_dir),
        PathBuf::from("/data/settings/cline_mcp_settings.json")
    );

    let explicit = ClineEnv {
        mcp_settings_path: Some(" /x/mcp.json ".into()),
        ..data_dir.clone()
    };
    assert_eq!(
        cline_settings_path(home, &explicit),
        PathBuf::from("/x/mcp.json")
    );

    let blank = ClineEnv {
        mcp_settings_path: Some("  ".into()),
        data_dir: Some("".into()),
        cline_dir: Some(" ".into()),
    };
    assert_eq!(cline_settings_path(home, &blank), settings(home));
}

#[test]
fn enable_writes_nested_streamable_http_entry_and_detects_it() {
    let home = temp_home("enable");
    set_cli_enabled_at_home(McpCli::Cline, true, 3100, &home).expect("enable");

    assert_eq!(
        read(&settings(&home)),
        json!({
            "mcpServers": {
                "openpencil": {
                    "transport": { "type": "streamableHttp", "url": "http://127.0.0.1:3100/mcp" },
                    "disabled": false
                }
            }
        })
    );
    assert!(detected(&home));
    let _ = fs::remove_dir_all(home);
}

#[test]
fn enable_and_disable_preserve_other_servers_and_settings() {
    let home = temp_home("preserve");
    let path = settings(&home);
    let other = json!({ "command": "node", "args": ["server.js"] });
    seed(
        &path,
        &json!({ "mcpServers": { "other": other }, "extra": 1 }),
    );

    set_cli_enabled_at_home(McpCli::Cline, true, 3100, &home).expect("enable");
    set_cli_enabled_at_home(McpCli::Cline, false, 3100, &home).expect("disable");

    assert_eq!(
        read(&path),
        json!({ "mcpServers": { "other": other }, "extra": 1 })
    );
    assert!(!detected(&home));
    let _ = fs::remove_dir_all(home);
}

#[test]
fn port_change_rewrites_url_and_keeps_cline_owned_fields() {
    let home = temp_home("port");
    let path = settings(&home);
    // Legacy flat form written by an older Cline, plus a user-tuned timeout.
    seed(
        &path,
        &json!({ "mcpServers": { "openpencil": {
            "type": "streamableHttp", "url": "http://127.0.0.1:3000/mcp",
            "disabled": true, "timeout": 120
        }}}),
    );

    set_cli_enabled_at_home(McpCli::Cline, true, 4200, &home).expect("re-enable on new port");

    assert_eq!(
        read(&path)["mcpServers"]["openpencil"],
        json!({
            "transport": { "type": "streamableHttp", "url": "http://127.0.0.1:4200/mcp" },
            "disabled": false,
            "timeout": 120
        })
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn disabled_entry_is_not_detected() {
    let home = temp_home("disabled");
    seed(
        &settings(&home),
        &json!({ "mcpServers": { "openpencil": {
            "transport": { "type": "streamableHttp", "url": "http://127.0.0.1:3100/mcp" },
            "disabled": true
        }}}),
    );
    assert!(!detected(&home));
    let _ = fs::remove_dir_all(home);
}

#[test]
fn foreign_openpencil_entry_is_never_overwritten_or_deleted() {
    for (name, entry) in [
        (
            "remote",
            json!({ "transport": { "type": "streamableHttp", "url": "https://example.com/mcp" } }),
        ),
        (
            "stdio",
            json!({ "transport": { "type": "stdio", "command": "op", "args": ["mcp"] } }),
        ),
    ] {
        let home = temp_home(&format!("foreign-{name}"));
        let path = settings(&home);
        let original = json!({ "mcpServers": { "openpencil": entry } });
        seed(&path, &original);

        for enabled in [true, false] {
            let error = set_cli_enabled_at_home(McpCli::Cline, enabled, 3100, &home)
                .expect_err("foreign entries must be refused");
            assert!(
                matches!(error, McpConfigError::ClineForeignEntry { .. }),
                "{error:?}"
            );
            assert_eq!(read(&path), original, "refusal must not modify the file");
        }
        let _ = fs::remove_dir_all(home);
    }
}

#[test]
fn malformed_mcp_servers_is_refused_not_replaced() {
    let home = temp_home("malformed");
    let path = settings(&home);
    seed(&path, &json!({ "mcpServers": ["not", "an", "object"] }));

    let error = set_cli_enabled_at_home(McpCli::Cline, true, 3100, &home).expect_err("refuse");
    assert!(
        matches!(error, McpConfigError::McpServersNotAnObject),
        "{error:?}"
    );
    assert_eq!(
        read(&path),
        json!({ "mcpServers": ["not", "an", "object"] })
    );

    fs::write(&path, "{ not json").expect("seed invalid json");
    assert!(set_cli_enabled_at_home(McpCli::Cline, true, 3100, &home).is_err());
    assert_eq!(fs::read_to_string(&path).expect("read"), "{ not json");
    let _ = fs::remove_dir_all(home);
}

#[test]
fn disable_without_settings_file_creates_nothing() {
    let home = temp_home("absent");
    set_cli_enabled_at_home(McpCli::Cline, false, 3100, &home).expect("disable no-op");
    assert!(!settings(&home).exists());
    let _ = fs::remove_dir_all(home);
}
