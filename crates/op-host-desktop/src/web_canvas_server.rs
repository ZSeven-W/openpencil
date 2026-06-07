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
//! (`run_web_canvas`, behind `openpencil-desktop --serve-web <port> [doc]`).
//! The browser-coupled pieces — an SSE endpoint that streams `version` bumps to
//! connected shells and static serving of the WASM bundle — layer on top and
//! are verified against the running shell.

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

/// Run the web-canvas daemon on `127.0.0.1:port`, backed by the document at
/// `path` (or an empty document when `None`). Serves the whole-document REST
/// sync + health routes and falls through to the JSON-RPC `/mcp` tool dispatch
/// (applied against the in-memory document). Blocks for the listener's lifetime.
pub fn run_web_canvas(path: Option<PathBuf>, port: u16) -> Result<(), String> {
    let editor = match path {
        Some(p) => crate::mcp_serve::load_editor_state(&p)?,
        None => EditorState::new(),
    };
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("bind 127.0.0.1:{port}: {e}"))?;
    let bound = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    eprintln!("openpencil-desktop --serve-web: listening on 127.0.0.1:{bound}");
    // Shared across connection threads: the document authority (one writer at a
    // time via the Mutex) + the SSE broadcast hub. Thread-per-connection so a
    // long-lived SSE stream (or a slow client) never blocks other clients.
    let state = Arc::new(Mutex::new(WebCanvasState::new(editor, bound)));
    let hub = Arc::new(SseHub::default());
    let conn_count = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
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
        let spawned = thread::Builder::new()
            .name("op-serve-web-conn".into())
            .spawn(move || {
                let _conn_guard = ConnGuard(conns);
                let _ = s.set_read_timeout(Some(IO_TIMEOUT));
                let _ = s.set_write_timeout(Some(IO_TIMEOUT));
                if let Err(e) = serve_one(&mut s, &state, &hub) {
                    eprintln!("openpencil-desktop --serve-web: {e}");
                }
            });
        if spawned.is_err() {
            conn_count.fetch_sub(1, Ordering::AcqRel);
        }
    }
    Ok(())
}

/// Handle one connection. Routes: SSE live-update stream (`GET
/// /api/mcp/events`); REST whole-doc sync / health (`/api/*` via
/// [`handle_web_canvas_request`]); else JSON-RPC `/mcp` tool dispatch. A
/// mutation (REST POST or a mutating tool call) bumps the version and is
/// broadcast to SSE subscribers. The state `Mutex` is held only across the
/// in-memory operation, never across the (long-lived) SSE wait.
fn serve_one<S: Read + Write>(
    stream: &mut S,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
) -> Result<(), String> {
    let req = crate::mcp_serve::read_http_request(stream)?;
    if req.method == "OPTIONS" {
        return crate::mcp_serve::write_mcp_http_response(stream, "204 No Content", "");
    }
    // SSE live-update stream: the browser shell subscribes and re-syncs whenever
    // the document version advances. Subscribe BEFORE reading the current
    // version so no broadcast is missed (a duplicate is harmless — versions are
    // monotonic). The state lock is released before the long SSE wait.
    if req.method == "GET" && req.path == "/api/mcp/events" {
        let rx = hub.subscribe();
        let current = state.lock().unwrap_or_else(|p| p.into_inner()).version;
        return serve_sse(stream, rx, current);
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
                .map_err(|e| format!("ai stream error: {e}"));
        };
        let provider = {
            let guard = state.lock().unwrap_or_else(|p| p.into_inner());
            crate::ai_proxy::proxy_provider(&guard.editor, &ai_req.model)
        };
        let Some(provider) = provider else {
            return crate::ai_proxy::write_sse_error(stream, "no model configured")
                .map_err(|e| format!("ai stream error: {e}"));
        };
        return crate::ai_proxy::stream_ai_response(stream, ai_req, provider.as_ref())
            .map_err(|e| format!("ai stream: {e}"));
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
        return crate::mcp_serve::write_mcp_http_response(stream, reply.status, &reply.body);
    }
    // JSON-RPC tool dispatch is served ONLY as a POST to `/` or `/mcp`. An
    // unknown path is 404; a known path with the wrong method (e.g. `GET /mcp`)
    // is 405 — never silently dispatched as a tool call.
    let is_jsonrpc_path = req.path == "/" || req.path == "/mcp";
    if !is_jsonrpc_path {
        return crate::mcp_serve::write_mcp_http_response(
            stream,
            "404 Not Found",
            r#"{"ok":false,"error":"Not found. Use /api/mcp/document, /api/mcp/server, /api/mcp/events, or /mcp."}"#,
        );
    }
    if req.method != "POST" {
        return crate::mcp_serve::write_mcp_http_response(
            stream,
            "405 Method Not Allowed",
            r#"{"ok":false,"error":"Method not allowed. POST a JSON-RPC message to /mcp."}"#,
        );
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
    crate::mcp_serve::write_mcp_http_response(stream, status, &response)
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
mod tests {
    use super::*;

    fn fresh_state() -> WebCanvasState {
        WebCanvasState::new(EditorState::new(), 3100)
    }

    // A minimal canonical document body in the TS `setSyncDocument` shape.
    const SYNC_BODY: &str = r##"{"document":{"version":"1.0.0","children":[{"id":"n9","type":"rectangle","name":"Synced Rect","x":1,"y":2,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]},"sourceClientId":"web"}"##;

    #[test]
    fn server_health_matches_ts_running_port_shape() {
        let r = handle_web_canvas_request("GET", "/api/mcp/server", "", &mut fresh_state());
        assert!(r.status.starts_with("200"));
        // TS `server.get.ts` parity: clients test `running` + `port`.
        assert!(r.body.contains(r#""running":true"#));
        assert!(r.body.contains(r#""port":3100"#));
        assert!(r.body.contains(r#""localIp":"#));
        assert!(r.body.contains(r#""server":"openpencil-mcp""#));
    }

    #[test]
    fn get_document_returns_doc_and_version() {
        let r = handle_web_canvas_request("GET", "/api/mcp/document", "", &mut fresh_state());
        assert!(r.status.starts_with("200"));
        assert!(r.body.contains(r#""document":"#));
        assert!(r.body.contains(r#""version":0"#));
    }

    #[test]
    fn post_document_replaces_doc_and_bumps_version() {
        use op_editor_core::PenNodeExt;
        let mut s = fresh_state();
        let r = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut s);
        assert!(r.status.starts_with("200"), "{}", r.body);
        assert!(r.body.contains(r#""ok":true"#));
        assert!(r.body.contains(r#""version":1"#));
        // The in-memory document was replaced with the synced tree.
        assert!(s
            .editor
            .active_children()
            .iter()
            .any(|n| n.base().name.as_deref() == Some("Synced Rect")));
        // A second sync bumps the version again (monotonic).
        let r2 = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut s);
        assert!(r2.body.contains(r#""version":2"#));
    }

    #[test]
    fn post_document_rejects_invalid_body_with_400() {
        let mut s = fresh_state();
        let r = handle_web_canvas_request("POST", "/api/mcp/document", r#"{"nope":1}"#, &mut s);
        assert!(r.status.starts_with("400"));
        assert!(r.body.contains("Missing document in request body"));
        // A rejected sync must not bump the version.
        assert_eq!(s.version, 0);
    }

    #[test]
    fn unknown_route_404s() {
        let r = handle_web_canvas_request("DELETE", "/whatever", "", &mut fresh_state());
        assert!(r.status.starts_with("404"));
    }

    #[test]
    fn get_ai_models_returns_json_array() {
        // The AI proxy model list is served as a JSON array — empty
        // when nothing is configured, but always well-formed JSON the
        // web bundle can `JSON.parse`.
        let r = handle_web_canvas_request("GET", "/api/ai/models", "", &mut fresh_state());
        assert!(r.status.starts_with("200"));
        let parsed: serde_json::Value =
            serde_json::from_str(&r.body).expect("models body is valid JSON");
        assert!(
            parsed.is_array(),
            "models body must be a JSON array: {}",
            r.body
        );
    }

    // --- serve_one routing (socket-level, via a mock stream) ---

    struct MockStream {
        input: std::io::Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl std::io::Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl std::io::Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Drive one request through `serve_one` and return the raw HTTP response.
    fn serve(method: &str, path: &str, body: &str) -> String {
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let mut stream = MockStream {
            input: std::io::Cursor::new(request.into_bytes()),
            output: Vec::new(),
        };
        let state = Mutex::new(fresh_state());
        let hub = SseHub::default();
        serve_one(&mut stream, &state, &hub).expect("serve_one");
        String::from_utf8_lossy(&stream.output).into_owned()
    }

    fn mock_stream(request: &str) -> MockStream {
        MockStream {
            input: std::io::Cursor::new(request.as_bytes().to_vec()),
            output: Vec::new(),
        }
    }

    #[test]
    fn sse_hub_broadcasts_version_to_all_subscribers() {
        let hub = SseHub::default();
        let a = hub.subscribe();
        let b = hub.subscribe();
        hub.broadcast(5);
        assert_eq!(a.recv().unwrap(), 5);
        assert_eq!(b.recv().unwrap(), 5);
    }

    #[test]
    fn sse_hub_prunes_disconnected_subscribers() {
        let hub = SseHub::default();
        let live = hub.subscribe();
        drop(hub.subscribe()); // a disconnected client (receiver dropped)
        assert_eq!(hub.subscriber_count(), 2);
        hub.broadcast(1); // prunes the dropped one
        assert_eq!(hub.subscriber_count(), 1);
        assert_eq!(live.recv().unwrap(), 1);
    }

    #[test]
    fn write_sse_event_emits_data_frame() {
        let mut stream = mock_stream("");
        write_sse_event(&mut stream, 42).expect("write");
        assert_eq!(
            String::from_utf8_lossy(&stream.output),
            "data: {\"version\":42}\n\n"
        );
    }

    #[test]
    fn serve_sse_emits_initial_then_each_version_until_hub_drops() {
        let (tx, rx) = mpsc::channel();
        tx.send(9).expect("send"); // one bump, then the sender drops → Disconnected
        drop(tx);
        let mut stream = mock_stream("");
        serve_sse(&mut stream, rx, 7).expect("serve_sse");
        let out = String::from_utf8_lossy(&stream.output);
        assert!(out.contains("text/event-stream"), "{out}");
        assert!(out.contains(r#"data: {"version":7}"#), "{out}"); // initial sync
        assert!(out.contains(r#"data: {"version":9}"#), "{out}"); // broadcast bump
    }

    #[test]
    fn serve_one_post_document_broadcasts_new_version_to_sse() {
        let state = Mutex::new(fresh_state());
        let hub = SseHub::default();
        let sub = hub.subscribe();
        let request = format!(
            "POST /api/mcp/document HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{}",
            SYNC_BODY.len(),
            SYNC_BODY
        );
        let mut stream = mock_stream(&request);
        serve_one(&mut stream, &state, &hub).expect("serve_one");
        // The whole-doc sync bumped the version to 1 and broadcast it.
        assert_eq!(sub.recv().unwrap(), 1);
    }

    #[test]
    fn serve_one_routes_rest_health_and_document() {
        assert!(serve("GET", "/api/mcp/server", "").contains("200 OK"));
        assert!(serve("GET", "/api/mcp/document", "").contains("200 OK"));
        let post = serve("POST", "/api/mcp/document", SYNC_BODY);
        assert!(post.contains("200 OK"), "{post}");
        assert!(post.contains(r#""ok":true"#));
    }

    #[test]
    fn serve_one_query_string_does_not_break_rest_routing() {
        // The query string must be stripped before exact-path routing.
        assert!(serve("GET", "/api/mcp/server?v=2", "").contains("200 OK"));
    }

    #[test]
    fn serve_one_post_mcp_dispatches_jsonrpc() {
        let r = serve(
            "POST",
            "/mcp",
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
        );
        assert!(r.contains("200 OK"), "{r}");
    }

    #[test]
    fn serve_one_get_mcp_is_405_not_a_tool_call() {
        let r = serve("GET", "/mcp", "");
        assert!(r.contains("405 Method Not Allowed"), "{r}");
    }

    #[test]
    fn serve_one_unknown_path_is_404() {
        let r = serve("GET", "/favicon.ico", "");
        assert!(r.contains("404 Not Found"), "{r}");
    }

    #[test]
    fn serve_one_unimplemented_api_route_is_404_not_jsonrpc() {
        // An `/api/mcp/*` route this daemon doesn't implement must 404, not
        // fall through to JSON-RPC dispatch.
        let r = serve("GET", "/api/mcp/selection", "");
        assert!(r.contains("404 Not Found"), "{r}");
    }
}
