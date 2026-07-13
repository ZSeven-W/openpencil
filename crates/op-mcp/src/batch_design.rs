//! `batch_design` write tool + nodes_json parser. Carved off
//! `write_tools.rs` to stay under the 800-line cap.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::promote::{promote_frame, widget_kind_for, PromoteNote};
use op_editor_core::{NodeId, PenNodeExt};

use super::batch_direct_ops::{is_direct_image_operation, parse_single_direct_operation};
use super::batch_layered::{dispatch_design_content, dispatch_design_skeleton};
use super::batch_page::{command_with_outer_page_id, optional_page_id};
use super::write_tools::{validate_hex, ALLOWED_KINDS};
use super::{BatchInsertItem, EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

#[path = "batch_design_fill_normalize.rs"]
mod fill_normalize;
use fill_normalize::normalize_fill;

// First-party `batch_design` tool — insert N leaf nodes on the
// active page in one atomic shot. Mirrors TS `batch_design` for
// the leaf subset.
//
// Wire shape: one scalar string arg `nodes_json` carrying a JSON
// array of node descriptors. The shell-core parser rejects
// structured args at the top level (so an LLM can't sneak a
// nested object past scalar contracts), but a JSON array
// embedded inside a quoted string round-trips cleanly. Each
// array entry is `{"kind":"...","name":"...","x":N,"y":N,
// "width":N,"height":N,"fill_hex":"#..."}` — the same shape
// `insert_node` accepts, minus the wire wrapping.
//
// The tool parses the inner JSON, validates EVERY entry, and
// emits `McpCommand::BatchInsert { items: ... }`. The apply
// path is all-or-nothing: a single bad entry rejects the whole
// batch so the LLM never sees a partial design tree.
// The `batch_design` tool (BatchDesign + batch_design_snapshot) lives in
// `batch_design_result.rs` so it can hold a document snapshot and emit TS's
// `{results:[{binding,nodeId}], nodeCount}` for the operations path (it
// predicts the host-assigned ids off the snapshot). Non-operations paths fall
// back to `dispatch_batch_design` here.

/// Shared core for `design_skeleton` / `design_content` /
/// `design_refine`. Each phase tool dispatches here with a label
/// stamped into the response so the LLM client can correlate the
/// call back to its layered-workflow phase. Today every phase
/// emits the same `BatchInsert` command — the phasing is purely
/// metadata. A future patch may grow per-phase apply semantics
/// (e.g. `design_refine` patching existing nodes via UpdateNode
/// batches) once a richer command exists.
pub(crate) fn dispatch_phase(args: &BTreeMap<String, String>, phase: &'static str) -> ToolOutcome {
    dispatch_batch_design(args, Some(phase))
}

/// Whether `args[key]` carries an actual input rather than an empty
/// placeholder (`""`, `[]`, `{}`, `null`).
pub(crate) fn carries_input(args: &BTreeMap<String, String>, key: &str) -> bool {
    args.get(key).is_some_and(|value| {
        let trimmed = value.trim();
        !matches!(trimmed, "" | "[]" | "{}" | "null")
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BatchInputKind {
    Script,
    Operations,
    NodesJson,
}

pub(crate) type BatchInputError = (ToolErrorCode, String);

/// Select the one non-empty write payload. Empty placeholders do not compete,
/// but two real payloads are always an error regardless of argument order.
pub(crate) fn select_batch_input(
    args: &BTreeMap<String, String>,
) -> Result<BatchInputKind, BatchInputError> {
    let active: Vec<BatchInputKind> = [
        ("script", BatchInputKind::Script),
        ("operations", BatchInputKind::Operations),
        ("nodes_json", BatchInputKind::NodesJson),
    ]
    .into_iter()
    .filter_map(|(key, kind)| carries_input(args, key).then_some(kind))
    .collect();
    match active.as_slice() {
        [kind] => Ok(*kind),
        [] => {
            // Preserve a lone slot's own parser error (`nodes_json:{}` is
            // malformed, `script:""` is empty) while treating placeholders
            // as absent whenever another slot carries the real input.
            let present: Vec<BatchInputKind> = [
                ("script", BatchInputKind::Script),
                ("operations", BatchInputKind::Operations),
                ("nodes_json", BatchInputKind::NodesJson),
            ]
            .into_iter()
            .filter_map(|(key, kind)| args.contains_key(key).then_some(kind))
            .collect();
            match present.as_slice() {
                [kind] => Ok(*kind),
                _ => Err((
                    ToolErrorCode::MissingArgument,
                    "one non-empty input is required: script, operations, or nodes_json".into(),
                )),
            }
        }
        _ => Err((
            ToolErrorCode::InvalidArgument,
            "provide only one of script, operations, or nodes_json".into(),
        )),
    }
}

/// Expand a `script` arg into the `operations` DSL program the rest of
/// `batch_design` already understands. Returns:
/// - `None` — the one active input is `operations` or `nodes_json`; an empty
///   `script` placeholder does not steal the route.
/// - `Some(Ok(rewritten))` — `script` removed, `operations` set to the
///   program the sandboxed runner recorded. Caller re-dispatches with the
///   rewritten args so BOTH the flat `dispatch_batch_design` path (used by
///   the `design_skeleton`/`design_content`/`design_refine` phase tools)
///   and the primary `batch_design` tool's richer `operations` handling
///   (`BatchDesign::call`, which intercepts `operations` before ever
///   calling `dispatch_batch_design` — see `batch_design_result.rs`) see
///   the exact same expansion and report through their own native shape.
/// - `Some(Err(outcome))` — zero/multiple real inputs, or (feature off) a real
///   `script` input.
pub(crate) fn expand_script_arg(
    args: &BTreeMap<String, String>,
) -> Option<Result<BTreeMap<String, String>, ToolOutcome>> {
    match select_batch_input(args) {
        Ok(BatchInputKind::Script) => {}
        Ok(BatchInputKind::Operations | BatchInputKind::NodesJson) => return None,
        Err((code, message)) => return Some(Err(ToolOutcome::Err(code, message))),
    }
    let script = args.get("script").expect("selected script exists");
    #[cfg(feature = "script")]
    {
        let program = match crate::script_runner::run_script_to_program(script) {
            Ok(p) => p,
            Err(e) => return Some(Err(ToolOutcome::Err(ToolErrorCode::InvalidArgument, e))),
        };
        let mut forwarded = args.clone();
        forwarded.remove("script");
        forwarded.insert("operations".to_string(), program);
        Some(Ok(forwarded))
    }
    #[cfg(not(feature = "script"))]
    {
        let _ = script;
        Some(Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "script input requires a script-enabled host build".into(),
        )))
    }
}

pub(crate) fn dispatch_batch_design(
    args: &BTreeMap<String, String>,
    phase: Option<&'static str>,
) -> ToolOutcome {
    if let Some(result) = expand_script_arg(args) {
        return match result {
            Ok(forwarded) => dispatch_batch_design(&forwarded, phase),
            Err(outcome) => outcome,
        };
    }
    let input = match select_batch_input(args) {
        Ok(input) => input,
        Err((code, message)) => return ToolOutcome::Err(code, message),
    };
    let page_id = optional_page_id(args);
    if input == BatchInputKind::Operations {
        let operations = args.get("operations").expect("selected operations exists");
        if let Some(phase) = phase.filter(|_| is_direct_image_operation(operations)) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "design_{phase} legacy operations cannot execute G() safely: this compatibility path has no document snapshot, so it cannot enforce G() placement or target geometry. Use batch_design with the same operations payload."
                ),
            );
        }
        return match parse_operations(operations) {
            Ok(ParsedOperations::Insert {
                parent_id,
                mut nodes,
                count,
                promoted,
                ..
            }) => {
                let mut out = BTreeMap::new();
                out.insert("wrote".into(), "true".into());
                out.insert("count".into(), count.to_string());
                if let Some(phase) = phase {
                    out.insert("phase".into(), phase.into());
                }
                // Surface Phase E3 promotions so the client sees the
                // legacy role frames that were normalized into widget nodes.
                surface_promotions(&mut out, &promoted);
                let hoist = hoist_generation_state(&mut nodes);
                ToolOutcome::OkWithCommand(
                    out,
                    with_hoisted_state(
                        hoist,
                        EditorCommand::InsertSubtree {
                            nodes,
                            parent_id,
                            page_id,
                        },
                    ),
                )
            }
            Ok(ParsedOperations::Direct(command)) => {
                let mut out = BTreeMap::new();
                out.insert("wrote".into(), "true".into());
                out.insert("count".into(), "1".into());
                if let Some(phase) = phase {
                    out.insert("phase".into(), phase.into());
                }
                ToolOutcome::OkWithCommand(out, command_with_outer_page_id(command, page_id))
            }
            Err(e) => ToolOutcome::Err(ToolErrorCode::InvalidArgument, e),
        };
    }
    let raw = args
        .get("nodes_json")
        .expect("selected nodes_json exists after script expansion");
    match parse_batch_items(raw) {
        Ok(items) if items.is_empty() => ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "nodes_json must contain at least one descriptor".into(),
        ),
        Ok(items) => {
            let mut out = BTreeMap::new();
            out.insert("wrote".into(), "true".into());
            out.insert("count".into(), items.len().to_string());
            if let Some(phase) = phase {
                out.insert("phase".into(), phase.into());
            }
            ToolOutcome::OkWithCommand(out, EditorCommand::BatchInsert { items, page_id })
        }
        Err(e) => ToolOutcome::Err(ToolErrorCode::InvalidArgument, e),
    }
}

struct ParsedInsert {
    binding: String,
    parent: ParentRef,
    node: PenNode,
}

enum ParentRef {
    Root,
    Ref(String),
}

pub(crate) enum ParsedOperations {
    Insert {
        parent_id: NodeId,
        nodes: Vec<PenNode>,
        count: usize,
        /// One binding name per top-level `I()` op, used to trace post-remap
        /// ids back to bindings for TS's `results:[{binding,nodeId}]`.
        bindings: Vec<String>,
        /// Per-node legacy-frame promotions applied to `nodes` (Phase E3).
        /// Empty when the AI emitted no explicitly-marked role frames.
        promoted: Vec<PromoteNote>,
    },
    Direct(EditorCommand),
}

pub(crate) fn parse_operations(input: &str) -> Result<ParsedOperations, String> {
    let lines = split_operations(input);
    if lines.len() == 1 {
        if let Some(command) = parse_single_direct_operation(&lines[0])? {
            return Ok(ParsedOperations::Direct(command));
        }
    }
    let (parent_id, mut nodes, _count, bindings) = parse_insert_operations(input)?;
    // Phase E3 — normalize explicitly-marked legacy frames (`role:"input"`
    // etc., or `semantics.role == input`) into first-class widget nodes
    // BEFORE they become the inserted command, so an old-style
    // `frame role="input"` the AI emits lands a real `text_input` node. Both
    // consumers (flat `InsertSubtree` + the `BatchDesign` result path's
    // `InsertAuthoredSubtree`) see the promoted forest. Recount afterwards:
    // promotion drops the marked frame's children (widget nodes are leaves).
    let mut promoted = Vec::new();
    promote_in_slice(&mut nodes, &mut promoted);
    let count = count_forest(&nodes);
    Ok(ParsedOperations::Insert {
        parent_id,
        nodes,
        count,
        bindings,
        promoted,
    })
}

/// Recursive promotion pass mirroring `jian_ops_schema::promote::
/// promote_document`'s internal slice walker (which isn't `pub`): for every
/// node, if `widget_kind_for` flags it as an explicitly-marked frame, replace
/// it in place with the built widget node; otherwise recurse into container
/// children (Frame / Group / Rectangle / Tabs / Ref). Widget nodes are leaves,
/// so a promoted frame is never recursed into. `notes` collects one
/// `PromoteNote` per promotion for the result surface.
pub(crate) fn promote_in_slice(nodes: &mut [PenNode], notes: &mut Vec<PromoteNote>) {
    for node in nodes.iter_mut() {
        if let Some(kind) = widget_kind_for(node) {
            let PenNode::Frame(frame) = node.clone() else {
                // `widget_kind_for` only returns Some for a Frame.
                continue;
            };
            let from_role = frame
                .base
                .role
                .clone()
                .unwrap_or_else(|| "semantics.role=input".into());
            let id = frame.base.id.clone();
            *node = promote_frame(&frame, kind);
            notes.push(PromoteNote {
                node_id: id,
                from_role,
                to: kind.tag(),
            });
        } else if let Some(children) = node.children_mut() {
            promote_in_slice(children, notes);
        }
    }
}

/// Drain node-level `state` from a generated insert forest into one
/// doc-root [`EditorCommand::MergeAppState`], tagged with the weakest
/// (unplanned) priority — MCP inserts have no orchestrator plan index.
/// Returns `None` when no node declared state, so plain inserts keep
/// their existing single-command shape.
pub(crate) fn hoist_generation_state(nodes: &mut [PenNode]) -> Option<EditorCommand> {
    let cmd = op_editor_core::hoist_app_state(nodes, op_editor_core::UNPLANNED_APP_STATE_IDX);
    match &cmd {
        EditorCommand::MergeAppState { state, .. } if !state.is_empty() => Some(cmd),
        _ => None,
    }
}

/// Wrap `insert` in a [`EditorCommand::Batch`] carrying the hoisted
/// `MergeAppState` FIRST (so `$app` keys land before the nodes that
/// reference them), or return `insert` unchanged when nothing was
/// hoisted. `MergeAppState` allocates no node ids, so prepending it
/// never disturbs id prediction.
pub(crate) fn with_hoisted_state(
    hoist: Option<EditorCommand>,
    insert: EditorCommand,
) -> EditorCommand {
    match hoist {
        Some(merge) => EditorCommand::Batch {
            commands: vec![merge, insert],
        },
        None => insert,
    }
}

/// Count every node in a forest (subtree-inclusive). Used to keep the flat
/// `count` accurate after promotion drops a marked frame's children.
fn count_forest(nodes: &[PenNode]) -> usize {
    fn count_subtree(node: &PenNode) -> usize {
        1 + node
            .children()
            .map(|c| c.iter().map(count_subtree).sum::<usize>())
            .unwrap_or(0)
    }
    nodes.iter().map(count_subtree).sum()
}

/// Stamp Phase E3 promotion info into a flat string-map result. No-op when
/// nothing was promoted, so existing batch_design results are byte-identical
/// for the common (no legacy frames) case. The `promoted` line mirrors TS's
/// pipeline-warning convention ("promoted N legacy role frames"); a per-node
/// `<id>` → `<widget>` summary rides alongside for traceability.
pub(crate) fn surface_promotions(out: &mut BTreeMap<String, String>, promoted: &[PromoteNote]) {
    if promoted.is_empty() {
        return;
    }
    out.insert("promoted".into(), promoted.len().to_string());
    let detail = promoted
        .iter()
        .map(|n| format!("{}({} -> {})", n.node_id, n.from_role, n.to))
        .collect::<Vec<_>>()
        .join(", ");
    out.insert("promotedNodes".into(), detail);
}

type InsertForest = (NodeId, Vec<PenNode>, usize, Vec<String>);

fn parse_insert_operations(input: &str) -> Result<InsertForest, String> {
    let lines = split_operations(input);
    if lines.is_empty() {
        return Err("operations must contain at least one I(parent, node) operation".into());
    }
    let mut inserts = Vec::new();
    let mut binding_to_idx = BTreeMap::new();
    let mut tmp_id = 1usize;
    for (line_idx, line) in lines.iter().enumerate() {
        let (binding, parent, data) = parse_insert_operation(line, line_idx)?;
        if binding_to_idx.contains_key(&binding) {
            return Err(format!("duplicate binding {binding:?}"));
        }
        let mut value: serde_json::Value =
            serde_json::from_str(data).map_err(|e| format!("{binding}: invalid node JSON: {e}"))?;
        normalize_node_shape(&mut value);
        ensure_node_ids(&mut value, &mut tmp_id);
        let mut node: PenNode = serde_json::from_value(value)
            .map_err(|e| format!("{binding}: invalid PenNode payload: {e}"))?;
        // Stamp the binding as the node's authored id so the post-insert remap
        // (which the `batch_design` tool simulates) can be traced back to its
        // binding for the TS `results:[{binding,nodeId}]` map. The host remaps
        // every id at apply, so this authored id is transient + harmless.
        node.base_mut().id = binding.clone();
        binding_to_idx.insert(binding.clone(), inserts.len());
        inserts.push(ParsedInsert {
            binding,
            parent,
            node,
        });
    }
    let bindings: Vec<String> = inserts.iter().map(|i| i.binding.clone()).collect();
    let (parent_id, nodes, count) = assemble_insert_forest(inserts, &binding_to_idx)?;
    Ok((parent_id, nodes, count, bindings))
}

fn assemble_insert_forest(
    inserts: Vec<ParsedInsert>,
    binding_to_idx: &BTreeMap<String, usize>,
) -> Result<(NodeId, Vec<PenNode>, usize), String> {
    let mut children_by_parent = vec![Vec::<usize>::new(); inserts.len()];
    let mut roots = Vec::<usize>::new();
    let mut real_parent: Option<NodeId> = None;
    for (idx, item) in inserts.iter().enumerate() {
        match &item.parent {
            ParentRef::Root => roots.push(idx),
            ParentRef::Ref(raw) => {
                if let Some(parent_idx) = binding_to_idx.get(raw).copied() {
                    if parent_idx == idx {
                        return Err(format!("{} cannot be inserted under itself", item.binding));
                    }
                    children_by_parent[parent_idx].push(idx);
                } else {
                    let parent_id = root_or_node_id(raw);
                    if parent_id.is_real() {
                        match &real_parent {
                            Some(existing) if existing != &parent_id => {
                                return Err(
                                    "operations can target only one existing parent per call"
                                        .into(),
                                );
                            }
                            None => real_parent = Some(parent_id),
                            _ => {}
                        }
                    }
                    roots.push(idx);
                }
            }
        }
    }
    if roots.is_empty() {
        return Err("operations must include at least one root insert".into());
    }
    let mut visit = vec![0u8; inserts.len()];
    let mut nodes = Vec::with_capacity(roots.len());
    for root in roots {
        nodes.push(build_tree(root, &inserts, &children_by_parent, &mut visit)?);
    }
    Ok((real_parent.unwrap_or(NodeId::NONE), nodes, inserts.len()))
}

fn build_tree(
    idx: usize,
    inserts: &[ParsedInsert],
    children_by_parent: &[Vec<usize>],
    visit: &mut [u8],
) -> Result<PenNode, String> {
    match visit[idx] {
        1 => return Err("operations contain a parent cycle".into()),
        2 => return Ok(inserts[idx].node.clone()),
        _ => {}
    }
    visit[idx] = 1;
    let mut node = inserts[idx].node.clone();
    for child_idx in &children_by_parent[idx] {
        let child = build_tree(*child_idx, inserts, children_by_parent, visit)?;
        let Some(children) = node.children_mut() else {
            return Err(format!(
                "binding {:?} cannot receive children because it is not a container",
                inserts[idx].binding
            ));
        };
        children.push(child);
    }
    visit[idx] = 2;
    Ok(node)
}

fn parse_insert_operation(line: &str, index: usize) -> Result<(String, ParentRef, &str), String> {
    let trimmed = line.trim().trim_end_matches(';').trim();
    let (binding, call) = match find_top_level_char(trimmed, '=') {
        Some(eq) => {
            let binding = trimmed[..eq].trim();
            if !is_binding(binding) {
                return Err(format!("invalid binding {binding:?}"));
            }
            (binding.to_string(), trimmed[eq + 1..].trim())
        }
        None => (format!("_auto_{index}_I"), trimmed),
    };
    if !call.starts_with("I(") || !call.ends_with(')') {
        return Err(format!(
            "{binding}: only I(parent, node) operations are supported"
        ));
    }
    let body = &call[2..call.len() - 1];
    let Some(comma) = find_top_level_char(body, ',') else {
        return Err(format!("{binding}: I() requires parent and node JSON"));
    };
    let parent = parse_parent_ref(body[..comma].trim())?;
    let data = body[comma + 1..].trim();
    if data.is_empty() {
        return Err(format!("{binding}: node JSON is empty"));
    }
    Ok((binding, parent, data))
}

/// Returns `true` when a physical line begins a new DSL operation —
/// `name=I(...)`, `I(...)`, `U(...)`, `D(...)`, `M(...)`, `C(...)`, `R(...)`,
/// `G(...)` (with an optional `binding =` prefix). Continuation lines of a
/// pretty-printed JSON body (`"key": value,`) never match, so they accumulate
/// onto the current operation.
fn line_starts_operation(line: &str) -> bool {
    let mut s = line.trim_start();
    if let Some(eq) = s.find('=') {
        let head = s[..eq].trim();
        if !head.is_empty() && head.chars().all(|c| c.is_alphanumeric() || c == '_') {
            s = s[eq + 1..].trim_start();
        }
    }
    let mut chars = s.chars();
    match chars.next() {
        Some('I' | 'C' | 'R' | 'M' | 'G' | 'U' | 'D') => {
            chars.as_str().trim_start().starts_with('(')
        }
        _ => false,
    }
}

/// Split a DSL program into one string per operation. Grouping is by the
/// physical-line operation-start grammar (`line_starts_operation`) rather than
/// a quote/bracket state machine: a weak model that emits an unbalanced quote
/// (e.g. `"fontWeight":"700,"fill"` — `fill` ends up unquoted, an odd number of
/// quotes) used to leak the open-string state across the newline and SWALLOW
/// every following operation into one malformed blob. Anchoring boundaries to
/// the next operation-start line keeps a stray quote contained to its own line
/// (where `parse_json_arg`'s lenient repair can still recover it), and
/// continuation lines of a multi-line JSON body still accumulate correctly.
/// Net bracket delta of a line — `([{` are +1, `)]}` are −1. Strings are NOT
/// tracked on purpose: a weak model's stray quote must not be able to hide a
/// bracket and leak the "open" state across newlines (the bug this guards).
/// A bracket inside a string value is rare and the operation-start guard in
/// `split_operations` recovers it.
fn bracket_delta(line: &str) -> i32 {
    line.chars().fold(0, |d, c| match c {
        '(' | '[' | '{' => d + 1,
        ')' | ']' | '}' => d - 1,
        _ => d,
    })
}

pub(crate) fn split_operations(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0i32;
    let flush = |buf: &mut String, out: &mut Vec<String>| {
        let line = buf.trim();
        if !line.is_empty() && !line.starts_with("//") {
            out.push(line.to_string());
        }
        buf.clear();
    };
    for line in raw.split('\n') {
        // A new operation-start line always begins a fresh operation, even if
        // the previous buffer's bracket count looked unbalanced (a stray quote
        // or a bracket inside a string value can throw the count off).
        if line_starts_operation(line) && !buf.trim().is_empty() {
            flush(&mut buf, &mut out);
            depth = 0;
        }
        if buf.is_empty() {
            let t = line.trim();
            if t.is_empty() || t.starts_with("//") {
                continue;
            }
        }
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(line);
        depth += bracket_delta(line);
        // Brackets balanced → the operation is complete (a multi-line JSON
        // body keeps depth > 0 until its closing `})` line).
        if depth <= 0 {
            flush(&mut buf, &mut out);
            depth = 0;
        }
    }
    flush(&mut buf, &mut out);
    out
}

pub(crate) fn find_top_level_char(s: &str, target: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string: Option<char> = None;
    let mut escape = false;
    for (idx, ch) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if in_string.is_some() && ch == '\\' {
            escape = true;
            continue;
        }
        if let Some(quote) = in_string {
            if ch == quote {
                in_string = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => in_string = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ if ch == target && depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
}

fn parse_parent_ref(raw: &str) -> Result<ParentRef, String> {
    let raw = raw.trim();
    if matches!(raw, "null" | "undefined" | "\"\"" | "''" | "0" | "\"0\"") {
        return Ok(ParentRef::Root);
    }
    if raw.starts_with('"') {
        return serde_json::from_str::<String>(raw)
            .map(ParentRef::Ref)
            .map_err(|e| format!("invalid quoted parent ref: {e}"));
    }
    if raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2 {
        return Ok(ParentRef::Ref(raw[1..raw.len() - 1].to_string()));
    }
    if raw.is_empty() {
        return Ok(ParentRef::Root);
    }
    Ok(ParentRef::Ref(raw.to_string()))
}

fn root_or_node_id(raw: &str) -> NodeId {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "0" {
        NodeId::NONE
    } else {
        NodeId::new(trimmed)
    }
}

fn is_binding(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(crate) fn normalize_node_shape(value: &mut serde_json::Value) {
    let serde_json::Value::Object(obj) = value else {
        return;
    };
    // Map Figma/Pencil auto-layout field names onto our schema FIRST — MiniMax-M3
    // (trained on Pencil's schema) emits `layoutMode`/`itemSpacing`/`strokeWeight`/
    // `primaryAxisAlignItems`/`counterAxisAlignItems`, which serde would silently
    // drop as unknown keys, leaving the frame with no layout → it renders as an
    // unstyled horizontal strip. Rename before anything else reads them.
    normalize_pencil_autolayout_dialect(obj);
    // Flatten a STRUCTURED `layout` object (`{type,gap,padding}` or the
    // externally-tagged `{Horizontal:{…}}`) down to our flat `layout` string +
    // hoisted gap/padding. glm-5.2 in the loop emits this Figma/flex shape; serde
    // rejects it against the string-typed `layout` field, so the WHOLE update
    // fails and the root never gets its layout (measured: glm built n1/n2/n3
    // correctly, then every `U(n1,{layout:{type:horizontal…}})` was rejected and
    // the tree thrashed to empty).
    normalize_layout_object(obj);
    if let Some(fill) = obj.get_mut("fill") {
        normalize_fill(fill);
    }
    // A weak model sometimes emits an empty stroke (`"stroke":[]` or `""`),
    // which deserializes as a 0-length `PenStroke` and fails the WHOLE node
    // (and cascades to every child that targets its binding). Treat an empty
    // stroke as "no stroke" — drop the key so the node still lands.
    if obj.get("stroke").is_some_and(|s| {
        matches!(s, serde_json::Value::Array(a) if a.is_empty())
            || matches!(s, serde_json::Value::String(t) if t.trim().is_empty())
            || s.is_null()
    }) {
        obj.remove("stroke");
    }
    if let Some(stroke) = obj.get_mut("stroke") {
        normalize_stroke(stroke);
    }
    if let Some(padding) = obj.get_mut("padding") {
        normalize_padding(padding);
    }
    super::node_shape_defaults::normalize_text_default_bounds(obj);
    normalize_layout_keyword(obj, "justifyContent");
    normalize_layout_keyword(obj, "alignItems");
    normalize_image_src(obj);
    normalize_text_growth(obj);
    normalize_sizing_keyword(obj, "width");
    normalize_sizing_keyword(obj, "height");
    if let Some(serde_json::Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            normalize_node_shape(child);
        }
    }
}

/// Rename Figma / Pencil auto-layout keys onto OpenPencil's schema so a model
/// trained on the Pencil dialect (MiniMax-M3) keeps its layout. Each rename is
/// applied ONLY when the canonical key is absent, so a node that already uses
/// our names is untouched. Axis-align values are lifted from Figma's
/// `MIN/MAX/CENTER/SPACE_BETWEEN` enum; unknown spellings pass through for the
/// downstream `normalize_layout_keyword` pass to handle.
fn normalize_pencil_autolayout_dialect(obj: &mut serde_json::Map<String, serde_json::Value>) {
    // layoutMode / direction → layout. `direction` is the flex/CSS alias glm-5.2
    // reaches for (measured: `{…,"direction":"horizontal",…}`), `layoutMode` the
    // Figma one M3 uses. First alias present wins.
    if !obj.contains_key("layout") {
        for alias in ["layoutMode", "direction"] {
            let Some(s) = obj
                .remove(alias)
                .and_then(|v| v.as_str().map(str::to_string))
            else {
                continue;
            };
            let mapped = match s.trim().to_ascii_lowercase().as_str() {
                "horizontal" | "row" => Some("horizontal"),
                "vertical" | "column" => Some("vertical"),
                "none" | "" => Some("none"),
                _ => None,
            };
            if let Some(m) = mapped {
                obj.insert("layout".into(), serde_json::Value::String(m.into()));
            }
            break;
        }
    }
    // itemSpacing → gap
    if !obj.contains_key("gap") {
        if let Some(v) = obj
            .remove("itemSpacing")
            .filter(serde_json::Value::is_number)
        {
            obj.insert("gap".into(), v);
        }
    }
    // primaryAxisAlignItems → justifyContent ; counterAxisAlignItems → alignItems
    for (from, to) in [
        ("primaryAxisAlignItems", "justifyContent"),
        ("counterAxisAlignItems", "alignItems"),
    ] {
        if obj.contains_key(to) {
            continue;
        }
        let Some(s) = obj
            .remove(from)
            .and_then(|v| v.as_str().map(str::to_string))
        else {
            continue;
        };
        let mapped = match s.trim().to_ascii_uppercase().as_str() {
            "MIN" => "start",
            "MAX" => "end",
            "CENTER" => "center",
            "SPACE_BETWEEN" => "space_between",
            _ => s.as_str(), // leave for normalize_layout_keyword
        };
        obj.insert(to.into(), serde_json::Value::String(mapped.to_string()));
    }
    // strokeWeight → stroke thickness (Figma names the width apart from the color)
    if let Some(weight) = obj
        .remove("strokeWeight")
        .filter(serde_json::Value::is_number)
    {
        match obj.get_mut("stroke") {
            Some(serde_json::Value::Object(s)) => {
                s.entry("thickness").or_insert(weight);
            }
            Some(serde_json::Value::String(color)) => {
                let color = color.clone();
                obj.insert(
                    "stroke".into(),
                    serde_json::json!({ "thickness": weight, "fill": color }),
                );
            }
            _ => {}
        }
    }
}

/// Flatten a STRUCTURED `layout` object onto our flat schema. glm-5.2 in the
/// agentic loop writes `layout` as a Figma/flex object —
/// `{"type":"horizontal","gap":0,"padding":[…]}` or the externally-tagged
/// `{"Horizontal":{"gap":0,…}}` — but our `layout` field is a plain string
/// (`"horizontal"`), with `gap`/`padding`/`justifyContent`/`alignItems` as
/// sibling keys. serde rejects the object, failing the whole insert/update, so
/// the node's layout never lands (measured: glm built its tree with correct ids,
/// then every `U(n1,{layout:{type:horizontal…}})` was rejected). Lift the
/// direction to the `layout` string and hoist the object's spacing keys to the
/// node (only where the node doesn't already set them). Unknown directions are
/// left untouched rather than guessed.
fn normalize_layout_object(obj: &mut serde_json::Map<String, serde_json::Value>) {
    let layout = match obj.get("layout") {
        Some(serde_json::Value::Object(m)) => m.clone(),
        _ => return,
    };
    // `{type:"horizontal",…}` (type-keyed) OR `{Horizontal:{…}}` (variant-keyed).
    let (raw_dir, inner) = if let Some(t) = layout.get("type").and_then(|v| v.as_str()) {
        (t.to_string(), None)
    } else if let Some((k, v)) = layout.iter().next() {
        (k.clone(), v.as_object().cloned())
    } else {
        return;
    };
    let dir = match raw_dir.trim().to_ascii_lowercase().as_str() {
        "horizontal" | "row" => "horizontal",
        "vertical" | "column" => "vertical",
        "none" => "none",
        _ => return,
    };
    let source = inner.as_ref().unwrap_or(&layout);
    let gap = source.get("gap").cloned();
    let padding = source.get("padding").cloned();
    let justify = source.get("justifyContent").cloned();
    let align = source.get("alignItems").cloned();
    obj.insert("layout".into(), serde_json::Value::String(dir.into()));
    if let Some(g) = gap {
        obj.entry("gap").or_insert(g);
    }
    if let Some(p) = padding {
        obj.entry("padding").or_insert(p);
    }
    if let Some(j) = justify {
        obj.entry("justifyContent").or_insert(j);
    }
    if let Some(a) = align {
        obj.entry("alignItems").or_insert(a);
    }
}

/// `width` / `height` accept `fill_container` / `fit_content` / a number. A weak
/// model sometimes appends a type-hint suffix (`fill_container_str` — a leaked
/// variable name) or uses a CSS-ish spelling (`fill-container` / `hug`), which
/// fails the `Sizing` enum and drops the whole node. Map the known content-hug /
/// fill spellings back to the canonical keyword; DROP the ambiguous `auto`
/// (schema default wins); leave numbers, numeric strings, and already-valid /
/// unrecognised values untouched.
fn normalize_sizing_keyword(obj: &mut serde_json::Map<String, serde_json::Value>, key: &str) {
    let Some(serde_json::Value::String(raw)) = obj.get(key) else {
        return;
    };
    let mut canon = raw.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    for suffix in ["_str", "_string", "_val", "_value"] {
        if let Some(stripped) = canon.strip_suffix(suffix) {
            canon = stripped.to_string();
            break;
        }
    }
    let normalized = match canon.as_str() {
        "fill_container" | "fillcontainer" | "fill" | "container" | "full" | "fill_width"
        | "fill_parent" => Some("fill_container"),
        "fit_content" | "fitcontent" | "fit" | "hug" | "hug_content" | "content" => {
            Some("fit_content")
        }
        // CSS `auto` is AMBIGUOUS: for a block's width it usually means
        // stretch/fill, for height it means hug — forcing either direction
        // inverts the author's intent half the time. Drop the key so the node
        // survives deserialization with the schema default instead.
        "auto" => {
            obj.remove(key);
            return;
        }
        _ => None,
    };
    if let Some(valid) = normalized {
        obj.insert(key.into(), serde_json::Value::String(valid.to_string()));
    }
}

/// A weak model sometimes emits an `image` node with NO `src` (or puts the URL
/// under an alias like `url`/`source`). `ImageNode.src` is REQUIRED, so the whole
/// node fails to deserialize and is dropped — the avatar/logo vanishes AND the
/// column it anchored collapses. Recover the src from a common alias, else inject
/// an empty placeholder (renders as a grey box) so the node — and the layout it
/// holds open — survives. (Measured: glm avatar images → 7× `missing field src`.)
fn normalize_image_src(obj: &mut serde_json::Map<String, serde_json::Value>) {
    if obj.get("type").and_then(serde_json::Value::as_str) != Some("image") {
        return;
    }
    let has_src = obj
        .get("src")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|t| !t.trim().is_empty());
    if has_src {
        return;
    }
    for alias in ["url", "source", "imageUrl", "image_url", "uri", "href"] {
        if let Some(v) = obj.get(alias).cloned() {
            if v.as_str().is_some_and(|t| !t.trim().is_empty()) {
                obj.insert("src".into(), v);
                return;
            }
        }
    }
    obj.insert("src".into(), serde_json::Value::String(String::new()));
}

/// `textGrowth` only accepts `auto` / `fixed-width` / `fixed-width-height`. A
/// weak model borrows a SIZING keyword (`fit_content` / `fill_container`) for it,
/// and the invalid variant drops the whole text node. Map the content-hugging
/// forms to `auto` (same intent); drop anything else so the node still lands with
/// the default growth. (Measured: glm text nodes → 3× `unknown variant fit_content`.)
fn normalize_text_growth(obj: &mut serde_json::Map<String, serde_json::Value>) {
    let Some(raw) = obj.get("textGrowth").and_then(serde_json::Value::as_str) else {
        return;
    };
    let canon = raw.trim().to_ascii_lowercase().replace([' ', '_'], "-");
    let normalized = match canon.as_str() {
        "auto" | "fit-content" | "hug" | "fill-container" | "fill" | "fit" => Some("auto"),
        "fixed-width" => Some("fixed-width"),
        "fixed-width-height" | "fixed-width-and-height" | "fixed" => Some("fixed-width-height"),
        _ => None,
    };
    // Unrecognized spellings: recover the intent from the words before falling
    // back to removal ("fixed_width_and_height", "fixedWidth" and friends carry
    // a clear meaning — silently reverting them to the default undoes a
    // deliberate wrap request).
    let normalized = normalized.or_else(|| {
        if canon.contains("width") && canon.contains("height") {
            Some("fixed-width-height")
        } else if canon.contains("width") {
            Some("fixed-width")
        } else {
            None
        }
    });
    match normalized {
        Some(valid) => {
            obj.insert(
                "textGrowth".into(),
                serde_json::Value::String(valid.to_string()),
            );
        }
        None => {
            obj.remove("textGrowth");
        }
    }
}

fn normalize_layout_keyword(obj: &mut serde_json::Map<String, serde_json::Value>, key: &str) {
    let Some(serde_json::Value::String(value)) = obj.get_mut(key) else {
        return;
    };
    let normalized = match (key, value.as_str()) {
        // CSS flexbox value names. A model fluent in CSS (glm-5.2 etc.) writes
        // `flex-start`/`flex-end` AND — by analogy to our snake_case `space_between`
        // — the underscore form `flex_start`/`flex_end`. The schema only accepts
        // `start`/`end`, so without this the WHOLE node fails to deserialize and is
        // silently dropped (a 5-column table loses every cell whose alignment is a
        // flex_* name — measured: glm dropped the right-aligned amount column + the
        // left-aligned header labels, keeping only the `center` ones).
        ("justifyContent" | "alignItems", "flex-start" | "flex_start" | "flexstart") => "start",
        ("justifyContent" | "alignItems", "flex-end" | "flex_end" | "flexend") => "end",
        ("justifyContent", "space-between") => "space_between",
        ("justifyContent", "space-around") => "space_around",
        ("justifyContent", "space-evenly" | "space_evenly") => "space_between",
        _ => return,
    };
    *value = normalized.to_string();
}

fn normalize_padding(value: &mut serde_json::Value) {
    if let Some(items) = value.as_array() {
        if items.len() == 1 {
            *value = items[0].clone();
        }
    }
}

fn normalize_stroke(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(color) => {
            *value = serde_json::json!({
                "thickness": 1,
                "fill": [{ "type": "solid", "color": color }]
            });
        }
        serde_json::Value::Object(obj) => {
            if let Some(color) = obj.remove("color") {
                obj.entry("fill")
                    .or_insert_with(|| serde_json::json!([{ "type": "solid", "color": color }]));
            }
            if let Some(fill) = obj.get_mut("fill") {
                normalize_fill(fill);
            }
            // `thickness` is REQUIRED by the schema, but models write
            // `{color}` / `{width}` / `{weight}` stroke objects without it —
            // and one rejected node cascades into "parent not found" for its
            // whole subtree (measured: a DeepSeek V4 run dropped 60+ lines
            // and shipped one empty section, 2026-07-12). Alias the common
            // spellings, then default to a hairline.
            if !obj.contains_key("thickness") {
                let aliased = ["width", "weight", "strokeWeight"]
                    .iter()
                    .find_map(|alias| obj.remove(*alias));
                obj.insert(
                    "thickness".into(),
                    aliased.unwrap_or_else(|| serde_json::json!(1)),
                );
            }
        }
        _ => {}
    }
}

pub(crate) fn ensure_node_ids(value: &mut serde_json::Value, next: &mut usize) {
    let serde_json::Value::Object(obj) = value else {
        return;
    };
    if obj.contains_key("type") && !obj.contains_key("id") {
        obj.insert(
            "id".into(),
            serde_json::Value::String(format!("__op_tmp_{next}")),
        );
        *next += 1;
    }
    if let Some(serde_json::Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            ensure_node_ids(child, next);
        }
    }
}

/// `design_skeleton` — phase 1 of TS's layered design workflow.
/// Same wire shape as `batch_design`; the result payload carries
/// `phase=skeleton` so clients can phase their prompting.
pub struct DesignSkeleton;
impl McpTool for DesignSkeleton {
    fn name(&self) -> &str {
        "design_skeleton"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args.contains_key("rootFrame") || args.contains_key("sections") {
            return dispatch_design_skeleton(args);
        }
        dispatch_phase(args, "skeleton")
    }
}
pub fn design_skeleton_snapshot() -> DesignSkeleton {
    DesignSkeleton
}

/// `design_content` — phase 2 of the layered design workflow.
/// Mirrors `batch_design` apply semantics; tagged `phase=content`.
pub struct DesignContent;
impl McpTool for DesignContent {
    fn name(&self) -> &str {
        "design_content"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args.contains_key("children") || args.contains_key("sectionId") {
            return dispatch_design_content(args);
        }
        dispatch_phase(args, "content")
    }
}
pub fn design_content_snapshot() -> DesignContent {
    DesignContent
}

// `design_refine` (the DesignRefine tool + design_refine_snapshot) lives in
// `design_refine_result.rs` so it can build TS's rich `{rootId, totalNodeCount,
// fixes[], layoutSnapshot}` result (it needs a document snapshot + the layout
// helper, which would push this file over the 800-line cap).

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
    let mut fill: Option<Vec<jian_ops_schema::style::PenFill>> = None;
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
            // Generic `fill` passthrough: a full canonical PenFill stack
            // (array of fill objects, or a single fill object) so a batch
            // item can carry gradient / mesh / image fills, not just a
            // solid `fill_hex`. Captured as a balanced raw-JSON slice and
            // deserialized straight into the canonical type.
            "fill" => {
                let raw = capture_raw_json_value(bytes, i)?;
                fill = Some(parse_fill_stack(&raw)?);
            }
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
        fill,
    })
}

/// Deserialize a raw JSON `fill` slice into a canonical fill stack.
/// Accepts either an array of fill objects (`[{...}, ...]`) or a single
/// fill object (`{...}`, wrapped into a 1-entry stack), mirroring the
/// `normalize_fill` shape-tolerance on the JSON nodes path.
fn parse_fill_stack(raw: &str) -> Result<Vec<jian_ops_schema::style::PenFill>, String> {
    use jian_ops_schema::style::PenFill;
    let trimmed = raw.trim_start();
    if trimmed.starts_with('[') {
        serde_json::from_str::<Vec<PenFill>>(raw).map_err(|e| format!("invalid `fill` array: {e}"))
    } else {
        serde_json::from_str::<PenFill>(raw)
            .map(|f| vec![f])
            .map_err(|e| format!("invalid `fill` object: {e}"))
    }
}

/// Scan one balanced JSON value (object / array / string / number /
/// `true` / `false` / `null`) starting at `*i`, advance `*i` past it,
/// and return the raw slice. Respects nesting + string escapes so a
/// `}`/`]` inside a string doesn't prematurely close the value.
fn capture_raw_json_value(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    skip_ws(bytes, i);
    if *i >= bytes.len() {
        return Err("expected a JSON value".into());
    }
    let start = *i;
    match bytes[*i] {
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut in_str = false;
            let mut escaped = false;
            while *i < bytes.len() {
                let c = bytes[*i];
                if in_str {
                    if escaped {
                        escaped = false;
                    } else if c == b'\\' {
                        escaped = true;
                    } else if c == b'"' {
                        in_str = false;
                    }
                } else {
                    match c {
                        b'"' => in_str = true,
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                *i += 1;
                                return slice_utf8(bytes, start, *i);
                            }
                        }
                        _ => {}
                    }
                }
                *i += 1;
            }
            Err("unterminated JSON value".into())
        }
        b'"' => {
            // Reuse the string parser to advance past escapes correctly,
            // then return the original quoted slice.
            let _ = parse_string(bytes, i)?;
            slice_utf8(bytes, start, *i)
        }
        _ => {
            // Number / literal — run to the next delimiter.
            while *i < bytes.len()
                && !matches!(bytes[*i], b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
            {
                *i += 1;
            }
            slice_utf8(bytes, start, *i)
        }
    }
}

fn slice_utf8(bytes: &[u8], start: usize, end: usize) -> Result<String, String> {
    std::str::from_utf8(&bytes[start..end])
        .map(|s| s.to_string())
        .map_err(|_| "invalid UTF-8 in JSON value".to_string())
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
