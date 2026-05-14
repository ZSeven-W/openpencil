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
        // `arguments` is nested object inside params. Three-way:
        //   no `arguments` key → empty args, parse succeeds.
        //   present + scalar-only → those args.
        //   present + any nested value → reject the parse so
        //     no tool sees a malformed input as a real scalar.
        let arguments = match extract_object_body(&params_body, "arguments") {
            None => BTreeMap::new(),
            Some(body) => parse_flat_object_body(&body)?,
        };
        (name, arguments)
    } else {
        // Legacy: method is the tool name; params is the args.
        // Same three-way as above.
        let arguments = match params_body_if_present(line) {
            ParamsResult::Missing => BTreeMap::new(),
            ParamsResult::Body(body) => parse_flat_object_body(&body)?,
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
                // Reject the entire parse when a structured value
                // shows up for any key. Tool args are scalars by
                // contract; surfacing a sentinel here would either
                // collide with a legitimate scalar (Codex flagged
                // `{...}` could match a real variable name) or
                // leave string-accepting tools unable to tell wire
                // malformed input from a real value. The wire
                // dispatch loop continues to the next line on
                // None, so the client sees no response for the
                // malformed call — same as any other unparseable
                // JSON-RPC frame. Any future tool that legitimately
                // needs structured args must add its own typed
                // path here.
                return None;
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
