//! JSON-RPC wire-format parser for MCP tool calls. Hand-rolled so
//! shell-core stays serde-free (the wasm32 bundle skips ~120 KB of
//! serde monomorphization that way).
//!
//! Pulled out of `mcp.rs` to honor the 800-line cap; the protocol
//! parser grew to ~250 lines once it had to handle both the real
//! MCP `tools/call` envelope and the legacy flat shape.

use std::collections::BTreeMap;

use super::{RequestId, ToolCall};

pub fn parse_tool_call(line: &str) -> Option<ToolCall> {
    // Hand-rolled JSON-RPC parser — shell-core stays serde-free so
    // the wasm32 bundle doesn't pay the serde cost. Supports two
    // call shapes:
    //
    // 1. **Real MCP** (`method == "tools/call"`):
    //    `{"id":1,"method":"tools/call","params":{"name":"get_node",
    //      "arguments":{"node_id":"42"}}}`
    //    Tool name comes from `params.name`; arguments from
    //    `params.arguments`. This is what real MCP clients
    //    (Claude Code, Codex, etc.) send.
    //
    // 2. **Legacy / direct** (`method != "tools/call"`):
    //    `{"id":1,"method":"get_node","params":{"node_id":"42"}}`
    //    Tool name comes straight from `method`; arguments from
    //    top-level `params`. Kept for tests + tools/list style
    //    introspection calls.
    let id = extract_request_id(line)?;
    let method = extract_field(line, "method")?.trim_matches('"').to_string();
    let (tool, arguments) = if method == "tools/call" {
        // Real MCP: tool name + arguments live inside params.
        let params_body = extract_params_body(line)?;
        let name = extract_string_field(&params_body, "name")?;
        // `arguments` is nested object inside params. Tri-state:
        //   key missing → empty args (legit MCP can omit it).
        //   key present + value is `{...}` → parse the body.
        //   key present + scalar (e.g. `"arguments":"oops"`) → reject
        //     the parse (codex stop-gate: previously downgraded to
        //     empty args, hiding wire-shape errors).
        let arguments = match arguments_field(&params_body) {
            ParamsResult::Missing => BTreeMap::new(),
            ParamsResult::Body(body) => parse_flat_object_body(&body, &name)?,
            ParamsResult::Malformed => return None,
        };
        (name, arguments)
    } else {
        // Legacy: method is the tool name; params is the args.
        // Same three-way as above.
        let arguments = match params_body_if_present(line) {
            ParamsResult::Missing => BTreeMap::new(),
            ParamsResult::Body(body) => parse_flat_object_body(&body, &method)?,
            ParamsResult::Malformed => return None,
        };
        (method, arguments)
    };
    Some(ToolCall {
        id,
        tool,
        arguments,
    })
}

/// Pull the JSON-RPC request id out of `line` without doing a full
/// `parse_tool_call`. Used by the stdio dispatcher to surface a
/// typed error response when the full parse fails — without the
/// id the client can't correlate the response and would hang
/// waiting on it. Returns `None` only when the id field itself is
/// missing or unreadable.
pub fn extract_request_id(line: &str) -> Option<RequestId> {
    let raw = extract_field(line, "id")?;
    Some(if let Ok(n) = raw.parse::<i64>() {
        RequestId::Num(n)
    } else {
        RequestId::Str(raw.trim_matches('"').to_string())
    })
}

/// Return the body string of the top-level `"params":{...}` object
/// without the surrounding braces. `None` when params is missing or
/// not an object. Mirrors the brace-walking logic in
/// `extract_params_object` but returns the raw body so the MCP path
/// can re-walk it for `name` / `arguments` fields.
fn extract_params_body(line: &str) -> Option<String> {
    let needle = "\"params\"";
    let start = line.find(needle)? + needle.len();
    let after_colon = &line[start..];
    let colon = after_colon.find(':')? + 1;
    let rest = after_colon[colon..].trim_start();
    if !rest.starts_with('{') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[1..i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract a string field's unquoted body from a JSON object body.
/// Used to fetch `name` out of an MCP `params` block. Returns the
/// raw string contents (escapes passed through; no unicode decode).
///
/// `pub(crate)` so `tool_search` can reuse the same extractor for
/// pulling `"name"` / `"description"` out of raw descriptor strings
/// without adding serde.
pub(crate) fn extract_string_field(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = body.find(&needle)? + needle.len();
    let after = &body[start..];
    let colon = after.find(':')? + 1;
    let rest = after[colon..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut i = 1;
    let val_start = i;
    while i < bytes.len() && bytes[i] != b'"' {
        if bytes[i] == b'\\' {
            i += 2;
        } else {
            i += 1;
        }
    }
    if i >= bytes.len() {
        return None;
    }
    Some(rest[val_start..i].to_string())
}

/// Tri-state lookup for MCP `arguments` inside a `params` block.
/// Walks `body` at depth 0 so we ignore nested keys (e.g. a sibling
/// `meta.arguments` can't shadow the real top-level field) and
/// string values that happen to read `"arguments"`. Returns:
///   - Missing: `arguments` key absent at top level (legit empty
///     args).
///   - Body(s): `arguments` is an object; returns its body without
///     the braces.
///   - Malformed: `arguments` is present but not an object
///     (e.g. `"arguments":"oops"` / `:42` / `:[]`), or the body
///     itself doesn't tokenize as a JSON object body.
///
/// Codex stop-gate, twice: first the present-but-non-object
/// downgrade to empty args; then the substring-find that
/// matched nested keys / string values containing the literal
/// `"arguments"`. Both addressed by walking top-level keys
/// explicitly.
fn arguments_field(body: &str) -> ParamsResult {
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // Skip whitespace + commas between keys.
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] != b'"' {
            return ParamsResult::Malformed;
        }
        // Read the quoted key.
        i += 1;
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'"' {
            if bytes[i] == b'\\' {
                i = i.saturating_add(2);
            } else {
                i += 1;
            }
        }
        if i >= bytes.len() {
            return ParamsResult::Malformed;
        }
        let key = &body[key_start..i];
        i += 1; // past closing quote
                // Whitespace, colon, whitespace.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b':' {
            return ParamsResult::Malformed;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return ParamsResult::Malformed;
        }
        let is_args = key == "arguments";
        match bytes[i] {
            b'"' => {
                // String value — read it to the matching quote.
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i = i.saturating_add(2);
                    } else {
                        i += 1;
                    }
                }
                if i >= bytes.len() {
                    return ParamsResult::Malformed;
                }
                i += 1;
                if is_args {
                    return ParamsResult::Malformed;
                }
            }
            b'{' => {
                let val_start = i + 1;
                let mut depth = 1i32;
                let mut j = i + 1;
                let mut in_str = false;
                let mut escape = false;
                while j < bytes.len() && depth > 0 {
                    let c = bytes[j];
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
                    } else if c == b'{' {
                        depth += 1;
                    } else if c == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    j += 1;
                }
                if depth != 0 || j >= bytes.len() {
                    return ParamsResult::Malformed;
                }
                if is_args {
                    return ParamsResult::Body(body[val_start..j].to_string());
                }
                i = j + 1;
            }
            b'[' => {
                let mut depth = 1i32;
                let mut j = i + 1;
                let mut in_str = false;
                let mut escape = false;
                while j < bytes.len() && depth > 0 {
                    let c = bytes[j];
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
                    } else if c == b'[' {
                        depth += 1;
                    } else if c == b']' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    j += 1;
                }
                if depth != 0 || j >= bytes.len() {
                    return ParamsResult::Malformed;
                }
                if is_args {
                    return ParamsResult::Malformed;
                }
                i = j + 1;
            }
            _ => {
                // Number / bool / null — read until comma / close.
                while i < bytes.len()
                    && !matches!(bytes[i], b',' | b'}' | b' ' | b'\t' | b'\n' | b'\r')
                {
                    i += 1;
                }
                if is_args {
                    return ParamsResult::Malformed;
                }
            }
        }
    }
    ParamsResult::Missing
}

/// Tri-state result of looking for `"params"` on a legacy JSON-RPC
/// line. The caller distinguishes "no params key" (legit empty
/// args) from "params present but malformed" (full call rejection,
/// e.g. a nested value for a key would otherwise have been
/// silently dropped) — see [`parse_tool_call`].
enum ParamsResult {
    Missing,
    Body(String),
    Malformed,
}

/// Locate `"params":{...}` on `line` and return its body without
/// the surrounding braces. Mirrors the brace-walking logic in
/// `extract_params_body` but with a tri-state result so callers
/// can react to "params present but unparseable" separately from
/// "params missing".
fn params_body_if_present(line: &str) -> ParamsResult {
    let needle = "\"params\"";
    let Some(found) = line.find(needle) else {
        return ParamsResult::Missing;
    };
    let start = found + needle.len();
    let after_colon = &line[start..];
    let Some(colon_off) = after_colon.find(':') else {
        return ParamsResult::Malformed;
    };
    let rest = after_colon[colon_off + 1..].trim_start();
    if !rest.starts_with('{') {
        return ParamsResult::Malformed;
    }
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    let mut end_byte = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end_byte = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return ParamsResult::Malformed;
    }
    ParamsResult::Body(rest[1..end_byte].to_string())
}

/// Parse the body of a JSON object (the content between `{` and `}`)
/// into a key→stringified-value map. Most tools accept only scalars;
/// the TypeScript parity bulk variable tools legitimately pass object
/// arguments (`variables`, `themes`), so those specific fields are
/// preserved as compact JSON strings for their tool layer to validate.
fn parse_flat_object_body(body: &str, tool: &str) -> Option<BTreeMap<String, String>> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // Skip whitespace + commas.
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // Expect a string key.
        if bytes[i] != b'"' {
            return None;
        }
        let key_start = i + 1;
        i += 1;
        while i < bytes.len() && bytes[i] != b'"' {
            if bytes[i] == b'\\' {
                i += 2;
            } else {
                i += 1;
            }
        }
        if i >= bytes.len() {
            return None;
        }
        let key = body[key_start..i].to_string();
        i += 1; // consume closing quote
                // Skip whitespace + colon.
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b':') {
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }
        // Parse value.
        match bytes[i] {
            b'"' => {
                let val_start = i + 1;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i >= bytes.len() {
                    return None;
                }
                let raw = &body[val_start..i];
                out.insert(key, decode_json_string_fragment(raw)?);
                i += 1; // closing quote
            }
            b'{' | b'[' => {
                if !structured_arg_allowed(tool, &key) {
                    return None;
                }
                let open = bytes[i];
                let close = if open == b'{' { b'}' } else { b']' };
                let mut depth = 1i32;
                let start = i;
                i += 1;
                let mut in_str = false;
                let mut escape = false;
                while i < bytes.len() && depth > 0 {
                    let c = bytes[i];
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
                        if depth == 0 {
                            break;
                        }
                    }
                    i += 1;
                }
                if depth != 0 || i >= bytes.len() {
                    return None;
                }
                out.insert(key, body[start..=i].to_string());
                i += 1;
            }
            _ => {
                // Number / true / false / null — read until comma /
                // close-brace / whitespace.
                let val_start = i;
                while i < bytes.len()
                    && !matches!(bytes[i], b',' | b'}' | b' ' | b'\t' | b'\n' | b'\r')
                {
                    i += 1;
                }
                out.insert(key, body[val_start..i].to_string());
            }
        }
    }
    Some(out)
}

fn structured_arg_allowed(tool: &str, key: &str) -> bool {
    if tool.starts_with("add_") {
        return !key.is_empty();
    }
    matches!(
        (tool, key),
        ("set_variables", "variables")
            | ("upsert_variables", "variables")
            | ("upsert_component", "node_json")
            | ("upsert_screen", "node_json")
            | ("set_themes", "themes")
            | ("read_nodes", "nodeIds")
            | ("batch_get", "patterns")
            | ("batch_get", "nodeIds")
            | ("search_all_unique_properties", "parents")
            | ("search_all_unique_properties", "properties")
            | ("replace_all_matching_properties", "parents")
            | ("replace_all_matching_properties", "properties")
            | ("codegen_plan", "plan")
            | ("codegen_submit_chunk", "result")
            | ("copy_node", "overrides")
            | ("insert_node", "data")
            | ("update_node", "data")
            | ("replace_node", "data")
            | ("design_skeleton", "rootFrame")
            | ("design_skeleton", "sections")
            | ("design_skeleton", "styleGuide")
            | ("design_content", "children")
            | ("get_style_guide", "tags")
            | ("finalize_design", "root_ids")
            | ("enrich_images", "root_ids")
    )
}

fn decode_json_string_fragment(raw: &str) -> Option<String> {
    let token = format!("\"{raw}\"");
    serde_json::from_str::<String>(&token).ok()
}

fn extract_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let start = line.find(&needle)? + needle.len();
    let after_colon = &line[start..];
    let colon = after_colon.find(':')? + 1;
    let val = after_colon[colon..].trim_start();
    let val_start = start + colon + (after_colon[colon..].len() - val.len());
    // Read until next , or }.
    let end_rel = val.find([',', '}']).unwrap_or(val.len());
    Some(line[val_start..val_start + end_rel].trim())
}
