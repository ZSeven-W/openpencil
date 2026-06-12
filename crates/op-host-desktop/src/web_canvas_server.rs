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
            // on an empty document (the web shell can then sync one in).
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

/// Run the web-canvas daemon on `host:port` (default `127.0.0.1`), backed by
/// the document at `path` (or an empty document when `None`). Serves the
/// static host page + bundle, the whole-document REST sync + health routes,
/// and falls through to the JSON-RPC `/mcp` tool dispatch (applied against
/// the in-memory document). Blocks until a token-authed shutdown request.
pub fn run_web_canvas(path: Option<PathBuf>, port: u16, host: &str) -> Result<(), String> {
    let editor = match path {
        Some(p) => crate::mcp_serve::load_editor_state(&p)?,
        None => EditorState::new(),
    };
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
