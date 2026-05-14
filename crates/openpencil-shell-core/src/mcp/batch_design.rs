//! `batch_design` write tool + nodes_json parser. Carved off
//! `write_tools.rs` to stay under the 800-line cap once the
//! batch surface landed.

use std::collections::BTreeMap;

use super::write_tools::{validate_hex, ALLOWED_KINDS};
use super::{BatchInsertItem, McpCommand, McpTool, ToolErrorCode, ToolOutcome};

/// First-party `batch_design` tool — insert N leaf nodes on the
/// active page in one atomic shot. Mirrors TS `batch_design` for
/// the leaf subset.
///
/// Wire shape: one scalar string arg `nodes_json` carrying a JSON
/// array of node descriptors. The shell-core parser rejects
/// structured args at the top level (so an LLM can't sneak a
/// nested object past scalar contracts), but a JSON array
/// embedded inside a quoted string round-trips cleanly. Each
/// array entry is `{"kind":"...","name":"...","x":N,"y":N,
/// "width":N,"height":N,"fill_hex":"#..."}` — the same shape
/// `insert_node` accepts, minus the wire wrapping.
///
/// The tool parses the inner JSON, validates EVERY entry, and
/// emits `McpCommand::BatchInsert { items: ... }`. The apply
/// path is all-or-nothing: a single bad entry rejects the whole
/// batch so the LLM never sees a partial design tree.
pub struct BatchDesign;

impl McpTool for BatchDesign {
    fn name(&self) -> &str {
        "batch_design"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("nodes_json") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "nodes_json is required (JSON array of node descriptors)".into(),
            );
        };
        match parse_batch_items(raw) {
            Ok(items) if items.is_empty() => ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "nodes_json must contain at least one descriptor".into(),
            ),
            Ok(items) => {
                let mut out = BTreeMap::new();
                out.insert("wrote".into(), "true".into());
                out.insert("count".into(), items.len().to_string());
                ToolOutcome::OkWithCommand(out, McpCommand::BatchInsert { items })
            }
            Err(e) => ToolOutcome::Err(ToolErrorCode::InvalidArgument, e),
        }
    }
}

pub fn batch_design_snapshot() -> BatchDesign {
    BatchDesign
}

/// Hand-rolled parser for the `nodes_json` payload. Shell-core
/// stays serde-free so the wasm32 bundle doesn't grow. Returns a
/// Vec<BatchInsertItem> on success, an English error string on
/// any structural problem.
///
/// Grammar (whitespace ignored):
///   array      = '[' (item (',' item)* )? ']'
///   item       = '{' pair (',' pair)* '}'
///   pair       = string ':' value
///   string     = '"' chars '"'
///   value      = string | number
///
/// Strings handle `\"` and `\\` escapes inline; no `\u` decode
/// (the wire never carries unicode escapes in tool args today).
fn parse_batch_items(input: &str) -> Result<Vec<BatchInsertItem>, String> {
    // The wire-level parser doesn't unescape JSON string contents
    // — `{"nodes_json":"[\"x\"]"}` arrives here as the raw bytes
    // `[\"x\"]` (backslash + quote). Pre-pass: unescape so the
    // grammar below sees real `"` / `\` / `\n` etc.
    let unescaped = unescape_wire_string(input)?;
    let bytes = unescaped.as_bytes();
    let mut i = 0usize;
    skip_ws(bytes, &mut i);
    if i >= bytes.len() || bytes[i] != b'[' {
        return Err("nodes_json must start with `[`".into());
    }
    i += 1;
    skip_ws(bytes, &mut i);
    let mut out = Vec::new();
    if i < bytes.len() && bytes[i] == b']' {
        return Ok(out); // empty array — caller surfaces InvalidArgument
    }
    loop {
        skip_ws(bytes, &mut i);
        let item = parse_item(bytes, &mut i)?;
        out.push(item);
        skip_ws(bytes, &mut i);
        if i >= bytes.len() {
            return Err("unterminated array".into());
        }
        match bytes[i] {
            b',' => {
                i += 1;
            }
            b']' => {
                i += 1;
                skip_ws(bytes, &mut i);
                if i != bytes.len() {
                    return Err("trailing garbage after array".into());
                }
                return Ok(out);
            }
            other => {
                return Err(format!(
                    "expected `,` or `]` after item, got {:?}",
                    other as char
                ));
            }
        }
    }
}

fn parse_item(bytes: &[u8], i: &mut usize) -> Result<BatchInsertItem, String> {
    if *i >= bytes.len() || bytes[*i] != b'{' {
        return Err("expected `{` to start a descriptor".into());
    }
    *i += 1;
    let mut kind: Option<String> = None;
    let mut name: Option<String> = None;
    let mut x: Option<i32> = None;
    let mut y: Option<i32> = None;
    let mut width: Option<i32> = None;
    let mut height: Option<i32> = None;
    let mut fill_hex: Option<String> = None;
    loop {
        skip_ws(bytes, i);
        if *i >= bytes.len() {
            return Err("unterminated descriptor".into());
        }
        if bytes[*i] == b'}' {
            *i += 1;
            break;
        }
        let key = parse_string(bytes, i)?;
        skip_ws(bytes, i);
        if *i >= bytes.len() || bytes[*i] != b':' {
            return Err(format!("expected `:` after key {key:?}"));
        }
        *i += 1;
        skip_ws(bytes, i);
        match key.as_str() {
            "kind" => kind = Some(parse_string(bytes, i)?),
            "name" => name = Some(parse_string(bytes, i)?),
            "fill_hex" => fill_hex = Some(parse_string(bytes, i)?),
            "x" => x = Some(parse_int(bytes, i)?),
            "y" => y = Some(parse_int(bytes, i)?),
            "width" => width = Some(parse_int(bytes, i)?),
            "height" => height = Some(parse_int(bytes, i)?),
            other => return Err(format!("unknown key {other:?} in descriptor")),
        }
        skip_ws(bytes, i);
        if *i < bytes.len() && bytes[*i] == b',' {
            *i += 1;
        }
    }
    let kind = kind.ok_or("descriptor missing `kind`")?;
    if !ALLOWED_KINDS.iter().any(|k| *k == kind) {
        return Err(format!(
            "kind {kind:?} not supported; allowed: {}",
            ALLOWED_KINDS.join(", ")
        ));
    }
    let name = name.ok_or("descriptor missing `name`")?;
    let x = x.ok_or("descriptor missing `x`")?;
    let y = y.ok_or("descriptor missing `y`")?;
    let width = width.ok_or("descriptor missing `width`")?;
    let height = height.ok_or("descriptor missing `height`")?;
    if width < 0 || height < 0 {
        return Err("width / height must be non-negative".into());
    }
    if let Some(ref hex) = fill_hex {
        if !validate_hex(hex) {
            return Err(format!(
                "fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {hex:?}"
            ));
        }
    }
    Ok(BatchInsertItem {
        kind,
        name,
        x,
        y,
        width,
        height,
        fill_hex,
    })
}

/// Reverse the JSON-string escaping the wire parser left intact.
/// Handles `\"` / `\\` / `\n` / `\t` / `\r` / `\/`. Anything else
/// passes through verbatim (no `\u` decode today).
fn unescape_wire_string(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            match next {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'/' => out.push('/'),
                _ => {
                    // Unknown escape — pass through verbatim so
                    // typos surface as parser errors downstream.
                    out.push('\\');
                    out.push(next as char);
                }
            }
            i += 2;
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\\' {
                i += 1;
            }
            let slice = std::str::from_utf8(&bytes[start..i])
                .map_err(|_| "invalid UTF-8 in nodes_json".to_string())?;
            out.push_str(slice);
        }
    }
    Ok(out)
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn parse_string(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    if *i >= bytes.len() || bytes[*i] != b'"' {
        return Err("expected string".into());
    }
    *i += 1;
    let mut out = String::new();
    while *i < bytes.len() {
        let c = bytes[*i];
        if c == b'"' {
            *i += 1;
            return Ok(out);
        }
        if c == b'\\' {
            *i += 1;
            if *i >= bytes.len() {
                return Err("unterminated escape".into());
            }
            let esc = bytes[*i];
            match esc {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'/' => out.push('/'),
                other => return Err(format!("unsupported escape \\{}", other as char)),
            }
            *i += 1;
        } else {
            // Find the next escape/quote and slice so multi-byte
            // chars stay intact (per-byte append would split them).
            let start = *i;
            while *i < bytes.len() && bytes[*i] != b'"' && bytes[*i] != b'\\' {
                *i += 1;
            }
            let slice = std::str::from_utf8(&bytes[start..*i])
                .map_err(|_| "invalid UTF-8 in string".to_string())?;
            out.push_str(slice);
        }
    }
    Err("unterminated string".into())
}

fn parse_int(bytes: &[u8], i: &mut usize) -> Result<i32, String> {
    let start = *i;
    if *i < bytes.len() && bytes[*i] == b'-' {
        *i += 1;
    }
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    if start == *i {
        return Err("expected integer".into());
    }
    let raw = std::str::from_utf8(&bytes[start..*i])
        .map_err(|_| "invalid UTF-8 in integer".to_string())?;
    raw.parse::<i32>()
        .map_err(|_| format!("expected i32, got {raw:?}"))
}
