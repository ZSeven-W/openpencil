//! Multi-line mixed `batch_design` DSL program executor.
//!
//! Mirrors TS `packages/pen-mcp/src/tools/batch-design-dsl.ts`
//! (`runBatchDesignDsl` + `executeLine`) for the line grammar: arbitrary
//! programs mixing I/U/C/R/G/M/D operations with shared bindings and
//! slash-path expressions, executed line by line.
//!
//! ## Line-failure policy
//!
//! The agent-facing tool surface is TRANSACTIONAL (Pencil's contract:
//! "if any operation fails, every already-executed operation of the
//! batch is rolled back"): when any line fails, NO command ships — the
//! live document is untouched, `errors[]` lists every failing line
//! (200-char preview), and the envelope carries `applied:false` plus a
//! resend hint. A half-applied batch is worse than a rejected one for a
//! model in a feedback loop: the loop's next batch would build on a tree
//! the model believes complete.
//!
//! The orchestrator's internal script-gen path opts back into the old
//! TS best-effort semantics (failing lines dropped, survivors apply)
//! via the internal `_line_policy=best_effort` arg — it runs against a
//! scratch document, surfaces drops as warnings, and has its own
//! retry/cleanup ladder downstream (`program_gen.rs`).
//!
//! ## Snapshot simulation + one host command
//!
//! TS mutates the document in-process; the Rust tool stays `&self` and
//! must hand the host applier a command. The executor therefore runs
//! every line against a CLONE of the document snapshot
//! (`sim.apply(...)` — the exact code the host runs at apply), so:
//!   - per-line success/failure matches what the host would decide,
//!   - bindings resolve to the REAL ids the host will assign (the
//!     executor assigns authored ids off the sim allocator and emits
//!     `InsertAuthoredSubtree`, the `batch_design_result.rs` id-predict
//!     discipline),
//!   - later lines observe earlier lines' mutations.
//!
//! The surviving commands ride home as ONE `EditorCommand::Batch`
//! (atomic at apply; a live-doc divergence rejects the whole batch
//! rather than landing silently-wrong ids).
//!
//! ## Documented divergences from TS
//!
//! - Inserted subtree ids are remapped to fresh editor ids (Rust id
//!   discipline); slash paths written against AUTHORED child ids keep
//!   working through an alias table (authored → final, first-wins).
//! - An `I`/`C`/`G` whose parent does not resolve to a container is a
//!   per-line ERROR; TS's `insertNodeInTree` silently drops the node
//!   while still reporting a binding for it.
//! - `M()` with a non-integer index errors; TS's `parseInt` NaN would
//!   silently splice at index 0.
//! - `C()` ignores `descendants` overrides: TS clones with fresh ids
//!   first, so override keys (source ids) never match — a no-op there,
//!   an explicit skip here.

use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::node::PenNode;
use op_editor_core::command_node::remap_subtree_ids_mapping;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use serde_json::{json, Value};

use super::batch_design::{find_top_level_char, normalize_node_shape, split_operations};
use super::batch_direct_ops::{split_top_level_args, update_command_from_value};
use super::batch_page::optional_page_id;
use super::batch_program_error::ProgramError;
use super::{EditorCommand, ToolOutcome};

// The DSL machinery lives in flat siblings carved off for the 800-line
// cap: `batch_program_exec_ops.rs` (C/K/R/G executors),
// `batch_program_parse.rs` (lenient JSON arg parsing), and
// `batch_program_resolve.rs` (binding / alias / path resolution).
use super::batch_program_exec_ops::{
    execute_copy, execute_image, execute_kit_instantiate, execute_replace,
};
use super::batch_program_parse::{parse_json_arg, parse_node_json, regex};
use super::batch_program_resolve::{
    count_forest, find_node_by_path, line_preview, lookup_id, parent_node_id, resolve_page_index,
    resolve_parent_ref, resolve_path_expr, resolve_ref, strip_outer_quotes, with_page_id,
};

/// Every fallible step of the executor fails with [`ProgramError`].
pub(crate) type Result<T> = std::result::Result<T, ProgramError>;

/// Run a mixed multi-op DSL program against the document snapshot and
/// return the TS `handleBatchDesign` envelope:
/// `{ results, nodeCount, postProcessed?, errors? }`.
pub(crate) fn run_batch_design_program(
    snapshot: &EditorState,
    operations: &str,
    args: &BTreeMap<String, String>,
) -> ToolOutcome {
    let page_id = optional_page_id(args);
    // TS batch_design `postProcess` defaults to false (unlike
    // design_content's default-true).
    let post_process = args
        .get("postProcess")
        .or_else(|| args.get("post_process"))
        .map(|raw| matches!(raw.trim(), "true" | "1"))
        .unwrap_or(false);
    let lines = split_operations(operations);
    let mut ctx = ProgramCtx {
        sim: snapshot.clone(),
        page_id: page_id.clone(),
        bindings: BTreeMap::new(),
        alias: BTreeMap::new(),
        results: Vec::new(),
        commands: Vec::new(),
        post_process,
        auto_seq: 0,
        current_line: 0,
        explicitly_sized_append_lines: explicitly_sized_append_lines(&lines),
        replaceable_empty_root_ids: Vec::new(),
        id_high_water: 0,
    };
    // Pin the sim's active page to the requested page so sim READS
    // (path lookups, node counts) see the same children every emitted
    // command targets via its `page_id` field. An unknown page id is
    // left to per-command apply rejection (consistent per-line errors).
    if let Some(raw) = page_id.as_deref() {
        if let Some(index) = resolve_page_index(&ctx.sim, raw) {
            let _ = ctx.sim.apply(EditorCommand::SetActivePage {
                index: index as u32,
            });
        }
    }
    // Only roots that were empty BEFORE this program began are starter
    // placeholders. An empty root inserted by an earlier line is a real
    // sibling screen, not a new placeholder for the next I(null, ...) line.
    // Without this snapshot, a three-screen shell batch repeatedly replaced
    // its own previous insert and silently kept only the final screen.
    ctx.replaceable_empty_root_ids = ctx
        .sim
        .active_children()
        .iter()
        .filter(|node| is_replaceable_starter_root(node))
        .map(|node| node.id_str().to_string())
        .collect();
    // Live-doc node count BEFORE any line runs — the honest `nodeCount`
    // for a rolled-back transaction (nothing will have been applied).
    let baseline_count = count_forest(ctx.sim.active_children());
    // Internal knob for the orchestrator's script-gen path (see module
    // doc); absent → transactional, the agent-facing contract.
    let transactional = args.get("_line_policy").map(String::as_str) != Some("best_effort");

    let mut errors: Vec<Value> = Vec::new();
    for (line_index, line) in lines.into_iter().enumerate() {
        ctx.current_line = line_index;
        if let Err(error) = execute_line(&line, &mut ctx) {
            errors.push(json!({ "line": line_preview(&line), "error": error.to_string() }));
        }
    }

    if transactional && !errors.is_empty() {
        // Roll the whole batch back: drop every recorded command so the
        // host applies NOTHING. Bindings/results are dropped too — their
        // node ids never land, and reporting them would invite the model
        // to reference phantom nodes in its next batch.
        let mut envelope = serde_json::Map::new();
        envelope.insert("results".into(), Value::Array(Vec::new()));
        envelope.insert("nodeCount".into(), json!(baseline_count));
        envelope.insert("applied".into(), Value::Bool(false));
        // The hint carries the FIRST failure verbatim. `errors` already holds
        // every line and reason, but a caller that renders only the hint (a
        // host tool card, a transcript summary) otherwise shows "N failed"
        // with no way to act — measured: a model burned six corrective rounds
        // re-sending near-identical batches and inventing a wrong theory about
        // the validator, when the real reason named the offending node.
        let first = errors.first().map(|entry| {
            format!(
                "First failure — {}: {}",
                entry
                    .get("line")
                    .and_then(Value::as_str)
                    .unwrap_or("<line>"),
                entry
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("<reason>"),
            )
        });
        envelope.insert(
            "hint".into(),
            json!(format!(
                "Transaction rolled back: {} operation(s) failed, so NONE of this batch was \
                 applied — the document is unchanged. Fix the failing line(s) and resend the \
                 whole corrected batch.{}",
                errors.len(),
                first.map(|line| format!(" {line}")).unwrap_or_default()
            )),
        );
        envelope.insert("errors".into(), Value::Array(errors));
        return ToolOutcome::OkJson(Value::Object(envelope).to_string());
    }

    let node_count = count_forest(ctx.sim.active_children());
    let mut envelope = serde_json::Map::new();
    envelope.insert("results".into(), Value::Array(ctx.results));
    envelope.insert("nodeCount".into(), json!(node_count));
    if post_process {
        envelope.insert("postProcessed".into(), Value::Bool(true));
    }
    if !errors.is_empty() {
        envelope.insert("errors".into(), Value::Array(errors));
    }
    let json = Value::Object(envelope).to_string();

    let mut commands = ctx.commands;
    match commands.len() {
        0 => ToolOutcome::OkJson(json),
        1 => ToolOutcome::OkJsonWithCommand(json, commands.remove(0)),
        _ => ToolOutcome::OkJsonWithCommand(json, EditorCommand::Batch { commands }),
    }
}

/// The name the shipped blank starter frame carries
/// (`op_editor_core::blank_starter`). Treated as "unnamed" below: it is the
/// placeholder the auto-replace exists for, not an authored screen.
const STARTER_FRAME_NAME: &str = "Frame";

/// Whether an existing root may be consumed by a root-level `I(null, frame)`.
///
/// Emptiness alone is not enough. The pre-program snapshot stops a batch from
/// eating its OWN earlier inserts, but the snapshot is retaken every program,
/// so a shell inserted by batch N is a "pre-existing empty root" to batch N+1 —
/// measured: batch 1 inserts A and B, batch 2 inserts C and A silently
/// disappears. An authored NAME is what separates the two: a screen shell the
/// model is about to fill is named, the placeholder it is replacing is not.
/// This also matches the policy `op_editor_core::blank_starter` states for the
/// document-level twin of this question — an empty frame the user drew and has
/// not filled in yet must not be treated as disposable.
fn is_replaceable_starter_root(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_))
        && node
            .children()
            .map(|children| children.is_empty())
            .unwrap_or(true)
        && node
            .base()
            .name
            .as_deref()
            .map(str::trim)
            .is_none_or(|name| name.is_empty() || name == STARTER_FRAME_NAME)
}

pub(crate) struct ProgramCtx {
    pub(crate) sim: EditorState,
    pub(crate) page_id: Option<String>,
    /// binding name → final (host-assigned) node id.
    pub(crate) bindings: BTreeMap<String, String>,
    /// authored id → final id for remapped inserts, first-wins (TS
    /// `findNodeInTree` finds the first match in tree order).
    pub(crate) alias: BTreeMap<String, String>,
    pub(crate) results: Vec<Value>,
    pub(crate) commands: Vec<EditorCommand>,
    pub(crate) post_process: bool,
    /// Monotonic counter for `_auto_*` bindless-line bindings.
    pub(crate) auto_seq: usize,
    /// Index of the operation currently being executed.
    pub(crate) current_line: usize,
    /// Append G() lines whose result binding receives explicit positive
    /// numeric width and height later in this same program.
    pub(crate) explicitly_sized_append_lines: BTreeSet<usize>,
    /// Root placeholders that existed before this program started. Consumed
    /// at most once so newly inserted empty screen shells remain siblings.
    pub(crate) replaceable_empty_root_ids: Vec<String>,
    /// Highest id this program has handed out, so an id freed by a `D()`
    /// earlier in the SAME batch is never reissued. The document's own seed
    /// is `max(existing) + 1`, which walks backwards after a delete: a
    /// measured batch bound both the deleted badge and the image that
    /// replaced it to `n16`, leaving one binding pointing at a node the
    /// caller believes is gone.
    pub(crate) id_high_water: u64,
}

impl ProgramCtx {
    /// Emit `cmd` AND apply it to the sim. The sim apply is the line's
    /// final validation gate — the host will run the same code.
    pub(crate) fn emit(&mut self, cmd: EditorCommand, failure: &str) -> Result<()> {
        if !self.sim.apply(cmd.clone()) {
            return Err(ProgramError::ApplyRejected(failure.to_string()));
        }
        self.commands.push(cmd);
        Ok(())
    }

    pub(crate) fn bind(&mut self, binding: &str, node_id: &str) {
        self.bindings
            .insert(binding.to_string(), node_id.to_string());
        // A re-bound name (a redraft, or scratch reuse) updates its existing
        // results entry in place — consumers look bindings up by FIRST match,
        // which must never point at a superseded draft's deleted id.
        if let Some(entry) = self
            .results
            .iter_mut()
            .find(|r| r.get("binding").and_then(Value::as_str) == Some(binding))
        {
            entry["nodeId"] = json!(node_id);
        } else {
            self.results
                .push(json!({ "binding": binding, "nodeId": node_id }));
        }
    }

    /// Assign fresh sim-allocator ids to `nodes` (in place); returns
    /// the (authored → final) map. Authored ids are recorded into the
    /// alias table so slash paths keep resolving TS-style.
    pub(crate) fn remap(&mut self, nodes: &mut [PenNode]) -> Result<Vec<(String, String)>> {
        let mut seed = self
            .sim
            .next_node_id_seed()
            .ok_or(ProgramError::IdSpaceExhausted)?
            .max(self.id_high_water);
        let mut taken = self.sim.collect_node_ids();
        let map = remap_subtree_ids_mapping(nodes, &mut seed, &mut taken)
            .ok_or(ProgramError::IdSpaceExhausted)?;
        self.id_high_water = seed;
        for (old, new) in &map {
            if !old.starts_with("__op_tmp_") {
                self.alias.entry(old.clone()).or_insert_with(|| new.clone());
            }
        }
        Ok(map)
    }
}

/// TS `executeLine` — one DSL operation.
fn execute_line(line: &str, ctx: &mut ProgramCtx) -> Result<()> {
    // Strip a trailing statement/list separator before the grammar sees the
    // line: the patterns below all anchor on `\)$`, so one stray `,` makes
    // the operation unparsable. A model that reaches for a separator writes
    // it on EVERY line, so the whole batch fails and the transaction rolls
    // back — measured 2026-07-31 with five `G(...)` image fills, after which
    // the model mis-read `Cannot parse operation` as an ARGUMENT-separator
    // problem and spent the rest of the run deleting and rebuilding subtrees
    // it had already committed, ending worse than where it started. Only the
    // line's own tail is touched, so a `,` inside an argument body is safe:
    // every real operation ends on `)`.
    let line = line.trim().trim_end_matches([';', ',']).trim();
    // TS line grammar (dotAll `s` flag — pretty-printed JSON bodies
    // carry literal newlines inside the arg list):
    //   binding=OP(args)  for I/C/K/R/M/G
    //   OP(args)          for I/C/K/R/G (auto-binding) and U/D/M (call)
    let assign = regex(r"(?s)^(\w+)\s*=\s*([ICKRMG])\((.+)\)$");
    let bindless = regex(r"(?s)^([ICKRG])\((.+)\)$");
    let call = regex(r"(?s)^([UDM])\((.+)\)$");

    if let Some(c) = assign.captures(line) {
        let binding = c.get(1).map_or("", |m| m.as_str()).to_string();
        let op = c.get(2).map_or("", |m| m.as_str());
        let args = c.get(3).map_or("", |m| m.as_str());
        return execute_assign(op, &binding, args, ctx);
    }
    if let Some(c) = bindless.captures(line) {
        let op = c.get(1).map_or("", |m| m.as_str());
        let args = c.get(2).map_or("", |m| m.as_str());
        // Numbered off a dedicated counter — `results.len()` stalls when a
        // rebind updates its entry in place, and a stalled counter would hand
        // two bindless lines the SAME auto name (turning the second into a
        // phantom "redraft" of the first).
        let binding = format!("_auto_{}_{op}", ctx.auto_seq);
        ctx.auto_seq += 1;
        return execute_assign(op, &binding, args, ctx);
    }
    if let Some(c) = call.captures(line) {
        let op = c.get(1).map_or("", |m| m.as_str());
        let args = c.get(2).map_or("", |m| m.as_str());
        return match op {
            "U" => execute_update(args, ctx),
            "D" => execute_delete(args, ctx),
            "M" => execute_move(args, ctx).map(|_| ()),
            _ => unreachable!(),
        };
    }
    Err(ProgramError::UnparsableLine(line.to_string()))
}

/// Return append-G line indexes that are robustly sized by later U() calls in
/// the same program. Parsing uses the DSL's top-level delimiter rules, so an
/// '=' inside a quoted image prompt is never mistaken for a result binding.
fn explicitly_sized_append_lines(lines: &[String]) -> BTreeSet<usize> {
    let mut sized = BTreeSet::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((Some(binding), 'G', args)) = parsed_operation(line) else {
            continue;
        };
        let parts = split_top_level_args(args);
        let is_append = parts.len() == 4
            && matches!(
                serde_json::from_str::<String>(parts[3].trim()),
                Ok(placement) if placement == "append"
            );
        if !is_append {
            continue;
        }

        let mut width_is_positive_number = false;
        let mut height_is_positive_number = false;
        for later in &lines[index + 1..] {
            let Some((later_binding, op, later_args)) = parsed_operation(later) else {
                continue;
            };
            // Rebinding closes this append's sizing window. A U() beyond it
            // would target the newer node, not this image.
            if later_binding == Some(binding) {
                break;
            }
            if op != 'U' {
                continue;
            }
            let Some(comma) = find_top_level_char(later_args, ',') else {
                continue;
            };
            let target = strip_outer_quotes(later_args[..comma].trim());
            if target != binding {
                continue;
            }
            let Ok(value) = parse_json_arg(&later_args[comma + 1..]) else {
                continue;
            };
            let Some(patch) = value.as_object() else {
                continue;
            };
            if let Some(width) = patch.get("width") {
                width_is_positive_number = positive_json_number(width);
            }
            if let Some(height) = patch.get("height") {
                height_is_positive_number = positive_json_number(height);
            }
        }
        if width_is_positive_number && height_is_positive_number {
            sized.insert(index);
        }
    }
    sized
}

/// Parse one complete DSL operation without splitting on delimiters nested in
/// calls or quoted strings. Returns `(binding, opcode, argument body)`.
fn parsed_operation(line: &str) -> Option<(Option<&str>, char, &str)> {
    // A trailing `;` or `,` is noise, not syntax. The program is one
    // operation per line, but models routinely reach for a statement or
    // list separator out of JS habit — and a comma is the costlier of the
    // two to reject, because a model that writes `I(...),` writes it on
    // EVERY line, so the whole batch fails at once and the transaction
    // rolls back. Measured 2026-07-31: five `G(...)` image fills, all
    // rejected for one trailing comma each; the model could not tell from
    // `Cannot parse operation` what was wrong, guessed at the argument
    // separator instead, and spent the rest of the run deleting and
    // rebuilding subtrees it had already committed — the design ended up
    // worse than before it tried. Nothing meaningful can end a line with
    // `,`: every operation closes on `)`.
    let line = line.trim().trim_end_matches([';', ',']).trim();
    let (binding, call) = match find_top_level_char(line, '=') {
        Some(eq) => {
            let binding = line[..eq].trim();
            if binding.is_empty()
                || !binding
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return None;
            }
            (Some(binding), line[eq + 1..].trim())
        }
        None => (None, line),
    };
    let mut chars = call.chars();
    let op = chars.next()?;
    let rest = chars.as_str();
    if !rest.starts_with('(') || !call.ends_with(')') {
        return None;
    }
    Some((binding, op, &rest[1..rest.len() - 1]))
}

fn positive_json_number(value: &Value) -> bool {
    value
        .as_f64()
        .is_some_and(|number| number.is_finite() && number > 0.0)
}

fn execute_assign(op: &str, binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<()> {
    match op {
        "I" => execute_insert(binding, args, ctx),
        "C" => execute_copy(binding, args, ctx),
        "K" => execute_kit_instantiate(binding, args, ctx),
        "R" => execute_replace(binding, args, ctx),
        "G" => execute_image(binding, args, ctx),
        "M" => {
            let node_id = execute_move(args, ctx)?;
            ctx.bind(binding, &node_id);
            Ok(())
        }
        _ => unreachable!(),
    }
}

/// `binding=I(parent, data)` — insert a (possibly nested) node.
fn execute_insert(binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<()> {
    let comma = find_top_level_char(args, ',')
        .ok_or_else(|| ProgramError::Syntax("Insert requires parent and node data".into()))?;
    let parent_raw = args[..comma].trim();
    let parent = resolve_parent_ref(parent_raw, &ctx.bindings);
    let mut node = parse_node_json(&args[comma + 1..], ctx.post_process, parent.is_none())?;
    delete_superseded_draft(binding, parent.as_deref(), &node, ctx);

    // TS auto-replace: a root-level frame insert replaces the first EMPTY
    // starter root (inheriting its x/y) instead of siblinging it. Restrict the
    // candidates to the pre-program snapshot: newly inserted empty frames are
    // authored screen shells and must not replace one another.
    let mut pre_commands: Vec<(EditorCommand, &str)> = Vec::new();
    if parent.is_none() && matches!(node, PenNode::Frame(_)) {
        let replaceable = ctx.replaceable_empty_root_ids.iter().find_map(|id| {
            ctx.sim
                .active_children()
                .iter()
                .find(|candidate| {
                    candidate.id_str() == id && is_replaceable_starter_root(candidate)
                })
                .map(|empty| (id.clone(), empty.base().x, empty.base().y))
        });
        if let Some((id, x, y)) = replaceable {
            ctx.replaceable_empty_root_ids
                .retain(|candidate| candidate != &id);
            if x.is_some() {
                node.base_mut().x = x;
            }
            if y.is_some() {
                node.base_mut().y = y;
            }
            pre_commands.push((
                EditorCommand::DeleteNode {
                    node_id: NodeId::new(id),
                    page_id: ctx.page_id.clone(),
                },
                "failed to replace the empty root frame",
            ));
        }
    }

    let mut nodes = vec![node];
    let map = ctx.remap(&mut nodes)?;
    let root_id = map
        .first()
        .map(|(_, new)| new.clone())
        .ok_or(ProgramError::ProducedNoNode("Insert"))?;
    for (cmd, failure) in pre_commands {
        ctx.emit(cmd, failure)?;
    }
    // Hoist node-level `state` as a SIBLING command — the program
    // finisher batches ctx.commands itself; wrapping here would nest
    // Batches, which apply rejects. Held until the insert below
    // succeeds: emitting it first would leak an orphan `$app` state
    // command into `ctx.commands` if the insert then fails (the line
    // as a whole errors and is dropped, but a prior `ctx.emit` already
    // recorded — state from a line that never landed must not ship).
    let merge = super::batch_design::hoist_generation_state(&mut nodes);
    ctx.emit(
        EditorCommand::InsertAuthoredSubtreePreservingRoots {
            nodes,
            parent_id: parent_node_id(parent.as_deref()),
            page_id: ctx.page_id.clone(),
        },
        &format!(
            "Insert parent not found or not a container: {}",
            parent.as_deref().unwrap_or("null")
        ),
    )?;
    if let Some(merge) = merge {
        // Emit AFTER the insert succeeds (a failed line must not leak
        // orphan $app state), then swap the two recorded commands so
        // the batch still carries MergeAppState before its insert.
        // Sim-apply order between the two is immaterial: merge touches
        // only doc.state, the insert only the tree. (If this merge emit
        // itself ever failed post-insert, the line would error after
        // its insert already landed — state dropped but never
        // orphaned; an additive MergeAppState on the sim can't
        // realistically fail.)
        ctx.emit(merge, "merge generated app state")?;
        let n = ctx.commands.len();
        ctx.commands.swap(n - 1, n - 2);
    }
    ctx.bind(binding, &root_id);
    Ok(())
}

/// `U(path, data)` — shallow-patch the node at `path`. No result entry
/// (TS call-form ops don't push results).
fn execute_update(args: &str, ctx: &mut ProgramCtx) -> Result<()> {
    let comma = find_top_level_char(args, ',')
        .ok_or_else(|| ProgramError::Syntax("Update requires path and update data".into()))?;
    let path = resolve_path_expr(args[..comma].trim(), &ctx.bindings);
    let mut value = parse_json_arg(&args[comma + 1..])?;
    normalize_node_shape(&mut value);
    let Some(target) = find_node_by_path(ctx.sim.active_children(), &path, &ctx.alias) else {
        return Err(ProgramError::NotFound(format!(
            "Update target not found: {path}"
        )));
    };
    let node_id = NodeId::new(target.id_str());
    // `batch_direct_ops` is shared with the non-program write paths and is
    // outside this conversion's scope; adapt its `String` at the boundary
    // rather than rippling the change into it.
    let cmd = update_command_from_value(node_id, &value)?;
    let cmd = with_page_id(cmd, ctx.page_id.clone());
    ctx.emit(cmd, &format!("Update failed for: {path}"))
}

/// `D(ref)` — delete. TS `removeNodeFromTree` silently no-ops on an
/// unknown id: no error, no result.
fn execute_delete(args: &str, ctx: &mut ProgramCtx) -> Result<()> {
    let raw = strip_outer_quotes(args.trim());
    let node_id = lookup_id(&resolve_ref(&raw, &ctx.bindings), &ctx.alias);
    if op_editor_core::walkers::find_node(ctx.sim.active_children(), &NodeId::new(&node_id))
        .is_none()
    {
        return Ok(());
    }
    ctx.emit(
        EditorCommand::DeleteNode {
            node_id: NodeId::new(&node_id),
            page_id: ctx.page_id.clone(),
        },
        &format!("Delete failed for: {node_id}"),
    )
}

/// `M(nodeId, parent[, index])` (call or bound form) — reparent.
/// Returns the moved node's id so the bound form can record it.
fn execute_move(args: &str, ctx: &mut ProgramCtx) -> Result<String> {
    let parts = split_top_level_args(args);
    if parts.len() < 2 {
        return Err(ProgramError::Syntax(
            "Move requires nodeId and parent".into(),
        ));
    }
    let node_id = lookup_id(&resolve_ref(parts[0].trim(), &ctx.bindings), &ctx.alias);
    if op_editor_core::walkers::find_node(ctx.sim.active_children(), &NodeId::new(&node_id))
        .is_none()
    {
        return Err(ProgramError::NotFound(format!(
            "Move target not found: {node_id}"
        )));
    }
    let parent_raw = parts[1].trim();
    let parent = resolve_parent_ref(parent_raw, &ctx.bindings);
    let index = match parts.get(2) {
        None => None,
        Some(raw) => Some(
            strip_outer_quotes(raw.trim())
                .parse::<usize>()
                .map_err(|_| {
                    ProgramError::Syntax(format!(
                        "M() index must be a non-negative integer, got {raw:?}"
                    ))
                })?,
        ),
    };
    ctx.emit(
        EditorCommand::MoveNode {
            node_id: NodeId::new(&node_id),
            target_parent: parent_node_id(parent.as_deref()),
            page_id: ctx.page_id.clone(),
            index,
        },
        &format!("Move failed for: {node_id}"),
    )?;
    Ok(node_id)
}

/// A RE-USED binding whose previous node sits under the SAME parent with the
/// same type and (non-empty) name is a REDRAFT, not a new node: a weak model
/// deliberating in-channel re-emits its section several times ("Let me
/// redo…"), and appending every draft shipped SEVEN stacked navbars from one
/// response (measured: minimax-m3, test0703-m3.op). Delete the superseded
/// draft before the re-insert so the LAST draft wins. Scratch-style binding
/// reuse (`t=I(cardA, …)` then `t=I(cardB, …)` — different parent, or unnamed
/// leaves) keeps appending exactly as before; all four gates must agree
/// before anything is removed. A previous draft already gone (its ancestor
/// was itself redrafted) is skipped by the sim-apply guard.
fn delete_superseded_draft(
    binding: &str,
    parent: Option<&str>,
    node: &PenNode,
    ctx: &mut ProgramCtx,
) {
    let Some(new_name) = node.base().name.as_deref().filter(|n| !n.is_empty()) else {
        return;
    };
    let Some(prev_id) = ctx.bindings.get(binding).cloned() else {
        return;
    };
    let parent_id = parent.map(|p| lookup_id(p, &ctx.alias));
    let Some((prev, prev_parent)) =
        find_node_with_parent(ctx.sim.active_children(), &prev_id, None)
    else {
        return;
    };
    let same_shape = std::mem::discriminant(prev) == std::mem::discriminant(node)
        && prev.base().name.as_deref() == Some(new_name);
    if !same_shape || prev_parent != parent_id.as_deref() {
        return;
    }
    let del = EditorCommand::DeleteNode {
        node_id: NodeId::new(&prev_id),
        page_id: ctx.page_id.clone(),
    };
    if ctx.sim.apply(del.clone()) {
        ctx.commands.push(del);
    }
}

/// Locate `id` in the forest and return it together with its parent's id
/// (`None` = document root).
fn find_node_with_parent<'a>(
    nodes: &'a [PenNode],
    id: &str,
    parent: Option<&'a str>,
) -> Option<(&'a PenNode, Option<&'a str>)> {
    for n in nodes {
        if n.id_str() == id {
            return Some((n, parent));
        }
        if let Some(children) = n.children() {
            if let Some(hit) = find_node_with_parent(children, id, Some(n.id_str())) {
                return Some(hit);
            }
        }
    }
    None
}
