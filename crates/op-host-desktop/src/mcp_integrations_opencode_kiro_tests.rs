//! Focused tests for OpenCode and Kiro's native MCP config layouts.

use super::*;

fn temp_home(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "openpencil-mcp-current-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("create temp home");
    path
}

fn cli_index(cli: McpCli) -> usize {
    McpCli::ALL
        .iter()
        .position(|candidate| *candidate == cli)
        .expect("CLI is registered")
}

#[test]
fn opencode_uses_current_path_and_remote_schema() {
    let home = temp_home("opencode");
    let path = home.join(".config").join("opencode").join("opencode.json");
    fs::create_dir_all(path.parent().expect("parent")).expect("create config dir");
    fs::write(
        &path,
        r#"{"theme":"dark","mcp":{"other":{"type":"remote","url":"https://example.com/mcp"}}}"#,
    )
    .expect("seed config");

    let written = set_cli_enabled_at_home(McpCli::OpenCode, true, 4100, &home).expect("install");
    assert_eq!(written, path);
    let root = read_json_object(&path).expect("parse installed config");
    assert_eq!(
        root.get("mcp")
            .and_then(Value::as_object)
            .and_then(|mcp| mcp.get(SERVER_NAME)),
        Some(&serde_json::json!({
            "type": "remote",
            "url": "http://127.0.0.1:4100/mcp",
            "enabled": true,
        }))
    );
    assert_eq!(root.get("theme").and_then(Value::as_str), Some("dark"));
    assert!(root["mcp"].as_object().expect("mcp").contains_key("other"));
    assert!(detect_enabled_clis_at_home(&home)[cli_index(McpCli::OpenCode)]);

    set_cli_enabled_at_home(McpCli::OpenCode, false, 4100, &home).expect("uninstall");
    let root = read_json_object(&path).expect("parse uninstalled config");
    assert!(!root["mcp"]
        .as_object()
        .expect("mcp")
        .contains_key(SERVER_NAME));
    assert!(root["mcp"].as_object().expect("mcp").contains_key("other"));
    assert_eq!(root.get("theme").and_then(Value::as_str), Some("dark"));
    let _ = fs::remove_dir_all(home);
}

#[test]
fn kiro_uses_current_path_and_native_remote_schema() {
    let home = temp_home("kiro");
    let path = home.join(".kiro").join("settings").join("mcp.json");
    fs::create_dir_all(path.parent().expect("parent")).expect("create config dir");
    fs::write(
        &path,
        r#"{"mcpServers":{"other":{"command":"echo","args":["hi"]}}}"#,
    )
    .expect("seed config");

    let written = set_cli_enabled_at_home(McpCli::Kiro, true, 4200, &home).expect("install");
    assert_eq!(written, path);
    let root = read_json_object(&path).expect("parse installed config");
    let servers = root["mcpServers"].as_object().expect("mcpServers");
    assert_eq!(
        servers.get(SERVER_NAME),
        Some(&serde_json::json!({
            "url": "http://127.0.0.1:4200/mcp",
            "disabled": false,
        }))
    );
    assert!(servers.contains_key("other"));
    assert!(detect_enabled_clis_at_home(&home)[cli_index(McpCli::Kiro)]);

    set_cli_enabled_at_home(McpCli::Kiro, false, 4200, &home).expect("uninstall");
    let root = read_json_object(&path).expect("parse uninstalled config");
    let servers = root["mcpServers"].as_object().expect("mcpServers");
    assert!(!servers.contains_key(SERVER_NAME));
    assert!(servers.contains_key("other"));
    let _ = fs::remove_dir_all(home);
}

#[test]
fn detection_respects_each_clients_disabled_flag() {
    let home = temp_home("disabled");
    for (cli, value) in [
        (
            McpCli::OpenCode,
            serde_json::json!({"mcp":{"openpencil":{"enabled":false}}}),
        ),
        (
            McpCli::Kiro,
            serde_json::json!({"mcpServers":{"openpencil":{"disabled":true}}}),
        ),
    ] {
        let path = config_path(cli, &home, false);
        fs::create_dir_all(path.parent().expect("parent")).expect("create config dir");
        fs::write(&path, serde_json::to_vec(&value).expect("serialize")).expect("write config");
        assert!(!detect_enabled_clis_at_home(&home)[cli_index(cli)]);
    }
    let _ = fs::remove_dir_all(home);
}
