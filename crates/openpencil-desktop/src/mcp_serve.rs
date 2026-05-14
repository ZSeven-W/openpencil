//! Stdio MCP server mode for the desktop binary.
//!
//! When `openpencil-desktop --mcp <path>` is invoked, we skip the
//! winit event loop and run a JSON-RPC stdio server against the
//! `.op` file at `<path>`. External CLIs (Claude Code / Codex /
//! Gemini / Copilot) can spawn the binary in this mode to drive
//! the Rust editor exactly the way they drive TS pen-mcp today.
//!
//! Per-line dispatch:
//!   - `initialize` → respond with protocol version + server
//!     capabilities so the client completes its handshake.
//!   - `tools/list` → respond with the 14-tool catalog + JSON
//!     schemas for each input.
//!   - `notifications/initialized` / `ping` → handled inline
//!     (no body / pong).
//!   - `tools/call` / legacy direct dispatch → routed through
//!     shell-core's `run_stdio_with_applier` so the parser-level
//!     hardening (structured-arg rejection / top-level walker /
//!     no-hang error) protects this binary too. The applier
//!     mutates the live document + saves on success.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use openpencil_shell_core::document::Document;
use openpencil_shell_core::mcp::{
    batch_design_snapshot, copy_node_snapshot, delete_node_snapshot,
    design_content_snapshot, design_refine_snapshot, design_skeleton_snapshot,
    add_page_snapshot, create_component_snapshot, delete_component_snapshot,
    delete_page_snapshot, document_info_snapshot, duplicate_page_snapshot,
    get_component_snapshot, instantiate_component_snapshot, list_components_snapshot,
    rename_component_snapshot, rename_page_snapshot, reorder_page_snapshot,
    set_active_page_snapshot,
    get_active_theme_snapshot, get_node_snapshot, insert_node_snapshot,
    list_pages_snapshot, list_variables_snapshot, move_node_snapshot,
    replace_node_snapshot, run_stdio_with_applier, selection_snapshot,
    set_active_axis_value_snapshot, set_variable_boolean_snapshot,
    set_variable_color_snapshot, set_variable_number_snapshot,
    set_variable_string_snapshot, update_node_snapshot, ToolRegistry,
};

use crate::persistence::{load_from_path, save_to_path};

/// Run the stdio MCP server against `path`. Returns Ok(()) on EOF,
/// Err on unrecoverable IO. Blocks the calling thread for the
/// lifetime of the stdio connection.
pub fn run(path: PathBuf) -> Result<(), String> {
    let mut doc = Document::empty();
    load_from_path(&mut doc, &path).map_err(|e| format!("load {}: {e}", path.display()))?;

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
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // MCP handshake / discovery methods short-circuit the
        // tool dispatcher. Detect them via a cheap method-field
        // sniff so we don't need to drag JSON parsing in here —
        // shell-core's parser stays the single source for the
        // `tools/call` envelope.
        let method = sniff_method(trimmed);
        match method.as_deref() {
            Some("initialize") => {
                if let Some(id_raw) = sniff_id_raw(trimmed) {
                    writeln!(writer, "{}", initialize_response(&id_raw))
                        .map_err(|e| format!("stdout write: {e}"))?;
                    writer.flush().map_err(|e| format!("stdout flush: {e}"))?;
                }
                continue;
            }
            Some("tools/list") => {
                if let Some(id_raw) = sniff_id_raw(trimmed) {
                    writeln!(writer, "{}", tools_list_response(&id_raw))
                        .map_err(|e| format!("stdout write: {e}"))?;
                    writer.flush().map_err(|e| format!("stdout flush: {e}"))?;
                }
                continue;
            }
            Some("notifications/initialized") | Some("initialized") => {
                // Notification — no response required by spec.
                continue;
            }
            Some("ping") => {
                if let Some(id_raw) = sniff_id_raw(trimmed) {
                    writeln!(writer, "{}", ping_response(&id_raw))
                        .map_err(|e| format!("stdout write: {e}"))?;
                    writer.flush().map_err(|e| format!("stdout flush: {e}"))?;
                }
                continue;
            }
            _ => {}
        }

        // Fall through: tools/call or legacy direct dispatch.
        let registry = rebuild_registry(&doc);
        let mut applier_failed: Option<String> = None;
        let path_for_save = path.clone();
        {
            let doc_ref = &mut doc;
            let path_ref = &path_for_save;
            let applier_failed_ref = &mut applier_failed;
            let mut input = std::io::Cursor::new(line.as_bytes());
            run_stdio_with_applier(&registry, &mut input, &mut writer, |cmd| {
                if !doc_ref.apply_mcp_command(cmd) {
                    return false;
                }
                if let Err(e) = save_to_path(doc_ref, path_ref) {
                    *applier_failed_ref = Some(format!("save failed: {e}"));
                    return false;
                }
                true
            })
            .map_err(|e| format!("dispatch: {e}"))?;
        }
        writer.flush().map_err(|e| format!("stdout flush: {e}"))?;
        if let Some(msg) = applier_failed {
            eprintln!("openpencil-desktop --mcp: {msg}");
        }
    }
}

/// Re-build the registry against the latest document so read-tool
/// snapshots reflect every prior write command's mutations.
fn rebuild_registry(doc: &Document) -> ToolRegistry {
    let mut r = ToolRegistry::default();
    r.register(Box::new(document_info_snapshot(doc)));
    r.register(Box::new(selection_snapshot(doc)));
    r.register(Box::new(get_node_snapshot(doc)));
    r.register(Box::new(list_pages_snapshot(doc)));
    r.register(Box::new(list_variables_snapshot(doc)));
    r.register(Box::new(get_active_theme_snapshot(doc)));
    r.register(Box::new(list_components_snapshot(doc)));
    r.register(Box::new(get_component_snapshot(doc)));
    r.register(Box::new(set_variable_color_snapshot(doc)));
    r.register(Box::new(set_active_axis_value_snapshot(doc)));
    r.register(Box::new(insert_node_snapshot()));
    r.register(Box::new(update_node_snapshot()));
    r.register(Box::new(delete_node_snapshot()));
    r.register(Box::new(move_node_snapshot()));
    r.register(Box::new(copy_node_snapshot()));
    r.register(Box::new(replace_node_snapshot()));
    r.register(Box::new(batch_design_snapshot()));
    r.register(Box::new(design_skeleton_snapshot()));
    r.register(Box::new(design_content_snapshot()));
    r.register(Box::new(design_refine_snapshot()));
    r.register(Box::new(set_variable_number_snapshot(doc)));
    r.register(Box::new(set_variable_string_snapshot(doc)));
    r.register(Box::new(set_variable_boolean_snapshot(doc)));
    r.register(Box::new(instantiate_component_snapshot()));
    r.register(Box::new(create_component_snapshot()));
    r.register(Box::new(delete_component_snapshot()));
    r.register(Box::new(rename_component_snapshot()));
    r.register(Box::new(set_active_page_snapshot()));
    r.register(Box::new(add_page_snapshot()));
    r.register(Box::new(rename_page_snapshot()));
    r.register(Box::new(delete_page_snapshot()));
    r.register(Box::new(duplicate_page_snapshot()));
    r.register(Box::new(reorder_page_snapshot()));
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
fn walk_top_level_for_string_value(
    bytes: &[u8],
    i: &mut usize,
    target: &str,
) -> Option<String> {
    walk_top_level(bytes, i, target, /*string_only=*/ true)
}

fn walk_top_level_for_raw_value(bytes: &[u8], i: &mut usize, target: &str) -> Option<String> {
    walk_top_level(bytes, i, target, /*string_only=*/ false)
}

fn walk_top_level(
    bytes: &[u8],
    i: &mut usize,
    target: &str,
    string_only: bool,
) -> Option<String> {
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

fn tools_list_response(id_raw: &str) -> String {
    // The tool catalog must match what `rebuild_registry`
    // installs. Schemas are minimal but sufficient for an MCP
    // client to render a tool picker + validate calls.
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"tools":[{}]}}}}"#,
        TOOL_SCHEMAS.join(",")
    )
}

/// Per-tool JSON schemas — kept as string literals to avoid
/// pulling serde into this binary's MCP path. Each entry is a
/// complete JSON object: name + description + inputSchema.
const TOOL_SCHEMAS: &[&str] = &[
    // --- read tools ---
    r#"{"name":"get_document_info","description":"Summarize the open document (page count, active page, etc).","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_selection","description":"Return the current selection state (ids, count).","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_node","description":"Read a node by id with depth-limited descendants.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string","description":"u64 node id"}},"required":["node_id"]}}"#,
    r#"{"name":"list_pages","description":"List page ids + names.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"list_variables","description":"List design variables with kinds.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_active_theme","description":"Return the active theme axis pinning per axis.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"list_components","description":"List registered components (saved Frames / Groups promoted via Save as Component). Returns count + a `;`-separated record of `name|id` pairs.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"get_component","description":"Fetch one component by id with detail: name, root node kind, and the subtree's leaf count.","inputSchema":{"type":"object","properties":{"component_id":{"type":"string","description":"positive u64 component id"}},"required":["component_id"]}}"#,
    // --- write tools ---
    r##"{"name":"set_variable_color","description":"Set a Color-kind variable's value.","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"hex":{"type":"string","description":"#rgb / #rrggbb / #rrggbbaa"}},"required":["name","hex"]}}"##,
    r#"{"name":"batch_design","description":"Insert N leaf nodes on the active page in one atomic shot. nodes_json must be a JSON array string of {kind,name,x,y,width,height,fill_hex?} descriptors.","inputSchema":{"type":"object","properties":{"nodes_json":{"type":"string","description":"JSON array of node descriptors"}},"required":["nodes_json"]}}"#,
    r#"{"name":"design_skeleton","description":"Layered design workflow phase 1: insert structural scaffolding. Apply behavior is identical to batch_design today (phase is metadata only); the response carries phase=skeleton so the client can phase its prompting. A future patch may add per-phase apply semantics.","inputSchema":{"type":"object","properties":{"nodes_json":{"type":"string","description":"JSON array of node descriptors"}},"required":["nodes_json"]}}"#,
    r#"{"name":"design_content","description":"Layered design workflow phase 2: fill content into the scaffold. Apply behavior is identical to batch_design today (phase is metadata only); the response carries phase=content. A future patch may add per-phase apply semantics.","inputSchema":{"type":"object","properties":{"nodes_json":{"type":"string","description":"JSON array of node descriptors"}},"required":["nodes_json"]}}"#,
    r#"{"name":"design_refine","description":"Layered design workflow phase 3: polish details. Apply behavior is identical to batch_design today (phase is metadata only); the response carries phase=refine. A future patch may add per-phase apply semantics.","inputSchema":{"type":"object","properties":{"nodes_json":{"type":"string","description":"JSON array of node descriptors"}},"required":["nodes_json"]}}"#,
    r#"{"name":"set_variable_number","description":"Set a Number-kind variable's value (decimal, may be negative or fractional).","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string"}},"required":["name","value"]}}"#,
    r#"{"name":"set_variable_string","description":"Set a String-kind variable's value (free-form text).","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string"}},"required":["name","value"]}}"#,
    r#"{"name":"set_variable_boolean","description":"Set a Boolean-kind variable's value (\"true\" or \"false\").","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string","enum":["true","false"]}},"required":["name","value"]}}"#,
    r#"{"name":"instantiate_component","description":"Drop a clone of a registered component's root subtree onto the active page. component_id is the id returned by list_components.","inputSchema":{"type":"object","properties":{"component_id":{"type":"string","description":"positive u64 component id"}},"required":["component_id"]}}"#,
    r#"{"name":"create_component","description":"Promote an existing Frame or Group on the active page to a registered component. Use list_components afterwards to see it, instantiate_component to drop a clone.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string","description":"positive u64 node id"},"name":{"type":"string"}},"required":["node_id","name"]}}"#,
    r#"{"name":"delete_component","description":"Remove a component from the registry by id. Live instances already on the page are NOT affected — they're independent clones.","inputSchema":{"type":"object","properties":{"component_id":{"type":"string","description":"positive u64 component id"}},"required":["component_id"]}}"#,
    r#"{"name":"rename_component","description":"Rename a registered component. Name must be non-empty / non-whitespace.","inputSchema":{"type":"object","properties":{"component_id":{"type":"string","description":"positive u64 component id"},"name":{"type":"string"}},"required":["component_id","name"]}}"#,
    r#"{"name":"set_active_page","description":"Switch which page is the active target for subsequent inserts / batch_design / design_* commands. index is 0-based.","inputSchema":{"type":"object","properties":{"index":{"type":"string","description":"0-based page index"}},"required":["index"]}}"#,
    r#"{"name":"add_page","description":"Append a fresh empty page and switch the active page to it. Returns false on id-space exhaustion.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}"#,
    r#"{"name":"rename_page","description":"Set a page's display name. Name must be non-empty / non-whitespace.","inputSchema":{"type":"object","properties":{"index":{"type":"string","description":"0-based page index"},"name":{"type":"string"}},"required":["index","name"]}}"#,
    r#"{"name":"delete_page","description":"Remove a page by index. The applier keeps the active page valid (clamps if needed).","inputSchema":{"type":"object","properties":{"index":{"type":"string","description":"0-based page index"}},"required":["index"]}}"#,
    r#"{"name":"duplicate_page","description":"Clone the page at index and switch the active page to the clone.","inputSchema":{"type":"object","properties":{"index":{"type":"string","description":"0-based page index"}},"required":["index"]}}"#,
    r#"{"name":"reorder_page","description":"Move a page from one index to another. `to` is clamped to [0, page_count). The active page tracking follows the move.","inputSchema":{"type":"object","properties":{"from":{"type":"string","description":"0-based source page index"},"to":{"type":"string","description":"0-based target page index"}},"required":["from","to"]}}"#,
    r#"{"name":"set_active_axis_value","description":"Pin a theme axis to one of its allowed values.","inputSchema":{"type":"object","properties":{"axis":{"type":"string"},"value":{"type":"string"}},"required":["axis","value"]}}"#,
    r#"{"name":"insert_node","description":"Create a new leaf node on the active page.","inputSchema":{"type":"object","properties":{"kind":{"type":"string","enum":["frame","group","rect","ellipse","polygon","line","text","path"]},"name":{"type":"string"},"x":{"type":"string"},"y":{"type":"string"},"width":{"type":"string"},"height":{"type":"string"},"fill_hex":{"type":"string"}},"required":["kind","name","x","y","width","height"]}}"#,
    r#"{"name":"update_node","description":"Patch fields on an existing node. Pass any subset of x/y/width/height/name/fill_hex.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"x":{"type":"string"},"y":{"type":"string"},"width":{"type":"string"},"height":{"type":"string"},"name":{"type":"string"},"fill_hex":{"type":"string"}},"required":["node_id"]}}"#,
    r#"{"name":"delete_node","description":"Remove a node + descendants from its parent.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"}},"required":["node_id"]}}"#,
    r#"{"name":"move_node","description":"Reparent a node. target_parent_id=0 puts it at the active page root.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"target_parent_id":{"type":"string"}},"required":["node_id","target_parent_id"]}}"#,
    r#"{"name":"copy_node","description":"Deep-clone a subtree with fresh ids under a new parent.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"target_parent_id":{"type":"string"}},"required":["node_id","target_parent_id"]}}"#,
    r#"{"name":"replace_node","description":"Swap an existing node at the same parent slot with a freshly-built leaf. Set drop_children=true to discard a container's subtree.","inputSchema":{"type":"object","properties":{"node_id":{"type":"string"},"kind":{"type":"string","enum":["frame","group","rect","ellipse","polygon","line","text","path"]},"name":{"type":"string"},"x":{"type":"string"},"y":{"type":"string"},"width":{"type":"string"},"height":{"type":"string"},"fill_hex":{"type":"string"},"drop_children":{"type":"string","enum":["true","false"]}},"required":["node_id","kind","name","x","y","width","height"]}}"#,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_method_walks_top_level() {
        assert_eq!(
            sniff_method(r#"{"id":1,"method":"initialize","params":{}}"#),
            Some("initialize".into())
        );
        assert_eq!(
            sniff_method(r#"{"id":1,"method":"tools/call","params":{"name":"x"}}"#),
            Some("tools/call".into())
        );
        // Nested `method` keys must not shadow the real one.
        assert_eq!(
            sniff_method(r#"{"id":1,"method":"tools/list","params":{"method":"fake"}}"#),
            Some("tools/list".into())
        );
        assert_eq!(sniff_method("not even json"), None);
    }

    #[test]
    fn sniff_id_raw_preserves_type() {
        assert_eq!(
            sniff_id_raw(r#"{"id":42,"method":"x"}"#),
            Some("42".into())
        );
        assert_eq!(
            sniff_id_raw(r#"{"id":"abc","method":"x"}"#),
            Some(r#""abc""#.into())
        );
    }

    #[test]
    fn initialize_response_includes_protocol_and_capabilities() {
        let r = initialize_response("7");
        assert!(r.contains(r#""id":7"#));
        assert!(r.contains(r#""protocolVersion""#));
        assert!(r.contains(r#""tools""#));
        assert!(r.contains(r#""serverInfo""#));
    }

    #[test]
    fn tools_list_response_includes_all_thirty_three_tools() {
        let r = tools_list_response("3");
        // Exact-count assertion: any tool added without
        // updating this test will trip the count first. Codex
        // stop-gate: previous `contains`-only checks would have
        // silently passed if a new tool slipped into TOOL_SCHEMAS
        // without being added to the list below.
        assert_eq!(
            TOOL_SCHEMAS.len(),
            33,
            "tools/list catalog count must match the registered tools — add the new tool to this test"
        );
        for name in [
            "get_document_info",
            "get_selection",
            "get_node",
            "list_pages",
            "list_variables",
            "get_active_theme",
            "list_components",
            "get_component",
            "instantiate_component",
            "create_component",
            "delete_component",
            "rename_component",
            "set_active_page",
            "add_page",
            "rename_page",
            "delete_page",
            "duplicate_page",
            "reorder_page",
            "set_variable_color",
            "set_active_axis_value",
            "insert_node",
            "update_node",
            "delete_node",
            "move_node",
            "copy_node",
            "replace_node",
            "batch_design",
            "set_variable_number",
            "set_variable_string",
            "set_variable_boolean",
            "design_skeleton",
            "design_content",
            "design_refine",
        ] {
            assert!(r.contains(name), "tools/list must include {name}: {r}");
        }
    }
}
