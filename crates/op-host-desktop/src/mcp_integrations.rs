//! Terminal-side MCP client configuration for the desktop settings panel.
//!
//! The live server is owned by the GUI process. CLI integrations point
//! at that process over streamable HTTP so terminal agents can reach the
//! same canvas state the user is editing.

use std::fs;
use std::path::{Path, PathBuf};

use op_editor_core::agent_settings::McpCli;
use serde_json::{Map, Value};

use crate::mcp_config_io::{
    atomic_write, grok_config_has_openpencil, update_grok_config, FileSnapshot,
};

const SERVER_NAME: &str = "openpencil";
const ANTIGRAVITY_MCP_PERMISSION: &str = "mcp(openpencil/*)";

pub(crate) fn set_cli_enabled(cli: McpCli, enabled: bool, port: u16) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "home directory not available".to_string())?;
    if cli == McpCli::Antigravity {
        return set_antigravity_enabled_at_home(enabled, port, &home);
    }
    let path = config_path(cli, &home, true);
    set_cli_enabled_at_path(cli, enabled, port, path)
}

pub(crate) fn detect_enabled_clis() -> [bool; 8] {
    let Some(home) = dirs::home_dir() else {
        return [false; 8];
    };
    detect_enabled_clis_for_home(&home, true)
}

/// Like [`set_cli_enabled`] but against an explicit home dir and without
/// reading `CODEX_HOME` (`use_env = false`). Used by tests to redirect CLI
/// config writes to a temp dir WITHOUT mutating process-global env.
pub(crate) fn set_cli_enabled_at_home(
    cli: McpCli,
    enabled: bool,
    port: u16,
    home: &Path,
) -> Result<PathBuf, String> {
    if cli == McpCli::Antigravity {
        return set_antigravity_enabled_at_home(enabled, port, home);
    }
    let path = config_path(cli, home, false);
    set_cli_enabled_at_path(cli, enabled, port, path)
}

/// Like [`detect_enabled_clis`] but against an explicit home dir (no env).
pub(crate) fn detect_enabled_clis_at_home(home: &Path) -> [bool; 8] {
    detect_enabled_clis_for_home(home, false)
}

fn detect_enabled_clis_for_home(home: &Path, use_env: bool) -> [bool; 8] {
    let mut flags = [false; 8];
    for (idx, cli) in McpCli::ALL.iter().copied().enumerate() {
        flags[idx] = if cli == McpCli::Antigravity {
            antigravity_config_has_openpencil(&config_path(cli, home, use_env))
                && antigravity_permission_is_present(&antigravity_permissions_path(home))
        } else {
            let path = config_path(cli, home, use_env);
            cli_config_has_openpencil(cli, &path)
        };
    }
    flags
}

fn set_cli_enabled_at_path(
    cli: McpCli,
    enabled: bool,
    port: u16,
    path: PathBuf,
) -> Result<PathBuf, String> {
    match cli {
        McpCli::Codex => update_codex_config(&path, enabled, port)?,
        McpCli::GrokBuild => update_grok_config(&path, enabled, &endpoint(port))?,
        McpCli::Antigravity => {
            return Err("Antigravity configuration requires a home directory".into())
        }
        McpCli::ClaudeCode
        | McpCli::Gemini
        | McpCli::OpenCode
        | McpCli::Kiro
        | McpCli::GithubCopilot => update_json_config(&path, enabled, port)?,
    }
    Ok(path)
}

fn cli_config_has_openpencil(cli: McpCli, path: &Path) -> bool {
    match cli {
        McpCli::Codex => fs::read_to_string(path)
            .map(|text| codex_config_has_openpencil(&text))
            .unwrap_or(false),
        McpCli::GrokBuild => fs::read_to_string(path)
            .map(|text| grok_config_has_openpencil(&text))
            .unwrap_or(false),
        McpCli::Antigravity => antigravity_config_has_openpencil(path),
        McpCli::ClaudeCode
        | McpCli::Gemini
        | McpCli::OpenCode
        | McpCli::Kiro
        | McpCli::GithubCopilot => json_config_has_openpencil(path),
    }
}

fn config_path(cli: McpCli, home: &Path, use_env: bool) -> PathBuf {
    match cli {
        McpCli::ClaudeCode => home.join(".claude.json"),
        McpCli::Codex => {
            if use_env {
                std::env::var_os("CODEX_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".codex"))
                    .join("config.toml")
            } else {
                home.join(".codex").join("config.toml")
            }
        }
        McpCli::Gemini => home.join(".gemini").join("settings.json"),
        McpCli::OpenCode => home.join(".opencode").join("config.json"),
        McpCli::Kiro => home.join(".kiro").join("settings.json"),
        McpCli::GithubCopilot => home.join(".config").join("github-copilot").join("mcp.json"),
        McpCli::Antigravity => home.join(".gemini").join("config").join("mcp_config.json"),
        McpCli::GrokBuild => {
            if use_env {
                std::env::var_os("GROK_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".grok"))
                    .join("config.toml")
            } else {
                home.join(".grok").join("config.toml")
            }
        }
    }
}

fn set_antigravity_enabled_at_home(
    enabled: bool,
    port: u16,
    home: &Path,
) -> Result<PathBuf, String> {
    let config = config_path(McpCli::Antigravity, home, false);
    let permissions = antigravity_permissions_path(home);
    let config_snapshot = FileSnapshot::capture(&config)?;
    let permissions_snapshot = FileSnapshot::capture(&permissions)?;

    update_antigravity_config(&config, enabled, port)?;
    if let Err(error) = update_antigravity_permissions(&permissions, enabled) {
        let mut rollback_errors = Vec::new();
        if let Err(rollback_error) = config_snapshot.restore(&config) {
            rollback_errors.push(rollback_error);
        }
        if let Err(rollback_error) = permissions_snapshot.restore(&permissions) {
            rollback_errors.push(rollback_error);
        }
        return if rollback_errors.is_empty() {
            Err(error)
        } else {
            Err(format!(
                "{error}; rollback failed: {}",
                rollback_errors.join("; ")
            ))
        };
    }
    Ok(config)
}

fn antigravity_permissions_path(home: &Path) -> PathBuf {
    home.join(".gemini")
        .join("antigravity-cli")
        .join("settings.json")
}

fn update_antigravity_permissions(path: &Path, enabled: bool) -> Result<(), String> {
    if !enabled && !path.exists() {
        return Ok(());
    }
    let mut root = read_json_object(path)?;
    if enabled {
        let permissions = root
            .entry("permissions")
            .or_insert_with(|| Value::Object(Map::new()));
        let permissions = permissions
            .as_object_mut()
            .ok_or_else(|| "permissions is not an object".to_string())?;
        let allow = permissions
            .entry("allow")
            .or_insert_with(|| Value::Array(Vec::new()));
        let allow = allow
            .as_array_mut()
            .ok_or_else(|| "permissions.allow is not an array".to_string())?;
        allow.retain(|rule| rule.as_str() != Some(ANTIGRAVITY_MCP_PERMISSION));
        allow.push(Value::String(ANTIGRAVITY_MCP_PERMISSION.into()));
        if let Some(deny) = permissions.get_mut("deny").and_then(Value::as_array_mut) {
            deny.retain(|rule| rule.as_str() != Some(ANTIGRAVITY_MCP_PERMISSION));
        }
    } else if let Some(allow) = root
        .get_mut("permissions")
        .and_then(Value::as_object_mut)
        .and_then(|permissions| permissions.get_mut("allow"))
        .and_then(Value::as_array_mut)
    {
        allow.retain(|rule| rule.as_str() != Some(ANTIGRAVITY_MCP_PERMISSION));
    }
    write_json_object(path, &root)
}

fn antigravity_permission_is_present(path: &Path) -> bool {
    read_json_object(path)
        .ok()
        .and_then(|root| {
            let permissions = root.get("permissions").and_then(Value::as_object)?;
            let denied = permissions
                .get("deny")
                .and_then(Value::as_array)
                .is_some_and(|rules| {
                    rules
                        .iter()
                        .any(|rule| rule.as_str() == Some(ANTIGRAVITY_MCP_PERMISSION))
                });
            permissions
                .get("allow")
                .and_then(Value::as_array)
                .map(|allow| {
                    !denied
                        && allow
                            .iter()
                            .any(|rule| rule.as_str() == Some(ANTIGRAVITY_MCP_PERMISSION))
                })
        })
        .unwrap_or(false)
}

fn json_config_has_openpencil(path: &Path) -> bool {
    read_json_object(path)
        .ok()
        .and_then(|root| {
            root.get("mcpServers")
                .and_then(Value::as_object)
                .map(|servers| servers.contains_key(SERVER_NAME))
        })
        .unwrap_or(false)
}

fn antigravity_config_has_openpencil(path: &Path) -> bool {
    let Some(server) = read_json_object(path).ok().and_then(|root| {
        root.get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(SERVER_NAME))
            .and_then(Value::as_object)
            .cloned()
    }) else {
        return false;
    };
    if server.get("disabled").and_then(Value::as_bool) == Some(true) {
        return false;
    }
    let Some(server_url) = server.get("serverUrl").and_then(Value::as_str) else {
        return false;
    };
    reqwest::Url::parse(server_url)
        .map(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        .unwrap_or(false)
}

fn update_json_config(path: &Path, enabled: bool, port: u16) -> Result<(), String> {
    let mut root = read_json_object(path)?;
    if enabled {
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()));
        if !servers.is_object() {
            *servers = Value::Object(Map::new());
        }
        let Some(servers) = servers.as_object_mut() else {
            return Err("mcpServers is not an object".into());
        };
        servers.insert(
            SERVER_NAME.into(),
            serde_json::json!({
                "type": "http",
                "url": endpoint(port),
            }),
        );
    } else if let Some(servers) = root.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(SERVER_NAME);
        if servers.is_empty() {
            root.remove("mcpServers");
        }
    }
    write_json_object(path, &root)
}

fn update_antigravity_config(path: &Path, enabled: bool, port: u16) -> Result<(), String> {
    if !enabled && !path.exists() {
        return Ok(());
    }
    let mut root = read_json_object(path)?;
    if enabled {
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()));
        if !servers.is_object() {
            *servers = Value::Object(Map::new());
        }
        let servers = servers
            .as_object_mut()
            .ok_or_else(|| "mcpServers is not an object".to_string())?;
        let mut server = Map::new();
        server.insert("serverUrl".into(), Value::String(endpoint(port)));
        servers.insert(SERVER_NAME.into(), Value::Object(server));
    } else if let Some(servers) = root.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(SERVER_NAME);
        if servers.is_empty() {
            root.remove("mcpServers");
        }
    }
    write_json_object(path, &root)
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{} must contain a JSON object", path.display()))
}

fn write_json_object(path: &Path, root: &Map<String, Value>) -> Result<(), String> {
    let text = serde_json::to_string_pretty(root)
        .map_err(|e| format!("serialize {}: {e}", path.display()))?;
    atomic_write(path, format!("{text}\n").as_bytes())
}

fn update_codex_config(path: &Path, enabled: bool, port: u16) -> Result<(), String> {
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    let mut text = remove_codex_server_block(&existing);
    if enabled {
        let prefix = text.trim_end();
        text = String::from(prefix);
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str("[mcp_servers.openpencil]\n");
        text.push_str(&format!(
            "url = \"{}\"\n",
            toml_basic_string_escape(&endpoint(port))
        ));
    }
    atomic_write(path, text.as_bytes())
}

fn codex_config_has_openpencil(input: &str) -> bool {
    input.lines().map(str::trim).any(is_codex_openpencil_table)
}

fn remove_codex_server_block(input: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim();
        if is_codex_openpencil_table(trimmed) {
            skipping = true;
            continue;
        }
        if skipping && trimmed.starts_with('[') {
            skipping = false;
        }
        if !skipping {
            out.push_str(line);
        }
    }
    out
}

fn is_codex_openpencil_table(line: &str) -> bool {
    matches!(
        line,
        "[mcp_servers.openpencil]"
            | "[mcp_servers.\"openpencil\"]"
            | "[\"mcp_servers\".\"openpencil\"]"
    )
}

fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

fn toml_basic_string_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(name: &str) -> PathBuf {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("test");
        let safe_thread_name: String = thread_name
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                    ch
                } else {
                    '-'
                }
            })
            .collect();
        let path = std::env::temp_dir().join(format!(
            "openpencil-mcp-{name}-{}-{}",
            std::process::id(),
            safe_thread_name
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp home");
        path
    }

    fn seed_antigravity_permission(home: &Path) {
        let path = antigravity_permissions_path(home);
        fs::create_dir_all(path.parent().expect("parent")).expect("create permission dir");
        fs::write(
            path,
            format!(r#"{{"permissions":{{"allow":["{ANTIGRAVITY_MCP_PERMISSION}"]}}}}"#),
        )
        .expect("seed Antigravity permission");
    }

    #[test]
    fn explicit_home_grok_config_path_ignores_process_environment() {
        let home = Path::new("/test/home");
        assert_eq!(
            config_path(McpCli::GrokBuild, home, false),
            home.join(".grok").join("config.toml")
        );
    }

    #[test]
    fn mcp_json_config_install_and_uninstall_preserves_other_servers() {
        let home = temp_home("json");
        let path = home.join(".claude.json");
        fs::write(
            &path,
            r#"{"theme":"dark","mcpServers":{"other":{"type":"http","url":"http://x"}}}"#,
        )
        .expect("seed config");

        set_cli_enabled_at_home(McpCli::ClaudeCode, true, 3101, &home).expect("install");
        let text = fs::read_to_string(&path).expect("read installed");
        assert!(text.contains(r#""openpencil""#), "{text}");
        assert!(
            text.contains(r#""url": "http://127.0.0.1:3101/mcp""#),
            "{text}"
        );
        assert!(text.contains(r#""other""#), "{text}");

        set_cli_enabled_at_home(McpCli::ClaudeCode, false, 3101, &home).expect("uninstall");
        let text = fs::read_to_string(&path).expect("read uninstalled");
        assert!(!text.contains(r#""openpencil""#), "{text}");
        assert!(text.contains(r#""other""#), "{text}");

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_codex_config_replaces_existing_openpencil_block() {
        let home = temp_home("codex");
        let path = home.join(".codex").join("config.toml");
        fs::create_dir_all(path.parent().expect("parent")).expect("create codex dir");
        fs::write(
            &path,
            "model = \"gpt-5\"\n\n[mcp_servers.openpencil]\nurl = \"http://old\"\n\n[profiles.dev]\nmodel = \"gpt-5-codex\"\n",
        )
        .expect("seed config");

        set_cli_enabled_at_home(McpCli::Codex, true, 3200, &home).expect("install");
        let text = fs::read_to_string(&path).expect("read installed");
        assert_eq!(
            text.matches("[mcp_servers.openpencil]").count(),
            1,
            "{text}"
        );
        assert!(
            text.contains("url = \"http://127.0.0.1:3200/mcp\""),
            "{text}"
        );
        assert!(text.contains("[profiles.dev]"), "{text}");

        set_cli_enabled_at_home(McpCli::Codex, false, 3200, &home).expect("uninstall");
        let text = fs::read_to_string(&path).expect("read uninstalled");
        assert!(!text.contains("[mcp_servers.openpencil]"), "{text}");
        assert!(text.contains("model = \"gpt-5\""), "{text}");
        assert!(text.contains("[profiles.dev]"), "{text}");

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_antigravity_config_uses_current_schema_and_preserves_other_servers() {
        let home = temp_home("antigravity");
        let path = home.join(".gemini").join("config").join("mcp_config.json");
        fs::create_dir_all(path.parent().expect("parent")).expect("create config dir");
        fs::write(
            &path,
            r#"{"permissionMode":"request-review","mcpServers":{"other":{"serverUrl":"https://example.com/mcp"}}}"#,
        )
        .expect("seed config");

        let written =
            set_cli_enabled_at_home(McpCli::Antigravity, true, 3300, &home).expect("install");
        assert_eq!(written, path);
        let root = read_json_object(&path).expect("read installed config");
        let servers = root
            .get("mcpServers")
            .and_then(Value::as_object)
            .expect("mcpServers");
        let openpencil = servers
            .get(SERVER_NAME)
            .and_then(Value::as_object)
            .expect("openpencil server");
        assert_eq!(
            openpencil.get("serverUrl").and_then(Value::as_str),
            Some("http://127.0.0.1:3300/mcp")
        );
        assert!(!openpencil.contains_key("serverURL"));
        assert!(!openpencil.contains_key("url"));
        assert!(servers.contains_key("other"));
        assert_eq!(
            root.get("permissionMode").and_then(Value::as_str),
            Some("request-review")
        );
        set_cli_enabled_at_home(McpCli::Antigravity, false, 3300, &home).expect("uninstall");
        let root = read_json_object(&path).expect("read uninstalled config");
        let servers = root
            .get("mcpServers")
            .and_then(Value::as_object)
            .expect("other server remains");
        assert!(!servers.contains_key(SERVER_NAME));
        assert!(servers.contains_key("other"));
        assert_eq!(
            root.get("permissionMode").and_then(Value::as_str),
            Some("request-review")
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_antigravity_permission_is_narrow_idempotent_and_removable() {
        let home = temp_home("antigravity-permission");
        let settings = home
            .join(".gemini")
            .join("antigravity-cli")
            .join("settings.json");
        fs::create_dir_all(settings.parent().expect("parent")).expect("create settings dir");
        fs::write(
            &settings,
            r#"{
                "colorScheme":"dark",
                "permissions":{
                    "allow":["shell(git status)","mcp(other/*)"],
                    "deny":["shell(rm -rf *)","mcp(openpencil/*)"]
                }
            }"#,
        )
        .expect("seed settings");
        set_cli_enabled_at_home(McpCli::Antigravity, true, 3302, &home).expect("install");
        set_cli_enabled_at_home(McpCli::Antigravity, true, 3302, &home)
            .expect("idempotent install");
        let root = read_json_object(&settings).expect("read enabled settings");
        let permissions = root["permissions"].as_object().expect("permissions");
        let allow = permissions["allow"].as_array().expect("allow array");
        assert_eq!(
            allow
                .iter()
                .filter(|rule| rule.as_str() == Some(ANTIGRAVITY_MCP_PERMISSION))
                .count(),
            1
        );
        assert!(allow
            .iter()
            .any(|rule| rule.as_str() == Some("shell(git status)")));
        assert!(allow
            .iter()
            .any(|rule| rule.as_str() == Some("mcp(other/*)")));
        assert_eq!(
            permissions["deny"]
                .as_array()
                .and_then(|rules| rules[0].as_str()),
            Some("shell(rm -rf *)")
        );
        assert!(!permissions["deny"]
            .as_array()
            .expect("deny array")
            .iter()
            .any(|rule| rule.as_str() == Some(ANTIGRAVITY_MCP_PERMISSION)));
        assert_eq!(root["colorScheme"].as_str(), Some("dark"));
        set_cli_enabled_at_home(McpCli::Antigravity, false, 3302, &home).expect("uninstall");
        let root = read_json_object(&settings).expect("read disabled settings");
        let permissions = root["permissions"].as_object().expect("permissions");
        let allow = permissions["allow"].as_array().expect("allow array");
        assert!(!allow
            .iter()
            .any(|rule| rule.as_str() == Some(ANTIGRAVITY_MCP_PERMISSION)));
        assert!(allow
            .iter()
            .any(|rule| rule.as_str() == Some("shell(git status)")));
        assert!(allow
            .iter()
            .any(|rule| rule.as_str() == Some("mcp(other/*)")));
        assert_eq!(
            permissions["deny"]
                .as_array()
                .and_then(|rules| rules[0].as_str()),
            Some("shell(rm -rf *)")
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_antigravity_rolls_back_config_when_permission_update_fails() {
        let home = temp_home("antigravity-rollback");
        let config = config_path(McpCli::Antigravity, &home, false);
        let settings = antigravity_permissions_path(&home);
        fs::create_dir_all(config.parent().expect("config parent")).expect("create config dir");
        fs::create_dir_all(settings.parent().expect("settings parent"))
            .expect("create settings dir");
        let original_config =
            br#"{"mcpServers":{"other":{"serverUrl":"https://example.com/mcp"}}}"#;
        let invalid_settings = b"{ this is not valid JSON";
        fs::write(&config, original_config).expect("seed config");
        fs::write(&settings, invalid_settings).expect("seed invalid settings");
        let error = set_cli_enabled_at_home(McpCli::Antigravity, true, 3303, &home)
            .expect_err("permission parse must fail");
        assert!(error.contains("parse"), "{error}");
        assert_eq!(
            fs::read(&config).expect("read rolled back config"),
            original_config
        );
        assert_eq!(
            fs::read(&settings).expect("read original settings"),
            invalid_settings
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_grok_config_replaces_only_openpencil_tables() {
        let home = temp_home("grok");
        let path = home.join(".grok").join("config.toml");
        fs::create_dir_all(path.parent().expect("parent")).expect("create grok dir");
        fs::write(
            &path,
            "[cli]\ninstaller = \"internal\"\n\n[mcp_servers.other]\nurl = \"https://example.com/mcp\"\n\n[mcp_servers.openpencil]\nurl = \"http://old/mcp\"\n\n[mcp_servers.openpencil.headers]\nAuthorization = \"Bearer stale\"\n\n[model.custom]\nmodel = \"custom-id\"\n",
        )
        .expect("seed config");
        set_cli_enabled_at_home(McpCli::GrokBuild, true, 3400, &home).expect("install");
        let text = fs::read_to_string(&path).expect("read installed");
        assert_eq!(
            text.matches("[mcp_servers.openpencil]").count(),
            1,
            "{text}"
        );
        assert!(
            text.contains("url = \"http://127.0.0.1:3400/mcp\""),
            "{text}"
        );
        assert!(!text.contains("Bearer stale"), "{text}");
        assert!(text.contains("[mcp_servers.other]"), "{text}");
        assert!(text.contains("[model.custom]"), "{text}");

        set_cli_enabled_at_home(McpCli::GrokBuild, false, 3400, &home).expect("uninstall");
        let text = fs::read_to_string(&path).expect("read uninstalled");
        assert!(!text.contains("mcp_servers.openpencil"), "{text}");
        assert!(text.contains("[mcp_servers.other]"), "{text}");
        assert!(text.contains("[model.custom]"), "{text}");

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn detects_antigravity_and_grok_openpencil_servers() {
        let home = temp_home("new-cli-detect");
        let antigravity = home.join(".gemini").join("config").join("mcp_config.json");
        fs::create_dir_all(antigravity.parent().expect("parent")).expect("create agy dir");
        fs::write(
            antigravity,
            r#"{"mcpServers":{"openpencil":{"serverUrl":"http://127.0.0.1:3000/mcp"}}}"#,
        )
        .expect("seed Antigravity config");
        seed_antigravity_permission(&home);
        let grok = home.join(".grok").join("config.toml");
        fs::create_dir_all(grok.parent().expect("parent")).expect("create Grok dir");
        fs::write(
            grok,
            "[mcp_servers.openpencil]\nurl = \"http://127.0.0.1:3000/mcp\"\n",
        )
        .expect("seed Grok config");

        let flags = detect_enabled_clis_at_home(&home);
        let antigravity_idx = McpCli::ALL
            .iter()
            .position(|cli| *cli == McpCli::Antigravity)
            .expect("Antigravity index");
        let grok_idx = McpCli::ALL
            .iter()
            .position(|cli| *cli == McpCli::GrokBuild)
            .expect("Grok index");
        assert!(flags[antigravity_idx], "{flags:?}");
        assert!(flags[grok_idx], "{flags:?}");
        assert_eq!(flags.iter().filter(|enabled| **enabled).count(), 2);

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn antigravity_detection_requires_a_valid_server_url() {
        let home = temp_home("antigravity-invalid-detect");
        let path = home.join(".gemini").join("config").join("mcp_config.json");
        fs::create_dir_all(path.parent().expect("parent")).expect("create config dir");
        let antigravity_idx = McpCli::ALL
            .iter()
            .position(|cli| *cli == McpCli::Antigravity)
            .expect("Antigravity index");
        seed_antigravity_permission(&home);

        for config in [
            r#"{"mcpServers":{"openpencil":{"serverURL":"http://127.0.0.1:3000/mcp"}}}"#,
            r#"{"mcpServers":{"openpencil":{"url":"http://127.0.0.1:3000/mcp"}}}"#,
            r#"{"mcpServers":{"openpencil":{"serverUrl":"not-a-url"}}}"#,
            r#"{"mcpServers":{"openpencil":{"serverUrl":"http://127.0.0.1:3000/mcp","disabled":true}}}"#,
        ] {
            fs::write(&path, config).expect("write invalid config");
            assert!(!detect_enabled_clis_at_home(&home)[antigravity_idx]);
        }

        fs::write(
            &path,
            r#"{"mcpServers":{"openpencil":{"serverUrl":"http://127.0.0.1:3000/mcp"}}}"#,
        )
        .expect("write valid config");
        assert!(detect_enabled_clis_at_home(&home)[antigravity_idx]);

        fs::write(
            antigravity_permissions_path(&home),
            format!(r#"{{"permissions":{{"allow":["{ANTIGRAVITY_MCP_PERMISSION}"],"deny":["{ANTIGRAVITY_MCP_PERMISSION}"]}}}}"#),
        )
        .expect("write denied permission");
        assert!(!detect_enabled_clis_at_home(&home)[antigravity_idx]);

        fs::remove_file(antigravity_permissions_path(&home)).expect("remove permission");
        assert!(!detect_enabled_clis_at_home(&home)[antigravity_idx]);

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn detects_legacy_codex_openpencil_server_config() {
        let home = temp_home("codex-detect");
        let path = home.join(".codex").join("config.toml");
        fs::create_dir_all(path.parent().expect("parent")).expect("create codex dir");
        fs::write(
            &path,
            "model = \"gpt-5\"\n\n[mcp_servers.openpencil]\ncommand = \"/usr/local/bin/node\"\nargs = [\"/Applications/OpenPencil.app/Contents/Resources/mcp-server.cjs\"]\n",
        )
        .expect("seed legacy config");

        let flags = detect_enabled_clis_at_home(&home);

        let codex_idx = McpCli::ALL
            .iter()
            .position(|cli| *cli == McpCli::Codex)
            .expect("Codex CLI index");
        assert!(flags[codex_idx]);
        assert!(
            flags.iter().filter(|enabled| **enabled).count() == 1,
            "{flags:?}"
        );

        let _ = fs::remove_dir_all(home);
    }
}
