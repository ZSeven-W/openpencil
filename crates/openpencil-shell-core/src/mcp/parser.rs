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
    let id = extract_field(line, "id")?;
    let id = if let Ok(n) = id.parse::<i64>() {
        RequestId::Num(n)
    } else {
        RequestId::Str(id.trim_matches('"').to_string())
    };
    let method = extract_field(line, "method")?.trim_matches('"').to_string();
    let (tool, arguments) = if method == "tools/call" {
        // Real MCP: tool name + arguments live inside params.
        let params_body = extract_params_body(line)?;
        let name = extract_string_field(&params_body, "name")?;
        // `arguments` is nested object inside params.
        let arguments = extract_object_body(&params_body, "arguments")
            .and_then(|body| parse_flat_object_body(&body))
            .unwrap_or_default();
        (name, arguments)
    } else {
        // Legacy: method is the tool name; params is the args.
        let arguments = extract_params_object(line).unwrap_or_default();
        (method, arguments)
    };
    Some(ToolCall {
        id,
        tool,
        arguments,
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
fn extract_string_field(body: &str, key: &str) -> Option<String> {
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

/// Extract a nested object field's body (without the surrounding
/// braces). Used to find the `arguments` map inside MCP `params`.
fn extract_object_body(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = body.find(&needle)? + needle.len();
    let after = &body[start..];
    let colon = after.find(':')? + 1;
    let rest = after[colon..].trim_start();
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

/// Find the `"params":{...}` object in `line` and parse it into a
/// flat key→stringified-value map. Returns `None` when there's no
/// params key (treated as empty map). Handles three value kinds:
///
/// - **Strings**: `"node_id":"42"` — quotes stripped, escaped chars
///   passed through (no `\u` decode today; if it matters when MCP
///   clients send Unicode keys we'll switch to serde).
/// - **Numbers / bools / null**: `"page":1`, `"active":true` —
///   stored as their literal text representation, so the tool's
///   `parse::<u64>()` / `.parse::<bool>()` work the same as if the
///   client had sent them as strings.
/// - Nested objects / arrays are skipped (no tool needs them yet).
fn extract_params_object(line: &str) -> Option<BTreeMap<String, String>> {
    let needle = "\"params\"";
    let start = line.find(needle)? + needle.len();
    let after_colon = &line[start..];
    let colon = after_colon.find(':')? + 1;
    let rest = after_colon[colon..].trim_start();
    if !rest.starts_with('{') {
        return None;
    }
    // Find the matching close-brace by tracking depth + quotes.
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
        return None;
    }
    let body = &rest[1..end_byte];
    parse_flat_object_body(body)
}

/// Parse the body of a JSON object (the content between `{` and `}`)
/// into a flat key→stringified-value map. Skips nested objects /
/// arrays so deeper structure doesn't poison the result.
fn parse_flat_object_body(body: &str) -> Option<BTreeMap<String, String>> {
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
                out.insert(key, body[val_start..i].to_string());
                i += 1; // closing quote
            }
            b'{' | b'[' => {
                // Walk past the nested literal with depth tracking so the
                // outer parse continues past it; insert a sentinel value
                // for the key so tools see the arg as present-but-non-
                // scalar (and reject it) instead of silently absent.
                // The sentinel is deliberately not a valid value for any
                // scalar argument: it's neither a bool / decimal / hex /
                // enum that any tool accepts, so every existing tool's
                // own validation surfaces it as `InvalidArgument`.
                // Codex flagged the previous behavior (silently dropping
                // the key) as a destructive-swap guard bypass — a caller
                // sending `{ "drop_children": {} }` saw the arg as
                // missing and got the safe default; same for any
                // structured value sent to a tool that uses scalar
                // defaults. Fail loudly instead.
                let open = bytes[i];
                let close = if open == b'{' { b'}' } else { b']' };
                let mut depth = 1i32;
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
                    }
                    i += 1;
                }
                let sentinel = if open == b'{' { "{...}" } else { "[...]" };
                out.insert(key, sentinel.into());
            }
            _ => {
                // Number / true / false / null — read until comma /
                // close-brace / whitespace.
                let val_start = i;
                while i < bytes.len() && !matches!(bytes[i], b',' | b'}' | b' ' | b'\t' | b'\n' | b'\r') {
                    i += 1;
                }
                out.insert(key, body[val_start..i].to_string());
            }
        }
    }
    Some(out)
}

fn extract_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let start = line.find(&needle)? + needle.len();
    let after_colon = &line[start..];
    let colon = after_colon.find(':')? + 1;
    let val = after_colon[colon..].trim_start();
    let val_start = start + colon + (after_colon[colon..].len() - val.len());
    // Read until next , or }.
    let end_rel = val
        .find(|c: char| c == ',' || c == '}')
        .unwrap_or(val.len());
    Some(line[val_start..val_start + end_rel].trim())
}
