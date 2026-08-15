//! Stdio MCP server mode for the desktop binary.
//!
//! `openpencil-desktop --mcp <path>` runs JSON-RPC stdio against a
//! `.op` file. External CLIs can spawn this mode to drive the Rust
//! editor the way they drive TS pen-mcp today.
//! The server backs the `.op` file with an `op_editor_core::
//! EditorState` (the canonical `jian_ops_schema::PenDocument`), not
//! the old shell-core `Document`. Loading a `.op` into a `PenDocument`
//! is plain `jian-ops-schema` deserialization — no `pen_doc_adapter`
//! needed for this path. Write tools apply through
//! `EditorState::apply(EditorCommand)`; on every successful write the
//! `PenDocument` is serialized straight back to disk.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::import_html_url::import_html_url_snapshot;
use op_editor_core::{EditorCommand, EditorState};
use op_mcp::import_html_tool::import_html_snapshot;
use op_mcp::import_snapshot_tool::import_web_snapshot_tool;
use op_mcp::{
    add_node_effect_snapshot, add_page_snapshot, align_selected_snapshot,
    apply_design_system_snapshot, batch_design_snapshot, batch_get_snapshot,
    clear_selection_snapshot, codegen_assemble_snapshot, codegen_clean_snapshot,
    codegen_plan_snapshot, codegen_submit_chunk_snapshot, conversion_status_snapshot,
    copy_node_snapshot, copy_selected_snapshot, count_nodes_snapshot, create_component_snapshot,
    create_variable_snapshot, cut_selected_snapshot, cycle_active_axis_value_snapshot,
    debug_tools_enabled, delete_component_snapshot, delete_node_snapshot, delete_page_snapshot,
    delete_selected_snapshot, delete_variable_snapshot, design_content_snapshot,
    design_refine_snapshot, design_skeleton_snapshot, document_info_snapshot,
    duplicate_page_snapshot, duplicate_selected_snapshot, export_design_md_snapshot,
    find_empty_space_snapshot, find_node_by_name_snapshot, get_active_theme_snapshot,
    get_canvas_bounds_snapshot, get_component_snapshot, get_design_md_snapshot,
    get_design_prompt_snapshot, get_editor_state_snapshot, get_guidelines_snapshot,
    get_history_depth_snapshot, get_node_children_snapshot, get_node_parent_snapshot,
    get_node_snapshot, get_selection_set_snapshot, get_style_guide_snapshot,
    get_style_guide_tags_snapshot, get_variables_snapshot, get_viewport_snapshot,
    group_selected_snapshot, import_svg_snapshot, insert_node_snapshot,
    instantiate_component_snapshot, lint_document_snapshot, list_components_snapshot,
    list_node_kinds_snapshot, list_pages_snapshot, list_style_guides_snapshot,
    list_theme_presets_snapshot, list_variables_snapshot, load_theme_preset_snapshot,
    move_node_snapshot, nudge_selected_snapshot, open_document_snapshot, paste_clipboard_snapshot,
    read_nodes_snapshot, redo_snapshot, remove_node_effect_snapshot, remove_page_snapshot,
    rename_component_snapshot, rename_page_snapshot, rename_variable_snapshot,
    reorder_page_snapshot, reorder_selected_snapshot, replace_all_matching_properties_snapshot,
    replace_node_snapshot, run_stdio_with_applier, save_document_snapshot,
    save_theme_preset_snapshot, search_all_unique_properties_snapshot, selection_snapshot,
    set_active_axis_value_snapshot, set_active_page_snapshot, set_active_tool_snapshot,
    set_design_md_snapshot, set_ellipse_arc_snapshot, set_node_collapsed_snapshot,
    set_node_corner_radius_snapshot, set_node_fill_hex_snapshot, set_node_flip_snapshot,
    set_node_font_size_snapshot, set_node_font_weight_snapshot, set_node_hidden_snapshot,
    set_node_locked_snapshot, set_node_name_snapshot, set_node_rotation_snapshot,
    set_node_stroke_hex_snapshot, set_node_stroke_side_width_snapshot,
    set_node_stroke_width_snapshot, set_node_text_snapshot, set_selection_set_snapshot,
    set_selection_snapshot, set_themes_snapshot, set_variable_boolean_snapshot,
    set_variable_color_snapshot, set_variable_number_snapshot, set_variable_string_snapshot,
    set_variables_snapshot, set_viewport_snapshot, snapshot_layout_snapshot, spawn_agents_snapshot,
    toggle_node_selection_snapshot, tool_search_snapshot, undo_snapshot, ungroup_selected_snapshot,
    update_node_snapshot, upsert_component_snapshot, upsert_screen_snapshot,
    upsert_variables_snapshot, McpTool, ToolRegistry,
};
#[cfg(feature = "mcp-debug-tools")]
use op_mcp::{
    debug_logs_tail_snapshot, debug_screenshot_snapshot, debug_validation_report_snapshot,
};

pub mod error;
pub mod file_path;
mod sniff;

pub use error::McpServeError;
use sniff::{sniff_id_raw, sniff_method};

/// Unwrap an MCP `tools/call` reply to the inner tool-result JSON text,
/// so tests assert on the flat result fields directly. Strips an HTTP
/// envelope first (live-server tests pass the raw HTTP response).
/// Mirrors the CLI's `unwrap_mcp_reply`. Public (not `#[cfg(test)]`) so
/// op-host-desktop's tests reach it across the crate boundary.
#[doc(hidden)]
pub fn tool_text(response: &str) -> String {
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or(response)
        .trim();
    let value: serde_json::Value = serde_json::from_str(body).expect("json-rpc reply");
    value["result"]["content"][0]["text"]
        .as_str()
        .expect("tools/call content text")
        .to_string()
}

/// Load a `.op` file into an `EditorState` via the schema compat layer.
pub fn load_editor_state(path: &Path) -> Result<EditorState, McpServeError> {
    crate::doc_io::load_editor_state(path, op_editor_core::Locale::EnUs)
        .map_err(|error| McpServeError::Document(format!("load {}: {error}", path.display())))
}

/// Serialize through the same streaming, atomic path as desktop Save.
fn save_editor_state(state: &EditorState, path: &Path) -> Result<(), McpServeError> {
    crate::doc_io::save_to_path(state, path)
        .map_err(|error| McpServeError::Document(format!("save {}: {error}", path.display())))
}

/// Process one JSON-RPC message line against the editor state.
fn process_message(
    state: &mut EditorState,
    path: &Path,
    line: &str,
) -> Result<Option<String>, McpServeError> {
    if let Some(response) = file_path::process_message_for_file_path_arg(Some(path), line)? {
        return Ok(Some(response));
    }
    let mut applier_failed: Option<String> = None;
    // File-backed mode has no live canvas for any tool call to animate —
    // the tool name is accepted (shared signature with the live-MCP path)
    // and deliberately ignored here.
    let response = process_message_with_applier(state, line, |_tool_name, state, cmd| {
        // `EditorState::apply` runs the pre-validate-then-mutate
        // discipline; `false` means the command rejected and the
        // document was NOT changed.
        if !state.apply(cmd.clone()) {
            return false;
        }
        if let Err(e) = save_editor_state(state, path) {
            applier_failed = Some(format!("save failed: {e}"));
            return false;
        }
        true
    })?;
    if let Some(msg) = applier_failed {
        eprintln!("openpencil-desktop mcp: {msg}");
    }
    Ok(response)
}

pub fn process_message_with_applier<F>(
    state: &mut EditorState,
    line: &str,
    apply: F,
) -> Result<Option<String>, McpServeError>
where
    F: FnMut(&str, &mut EditorState, &EditorCommand) -> bool,
{
    // The unrestricted profile is the whole catalog with full authority —
    // exactly what this function did before capability profiles existed, so
    // every local and managed caller is unchanged.
    process_message_with_applier_profiled(
        state,
        line,
        tool_profile::McpAccessProfile::UNRESTRICTED,
        apply,
    )
}

/// [`process_message_with_applier`] under an explicit capability profile.
///
/// The profile decides which tools the catalog advertises and which calls are
/// refused before they run. A refusal is a normal `tools/call` error envelope
/// carrying the originating request id — never a panic, and never a transport
/// error, because an MCP client has to be able to keep the session.
pub fn process_message_with_applier_profiled<F>(
    state: &mut EditorState,
    line: &str,
    profile: tool_profile::McpAccessProfile,
    mut apply: F,
) -> Result<Option<String>, McpServeError>
where
    F: FnMut(&str, &mut EditorState, &EditorCommand) -> bool,
{
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    // MCP handshake / discovery methods short-circuit the tool dispatcher.
    match sniff_method(trimmed).as_deref() {
        Some("initialize") => {
            return Ok(sniff_id_raw(trimmed).map(|id| initialize_response(&id)));
        }
        Some("tools/list") => {
            return Ok(sniff_id_raw(trimmed)
                .map(|id| tools_list_response(&id, state, debug_tools_enabled(), profile)));
        }
        Some("notifications/initialized") | Some("initialized") => {
            return Ok(None); // notification — no response required
        }
        Some("ping") => {
            return Ok(sniff_id_raw(trimmed)
                .map(|id| ping_response(&id, headless_token_from_env().as_deref())));
        }
        _ => {}
    }
    // Fall through: tools/call or legacy direct dispatch. The
    // registry snapshots `state` at build time, so it no longer
    // borrows it once the applier closure mutates it.
    let call = op_mcp::parse_tool_call(trimmed);
    // Enforcement, ahead of the registry build and therefore ahead of the
    // tool's own argument parsing: a denied tool never sees the path it was
    // asked to open.
    if let Some(call) = call.as_ref() {
        if let Some(refusal) = profile.refuse_call(&call.tool, &call.arguments) {
            // The ordinary tools/call error envelope (`isError:true`) with
            // the originating id, so a client sees a refusal it can read
            // rather than a transport failure that would drop the session.
            return Ok(Some(op_mcp::tool_response_to_json(
                &op_mcp::ToolResponse::Err {
                    id: call.id.clone(),
                    code: op_mcp::ToolErrorCode::ToolFailed,
                    message: refusal.message(&call.tool),
                },
            )));
        }
    }
    let requested_tool = call.map(|call| call.tool);
    let registry = rebuild_registry(state, requested_tool.as_deref(), profile);
    process_tool_message_with_registry(&registry, line, |tool_name, cmd| {
        apply(tool_name, state, cmd)
    })
}

/// Dispatch one already-classified tool call through a caller-provided
/// registry. Live hosts use this seam for tools whose complete snapshot is
/// much smaller than an [`EditorState`] (for example `list_pages`) or that do
/// not need state at all (`set_active_page`). The parser, command-application
/// contract, and wire serializer remain the same as the general path above.
///
/// Reports [`McpServeError::Dispatch`] — the `dispatch: ` prefix that used
/// to be baked into a `String` here is now part of that variant's `Display`
/// input, so the message is unchanged.
pub(crate) fn process_tool_message_with_registry<F>(
    registry: &ToolRegistry,
    line: &str,
    mut apply: F,
) -> Result<Option<String>, McpServeError>
where
    F: FnMut(&str, &EditorCommand) -> bool,
{
    let mut out: Vec<u8> = Vec::new();
    {
        let mut input = std::io::Cursor::new(line.as_bytes());
        run_stdio_with_applier(registry, &mut input, &mut out, |tool_name, cmd| {
            apply(tool_name, cmd)
        })
        .map_err(|e| McpServeError::Dispatch(format!("dispatch: {e}")))?;
    }
    let resp = String::from_utf8_lossy(&out).trim().to_string();
    Ok((!resp.is_empty()).then_some(resp))
}

/// Run the stdio MCP server against `path`. Returns Ok(()) on EOF,
/// Err on unrecoverable IO. Blocks the calling thread for the
/// lifetime of the stdio connection.
pub fn run(path: PathBuf) -> Result<(), McpServeError> {
    let mut state = load_editor_state(&path)?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| McpServeError::Io(format!("stdin read: {e}")))?;
        if n == 0 {
            return Ok(()); // EOF
        }
        if let Some(resp) = process_message(&mut state, &path, &line)? {
            writeln!(writer, "{resp}")
                .map_err(|e| McpServeError::Io(format!("stdout write: {e}")))?;
            writer
                .flush()
                .map_err(|e| McpServeError::Io(format!("stdout flush: {e}")))?;
        }
    }
}

/// Run the MCP server over HTTP on `127.0.0.1:port`. Each connection
/// carries one JSON-RPC message POSTed to any path; the response is
/// the JSON-RPC reply as `application/json`. A minimal non-streaming
/// Streamable-HTTP transport — enough for HTTP MCP clients that POST
/// one request per connection. Blocks for the listener's lifetime.
pub fn run_http(path: PathBuf, port: u16) -> Result<(), McpServeError> {
    // Bound a slow/stalled peer: with bodies now up to 256 MiB, a connection
    // that opens and then dribbles (or never finishes) its body must not pin
    // this thread indefinitely. The live server sets the same kind of timeout
    // on its accepted sockets (`mcp_live.rs`).
    const HTTP_IO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    let mut state = load_editor_state(&path)?;
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| McpServeError::Config(format!("bind 127.0.0.1:{port}: {e}")))?;
    eprintln!("openpencil-desktop --mcp-http: listening on 127.0.0.1:{port}");
    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                let _ = s.set_read_timeout(Some(HTTP_IO_TIMEOUT));
                let _ = s.set_write_timeout(Some(HTTP_IO_TIMEOUT));
                match serve_http_connection(&mut s, &mut state, &path) {
                    Ok(true) => {
                        eprintln!("openpencil-desktop --mcp-http: shutdown requested; exiting");
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => eprintln!("openpencil-desktop --mcp-http: {e}"),
                }
            }
            Err(e) => eprintln!("openpencil-desktop --mcp-http: accept: {e}"),
        }
    }
    Ok(())
}

/// Handle one HTTP connection: parse the request, run its JSON-RPC
/// body through [`process_message`], write the JSON-RPC reply back as
/// an `application/json` response. Generic over the stream so it is
/// unit-testable without a real socket.
/// Returns `Ok(true)` when the client requested a (token-authed) graceful
/// shutdown — the caller (`run_http`) then stops the accept loop and the
/// process exits cleanly, so `op stop` never has to signal a pid.
fn serve_http_connection<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    state: &mut EditorState,
    path: &std::path::Path,
) -> Result<bool, McpServeError> {
    // Routes through the `_with_origin` primitive with the same permissive
    // `*` value `write_mcp_http_response` supplies.
    let reply = |stream: &mut S, status: &str, body: &str| {
        write_mcp_http_response_with_origin(stream, status, body, Some("*"))
    };
    let req = read_http_request(stream)?;
    if req.method == "OPTIONS" {
        return reply(stream, "204 No Content", "").map(|()| false);
    }
    if req.path != "/mcp" && req.path != "/" {
        return reply(stream, "404 Not Found", r#"{"error":"Not found"}"#).map(|()| false);
    }
    if req.method != "POST" {
        return reply(
            stream,
            "400 Bad Request",
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"Invalid or missing session ID"},"id":null}"#,
        )
        .map(|()| false);
    }
    if let Some(id) = shutdown_request_id(&req.body, &headless_token_from_env().unwrap_or_default())
    {
        reply(stream, "200 OK", &shutdown_ok_response(&id))?;
        return Ok(true);
    }
    match process_message(state, path, &req.body)? {
        Some(response) => reply(stream, "200 OK", &response).map(|()| false),
        None => reply(stream, "202 Accepted", "").map(|()| false),
    }
}

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub body: String,
    pub host: Option<String>,
    pub origin: Option<String>,
    /// `X-OpenPencil-Token` header value, when present — the managed
    /// web-canvas daemon's per-instance auth token (see
    /// `web_canvas_server::RequestAuth`).
    pub token: Option<String>,
    /// `Content-Type` header value, when present. Browser-facing JSON routes
    /// require `application/json` so cross-origin "simple requests" (which
    /// skip the CORS preflight) cannot reach them.
    pub content_type: Option<String>,
    /// `Authorization` header value, verbatim (scheme included), when
    /// present. The multi-account online daemon reads a `Bearer <token>`
    /// out of it; every other mode ignores it.
    pub authorization: Option<String>,
    /// `Cookie` header value, verbatim, when present. The online daemon
    /// extracts its session cookie from it; every other mode ignores it.
    pub cookie: Option<String>,
    /// The raw query string (no leading `?`), when the target had one.
    ///
    /// `path` keeps its query stripped so exact-path routing is unaffected;
    /// this is captured alongside so the online daemon can read the tenant
    /// parameter. `EventSource` cannot set headers, which is why the tenant
    /// travels in the query at all — see `share_routes::TENANT_QUERY`.
    pub query: Option<String>,
}

/// Parse a capped HTTP header and then read exactly its declared body length.
/// Query strings are stripped from the path before routing.
pub fn read_http_request<S: std::io::Read>(stream: &mut S) -> Result<HttpRequest, McpServeError> {
    const MAX_HEADER: usize = 64 * 1024;
    // Whole-document sync can carry embedded images, so retain the 64 MiB
    // ceiling while reading incrementally to avoid allocating from a claimed
    // Content-Length alone.
    const MAX_BODY: usize = 64 * 1024 * 1024;
    let mut head: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = stream
            .read(&mut byte)
            .map_err(|e| McpServeError::Io(format!("http read: {e}")))?;
        if n == 0 {
            return Err(McpServeError::Protocol(
                "connection closed before headers completed".into(),
            ));
        }
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
        if head.len() > MAX_HEADER {
            return Err(McpServeError::Protocol(
                "request headers exceed 64 KiB".into(),
            ));
        }
    }
    let headers = String::from_utf8_lossy(&head);
    let mut lines = headers.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| McpServeError::Protocol("request line missing".into()))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| McpServeError::Protocol("request method missing".into()))?
        .to_ascii_uppercase();
    // Strip any `?query` from the request target so exact-path routing
    // (`/api/mcp/document`, `/mcp`, …) isn't defeated by `/api/mcp/document?x=1`.
    let target = request_parts
        .next()
        .ok_or_else(|| McpServeError::Protocol("request path missing".into()))?;
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (
            path.to_string(),
            (!query.is_empty()).then(|| query.to_string()),
        ),
        None => (target.to_string(), None),
    };
    // Parse `Content-Length` via `split_once(':')` — byte-slicing a `&str`
    // (e.g. `l[..15]`) would panic if a crafted header puts a multibyte UTF-8
    // boundary mid-slice; that panic would also bypass the live server's
    // connection-count decrement. A malformed length falls back to 0 (empty
    // body) rather than erroring.
    let declared_length = headers.lines().find_map(|l| {
        let (name, value) = l.trim().split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    });
    let content_length = declared_length.unwrap_or(0);
    // The live endpoint's browser-extension snapshot ingress is the one
    // body-carrying route reachable WITHOUT the per-instance token, so it
    // caps its body far below the endpoint-wide `MAX_BODY` — and does it
    // here, before a single body byte is read, so an untokened caller
    // cannot make this process buffer 64 MiB. See
    // `mcp_live::snapshot_ingest::MAX_SNAPSHOT_BODY`.
    if method == "POST" && path == crate::mcp_live::snapshot_ingest::SNAPSHOT_INGEST_PATH {
        let limit = crate::mcp_live::snapshot_ingest::MAX_SNAPSHOT_BODY;
        match declared_length {
            // The extension always sends `Content-Length` (it POSTs a
            // string body through `fetch`), so a missing one is not a
            // client this route has to serve — and serving it would mean
            // reading an unbounded body to find out how big it is.
            None => {
                return Err(McpServeError::Framing {
                    status: "411 Length Required",
                    message: "web snapshot ingress requires a Content-Length header".into(),
                })
            }
            Some(declared) if declared > limit => {
                return Err(McpServeError::Framing {
                    status: "413 Payload Too Large",
                    message: format!("web snapshot body exceeds {} MiB", limit / (1024 * 1024)),
                })
            }
            Some(_) => {}
        }
    }
    let credential_body_label = match path.as_str() {
        "/api/settings/credentials" => Some("credential settings"),
        "/api/ai/models/discover" => Some("model discovery"),
        _ => None,
    };
    if let Some(label) = credential_body_label
        .filter(|_| content_length > crate::web_credentials::MAX_CREDENTIAL_BODY_BYTES)
    {
        return Err(McpServeError::Protocol(format!(
            "{label} body exceeds 256 KiB"
        )));
    }
    let header_value = |wanted: &str| {
        headers.lines().skip(1).find_map(|line| {
            let (name, value) = line.trim().split_once(':')?;
            name.eq_ignore_ascii_case(wanted)
                .then(|| value.trim().to_string())
        })
    };
    let host = header_value("host");
    let origin = header_value("origin");
    let token = header_value("x-openpencil-token");
    let content_type = header_value("content-type");
    let authorization = header_value("authorization");
    let cookie = header_value("cookie");
    if content_length > MAX_BODY {
        return Err(McpServeError::Protocol(format!(
            "request body exceeds {} MiB",
            MAX_BODY / (1024 * 1024)
        )));
    }
    // Grow the body from bytes actually received, not the declared length;
    // socket timeouts bound a stalled peer.
    let mut body = Vec::with_capacity(content_length.min(64 * 1024));
    let mut remaining = content_length;
    let mut chunk = [0u8; 64 * 1024];
    while remaining > 0 {
        let want = remaining.min(chunk.len());
        let n = stream
            .read(&mut chunk[..want])
            .map_err(|e| McpServeError::Io(format!("http body read: {e}")))?;
        if n == 0 {
            return Err(McpServeError::Protocol(
                "connection closed before body completed".into(),
            ));
        }
        body.extend_from_slice(&chunk[..n]);
        remaining -= n;
    }
    Ok(HttpRequest {
        method,
        path,
        body: String::from_utf8_lossy(&body).into_owned(),
        host,
        origin,
        token,
        content_type,
        authorization,
        cookie,
        query,
    })
}

/// Compatibility wrapper for older tests/callers that only care about
/// the JSON-RPC body.
#[cfg(test)]
pub fn read_http_request_body<S: std::io::Read>(stream: &mut S) -> Result<String, McpServeError> {
    read_http_request(stream).map(|req| req.body)
}

/// Permissive-CORS (`*`) response writer — the convenience wrapper over
/// [`write_mcp_http_response_with_origin`] for every caller that does not
/// compute a per-request `Access-Control-Allow-Origin`.
pub fn write_mcp_http_response<S: std::io::Write>(
    stream: &mut S,
    status: &str,
    body: &str,
) -> Result<(), McpServeError> {
    write_mcp_http_response_with_origin(stream, status, body, Some("*"))
}

/// Like [`write_mcp_http_response`], but lets the caller supply the exact
/// `Access-Control-Allow-Origin` value to emit: `Some(origin)` echoes that
/// literal value, `None` omits the header entirely. Used by the managed
/// web-canvas daemon (`web_canvas_server::serve_one`), which computes the
/// right value per-request via `cors_origin_for` against its
/// `--allow-origin` allowlist; every other caller (the `--mcp-http`
/// server, the legacy `--serve-web` accept-loop overflow reply) keeps the
/// permissive `*` via `write_mcp_http_response` above, unchanged.
pub(crate) fn write_mcp_http_response_with_origin<S: std::io::Write>(
    stream: &mut S,
    status: &str,
    body: &str,
    cors_origin: Option<&str>,
) -> Result<(), McpServeError> {
    let cors_line = cors_origin
        .map(|origin| format!("Access-Control-Allow-Origin: {origin}\r\n"))
        .unwrap_or_default();
    let http = format!(
        "HTTP/1.1 {status}\r\n\
         {cors_line}\
         Access-Control-Allow-Methods: GET, POST, DELETE, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, mcp-session-id, X-OpenPencil-Token, Authorization\r\n\
         Access-Control-Expose-Headers: mcp-session-id\r\n\
         mcp-session-id: openpencil\r\n\
         Cache-Control: no-store\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(http.as_bytes())
        .map_err(|e| McpServeError::Io(format!("http write: {e}")))?;
    stream
        .flush()
        .map_err(|e| McpServeError::Io(format!("http flush: {e}")))
}

mod registry;
use registry::rebuild_registry;

mod wire;
pub use wire::*;

mod doc_sync;
pub use doc_sync::*;

pub(crate) mod schemas;
mod tools_list;
use tools_list::tools_list_response;
pub mod tool_profile;
#[cfg(not(feature = "mcp-debug-tools"))]
pub use schemas::TOOL_SCHEMAS;
#[cfg(feature = "mcp-debug-tools")]
pub use schemas::{DEBUG_TOOL_SCHEMAS, TOOL_SCHEMAS};

pub(crate) mod screenshot_tool;
use screenshot_tool::get_screenshot_snapshot;

pub(crate) mod export_tool;
use export_tool::export_nodes_snapshot;

pub(crate) mod export_item_tool;
use export_item_tool::export_item_snapshot;
pub(crate) mod export_deck_tool;
use export_deck_tool::export_deck_snapshot;
pub(crate) mod export_frames_tool;
use export_frames_tool::{export_frames_snapshot, get_deck_boards_snapshot};
pub(crate) mod scene_template_tools;
use scene_template_tools::{list_scene_templates_snapshot, use_scene_template_snapshot};

pub(crate) mod finalize_tool;
use finalize_tool::finalize_design_snapshot;
pub(crate) mod enrich_images_tool;
use enrich_images_tool::enrich_images_snapshot;

#[cfg(test)]
mod codegen_wire_tests;
#[cfg(test)]
mod conversion_flow_tests;
#[cfg(test)]
mod enrich_images_tool_tests;
#[cfg(test)]
mod finalize_tool_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wire_tests;
