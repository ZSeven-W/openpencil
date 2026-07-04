//! Shared QuickJS script→program runner (feature `script`, native-only).
//!
//! One implementation for BOTH callers: the orchestrator's script-gen
//! subagent path and external `batch_design(script)` calls. The script may
//! only cause effects through the bound `I(parent, obj)` recorder; the
//! result is a `batch_design` operations program executed by the existing
//! `batch_program` executor. Hard limits guard externally-supplied scripts:
//! memory, wall-clock interrupt, recorded-line cap, and source-size cap.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::{Context, Function, Runtime};

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

const PRELUDE: &str = r#"
globalThis.I = function (parent, obj) {
  return __record(parent == null ? "null" : String(parent), JSON.stringify(obj));
};
globalThis.K = function (kitComponentId, parent, overrides) {
  return __recordK(JSON.stringify(String(kitComponentId)), parent == null ? "null" : String(parent), JSON.stringify(overrides == null ? {} : overrides));
};
var __stubSeq = 0;
function __opStub() { __stubSeq += 1; return "stub" + __stubSeq; }
globalThis.C = __opStub;
globalThis.U = __opStub;
globalThis.D = __opStub;
globalThis.M = __opStub;
globalThis.R = __opStub;
globalThis.G = __opStub;
var __noop = function () {};
globalThis.console = { log: __noop, warn: __noop, error: __noop, info: __noop, debug: __noop };
"#;

/// Strip fences, enforce caps, eval, and return the recorded program.
/// Retries once with `repair_truncated_script` when the first eval fails,
/// so a model-truncated script salvages its complete-statement prefix
/// instead of losing the whole section to a trailing SyntaxError.
pub fn run_script_to_program(text: &str) -> Result<String, String> {
    let script = strip_fences(text);
    if script.trim().is_empty() {
        return Err("script is empty after stripping fences".into());
    }
    if script.len() > MAX_SCRIPT_BYTES {
        return Err(format!(
            "script too large: {} bytes (max {MAX_SCRIPT_BYTES})",
            script.len()
        ));
    }
    let program = match eval_to_program(&script) {
        Ok(p) => p,
        Err(first_err) => match repair_truncated_script(&script) {
            Some(repaired) => match eval_to_program(&repaired) {
                Ok(p) => {
                    tracing::warn!(
                        original_len = script.len(),
                        repaired_len = repaired.len(),
                        "script failed as-is; truncation repair salvaged a runnable prefix"
                    );
                    p
                }
                Err(_) => return Err(first_err),
            },
            None => return Err(first_err),
        },
    };
    if program.trim().is_empty() {
        return Err("script emitted no I(...) operations".into());
    }
    Ok(program)
}

/// Eval in a fresh limited QuickJS context. Ok(program) on success OR on a
/// mid-run throw with a non-empty recording (partial salvage); Err only when
/// nothing was recorded.
fn eval_to_program(script: &str) -> Result<String, String> {
    let rt = Runtime::new().map_err(|e| format!("js runtime: {e}"))?;
    rt.set_memory_limit(MEMORY_LIMIT_BYTES);
    let deadline = Instant::now() + EVAL_BUDGET;
    rt.set_interrupt_handler(Some(Box::new(move || Instant::now() > deadline)));

    let ctx = Context::full(&rt).map_err(|e| format!("js context: {e}"))?;
    let lines: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let counter: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let bytes_used: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let lines_rec = lines.clone();
    let counter_rec = counter.clone();
    let bytes_rec = bytes_used.clone();
    let lines_rec_k = lines.clone();
    let counter_rec_k = counter.clone();
    let bytes_rec_k = bytes_used.clone();

    let outcome: Result<(), String> = ctx.with(|ctx| {
        let record = Function::new(ctx.clone(), move |parent: String, json: String| -> String {
            push_recorded_line(&lines_rec, &counter_rec, &bytes_rec, |bind| {
                format!("{bind}=I({parent}, {json})")
            })
        })
        .map_err(|e| format!("bind __record: {e}"))?;
        let record_k = Function::new(
            ctx.clone(),
            move |kit: String, parent: String, json: String| -> String {
                push_recorded_line(&lines_rec_k, &counter_rec_k, &bytes_rec_k, |bind| {
                    format!("{bind}=K({kit}, {parent}, {json})")
                })
            },
        )
        .map_err(|e| format!("bind __recordK: {e}"))?;
        ctx.globals()
            .set("__record", record)
            .map_err(|e| format!("set __record: {e}"))?;
        ctx.globals()
            .set("__recordK", record_k)
            .map_err(|e| format!("set __recordK: {e}"))?;
        ctx.eval::<(), _>(PRELUDE)
            .map_err(|e| format!("prelude: {e}"))?;
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
        Err(e) if program.trim().is_empty() => Err(e),
        Err(e) => {
            tracing::warn!(
                error = %e,
                recorded_ops = lines.borrow().len(),
                "script threw mid-run; salvaging the nodes recorded before the throw"
            );
            Ok(program)
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
    // Mirrors the line-count cap: bindings keep incrementing so later
    // calls still resolve, but once either cap is met the line stops
    // accumulating. The byte cap is enforced on the WHOLE line BEFORE
    // pushing (check-then-add would let one arbitrarily large line
    // overshoot the advertised cap by its own size), and it latches:
    // a refused line ends recording so the program stays a clean
    // prefix — no holes referencing bindings whose insert was dropped.
    if n < MAX_RECORDED_LINES && bytes_used.get() < MAX_RECORDED_BYTES {
        let line = build(&bind);
        if bytes_used.get() + line.len() <= MAX_RECORDED_BYTES {
            bytes_used.set(bytes_used.get() + line.len());
            lines.borrow_mut().push(line);
        } else {
            bytes_used.set(MAX_RECORDED_BYTES);
        }
    }
    bind
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

fn describe_js_error(ctx: &rquickjs::Ctx<'_>, err: rquickjs::Error) -> String {
    if err.is_exception() {
        let caught = ctx.catch();
        if let Some(exc) = caught.as_exception() {
            let msg = exc.message().unwrap_or_default();
            if !msg.is_empty() {
                return format!("script error: {msg}");
            }
        }
        if let Some(s) = caught.as_string().and_then(|s| s.to_string().ok()) {
            return format!("script error: {s}");
        }
        return "script error: uncaught JS exception".to_string();
    }
    format!("script error: {err}")
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
