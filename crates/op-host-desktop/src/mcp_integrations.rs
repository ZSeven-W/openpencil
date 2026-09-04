//! Terminal-side MCP client configuration for the desktop settings panel.
//!
//! The live server is owned by the GUI process. CLI integrations point
//! at that process over streamable HTTP so terminal agents can reach the
//! same canvas state the user is editing.

use std::fs;
use std::path::{Path, PathBuf};

use op_editor_core::agent_settings::McpCli;
use serde_json::{Map, Value};

use crate::mcp_config_error::McpConfigError;
use crate::mcp_config_io::{
    atomic_write, grok_config_has_openpencil, update_grok_config, FileSnapshot,
};
use crate::mcp_integrations_dsh::{dsh_config_has_openpencil, update_dsh_patch_config};

const SERVER_NAME: &str = "openpencil";
const ANTIGRAVITY_MCP_PERMISSION: &str = "mcp(openpencil/*)";

pub(crate) fn set_cli_enabled(
    cli: McpCli,
    enabled: bool,
    port: u16,
) -> Result<PathBuf, McpConfigError> {
    let home = dirs::home_dir().ok_or(McpConfigError::HomeDirUnavailable)?;
    if cli == McpCli::Antigravity {
        return set_antigravity_enabled_at_home(enabled, port, &home);
    }
    let path = config_path(cli, &home, true);
    set_cli_enabled_at_path(cli, enabled, port, path)
}

pub(crate) fn detect_enabled_clis() -> [bool; 13] {
    let Some(home) = dirs::home_dir() else {
        return [false; 13];
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
) -> Result<PathBuf, McpConfigError> {
    if cli == McpCli::Antigravity {
        return set_antigravity_enabled_at_home(enabled, port, home);
    }
    let path = config_path(cli, home, false);
    set_cli_enabled_at_path(cli, enabled, port, path)
}

/// Like [`detect_enabled_clis`] but against an explicit home dir (no env).
pub(crate) fn detect_enabled_clis_at_home(home: &Path) -> [bool; 13] {
    detect_enabled_clis_for_home(home, false)
}

fn detect_enabled_clis_for_home(home: &Path, use_env: bool) -> [bool; 13] {
    let mut flags = [false; 13];
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
) -> Result<PathBuf, McpConfigError> {
    match cli {
        McpCli::Codex => update_codex_config(&path, enabled, port)?,
        McpCli::GrokBuild => update_grok_config(&path, enabled, &endpoint(port))?,
        McpCli::Antigravity => return Err(McpConfigError::AntigravityNeedsHome),
        McpCli::OpenCode => update_opencode_config(&path, enabled, port)?,
        McpCli::QwenCode => update_mcp_servers_json(&path, enabled, qwen_server(port))?,
        McpCli::Kiro => update_mcp_servers_json(&path, enabled, kiro_server(port))?,
        McpCli::Kimi => update_mcp_servers_json(&path, enabled, kimi_server(port))?,
        McpCli::ZCode => update_zcode_config(&path, enabled, port)?,
        McpCli::Dsh => update_dsh_patch_config(&path, enabled, port)?,
        McpCli::ClaudeCode | McpCli::GithubCopilot | McpCli::GeminiCli | McpCli::Cursor => {
            update_json_config(&path, enabled, port)?
        }
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
        McpCli::OpenCode => opencode_config_has_openpencil(path),
        McpCli::Kiro => kiro_config_has_openpencil(path),
        McpCli::ZCode => zcode_config_has_openpencil(path),
        McpCli::Dsh => dsh_config_has_openpencil(path),
        // Every remaining CLI keys its servers off `mcpServers.openpencil`;
        // only the value shape differs, so presence is the same check.
        McpCli::ClaudeCode
        | McpCli::GithubCopilot
        | McpCli::GeminiCli
        | McpCli::QwenCode
        | McpCli::Cursor
        | McpCli::Kimi => json_config_has_openpencil(path),
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
        McpCli::OpenCode => home.join(".config").join("opencode").join("opencode.json"),
        McpCli::Kiro => home.join(".kiro").join("settings").join("mcp.json"),
        McpCli::GithubCopilot => home.join(".config").join("github-copilot").join("mcp.json"),
        // Antigravity and the Gemini CLI both live under `~/.gemini` but read
        // different files, so enabling one leaves the other untouched.
        McpCli::Antigravity => home.join(".gemini").join("config").join("mcp_config.json"),
        McpCli::GeminiCli => home.join(".gemini").join("settings.json"),
        McpCli::QwenCode => home.join(".qwen").join("settings.json"),
        // Shared with the Cursor editor rather than a separate CLI file — a
        // feature, not a collision: the editor picks the server up too.
        McpCli::Cursor => home.join(".cursor").join("mcp.json"),
        // kimi-code, which supersedes the pip `kimi-cli` (it ships a
        // `kimi migrate` for it). It resolves its data root from
        // `KIMI_CODE_HOME` before falling back to `~/.kimi-code`.
        McpCli::Kimi => {
            if use_env {
                std::env::var_os("KIMI_CODE_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".kimi-code"))
                    .join("mcp.json")
            } else {
                home.join(".kimi-code").join("mcp.json")
            }
        }
        // ZCode's own user-level config. It also reads the cross-agent
        // `~/.agents/mcp.json`, but that file is shared with every tool
        // implementing that convention, so writing it would reach beyond
        // this toggle; a dedicated "cross-agent standard" integration would
        // be the place for it.
        McpCli::ZCode => home.join(".zcode").join("cli").join("config.json"),
        // DeepSeek Harness reads its home-level patch layer — one file
        // that applies across all profiles, so a single write covers every
        // profile. `DSH_HOME` overrides the data root, mirroring Codex's
        // `CODEX_HOME` handling.
        McpCli::Dsh => {
            if use_env {
                std::env::var_os("DSH_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".dsh"))
                    .join("cordis.patch.yml")
            } else {
                home.join(".dsh").join("cordis.patch.yml")
            }
        }
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
) -> Result<PathBuf, McpConfigError> {
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
            Err(McpConfigError::Rollback {
                cause: Box::new(error),
                failures: rollback_errors,
            })
        };
    }
    Ok(config)
}

fn antigravity_permissions_path(home: &Path) -> PathBuf {
    home.join(".gemini")
        .join("antigravity-cli")
        .join("settings.json")
}

fn update_antigravity_permissions(path: &Path, enabled: bool) -> Result<(), McpConfigError> {
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
            .ok_or(McpConfigError::PermissionsNotAnObject)?;
        let allow = permissions
            .entry("allow")
            .or_insert_with(|| Value::Array(Vec::new()));
        let allow = allow
            .as_array_mut()
            .ok_or(McpConfigError::PermissionsAllowNotAnArray)?;
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

fn opencode_config_has_openpencil(path: &Path) -> bool {
    read_json_object(path)
        .ok()
        .and_then(|root| {
            root.get("mcp")
                .and_then(Value::as_object)?
                .get(SERVER_NAME)
                .and_then(Value::as_object)
                .map(|server| server.get("enabled").and_then(Value::as_bool) != Some(false))
        })
        .unwrap_or(false)
}

fn kiro_config_has_openpencil(path: &Path) -> bool {
    read_json_object(path)
        .ok()
        .and_then(|root| {
            root.get("mcpServers")
                .and_then(Value::as_object)?
                .get(SERVER_NAME)
                .and_then(Value::as_object)
                .map(|server| server.get("disabled").and_then(Value::as_bool) != Some(true))
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

/// The `type` + `url` streamable-HTTP shape. Claude Code, Copilot, the
/// Gemini CLI, Cursor, and ZCode all accept it — verified
/// against what `gemini mcp add --transport http` writes, against Cursor's
/// own config reader (which keys off `url` and ignores the extra `type`), and
/// against ZCode's settings form, which documents exactly this pair.
fn streamable_http_server(port: u16) -> Value {
    serde_json::json!({
        "type": "http",
        "url": endpoint(port),
    })
}

fn update_json_config(path: &Path, enabled: bool, port: u16) -> Result<(), McpConfigError> {
    update_mcp_servers_json(path, enabled, streamable_http_server(port))
}

/// OpenCode stores remote servers directly under `mcp` and uses `remote` as
/// the transport discriminator. This is deliberately separate from the
/// `mcpServers` layout used by most other clients.
fn update_opencode_config(path: &Path, enabled: bool, port: u16) -> Result<(), McpConfigError> {
    let mut root = read_json_object(path)?;
    if enabled {
        let mcp = root
            .entry("mcp")
            .or_insert_with(|| Value::Object(Map::new()));
        if !mcp.is_object() {
            *mcp = Value::Object(Map::new());
        }
        let mcp = mcp
            .as_object_mut()
            .ok_or(McpConfigError::McpServersNotAnObject)?;
        mcp.insert(
            SERVER_NAME.into(),
            serde_json::json!({
                "type": "remote",
                "url": endpoint(port),
                "enabled": true,
            }),
        );
    } else if let Some(mcp) = root.get_mut("mcp").and_then(Value::as_object_mut) {
        mcp.remove(SERVER_NAME);
        if mcp.is_empty() {
            root.remove("mcp");
        }
    }
    write_json_object(path, &root)
}

/// Kiro infers a remote transport from `url`; `type: "http"` is not part of
/// its native remote-server shape. Writing `disabled: false` makes re-enabling
/// an entry explicit.
fn kiro_server(port: u16) -> Value {
    serde_json::json!({
        "url": endpoint(port),
        "disabled": false,
    })
}

/// Qwen Code reads a plain `url` as SSE — `qwen mcp list` reports the
/// transport as `(sse)` — and only treats `httpUrl` as streamable HTTP, which
/// is what `qwen mcp add --transport http` itself writes.
fn qwen_server(port: u16) -> Value {
    serde_json::json!({ "httpUrl": endpoint(port) })
}

/// Kimi spells the discriminator `transport`, not `type`: kimi-code's config
/// schema is a union discriminated on that field, and only falls back to
/// inferring the transport from `command` vs `url` when it is absent. Stating
/// it keeps the entry unambiguous; the legacy `kimi-cli` writes the same pair.
fn kimi_server(port: u16) -> Value {
    serde_json::json!({
        "url": endpoint(port),
        "transport": "http",
    })
}

/// ZCode nests its server map at `mcp.servers` rather than using a top-level
/// `mcpServers`, so it needs its own reader/writer pair. The entry value is
/// the same `type` + `url` shape the other HTTP clients take. A server is
/// enabled unless it carries `enabled: false`, so enabling writes no flag.
fn update_zcode_config(path: &Path, enabled: bool, port: u16) -> Result<(), McpConfigError> {
    let mut root = read_json_object(path)?;
    if enabled {
        let mcp = root
            .entry("mcp")
            .or_insert_with(|| Value::Object(Map::new()));
        if !mcp.is_object() {
            *mcp = Value::Object(Map::new());
        }
        let Some(mcp) = mcp.as_object_mut() else {
            return Err(McpConfigError::McpServersNotAnObject);
        };
        let servers = mcp
            .entry("servers")
            .or_insert_with(|| Value::Object(Map::new()));
        if !servers.is_object() {
            *servers = Value::Object(Map::new());
        }
        let Some(servers) = servers.as_object_mut() else {
            return Err(McpConfigError::McpServersNotAnObject);
        };
        servers.insert(SERVER_NAME.into(), streamable_http_server(port));
    } else if let Some(mcp) = root.get_mut("mcp").and_then(Value::as_object_mut) {
        // Only prune containers this integration created — sibling keys under
        // `mcp` belong to ZCode's own settings.
        if let Some(servers) = mcp.get_mut("servers").and_then(Value::as_object_mut) {
            servers.remove(SERVER_NAME);
            if servers.is_empty() {
                mcp.remove("servers");
            }
        }
        if mcp.is_empty() {
            root.remove("mcp");
        }
    }
    write_json_object(path, &root)
}

fn zcode_config_has_openpencil(path: &Path) -> bool {
    read_json_object(path)
        .ok()
        .and_then(|root| {
            let servers = root
                .get("mcp")
                .and_then(Value::as_object)?
                .get("servers")
                .and_then(Value::as_object)?;
            Some(servers.contains_key(SERVER_NAME))
        })
        .unwrap_or(false)
}

/// Merge `server` in under `mcpServers.openpencil` (or remove that key when
/// disabling), leaving every other server and top-level setting untouched.
fn update_mcp_servers_json(
    path: &Path,
    enabled: bool,
    server: Value,
) -> Result<(), McpConfigError> {
    let mut root = read_json_object(path)?;
    if enabled {
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()));
        if !servers.is_object() {
            *servers = Value::Object(Map::new());
        }
        let Some(servers) = servers.as_object_mut() else {
            return Err(McpConfigError::McpServersNotAnObject);
        };
        servers.insert(SERVER_NAME.into(), server);
    } else if let Some(servers) = root.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(SERVER_NAME);
        if servers.is_empty() {
            root.remove("mcpServers");
        }
    }
    write_json_object(path, &root)
}

fn update_antigravity_config(path: &Path, enabled: bool, port: u16) -> Result<(), McpConfigError> {
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
            .ok_or(McpConfigError::McpServersNotAnObject)?;
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

fn read_json_object(path: &Path) -> Result<Map<String, Value>, McpConfigError> {
    // `std::io::Error` / `serde_json::Error` come from crates this pass does
    // not own, so their messages ride along as text.
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(e) => {
            return Err(McpConfigError::Read {
                path: path.to_path_buf(),
                message: e.to_string(),
            })
        }
    };
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|e| McpConfigError::Parse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| McpConfigError::NotAJsonObject {
            path: path.to_path_buf(),
        })
}

fn write_json_object(path: &Path, root: &Map<String, Value>) -> Result<(), McpConfigError> {
    let text = serde_json::to_string_pretty(root).map_err(|e| McpConfigError::Serialize {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    atomic_write(path, format!("{text}\n").as_bytes())
}

fn update_codex_config(path: &Path, enabled: bool, port: u16) -> Result<(), McpConfigError> {
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(McpConfigError::Read {
                path: path.to_path_buf(),
                message: e.to_string(),
            })
        }
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
#[path = "mcp_integrations_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mcp_integrations_opencode_kiro_tests.rs"]
mod opencode_kiro_tests;

#[cfg(test)]
#[path = "mcp_integrations_dsh_tests.rs"]
mod dsh_tests;
