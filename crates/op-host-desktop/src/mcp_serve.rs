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

use op_editor_core::{EditorCommand, EditorState};
use op_mcp::{
    add_node_effect_snapshot, add_page_snapshot, align_selected_snapshot, batch_design_snapshot,
    batch_get_snapshot, clear_selection_snapshot, codegen_assemble_snapshot,
    codegen_clean_snapshot, codegen_plan_snapshot, codegen_submit_chunk_snapshot,
    copy_node_snapshot, copy_selected_snapshot, count_nodes_snapshot, create_component_snapshot,
    create_variable_snapshot, cut_selected_snapshot, cycle_active_axis_value_snapshot,
    debug_logs_tail_snapshot, debug_screenshot_snapshot, debug_tools_enabled,
    debug_validation_report_snapshot, delete_component_snapshot, delete_node_snapshot,
    delete_page_snapshot, delete_selected_snapshot, delete_variable_snapshot,
    design_content_snapshot, design_refine_snapshot, design_skeleton_snapshot,
    document_info_snapshot, duplicate_page_snapshot, duplicate_selected_snapshot,
    export_design_md_snapshot, find_empty_space_snapshot, find_node_by_name_snapshot,
    get_active_theme_snapshot, get_canvas_bounds_snapshot, get_component_snapshot,
    get_design_md_snapshot, get_design_prompt_snapshot, get_history_depth_snapshot,
    get_node_children_snapshot, get_node_parent_snapshot, get_node_snapshot,
    get_selection_set_snapshot, get_style_guide_snapshot, get_style_guide_tags_snapshot,
    get_variables_snapshot, get_viewport_snapshot, group_selected_snapshot, import_svg_snapshot,
    insert_node_snapshot, instantiate_component_snapshot, list_components_snapshot,
    list_node_kinds_snapshot, list_pages_snapshot, list_theme_presets_snapshot,
    list_variables_snapshot, load_theme_preset_snapshot, move_node_snapshot,
    nudge_selected_snapshot, open_document_snapshot, paste_clipboard_snapshot, read_nodes_snapshot,
    redo_snapshot, remove_node_effect_snapshot, remove_page_snapshot, rename_component_snapshot,
    rename_page_snapshot, rename_variable_snapshot, reorder_page_snapshot,
    reorder_selected_snapshot, replace_all_matching_properties_snapshot, replace_node_snapshot,
    run_stdio_with_applier, save_document_snapshot, save_theme_preset_snapshot,
    search_all_unique_properties_snapshot, selection_snapshot, set_active_axis_value_snapshot,
    set_active_page_snapshot, set_active_tool_snapshot, set_design_md_snapshot,
    set_ellipse_arc_snapshot, set_node_collapsed_snapshot, set_node_corner_radius_snapshot,
    set_node_fill_hex_snapshot, set_node_flip_snapshot, set_node_font_size_snapshot,
    set_node_font_weight_snapshot, set_node_hidden_snapshot, set_node_locked_snapshot,
    set_node_name_snapshot, set_node_rotation_snapshot, set_node_stroke_hex_snapshot,
    set_node_stroke_width_snapshot, set_node_text_snapshot, set_selection_set_snapshot,
    set_selection_snapshot, set_themes_snapshot, set_variable_boolean_snapshot,
    set_variable_color_snapshot, set_variable_number_snapshot, set_variable_string_snapshot,
    set_variables_snapshot, set_viewport_snapshot, snapshot_layout_snapshot,
    toggle_node_selection_snapshot, undo_snapshot, ungroup_selected_snapshot, update_node_snapshot,
    ToolRegistry,
};

pub(crate) mod file_path;

/// Load a `.op` file into an `EditorState` via the schema compat layer.
pub(crate) fn load_editor_state(path: &Path) -> Result<EditorState, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let loaded =
        jian_ops_schema::load_str(&src).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok(EditorState::from_document(loaded.value))
}

/// Serialize the editor state's canonical document back to `path`.
fn save_editor_state(state: &EditorState, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&state.doc)
        .map_err(|e| format!("serialize {}: {e}", path.display()))?;
    std::fs::write(path, json).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Process one JSON-RPC message line against the editor state.
fn process_message(
    state: &mut EditorState,
    path: &Path,
    line: &str,
) -> Result<Option<String>, String> {
    if let Some(response) = file_path::process_message_for_file_path_arg(Some(path), line)? {
        return Ok(Some(response));
    }
    let mut applier_failed: Option<String> = None;
    let response = process_message_with_applier(state, line, |state, cmd| {
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

pub(crate) fn process_message_with_applier<F>(
    state: &mut EditorState,
    line: &str,
    mut apply: F,
) -> Result<Option<String>, String>
where
    F: FnMut(&mut EditorState, &EditorCommand) -> bool,
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
                .map(|id| tools_list_response(&id, state, debug_tools_enabled())));
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
    let registry = rebuild_registry(state);
    let mut out: Vec<u8> = Vec::new();
    {
        let mut input = std::io::Cursor::new(line.as_bytes());
        run_stdio_with_applier(&registry, &mut input, &mut out, |cmd| apply(state, cmd))
            .map_err(|e| format!("dispatch: {e}"))?;
    }
    let resp = String::from_utf8_lossy(&out).trim().to_string();
    Ok((!resp.is_empty()).then_some(resp))
}

/// Run the stdio MCP server against `path`. Returns Ok(()) on EOF,
/// Err on unrecoverable IO. Blocks the calling thread for the
/// lifetime of the stdio connection.
/// If argv requests an MCP server mode, run it and return `true` —
/// the caller (`main`) should then exit. Returns `false` for normal
/// GUI mode. Exits the process on a malformed invocation.
///
/// - `--mcp <path>` — JSON-RPC stdio MCP server backed by `<path>`.
/// - `--mcp-http <port> <path>` — Streamable-HTTP MCP server.
///
/// External CLIs (Claude Code / Codex / Gemini / Copilot) spawn the
/// binary in these modes to drive the Rust editor the same way they
/// drive TS pen-mcp.
pub fn run_cli_if_requested() -> bool {
    let mut args = std::env::args().skip(1);
    let Some(first) = args.next() else {
        return false;
    };
    if first == "--mcp" {
        let Some(path) = args.next() else {
            eprintln!("openpencil-desktop --mcp: missing <path> arg");
            std::process::exit(2);
        };
        if let Err(e) = run(PathBuf::from(path)) {
            eprintln!("openpencil-desktop --mcp: {e}");
            std::process::exit(1);
        }
        return true;
    }
    if first == "--mcp-http" {
        let Some(port_arg) = args.next() else {
            eprintln!("openpencil-desktop --mcp-http: missing <port> arg");
            std::process::exit(2);
        };
        let Ok(port) = port_arg.parse::<u16>() else {
            eprintln!("openpencil-desktop --mcp-http: <port> must be a u16, got {port_arg:?}");
            std::process::exit(2);
        };
        let Some(path) = args.next() else {
            eprintln!("openpencil-desktop --mcp-http: missing <path> arg");
            std::process::exit(2);
        };
        if let Err(e) = run_http(PathBuf::from(path), port) {
            eprintln!("openpencil-desktop --mcp-http: {e}");
            std::process::exit(1);
        }
        return true;
    }
    if first == "--serve-web" {
        let Some(port_arg) = args.next() else {
            eprintln!("openpencil-desktop --serve-web: missing <port> arg");
            std::process::exit(2);
        };
        let Ok(port) = port_arg.parse::<u16>() else {
            eprintln!("openpencil-desktop --serve-web: <port> must be a u16, got {port_arg:?}");
            std::process::exit(2);
        };
        // The document path is optional — without it the daemon starts on an
        // empty document (the web shell can then sync one in).
        let path = args.next().map(PathBuf::from);
        if let Err(e) = crate::web_canvas_server::run_web_canvas(path, port) {
            eprintln!("openpencil-desktop --serve-web: {e}");
            std::process::exit(1);
        }
        return true;
    }
    // Unknown leading arg → fall through to GUI mode for now.
    false
}

pub fn run(path: PathBuf) -> Result<(), String> {
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
            .map_err(|e| format!("stdin read: {e}"))?;
        if n == 0 {
            return Ok(()); // EOF
        }
        if let Some(resp) = process_message(&mut state, &path, &line)? {
            writeln!(writer, "{resp}").map_err(|e| format!("stdout write: {e}"))?;
            writer.flush().map_err(|e| format!("stdout flush: {e}"))?;
        }
    }
}

/// Run the MCP server over HTTP on `127.0.0.1:port`. Each connection
/// carries one JSON-RPC message POSTed to any path; the response is
/// the JSON-RPC reply as `application/json`. A minimal non-streaming
/// Streamable-HTTP transport — enough for HTTP MCP clients that POST
/// one request per connection. Blocks for the listener's lifetime.
pub fn run_http(path: PathBuf, port: u16) -> Result<(), String> {
    // Bound a slow/stalled peer: with bodies now up to 256 MiB, a connection
    // that opens and then dribbles (or never finishes) its body must not pin
    // this thread indefinitely. The live server sets the same kind of timeout
    // on its accepted sockets (`mcp_live.rs`).
    const HTTP_IO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    let mut state = load_editor_state(&path)?;
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("bind 127.0.0.1:{port}: {e}"))?;
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
) -> Result<bool, String> {
    let req = read_http_request(stream)?;
    if req.method == "OPTIONS" {
        return write_mcp_http_response(stream, "204 No Content", "").map(|()| false);
    }
    if req.path != "/mcp" && req.path != "/" {
        return write_mcp_http_response(stream, "404 Not Found", r#"{"error":"Not found"}"#)
            .map(|()| false);
    }
    if req.method != "POST" {
        return write_mcp_http_response(
            stream,
            "400 Bad Request",
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"Invalid or missing session ID"},"id":null}"#,
        )
        .map(|()| false);
    }
    if let Some(id) = shutdown_request_id(&req.body, &headless_token_from_env().unwrap_or_default())
    {
        write_mcp_http_response(stream, "200 OK", &shutdown_ok_response(&id))?;
        return Ok(true);
    }
    match process_message(state, path, &req.body)? {
        Some(response) => write_mcp_http_response(stream, "200 OK", &response).map(|()| false),
        None => write_mcp_http_response(stream, "202 Accepted", "").map(|()| false),
    }
}

#[derive(Debug)]
pub(crate) struct HttpRequest {
    pub method: String,
    pub path: String,
    pub body: String,
}

/// Read an HTTP request off `stream`. Reads to the `\r\n\r\n` header
/// terminator, parses the request line + `Content-Length`, then reads
/// exactly that many body bytes. The header block is capped so a
/// malformed peer can't exhaust memory.
pub(crate) fn read_http_request<S: std::io::Read>(stream: &mut S) -> Result<HttpRequest, String> {
    const MAX_HEADER: usize = 64 * 1024;
    // Body ceiling. A whole-document live sync (`/api/mcp/document`), or an
    // `insert_node` carrying an embedded base64 image, is legitimately large —
    // realistic image-heavy designs run to tens of MiB, so the old 8 MiB cap
    // rejected them. 64 MiB comfortably accepts realistic documents while
    // bounding peak memory: the doc-sync path parses the body into a
    // `serde_json::Value` and reserializes the inner document before the
    // canonical load, so actual peak is a few× the body — a 64 MiB ceiling
    // keeps that within a sane envelope for this localhost, single-user server.
    // The body is read incrementally (below), so a lying `Content-Length` can't
    // force an up-front allocation.
    const MAX_BODY: usize = 64 * 1024 * 1024;
    let mut head: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = stream
            .read(&mut byte)
            .map_err(|e| format!("http read: {e}"))?;
        if n == 0 {
            return Err("connection closed before headers completed".into());
        }
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
        if head.len() > MAX_HEADER {
            return Err("request headers exceed 64 KiB".into());
        }
    }
    let headers = String::from_utf8_lossy(&head);
    let mut lines = headers.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "request line missing".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "request method missing".to_string())?
        .to_ascii_uppercase();
    // Strip any `?query` from the request target so exact-path routing
    // (`/api/mcp/document`, `/mcp`, …) isn't defeated by `/api/mcp/document?x=1`.
    let path = request_parts
        .next()
        .ok_or_else(|| "request path missing".to_string())?
        .split('?')
        .next()
        .unwrap_or("")
        .to_string();
    // Parse `Content-Length` via `split_once(':')` — byte-slicing a `&str`
    // (e.g. `l[..15]`) would panic if a crafted header puts a multibyte UTF-8
    // boundary mid-slice; that panic would also bypass the live server's
    // connection-count decrement. A malformed length falls back to 0 (empty
    // body) rather than erroring.
    let content_length = headers
        .lines()
        .find_map(|l| {
            let (name, value) = l.trim().split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    if content_length > MAX_BODY {
        return Err(format!(
            "request body exceeds {} MiB",
            MAX_BODY / (1024 * 1024)
        ));
    }
    // Read the body incrementally so memory tracks bytes ACTUALLY received,
    // not the peer-declared Content-Length: a lying or oversized length can't
    // force a big up-front allocation — the buffer grows only as bytes arrive,
    // and a stalled peer trips the socket read timeout instead of pinning a
    // thread indefinitely.
    let mut body = Vec::with_capacity(content_length.min(64 * 1024));
    let mut remaining = content_length;
    let mut chunk = [0u8; 64 * 1024];
    while remaining > 0 {
        let want = remaining.min(chunk.len());
        let n = stream
            .read(&mut chunk[..want])
            .map_err(|e| format!("http body read: {e}"))?;
        if n == 0 {
            return Err("connection closed before body completed".into());
        }
        body.extend_from_slice(&chunk[..n]);
        remaining -= n;
    }
    Ok(HttpRequest {
        method,
        path,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

/// Compatibility wrapper for older tests/callers that only care about
/// the JSON-RPC body.
#[cfg(test)]
pub(crate) fn read_http_request_body<S: std::io::Read>(stream: &mut S) -> Result<String, String> {
    read_http_request(stream).map(|req| req.body)
}

pub(crate) fn write_mcp_http_response<S: std::io::Write>(
    stream: &mut S,
    status: &str,
    body: &str,
) -> Result<(), String> {
    let http = format!(
        "HTTP/1.1 {status}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, DELETE, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, mcp-session-id\r\n\
         Access-Control-Expose-Headers: mcp-session-id\r\n\
         mcp-session-id: openpencil\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(http.as_bytes())
        .map_err(|e| format!("http write: {e}"))?;
    stream.flush().map_err(|e| format!("http flush: {e}"))
}

fn rebuild_registry(doc: &EditorState) -> ToolRegistry {
    let mut r = ToolRegistry::default();
    for tool in op_mcp::element_tools::insert_kit_component_tools(doc) {
        r.register(Box::new(tool));
    }
    r.register(Box::new(open_document_snapshot(doc)));
    r.register(Box::new(save_document_snapshot(doc)));
    r.register(Box::new(document_info_snapshot(doc)));
    r.register(Box::new(selection_snapshot(doc)));
    r.register(Box::new(get_node_snapshot(doc)));
    r.register(Box::new(list_pages_snapshot(doc)));
    r.register(Box::new(list_variables_snapshot(doc)));
    r.register(Box::new(get_variables_snapshot(doc)));
    r.register(Box::new(save_theme_preset_snapshot(doc)));
    r.register(Box::new(load_theme_preset_snapshot()));
    r.register(Box::new(list_theme_presets_snapshot()));
    r.register(Box::new(get_design_md_snapshot(doc)));
    r.register(Box::new(set_design_md_snapshot(doc)));
    r.register(Box::new(export_design_md_snapshot(doc)));
    r.register(Box::new(get_style_guide_tags_snapshot()));
    r.register(Box::new(get_style_guide_snapshot()));
    r.register(Box::new(get_active_theme_snapshot(doc)));
    r.register(Box::new(list_components_snapshot(doc)));
    r.register(Box::new(get_component_snapshot(doc)));
    r.register(Box::new(batch_get_snapshot(doc)));
    r.register(Box::new(read_nodes_snapshot(doc)));
    r.register(Box::new(codegen_plan_snapshot()));
    r.register(Box::new(codegen_submit_chunk_snapshot()));
    r.register(Box::new(codegen_assemble_snapshot()));
    r.register(Box::new(codegen_clean_snapshot()));
    r.register(Box::new(search_all_unique_properties_snapshot(doc)));
    r.register(Box::new(replace_all_matching_properties_snapshot(doc)));
    r.register(Box::new(snapshot_layout_snapshot(doc)));
    r.register(Box::new(find_empty_space_snapshot(doc)));
    r.register(Box::new(get_canvas_bounds_snapshot(doc)));
    r.register(Box::new(find_node_by_name_snapshot(doc)));
    r.register(Box::new(get_node_parent_snapshot(doc)));
    r.register(Box::new(get_node_children_snapshot(doc)));
    r.register(Box::new(count_nodes_snapshot(doc)));
    r.register(Box::new(list_node_kinds_snapshot(doc)));
    r.register(Box::new(get_history_depth_snapshot(doc)));
    r.register(Box::new(get_viewport_snapshot(doc)));
    r.register(Box::new(get_selection_set_snapshot(doc)));
    if debug_tools_enabled() {
        r.register(Box::new(debug_validation_report_snapshot(doc)));
        r.register(Box::new(debug_logs_tail_snapshot()));
        r.register(Box::new(debug_screenshot_snapshot()));
    }
    r.register(Box::new(clear_selection_snapshot()));
    r.register(Box::new(set_selection_snapshot()));
    r.register(Box::new(set_viewport_snapshot()));
    r.register(Box::new(set_node_hidden_snapshot()));
    r.register(Box::new(set_node_locked_snapshot()));
    r.register(Box::new(set_node_collapsed_snapshot()));
    r.register(Box::new(set_active_tool_snapshot()));
    r.register(Box::new(undo_snapshot()));
    r.register(Box::new(redo_snapshot()));
    r.register(Box::new(duplicate_selected_snapshot()));
    r.register(Box::new(delete_selected_snapshot()));
    r.register(Box::new(nudge_selected_snapshot()));
    r.register(Box::new(group_selected_snapshot()));
    r.register(Box::new(ungroup_selected_snapshot()));
    r.register(Box::new(reorder_selected_snapshot()));
    r.register(Box::new(set_node_rotation_snapshot()));
    r.register(Box::new(set_node_text_snapshot()));
    r.register(Box::new(set_node_corner_radius_snapshot()));
    r.register(Box::new(set_node_font_size_snapshot()));
    r.register(Box::new(set_node_font_weight_snapshot()));
    r.register(Box::new(set_node_stroke_hex_snapshot()));
    r.register(Box::new(set_node_stroke_width_snapshot()));
    r.register(Box::new(align_selected_snapshot()));
    r.register(Box::new(set_node_fill_hex_snapshot()));
    r.register(Box::new(set_node_flip_snapshot()));
    r.register(Box::new(set_ellipse_arc_snapshot()));
    r.register(Box::new(add_node_effect_snapshot()));
    r.register(Box::new(remove_node_effect_snapshot()));
    r.register(Box::new(set_node_name_snapshot()));
    r.register(Box::new(set_selection_set_snapshot()));
    r.register(Box::new(toggle_node_selection_snapshot()));
    r.register(Box::new(cycle_active_axis_value_snapshot(doc)));
    r.register(Box::new(copy_selected_snapshot()));
    r.register(Box::new(cut_selected_snapshot()));
    r.register(Box::new(paste_clipboard_snapshot()));
    r.register(Box::new(set_variable_color_snapshot(doc)));
    r.register(Box::new(set_active_axis_value_snapshot(doc)));
    r.register(Box::new(insert_node_snapshot()));
    r.register(Box::new(import_svg_snapshot()));
    r.register(Box::new(update_node_snapshot()));
    r.register(Box::new(delete_node_snapshot()));
    r.register(Box::new(move_node_snapshot()));
    r.register(Box::new(copy_node_snapshot()));
    r.register(Box::new(replace_node_snapshot()));
    r.register(Box::new(batch_design_snapshot(doc)));
    r.register(Box::new(get_design_prompt_snapshot(doc)));
    r.register(Box::new(design_skeleton_snapshot()));
    r.register(Box::new(design_content_snapshot()));
    r.register(Box::new(design_refine_snapshot(doc)));
    r.register(Box::new(set_variable_number_snapshot(doc)));
    r.register(Box::new(set_variable_string_snapshot(doc)));
    r.register(Box::new(set_variable_boolean_snapshot(doc)));
    r.register(Box::new(set_variables_snapshot()));
    r.register(Box::new(set_themes_snapshot()));
    r.register(Box::new(create_variable_snapshot(doc)));
    r.register(Box::new(delete_variable_snapshot(doc)));
    r.register(Box::new(rename_variable_snapshot(doc)));
    r.register(Box::new(instantiate_component_snapshot()));
    r.register(Box::new(create_component_snapshot()));
    r.register(Box::new(delete_component_snapshot()));
    r.register(Box::new(rename_component_snapshot()));
    r.register(Box::new(set_active_page_snapshot()));
    r.register(Box::new(add_page_snapshot()));
    r.register(Box::new(rename_page_snapshot(doc)));
    r.register(Box::new(delete_page_snapshot(doc)));
    r.register(Box::new(remove_page_snapshot(doc)));
    r.register(Box::new(duplicate_page_snapshot(doc)));
    r.register(Box::new(reorder_page_snapshot(doc)));
    r
}

/// Cheap top-level "method" field extractor. Returns the unquoted
/// string value; None if the field is missing or unparseable.
/// Walks the line key by key so a nested or string-valued
/// "method" in another field can't shadow the real top-level
/// method (mirrors `arguments_field`'s discipline in shell-core).
fn sniff_method(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    // Skip past the leading `{` if present.
    let mut i = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'{' {
        return None;
    }
    i += 1;
    walk_top_level_for_string_value(bytes, &mut i, "method")
}

/// Return the JSON token (verbatim — with quotes if string) that
/// follows `"id":` at the top level. Preserves the original
/// representation so the response carries the same id type the
/// client sent.
fn sniff_id_raw(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'{' {
        return None;
    }
    i += 1;
    walk_top_level_for_raw_value(bytes, &mut i, "id")
}

/// Generic top-level key walker — returns the value for `target`
/// when seen at depth 0 of the object body starting at `*i`.
/// `string_only` extracts the inner contents (without quotes);
/// the verbatim variant returns the full literal.
fn walk_top_level_for_string_value(bytes: &[u8], i: &mut usize, target: &str) -> Option<String> {
    walk_top_level(bytes, i, target, /*string_only=*/ true)
}

fn walk_top_level_for_raw_value(bytes: &[u8], i: &mut usize, target: &str) -> Option<String> {
    walk_top_level(bytes, i, target, /*string_only=*/ false)
}

fn walk_top_level(bytes: &[u8], i: &mut usize, target: &str, string_only: bool) -> Option<String> {
    loop {
        // Skip whitespace + commas.
        while *i < bytes.len() && (bytes[*i].is_ascii_whitespace() || bytes[*i] == b',') {
            *i += 1;
        }
        if *i >= bytes.len() || bytes[*i] == b'}' {
            return None;
        }
        if bytes[*i] != b'"' {
            return None;
        }
        *i += 1;
        let key_start = *i;
        while *i < bytes.len() && bytes[*i] != b'"' {
            if bytes[*i] == b'\\' {
                *i = i.saturating_add(2);
            } else {
                *i += 1;
            }
        }
        if *i >= bytes.len() {
            return None;
        }
        let key = std::str::from_utf8(&bytes[key_start..*i]).ok()?;
        *i += 1;
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if *i >= bytes.len() || bytes[*i] != b':' {
            return None;
        }
        *i += 1;
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if *i >= bytes.len() {
            return None;
        }
        let val_start = *i;
        match bytes[*i] {
            b'"' => {
                *i += 1;
                let inner_start = *i;
                while *i < bytes.len() && bytes[*i] != b'"' {
                    if bytes[*i] == b'\\' {
                        *i = i.saturating_add(2);
                    } else {
                        *i += 1;
                    }
                }
                if *i >= bytes.len() {
                    return None;
                }
                let inner_end = *i;
                *i += 1;
                if key == target {
                    if string_only {
                        return std::str::from_utf8(&bytes[inner_start..inner_end])
                            .ok()
                            .map(|s| s.to_string());
                    } else {
                        return std::str::from_utf8(&bytes[val_start..*i])
                            .ok()
                            .map(|s| s.to_string());
                    }
                }
            }
            b'{' | b'[' => {
                // Walk past structured value, depth-tracked.
                let open = bytes[*i];
                let close = if open == b'{' { b'}' } else { b']' };
                let mut depth = 1i32;
                *i += 1;
                let mut in_str = false;
                let mut escape = false;
                while *i < bytes.len() && depth > 0 {
                    let c = bytes[*i];
                    if in_str {
                        if escape {
                            escape = false;
                        } else if c == b'\\' {
                            escape = true;
                        } else if c == b'"' {
                            in_str = false;
                        }
                    } else if c == b'"' {
                        in_str = true;
                    } else if c == open {
                        depth += 1;
                    } else if c == close {
                        depth -= 1;
                    }
                    *i += 1;
                }
                if key == target {
                    // Structured value where caller asked for a
                    // scalar. Treat as absent — caller may fall
                    // back to a default response shape.
                    return None;
                }
            }
            _ => {
                while *i < bytes.len()
                    && !matches!(bytes[*i], b',' | b'}' | b' ' | b'\t' | b'\n' | b'\r')
                {
                    *i += 1;
                }
                if key == target {
                    return std::str::from_utf8(&bytes[val_start..*i])
                        .ok()
                        .map(|s| s.to_string());
                }
            }
        }
    }
}

mod wire;
pub(crate) use wire::*;

mod doc_sync;
pub(crate) use doc_sync::*;

fn tools_list_response(id_raw: &str, state: &EditorState, debug_enabled: bool) -> String {
    let mut entries: Vec<String> = TOOL_SCHEMAS.iter().map(|s| (*s).to_string()).collect();
    entries.extend(op_mcp::element_tools::element_tool_schemas(state));
    if debug_enabled {
        entries.extend(DEBUG_TOOL_SCHEMAS.iter().map(|s| (*s).to_string()));
    }
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"tools":[{}]}}}}"#,
        entries.join(",")
    )
}

mod schemas;
pub(crate) use schemas::{DEBUG_TOOL_SCHEMAS, TOOL_SCHEMAS};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod wire_tests;
