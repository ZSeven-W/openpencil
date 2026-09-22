//! Cline CLI MCP settings editing.
//!
//! Cline reads remote servers from `cline_mcp_settings.json`, resolved the
//! same way as its `resolveMcpSettingsPath()`: `CLINE_MCP_SETTINGS_PATH`,
//! else `$CLINE_DATA_DIR/settings/`, else `$CLINE_DIR/data/settings/`, else
//! `~/.cline/data/settings/`. (Its docs mention `~/.cline/mcp.json`; the CLI
//! runtime does not read that file.)
//!
//! The shared `{"type": "http", "url"}` shape is NOT valid here: Cline's
//! loader only accepts `stdio` / `sse` / `streamableHttp` as a `type`, and a
//! bare `url` defaults to SSE. The entry is therefore written in Cline's
//! canonical nested form, exactly what `cline mcp install --transport http`
//! stores:
//!
//! ```json
//! { "transport": { "type": "streamableHttp", "url": "http://127.0.0.1:<port>/mcp" },
//!   "disabled": false }
//! ```
//!
//! Unlike the generic `mcpServers` merge, a malformed `mcpServers` value is
//! refused rather than replaced, and an `openpencil` entry that does not
//! point at a local OpenPencil endpoint is treated as the user's own and
//! never overwritten or deleted.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::mcp_config_error::McpConfigError;
use crate::mcp_integrations::{read_json_object, write_json_object};

const SERVER_NAME: &str = "openpencil";
const SETTINGS_FILE_NAME: &str = "cline_mcp_settings.json";

/// The Cline-relevant environment, captured once so path resolution is a
/// pure function tests can drive without touching process-global env.
#[derive(Debug, Default, Clone)]
pub(crate) struct ClineEnv {
    pub(crate) mcp_settings_path: Option<OsString>,
    pub(crate) data_dir: Option<OsString>,
    pub(crate) cline_dir: Option<OsString>,
}

impl ClineEnv {
    pub(crate) fn from_process() -> Self {
        Self {
            mcp_settings_path: std::env::var_os("CLINE_MCP_SETTINGS_PATH"),
            data_dir: std::env::var_os("CLINE_DATA_DIR"),
            cline_dir: std::env::var_os("CLINE_DIR"),
        }
    }
}

/// Cline trims each override and ignores blank ones.
fn non_blank(value: &Option<OsString>) -> Option<PathBuf> {
    value
        .as_ref()
        .map(|v| v.to_string_lossy().trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn cline_settings_path(home: &Path, env: &ClineEnv) -> PathBuf {
    if let Some(path) = non_blank(&env.mcp_settings_path) {
        return path;
    }
    let data_dir = non_blank(&env.data_dir).unwrap_or_else(|| {
        non_blank(&env.cline_dir)
            .unwrap_or_else(|| home.join(".cline"))
            .join("data")
    });
    data_dir.join("settings").join(SETTINGS_FILE_NAME)
}

fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

/// The URL an entry points at, across Cline's nested and legacy flat forms.
fn entry_url(server: &Map<String, Value>) -> Option<&str> {
    server
        .get("transport")
        .and_then(Value::as_object)
        .and_then(|transport| transport.get("url"))
        .or_else(|| server.get("url"))
        .and_then(Value::as_str)
}

/// Whether `server` is an entry this integration may rewrite or remove: it
/// targets the loopback `/mcp` endpoint the desktop host serves (any port —
/// the port changes when the user reconfigures the server).
fn is_openpencil_managed(server: &Value) -> bool {
    let Some(url) = server.as_object().and_then(entry_url) else {
        return false;
    };
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.path() == "/mcp"
            && url.query().is_none()
    })
}

pub(crate) fn update_cline_config(
    path: &Path,
    enabled: bool,
    port: u16,
) -> Result<(), McpConfigError> {
    if !enabled && !path.exists() {
        return Ok(());
    }
    let mut root = read_json_object(path)?;
    let servers = match root.get_mut("mcpServers") {
        Some(Value::Object(servers)) => Some(servers),
        Some(_) => return Err(McpConfigError::McpServersNotAnObject),
        None => None,
    };
    let existing = servers
        .as_ref()
        .and_then(|servers| servers.get(SERVER_NAME));
    if existing.is_some_and(|server| !is_openpencil_managed(server)) {
        return Err(McpConfigError::ClineForeignEntry {
            path: path.to_path_buf(),
        });
    }

    if enabled {
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or(McpConfigError::McpServersNotAnObject)?;
        // Keep Cline-owned per-server settings (`timeout`, `metadata`, …)
        // across re-enables and port changes; drop legacy flat transport
        // keys so the nested form is the only one left.
        let mut server = servers
            .remove(SERVER_NAME)
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        for legacy in ["type", "transportType", "url", "headers"] {
            server.remove(legacy);
        }
        server.insert(
            "transport".into(),
            serde_json::json!({ "type": "streamableHttp", "url": endpoint(port) }),
        );
        server.insert("disabled".into(), Value::Bool(false));
        servers.insert(SERVER_NAME.into(), Value::Object(server));
    } else if let Some(servers) = root.get_mut("mcpServers").and_then(Value::as_object_mut) {
        if servers.remove(SERVER_NAME).is_none() {
            return Ok(());
        }
        // Cline itself writes `{"mcpServers": {}}`; keep the container.
    } else {
        return Ok(());
    }
    write_json_object(path, &root)
}

/// Enabled means an `openpencil` entry Cline would load: present, not
/// `disabled: true`, and carrying a URL. Hand-wired entries count too so the
/// toggle reflects what Cline will actually do.
pub(crate) fn cline_config_has_openpencil(path: &Path) -> bool {
    read_json_object(path)
        .ok()
        .and_then(|root| {
            let server = root
                .get("mcpServers")?
                .as_object()?
                .get(SERVER_NAME)?
                .as_object()?
                .clone();
            Some(
                server.get("disabled").and_then(Value::as_bool) != Some(true)
                    && entry_url(&server).is_some(),
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "mcp_integrations_cline_tests.rs"]
mod tests;
