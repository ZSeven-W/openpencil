//! Full-Pencil JS-script generation path (`OPENPENCIL_SCRIPT_GEN`).
//!
//! The sub-agent writes a REAL JavaScript program (the Pencil `batch_design`
//! model): `const row = I(parent, {...}); for (const r of data) I(row, {...})`.
//! `I` is a Rust function bound into an embedded QuickJS (rquickjs) engine. It
//! does NOT build nodes in JS — it records one `batch_design` DSL line
//! (`b{n}=I({parent}, {JSON.stringify(obj)})`) and returns a synthetic binding
//! name, so after the script runs we hand the assembled program to the SAME
//! executor `program_gen` uses ([`crate::program_gen::run_program_to_forest`]).
//!
//! Two wins over hand-authored JSON (the program-DSL path):
//!   - LOOPS: the JS engine provides `for`/`map`, so a 50-row table is a loop
//!     over a data array, not 50 hand-written lines that a weak model truncates.
//!   - NO JSON TYPOS: the engine serializes each object to perfect JSON, so the
//!     brace/quote/missing-comma typo long tail (which the program-DSL parser has
//!     to repair) simply cannot occur.
//!
//! Native-only (op-orchestrator sits behind the `design` feature wasm never
//! enables), so rquickjs's bundled C engine never reaches the web bundle.
//! Sandbox: `I` is the only effectful builder; `console` + Pencil's other
//! `batch_design` ops (`C/U/D/M/R/G`) are no-op stubs so a stray model call can't
//! abort the script. No fs / net / eval / module escape is exposed.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use jian_ops_schema::node::PenNode;
use rquickjs::{Context, Function, Runtime};

/// JS prelude defining the global `I(parent, obj)`. It does `JSON.stringify` in
/// JS-land (engine-native → perfect JSON, no hand-typed-brace risk) and hands a
/// pair of plain STRINGS to the Rust recorder `__record` — so the Rust closure
/// never touches a `Value`/`Ctx` (sidestepping rquickjs's invariant `'js`).
///
/// Beyond `I`, the prelude STUBS the rest of Pencil's `batch_design` op set
/// (`C/U/D/M/R/G`) and `console`. A weak model is not actually bad at JS — a
/// probe confirmed `Math`/`Date`/template-literals/`.map` all run fine — but it
/// is trained on / inclined toward Pencil's FULL op vocabulary (Pencil's own
/// free backend is pencil-minimax-m3) and habitually calls `console.log`. In a
/// bare sandbox that only defines `I`, the FIRST such call is a `ReferenceError`
/// that aborts the WHOLE script and loses the entire section. The stubs are
/// no-ops returning a fresh synthetic id, so a stray `console.log` / `G(...)` /
/// `C(...)` degrades to "that one op did nothing" instead of nuking everything.
/// (`C/U/D/M/R` operate on pre-existing nodes — meaningless when building a fresh
/// section from scratch — and image-gen `G` isn't wired here; the meaningful op
/// is `I`. The prompt still steers toward `I` only.)
const PRELUDE: &str = r#"
globalThis.I = function (parent, obj) {
  return __record(parent == null ? "null" : String(parent), JSON.stringify(obj));
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

/// Whether the sub-agent should emit an executable JS script for `model`.
///
/// Script-gen (real JavaScript with loops, run in QuickJS) is now **opt-in
/// only** — set `OPENPENCIL_SCRIPT_GEN=1` to force it. The default weak-model
/// protocol is program-DSL ([`crate::program_gen`]): the same
/// parent-by-reference nesting, but authored as explicit `I(parent, {...})`
/// ops (no loops — matching Pencil's actual `batch_design` output) and parsed
/// **best-effort per line**.
///
/// That resilience is why program-DSL replaced script-gen as the default. A
/// weak model that stops or garbles mid-program loses only the trailing op;
/// but a single QuickJS `SyntaxError` (e.g. a script the model truncated
/// mid-token) throws away the ENTIRE section, and the retry then falls back to
/// flat JSONL — which flattens a table's cells into full-width siblings of the
/// row. Keeping script-gen behind an env flag preserves the A-B lever without
/// exposing production to that all-or-nothing cliff.
pub fn script_gen_enabled_for_model(_model: &str) -> bool {
    std::env::var("OPENPENCIL_SCRIPT_GEN")
        .map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "on"))
        .unwrap_or(false)
}

/// Run the emitted JS program in QuickJS, collecting the `I(...)` calls into a
/// `batch_design` program, then expand that to a section forest via the shared
/// executor.
pub fn parse_script(text: &str) -> Result<Vec<PenNode>, String> {
    let script = strip_fences(text);
    if script.trim().is_empty() {
        return Err("script is empty after stripping fences".into());
    }
    let program = run_js_to_program(&script)?;
    if program.trim().is_empty() {
        return Err("script emitted no I(...) operations".into());
    }
    crate::program_gen::run_program_to_forest(&program)
}

/// Execute `script` in a fresh QuickJS context with the `I(...)` prelude loaded,
/// and return the recorded `batch_design` DSL program (one `b{n}=I(...)` line per
/// `I` call). `__record(parentStr, jsonStr) -> binding` is the only Rust-bound
/// function; the closure captures the line buffer + counter (all `'static` `Rc`,
/// no JS lifetimes).
fn run_js_to_program(script: &str) -> Result<String, String> {
    let rt = Runtime::new().map_err(|e| format!("js runtime: {e}"))?;
    let ctx = Context::full(&rt).map_err(|e| format!("js context: {e}"))?;
    let lines: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let counter: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let lines_rec = lines.clone();

    let outcome: Result<(), String> = ctx.with(|ctx| {
        let record = Function::new(ctx.clone(), move |parent: String, json: String| -> String {
            let n = counter.get();
            counter.set(n + 1);
            let bind = format!("b{n}");
            lines_rec
                .borrow_mut()
                .push(format!("{bind}=I({parent}, {json})"));
            bind
        })
        .map_err(|e| format!("bind __record: {e}"))?;
        ctx.globals()
            .set("__record", record)
            .map_err(|e| format!("set __record: {e}"))?;
        ctx.eval::<(), _>(PRELUDE)
            .map_err(|e| format!("prelude: {e}"))?;
        ctx.eval::<(), _>(script)
            .map_err(|e| describe_js_error(&ctx, e))
    });
    // Partial-success recovery: `__record` appends to `lines` as each `I(...)`
    // runs, so a script that throws PART-WAY (a late `console.log`, a typo'd
    // binding reference, an unstubbed call) has already recorded every node
    // built before the throw. Salvage those instead of discarding the whole
    // section — a stray error on line 50 must not erase 49 good nodes. Only a
    // throw that fires before ANY `I(...)` ran (empty buffer) is a hard failure.
    let program = lines.borrow().join("\n");
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

/// Turn a rquickjs eval error into a message carrying the ACTUAL JS exception
/// (message + type), not the opaque `Exception generated by QuickJS`. A weak
/// model's emitted script throws real runtime errors (`x is not defined`,
/// `Cannot read properties of undefined`); surfacing them lets the retry ladder
/// feed the cause back and lets us diagnose systematic faults. On a non-exception
/// error (e.g. syntax) fall back to the Display form.
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

/// Strip an accidental ```fence``` wrapper (a chat model sometimes adds one).
fn strip_fences(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let body = rest.split_once('\n').map(|x| x.1).unwrap_or(rest);
        let body = body.rsplit_once("```").map(|x| x.0).unwrap_or(body);
        return body.trim().to_string();
    }
    trimmed.to_string()
}

#[cfg(test)]
#[path = "script_gen_tests.rs"]
mod tests;
