//! Multi-line mixed `batch_design` DSL program executor.
//!
//! Mirrors TS `packages/pen-mcp/src/tools/batch-design-dsl.ts`
//! (`runBatchDesignDsl` + `executeLine`): arbitrary programs mixing
//! I/U/C/R/G/M/D operations with shared bindings and slash-path
//! expressions, executed line by line with TS's best-effort error
//! semantics — a failing line is collected into `errors[]` (with a
//! 200-char line preview) and execution CONTINUES, exactly like the
//! TS `try/catch` around `executeLine`.
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

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::command_node::remap_subtree_ids_mapping;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use regex::Regex;
use serde_json::{json, Value};

use super::batch_design::{
    ensure_node_ids, find_top_level_char, normalize_node_shape, split_operations,
};
use super::batch_direct_ops::{split_top_level_args, update_command_from_value};
use super::batch_page::optional_page_id;
use super::{EditorCommand, ToolOutcome};

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
    let mut ctx = ProgramCtx {
        sim: snapshot.clone(),
        page_id: page_id.clone(),
        bindings: BTreeMap::new(),
        alias: BTreeMap::new(),
        results: Vec::new(),
        commands: Vec::new(),
        post_process,
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

    let mut errors: Vec<Value> = Vec::new();
    for line in split_operations(operations) {
        if let Err(error) = execute_line(&line, &mut ctx) {
            errors.push(json!({ "line": line_preview(&line), "error": error }));
        }
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

struct ProgramCtx {
    sim: EditorState,
    page_id: Option<String>,
    /// binding name → final (host-assigned) node id.
    bindings: BTreeMap<String, String>,
    /// authored id → final id for remapped inserts, first-wins (TS
    /// `findNodeInTree` finds the first match in tree order).
    alias: BTreeMap<String, String>,
    results: Vec<Value>,
    commands: Vec<EditorCommand>,
    post_process: bool,
}

impl ProgramCtx {
    /// Emit `cmd` AND apply it to the sim. The sim apply is the line's
    /// final validation gate — the host will run the same code.
    fn emit(&mut self, cmd: EditorCommand, failure: &str) -> Result<(), String> {
        if !self.sim.apply(cmd.clone()) {
            return Err(failure.to_string());
        }
        self.commands.push(cmd);
        Ok(())
    }

    fn bind(&mut self, binding: &str, node_id: &str) {
        self.bindings
            .insert(binding.to_string(), node_id.to_string());
        self.results
            .push(json!({ "binding": binding, "nodeId": node_id }));
    }

    /// Assign fresh sim-allocator ids to `nodes` (in place); returns
    /// the (authored → final) map. Authored ids are recorded into the
    /// alias table so slash paths keep resolving TS-style.
    fn remap(&mut self, nodes: &mut [PenNode]) -> Result<Vec<(String, String)>, String> {
        let mut seed = self
            .sim
            .next_node_id_seed()
            .ok_or("node id space exhausted")?;
        let mut taken = self.sim.collect_node_ids();
        let map = remap_subtree_ids_mapping(nodes, &mut seed, &mut taken)
            .ok_or("node id space exhausted")?;
        for (old, new) in &map {
            if !old.starts_with("__op_tmp_") {
                self.alias.entry(old.clone()).or_insert_with(|| new.clone());
            }
        }
        Ok(map)
    }
}

/// TS `executeLine` — one DSL operation.
fn execute_line(line: &str, ctx: &mut ProgramCtx) -> Result<(), String> {
    // TS line grammar (dotAll `s` flag — pretty-printed JSON bodies
    // carry literal newlines inside the arg list):
    //   binding=OP(args)  for I/C/R/M/G
    //   OP(args)          for I/C/R/G (auto-binding) and U/D/M (call)
    let assign = regex(r"(?s)^(\w+)\s*=\s*([ICRMG])\((.+)\)$");
    let bindless = regex(r"(?s)^([ICRG])\((.+)\)$");
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
        let binding = format!("_auto_{}_{op}", ctx.results.len());
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
    Err(format!("Cannot parse operation: {line}"))
}

fn execute_assign(op: &str, binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<(), String> {
    match op {
        "I" => execute_insert(binding, args, ctx),
        "C" => execute_copy(binding, args, ctx),
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
fn execute_insert(binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<(), String> {
    let comma = find_top_level_char(args, ',').ok_or("Insert requires parent and node data")?;
    let parent_raw = args[..comma].trim();
    let parent = if parent_raw == "null" {
        None
    } else {
        Some(resolve_ref(parent_raw, &ctx.bindings))
    };
    let mut node = parse_node_json(&args[comma + 1..], ctx.post_process)?;

    // TS auto-replace: a root-level frame insert replaces the first
    // EMPTY root frame (inheriting its x/y) instead of siblinging it.
    let mut pre_commands: Vec<(EditorCommand, &str)> = Vec::new();
    if parent.is_none() && matches!(node, PenNode::Frame(_)) {
        if let Some(empty) = first_empty_frame(ctx.sim.active_children()) {
            let (id, x, y) = (empty.id_str().to_string(), empty.base().x, empty.base().y);
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
        .ok_or("Insert produced no node")?;
    for (cmd, failure) in pre_commands {
        ctx.emit(cmd, failure)?;
    }
    ctx.emit(
        EditorCommand::InsertAuthoredSubtree {
            nodes,
            parent_id: parent_node_id(parent.as_deref()),
            page_id: ctx.page_id.clone(),
        },
        &format!(
            "Insert parent not found or not a container: {}",
            parent.as_deref().unwrap_or("null")
        ),
    )?;
    ctx.bind(binding, &root_id);
    Ok(())
}

/// `binding=C(sourceId, parent[, overrides])` — clone with fresh ids.
fn execute_copy(binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<(), String> {
    let first = find_top_level_char(args, ',').ok_or("Copy requires sourceId, parent, and data")?;
    let source_raw = args[..first].trim();
    let rest = args[first + 1..].trim();
    let (parent_raw, data_str) = match find_top_level_char(rest, ',') {
        None => (rest, "{}"),
        Some(second) => (rest[..second].trim(), rest[second + 1..].trim()),
    };
    let source_id = lookup_id(&resolve_ref(source_raw, &ctx.bindings), &ctx.alias);
    let Some(source) =
        op_editor_core::walkers::find_node(ctx.sim.active_children(), &NodeId::new(&source_id))
    else {
        return Err(format!("Copy source not found: {source_id}"));
    };
    let mut cloned_value =
        serde_json::to_value(source).map_err(|e| format!("Copy source unserializable: {e}"))?;
    // TS `Object.assign(cloned, data)` minus `id` (never overridden)
    // and `descendants` (a TS no-op — see module docs).
    let overrides = parse_json_arg(data_str)?;
    let Some(overrides) = overrides.as_object() else {
        return Err("C() overrides JSON must be an object".into());
    };
    if let Some(obj) = cloned_value.as_object_mut() {
        for (key, value) in overrides {
            if key == "id" || key == "descendants" {
                continue;
            }
            obj.insert(key.clone(), value.clone());
        }
    }
    normalize_node_shape(&mut cloned_value);
    let mut node: PenNode = serde_json::from_value(cloned_value)
        .map_err(|e| format!("C() overrides produce an invalid node: {e}"))?;
    if ctx.post_process {
        let _ = op_editor_core::command_refine::refine_subtree(&mut node);
    }

    let mut nodes = vec![node];
    // Cloning mints fresh ids for the whole subtree
    // (`cloneNodeWithNewIds` parity); the clone's old ids are the LIVE
    // source ids, so they must NOT enter the alias table — `remap`'s
    // first-wins entry never fires because the source ids already
    // resolve directly in the sim tree.
    let map = ctx.remap(&mut nodes)?;
    let clone_id = map
        .first()
        .map(|(_, new)| new.clone())
        .ok_or("Copy produced no node")?;
    let parent = if parent_raw == "null" {
        None
    } else {
        Some(resolve_ref(parent_raw, &ctx.bindings))
    };
    ctx.emit(
        EditorCommand::InsertAuthoredSubtree {
            nodes,
            parent_id: parent_node_id(parent.as_deref()),
            page_id: ctx.page_id.clone(),
        },
        &format!(
            "Copy parent not found or not a container: {}",
            parent.as_deref().unwrap_or("null")
        ),
    )?;
    ctx.bind(binding, &clone_id);
    Ok(())
}

/// `binding=R(path, data)` — replace the node at `path` with a fresh
/// node built from `data` (same slot, fresh id, children dropped).
fn execute_replace(binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<(), String> {
    let comma = find_top_level_char(args, ',').ok_or("Replace requires path and node data")?;
    let path_raw = args[..comma].trim();
    let path = resolve_path_expr(path_raw, &ctx.bindings);
    let Some(old) = find_node_by_path(ctx.sim.active_children(), &path, &ctx.alias) else {
        return Err(format!("Replace target not found: {path}"));
    };
    let old_id = old.id_str().to_string();
    let node = parse_node_json(&args[comma + 1..], ctx.post_process)?;

    // Predict the ids `cmd_replace_subtree` will assign: it remaps off
    // the live seed/taken BEFORE removing the old subtree — identical
    // inputs to this sim-side dry run, so the mapping is identical.
    let mut probe = vec![node.clone()];
    let map = ctx.remap(&mut probe)?;
    let new_id = map
        .first()
        .map(|(_, new)| new.clone())
        .ok_or("Replace produced no node")?;
    ctx.emit(
        EditorCommand::ReplaceSubtree {
            node_id: NodeId::new(&old_id),
            node: Box::new(node),
            drop_children: true,
            page_id: ctx.page_id.clone(),
        },
        &format!("Replace failed for: {path}"),
    )?;
    ctx.bind(binding, &new_id);
    Ok(())
}

/// `binding=G(parent, mode, prompt)` — emit an image node. TS requires
/// every argument quoted; `mode` must be `search` or `generate`. No
/// fetcher at this layer — `src` stays empty (browser-caller parity);
/// the host's own image pipeline enriches later.
fn execute_image(binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<(), String> {
    let g = regex(r#"^"([^"]+)"\s*,\s*"(search|generate)"\s*,\s*"([^"]+)"$"#);
    let Some(c) = g.captures(args.trim()) else {
        return Err(format!("Invalid G() syntax: {args}"));
    };
    let parent = resolve_ref(c.get(1).map_or("", |m| m.as_str()), &ctx.bindings);
    let prompt = c.get(3).map_or("", |m| m.as_str());
    let name: String = prompt.chars().take(40).collect();
    let node: PenNode = serde_json::from_value(json!({
        "type": "image",
        "id": "__op_tmp_image_1",
        "name": name,
        "imagePrompt": prompt,
        "src": "",
        "width": 400,
        "height": 300
    }))
    .map_err(|e| format!("invalid G() image node: {e}"))?;
    let mut nodes = vec![node];
    let map = ctx.remap(&mut nodes)?;
    let image_id = map
        .first()
        .map(|(_, new)| new.clone())
        .ok_or("G() produced no node")?;
    ctx.emit(
        EditorCommand::InsertAuthoredSubtree {
            nodes,
            parent_id: parent_node_id(Some(&parent)),
            page_id: ctx.page_id.clone(),
        },
        &format!("G() parent not found or not a container: {parent}"),
    )?;
    ctx.bind(binding, &image_id);
    Ok(())
}

/// `U(path, data)` — shallow-patch the node at `path`. No result entry
/// (TS call-form ops don't push results).
fn execute_update(args: &str, ctx: &mut ProgramCtx) -> Result<(), String> {
    let comma = find_top_level_char(args, ',').ok_or("Update requires path and update data")?;
    let path = resolve_path_expr(args[..comma].trim(), &ctx.bindings);
    let mut value = parse_json_arg(&args[comma + 1..])?;
    normalize_node_shape(&mut value);
    let Some(target) = find_node_by_path(ctx.sim.active_children(), &path, &ctx.alias) else {
        return Err(format!("Update target not found: {path}"));
    };
    let node_id = NodeId::new(target.id_str());
    let cmd = update_command_from_value(node_id, &value)?;
    let cmd = with_page_id(cmd, ctx.page_id.clone());
    ctx.emit(cmd, &format!("Update failed for: {path}"))
}

/// `D(ref)` — delete. TS `removeNodeFromTree` silently no-ops on an
/// unknown id: no error, no result.
fn execute_delete(args: &str, ctx: &mut ProgramCtx) -> Result<(), String> {
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
fn execute_move(args: &str, ctx: &mut ProgramCtx) -> Result<String, String> {
    let parts = split_top_level_args(args);
    if parts.len() < 2 {
        return Err("Move requires nodeId and parent".into());
    }
    let node_id = lookup_id(&resolve_ref(parts[0].trim(), &ctx.bindings), &ctx.alias);
    if op_editor_core::walkers::find_node(ctx.sim.active_children(), &NodeId::new(&node_id))
        .is_none()
    {
        return Err(format!("Move target not found: {node_id}"));
    }
    let parent_raw = parts[1].trim();
    let parent = if matches!(parent_raw, "null" | "undefined") {
        None
    } else {
        Some(resolve_ref(parent_raw, &ctx.bindings))
    };
    let index = match parts.get(2) {
        None => None,
        Some(raw) => Some(
            strip_outer_quotes(raw.trim())
                .parse::<usize>()
                .map_err(|_| format!("M() index must be a non-negative integer, got {raw:?}"))?,
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

// --- Node JSON --------------------------------------------------------

/// Parse + normalize an I()/R() node body into a `PenNode` with
/// authored ids filled in (the caller remaps them to final ids).
fn parse_node_json(raw: &str, post_process: bool) -> Result<PenNode, String> {
    let mut value = parse_json_arg(raw)?;
    if !value.is_object() {
        return Err("node data must be a JSON object".into());
    }
    normalize_node_shape(&mut value);
    let mut tmp = 1usize;
    ensure_node_ids(&mut value, &mut tmp);
    let mut node: PenNode =
        serde_json::from_value(value).map_err(|e| format!("invalid PenNode payload: {e}"))?;
    if post_process {
        // TS postProcess hooks (emoji strip, unique ids, layout-child
        // position sanitize, screen-bounds clamp) — the deterministic
        // subset shipped in `command_refine.rs`.
        let _ = op_editor_core::command_refine::refine_subtree(&mut node);
    }
    Ok(node)
}

/// TS `parseJsonArg` — strict JSON first, then the lenient agent-typo
/// pipeline: quote unquoted keys, single→double quote delimiters,
/// strip empty-key artifacts and trailing commas.
fn parse_json_arg(raw: &str) -> Result<Value, String> {
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
    normalized = regex(r#",\s*""\s*:\s*[^,}\]]+"#)
        .replace_all(&normalized, "")
        .into_owned();
    normalized = regex(r",(\s*[}\]])")
        .replace_all(&normalized, "$1")
        .into_owned();
    serde_json::from_str(&normalized).map_err(|e| {
        let snippet: String = raw.chars().take(300).collect();
        let ellipsis = if raw.chars().count() > 300 { "..." } else { "" };
        format!("Failed to parse JSON ({e}): {snippet}{ellipsis}")
    })
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

// --- Reference + path resolution --------------------------------------

/// TS `resolveRef` — strip one leading + one trailing double quote,
/// then look the cleaned token up in the binding table.
fn resolve_ref(raw: &str, bindings: &BTreeMap<String, String>) -> String {
    let cleaned = strip_outer_quotes(raw);
    bindings.get(cleaned.as_str()).cloned().unwrap_or(cleaned)
}

fn strip_outer_quotes(raw: &str) -> String {
    let s = raw.strip_prefix('"').unwrap_or(raw);
    let s = s.strip_suffix('"').unwrap_or(s);
    s.to_string()
}

/// TS `resolvePathExpr` — `binding+"/child"` concatenation, else a
/// plain (possibly quoted) ref.
fn resolve_path_expr(raw: &str, bindings: &BTreeMap<String, String>) -> String {
    if raw.contains('+') {
        return raw
            .split('+')
            .map(|part| {
                let t = part.trim();
                if t.starts_with('"') || t.starts_with('\'') {
                    // TS `slice(1, -1)` — drop the delimiters.
                    let mut cs = t.chars();
                    cs.next();
                    cs.next_back();
                    cs.as_str().to_string()
                } else {
                    bindings.get(t).cloned().unwrap_or_else(|| t.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join("");
    }
    resolve_ref(raw, bindings)
}

/// One path segment → live sim id, translating authored ids through
/// the alias table when the segment doesn't resolve directly.
fn lookup_id(id: &str, alias: &BTreeMap<String, String>) -> String {
    alias.get(id).cloned().unwrap_or_else(|| id.to_string())
}

/// TS `findNodeByPath` — first segment is a deep `findNodeInTree`,
/// every later segment must be a DIRECT child id. Segments written
/// against authored (pre-remap) ids fall back through the alias table.
fn find_node_by_path<'a>(
    children: &'a [PenNode],
    path: &str,
    alias: &BTreeMap<String, String>,
) -> Option<&'a PenNode> {
    let mut parts = path.split('/');
    let first = parts.next()?;
    let first_id = lookup_id(first, alias);
    let mut current = op_editor_core::walkers::find_node(children, &NodeId::new(&first_id))?;
    for part in parts {
        let part_id = lookup_id(part, alias);
        let kids = current.children()?;
        current = kids.iter().find(|c| c.id_str() == part_id)?;
    }
    Some(current)
}

// --- Small helpers ------------------------------------------------------

fn parent_node_id(parent: Option<&str>) -> NodeId {
    match parent {
        None => NodeId::NONE,
        Some(raw) if raw.trim().is_empty() || raw.trim() == "0" => NodeId::NONE,
        Some(raw) => NodeId::new(raw.trim()),
    }
}

/// Stamp the program-level pageId onto a U() command built by the
/// shared direct-op builder (which emits `page_id: None`).
fn with_page_id(cmd: EditorCommand, page_id: Option<String>) -> EditorCommand {
    match cmd {
        EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            ..
        } => EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id,
        },
        EditorCommand::UpdateNode {
            node_id,
            x,
            y,
            width,
            height,
            name,
            fill_hex,
            ..
        } => EditorCommand::UpdateNode {
            node_id,
            x,
            y,
            width,
            height,
            name,
            fill_hex,
            page_id,
        },
        other => other,
    }
}

/// TS `isEmptyFrame` over the page roots — the first root-level frame
/// with no children.
fn first_empty_frame(children: &[PenNode]) -> Option<&PenNode> {
    children.iter().find(|node| {
        matches!(node, PenNode::Frame(_)) && node.children().map(|c| c.is_empty()).unwrap_or(true)
    })
}

/// Mirror of `command_apply::command_page_index`'s explicit-page arm:
/// page id match first, then a legacy numeric index.
fn resolve_page_index(state: &EditorState, raw: &str) -> Option<usize> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => pages
            .iter()
            .position(|page| page.id == raw)
            .or_else(|| raw.parse::<usize>().ok().filter(|idx| *idx < pages.len())),
        _ => raw.parse::<usize>().ok().filter(|idx| *idx == 0),
    }
}

/// TS error-line preview: 200 chars + `...`.
fn line_preview(line: &str) -> String {
    let mut preview: String = line.chars().take(200).collect();
    if line.chars().count() > 200 {
        preview.push_str("...");
    }
    preview
}

/// TS `countNodes`.
fn count_forest(nodes: &[PenNode]) -> usize {
    nodes
        .iter()
        .map(|node| {
            1 + node
                .children()
                .map(|children| count_forest(children))
                .unwrap_or(0)
        })
        .sum()
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static DSL regex must compile")
}
