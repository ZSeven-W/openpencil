//! Shared QuickJS script→program runner (feature `script`, native-only).
//!
//! One implementation for BOTH callers: the orchestrator's script-gen
//! subagent path and external `batch_design(script)` calls. The script may
//! only cause effects through the bound `I(parent, obj)`, `K(...)`, and
//! `U(nodeId, patch)` recorders; the result is a `batch_design` operations
//! program executed by the existing `batch_program` executor. Hard limits
//! guard externally-supplied scripts:
//! memory, wall-clock interrupt, recorded-line cap, and source-size cap.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::{Context, Function, Runtime};

#[path = "script_runner_error.rs"]
mod error;

#[path = "script_runner_dotted_keys.rs"]
mod dotted_keys;

#[path = "script_runner_prelude.rs"]
mod prelude;

use prelude::PRELUDE;

pub use error::ScriptError;

pub const MAX_SCRIPT_BYTES: usize = 262_144;
pub const MAX_RECORDED_LINES: usize = 4096;
/// Bounds recorded-line *bytes*, independent of the line-count cap: a script
/// can stay well under `MAX_RECORDED_LINES` while each line embeds a huge
/// `JSON.stringify` payload (e.g. a multi-MB string literal), ballooning host
/// memory even though QuickJS itself stays inside `MEMORY_LIMIT_BYTES` (the
/// recorded `String`/`Vec` lives outside the JS heap the runtime limit
/// governs).
pub const MAX_RECORDED_BYTES: usize = 8 * 1024 * 1024;
const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const EVAL_BUDGET: Duration = Duration::from_secs(2);

/// Strip fences, enforce caps, eval, and return the recorded program.
/// Retries once with `repair_truncated_script` when the first eval fails,
/// so a model-truncated script salvages its complete-statement prefix
/// instead of losing the whole section to a trailing SyntaxError.
pub fn run_script_to_program(text: &str) -> Result<String, ScriptError> {
    let script = strip_fences(text);
    if script.trim().is_empty() {
        return Err(ScriptError::EmptySource);
    }
    if script.len() > MAX_SCRIPT_BYTES {
        return Err(ScriptError::SourceTooLarge {
            bytes: script.len(),
            max: MAX_SCRIPT_BYTES,
        });
    }
    let program = match eval_to_program(&script) {
        Ok(p) => p,
        Err(first_err) => eval_after_initial_failure(&script, first_err)?,
    };
    if program.trim().is_empty() {
        return Err(ScriptError::NoOperations);
    }
    Ok(program)
}

/// The first substantial declaration statement recurring later in the
/// source marks a whole-script echo; return the text up to (excluding)
/// the second occurrence. `None` when no duplication is present.
fn truncate_duplicate_script(script: &str) -> Option<String> {
    let needle = script.lines().find_map(|line| {
        let trimmed = line.trim();
        (trimmed.len() >= 20
            && (trimmed.starts_with("const ")
                || trimmed.starts_with("let ")
                || trimmed.starts_with("var ")))
        .then(|| {
            // Byte 60 may fall inside a multi-byte char (CJK node names
            // are routine in generated scripts); back up to a boundary.
            let mut end = trimmed.len().min(60);
            while !trimmed.is_char_boundary(end) {
                end -= 1;
            }
            &trimmed[..end]
        })
    })?;
    let first = script.find(needle)?;
    let after = first + needle.len();
    let second_rel = script[after..].find(needle)?;
    Some(script[..after + second_rel].to_string())
}

fn eval_after_initial_failure(script: &str, first_err: ScriptError) -> Result<String, ScriptError> {
    // gemini-3.6-flash writes schema property names with the separator it
    // reads in the docs — `justify.content:` instead of `justifyContent:`.
    // A bare dotted key is a SyntaxError at the first `.`, so QuickJS
    // rejected an otherwise-correct slide before recording one `I(...)`.
    // Normalize FIRST, then let the rest of the ladder work on the repaired
    // source: a script can be both mis-keyed and truncated.
    let script = match dotted_keys::repair_dotted_object_keys(script) {
        Some(repaired) => match eval_to_program(&repaired) {
            Ok(p) => {
                tracing::warn!(
                    original_len = script.len(),
                    repaired_len = repaired.len(),
                    "script failed as-is; dotted-property-key repair recovered a runnable source"
                );
                return Ok(p);
            }
            Err(_) => repaired,
        },
        None => script.to_string(),
    };
    let script = match escape_raw_newlines_in_quoted_strings(&script) {
        Some(repaired) => match eval_to_program(&repaired) {
            Ok(p) => {
                tracing::warn!(
                    original_len = script.len(),
                    repaired_len = repaired.len(),
                    "script failed as-is; raw newline repair recovered a quoted string"
                );
                return Ok(p);
            }
            Err(_) => repaired,
        },
        None => script,
    };
    let script = script.as_str();

    // GLM-5.2 commonly drops the outer `}` when `stroke:{...}` is the final
    // property of an I() object, so QuickJS reaches `)` with `{` still open.
    let balanced = balance_brackets(script);
    if balanced != script {
        if let Ok(p) = eval_to_program(&balanced) {
            tracing::warn!(
                original_len = script.len(),
                repaired_len = balanced.len(),
                "script failed as-is; bracket balance repair recovered a runnable source"
            );
            return Ok(p);
        }
    }

    // DeepSeek V4 sometimes ECHOES the whole script a second time, glued
    // straight onto the first copy (often mid-line after a trailing
    // comment) - the re-declared `const` bindings then throw "invalid
    // redefinition of lexical identifier" and the whole section is lost.
    // Detect the first declaration recurring and run the first copy alone.
    if let Some(deduped) = truncate_duplicate_script(script) {
        if let Ok(p) = eval_to_program(&deduped) {
            tracing::warn!(
                original_len = script.len(),
                deduped_len = deduped.len(),
                "script failed as-is; duplicate-echo truncation recovered the first copy"
            );
            return Ok(p);
        }
    }

    match repair_truncated_script(script) {
        Some(repaired) => match eval_to_program(&repaired) {
            Ok(p) => {
                tracing::warn!(
                    original_len = script.len(),
                    repaired_len = repaired.len(),
                    "script failed as-is; truncation repair salvaged a runnable prefix"
                );
                Ok(p)
            }
            Err(_) => Err(first_err),
        },
        None => Err(first_err),
    }
}

/// A normal JavaScript string cannot contain a literal line break. This shape
/// appears when an outer `run_code` template evaluates `\\n` before passing the
/// inner QuickJS source. Escape only line breaks found inside single/double
/// quoted strings; template literals and comments keep their original bytes.
fn escape_raw_newlines_in_quoted_strings(src: &str) -> Option<String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Normal,
        Single,
        Double,
        Template,
        LineComment,
        BlockComment,
    }

    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut state = State::Normal;
    let mut escaped = false;
    let mut changed = false;

    while let Some(ch) = chars.next() {
        match state {
            State::Normal => {
                out.push(ch);
                match ch {
                    '\'' => state = State::Single,
                    '"' => state = State::Double,
                    '`' => state = State::Template,
                    '/' if chars.peek() == Some(&'/') => {
                        out.push(chars.next().expect("peeked slash"));
                        state = State::LineComment;
                    }
                    '/' if chars.peek() == Some(&'*') => {
                        out.push(chars.next().expect("peeked star"));
                        state = State::BlockComment;
                    }
                    _ => {}
                }
            }
            State::Single | State::Double => {
                let quote = if state == State::Single { '\'' } else { '"' };
                if escaped {
                    out.push(ch);
                    escaped = false;
                } else if ch == '\\' {
                    out.push(ch);
                    escaped = true;
                } else if ch == quote {
                    out.push(ch);
                    state = State::Normal;
                } else if ch == '\n' {
                    out.push_str("\\n");
                    changed = true;
                } else if ch == '\r' {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    out.push_str("\\n");
                    changed = true;
                } else {
                    out.push(ch);
                }
            }
            State::Template => {
                out.push(ch);
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '`' {
                    state = State::Normal;
                }
            }
            State::LineComment => {
                out.push(ch);
                if ch == '\n' || ch == '\r' {
                    state = State::Normal;
                }
            }
            State::BlockComment => {
                out.push(ch);
                if ch == '*' && chars.peek() == Some(&'/') {
                    out.push(chars.next().expect("peeked slash"));
                    state = State::Normal;
                }
            }
        }
    }

    changed.then_some(out)
}

/// Eval in a fresh limited QuickJS context. A runtime throw is always an
/// error, even when earlier I/K/U calls were recorded. The caller executes
/// the returned program transactionally, so returning a prefix here would
/// turn an incomplete JavaScript transaction into a misleading success.
/// Syntax-level truncation recovery remains in `eval_after_initial_failure`.
fn eval_to_program(script: &str) -> Result<String, ScriptError> {
    let rt = Runtime::new().map_err(|e| ScriptError::RuntimeInit(e.to_string()))?;
    rt.set_memory_limit(MEMORY_LIMIT_BYTES);
    let deadline = Instant::now() + EVAL_BUDGET;
    rt.set_interrupt_handler(Some(Box::new(move || Instant::now() > deadline)));

    let ctx = Context::full(&rt).map_err(|e| ScriptError::ContextInit(e.to_string()))?;
    let lines: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let counter: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let bytes_used: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let lines_rec = lines.clone();
    let counter_rec = counter.clone();
    let bytes_rec = bytes_used.clone();
    let lines_rec_k = lines.clone();
    let counter_rec_k = counter.clone();
    let bytes_rec_k = bytes_used.clone();
    let lines_rec_u = lines.clone();
    let bytes_rec_u = bytes_used.clone();

    let outcome: Result<(), ScriptError> = ctx.with(|ctx| {
        let record = Function::new(ctx.clone(), move |parent: String, json: String| -> String {
            push_recorded_line(&lines_rec, &counter_rec, &bytes_rec, |bind| {
                format!("{bind}=I({parent}, {json})")
            })
        })
        .map_err(|e| ScriptError::BindHostFn {
            name: "__record",
            detail: e.to_string(),
        })?;
        let record_k = Function::new(
            ctx.clone(),
            move |kit: String, parent: String, json: String| -> String {
                push_recorded_line(&lines_rec_k, &counter_rec_k, &bytes_rec_k, |bind| {
                    format!("{bind}=K({kit}, {parent}, {json})")
                })
            },
        )
        .map_err(|e| ScriptError::BindHostFn {
            name: "__recordK",
            detail: e.to_string(),
        })?;
        let record_u = Function::new(
            ctx.clone(),
            move |node_id: String, json: String| -> String {
                push_recorded_operation(
                    &lines_rec_u,
                    &bytes_rec_u,
                    format!("U({node_id}, {json})"),
                );
                node_id
            },
        )
        .map_err(|e| ScriptError::BindHostFn {
            name: "__recordU",
            detail: e.to_string(),
        })?;
        ctx.globals()
            .set("__record", record)
            .map_err(|e| ScriptError::SetGlobal {
                name: "__record",
                detail: e.to_string(),
            })?;
        ctx.globals()
            .set("__recordK", record_k)
            .map_err(|e| ScriptError::SetGlobal {
                name: "__recordK",
                detail: e.to_string(),
            })?;
        ctx.globals()
            .set("__recordU", record_u)
            .map_err(|e| ScriptError::SetGlobal {
                name: "__recordU",
                detail: e.to_string(),
            })?;
        ctx.eval::<(), _>(PRELUDE)
            .map_err(|e| ScriptError::Prelude(e.to_string()))?;
        ctx.eval::<(), _>(script)
            .map_err(|e| describe_js_error(&ctx, e))
    });
    let program = lines.borrow().join("\n");
    if bytes_used.get() >= MAX_RECORDED_BYTES && !program.trim().is_empty() {
        tracing::warn!(
            recorded_bytes = bytes_used.get(),
            recorded_ops = lines.borrow().len(),
            "script recorded output hit the byte cap; truncating to the recorded prefix"
        );
    }
    match outcome {
        Ok(()) => Ok(program),
        // Matched on the rendered message (not the variant) so the guard
        // keeps the exact reach the `String` version had: the sentinel is
        // raised by the prelude's `__unsupported` thrower, so it can only
        // ever arrive inside a script-throw payload.
        Err(e) if e.to_string().contains("OP_SCRIPT_MODE_UNSUPPORTED") => Err(e),
        Err(e) => {
            tracing::warn!(
                error = %e,
                recorded_ops = lines.borrow().len(),
                "script threw mid-run; discarding the recorded prefix"
            );
            Err(e)
        }
    }
}

fn push_recorded_line(
    lines: &Rc<RefCell<Vec<String>>>,
    counter: &Rc<Cell<usize>>,
    bytes_used: &Rc<Cell<usize>>,
    build: impl FnOnce(&str) -> String,
) -> String {
    let n = counter.get();
    counter.set(n + 1);
    let bind = format!("b{n}");
    push_recorded_operation(lines, bytes_used, build(&bind));
    bind
}

fn push_recorded_operation(
    lines: &Rc<RefCell<Vec<String>>>,
    bytes_used: &Rc<Cell<usize>>,
    line: String,
) {
    // Bindings keep incrementing independently of unbound U() calls, while
    // the cap applies to every recorded operation. The byte cap is enforced
    // on the WHOLE line BEFORE pushing and latches after an oversized line,
    // keeping the returned program a clean prefix.
    if lines.borrow().len() >= MAX_RECORDED_LINES || bytes_used.get() >= MAX_RECORDED_BYTES {
        return;
    }
    if bytes_used.get() + line.len() <= MAX_RECORDED_BYTES {
        bytes_used.set(bytes_used.get() + line.len());
        lines.borrow_mut().push(line);
    } else {
        bytes_used.set(MAX_RECORDED_BYTES);
    }
}

fn balance_brackets(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut stack: Vec<char> = Vec::new();
    let mut chars = src.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' | '\'' | '`' => {
                out.push(ch);
                let quote = ch;
                let mut escaped = false;
                for next in chars.by_ref() {
                    out.push(next);
                    if escaped {
                        escaped = false;
                    } else if next == '\\' {
                        escaped = true;
                    } else if next == quote {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                out.push(ch);
                if let Some(next) = chars.next() {
                    out.push(next);
                }
                for next in chars.by_ref() {
                    out.push(next);
                    if next == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                out.push(ch);
                if let Some(next) = chars.next() {
                    out.push(next);
                }
                let mut prev = '\0';
                for next in chars.by_ref() {
                    out.push(next);
                    if prev == '*' && next == '/' {
                        break;
                    }
                    prev = next;
                }
            }
            '(' | '[' | '{' => {
                stack.push(ch);
                out.push(ch);
            }
            ')' | ']' | '}' => {
                while let Some(&open) = stack.last() {
                    if closer_for(open) == ch {
                        break;
                    }
                    out.push(closer_for(open));
                    stack.pop();
                }
                if stack.last().is_some_and(|&open| closer_for(open) == ch) {
                    stack.pop();
                }
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }

    while let Some(open) = stack.pop() {
        out.push(closer_for(open));
    }
    out
}

fn closer_for(open: char) -> char {
    match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => unreachable!("stack only contains bracket openers"),
    }
}

/// Best-effort repair for a model-truncated script: cut back to the last
/// complete statement boundary (`;` or `}` seen outside strings/comments),
/// drop the trailing fragment, and append closers for brackets still open
/// at the cut. Returns None when no strictly-shorter cut exists.
pub(crate) fn repair_truncated_script(script: &str) -> Option<String> {
    let bytes = script.as_bytes();
    let mut in_str: Option<u8> = None; // b'\'' | b'"' | b'`'
    let mut escaped = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut stack: Vec<u8> = Vec::new();
    let mut last_cut: Option<(usize, usize)> = None; // (byte index AFTER boundary, stack depth)

    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_line_comment {
            if c == b'\n' {
                in_line_comment = false;
            }
        } else if in_block_comment {
            if c == b'*' && bytes.get(i + 1) == Some(&b'/') {
                in_block_comment = false;
                i += 1;
            }
        } else if let Some(q) = in_str {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == q {
                in_str = None;
            }
        } else {
            match c {
                b'\'' | b'"' | b'`' => in_str = Some(c),
                b'/' if bytes.get(i + 1) == Some(&b'/') => in_line_comment = true,
                b'/' if bytes.get(i + 1) == Some(&b'*') => in_block_comment = true,
                b'{' | b'(' | b'[' => stack.push(c),
                b'}' | b')' | b']' => {
                    stack.pop();
                    if c == b'}' {
                        last_cut = Some((i + 1, stack.len()));
                    }
                }
                b';' => last_cut = Some((i + 1, stack.len())),
                _ => {}
            }
        }
        i += 1;
    }

    let clean_end = in_str.is_none() && !in_block_comment && stack.is_empty();
    let trailing = last_cut.is_none_or(|(idx, _)| script[idx..].trim().is_empty());
    if clean_end && trailing {
        return None; // nothing dangling — a repair would change nothing
    }
    let (cut_at, _) = last_cut?;
    // Re-scan the kept prefix to find brackets still open at the cut.
    let prefix = &script[..cut_at];
    let mut open: Vec<u8> = Vec::new();
    let mut in_str2: Option<u8> = None;
    let mut esc2 = false;
    let mut line_c = false;
    let mut block_c = false;
    let pb = prefix.as_bytes();
    let mut j = 0;
    while j < pb.len() {
        let c = pb[j];
        if line_c {
            if c == b'\n' {
                line_c = false;
            }
        } else if block_c {
            if c == b'*' && pb.get(j + 1) == Some(&b'/') {
                block_c = false;
                j += 1;
            }
        } else if let Some(q) = in_str2 {
            if esc2 {
                esc2 = false;
            } else if c == b'\\' {
                esc2 = true;
            } else if c == q {
                in_str2 = None;
            }
        } else {
            match c {
                b'\'' | b'"' | b'`' => in_str2 = Some(c),
                b'/' if pb.get(j + 1) == Some(&b'/') => line_c = true,
                b'/' if pb.get(j + 1) == Some(&b'*') => block_c = true,
                b'{' | b'(' | b'[' => open.push(c),
                b'}' | b')' | b']' => {
                    open.pop();
                }
                _ => {}
            }
        }
        j += 1;
    }
    let mut repaired = prefix.to_string();
    for b in open.iter().rev() {
        repaired.push(match b {
            b'{' => '}',
            b'(' => ')',
            _ => ']',
        });
    }
    Some(repaired)
}

/// A bare "x is not defined" tells the model nothing about WHY. Every script
/// runs in a FRESH sandbox, so a variable that held a node in an earlier batch
/// is gone — the model must reference that node by its id STRING. Say so
/// (measured 2026-07-12: a batch died on `header is not defined`, where
/// `header` was a `const` from the previous batch's script).
fn explain_reference_error(msg: &str) -> Option<ScriptError> {
    let name = msg.strip_suffix(" is not defined")?;
    Some(ScriptError::StaleReference {
        message: msg.to_string(),
        name: name.to_string(),
    })
}

fn describe_js_error(ctx: &rquickjs::Ctx<'_>, err: rquickjs::Error) -> ScriptError {
    if err.is_exception() {
        let caught = ctx.catch();
        if let Some(exc) = caught.as_exception() {
            let msg = exc.message().unwrap_or_default();
            if !msg.is_empty() {
                if let Some(explained) = explain_reference_error(&msg) {
                    return explained;
                }
                return ScriptError::Threw(msg);
            }
        }
        if let Some(s) = caught.as_string().and_then(|s| s.to_string().ok()) {
            return ScriptError::Threw(s);
        }
        return ScriptError::UncaughtException;
    }
    ScriptError::Threw(err.to_string())
}

/// Strip reasoning-model `<think>…</think>` blocks, returning the real
/// answer. Mirrors `op_orchestrator::parse::strip_reasoning` (kept local —
/// op-mcp is below op-orchestrator in the crate graph): a reasoning model
/// (MiniMax-M3, DeepSeek-R) emits `<think>…</think>` before its script, and
/// the think body itself is full of draft JS. Take everything after the LAST
/// closing tag, then drop any trailing unclosed `<think>` (a think block
/// truncated by max_tokens).
fn strip_reasoning(text: &str) -> &str {
    let after_closed = ["</think>", "</thinking>"]
        .iter()
        .filter_map(|tag| text.rfind(tag).map(|i| i + tag.len()))
        .max()
        .map(|i| &text[i..])
        .unwrap_or(text);
    match ["<think>", "<thinking>"]
        .iter()
        .filter_map(|tag| after_closed.find(tag))
        .min()
    {
        Some(start) => &after_closed[..start],
        None => after_closed,
    }
}

/// Extract the JavaScript program from a raw model response.
///
/// Robust to what real models actually emit around the script — not just a
/// clean fenced block at position zero:
/// 1. Reasoning `<think>…</think>` is stripped first. A model that keeps its
///    reasoning (M3 rides `Adaptive`) prefixes the script with a think block
///    full of draft JS; feeding that to QuickJS is a guaranteed syntax error,
///    which used to drop the model onto the fragile flat-JSONL retry rung
///    (measured: a full travel page collapsed to 44 flat siblings).
/// 2. A ```` ``` ```` fenced block is extracted from ANYWHERE — models add a
///    prose preamble ("Here's the design:") before the fence, so a
///    start-anchored strip missed it and passed the prose to the runtime.
/// 3. No fence → the reasoning-stripped text is the script (bare-script case).
fn strip_fences(text: &str) -> String {
    let text = strip_reasoning(text).trim();
    if let Some(open) = text.find("```") {
        // Drop the ``` and any language tag on the fence line (```js).
        let after_open = &text[open + 3..];
        let body = after_open.split_once('\n').map(|x| x.1).unwrap_or("");
        // Body ends at the next closing fence; if the response was truncated
        // mid-block there is no closing fence, so keep the runnable prefix and
        // let `repair_truncated_script` salvage it.
        let body = body.rsplit_once("```").map(|x| x.0).unwrap_or(body);
        return body.trim().to_string();
    }
    text.to_string()
}

#[cfg(test)]
#[path = "script_runner_tests.rs"]
mod tests;

#[cfg(test)]
mod duplicate_echo_tests {
    use super::run_script_to_program;

    /// DeepSeek V4 measured shape (2026-07-12): the model echoed the whole
    /// script a second time, glued mid-line after a trailing comment — the
    /// re-declared consts threw "invalid redefinition" and a valid section
    /// was lost to retries.
    #[test]
    fn duplicated_script_echo_runs_the_first_copy() {
        let single = r#"const sec = I(null, {type:"frame", name:"Main Workspace", width:"fill_container"});
const bar = I(sec, {type:"frame", name:"Top Bar", height:56});
"#;
        let doubled = format!("{single}// keep the section around 150px.{single}");
        let program = run_script_to_program(&doubled).expect("first copy salvaged");
        assert_eq!(
            program.lines().count(),
            2,
            "exactly the first copy's two inserts: {program}"
        );
    }
}
