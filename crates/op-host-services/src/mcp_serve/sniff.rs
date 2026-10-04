//! Top-level JSON key walkers for the MCP wire sniffers.
//!
//! Split out of `mcp_serve.rs` to keep that spine under the repository's
//! 800-line ceiling. Behaviour is unchanged: these walk a JSON-RPC line key
//! by key at depth 0 so a nested or string-valued `"method"` / `"id"` in
//! another field cannot shadow the real top-level one (the same discipline
//! `op_mcp::parser::arguments_field` uses).

/// Cheap top-level "method" field extractor. Returns the unquoted
/// string value; None if the field is missing or unparseable.
/// Walks the line key by key so a nested or string-valued
/// "method" in another field can't shadow the real top-level
/// method (mirrors `arguments_field`'s discipline in shell-core).
/// `pub(crate)` so the web-canvas `/mcp` loop can gate its (heavier)
/// `initialize`-body parse behind this cheap check.
pub(crate) fn sniff_method(line: &str) -> Option<String> {
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
pub(super) fn sniff_id_raw(line: &str) -> Option<String> {
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
