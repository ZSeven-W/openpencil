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
    instantiate_component_snapshot, list_components_snapshot, list_node_kinds_snapshot,
    list_pages_snapshot, list_theme_presets_snapshot, list_variables_snapshot,
    load_theme_preset_snapshot, move_node_snapshot, nudge_selected_snapshot,
    open_document_snapshot, paste_clipboard_snapshot, read_nodes_snapshot, redo_snapshot,
    remove_node_effect_snapshot, remove_page_snapshot, rename_component_snapshot,
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
    set_variables_snapshot, set_viewport_snapshot, snapshot_layout_snapshot, spawn_agents_snapshot,
    toggle_node_selection_snapshot, tool_search_snapshot, undo_snapshot, ungroup_selected_snapshot,
    update_node_snapshot, McpTool, ToolRegistry,
};
#[cfg(feature = "mcp-debug-tools")]
use op_mcp::{
    debug_logs_tail_snapshot, debug_screenshot_snapshot, debug_validation_report_snapshot,
};

pub mod file_path;

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
pub fn load_editor_state(path: &Path) -> Result<EditorState, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let loaded = op_pen_loader::load_canonical(&src)
        .map_err(|e| format!("parse {}: {e}", path.display()))?;
    for warning in &loaded.warnings {
        eprintln!("openpencil-desktop mcp: schema warning: {warning:?}");
    }
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

pub fn process_message_with_applier<F>(
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
    let requested_tool = op_mcp::parse_tool_call(trimmed).map(|call| call.tool);
    let registry = rebuild_registry(state, requested_tool.as_deref());
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
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub body: String,
}

/// Read an HTTP request off `stream`. Reads to the `\r\n\r\n` header
/// terminator, parses the request line + `Content-Length`, then reads
/// exactly that many body bytes. The header block is capped so a
/// malformed peer can't exhaust memory.
pub fn read_http_request<S: std::io::Read>(stream: &mut S) -> Result<HttpRequest, String> {
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
pub fn read_http_request_body<S: std::io::Read>(stream: &mut S) -> Result<String, String> {
    read_http_request(stream).map(|req| req.body)
}

pub fn write_mcp_http_response<S: std::io::Write>(
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

fn rebuild_registry(doc: &EditorState, requested_tool: Option<&str>) -> ToolRegistry {
    let mut r = ToolRegistry::default();

    macro_rules! register_tool {
        ($name:literal, $tool:expr) => {
            if should_register(requested_tool, $name) {
                r.register(Box::new($tool));
            }
        };
    }

    if requested_tool
        .map(|name| name.starts_with("add_"))
        .unwrap_or(true)
    {
        for tool in op_mcp::element_tools::insert_kit_component_tools(doc) {
            if should_register(requested_tool, tool.name()) {
                r.register(Box::new(tool));
            }
        }
    }
    register_tool!("open_document", open_document_snapshot(doc));
    register_tool!("save_document", save_document_snapshot(doc));
    register_tool!("get_document_info", document_info_snapshot(doc));
    register_tool!("get_selection", selection_snapshot(doc));
    register_tool!("get_node", get_node_snapshot(doc));
    register_tool!("list_pages", list_pages_snapshot(doc));
    register_tool!("list_variables", list_variables_snapshot(doc));
    register_tool!("get_variables", get_variables_snapshot(doc));
    register_tool!("save_theme_preset", save_theme_preset_snapshot(doc));
    register_tool!("load_theme_preset", load_theme_preset_snapshot());
    register_tool!("list_theme_presets", list_theme_presets_snapshot());
    register_tool!("get_design_md", get_design_md_snapshot(doc));
    register_tool!("set_design_md", set_design_md_snapshot(doc));
    register_tool!("export_design_md", export_design_md_snapshot(doc));
    register_tool!("get_style_guide_tags", get_style_guide_tags_snapshot());
    register_tool!("get_style_guide", get_style_guide_snapshot());
    register_tool!("get_guidelines", get_guidelines_snapshot());
    // Phase 0: always register spawn_agents (validates + returns request result).
    // Actual parallel execution is deferred to Phase 3 (Task 3.1).
    register_tool!("spawn_agents", spawn_agents_snapshot());
    register_tool!("ToolSearch", tool_search_snapshot(schemas::TOOL_SCHEMAS));
    register_tool!("get_screenshot", get_screenshot_snapshot(doc));
    register_tool!("export_nodes", export_nodes_snapshot(doc));
    register_tool!("get_active_theme", get_active_theme_snapshot(doc));
    register_tool!("list_components", list_components_snapshot(doc));
    register_tool!("get_component", get_component_snapshot(doc));
    register_tool!("batch_get", batch_get_snapshot(doc));
    register_tool!("read_nodes", read_nodes_snapshot(doc));
    register_tool!("codegen_plan", codegen_plan_snapshot(doc));
    register_tool!("codegen_submit_chunk", codegen_submit_chunk_snapshot());
    register_tool!("codegen_assemble", codegen_assemble_snapshot());
    register_tool!("codegen_clean", codegen_clean_snapshot());
    register_tool!(
        "search_all_unique_properties",
        search_all_unique_properties_snapshot(doc)
    );
    register_tool!(
        "replace_all_matching_properties",
        replace_all_matching_properties_snapshot(doc)
    );
    register_tool!("snapshot_layout", snapshot_layout_snapshot(doc));
    register_tool!("find_empty_space", find_empty_space_snapshot(doc));
    register_tool!("get_canvas_bounds", get_canvas_bounds_snapshot(doc));
    register_tool!("find_node_by_name", find_node_by_name_snapshot(doc));
    register_tool!("get_node_parent", get_node_parent_snapshot(doc));
    register_tool!("get_node_children", get_node_children_snapshot(doc));
    register_tool!("count_nodes", count_nodes_snapshot(doc));
    register_tool!("list_node_kinds", list_node_kinds_snapshot(doc));
    register_tool!("get_history_depth", get_history_depth_snapshot(doc));
    register_tool!("get_viewport", get_viewport_snapshot(doc));
    register_tool!("get_selection_set", get_selection_set_snapshot(doc));
    register_tool!("get_editor_state", get_editor_state_snapshot(doc));
    #[cfg(feature = "mcp-debug-tools")]
    if debug_tools_enabled() {
        register_tool!(
            "debug_validation_report",
            debug_validation_report_snapshot(doc)
        );
        register_tool!("debug_logs_tail", debug_logs_tail_snapshot());
        register_tool!("debug_screenshot", debug_screenshot_snapshot());
    }
    register_tool!("clear_selection", clear_selection_snapshot());
    register_tool!("set_selection", set_selection_snapshot());
    register_tool!("set_viewport", set_viewport_snapshot());
    register_tool!("set_node_hidden", set_node_hidden_snapshot());
    register_tool!("set_node_locked", set_node_locked_snapshot());
    register_tool!("set_node_collapsed", set_node_collapsed_snapshot());
    register_tool!("set_active_tool", set_active_tool_snapshot());
    register_tool!("undo", undo_snapshot());
    register_tool!("redo", redo_snapshot());
    register_tool!("duplicate_selected", duplicate_selected_snapshot());
    register_tool!("delete_selected", delete_selected_snapshot());
    register_tool!("nudge_selected", nudge_selected_snapshot());
    register_tool!("group_selected", group_selected_snapshot());
    register_tool!("ungroup_selected", ungroup_selected_snapshot());
    register_tool!("reorder_selected", reorder_selected_snapshot());
    register_tool!("set_node_rotation", set_node_rotation_snapshot());
    register_tool!("set_node_text", set_node_text_snapshot());
    register_tool!("set_node_corner_radius", set_node_corner_radius_snapshot());
    register_tool!("set_node_font_size", set_node_font_size_snapshot());
    register_tool!("set_node_font_weight", set_node_font_weight_snapshot());
    register_tool!("set_node_stroke_hex", set_node_stroke_hex_snapshot());
    register_tool!("set_node_stroke_width", set_node_stroke_width_snapshot());
    register_tool!("align_selected", align_selected_snapshot());
    register_tool!("set_node_fill_hex", set_node_fill_hex_snapshot());
    register_tool!("set_node_flip", set_node_flip_snapshot());
    register_tool!("set_ellipse_arc", set_ellipse_arc_snapshot());
    register_tool!("add_node_effect", add_node_effect_snapshot());
    register_tool!("remove_node_effect", remove_node_effect_snapshot());
    register_tool!("set_node_name", set_node_name_snapshot());
    register_tool!("set_selection_set", set_selection_set_snapshot());
    register_tool!("toggle_node_selection", toggle_node_selection_snapshot());
    register_tool!(
        "cycle_active_axis_value",
        cycle_active_axis_value_snapshot(doc)
    );
    register_tool!("copy_selected", copy_selected_snapshot());
    register_tool!("cut_selected", cut_selected_snapshot());
    register_tool!("paste_clipboard", paste_clipboard_snapshot());
    register_tool!("set_variable_color", set_variable_color_snapshot(doc));
    register_tool!("set_active_axis_value", set_active_axis_value_snapshot(doc));
    register_tool!("insert_node", insert_node_snapshot());
    register_tool!("import_svg", import_svg_snapshot());
    register_tool!("update_node", update_node_snapshot());
    register_tool!("delete_node", delete_node_snapshot());
    register_tool!("move_node", move_node_snapshot());
    register_tool!("copy_node", copy_node_snapshot());
    register_tool!("replace_node", replace_node_snapshot());
    register_tool!("batch_design", batch_design_snapshot(doc));
    register_tool!("get_design_prompt", get_design_prompt_snapshot(doc));
    register_tool!("design_skeleton", design_skeleton_snapshot());
    register_tool!("design_content", design_content_snapshot());
    register_tool!("design_refine", design_refine_snapshot(doc));
    register_tool!("set_variable_number", set_variable_number_snapshot(doc));
    register_tool!("set_variable_string", set_variable_string_snapshot(doc));
    register_tool!("set_variable_boolean", set_variable_boolean_snapshot(doc));
    register_tool!("set_variables", set_variables_snapshot());
    register_tool!("set_themes", set_themes_snapshot());
    register_tool!("create_variable", create_variable_snapshot(doc));
    register_tool!("delete_variable", delete_variable_snapshot(doc));
    register_tool!("rename_variable", rename_variable_snapshot(doc));
    register_tool!("instantiate_component", instantiate_component_snapshot());
    register_tool!("create_component", create_component_snapshot());
    register_tool!("delete_component", delete_component_snapshot());
    register_tool!("rename_component", rename_component_snapshot());
    register_tool!("set_active_page", set_active_page_snapshot());
    register_tool!("add_page", add_page_snapshot());
    register_tool!("rename_page", rename_page_snapshot(doc));
    register_tool!("delete_page", delete_page_snapshot(doc));
    register_tool!("remove_page", remove_page_snapshot(doc));
    register_tool!("duplicate_page", duplicate_page_snapshot(doc));
    register_tool!("reorder_page", reorder_page_snapshot(doc));
    r
}

fn should_register(requested_tool: Option<&str>, tool_name: &str) -> bool {
    requested_tool
        .map(|requested| requested == tool_name)
        .unwrap_or(true)
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
pub use wire::*;

mod doc_sync;
pub use doc_sync::*;

fn tools_list_response(id_raw: &str, state: &EditorState, debug_enabled: bool) -> String {
    let mut entries: Vec<String> = TOOL_SCHEMAS.iter().map(|s| (*s).to_string()).collect();
    entries.extend(op_mcp::element_tools::element_tool_schemas(state));
    #[cfg(not(feature = "mcp-debug-tools"))]
    let _ = debug_enabled;
    #[cfg(feature = "mcp-debug-tools")]
    if debug_enabled {
        entries.extend(DEBUG_TOOL_SCHEMAS.iter().map(|s| (*s).to_string()));
    }
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"tools":[{}]}}}}"#,
        entries.join(",")
    )
}

pub(crate) mod schemas;
#[cfg(not(feature = "mcp-debug-tools"))]
pub use schemas::TOOL_SCHEMAS;
#[cfg(feature = "mcp-debug-tools")]
pub use schemas::{DEBUG_TOOL_SCHEMAS, TOOL_SCHEMAS};

pub(crate) mod screenshot_tool;
use screenshot_tool::get_screenshot_snapshot;

pub(crate) mod export_tool;
use export_tool::export_nodes_snapshot;

#[cfg(test)]
mod codegen_wire_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wire_tests;
