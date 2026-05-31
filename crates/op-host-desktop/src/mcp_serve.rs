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
use std::path::PathBuf;

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

/// Load a `.op` file into an `EditorState`. The `.op` format is plain
/// `jian_ops_schema::PenDocument` JSON, so the loader is a serde parse
/// via the schema's compat layer (which tolerates legacy major
/// versions + collects non-fatal warnings).
fn load_editor_state(path: &std::path::Path) -> Result<EditorState, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let loaded =
        jian_ops_schema::load_str(&src).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok(EditorState::from_document(loaded.value))
}

/// Serialize the editor state's canonical document back to `path`.
fn save_editor_state(state: &EditorState, path: &std::path::Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&state.doc)
        .map_err(|e| format!("serialize {}: {e}", path.display()))?;
    std::fs::write(path, json).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Process one JSON-RPC message line against the editor state,
/// returning the response line to send back — `None` for a
/// notification (no response per spec). Shared by the stdio and HTTP
/// transports so both speak an identical protocol.
fn process_message(
    state: &mut EditorState,
    path: &std::path::Path,
    line: &str,
) -> Result<Option<String>, String> {
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
            return Ok(sniff_id_raw(trimmed).map(|id| tools_list_response(&id, state)));
        }
        Some("notifications/initialized") | Some("initialized") => {
            return Ok(None); // notification — no response required
        }
        Some("ping") => {
            return Ok(sniff_id_raw(trimmed).map(|id| ping_response(&id)));
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
    let mut state = load_editor_state(&path)?;
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("bind 127.0.0.1:{port}: {e}"))?;
    eprintln!("openpencil-desktop --mcp-http: listening on 127.0.0.1:{port}");
    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                if let Err(e) = serve_http_connection(&mut s, &mut state, &path) {
                    eprintln!("openpencil-desktop --mcp-http: {e}");
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
fn serve_http_connection<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    state: &mut EditorState,
    path: &std::path::Path,
) -> Result<(), String> {
    let req = read_http_request(stream)?;
    if req.method == "OPTIONS" {
        return write_mcp_http_response(stream, "204 No Content", "");
    }
    if req.path != "/mcp" && req.path != "/" {
        return write_mcp_http_response(stream, "404 Not Found", r#"{"error":"Not found"}"#);
    }
    if req.method != "POST" {
        return write_mcp_http_response(
            stream,
            "400 Bad Request",
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"Invalid or missing session ID"},"id":null}"#,
        );
    }
    match process_message(state, path, &req.body)? {
        Some(response) => write_mcp_http_response(stream, "200 OK", &response),
        None => write_mcp_http_response(stream, "202 Accepted", ""),
    }
}

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
    const MAX_BODY: usize = 8 * 1024 * 1024;
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
    let path = request_parts
        .next()
        .ok_or_else(|| "request path missing".to_string())?
        .to_string();
    let content_length = headers
        .lines()
        .find_map(|l| {
            let l = l.trim();
            (l.len() >= 15 && l[..15].eq_ignore_ascii_case("content-length:"))
                .then(|| l[15..].trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    if content_length > MAX_BODY {
        return Err("request body exceeds 8 MiB".into());
    }
    let mut body = vec![0u8; content_length];
    stream
        .read_exact(&mut body)
        .map_err(|e| format!("http body read: {e}"))?;
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
    r.register(Box::new(batch_design_snapshot()));
    r.register(Box::new(get_design_prompt_snapshot(doc)));
    r.register(Box::new(design_skeleton_snapshot()));
    r.register(Box::new(design_content_snapshot()));
    r.register(Box::new(design_refine_snapshot()));
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

fn initialize_response(id_raw: &str) -> String {
    // Spec: `initialize` returns protocolVersion + capabilities +
    // serverInfo. We declare only `tools` capabilities — no
    // resources / prompts / completion are exposed yet.
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"protocolVersion":"2024-11-05","capabilities":{{"tools":{{"listChanged":false}}}},"serverInfo":{{"name":"openpencil-mcp","version":"0.1.0"}}}}}}"#
    )
}

fn ping_response(id_raw: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{}}}}"#)
}

fn tools_list_response(id_raw: &str, state: &EditorState) -> String {
    let mut entries: Vec<String> = TOOL_SCHEMAS.iter().map(|s| (*s).to_string()).collect();
    entries.extend(op_mcp::element_tools::element_tool_schemas(state));
    if debug_tools_enabled() {
        entries.extend(DEBUG_TOOL_SCHEMAS.iter().map(|s| (*s).to_string()));
    }
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"tools":[{}]}}}}"#,
        entries.join(",")
    )
}

const TOOL_SCHEMAS: &[&str] = &[
    // --- read tools ---
    r#"{"name":"open_document","description":"Connect to the current Rust MCP document and return metadata, context summary, and design prompt. filePath is accepted for TS CLI compatibility; the Rust server remains bound to the document it was started with.","inputSchema":{"type":"object","properties":{"filePath":{"type":"string","description":"Accepted for TS compatibility; use live://canvas/current server document"}}}}"#,
    r#"{"name":"save_document","description":"Save the current Rust MCP document snapshot to a .op file. Used by the Rust HTTP CLI to match TS `op save`.","inputSchema":{"type":"object","properties":{"filePath":{"type":"string","description":"Target .op file path"}},"required":["filePath"]}}"#,
    r#"{"name":"get_document_info","description":"Summarize the open document (page count, active page, etc).","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_selection","description":"Return the current selection state (ids, count).","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_node","description":"Read a node by id with depth-limited descendants.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string","description":"u64 node id"}},"required":["node_id"]}}"#,
    r#"{"name":"list_pages","description":"List page ids + names. Result includes page_count, active_page_index, ids, and names as comma-separated strings.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"list_variables","description":"List design variables with kinds.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_variables","description":"Return all design variables and theme axes as JSON strings. TS-compatible read alias for variables/theme metadata.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"save_theme_preset","description":"Save the current document themes and variables as a reusable .optheme preset file.","inputSchema":{"type":"object","properties":{"presetPath":{"type":"string","description":"Path for the output .optheme file"},"name":{"type":"string","description":"Display name for the preset; defaults to file name"}},"required":["presetPath"]}}"#,
    r#"{"name":"load_theme_preset","description":"Load a .optheme preset file and merge its themes and variables into the live document.","inputSchema":{"type":"object","properties":{"presetPath":{"type":"string","description":"Path to the .optheme file to load"}},"required":["presetPath"]}}"#,
    r#"{"name":"list_theme_presets","description":"List valid .optheme preset files in a directory.","inputSchema":{"type":"object","properties":{"directory":{"type":"string","description":"Directory to scan for .optheme files"}},"required":["directory"]}}"#,
    r#"{"name":"get_design_md","description":"Get the document design.md spec and markdown, falling back to best-effort extraction from variables and typography.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"set_design_md","description":"Import design.md markdown into the live document, or pass autoExtract=true to derive it from current variables and typography.","inputSchema":{"type":"object","properties":{"markdown":{"type":"string","description":"Raw design.md markdown"},"autoExtract":{"type":"boolean","description":"Derive design.md from the current document"}}}}"#,
    r#"{"name":"export_design_md","description":"Export design.md markdown, falling back to best-effort extraction when none is persisted.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_style_guide_tags","description":"Return all available style guide tags for filtering light/dark visual styles.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_style_guide","description":"Return a style guide by name or best tag match. Provide tags array/string, name, and optional platform.","inputSchema":{"type":"object","properties":{"tags":{"type":"array","items":{"type":"string"}},"name":{"type":"string"},"platform":{"type":"string","enum":["webapp","mobile","landing-page","slides"]}}}}"#,
    r#"{"name":"get_active_theme","description":"Return the active theme axis pinning per axis.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"list_components","description":"List registered components (saved Frames / Groups promoted via Save as Component). Returns count + a `;`-separated record of `name|id` pairs.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_component","description":"Fetch one component by id with detail: name, root node kind, and the subtree's leaf count.","inputSchema":{"type":"object","properties":{"component_id":{"type":"string","description":"positive u64 component id"}},"required":["component_id"]}}"#,
    r#"{"name":"batch_get","description":"Search and read nodes from the document. With no patterns/nodeIds, returns top-level children. Supports type/name regex patterns, nodeIds, parentId, readDepth, searchDepth, and pageId.","inputSchema":{"type":"object","properties":{"patterns":{"type":"array","items":{"type":"object","properties":{"type":{"type":"string"},"name":{"type":"string"},"reusable":{"type":"boolean"}}}},"nodeIds":{"type":"array","items":{"type":"string"}},"parentId":{"type":"string"},"readDepth":{"type":"number"},"searchDepth":{"type":"number"},"pageId":{"type":"string"},"resolve_refs":{"type":"boolean"}}}}"#,
    r#"{"name":"read_nodes","description":"Read nodes with depth control. Omit nodeIds to return top-level page children; depth=0 truncates children to \"...\", depth=-1 returns full subtrees. includeVariables=true attaches variables/themes JSON strings.","inputSchema":{"type":"object","properties":{"nodeIds":{"type":"array","items":{"type":"string"},"description":"Node ids to read; omit for top-level children"},"depth":{"type":"number","description":"0=node only, 1=direct children, -1=full subtree"},"pageId":{"type":"string"},"includeVariables":{"type":"boolean"}}}}"#,
    r#"{"name":"codegen_plan","description":"Submit a code generation plan. In file-backed Rust MCP this reports the same live-canvas requirement as TS standalone mode because pipeline state is stored in App memory.","inputSchema":{"type":"object","properties":{"plan":{"type":"object","description":"CodePlanFromAI: { chunks, sharedStyles, rootLayout }"},"filePath":{"type":"string"},"pageId":{"type":"string"}},"required":["plan"]}}"#,
    r#"{"name":"codegen_submit_chunk","description":"Submit generated code for one chunk. In file-backed Rust MCP this reports the same live-canvas requirement as TS standalone mode.","inputSchema":{"type":"object","properties":{"planId":{"type":"string"},"result":{"type":"object","description":"ChunkResult: { chunkId, code, contract }"},"status":{"type":"string","enum":["failed","skipped"]}},"required":["planId","result"]}}"#,
    r#"{"name":"codegen_assemble","description":"Retrieve all chunk results for final assembly. In file-backed Rust MCP this reports the same live-canvas requirement as TS standalone mode.","inputSchema":{"type":"object","properties":{"planId":{"type":"string"},"framework":{"type":"string","enum":["react","vue","svelte","html","flutter","swiftui","compose","react-native"]}},"required":["planId","framework"]}}"#,
    r#"{"name":"codegen_clean","description":"Manually clean up an abandoned codegen plan. Idempotent; without live canvas state returns ok=true and deleted=false.","inputSchema":{"type":"object","properties":{"planId":{"type":"string"}},"required":["planId"]}}"#,
    r#"{"name":"search_all_unique_properties","description":"Recursively search unique style property values under the provided parent node ids. Result `properties` is a JSON object keyed by requested property names.","inputSchema":{"type":"object","properties":{"parents":{"type":"array","items":{"type":"string"},"description":"Parent node ids to search; descendants and the parent itself are included"},"properties":{"type":"array","items":{"type":"string","enum":["fillColor","textColor","strokeColor","strokeThickness","cornerRadius","padding","gap","fontSize","fontFamily","fontWeight"]}},"pageId":{"type":"string"},"filePath":{"type":"string","description":"Accepted for TS compatibility; live Rust MCP uses the server document"}},"required":["parents","properties"]}}"#,
    r#"{"name":"replace_all_matching_properties","description":"Recursively replace matching style property values under parent node ids. Returns replacedCount and applies one bulk edit when matches exist.","inputSchema":{"type":"object","properties":{"parents":{"type":"array","items":{"type":"string"}},"properties":{"type":"object","description":"property -> array of {from,to} replacement rules"},"pageId":{"type":"string"},"filePath":{"type":"string","description":"Accepted for TS compatibility; live Rust MCP uses the server document"}},"required":["parents","properties"]}}"#,
    r#"{"name":"snapshot_layout","description":"Return a depth-limited layout snapshot. Result `layout` is a `;`-separated record of `id|x|y|w|h` (ints, doc-px).","inputSchema":{"type":"object","properties":{"parentId":{"type":"string"},"maxDepth":{"type":"string","description":"u32 depth, default 1 when arguments are present"},"pageId":{"type":"string"}}}}"#,
    r#"{"name":"find_empty_space","description":"Find padded empty canvas space in one direction for placing new content.","inputSchema":{"type":"object","properties":{"width":{"type":"string","description":"i32 doc-px"},"height":{"type":"string","description":"i32 doc-px"},"padding":{"type":"string","description":"i32 doc-px, default 50"},"direction":{"type":"string","enum":["top","right","bottom","left"]},"nodeId":{"type":"string"},"pageId":{"type":"string"}},"required":["width","height","direction"]}}"#,
    r#"{"name":"get_canvas_bounds","description":"Return the union bounding box of every top-level node on the active page (x/y/w/h ints + has_content true/false).","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"find_node_by_name","description":"Locate the first node whose name matches (case-sensitive, exact) anywhere on the active page. Returns id + kind. ToolFailed when no match.","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}}"#,
    r#"{"name":"get_node_parent","description":"Return the parent id of node_id on the active page. parent_id=0 means the node is at the page root. depth is distance from root.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string","description":"positive u64 node id"}},"required":["node_id"]}}"#,
    r#"{"name":"get_node_children","description":"List the immediate children of a node. Returns count + comma-separated ids + per-child (child_<i>_id/kind/name/x/y/width/height). Known leaves and empty containers return count=0 (NOT an error). Only an unknown node_id returns ToolFailed.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string","description":"positive u64 node id"}},"required":["node_id"]}}"#,
    r#"{"name":"count_nodes","description":"Return total node count across all pages + a per-page breakdown. Result `per_page` is `;`-separated `index|count` records.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"list_node_kinds","description":"Return a per-kind histogram of nodes on the active page (frame/group/rect/ellipse/polygon/line/text/path/other). Result `kinds` is `;`-separated `kind|count` records.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_history_depth","description":"Return undo + redo stack sizes. Useful before bulk rollback to know how many steps are available.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_viewport","description":"Return current canvas pan + zoom. pan_x/pan_y are i32 doc-px; zoom_percent is the zoom * 100 as int.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_selection_set","description":"Return every id in the multi-select set (vs get_selection which returns only the anchor). Result: count + comma-separated ids + anchor.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"clear_selection","description":"Drop the current multi-select. No args.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"set_selection","description":"Set selection to a single node by id (scoped to the ACTIVE page only). Rejects unknown ids and ids that live on a non-active page — switch the active page first with set_active_page.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string","description":"positive u64 node id on the active page"}},"required":["node_id"]}}"#,
    r#"{"name":"set_viewport","description":"Set canvas pan + zoom. Pass any subset of pan_x / pan_y / zoom_percent — omitted axes are left unchanged. zoom_percent clamps to [10, 2000].","inputSchema":{"type":"object","properties":{"pan_x":{"type":"string","description":"i32 doc-px"},"pan_y":{"type":"string","description":"i32 doc-px"},"zoom_percent":{"type":"string","description":"int * 100 (100 == 1.0×)"}}}}"#,
    r#"{"name":"set_node_hidden","description":"Toggle a node's visibility (layer-panel eye icon). value is \"true\" to hide.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"value":{"type":"string","enum":["true","false"]}},"required":["node_id","value"]}}"#,
    r#"{"name":"set_node_locked","description":"Toggle a node's lock (layer-panel padlock icon). value is \"true\" to lock.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"value":{"type":"string","enum":["true","false"]}},"required":["node_id","value"]}}"#,
    r#"{"name":"set_node_collapsed","description":"Toggle a node's layer-panel disclosure state. value is \"true\" to collapse.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"value":{"type":"string","enum":["true","false"]}},"required":["node_id","value"]}}"#,
    r#"{"name":"set_active_tool","description":"Change the active canvas tool (left toolbar). Accepts select / rect / ellipse / polygon / line / pen / text / frame / hand.","inputSchema":{"type":"object","properties":{"tool":{"type":"string","enum":["select","rect","ellipse","polygon","line","pen","text","frame","hand"]}},"required":["tool"]}}"#,
    r#"{"name":"undo","description":"Pop the last history snapshot. Returns false when the past stack is empty.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"redo","description":"Push the last undone snapshot back. Returns false when the redo stack is empty.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"duplicate_selected","description":"Duplicate the currently-selected node and select the clone. Optional offset_px shifts the clone (default 10).","inputSchema":{"type":"object","properties":{"offset_px":{"type":"string","description":"i32 doc-px shift (default 10)"}}}}"#,
    r#"{"name":"delete_selected","description":"Delete the currently-selected node. Returns false when nothing is selected. Undoable.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"nudge_selected","description":"Translate the currently-selected node by (dx, dy) doc-px. Both 0 rejects.","inputSchema":{"type":"object","properties":{"dx":{"type":"string","description":"i32 doc-px"},"dy":{"type":"string","description":"i32 doc-px"}},"required":["dx","dy"]}}"#,
    r#"{"name":"group_selected","description":"Wrap the multi-selected siblings in a new Group. Cmd+G equivalent. Rejects when selection is empty / single / spans parents.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"ungroup_selected","description":"Replace the selected Group with its children. Cmd+Shift+G equivalent. Rejects when anchor is not a Group.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"reorder_selected","description":"Move the currently-selected node forward (\"up\") or back (\"down\") in z-order. Mirrors layer-panel [ / ].","inputSchema":{"type":"object","properties":{"direction":{"type":"string","enum":["up","down"]}},"required":["direction"]}}"#,
    r#"{"name":"set_node_rotation","description":"Set node rotation in degrees on a node by id.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"degrees":{"type":"string","description":"finite f32 rotation in degrees"}},"required":["node_id","degrees"]}}"#,
    r#"{"name":"set_node_text","description":"Set text content on a Text-kind node by id. Rejects non-Text kinds.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"text":{"type":"string"}},"required":["node_id","text"]}}"#,
    r#"{"name":"set_node_corner_radius","description":"Set corner-radius (non-negative doc-px) on a node by id. Honored at paint time for Rect / Frame; other kinds accept the write but the radius is invisible.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"radius":{"type":"string","description":"non-negative finite f32"}},"required":["node_id","radius"]}}"#,
    r#"{"name":"set_node_font_size","description":"Set font size (positive finite doc-px) on a Text-kind node by id. Rejects non-Text kinds.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"font_size":{"type":"string","description":"positive finite f32 doc-px"}},"required":["node_id","font_size"]}}"#,
    r#"{"name":"set_node_font_weight","description":"Set OpenType font weight (1..=1000) on a Text-kind node by id. Rejects non-Text kinds and out-of-range weights.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"font_weight":{"type":"string","description":"u16 in 1..=1000 (e.g. 400=Regular, 700=Bold)"}},"required":["node_id","font_weight"]}}"#,
    r##"{"name":"set_node_stroke_hex","description":"Set the stroke color on a node. Existing stroke gets its color overwritten; missing stroke gets a fresh 1 doc-px stroke attached at the parsed color so the change is visible immediately.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"hex":{"type":"string","description":"#rgb / #rrggbb / #rrggbbaa"}},"required":["node_id","hex"]}}"##,
    r#"{"name":"set_node_stroke_width","description":"Set the stroke width (doc-px) on a node. width=0 clears the stroke; width>0 on a node without an existing stroke attaches a fresh black-default stroke at that width.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"width":{"type":"string","description":"non-negative finite f32 doc-px"}},"required":["node_id","width"]}}"#,
    r#"{"name":"align_selected","description":"Align or distribute the current multi-selection. Mirrors the PropertyPanel Align section. Distribute variants silently no-op for fewer than 3 selected nodes.","inputSchema":{"type":"object","properties":{"action":{"type":"string","description":"one of left, center_h, right, top, center_v, bottom, distribute_h, distribute_v"}},"required":["action"]}}"#,
    r##"{"name":"set_node_fill_hex","description":"Set the fill color on a node by id. Sister tool to set_node_stroke_hex; one-call color change without the other update_node fields.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"hex":{"type":"string","description":"#rgb / #rrggbb / #rrggbbaa"}},"required":["node_id","hex"]}}"##,
    r#"{"name":"set_node_flip","description":"Mirror a node horizontally / vertically. Pass any subset of flip_x / flip_y as \"true\"/\"false\"; omitted axes are left unchanged. At least one axis is required.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"flip_x":{"type":"string","enum":["true","false"]},"flip_y":{"type":"string","enum":["true","false"]}},"required":["node_id"]}}"#,
    r#"{"name":"set_ellipse_arc","description":"Set arc geometry on an Ellipse node: start_angle / sweep_angle (degrees) carve a pie/arc, inner_radius (0.0..=1.0 fraction) carves a donut hole. Pass any subset; omitted fields are left unchanged. Rejects non-Ellipse kinds.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"start_angle":{"type":"string","description":"finite degrees"},"sweep_angle":{"type":"string","description":"finite degrees"},"inner_radius":{"type":"string","description":"0.0..=1.0 fraction"}},"required":["node_id"]}}"#,
    r#"{"name":"add_node_effect","description":"Append a visual effect to a node with default parameters. kind is shadow / blur / background_blur. Frame/Group/Rectangle and the leaf shapes accept effects; IconFont/Ref do not.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"kind":{"type":"string","enum":["shadow","blur","background_blur"]}},"required":["node_id","kind"]}}"#,
    r#"{"name":"remove_node_effect","description":"Remove the effect at a 0-based index from a node's effect list. The list is cleared once empty.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"index":{"type":"string","description":"0-based u32 effect index"}},"required":["node_id","index"]}}"#,
    r#"{"name":"set_node_name","description":"Rename a node by id. Empty names (after trim) are rejected so the LayerPanel never shows blank rows.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"name":{"type":"string"}},"required":["node_id","name"]}}"#,
    r#"{"name":"set_selection_set","description":"Replace the multi-selection (active page only) with the supplied comma-separated node_ids. Empty list clears the selection. Unknown ids AND ids that live on a non-active page drop silently.","inputSchema":{"type":"object","properties":{"node_ids":{"type":"string","description":"comma-separated positive u64 active-page node ids; empty string clears"}},"required":["node_ids"]}}"#,
    r#"{"name":"toggle_node_selection","description":"Shift-click parity: toggle node_id in the multi-selection (scoped to the ACTIVE page only). Already-selected ⇒ remove (anchor reassigns to last surviving id); otherwise add as new anchor. Rejects unknown ids and ids that live on a non-active page.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string","description":"positive u64 node id on the active page"}},"required":["node_id"]}}"#,
    r#"{"name":"cycle_active_axis_value","description":"Advance the active value for a theme axis to its next entry (wrapping back to the first). Seeds the axis to its first value when nothing is set. Rejects unknown axes and axes whose values list is empty.","inputSchema":{"type":"object","properties":{"axis":{"type":"string","description":"theme axis name (e.g. \"mode\", \"density\")"}},"required":["axis"]}}"#,
    r#"{"name":"copy_selected","description":"Cmd+C parity. Deep-clones the active-page selection into the document's internal clipboard. Apply-time false when nothing is selected.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"cut_selected","description":"Cmd+X parity. Copies the selection then deletes it. History snapshot pushed so undo restores both clipboard and tree.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"paste_clipboard","description":"Cmd+V parity. Pastes the document clipboard as top-level siblings on the active page, offset by offset_px doc-px (defaults to 10). Mints fresh ids past max_node_id(). Replaces selection with the new ids. Apply-time false when the clipboard is empty or id-space is exhausted.","inputSchema":{"type":"object","properties":{"offset_px":{"type":"string","description":"i32 doc-px offset; defaults to 10 when omitted"}},"required":[]}}"#,
    // --- write tools ---
    r##"{"name":"set_variable_color","description":"Set a Color-kind variable's value.","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"hex":{"type":"string","description":"#rgb / #rrggbb / #rrggbbaa"}},"required":["name","hex"]}}"##,
    r#"{"name":"batch_design","description":"Insert design content. Accepts either nodes_json (JSON array string of {kind,name,x,y,width,height,fill_hex?}) or operations (TS batch_design DSL with I(parent,nodeJson) inserts for nested PenNode trees).","inputSchema":{"type":"object","properties":{"nodes_json":{"type":"string","description":"JSON array of simple leaf descriptors"},"operations":{"type":"string","description":"TS batch_design DSL, e.g. root=I(null,{...}) then child=I(root,{...})"}}}}"#,
    r#"{"name":"get_design_prompt","description":"Get OpenPencil design-generation prompt knowledge. Pass section for a focused subset; omit it for all sections. style and design-md are derived from the live document's design.md when present.","inputSchema":{"type":"object","properties":{"section":{"type":"string","description":"Prompt section name, e.g. all, layout, style, design-md, elements, codegen-react"}},"required":[]}}"#,
    r#"{"name":"design_skeleton","description":"Layered design workflow phase 1: insert structural scaffolding. Accepts nodes_json or operations, same as batch_design; response carries phase=skeleton.","inputSchema":{"type":"object","properties":{"nodes_json":{"type":"string","description":"JSON array of simple leaf descriptors"},"operations":{"type":"string","description":"TS batch_design DSL with I(parent,nodeJson) inserts"}}}}"#,
    r#"{"name":"design_content","description":"Layered design workflow phase 2: fill content into the scaffold. Accepts nodes_json or operations, same as batch_design; response carries phase=content.","inputSchema":{"type":"object","properties":{"nodes_json":{"type":"string","description":"JSON array of simple leaf descriptors"},"operations":{"type":"string","description":"TS batch_design DSL with I(parent,nodeJson) inserts"}}}}"#,
    r#"{"name":"design_refine","description":"Layered design workflow phase 3: polish details. Accepts nodes_json or operations, same as batch_design; response carries phase=refine.","inputSchema":{"type":"object","properties":{"nodes_json":{"type":"string","description":"JSON array of simple leaf descriptors"},"operations":{"type":"string","description":"TS batch_design DSL with I(parent,nodeJson) inserts"}}}}"#,
    r#"{"name":"set_variable_number","description":"Set a Number-kind variable's value (decimal, may be negative or fractional).","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string"}},"required":["name","value"]}}"#,
    r#"{"name":"set_variable_string","description":"Set a String-kind variable's value (free-form text).","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string"}},"required":["name","value"]}}"#,
    r#"{"name":"set_variable_boolean","description":"Set a Boolean-kind variable's value (\"true\" or \"false\").","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string","enum":["true","false"]}},"required":["name","value"]}}"#,
    r#"{"name":"set_variables","description":"Add/update or replace the document variables map. Accepts TS-style variables object and optional replace boolean.","inputSchema":{"type":"object","properties":{"variables":{"type":"object","description":"name -> { type, value } variable definitions"},"replace":{"type":"boolean","description":"Replace all variables instead of merging"}},"required":["variables"]}}"#,
    r#"{"name":"set_themes","description":"Add/update or replace theme axes. Accepts TS-style themes object and optional replace boolean.","inputSchema":{"type":"object","properties":{"themes":{"type":"object","description":"axis name -> variant names array"},"replace":{"type":"boolean","description":"Replace all theme axes instead of merging"}},"required":["themes"]}}"#,
    r##"{"name":"create_variable","description":"Create a new design-token variable. kind is color/number/boolean/string; default_value is parsed per kind (hex for color, decimal for number, true/false for boolean, free text for string). Rejects empty/duplicate names and bad defaults.","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"kind":{"type":"string","enum":["color","number","boolean","string"]},"default_value":{"type":"string","description":"hex / decimal / true|false / text per kind"}},"required":["name","kind","default_value"]}}"##,
    r#"{"name":"delete_variable","description":"Delete a design-token variable by name. Also drops any node $ref pointing at it. Rejects unknown names.","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}}"#,
    r#"{"name":"rename_variable","description":"Rename a design-token variable and rewrite every node $ref pointing at it. Rejects unknown old_name, empty new_name, or a new_name colliding with a different variable.","inputSchema":{"type":"object","properties":{"old_name":{"type":"string"},"new_name":{"type":"string"}},"required":["old_name","new_name"]}}"#,
    r#"{"name":"instantiate_component","description":"Drop a clone of a registered component's root subtree onto the active page. component_id is the id returned by list_components.","inputSchema":{"type":"object","properties":{"component_id":{"type":"string","description":"positive u64 component id"}},"required":["component_id"]}}"#,
    r#"{"name":"create_component","description":"Promote an existing Frame or Group on the active page to a registered component. Use list_components afterwards to see it, instantiate_component to drop a clone.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string","description":"positive u64 node id"},"name":{"type":"string"}},"required":["node_id","name"]}}"#,
    r#"{"name":"delete_component","description":"Remove a component from the registry by id. Live instances already on the page are NOT affected — they're independent clones.","inputSchema":{"type":"object","properties":{"component_id":{"type":"string","description":"positive u64 component id"}},"required":["component_id"]}}"#,
    r#"{"name":"rename_component","description":"Rename a registered component. Name must be non-empty / non-whitespace.","inputSchema":{"type":"object","properties":{"component_id":{"type":"string","description":"positive u64 component id"},"name":{"type":"string"}},"required":["component_id","name"]}}"#,
    r#"{"name":"set_active_page","description":"Switch which page is the active target for subsequent inserts / batch_design / design_* commands. index is 0-based.","inputSchema":{"type":"object","properties":{"index":{"type":"string","description":"0-based page index"}},"required":["index"]}}"#,
    r#"{"name":"add_page","description":"Append a fresh empty page and switch the active page to it. Optional name mirrors the TS MCP page tool. Returns false on id-space exhaustion or an empty name.","inputSchema":{"type":"object","properties":{"name":{"type":"string","description":"Page name (default: Page N)"}}}}"#,
    r#"{"name":"rename_page","description":"Set a page's display name. Accepts TS-style pageId or legacy index. Name must be non-empty / non-whitespace.","inputSchema":{"type":"object","properties":{"pageId":{"type":"string","description":"Page id returned by list_pages"},"index":{"type":"string","description":"0-based page index (legacy)"},"name":{"type":"string"}},"required":["name"]}}"#,
    r#"{"name":"delete_page","description":"Remove a page by index or pageId. The applier keeps the active page valid (clamps if needed).","inputSchema":{"type":"object","properties":{"pageId":{"type":"string","description":"Page id returned by list_pages"},"index":{"type":"string","description":"0-based page index (legacy)"}}}}"#,
    r#"{"name":"remove_page","description":"TS-compatible alias for delete_page. Removes a page by pageId or index.","inputSchema":{"type":"object","properties":{"pageId":{"type":"string","description":"Page id returned by list_pages"},"index":{"type":"string","description":"0-based page index"}}}}"#,
    r#"{"name":"duplicate_page","description":"Clone a page and switch the active page to the clone. Accepts TS-style pageId or legacy index; optional name overrides the clone name.","inputSchema":{"type":"object","properties":{"pageId":{"type":"string","description":"Page id returned by list_pages"},"index":{"type":"string","description":"0-based page index (legacy)"},"name":{"type":"string","description":"Optional clone name"}}}}"#,
    r#"{"name":"reorder_page","description":"Move a page. Accepts TS-style pageId + index or legacy from + to. Target is clamped to [0, page_count).","inputSchema":{"type":"object","properties":{"pageId":{"type":"string","description":"Page id returned by list_pages"},"index":{"type":"string","description":"0-based target page index"},"from":{"type":"string","description":"0-based source page index (legacy)"},"to":{"type":"string","description":"0-based target page index (legacy)"}}}}"#,
    r#"{"name":"set_active_axis_value","description":"Pin a theme axis to one of its allowed values.","inputSchema":{"type":"object","properties":{"axis":{"type":"string"},"value":{"type":"string"}},"required":["axis","value"]}}"#,
    r#"{"name":"insert_node","description":"Create a new leaf node on the active page.","inputSchema":{"type":"object","properties":{"kind":{"type":"string","enum":["frame","group","rect","ellipse","polygon","line","text","path"]},"name":{"type":"string"},"x":{"type":"string"},"y":{"type":"string"},"width":{"type":"string"},"height":{"type":"string"},"fill_hex":{"type":"string"}},"required":["kind","name","x","y","width","height"]}}"#,
    r#"{"name":"update_node","description":"Patch fields on an existing node. Pass any subset of x/y/width/height/name/fill_hex.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"x":{"type":"string"},"y":{"type":"string"},"width":{"type":"string"},"height":{"type":"string"},"name":{"type":"string"},"fill_hex":{"type":"string"}},"required":["node_id"]}}"#,
    r#"{"name":"import_svg","description":"Parse an SVG document and insert the resulting nodes on the active page. Supports rect/circle/ellipse/line/polyline/polygon and path (M/L/H/V/C/S/Q/T/Z); <g>/transforms/CSS are skipped. x/y (optional, default 0) offset the imported nodes in doc-px.","inputSchema":{"type":"object","properties":{"svg":{"type":"string","description":"SVG document text"},"x":{"type":"string","description":"i32 doc-px x offset (default 0)"},"y":{"type":"string","description":"i32 doc-px y offset (default 0)"}},"required":["svg"]}}"#,
    r#"{"name":"delete_node","description":"Remove a node + descendants from its parent.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"}},"required":["node_id"]}}"#,
    r#"{"name":"move_node","description":"Reparent a node. target_parent_id=0 puts it at the active page root.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"target_parent_id":{"type":"string"}},"required":["node_id","target_parent_id"]}}"#,
    r#"{"name":"copy_node","description":"Deep-clone a subtree with fresh ids under a new parent.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"target_parent_id":{"type":"string"}},"required":["node_id","target_parent_id"]}}"#,
    r#"{"name":"replace_node","description":"Swap an existing node at the same parent slot with a freshly-built leaf. Set drop_children=true to discard a container's subtree.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"kind":{"type":"string","enum":["frame","group","rect","ellipse","polygon","line","text","path"]},"name":{"type":"string"},"x":{"type":"string"},"y":{"type":"string"},"width":{"type":"string"},"height":{"type":"string"},"fill_hex":{"type":"string"},"drop_children":{"type":"string","enum":["true","false"]}},"required":["node_id","kind","name","x","y","width","height"]}}"#,
];

const DEBUG_TOOL_SCHEMAS: &[&str] = &[
    r#"{"name":"debug_validation_report","description":"Run the op-design-lint detectors over the active page and return the design-issue list. Read-only, no parameters. Result: count + categories (`;`-separated `category|count`) + issues (JSON-serialized Issue array). Gated behind the OPENPENCIL_DEBUG_TOOLS=1 env flag.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"debug_logs_tail","description":"Read the tail of ~/.openpencil/logs/server-YYYY-MM-DD.log with API keys and Authorization headers redacted. Gated behind OPENPENCIL_DEBUG_TOOLS=1.","inputSchema":{"type":"object","properties":{"tailLines":{"type":"number","description":"Maximum lines to return (default 100, max 500)."},"sinceMs":{"type":"number","description":"Unix ms timestamp; only return lines newer than this."},"grep":{"type":"string","description":"Regex to filter lines by content after redaction."}}}}"#,
    r#"{"name":"debug_screenshot","description":"Capture a PNG screenshot of the live canvas via the renderer. File-backed Rust MCP reports the same no-live-canvas error as TS standalone mode. Gated behind OPENPENCIL_DEBUG_TOOLS=1.","inputSchema":{"type":"object","properties":{"target":{"type":"string","enum":["node","root"]},"nodeId":{"type":"string","description":"Required when target=node."},"padding":{"type":"number"},"dpr":{"type":"number"},"timeoutMs":{"type":"number","description":"Default 15000, max 60000."}},"required":["target"]}}"#,
];

#[cfg(test)]
mod tests;
