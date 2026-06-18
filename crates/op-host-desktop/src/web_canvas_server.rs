//! Headless web-canvas daemon — serves the document to the Rust WASM web shell
//! (`op-host-web`, which runs in a browser and can't bind a socket) and to
//! external MCP/CLI clients. It is the Rust analog of the TS web app's
//! `apps/web/server/api/mcp/*` Nitro routes + `setSyncDocument`: it owns the
//! canonical document in memory and answers the same whole-document REST sync
//! shape, so a JSON-RPC/REST client (e.g. the `op` CLI or any MCP client) can
//! drive the Rust *web* canvas the same way it drives the desktop canvas.
//!
//! This module ships the request-handling CORE (`handle_web_canvas_request`,
//! fully unit-testable without a socket) plus a runnable loop
//! (`run_web_canvas`, behind
//! `openpencil-desktop --serve-web <port> [doc] [--host <addr>]`).
//! Layered on top: an SSE endpoint that streams `version` bumps to connected
//! shells, static serving of the host page + WASM bundle (`crate::web_static`
//! — `GET /` and `GET /pkg/*`), and a token-authed `openpencil/shutdown`
//! (same contract as `--mcp-http`) so `op stop` works against this daemon.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use op_editor_core::agent_settings::{AcpAgentConnectOutcome, ProviderConnectOutcome};
use op_editor_core::EditorState;

/// Slow/stalled-peer bound — bodies can be large (whole documents with embedded
/// images), so a connection that opens and dribbles must not pin a thread.
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Max concurrent connections (SSE streams are long-lived, so bound them).
const MAX_CONNS: usize = 64;

/// SSE keep-alive cadence — also how quickly a disconnected SSE client is
/// detected (the heartbeat write fails once the socket is gone).
const SSE_HEARTBEAT: Duration = Duration::from_secs(15);

/// Broadcast hub for SSE subscribers. Each `GET /api/mcp/events` connection
/// registers a channel; a document mutation broadcasts the new version to all
/// of them, and each SSE connection thread writes it to its socket. Senders to
/// disconnected clients are pruned on the next broadcast.
#[derive(Default)]
pub(crate) struct SseHub {
    subscribers: Mutex<Vec<mpsc::Sender<u64>>>,
}

impl SseHub {
    /// Register a subscriber; the SSE connection thread blocks on the returned
    /// receiver for version bumps.
    pub(crate) fn subscribe(&self) -> Receiver<u64> {
        let (tx, rx) = mpsc::channel();
        self.subscribers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(tx);
        rx
    }

    /// Broadcast a version bump to all live subscribers, pruning any whose
    /// receiver was dropped (client disconnected).
    pub(crate) fn broadcast(&self, version: u64) {
        self.subscribers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|tx| tx.send(version).is_ok());
    }

    #[cfg(test)]
    pub(crate) fn subscriber_count(&self) -> usize {
        self.subscribers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }
}

/// RAII decrement for the connection counter — `Drop` runs on normal exit AND
/// panic unwind, so a panicking connection can't leak its `MAX_CONNS` slot.
struct ConnGuard(Arc<AtomicUsize>);

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// In-memory document authority for the web-canvas daemon — the Rust mirror of
/// the TS `mcp-sync-state` (document + monotonic version). The browser shell
/// mirrors this (over the SSE endpoint, layered on later); external MCP clients
/// read/replace it over `/api/mcp/document`.
pub(crate) struct WebCanvasState {
    pub(crate) editor: EditorState,
    /// Monotonic sync version, bumped on every document mutation — the key the
    /// browser shell uses to detect that the live document changed.
    pub(crate) version: u64,
    /// The bound port, reported by `GET /api/mcp/server` (TS `server.get.ts`
    /// parity).
    pub(crate) port: u16,
}

impl WebCanvasState {
    pub(crate) fn new(editor: EditorState, port: u16) -> Self {
        Self {
            editor,
            version: 0,
            port,
        }
    }

    /// Replace the whole document (an already-loaded `POST /api/mcp/document`
    /// body), bump and return the new version.
    pub(crate) fn replace_document(&mut self, doc: jian_ops_schema::PenDocument) -> u64 {
        self.editor.replace_document(doc);
        self.version += 1;
        self.version
    }
}

/// A handled reply: HTTP status line + JSON body, ready for
/// `write_mcp_http_response`.
pub(crate) struct WebReply {
    pub(crate) status: &'static str,
    pub(crate) body: String,
}

/// Handle one parsed web-canvas REST request against the in-memory state. Pure
/// w.r.t. IO — fully unit-testable without a socket. Mirrors the TS Nitro
/// routes:
/// - `GET  /api/mcp/server`   → health `{ok:true,…}` (like `server.get.ts`)
/// - `GET  /api/mcp/document` → `{document:<doc>,version}` (like `document.get.ts`)
/// - `POST /api/mcp/document` → whole-doc replace → `{ok:true,version}` (like `document.post.ts`)
/// - `GET  /api/mcp/version`  → `{version}` — Rust-only cheap change probe; the
///   TS stack pushes documents over SSE instead, so it never needs one. The
///   browser shell polls this and fetches the full document only on a bump.
/// - `GET  /api/mcp/selection` → `{selectedIds,activePageId}` (like `selection.get.ts`)
/// - `POST /api/mcp/selection` → renderer selection push (like `selection.post.ts`)
/// - `POST /api/file/open-recent` → local-daemon recent-file open, used by the
///   browser shell because only the daemon can read local paths.
/// - anything else → 404 (the JSON-RPC `/mcp` path + SSE are handled by the
///   caller's connection loop, not here).
pub(crate) fn handle_web_canvas_request(
    method: &str,
    path: &str,
    body: &str,
    state: &mut WebCanvasState,
) -> WebReply {
    match (method, path) {
        ("GET", "/api/mcp/server") => WebReply {
            status: "200 OK",
            // `{running,port,localIp}` matches TS `server.get.ts`; the daemon
            // binds 127.0.0.1 (localhost-only) so localIp is loopback. Extra
            // `server`/`mode` fields are additive diagnostics.
            body: format!(
                r#"{{"running":true,"port":{},"localIp":"127.0.0.1","server":"openpencil-mcp","mode":"web-canvas"}}"#,
                state.port
            ),
        },
        ("GET", "/api/mcp/document") => match serde_json::to_string(&state.editor.doc) {
            Ok(doc_json) => WebReply {
                status: "200 OK",
                body: format!(r#"{{"document":{doc_json},"version":{}}}"#, state.version),
            },
            Err(e) => WebReply {
                status: "500 Internal Server Error",
                body: crate::mcp_serve::rest_error_body(&e.to_string()),
            },
        },
        ("POST", "/api/mcp/document") => {
            let document_json = match crate::mcp_serve::parse_document_sync_body(body) {
                Ok(json) => json,
                Err(message) => {
                    return WebReply {
                        status: "400 Bad Request",
                        body: crate::mcp_serve::rest_error_body(&message),
                    };
                }
            };
            // Load via the same proven path as desktop file-open. A load failure
            // is a client fault → 400, like the TS validation 400s.
            match op_pen_loader::load_canonical(&document_json) {
                Ok(loaded) => {
                    for w in &loaded.warnings {
                        eprintln!("openpencil-desktop --serve-web: schema warning: {w:?}");
                    }
                    let version = state.replace_document(loaded.value);
                    WebReply {
                        status: "200 OK",
                        body: crate::mcp_serve::document_sync_ok(version),
                    }
                }
                Err(e) => WebReply {
                    status: "400 Bad Request",
                    body: crate::mcp_serve::rest_error_body(&e.to_string()),
                },
            }
        }
        ("GET", "/api/mcp/version") => WebReply {
            status: "200 OK",
            body: format!(r#"{{"version":{}}}"#, state.version),
        },
        ("GET", "/api/mcp/selection") => {
            // TS `selection.get.ts` → `getSyncSelection()` shape:
            // `{selectedIds, activePageId}`. Read straight off the live
            // editor selection so MCP clients and the REST route agree.
            let ids: Vec<&str> = state
                .editor
                .selection
                .set
                .iter()
                .map(|id| id.as_str())
                .collect();
            let active_page_id = state
                .editor
                .doc
                .pages
                .as_ref()
                .and_then(|pages| pages.get(state.editor.ui.active_page_index))
                .map(|page| page.id.clone());
            let body = serde_json::json!({
                "selectedIds": ids,
                "activePageId": active_page_id,
            });
            WebReply {
                status: "200 OK",
                body: serde_json::to_string(&body)
                    .unwrap_or_else(|_| r#"{"selectedIds":[],"activePageId":null}"#.to_string()),
            }
        }
        ("POST", "/api/mcp/selection") => apply_selection_sync(body, state),
        ("POST", "/api/file/open-recent") => open_recent_file(body, state),
        ("POST", "/api/agents/connect") => {
            handle_provider_connect_request_with_probe(body, state, crate::provider_probe::connect_provider)
        }
        ("POST", "/api/acp/connect") => {
            handle_acp_agent_connect_request_with_probe(
                body,
                state,
                crate::acp_agent_probe_host::probe_acp_agent_config,
            )
        }
        ("GET", "/api/ai/models") => WebReply {
            // JSON array of model ids the AI proxy can serve (the
            // configured built-in agents). The web bundle queries this
            // to populate its model picker without bundling a static
            // list or holding API keys. `POST /api/ai/stream` is a
            // streaming route handled in the connection loop, not here.
            status: "200 OK",
            body: crate::ai_proxy::models_json(&state.editor),
        },
        _ => WebReply {
            status: "404 Not Found",
            body: r#"{"ok":false,"error":"Not found. Use /api/mcp/document, /api/mcp/server, or /mcp."}"#
                .to_string(),
        },
    }
}

pub(crate) fn handle_acp_agent_connect_request_with_probe<F>(
    body: &str,
    state: &mut WebCanvasState,
    probe: F,
) -> WebReply
where
    F: FnOnce(op_acp::AcpAgentConfig) -> crate::acp_agent_probe_host::AcpAgentProbeOutcome,
{
    let Some(id) = parse_acp_agent_connect_request(body) else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("Missing ACP agent id"),
        };
    };
    let Some(index) = state
        .editor
        .editor_ui
        .agent_settings
        .acp_agents
        .iter()
        .position(|agent| agent.id == id && agent.ready())
    else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("ACP agent is not configured"),
        };
    };
    let agent = state.editor.editor_ui.agent_settings.acp_agents[index].clone();
    state
        .editor
        .editor_ui
        .agent_settings
        .begin_acp_agent_connect(index);
    let outcome = probe(crate::acp_agent_probe_host::acp_config_for_probe(&agent));
    apply_acp_agent_probe_outcome(&id, outcome, state)
}

fn parse_acp_agent_connect_request(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    parsed
        .get("id")
        .or_else(|| parsed.get("agentId"))
        .and_then(|v| v.as_str())
        .filter(|id| !id.trim().is_empty())
        .map(str::to_string)
}

fn apply_acp_agent_probe_outcome(
    id: &str,
    outcome: crate::acp_agent_probe_host::AcpAgentProbeOutcome,
    state: &mut WebCanvasState,
) -> WebReply {
    state
        .editor
        .editor_ui
        .agent_settings
        .apply_acp_agent_connect_outcome(
            id,
            AcpAgentConnectOutcome {
                connected: outcome.connected,
                info: outcome.info.clone(),
                error: outcome.error.clone(),
            },
        );
    state.editor.rebuild_chat_models();
    WebReply {
        status: "200 OK",
        body: serde_json::json!({
            "ok": true,
            "id": id,
            "connected": outcome.connected,
            "connectionInfo": outcome.info,
            "error": outcome.error,
        })
        .to_string(),
    }
}

pub(crate) fn handle_provider_connect_request_with_probe<F>(
    body: &str,
    state: &mut WebCanvasState,
    probe: F,
) -> WebReply
where
    F: FnOnce(op_ai::agent_settings_state::AgentProvider) -> crate::provider_probe::ProbeOutcome,
{
    let Some(provider) = parse_provider_connect_request(body) else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("Missing provider"),
        };
    };
    state
        .editor
        .editor_ui
        .agent_settings
        .begin_provider_connect(provider);
    let outcome = probe(provider_to_probe(provider));
    apply_provider_probe_outcome(provider, outcome, state)
}

fn parse_provider_connect_request(body: &str) -> Option<op_editor_core::AgentProvider> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    parsed
        .get("provider")
        .and_then(|v| v.as_str())
        .and_then(parse_agent_provider)
}

fn parse_agent_provider(raw: &str) -> Option<op_editor_core::AgentProvider> {
    use op_editor_core::AgentProvider;
    let normalized = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect::<String>();
    match normalized.as_str() {
        "claude" | "claudecode" => Some(AgentProvider::ClaudeCode),
        "codex" | "codexcli" => Some(AgentProvider::CodexCli),
        "opencode" => Some(AgentProvider::OpenCode),
        "githubcopilot" | "copilot" => Some(AgentProvider::GithubCopilot),
        "gemini" | "geminicli" => Some(AgentProvider::GeminiCli),
        _ => None,
    }
}

fn provider_to_probe(
    provider: op_editor_core::AgentProvider,
) -> op_ai::agent_settings_state::AgentProvider {
    use op_ai::agent_settings_state::AgentProvider as ProbeProvider;
    use op_editor_core::AgentProvider;
    match provider {
        AgentProvider::ClaudeCode => ProbeProvider::ClaudeCode,
        AgentProvider::CodexCli => ProbeProvider::CodexCli,
        AgentProvider::OpenCode => ProbeProvider::OpenCode,
        AgentProvider::GithubCopilot => ProbeProvider::GithubCopilot,
        AgentProvider::GeminiCli => ProbeProvider::GeminiCli,
    }
}

fn apply_provider_probe_outcome(
    provider: op_editor_core::AgentProvider,
    outcome: crate::provider_probe::ProbeOutcome,
    state: &mut WebCanvasState,
) -> WebReply {
    let outcome = crate::provider_probe_host::normalize_provider_probe_outcome(provider, outcome);
    let crate::provider_probe::ProbeOutcome {
        connected,
        models,
        error,
        warning,
        not_installed,
        install_command,
        connection_info,
        hint_path,
        version,
    } = outcome;
    let response_models: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            serde_json::json!({
                "provider": provider_key(provider),
                "value": m.value,
                "displayName": m.display_name,
            })
        })
        .collect();
    state
        .editor
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            provider,
            ProviderConnectOutcome {
                connected,
                info: connection_info.clone(),
                warning: warning.clone(),
                error: error.clone(),
                not_installed,
                install_command: install_command.clone(),
                hint_path: hint_path.clone(),
                version: version.clone(),
            },
        );
    state
        .editor
        .editor_ui
        .agent_settings
        .pending_provider_connect = None;
    if connected && !models.is_empty() {
        state
            .editor
            .chat
            .discovered_models
            .retain(|m| m.provider != provider);
        state.editor.chat.discovered_models.extend(
            models
                .into_iter()
                .map(crate::model_discovery::model_entry_to_ec),
        );
        sort_discovered_models(&mut state.editor);
    }
    state.editor.rebuild_chat_models();
    WebReply {
        status: "200 OK",
        body: serde_json::json!({
            "ok": true,
            "provider": provider_key(provider),
            "connected": connected,
            "models": response_models,
            "error": error,
            "warning": warning,
            "notInstalled": not_installed,
            "installCommand": install_command,
            "connectionInfo": connection_info,
            "hintPath": hint_path,
            "version": version,
        })
        .to_string(),
    }
}

fn sort_discovered_models(editor: &mut EditorState) {
    editor.chat.discovered_models.sort_by_key(|m| {
        op_editor_core::AgentProvider::ALL
            .iter()
            .position(|p| *p == m.provider)
            .unwrap_or(usize::MAX)
    });
}

fn provider_key(provider: op_editor_core::AgentProvider) -> &'static str {
    use op_editor_core::AgentProvider;
    match provider {
        AgentProvider::ClaudeCode => "claude",
        AgentProvider::CodexCli => "codex",
        AgentProvider::OpenCode => "opencode",
        AgentProvider::GithubCopilot => "github-copilot",
        AgentProvider::GeminiCli => "gemini",
    }
}

fn open_recent_file(body: &str, state: &mut WebCanvasState) -> WebReply {
    let parsed: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let Some(path_s) = parsed
        .as_ref()
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
    else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("Missing path string"),
        };
    };
    if !state
        .editor
        .editor_ui
        .recent_files
        .iter()
        .any(|recent| recent.path == path_s)
    {
        return WebReply {
            status: "404 Not Found",
            body: crate::mcp_serve::rest_error_body("Path is not in recent files"),
        };
    }
    let path = PathBuf::from(&path_s);
    match crate::mcp_serve::load_editor_state(&path) {
        Ok(mut next) => {
            preserve_web_canvas_preferences(&state.editor, &mut next);
            set_file_name_display(&mut next, &path);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            next.editor_ui.touch_recent_file(path_s, now);
            state.editor = next;
            state.version += 1;
            WebReply {
                status: "200 OK",
                body: crate::mcp_serve::document_sync_ok(state.version),
            }
        }
        Err(e) => {
            let pruned = state.editor.editor_ui.remove_recent_file(&path_s);
            WebReply {
                status: "400 Bad Request",
                body: serde_json::json!({
                    "ok": false,
                    "pruned": pruned,
                    "error": e,
                })
                .to_string(),
            }
        }
    }
}

fn preserve_web_canvas_preferences(previous: &EditorState, next: &mut EditorState) {
    let previous_selected_model = previous.chat.selected_model_entry().cloned();
    next.editor_ui.theme_mode = previous.editor_ui.theme_mode;
    next.editor_ui.locale = previous.editor_ui.locale;
    next.editor_ui.recent_files = previous.editor_ui.recent_files.clone();
    next.ui_kits = previous.ui_kits.clone();
    next.theme_presets = previous.theme_presets.clone();
    next.theme_presets_dirty = previous.theme_presets_dirty;
    next.editor_ui.agent_settings = previous.editor_ui.agent_settings.clone();
    next.editor_ui.chat_selected_agent = previous.editor_ui.chat_selected_agent;
    next.chat.discovered_models = previous.chat.discovered_models.clone();
    next.rebuild_chat_models();
    if let Some(prev) = previous_selected_model {
        if let Some(idx) = next.chat.available_models.iter().position(|m| {
            m.provider == prev.provider
                && m.value == prev.value
                && m.builtin_provider_id == prev.builtin_provider_id
        }) {
            next.select_chat_model(idx);
        }
    }
}

fn set_file_name_display(state: &mut EditorState, path: &std::path::Path) {
    state.editor_ui.file_name_display = path.file_name().map(|n| n.to_string_lossy().into_owned());
}

/// Apply a renderer selection push (`POST /api/mcp/selection`) to the live
/// editor state, mirroring TS `selection.post.ts` + `setSyncSelection`:
/// `selectedIds` must be an array (else 400 with the TS error text); the ids
/// are stored verbatim (TS does no validation — the browser's document is the
/// same synced document, so its ids are normally live here too); a present,
/// non-null `activePageId` switches the active page WHEN the id resolves
/// (documented divergence: TS stores the raw string, Rust keeps a page index
/// so an unknown id is ignored rather than stored). Selection is not part of
/// the document, so no version bump / SSE broadcast happens (TS parity).
fn apply_selection_sync(body: &str, state: &mut WebCanvasState) -> WebReply {
    let parsed: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let Some(ids) = parsed
        .as_ref()
        .and_then(|v| v.get("selectedIds"))
        .and_then(|v| v.as_array())
    else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("Missing selectedIds array"),
        };
    };
    let node_ids: Vec<op_editor_core::NodeId> = ids
        .iter()
        .filter_map(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(op_editor_core::NodeId::new)
        .collect();
    let editor = &mut state.editor;
    editor.selection.anchor = node_ids
        .last()
        .cloned()
        .unwrap_or(op_editor_core::NodeId::NONE);
    editor.selection.set = node_ids;
    if let Some(page_id) = parsed
        .as_ref()
        .and_then(|v| v.get("activePageId"))
        .and_then(|v| v.as_str())
    {
        let index = editor
            .doc
            .pages
            .as_ref()
            .and_then(|pages| pages.iter().position(|p| p.id == page_id));
        if let Some(index) = index {
            let _ = editor.set_active_page(index);
        }
    }
    WebReply {
        status: "200 OK",
        body: r#"{"ok":true}"#.to_string(),
    }
}

/// Parse the argv tail of `--serve-web <port> [doc] [--host <addr>]` (the
/// args after `--serve-web` itself). Pure, so the flag shape is unit-testable
/// without spawning the binary. The host defaults to loopback; `--host
/// 0.0.0.0` is the LAN/Docker opt-in (no TLS — deploy behind a proxy for
/// anything beyond a trusted network).
pub(crate) fn parse_serve_web_args<I: Iterator<Item = String>>(
    mut args: I,
) -> Result<(u16, Option<PathBuf>, String), String> {
    let Some(port_arg) = args.next() else {
        return Err("missing <port> arg".into());
    };
    let Ok(port) = port_arg.parse::<u16>() else {
        return Err(format!("<port> must be a u16, got {port_arg:?}"));
    };
    let mut path: Option<PathBuf> = None;
    let mut host = "127.0.0.1".to_string();
    while let Some(arg) = args.next() {
        if arg == "--host" {
            host = args.next().ok_or("--host needs a value (e.g. 0.0.0.0)")?;
        } else if let Some(value) = arg.strip_prefix("--host=") {
            host = value.to_string();
        } else if path.is_none() {
            // The document path is optional — without it the daemon starts
            // from the same starter document the web shell paints locally.
            path = Some(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected arg {arg:?}"));
        }
    }
    if host.is_empty() {
        return Err("--host must not be empty".into());
    }
    Ok((port, path, host))
}

fn startup_editor_from_base_for_web_canvas(
    base: EditorState,
    path: Option<PathBuf>,
) -> Result<EditorState, String> {
    match path {
        Some(p) => {
            let mut next = crate::mcp_serve::load_editor_state(&p)?;
            preserve_web_canvas_preferences(&base, &mut next);
            set_file_name_display(&mut next, &p);
            next.editor_ui.touch_recent_file(
                p.to_string_lossy().into_owned(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            );
            Ok(next)
        }
        None => Ok(base),
    }
}

pub(crate) fn startup_editor_for_web_canvas(path: Option<PathBuf>) -> Result<EditorState, String> {
    let mut base = EditorState::starter();
    crate::settings_io::load(&mut base);
    startup_editor_from_base_for_web_canvas(base, path)
}

/// Run the web-canvas daemon on `host:port` (default `127.0.0.1`), backed by
/// the document at `path` (or the starter document when `None`). Serves the
/// static host page + bundle, the whole-document REST sync + health routes,
/// and falls through to the JSON-RPC `/mcp` tool dispatch (applied against
/// the in-memory document). Blocks until a token-authed shutdown request.
pub fn run_web_canvas(path: Option<PathBuf>, port: u16, host: &str) -> Result<(), String> {
    let editor = startup_editor_for_web_canvas(path)?;
    let listener =
        TcpListener::bind((host, port)).map_err(|e| format!("bind {host}:{port}: {e}"))?;
    let bound = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    eprintln!("openpencil-desktop --serve-web: listening on {host}:{bound}");
    match crate::web_static::resolve_bundle_dir() {
        Some(dir) => eprintln!(
            "openpencil-desktop --serve-web: serving web bundle from {}",
            dir.display()
        ),
        None => eprintln!(
            "openpencil-desktop --serve-web: no web bundle found — `/` serves build \
             instructions (tools/check-wasm-bundle.sh, or set OPENPENCIL_WEB_BUNDLE_DIR)"
        ),
    }
    // Shared across connection threads: the document authority (one writer at a
    // time via the Mutex) + the SSE broadcast hub. Thread-per-connection so a
    // long-lived SSE stream (or a slow client) never blocks other clients.
    let state = Arc::new(Mutex::new(WebCanvasState::new(editor, bound)));
    let hub = Arc::new(SseHub::default());
    let conn_count = Arc::new(AtomicUsize::new(0));
    // Raised by a connection thread that accepted a token-authed
    // `openpencil/shutdown`; the accept loop checks it per iteration. The
    // raiser also pokes the listener with a throwaway connection so a blocked
    // `accept` wakes up and observes the flag.
    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    for stream in listener.incoming() {
        if shutdown.load(Ordering::Acquire) {
            break;
        }
        let mut s = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("openpencil-desktop --serve-web: accept: {e}");
                continue;
            }
        };
        if conn_count.load(Ordering::Acquire) >= MAX_CONNS {
            let _ = s.set_write_timeout(Some(IO_TIMEOUT));
            let _ = crate::mcp_serve::write_mcp_http_response(
                &mut s,
                "503 Service Unavailable",
                r#"{"ok":false,"error":"server busy"}"#,
            );
            continue;
        }
        conn_count.fetch_add(1, Ordering::AcqRel);
        let state = Arc::clone(&state);
        let hub = Arc::clone(&hub);
        let conns = Arc::clone(&conn_count);
        let shutdown_flag = Arc::clone(&shutdown);
        let spawned = thread::Builder::new()
            .name("op-serve-web-conn".into())
            .spawn(move || {
                let _conn_guard = ConnGuard(conns);
                let _ = s.set_read_timeout(Some(IO_TIMEOUT));
                let _ = s.set_write_timeout(Some(IO_TIMEOUT));
                match serve_one(&mut s, &state, &hub) {
                    Ok(true) => {
                        shutdown_flag.store(true, Ordering::Release);
                        // Wake the (possibly blocked) accept loop. Loopback
                        // reaches the listener for both the 127.0.0.1 and the
                        // 0.0.0.0 binds.
                        let _ = std::net::TcpStream::connect(("127.0.0.1", bound));
                    }
                    Ok(false) => {}
                    Err(e) => eprintln!("openpencil-desktop --serve-web: {e}"),
                }
            });
        if spawned.is_err() {
            conn_count.fetch_sub(1, Ordering::AcqRel);
        }
    }
    eprintln!("openpencil-desktop --serve-web: shutdown requested; exiting");
    Ok(())
}

/// Handle one connection. Routes: static host page + wasm bundle (`GET /`,
/// `GET /pkg/*` via `crate::web_static`); SSE live-update stream (`GET
/// /api/mcp/events`); REST whole-doc sync / health (`/api/*` via
/// [`handle_web_canvas_request`]); else JSON-RPC `/mcp` tool dispatch. A
/// mutation (REST POST or a mutating tool call) bumps the version and is
/// broadcast to SSE subscribers. The state `Mutex` is held only across the
/// in-memory operation, never across the (long-lived) SSE wait.
///
/// Returns `Ok(true)` when the client requested a token-authed graceful
/// shutdown (same `openpencil/shutdown` contract as `--mcp-http`) — the
/// caller then stops the accept loop so `op stop` never signals a pid.
fn serve_one<S: Read + Write>(
    stream: &mut S,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
) -> Result<bool, String> {
    let req = crate::mcp_serve::read_http_request(stream)?;
    if req.method == "OPTIONS" {
        return crate::mcp_serve::write_mcp_http_response(stream, "204 No Content", "")
            .map(|()| false);
    }
    // Static serving: the host page (`/`) and the wasm-bindgen bundle
    // (`/pkg/*`). Owns only those paths — everything else falls through.
    if req.method == "GET" {
        let bundle_dir = crate::web_static::resolve_bundle_dir();
        if let Some(reply) =
            crate::web_static::handle_static_request(&req.path, bundle_dir.as_deref())
        {
            return crate::web_static::write_static_response(stream, &reply).map(|()| false);
        }
    }
    // SSE live-update stream: the browser shell subscribes and re-syncs whenever
    // the document version advances. Subscribe BEFORE reading the current
    // version so no broadcast is missed (a duplicate is harmless — versions are
    // monotonic). The state lock is released before the long SSE wait.
    if req.method == "GET" && req.path == "/api/mcp/events" {
        let rx = hub.subscribe();
        let current = state.lock().unwrap_or_else(|p| p.into_inner()).version;
        return serve_sse(stream, rx, current).map(|()| false);
    }
    // AI proxy stream: the browser bundle POSTs a model request and we
    // stream the provider's `ChatDelta`s back as SSE. Streaming route
    // (long-lived socket write), so handled here rather than in the
    // whole-body REST handler. Parse the body + build the provider
    // under the state lock, then DROP the lock before the long stream
    // — `proxy_provider` returns an owned `Box<dyn ChatProvider>`, so
    // nothing borrows the editor across the stream.
    if req.method == "POST" && req.path == "/api/ai/stream" {
        let Some(ai_req) = crate::ai_proxy::parse_ai_stream_body(&req.body) else {
            return crate::ai_proxy::write_sse_error(stream, "invalid request body")
                .map_err(|e| format!("ai stream error: {e}"))
                .map(|()| false);
        };
        let provider = {
            let guard = state.lock().unwrap_or_else(|p| p.into_inner());
            crate::ai_proxy::proxy_provider(&guard.editor, &ai_req.model)
        };
        let Some(provider) = provider else {
            return crate::ai_proxy::write_sse_error(stream, "no model configured")
                .map_err(|e| format!("ai stream error: {e}"))
                .map(|()| false);
        };
        return crate::ai_proxy::stream_ai_response(stream, ai_req, provider.as_ref())
            .map_err(|e| format!("ai stream: {e}"))
            .map(|()| false);
    }
    // Standard web chat/design turn: same external-CLI routing shape as
    // desktop standard mode (classify → chat / modify / new design), but
    // applied against this web-canvas daemon's document authority.
    if req.method == "POST" && req.path == "/api/ai/standard" {
        let Some(standard_req) = crate::web_chat_standard::parse_standard_turn_body(&req.body)
        else {
            return crate::ai_proxy::write_sse_error(stream, "invalid request body")
                .map_err(|e| format!("ai standard error: {e}"))
                .map(|()| false);
        };
        return crate::web_chat_standard::stream_standard_turn(stream, standard_req, state, hub)
            .map_err(|e| format!("ai standard: {e}"))
            .map(|()| false);
    }
    if req.method == "POST" && req.path == "/api/agents/connect" {
        let Some(provider) = parse_provider_connect_request(&req.body) else {
            return crate::mcp_serve::write_mcp_http_response(
                stream,
                "400 Bad Request",
                &crate::mcp_serve::rest_error_body("Missing provider"),
            )
            .map(|()| false);
        };
        let outcome = crate::provider_probe::connect_provider(provider_to_probe(provider));
        let reply = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            guard
                .editor
                .editor_ui
                .agent_settings
                .begin_provider_connect(provider);
            let reply = apply_provider_probe_outcome(provider, outcome, &mut guard);
            crate::settings_io::save(&guard.editor);
            reply
        };
        return crate::mcp_serve::write_mcp_http_response(stream, reply.status, &reply.body)
            .map(|()| false);
    }
    if req.method == "POST" && req.path == "/api/acp/connect" {
        let Some(id) = parse_acp_agent_connect_request(&req.body) else {
            return crate::mcp_serve::write_mcp_http_response(
                stream,
                "400 Bad Request",
                &crate::mcp_serve::rest_error_body("Missing ACP agent id"),
            )
            .map(|()| false);
        };
        let agent = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            let Some(index) = guard
                .editor
                .editor_ui
                .agent_settings
                .acp_agents
                .iter()
                .position(|agent| agent.id == id && agent.ready())
            else {
                return crate::mcp_serve::write_mcp_http_response(
                    stream,
                    "400 Bad Request",
                    &crate::mcp_serve::rest_error_body("ACP agent is not configured"),
                )
                .map(|()| false);
            };
            let agent = guard.editor.editor_ui.agent_settings.acp_agents[index].clone();
            guard
                .editor
                .editor_ui
                .agent_settings
                .begin_acp_agent_connect(index);
            agent
        };
        let outcome = crate::acp_agent_probe_host::probe_acp_agent_config(
            crate::acp_agent_probe_host::acp_config_for_probe(&agent),
        );
        let reply = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            let reply = apply_acp_agent_probe_outcome(&id, outcome, &mut guard);
            crate::settings_io::save(&guard.editor);
            reply
        };
        return crate::mcp_serve::write_mcp_http_response(stream, reply.status, &reply.body)
            .map(|()| false);
    }
    // All `/api/mcp/*` REST paths go to the REST handler — including ones this
    // daemon doesn't implement yet, which it answers with 404 rather than
    // mis-routing them into the JSON-RPC dispatch below.
    if req.path.starts_with("/api/") {
        let reply = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            let before = guard.version;
            let reply = handle_web_canvas_request(&req.method, &req.path, &req.body, &mut guard);
            // Broadcast INSIDE the state lock so the version bump and its
            // broadcast are atomic — otherwise two concurrent mutations could
            // broadcast their versions out of order (SSE clients seeing N then
            // N-1). `broadcast` only sends to unbounded channels (non-blocking),
            // so the lock is held briefly. Lock order is always state→hub.
            if guard.version != before {
                hub.broadcast(guard.version);
            }
            reply
        };
        return crate::mcp_serve::write_mcp_http_response(stream, reply.status, &reply.body)
            .map(|()| false);
    }
    // JSON-RPC tool dispatch is served ONLY as a POST to `/` or `/mcp`. An
    // unknown path is 404; a known path with the wrong method (e.g. `GET /mcp`)
    // is 405 — never silently dispatched as a tool call.
    let is_jsonrpc_path = req.path == "/" || req.path == "/mcp";
    if !is_jsonrpc_path {
        return crate::mcp_serve::write_mcp_http_response(
            stream,
            "404 Not Found",
            r#"{"ok":false,"error":"Not found. Use /, /pkg/*, /api/mcp/document, /api/mcp/server, /api/mcp/events, or /mcp."}"#,
        )
        .map(|()| false);
    }
    if req.method != "POST" {
        return crate::mcp_serve::write_mcp_http_response(
            stream,
            "405 Method Not Allowed",
            r#"{"ok":false,"error":"Method not allowed. POST a JSON-RPC message to /mcp."}"#,
        )
        .map(|()| false);
    }
    // Token-authed graceful shutdown (`op stop`): same contract as the
    // `--mcp-http` server — only the exact per-instance token passed by the
    // spawning CLI (via OPENPENCIL_MCP_TOKEN) authenticates; a stale file, a
    // recycled pid, or a random client cannot shut the daemon down.
    if let Some(id) = crate::mcp_serve::shutdown_request_id(
        &req.body,
        &crate::mcp_serve::headless_token_from_env().unwrap_or_default(),
    ) {
        crate::mcp_serve::write_mcp_http_response(
            stream,
            "200 OK",
            &crate::mcp_serve::shutdown_ok_response(&id),
        )?;
        return Ok(true);
    }
    // JSON-RPC `/mcp` dispatch against the in-memory document. A mutating apply
    // bumps the sync version, broadcast to SSE subscribers so the browser shell
    // sees JSON-RPC-driven changes too.
    let response = {
        let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
        let before = guard.version;
        let mut applied_any = false;
        let response = crate::mcp_serve::process_message_with_applier(
            &mut guard.editor,
            &req.body,
            |editor, cmd| {
                let ok = editor.apply(cmd.clone());
                applied_any |= ok;
                ok
            },
        )?
        .unwrap_or_default();
        if applied_any {
            guard.version += 1;
        }
        // Atomic bump+broadcast under the state lock (see the REST path) so SSE
        // version events stay monotonic across concurrent mutations.
        if guard.version != before {
            hub.broadcast(guard.version);
        }
        response
    };
    let status = if response.is_empty() {
        "202 Accepted"
    } else {
        "200 OK"
    };
    crate::mcp_serve::write_mcp_http_response(stream, status, &response).map(|()| false)
}

/// Stream Server-Sent Events to a subscribed client: write the SSE headers,
/// emit the current version immediately (initial sync), then forward each
/// version bump from `rx` as a `data: {"version":N}` event. A periodic
/// heartbeat comment keeps the connection alive AND detects a disconnected
/// client (the write fails once the socket is gone). Returns when the client
/// disconnects (write error) or the hub is dropped.
fn serve_sse<S: Write>(
    stream: &mut S,
    rx: Receiver<u64>,
    current_version: u64,
) -> Result<(), String> {
    let headers = "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: keep-alive\r\n\
         Access-Control-Allow-Origin: *\r\n\r\n";
    stream
        .write_all(headers.as_bytes())
        .map_err(|e| format!("sse headers: {e}"))?;
    write_sse_event(stream, current_version)?;
    loop {
        match rx.recv_timeout(SSE_HEARTBEAT) {
            Ok(mut version) => {
                // Coalesce any further queued bumps — only the latest version
                // matters (the client re-fetches the whole document on it), so
                // a burst of mutations collapses to a single event and the
                // channel can't accumulate unboundedly behind a slow client.
                while let Ok(next) = rx.try_recv() {
                    version = next;
                }
                write_sse_event(stream, version)?;
            }
            Err(RecvTimeoutError::Timeout) => {
                // SSE comment heartbeat — no-op for the client, but a failed
                // write here is how we notice it disconnected.
                stream
                    .write_all(b": ping\n\n")
                    .map_err(|e| format!("sse heartbeat: {e}"))?;
                stream.flush().map_err(|e| format!("sse flush: {e}"))?;
            }
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

/// Format + write one SSE `data:` event carrying the document version.
fn write_sse_event<S: Write>(stream: &mut S, version: u64) -> Result<(), String> {
    let event = format!("data: {{\"version\":{version}}}\n\n");
    stream
        .write_all(event.as_bytes())
        .map_err(|e| format!("sse write: {e}"))?;
    stream.flush().map_err(|e| format!("sse flush: {e}"))
}

#[cfg(test)]
#[path = "web_canvas_server_tests.rs"]
mod tests;
