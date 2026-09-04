//! Lenient JSON argument parsing for the `batch_design` DSL executor:
//! the strict-then-repair `parse_json_arg` pipeline, its bracket /
//! quote repair helpers, and the node-body → `PenNode` step.
//!
//! Split out of `batch_program.rs` to stay under the 800-line cap.

use jian_ops_schema::node::PenNode;
use regex::Regex;
use serde_json::Value;

use super::batch_design::{ensure_node_ids, normalize_node_shape};
use super::batch_program::Result;
use super::batch_program_error::ProgramError;

// --- Node JSON --------------------------------------------------------

/// Parse + normalize an I()/R() node body into a `PenNode` with authored ids
/// filled in (the caller remaps them to final ids). `document_root` controls
/// the one refine pass that is valid only for a real document root; child
/// insertion payloads still receive every subtree-safe post-process fix.
pub(crate) fn parse_node_json(
    raw: &str,
    post_process: bool,
    document_root: bool,
) -> Result<PenNode> {
    let mut value = parse_json_arg(raw)?;
    if !value.is_object() {
        return Err(ProgramError::Json("node data must be a JSON object".into()));
    }
    normalize_node_shape(&mut value);
    let mut tmp = 1usize;
    ensure_node_ids(&mut value, &mut tmp);
    let mut node: PenNode = serde_json::from_value(value)
        .map_err(|e| ProgramError::InvalidNode(format!("invalid PenNode payload: {e}")))?;
    if post_process {
        // TS postProcess hooks (emoji strip, unique ids, layout-child
        // position sanitize, screen-bounds clamp) — the deterministic
        // subset shipped in `command_refine.rs`.
        if document_root {
            let _ = op_editor_core::command_refine::refine_subtree(&mut node);
        } else {
            let _ = op_editor_core::command_refine::refine_child_subtree(&mut node);
        }
    }
    Ok(node)
}

/// TS `parseJsonArg` — strict JSON first, then the lenient agent-typo
/// pipeline: quote unquoted keys, single→double quote delimiters,
/// strip empty-key artifacts and trailing commas.
pub(crate) fn parse_json_arg(raw: &str) -> Result<Value> {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }
    // `(?<=\{|,)\s*(\w+)\s*:` has a lookbehind; the Rust regex crate
    // doesn't support those, so capture + reinsert the brace/comma.
    let mut normalized = regex(r"([{,])\s*(\w+)\s*:")
        .replace_all(trimmed, "$1 \"$2\":")
        .into_owned();
    normalized = replace_single_quote_delimiters(&normalized);
    // Repair the common weak-model typo where a string value's closing quote
    // fuses with the trailing comma — `"k":"700,"next":...` meant
    // `"k":"700","next":...`. Anchored to a following `"<word>"` so it only
    // fires when the next token looks like a new key (a valid `"a","b"` has the
    // value's own close-quote before the comma and never matches).
    normalized = regex(r#":\s*"([^"]*),"(\w+)""#)
        .replace_all(&normalized, r#":"${1}","${2}""#)
        .into_owned();
    // Repair a string value missing its OPENING quote — `"width":fill_container"`
    // meant `"width":"fill_container"` (weak model dropped the leading `"`).
    // Anchored: a letter-led bareword that ENDS with a `"` right after a colon
    // (a valid value starts WITH the quote, so `:"x"` never matches; bare
    // `true`/`false`/`null` aren't followed by a `"` so they never match either).
    normalized = regex(r#":(\s*)([A-Za-z][\w-]*)""#)
        .replace_all(&normalized, r#":${1}"${2}""#)
        .into_owned();
    // Repair a FULLY-unquoted string value — `"width":fill_container_str,` meant
    // `"width":"fill_container_str"` (a weak model emitted a bare identifier, e.g.
    // a leaked JS variable name). A letter-led bareword between a colon and a
    // `,`/`}`/`]`. Numbers are digit-led (never match); a real quoted value
    // starts with `"` (never matches); `true`/`false`/`null` get re-unquoted next.
    normalized = regex(r#":(\s*)([A-Za-z][\w-]*)(\s*[,}\]])"#)
        .replace_all(&normalized, r#":${1}"${2}"${3}"#)
        .into_owned();
    normalized = regex(r#":(\s*)"(true|false|null)"(\s*[,}\]])"#)
        .replace_all(&normalized, r#":${1}${2}${3}"#)
        .into_owned();
    normalized = regex(r#",\s*""\s*:\s*[^,}\]]+"#)
        .replace_all(&normalized, "")
        .into_owned();
    normalized = regex(r",(\s*[}\]])")
        .replace_all(&normalized, "$1")
        .into_owned();
    // Last resort: a weak model commonly drops the trailing closing brace(s) of a
    // node — especially one with a nested object like `"stroke":{"thickness":{...}`
    // — leaving `{...}` short a `}`. When that node is the program's ROOT binding,
    // the parse failure cascades (every child's `I(rootBinding, ...)` then can't
    // find its parent). Auto-closing the unbalanced brackets recovers it.
    serde_json::from_str(&normalized)
        .or_else(|_| serde_json::from_str(&close_unbalanced_brackets(&normalized)))
        .map_err(|e| {
            let snippet: String = raw.chars().take(300).collect();
            let ellipsis = if raw.chars().count() > 300 { "..." } else { "" };
            ProgramError::Json(format!("Failed to parse JSON ({e}): {snippet}{ellipsis}"))
        })
}

pub(crate) fn parse_string_arg(raw: &str, label: &str) -> Result<String> {
    let value = parse_json_arg(raw)?;
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| ProgramError::Syntax(format!("{label} must be a JSON string")))
}

/// Append the closing brackets for any `{`/`[` left open at end-of-string
/// (respecting string literals + escapes), in correct nesting order. A no-op on
/// already-balanced input. This is a best-effort repair for weak models that
/// drop trailing `}` / `]`; it cannot recover a value truncated mid-token, but
/// it rescues the dominant "forgot the closers" shape.
fn close_unbalanced_brackets(s: &str) -> String {
    let mut stack: Vec<char> = Vec::new();
    let mut in_string: Option<char> = None;
    let mut escape = false;
    for ch in s.chars() {
        if escape {
            escape = false;
            continue;
        }
        if let Some(quote) = in_string {
            if ch == '\\' {
                escape = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => in_string = Some(ch),
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                stack.pop();
            }
            _ => {}
        }
    }
    if stack.is_empty() {
        return s.to_string();
    }
    let mut out = s.to_string();
    while let Some(closer) = stack.pop() {
        out.push(closer);
    }
    out
}

/// TS `replaceSingleQuoteDelimiters` — swap single-quote string
/// delimiters for double quotes, leaving apostrophes inside
/// double-quoted strings alone.
fn replace_single_quote_delimiters(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    let mut in_double = false;
    let mut in_single = false;
    while let Some(ch) = chars.next() {
        if ch == '\\' && (in_double || in_single) {
            out.push(ch);
            if let Some(next) = chars.next() {
                out.push(next);
            }
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            }
            out.push(ch);
        } else if in_single {
            if ch == '\'' {
                in_single = false;
                out.push('"');
            } else {
                out.push(ch);
            }
        } else if ch == '"' {
            in_double = true;
            out.push(ch);
        } else if ch == '\'' {
            in_single = true;
            out.push('"');
        } else {
            out.push(ch);
        }
    }
    out
}

pub(crate) fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static DSL regex must compile")
}
