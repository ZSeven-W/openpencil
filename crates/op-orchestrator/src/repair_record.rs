//! Field-level record of every document edit the deterministic quality
//! passes applied — the itemized half of `crate::repair_summary`'s tally.
//!
//! **Why this exists.** The credential the user sees used to be a bare
//! count ("41 auto-repair(s) applied"). When a pass repaired something the
//! user did NOT want repaired — a pale showcase band restyled, a section's
//! alignment rewritten — there was no way to find out which pass touched
//! which node, so a mis-repair was indistinguishable from a model mistake.
//! One [`RepairRecord`] per accepted edit makes that answerable.
//!
//! **One source of truth.** Records are produced at exactly the same place
//! the counter counts — `repair_summary::RecordingSink::apply` — so counts
//! are `records.len()` grouped by category, never a parallel tally that can
//! drift from the list.
//!
//! **Granularity, honestly stated.** [`describe_command`] reads the target
//! node BEFORE the edit lands, so single-property commands
//! (`SetNodeLayoutProp` / `SetNodeFillHex` / `PatchNodeData` / …) carry a
//! real `before → after`. Whole-subtree swaps (`ReplaceSubtree`, the shape
//! every `apply_root_transform` pass uses) carry a bounded diff of what
//! changed inside the subtree instead — coarser, but still node-and-field
//! level. Anything else degrades to the command's own name rather than
//! inventing a description.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, LayoutPropValue, NodeId, PenNodeExt};
use serde_json::Value;

use crate::repair_summary::CheckCategory;

/// How many changed nodes a subtree-replacement diff names before it
/// summarizes the rest as a count. Keeps one restructure from producing a
/// transcript-length line.
const SUBTREE_DIFF_NODE_LIMIT: usize = 3;
/// How many changed fields are named per node inside that diff.
const SUBTREE_DIFF_FIELD_LIMIT: usize = 3;
/// How an absent value renders on either side of a `before → after`. A word,
/// not a dash: a repair that DROPS a property (`strip_wrapper_double_inset`
/// removes `padding` outright when every side reaches 0) must read as "this
/// property is now unset", which is the disputed fact — a bare dash reads
/// like a placeholder the renderer failed to fill in.
const UNSET: &str = "(unset)";
/// Longest rendered value before it is elided. A fill body or a styled-text
/// run serializes to hundreds of characters; the point is recognizability.
const VALUE_MAX_CHARS: usize = 48;

/// Node fields a subtree diff reports. Deliberately the *visible* ones —
/// the properties a user can see go wrong (surface colour, spacing,
/// alignment, sizing, clipping) — so the record answers "what changed on
/// screen", not "what moved in the JSON".
const DIFFED_FIELDS: [&str; 16] = [
    "fills",
    "fill",
    "backgroundColor",
    "stroke",
    "padding",
    "gap",
    "layoutMode",
    "justifyContent",
    "alignItems",
    "clipContent",
    "width",
    "height",
    "opacity",
    "cornerRadius",
    "fontSize",
    "fontWeight",
];

/// One accepted document edit a quality pass applied.
///
/// `pass` is the pass GROUP the edit belongs to, stamped at the checkpoint
/// that closes the group (see `RepairCounter::checkpoint`). Groups that run
/// a single pass name that pass exactly; multi-pass groups name the family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairRecord {
    /// Pass (or pass group) that applied the edit.
    pub pass: String,
    /// Check family the edit is credited to in the summary.
    pub category: CheckCategory,
    /// Target node id — empty for document-level edits (variables).
    pub node_id: String,
    /// Target node's name at edit time, when it had one.
    pub node_name: Option<String>,
    /// What changed, field level: `gap 24 → 16`.
    pub detail: String,
}

impl RepairRecord {
    /// One-line rendering for the transcript and the log:
    /// `layout · table-repair · Pricing Row [n42] · gap 0 → 16`.
    pub fn line(&self) -> String {
        let mut out = String::new();
        out.push_str(self.category.key());
        out.push_str(" · ");
        out.push_str(&self.pass);
        let who = self.node_label();
        if !who.is_empty() {
            out.push_str(" · ");
            out.push_str(&who);
        }
        out.push_str(" · ");
        out.push_str(&self.detail);
        out
    }

    /// `Name [id]`, `[id]`, or empty — whichever the node actually has.
    fn node_label(&self) -> String {
        let name = self
            .node_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty());
        match (name, self.node_id.as_str()) {
            (Some(name), "") => name.to_string(),
            (Some(name), id) => format!("{name} [{id}]"),
            (None, "") => String::new(),
            (None, id) => format!("[{id}]"),
        }
    }
}

/// A repair captured at apply time, before a checkpoint attributes it to a
/// category and a pass group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingRepair {
    pub(crate) node_id: String,
    pub(crate) node_name: Option<String>,
    pub(crate) detail: String,
}

impl PendingRepair {
    pub(crate) fn attribute(self, category: CheckCategory, pass: &str) -> RepairRecord {
        RepairRecord {
            pass: pass.to_string(),
            category,
            node_id: self.node_id,
            node_name: self.node_name,
            detail: self.detail,
        }
    }
}

/// Describe `cmd` against the document as it stands BEFORE the command is
/// applied, so `before → after` reflects the real prior value.
///
/// Never fails: an unrecognized command still yields a record naming the
/// operation, because a silent gap in the list would break the "count ==
/// records" contract the summary depends on.
pub(crate) fn describe_command(state: &EditorState, cmd: &EditorCommand) -> PendingRepair {
    match cmd {
        EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            ..
        } => {
            let detail = describe_patch(node_value(state, node_id).as_ref(), patch_json);
            pending(state, node_id, detail)
        }
        EditorCommand::SetNodeLayoutProp {
            node_id,
            property,
            value,
        } => {
            let before = node_field(state, node_id, property);
            pending(
                state,
                node_id,
                format!(
                    "{property} {} → {}",
                    render_opt(before.as_ref()),
                    render_layout_value(value)
                ),
            )
        }
        EditorCommand::SetNodeFillHex { node_id, hex } => {
            let before = find(state, node_id).and_then(op_editor_core::first_solid_fill_hex);
            pending(
                state,
                node_id,
                format!("fill {} → {hex}", before.unwrap_or(UNSET)),
            )
        }
        EditorCommand::SetNodeStrokeHex { node_id, hex } => {
            pending(state, node_id, format!("stroke → {hex}"))
        }
        EditorCommand::SetNodeStrokeWidth { node_id, width } => {
            let before = find(state, node_id).and_then(op_editor_core::fills::node_stroke_width);
            pending(
                state,
                node_id,
                format!("strokeWidth {} → {width}", render_num_opt(before)),
            )
        }
        EditorCommand::SetNodeStrokeSideWidth {
            node_id,
            side,
            width,
        } => pending(state, node_id, format!("strokeWidth {side:?} → {width}")),
        EditorCommand::SetNodeFontSize { node_id, font_size } => {
            let before = node_field(state, node_id, "fontSize");
            pending(
                state,
                node_id,
                format!("fontSize {} → {font_size}", render_opt(before.as_ref())),
            )
        }
        EditorCommand::SetNodeFontWeight {
            node_id,
            font_weight,
        } => {
            let before = node_field(state, node_id, "fontWeight");
            pending(
                state,
                node_id,
                format!("fontWeight {} → {font_weight}", render_opt(before.as_ref())),
            )
        }
        EditorCommand::SetNodeCornerRadius { node_id, radius } => {
            let before = node_field(state, node_id, "cornerRadius");
            pending(
                state,
                node_id,
                format!("cornerRadius {} → {radius}", render_opt(before.as_ref())),
            )
        }
        EditorCommand::SetNodeRotation { node_id, degrees } => {
            pending(state, node_id, format!("rotation → {degrees}"))
        }
        EditorCommand::SetNodeText { node_id, text } => {
            pending(state, node_id, format!("text → {}", elide(text)))
        }
        EditorCommand::SetNodeName { node_id, name } => {
            pending(state, node_id, format!("name → {}", elide(name)))
        }
        EditorCommand::SetNodeFlag {
            node_id,
            flag,
            value,
        } => pending(state, node_id, format!("{flag:?} → {value}")),
        EditorCommand::UpdateNode {
            node_id,
            x,
            y,
            width,
            height,
            name,
            fill_hex,
            ..
        } => {
            let before = node_value(state, node_id);
            let mut parts = Vec::new();
            push_num_change(&mut parts, before.as_ref(), "x", x.map(f64::from));
            push_num_change(&mut parts, before.as_ref(), "y", y.map(f64::from));
            push_num_change(&mut parts, before.as_ref(), "width", width.map(f64::from));
            push_num_change(&mut parts, before.as_ref(), "height", height.map(f64::from));
            if let Some(name) = name {
                parts.push(format!("name → {}", elide(name)));
            }
            if let Some(hex) = fill_hex {
                let prior = find(state, node_id).and_then(op_editor_core::first_solid_fill_hex);
                parts.push(format!("fill {} → {hex}", prior.unwrap_or(UNSET)));
            }
            let detail = if parts.is_empty() {
                "updated".to_string()
            } else {
                parts.join(", ")
            };
            pending(state, node_id, detail)
        }
        EditorCommand::DeleteNode { node_id, .. } => {
            let kind = find(state, node_id)
                .map(node_kind_label)
                .unwrap_or_else(|| "node".to_string());
            let descendants = find(state, node_id)
                .map(crate::cleanup::count_descendants)
                .unwrap_or(0);
            pending(
                state,
                node_id,
                format!("removed {kind} (+{descendants} descendant(s))"),
            )
        }
        EditorCommand::MoveNode {
            node_id,
            target_parent,
            index,
            ..
        } => {
            let parent = node_label_for(state, target_parent);
            let slot = index.map(|i| format!(" at index {i}")).unwrap_or_default();
            pending(state, node_id, format!("moved under {parent}{slot}"))
        }
        EditorCommand::ReplaceSubtree { node_id, node, .. } => {
            let detail = match find(state, node_id) {
                Some(before) => describe_subtree_replacement(before, node),
                None => "restructured".to_string(),
            };
            pending(state, node_id, detail)
        }
        EditorCommand::InsertSubtree {
            nodes, parent_id, ..
        }
        | EditorCommand::InsertAuthoredSubtree {
            nodes, parent_id, ..
        }
        | EditorCommand::InsertAuthoredSubtreePreservingRoots {
            nodes, parent_id, ..
        } => pending(
            state,
            parent_id,
            format!("inserted {} subtree(s)", nodes.len()),
        ),
        EditorCommand::SetVariableColor { name, hex } => {
            doc_level(format!("variable {name} → {hex}"))
        }
        EditorCommand::Batch { commands } => {
            doc_level(format!("batch of {} edit(s)", commands.len()))
        }
        other => doc_level(command_label(other).to_string()),
    }
}

/// Fallback label for a command this module does not describe field-by-field.
/// A short static name, never a `Debug` dump — `ReplaceSubtree`'s payload
/// alone would be a whole document.
fn command_label(cmd: &EditorCommand) -> &'static str {
    match cmd {
        EditorCommand::SetVariableScalar { .. } => "variable value set",
        EditorCommand::SetVariables { .. } | EditorCommand::UpsertVariables { .. } => {
            "variables merged"
        }
        EditorCommand::CreateVariable { .. } => "variable created",
        EditorCommand::DeleteVariable { .. } => "variable deleted",
        EditorCommand::RenameVariable { .. } => "variable renamed",
        EditorCommand::SetThemes { .. } | EditorCommand::MergeThemePreset { .. } => "theme merged",
        EditorCommand::MergeAppState { .. } => "app state merged",
        EditorCommand::ReplaceFontFamily { .. } => "font family replaced",
        EditorCommand::ReplaceAllMatchingProperties { .. } => "matching properties replaced",
        EditorCommand::SetEllipseArc { .. } => "arc geometry set",
        EditorCommand::AddNodeEffect { .. } => "effect added",
        EditorCommand::RemoveNodeEffect { .. } => "effect removed",
        EditorCommand::SetEffectParam { .. } | EditorCommand::SetEffectColor { .. } => {
            "effect updated"
        }
        _ => "edit applied",
    }
}

/// Compare a `PatchNodeData` payload against the node's current fields.
fn describe_patch(before: Option<&Value>, patch_json: &str) -> String {
    let Ok(patch) = serde_json::from_str::<Value>(patch_json) else {
        return format!("patched {}", elide(patch_json));
    };
    let Some(fields) = patch.as_object() else {
        return format!("patched {}", elide(patch_json));
    };
    if let Some(detail) = describe_category_grid_reflow(before, fields) {
        return detail;
    }
    let mut parts = Vec::new();
    for (key, after) in fields {
        let prior = before.and_then(|value| value.get(key));
        if prior == Some(after) {
            continue;
        }
        parts.push(format!(
            "{key} {} → {}",
            render_opt(prior),
            render_value(after)
        ));
    }
    if parts.is_empty() {
        return "patched (no field change)".to_string();
    }
    parts.join(", ")
}

fn describe_category_grid_reflow(
    before: Option<&Value>,
    fields: &serde_json::Map<String, Value>,
) -> Option<String> {
    let before = before?;
    let next_children = fields.get("children")?.as_array()?;
    let before_counts = horizontal_row_counts(before)?;
    let after = before.as_object()?.clone();
    let mut after = Value::Object(after);
    after["children"] = Value::Array(next_children.clone());
    let after_counts = horizontal_row_counts(&after)?;
    let total = before_counts.iter().sum::<usize>();
    if before_counts.len() < 2
        || total < 6
        || before_counts.iter().any(|count| *count > 4)
        || after_counts != category_grid_target_row_lengths(total)
        || !reflow_rows_have_canonical_layout(&after)
    {
        return None;
    }

    let name = before
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .or_else(|| before.get("id").and_then(Value::as_str))?;
    let mut detail = format!(
        "category-grid-reflow · {name} · {} → {}",
        format_row_counts(&before_counts, "×"),
        format_row_counts(&after_counts, "+")
    );
    if numeric_tile_width_count(before) > 0 {
        detail.push_str("; numeric tile width(s) dropped → fill_container");
    }
    Some(detail)
}

fn horizontal_row_counts(value: &Value) -> Option<Vec<usize>> {
    if value.get("type").and_then(Value::as_str) != Some("frame")
        || value.get("layout").and_then(Value::as_str) != Some("vertical")
    {
        return None;
    }
    let rows = value.get("children").and_then(Value::as_array)?;
    if !(2..=3).contains(&rows.len()) {
        return None;
    }
    rows.iter()
        .map(|row| {
            (row.get("type").and_then(Value::as_str) == Some("frame")
                && row.get("layout").and_then(Value::as_str) == Some("horizontal"))
            .then(|| row.get("children").and_then(Value::as_array).map(Vec::len))
            .flatten()
        })
        .collect()
}

fn reflow_rows_have_canonical_layout(value: &Value) -> bool {
    value
        .get("children")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter().all(|row| {
                row.get("gap").and_then(Value::as_f64) == Some(12.0)
                    && row.get("justifyContent").and_then(Value::as_str) == Some("start")
                    && row.get("width").and_then(Value::as_str) == Some("fill_container")
                    && row.get("height").and_then(Value::as_str) == Some("fit_content")
                    && row
                        .get("children")
                        .and_then(Value::as_array)
                        .is_some_and(|tiles| {
                            tiles.iter().all(|tile| {
                                tile.get("width").and_then(Value::as_str) == Some("fill_container")
                            })
                        })
            })
        })
}

fn numeric_tile_width_count(value: &Value) -> usize {
    value
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|row| {
            row.get("children")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter(|tile| tile.get("width").and_then(Value::as_f64).is_some())
        .count()
}

fn category_grid_target_row_lengths(tile_count: usize) -> Vec<usize> {
    if matches!(tile_count % 5, 1 | 2) {
        let row_count = tile_count.div_ceil(5);
        let base = tile_count / row_count;
        let extra = tile_count % row_count;
        return (0..row_count)
            .map(|index| base + usize::from(index < extra))
            .collect();
    }

    let mut lengths = Vec::new();
    let mut remaining = tile_count;
    while remaining > 0 {
        let length = remaining.min(5);
        lengths.push(length);
        remaining -= length;
    }
    lengths
}

fn format_row_counts(counts: &[usize], separator: &str) -> String {
    if separator == "×" && counts.len() > 1 && counts.windows(2).all(|pair| pair[0] == pair[1]) {
        return format!("{}×{}", counts[0], counts.len());
    }
    counts
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(separator)
}

/// Bounded node-and-field diff of a whole-subtree swap.
///
/// Matching is by node id: `ReplaceSubtree` rebuilds the root but keeps the
/// ids of the nodes it carries over, so ids present on both sides are the
/// nodes a pass EDITED (as opposed to added or dropped) — exactly the set a
/// user asking "what did the polish stage do to my section" wants named.
fn describe_subtree_replacement(before: &PenNode, after: &PenNode) -> String {
    let old_nodes = flatten(before);
    let new_nodes = flatten(after);
    let mut changed: Vec<String> = Vec::new();
    let mut changed_total = 0usize;
    for (id, new_value) in &new_nodes {
        let Some(old_value) = old_nodes.get(id) else {
            continue;
        };
        let fields = changed_fields(old_value, new_value);
        if fields.is_empty() {
            continue;
        }
        changed_total += 1;
        if changed.len() < SUBTREE_DIFF_NODE_LIMIT {
            let label = new_value
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(|name| format!("{name} [{id}]"))
                .unwrap_or_else(|| format!("[{id}]"));
            changed.push(format!("{label}: {}", fields.join(", ")));
        }
    }
    let added = new_nodes
        .keys()
        .filter(|id| !old_nodes.contains_key(*id))
        .count();
    let removed = old_nodes
        .keys()
        .filter(|id| !new_nodes.contains_key(*id))
        .count();

    let mut out = String::from("restructured");
    if !changed.is_empty() {
        out.push_str(&format!(" — {}", changed.join("; ")));
        if changed_total > changed.len() {
            out.push_str(&format!(
                " (+{} more node(s))",
                changed_total - changed.len()
            ));
        }
    } else if changed_total > 0 {
        out.push_str(&format!(" — {changed_total} node(s) changed"));
    }
    if added > 0 || removed > 0 {
        out.push_str(&format!(" [+{added}/-{removed} node(s)]"));
    }
    out
}

/// Changed [`DIFFED_FIELDS`] between two serialized nodes, capped.
fn changed_fields(before: &Value, after: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in DIFFED_FIELDS {
        let old = before.get(key);
        let new = after.get(key);
        if old == new {
            continue;
        }
        out.push(format!("{key} {} → {}", render_opt(old), render_opt(new)));
        if out.len() == SUBTREE_DIFF_FIELD_LIMIT {
            break;
        }
    }
    out
}

/// Serialize a subtree into `id -> node object` with `children` stripped, so
/// a field comparison sees only the node's own properties.
fn flatten(node: &PenNode) -> std::collections::BTreeMap<String, Value> {
    let mut out = std::collections::BTreeMap::new();
    if let Ok(value) = serde_json::to_value(node) {
        flatten_value(&value, &mut out);
    }
    out
}

fn flatten_value(value: &Value, out: &mut std::collections::BTreeMap<String, Value>) {
    let Some(object) = value.as_object() else {
        return;
    };
    if let Some(children) = object.get("children").and_then(Value::as_array) {
        for child in children {
            flatten_value(child, out);
        }
    }
    let Some(id) = object.get("id").and_then(Value::as_str) else {
        return;
    };
    let mut shallow = object.clone();
    shallow.remove("children");
    out.insert(id.to_string(), Value::Object(shallow));
}

fn pending(state: &EditorState, node_id: &NodeId, detail: String) -> PendingRepair {
    PendingRepair {
        node_id: node_id.as_str().to_string(),
        node_name: find(state, node_id).and_then(|node| node.base().name.clone()),
        detail,
    }
}

fn doc_level(detail: String) -> PendingRepair {
    PendingRepair {
        node_id: String::new(),
        node_name: None,
        detail,
    }
}

fn find<'a>(state: &'a EditorState, node_id: &NodeId) -> Option<&'a PenNode> {
    op_editor_core::walkers::find_node(state.active_children(), node_id)
}

fn node_value(state: &EditorState, node_id: &NodeId) -> Option<Value> {
    find(state, node_id).and_then(|node| serde_json::to_value(node).ok())
}

fn node_field(state: &EditorState, node_id: &NodeId, key: &str) -> Option<Value> {
    node_value(state, node_id).and_then(|value| value.get(key).cloned())
}

fn node_kind_label(node: &PenNode) -> String {
    serde_json::to_value(node)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .or_else(|| value.get("kind"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "node".to_string())
}

fn node_label_for(state: &EditorState, node_id: &NodeId) -> String {
    if node_id.as_str().is_empty() {
        return "page root".to_string();
    }
    match find(state, node_id).and_then(|node| node.base().name.clone()) {
        Some(name) if !name.trim().is_empty() => format!("{name} [{}]", node_id.as_str()),
        _ => format!("[{}]", node_id.as_str()),
    }
}

fn push_num_change(parts: &mut Vec<String>, before: Option<&Value>, key: &str, after: Option<f64>) {
    let Some(after) = after else {
        return;
    };
    let prior = before.and_then(|value| value.get(key));
    parts.push(format!("{key} {} → {}", render_opt(prior), trim_num(after)));
}

fn render_layout_value(value: &LayoutPropValue) -> String {
    match value {
        LayoutPropValue::Number(n) => trim_num(*n),
        LayoutPropValue::Keyword(k) => k.clone(),
        LayoutPropValue::Bool(b) => b.to_string(),
        LayoutPropValue::NumberArray(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|n| trim_num(*n))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_opt(value: Option<&Value>) -> String {
    value.map(render_value).unwrap_or_else(|| UNSET.to_string())
}

fn render_num_opt(value: Option<f64>) -> String {
    value.map(trim_num).unwrap_or_else(|| UNSET.to_string())
}

/// Compact, human-first rendering: strings unquoted, numbers without a
/// trailing `.0`, containers elided once they stop being recognizable.
fn render_value(value: &Value) -> String {
    match value {
        Value::Null => UNSET.to_string(),
        Value::String(s) => elide(s),
        Value::Number(n) => n.as_f64().map(trim_num).unwrap_or_else(|| n.to_string()),
        Value::Bool(b) => b.to_string(),
        // Padding / radii arrive as number arrays; rendering them through
        // `to_string` would print `[32.0,32.0,32.0,32.0]` for what the user
        // authored as `[32,32,32,32]`.
        Value::Array(items) => elide(&format!(
            "[{}]",
            items.iter().map(render_value).collect::<Vec<_>>().join(",")
        )),
        other => elide(&other.to_string()),
    }
}

fn trim_num(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value:.2}")
    }
}

fn elide(text: &str) -> String {
    let cleaned = text.replace('\n', " ");
    if cleaned.chars().count() <= VALUE_MAX_CHARS {
        return cleaned;
    }
    let head: String = cleaned.chars().take(VALUE_MAX_CHARS).collect();
    format!("{head}…")
}

#[cfg(test)]
#[path = "repair_record_tests.rs"]
mod tests;
