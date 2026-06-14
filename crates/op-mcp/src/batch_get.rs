//! TS-compatible `batch_get` document read/search tool.

use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::node::PenNode;
use jian_ops_schema::variable::VariableScalar;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::walkers::find_node;
use op_editor_core::{EditorState, NodeId};
use regex::{Regex, RegexBuilder};
use serde_json::{json, Value};

use super::read_nodes::{node_snapshot_value, page_nodes_snapshots, PageNodes};
use super::{McpTool, ToolErrorCode, ToolOutcome};

type ToolCallError = (ToolErrorCode, String);

pub struct BatchGet {
    pages: Vec<PageNodes>,
    active_page_id: String,
    /// Snapshot of every variable resolved to its concrete scalar under the
    /// active theme (name → value), for `resolveRefs=true`. Mirrors the
    /// inputs TS `resolveNodeForCanvas` resolves against.
    resolved_vars: BTreeMap<String, VariableScalar>,
}

struct SearchPattern {
    node_type: Option<String>,
    name: Option<Regex>,
    reusable: Option<bool>,
}

impl McpTool for BatchGet {
    fn name(&self) -> &str {
        "batch_get"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let resolve_refs = matches!(
            arg_alias(args, &["resolveRefs", "resolve_refs"]),
            Some("true")
        );
        let page = match self.page_nodes(args) {
            Ok(page) => page,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let read_depth = match parse_i32_arg(args, &["readDepth", "read_depth", "depth"], 1) {
            Ok(depth) => depth,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let search_depth = match parse_search_depth(args) {
            Ok(depth) => depth,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let patterns = match parse_patterns(args) {
            Ok(patterns) => patterns,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let node_ids = match parse_node_ids(args) {
            Ok(ids) => ids,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let parent_id = arg_alias(args, &["parentId", "parent_id", "parent"]);
        let search_roots = search_roots(&page.roots, parent_id);
        let mut nodes: Vec<&PenNode> = Vec::new();
        let mut seen = BTreeSet::new();

        if patterns.is_empty() && node_ids.is_empty() {
            nodes.extend(search_roots.iter());
        } else {
            for pattern in &patterns {
                match collect_matches(
                    search_roots,
                    pattern,
                    search_depth,
                    0,
                    &mut seen,
                    &mut nodes,
                ) {
                    Ok(()) => {}
                    Err((code, msg)) => return ToolOutcome::Err(code, msg),
                }
            }
            for id in &node_ids {
                if seen.contains(id) {
                    continue;
                }
                if let Some(nid) = NodeId::new_opt(id) {
                    if let Some(node) = find_node(&page.roots, &nid) {
                        seen.insert(id.clone());
                        nodes.push(node);
                    }
                }
            }
        }

        let mut values: Vec<Value> = nodes
            .iter()
            .map(|node| node_snapshot_value(node, read_depth))
            .collect();
        // resolveRefs=true: expand `$variable` refs to their concrete value
        // under the active theme (mirrors TS resolveNodeForCanvas — fill /
        // stroke colors + opacity / gap / padding numeric refs).
        if resolve_refs {
            for value in &mut values {
                resolve_refs_in_node(value, &self.resolved_vars);
            }
        }
        // TS `batch_get` returns `{ nodes: [...] }` (nodes is a JSON ARRAY,
        // no `count`). Emit the nested JSON verbatim via OkJson so the wire
        // result matches byte-for-byte instead of a flat `{count, nodes:"<str>"}`.
        match serde_json::to_string(&json!({ "nodes": values })) {
            Ok(json) => ToolOutcome::OkJson(json),
            Err(e) => ToolOutcome::Err(
                ToolErrorCode::Internal,
                format!("serialize nodes failed: {e}"),
            ),
        }
    }
}

/// Resolve a `$name` ref string against the snapshot map. `None` for a
/// non-ref (no `$`) or unknown name — callers leave such values unchanged
/// (matching TS, which only rewrites genuine refs it can resolve).
fn resolve_ref_value(s: &str, vars: &BTreeMap<String, VariableScalar>) -> Option<Value> {
    let name = s.strip_prefix('$')?;
    Some(match vars.get(name)? {
        VariableScalar::Str(v) => Value::String(v.clone()),
        VariableScalar::Num(n) => json!(n),
        VariableScalar::Bool(b) => json!(b),
    })
}

/// Resolve a `$ref` `color` string field of an object in place (used for
/// solid fills, gradient stops, and shadow effects).
fn resolve_color_in_obj(
    obj: &mut serde_json::Map<String, Value>,
    vars: &BTreeMap<String, VariableScalar>,
) {
    if let Some(Value::String(raw)) = obj.get("color") {
        if let Some(Value::String(resolved)) = resolve_ref_value(raw, vars) {
            obj.insert("color".into(), Value::String(resolved));
        }
    }
}

/// Resolve color refs in one fill: its own `color` (solid) AND any gradient
/// `stops[].color` (linear/radial gradient bodies).
fn resolve_color_field(fill: &mut Value, vars: &BTreeMap<String, VariableScalar>) {
    if let Value::Object(obj) = fill {
        resolve_color_in_obj(obj, vars);
        if let Some(Value::Array(stops)) = obj.get_mut("stops") {
            for stop in stops {
                if let Value::Object(stop_obj) = stop {
                    resolve_color_in_obj(stop_obj, vars);
                }
            }
        }
    }
}

/// Recursively resolve `$variable` refs in a serialized node, targeting the
/// ref-bearing fields TS `resolveNodeForCanvas` covers: fill colors (solid +
/// gradient stops), stroke-fill colors, shadow/effect colors, and the
/// `opacity` / `gap` / `padding` / `cornerRadius` numeric slots. Unresolved
/// refs are left verbatim (visible `$name`), never replaced with a wrong
/// value. Known subset vs TS (refs left unresolved, not mis-resolved): text
/// `content` / typography font refs; themed vars with no `theme:none` default
/// when the active theme is empty (TS fills it via `getDefaultTheme`); the
/// default-palette fallback for un-seeded refs.
pub(crate) fn resolve_refs_in_node(value: &mut Value, vars: &BTreeMap<String, VariableScalar>) {
    let Value::Object(map) = value else {
        return;
    };
    if let Some(Value::Array(fills)) = map.get_mut("fill") {
        for fill in fills {
            resolve_color_field(fill, vars);
        }
    }
    if let Some(Value::Object(stroke)) = map.get_mut("stroke") {
        if let Some(Value::Array(fills)) = stroke.get_mut("fill") {
            for fill in fills {
                resolve_color_field(fill, vars);
            }
        }
    }
    // Shadow/effect colors (`effects[].color`).
    if let Some(Value::Array(effects)) = map.get_mut("effects") {
        for effect in effects {
            if let Value::Object(effect_obj) = effect {
                resolve_color_in_obj(effect_obj, vars);
            }
        }
    }
    // Numeric refs: opacity/gap/padding/cornerRadius resolve only when the
    // slot is a `$ref` string (numeric/array values are left untouched).
    for key in ["opacity", "gap", "padding", "cornerRadius"] {
        if let Some(slot @ Value::String(_)) = map.get_mut(key) {
            if let Value::String(raw) = slot {
                if let Some(resolved) = resolve_ref_value(raw, vars) {
                    *slot = resolved;
                }
            }
        }
    }
    if let Some(Value::Array(children)) = map.get_mut("children") {
        for child in children {
            resolve_refs_in_node(child, vars);
        }
    }
}

pub fn batch_get_snapshot(state: &EditorState) -> BatchGet {
    let (pages, active_page_id) = page_nodes_snapshots(state);
    // Resolve every variable to its concrete scalar under the active theme
    // now, so `resolveRefs=true` reads stay `&self` (no live state needed).
    let resolved_vars = state
        .doc
        .variables
        .as_ref()
        .map(|vars| {
            vars.keys()
                .filter_map(|name| {
                    state
                        .resolve_variable(name)
                        .map(|scalar| (name.clone(), scalar.clone()))
                })
                .collect()
        })
        .unwrap_or_default();
    BatchGet {
        pages,
        active_page_id,
        resolved_vars,
    }
}

impl BatchGet {
    fn page_nodes(&self, args: &BTreeMap<String, String>) -> Result<&PageNodes, ToolCallError> {
        let target =
            arg_alias(args, &["pageId", "page_id", "page"]).unwrap_or(&self.active_page_id);
        self.pages
            .iter()
            .find(|page| page.id == target)
            .ok_or_else(|| {
                (
                    ToolErrorCode::ToolFailed,
                    format!("page not found: {target}"),
                )
            })
    }
}

fn search_roots<'a>(roots: &'a [PenNode], parent_id: Option<&str>) -> &'a [PenNode] {
    let Some(parent_id) = parent_id else {
        return roots;
    };
    NodeId::new_opt(parent_id)
        .and_then(|id| find_node(roots, &id))
        .and_then(PenNodeExt::children)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn collect_matches<'a>(
    roots: &'a [PenNode],
    pattern: &SearchPattern,
    max_depth: Option<usize>,
    current_depth: usize,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<&'a PenNode>,
) -> Result<(), ToolCallError> {
    if max_depth.is_some_and(|max| current_depth > max) {
        return Ok(());
    }
    for node in roots {
        if matches_pattern(node, pattern) {
            let id = node.id_str().to_string();
            if seen.insert(id) {
                out.push(node);
            }
        }
        if let Some(children) = node.children() {
            collect_matches(children, pattern, max_depth, current_depth + 1, seen, out)?;
        }
    }
    Ok(())
}

fn matches_pattern(node: &PenNode, pattern: &SearchPattern) -> bool {
    if let Some(expected) = &pattern.node_type {
        if node_type(node) != normalize_node_type(expected) {
            return false;
        }
    }
    if let Some(name) = &pattern.name {
        if !name.is_match(node.base().name.as_deref().unwrap_or("")) {
            return false;
        }
    }
    if let Some(reusable) = pattern.reusable {
        if node_reusable(node) != reusable {
            return false;
        }
    }
    true
}

fn parse_patterns(args: &BTreeMap<String, String>) -> Result<Vec<SearchPattern>, ToolCallError> {
    let Some(raw) = arg_alias(args, &["patterns"]) else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        (
            ToolErrorCode::InvalidArgument,
            format!("patterns must be a JSON array: {e}"),
        )
    })?;
    let Value::Array(items) = value else {
        return Err((
            ToolErrorCode::InvalidArgument,
            "patterns must be a JSON array".into(),
        ));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Value::Object(map) = item else {
            return Err((
                ToolErrorCode::InvalidArgument,
                "each pattern must be an object".into(),
            ));
        };
        let name = optional_string(&map, "name")?
            .map(|raw| RegexBuilder::new(&raw).case_insensitive(true).build())
            .transpose()
            .map_err(|e| {
                (
                    ToolErrorCode::InvalidArgument,
                    format!("pattern name must be a valid regex: {e}"),
                )
            })?;
        out.push(SearchPattern {
            node_type: optional_string(&map, "type")?,
            name,
            reusable: optional_bool(&map, "reusable")?,
        });
    }
    Ok(out)
}

fn parse_node_ids(args: &BTreeMap<String, String>) -> Result<Vec<String>, ToolCallError> {
    let Some(raw) = arg_alias(args, &["nodeIds", "node_ids", "ids"]) else {
        return Ok(Vec::new());
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<String>>(trimmed).map_err(|e| {
            (
                ToolErrorCode::InvalidArgument,
                format!("nodeIds must be a JSON string array: {e}"),
            )
        });
    }
    Ok(trimmed
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect())
}

fn parse_search_depth(args: &BTreeMap<String, String>) -> Result<Option<usize>, ToolCallError> {
    let Some(raw) = arg_alias(args, &["searchDepth", "search_depth"]) else {
        return Ok(None);
    };
    if raw.eq_ignore_ascii_case("infinity") {
        return Ok(None);
    }
    raw.parse::<usize>().map(Some).map_err(|_| {
        (
            ToolErrorCode::InvalidArgument,
            format!("searchDepth must be a non-negative integer, got {raw:?}"),
        )
    })
}

fn parse_i32_arg(
    args: &BTreeMap<String, String>,
    keys: &[&str],
    default: i32,
) -> Result<i32, ToolCallError> {
    let Some(raw) = arg_alias(args, keys) else {
        return Ok(default);
    };
    raw.parse::<i32>().map_err(|_| {
        (
            ToolErrorCode::InvalidArgument,
            format!("{} must be an i32, got {raw:?}", keys[0]),
        )
    })
}

fn optional_string(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, ToolCallError> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err((
            ToolErrorCode::InvalidArgument,
            format!("pattern {key} must be a string"),
        )),
    }
}

fn optional_bool(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<bool>, ToolCallError> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(v)) => Ok(Some(*v)),
        _ => Err((
            ToolErrorCode::InvalidArgument,
            format!("pattern {key} must be a boolean"),
        )),
    }
}

fn node_type(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::TextArea(_) => "text_area",
        PenNode::Select(_) => "select",
        PenNode::Switch(_) => "switch",
        PenNode::Checkbox(_) => "checkbox",
        PenNode::Slider(_) => "slider",
        PenNode::RadioGroup(_) => "radio_group",
        PenNode::NumberInput(_) => "number_input",
        PenNode::Progress(_) => "progress",
        PenNode::Tabs(_) => "tabs",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
    }
}

fn normalize_node_type(raw: &str) -> &str {
    match raw {
        "rect" => "rectangle",
        "textInput" => "text_input",
        "iconFont" => "icon_font",
        "radioGroup" => "radio_group",
        "numberInput" => "number_input",
        other => other,
    }
}

fn node_reusable(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(frame) if frame.reusable == Some(true))
}

fn arg_alias<'a>(args: &'a BTreeMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| args.get(*key).map(String::as_str))
}
