//! Compact oversized mobile category grids without changing their content.

use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{Map, Value};

const MAX_CONTAINER: f64 = 44.0;
const MAX_ICON: f64 = 24.0;
const MAX_LABEL: f64 = 12.0;
const MAX_TILE_GAP: f64 = 8.0;
const MAX_ROW_GAP: f64 = 12.0;
const MIN_TILES_IN_LONE_ROW: usize = 4;
const MIN_TILES_IN_GRID_ROW: usize = 3;

#[derive(Debug, Default)]
struct RepairPlan {
    patches: Vec<NodePatch>,
}

#[derive(Debug)]
struct NodePatch {
    node_id: String,
    fields: Map<String, Value>,
}

#[derive(Debug, Clone, Copy)]
enum IconLocation {
    Direct(usize),
    InContainer { holder: usize, icon: usize },
}

#[derive(Debug, Clone, Copy)]
struct TileParts {
    label: usize,
    icon: IconLocation,
}

/// Repair the category-grid density contract for one mobile root.
pub(super) fn repair_category_grid_density(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = super::find_root(sink.state(), root_id) else {
        return 0;
    };
    if !root
        .width_px()
        .is_some_and(|width| width.is_finite() && width <= 480.0)
    {
        return 0;
    }
    let Ok(root_value) = serde_json::to_value(root) else {
        return 0;
    };

    let mut plan = RepairPlan::default();
    collect_repairs(&root_value, None, &[], &root_value, root_id, &mut plan);

    plan.patches
        .into_iter()
        .filter(|patch| {
            sink.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(patch.node_id.clone()),
                patch_json: Value::Object(patch.fields.clone()).to_string(),
                page_id: None,
            })
        })
        .count()
        + reflow::repair_category_grid_reflow(sink, root_id)
}

fn collect_repairs(
    node: &Value,
    parent: Option<&Value>,
    ancestors: &[&Value],
    root: &Value,
    root_id: &str,
    plan: &mut RepairPlan,
) {
    let excluded = is_excluded(node, parent, ancestors, root, root_id);

    if !excluded {
        if let Some(rows) = category_grid_rows(node) {
            if rows.len() > 1 {
                let mut fields = Map::new();
                if number_field(node, "gap").is_some_and(|gap| gap > MAX_ROW_GAP) {
                    fields.insert("gap".to_string(), Value::from(MAX_ROW_GAP));
                }
                push_patch(node, fields, plan);

                let grid_gap = number_field(node, "gap")
                    .map(|gap| gap.min(MAX_ROW_GAP))
                    .unwrap_or(0.0);
                let estimated_height = rows.len() as f64 * (MAX_CONTAINER + MAX_TILE_GAP + 16.0)
                    + (rows.len() - 1) as f64 * grid_gap;
                if rows.len() > 2 && estimated_height > 150.0 {
                    tracing::debug!(
                        grid = %node_name(node),
                        rows = rows.len(),
                        estimated_height,
                        "category grid remains over the height budget after density repair"
                    );
                }
            }
        }

        let in_grid = parent.is_some_and(|parent| category_grid_rows(parent).is_some());
        if is_tile_row_with_min(
            node,
            if in_grid {
                MIN_TILES_IN_GRID_ROW
            } else {
                MIN_TILES_IN_LONE_ROW
            },
        ) {
            repair_row(node, plan);
        }
    }

    let mut next_ancestors = ancestors.to_vec();
    next_ancestors.push(node);
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            collect_repairs(child, Some(node), &next_ancestors, root, root_id, plan);
        }
    }
}

fn repair_row(row: &Value, plan: &mut RepairPlan) {
    let Some(children) = row.get("children").and_then(Value::as_array) else {
        return;
    };
    for tile in children {
        repair_tile(tile, plan);
    }

    let mut fields = Map::new();
    if number_field(row, "gap").is_some_and(|gap| gap > MAX_ROW_GAP) {
        fields.insert("gap".to_string(), Value::from(MAX_ROW_GAP));
    }
    push_patch(row, fields, plan);
}

fn repair_tile(tile: &Value, plan: &mut RepairPlan) {
    let Some(parts) = tile_parts(tile) else {
        return;
    };
    let Some(children) = tile.get("children").and_then(Value::as_array) else {
        return;
    };

    let mut tile_fields = Map::new();
    if horizontal_padding(tile).is_some_and(|padding| padding >= 8.0) {
        tile_fields.insert("padding".to_string(), Value::Null);
    }
    if has_value(tile, "stroke") {
        tile_fields.insert("stroke".to_string(), Value::Null);
    }
    if has_card_like_solid_fill(tile) {
        tile_fields.insert("fill".to_string(), Value::Null);
    }
    if number_field(tile, "gap").is_some_and(|gap| gap > MAX_TILE_GAP) {
        tile_fields.insert("gap".to_string(), Value::from(MAX_TILE_GAP));
    }
    push_patch(tile, tile_fields, plan);

    let (container, icon) = match parts.icon {
        IconLocation::Direct(index) => (None, &children[index]),
        IconLocation::InContainer { holder, icon } => {
            let holder_node = &children[holder];
            (Some(holder_node), &holder_node["children"][icon])
        }
    };
    if let Some(holder) = container {
        let mut holder_fields = Map::new();
        if number_field(holder, "width").is_some_and(|width| width > MAX_CONTAINER) {
            holder_fields.insert("width".to_string(), Value::from(MAX_CONTAINER));
        }
        if number_field(holder, "height").is_some_and(|height| height > MAX_CONTAINER) {
            holder_fields.insert("height".to_string(), Value::from(MAX_CONTAINER));
        }
        if has_value(holder, "stroke") && (has_fill(holder) || has_value(holder, "cornerRadius")) {
            holder_fields.insert("stroke".to_string(), Value::Null);
        }
        push_patch(holder, holder_fields, plan);
    }
    let mut icon_fields = Map::new();
    if number_field(icon, "width").is_some_and(|width| width > MAX_ICON) {
        icon_fields.insert("width".to_string(), Value::from(MAX_ICON));
    }
    if number_field(icon, "height").is_some_and(|height| height > MAX_ICON) {
        icon_fields.insert("height".to_string(), Value::from(MAX_ICON));
    }
    push_patch(icon, icon_fields, plan);

    let label = &children[parts.label];
    let mut label_fields = Map::new();
    if number_field(label, "fontSize").is_some_and(|size| size > MAX_LABEL) {
        label_fields.insert("fontSize".to_string(), Value::from(MAX_LABEL));
    }
    push_patch(label, label_fields, plan);
}

pub(super) fn category_grid_rows(node: &Value) -> Option<&[Value]> {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("vertical")
    {
        return None;
    }
    let children = node.get("children").and_then(Value::as_array)?;
    if !(1..=3).contains(&children.len()) {
        return None;
    }
    // A single row must carry four tiles to read as a category strip; a
    // grid of two or three rows already reads as one even at three per row
    // (the literal 九宫格 a model draws for "分类九宫格").
    let min_tiles = if children.len() >= 2 {
        MIN_TILES_IN_GRID_ROW
    } else {
        MIN_TILES_IN_LONE_ROW
    };
    children
        .iter()
        .all(|row| is_tile_row_with_min(row, min_tiles))
        .then_some(children.as_slice())
}

fn is_tile_row_with_min(node: &Value, min_tiles: usize) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return false;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return false;
    };
    children.len() >= min_tiles
        && children.iter().all(|tile| {
            tile_parts(tile).is_some_and(|parts| {
                tile.get("children")
                    .and_then(Value::as_array)
                    .is_some_and(|children| !is_numeric_only_label(&children[parts.label]))
            })
        })
}

fn tile_parts(tile: &Value) -> Option<TileParts> {
    if tile.get("type").and_then(Value::as_str) != Some("frame") {
        return None;
    }
    if let Some(layout) = tile.get("layout").and_then(Value::as_str) {
        if !matches!(layout, "vertical" | "none") {
            return None;
        }
    }
    let children = tile.get("children").and_then(Value::as_array)?;
    if children.len() != 2 {
        return None;
    }
    for label_index in 0..children.len() {
        if !is_label(&children[label_index]) {
            continue;
        }
        let icon_index = 1 - label_index;
        let Some(icon) = icon_location(&children[icon_index]) else {
            continue;
        };
        return Some(TileParts {
            label: label_index,
            icon: match icon {
                IconLocation::Direct(_) => IconLocation::Direct(icon_index),
                IconLocation::InContainer { icon, .. } => IconLocation::InContainer {
                    holder: icon_index,
                    icon,
                },
            },
        });
    }
    None
}

fn icon_location(node: &Value) -> Option<IconLocation> {
    if is_icon(node) {
        return Some(IconLocation::Direct(0));
    }
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return None;
    }
    let children = node.get("children").and_then(Value::as_array)?;
    (children.len() == 1 && is_icon(&children[0]))
        .then_some(IconLocation::InContainer { holder: 0, icon: 0 })
}

fn is_icon(node: &Value) -> bool {
    matches!(
        node.get("type").and_then(Value::as_str),
        Some("icon_font" | "path")
    )
}

fn is_label(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("text") {
        return false;
    }
    let Some(content) = node.get("content").and_then(Value::as_str) else {
        return false;
    };
    let content = content.trim();
    let total = content.chars().count();
    let cjk = content
        .chars()
        .filter(|character| is_cjk(*character))
        .count();
    !content.is_empty() && total <= 8 && cjk <= 4
}

fn is_numeric_only_label(node: &Value) -> bool {
    node.get("content")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|content| !content.is_empty() && content.chars().all(char::is_numeric))
}

fn is_cjk(character: char) -> bool {
    matches!(
        character,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2FA1F}'
    )
}

fn is_excluded(
    node: &Value,
    parent: Option<&Value>,
    ancestors: &[&Value],
    root: &Value,
    root_id: &str,
) -> bool {
    if node.get("id").and_then(Value::as_str) == Some(root_id) {
        return true;
    }
    if super::is_status_bar_from_json(node)
        || ancestors
            .iter()
            .any(|ancestor| super::is_status_bar_from_json(ancestor))
    {
        return true;
    }
    if has_excluded_name(node) || ancestors.iter().any(|ancestor| has_excluded_name(ancestor)) {
        return true;
    }
    if is_bottom_navigation(node)
        || ancestors.iter().any(|ancestor| {
            ancestor.get("id").and_then(Value::as_str) != Some(root_id)
                && is_bottom_navigation(ancestor)
        })
    {
        return true;
    }
    parent.is_some_and(|parent| is_root_trailing_tab_bar(parent, root))
        || is_root_trailing_tab_bar(node, root)
}

fn is_bottom_navigation(node: &Value) -> bool {
    serde_json::from_value::<jian_ops_schema::node::PenNode>(node.clone())
        .ok()
        .is_some_and(|node| {
            crate::cleanup::is_bottom_nav_section(&node)
                || crate::cleanup::is_trailing_bottom_nav_section(&node)
        })
}

fn is_root_trailing_tab_bar(parent: &Value, root: &Value) -> bool {
    let Some(parent_id) = parent.get("id").and_then(Value::as_str) else {
        return false;
    };
    let is_last_root_child = root
        .get("children")
        .and_then(Value::as_array)
        .and_then(|children| children.last())
        .and_then(|child| child.get("id"))
        .and_then(Value::as_str)
        == Some(parent_id);
    is_last_root_child && has_tab_bar_identity(parent)
}

fn has_tab_bar_identity(node: &Value) -> bool {
    ["name", "role"].iter().any(|key| {
        node.get(*key)
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase)
            .is_some_and(|value| {
                value.contains("tab-bar") || value.contains("tab bar") || value == "tabbar"
            })
    })
}

fn has_excluded_name(node: &Value) -> bool {
    node.get("name")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase)
        .is_some_and(|name| {
            ["map", "keypad", "numpad", "calendar", "week"]
                .iter()
                .any(|part| name.contains(part))
                || name.contains("progress")
                || name.contains("seven day")
        })
}

fn repair_node_id(node: &Value) -> Option<String> {
    node.get("id").and_then(Value::as_str).map(str::to_string)
}

fn push_patch(node: &Value, fields: Map<String, Value>, plan: &mut RepairPlan) {
    if fields.is_empty() {
        return;
    }
    let Some(node_id) = repair_node_id(node) else {
        return;
    };
    plan.patches.push(NodePatch { node_id, fields });
}

fn has_value(node: &Value, key: &str) -> bool {
    node.get(key).is_some_and(|value| !value.is_null())
}

fn has_fill(node: &Value) -> bool {
    node.get("fill").is_some_and(|value| match value {
        Value::Null => false,
        Value::Array(values) => !values.is_empty(),
        _ => true,
    })
}

fn has_card_like_solid_fill(node: &Value) -> bool {
    match node.get("fill") {
        Some(Value::Array(values)) => {
            !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.get("type").and_then(Value::as_str) == Some("solid"))
        }
        Some(value) => value.get("type").and_then(Value::as_str) == Some("solid"),
        None => false,
    }
}

fn number_field(node: &Value, key: &str) -> Option<f64> {
    node.get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn horizontal_padding(node: &Value) -> Option<f64> {
    match node.get("padding")? {
        Value::Number(value) => value.as_f64(),
        Value::Array(values) => match values.as_slice() {
            [vertical, horizontal] => horizontal.as_f64().or_else(|| vertical.as_f64()),
            [_, right, _, left] => Some(right.as_f64()?.max(left.as_f64()?)),
            _ => None,
        },
        _ => None,
    }
}

fn node_name(node: &Value) -> String {
    node.get("name")
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            node.get("id")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>")
        })
        .to_string()
}

#[cfg(test)]
#[path = "category_grid_density_tests.rs"]
mod tests;

#[path = "category_grid_reflow.rs"]
mod reflow;
