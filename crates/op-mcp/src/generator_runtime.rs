//! The generator runtime (feature `script`): runs a generator node's
//! program with the SAME sandbox and node parser `batch_design { script }`
//! uses, so a generator has exactly the node vocabulary agents already
//! know and no second language exists.
//!
//! Pipeline:
//!   1. QuickJS eval through `script_runner::eval_recorded` — 64 MiB heap,
//!      a wall-clock interrupt ([`GENERATOR_TIME_BUDGET`]), the recorder's
//!      line / byte caps (a capped run is REFUSED, never truncated), and
//!      no IO: the sandbox exposes nothing but the recorders.
//!   2. A generator preamble rebinds the globals: `params` (frozen),
//!      `root` (the insert target), a plain `I(parent, node)` WITHOUT the
//!      model-repair heuristics of the agent prelude (a user's program
//!      gets exactly the colours it wrote), `U()` / `K()` disabled, a
//!      `Math.random` seeded from the generator id, and a frozen clock
//!      (`Date.now() == 0`, `new Date()` is the epoch) so output depends
//!      only on program + params.
//!   3. The recorded `I(...)` lines are assembled into a tree with the
//!      executor's own node parser / normalizer (`parse_node_json`, the
//!      exact vocabulary `batch_design` accepts) and the same container
//!      rule. The full `batch_program` executor is NOT used: it
//!      re-simulates the document per line (quadratic — ~7 s for 1 000
//!      nodes in a debug build), and a generator's output is insert-only
//!      under a fresh root, so a linear pass is equivalent. op-editor-core
//!      then derives the final ids.
//!
//! Why this engine rather than the bare `I(...)` DSL: the DSL has no
//! loops, conditionals, or arithmetic, so "N cards from a table" or a
//! month grid cannot be expressed in it. QuickJS is native-only; the
//! browser bundle does not install a runtime and treats generators as
//! read-only materialized frames, which is exactly why the output is
//! stored as ordinary children.

use std::collections::HashMap;
use std::time::Duration;

use jian_ops_schema::node::PenNode;
use op_editor_core::generator::{GeneratorError, GeneratorRequest, MAX_GENERATED_NODES};
use op_editor_core::PenNodeExt;

use crate::batch_program_node_parse::parse_node_json;
use crate::script_runner::{eval_recorded, ScriptError, MAX_RECORDED_LINES};

/// Wall-clock budget for one generator run.
pub const GENERATOR_TIME_BUDGET: Duration = Duration::from_millis(1_500);

/// The program's `root` global: the parent token meaning "the generator".
const ROOT_TOKEN: &str = "genroot";

/// Install [`run_generator`] as this process's generator runtime.
pub fn install() -> bool {
    op_editor_core::generator::install_generator_runner(run_generator)
}

/// Run one generator program; returns the raw children it built.
pub fn run_generator(request: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
    request.spec.validate()?;
    let preamble = preamble(request);
    let recorded = eval_recorded(
        &request.spec.program,
        Some(&preamble),
        GENERATOR_TIME_BUDGET,
    )
    .map_err(map_script_error)?;
    if recorded.capped {
        return Err(GeneratorError::TooManyNodes {
            count: MAX_RECORDED_LINES + 1,
            max: MAX_GENERATED_NODES,
        });
    }
    build_tree(&recorded.program, request.frame)
}

/// One recorded insert, held until the tree is assembled.
struct Pending {
    node: PenNode,
    children: Vec<usize>,
}

/// Assemble the recorded `bN=I("parent", {json})` lines into the root's
/// children. Lines are newline-separated and every payload comes from
/// `JSON.stringify`, which never emits a raw newline.
fn build_tree(program: &str, frame: &PenNode) -> Result<Vec<PenNode>, GeneratorError> {
    let lines: Vec<&str> = program
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.len() > MAX_GENERATED_NODES {
        return Err(GeneratorError::TooManyNodes {
            count: lines.len(),
            max: MAX_GENERATED_NODES,
        });
    }
    let mut arena: Vec<Pending> = Vec::with_capacity(lines.len());
    let mut roots: Vec<usize> = Vec::new();
    let mut bindings: HashMap<&str, usize> = HashMap::new();
    for line in lines {
        let (binding, parent, json) = split_insert(line)?;
        let node = parse_node_json(json, false, false)
            .map_err(|error| GeneratorError::Program(error.to_string()))?;
        let index = arena.len();
        match bindings.get(parent.as_str()) {
            Some(&parent_index) if arena[parent_index].node.is_container() => {
                arena[parent_index].children.push(index);
            }
            None if parent == ROOT_TOKEN && frame.is_container() => roots.push(index),
            _ => return Err(not_a_container(&parent)),
        }
        arena.push(Pending {
            node,
            children: Vec::new(),
        });
        bindings.insert(binding, index);
    }
    let mut slots: Vec<Option<Pending>> = arena.into_iter().map(Some).collect();
    Ok(roots
        .into_iter()
        .map(|index| assemble(index, &mut slots))
        .collect())
}

fn not_a_container(parent: &str) -> GeneratorError {
    GeneratorError::Program(format!(
        "Insert parent not found or not a container: {parent}"
    ))
}

/// Children are always recorded after their parent, so a depth-first
/// assembly takes every slot exactly once.
fn assemble(index: usize, slots: &mut [Option<Pending>]) -> PenNode {
    let Pending { mut node, children } = slots[index]
        .take()
        .expect("each recorded node has exactly one parent");
    if !children.is_empty() {
        let built: Vec<PenNode> = children
            .into_iter()
            .map(|child| assemble(child, slots))
            .collect();
        if let Some(slot) = node.children_mut() {
            slot.extend(built);
        }
    }
    node
}

/// `bN=I("parent", {json})` → (`bN`, parent, json).
fn split_insert(line: &str) -> Result<(&str, String, &str), GeneratorError> {
    let malformed = || GeneratorError::Program(format!("unrecognized operation: {line}"));
    let (binding, rest) = line.split_once("=I(").ok_or_else(malformed)?;
    let body = rest.strip_suffix(')').ok_or_else(malformed)?;
    let mut stream = serde_json::Deserializer::from_str(body).into_iter::<String>();
    let parent = stream.next().and_then(Result::ok).ok_or_else(malformed)?;
    let json = body[stream.byte_offset()..]
        .trim_start()
        .strip_prefix(',')
        .ok_or_else(malformed)?;
    Ok((binding.trim(), parent, json))
}

fn preamble(request: &GeneratorRequest<'_>) -> String {
    let params = request.spec.params_object();
    let seed = request
        .generator_id
        .bytes()
        .fold(0x811c_9dc5u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
        });
    format!(
        r#"(function () {{
  var s = {seed} | 0;
  Math.random = function () {{
    s = (s + 0x6D2B79F5) | 0;
    var t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }};
  var RealDate = Date;
  function FrozenDate() {{
    if (arguments.length === 0) return new RealDate(0);
    var args = [null];
    for (var i = 0; i < arguments.length; i++) args.push(arguments[i]);
    return new (Function.prototype.bind.apply(RealDate, args))();
  }}
  FrozenDate.UTC = RealDate.UTC;
  FrozenDate.parse = RealDate.parse;
  FrozenDate.now = function () {{ return 0; }};
  FrozenDate.prototype = RealDate.prototype;
  globalThis.Date = FrozenDate;
}})();
globalThis.params = Object.freeze({params});
globalThis.root = {root};
globalThis.I = function (parent, node) {{
  if (parent === undefined || parent === null) {{
    throw new Error("I(parent, node): generators insert under root or a binding returned by I()");
  }}
  if (node === null || typeof node !== "object" || Array.isArray(node)) {{
    throw new Error("I(parent, node): node must be an object");
  }}
  return __record(JSON.stringify(String(parent)), JSON.stringify(node));
}};
globalThis.K = function () {{
  throw new Error("K() kit components are not available in generators; build nodes with I()");
}};
globalThis.U = function () {{
  throw new Error("U() is not available in generators; build nodes with I()");
}};
"#,
        root = serde_json::Value::String(ROOT_TOKEN.to_string()),
    )
}

fn map_script_error(error: ScriptError) -> GeneratorError {
    match &error {
        ScriptError::Threw(message) if message.contains("interrupted") => {
            GeneratorError::TimeLimit {
                budget_ms: GENERATOR_TIME_BUDGET.as_millis() as u64,
            }
        }
        ScriptError::Threw(message) if message.contains("out of memory") => {
            GeneratorError::Program("out of memory (64 MiB limit)".into())
        }
        _ => GeneratorError::Program(error.to_string()),
    }
}

#[cfg(test)]
#[path = "generator_runtime_tests.rs"]
mod tests;
