//! Terminal-side MCP client configuration for the desktop settings panel.
//!
//! The live server is owned by the GUI process. CLI integrations point
//! at that process over streamable HTTP so terminal agents can reach the
//! same canvas state the user is editing.

use std::fs;
use std::path::{Path, PathBuf};

use op_editor_core::agent_settings::McpCli;
use serde_json::{Map, Value};

const SERVER_NAME: &str = "openpencil";

pub(crate) fn set_cli_enabled(cli: McpCli, enabled: bool, port: u16) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "home directory not available".to_string())?;
    let path = config_path(cli, &home, true);
    set_cli_enabled_at_path(cli, enabled, port, path)
}

pub(crate) fn detect_enabled_clis() -> [bool; 6] {
    let Some(home) = dirs::home_dir() else {
        return [false; 6];
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
    let path = config_path(cli, home, false);
    set_cli_enabled_at_path(cli, enabled, port, path)
}

/// Like [`detect_enabled_clis`] but against an explicit home dir (no env).
pub(crate) fn detect_enabled_clis_at_home(home: &Path) -> [bool; 6] {
    detect_enabled_clis_for_home(home, false)
}

fn detect_enabled_clis_for_home(home: &Path, use_env: bool) -> [bool; 6] {
    let mut flags = [false; 6];
    for (idx, cli) in McpCli::ALL.iter().copied().enumerate() {
        let path = config_path(cli, home, use_env);
        flags[idx] = cli_config_has_openpencil(cli, &path);
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
    }
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(root)
        .map_err(|e| format!("serialize {}: {e}", path.display()))?;
    fs::write(path, format!("{text}\n")).map_err(|e| format!("write {}: {e}", path.display()))
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))
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
