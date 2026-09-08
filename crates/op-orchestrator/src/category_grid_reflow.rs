//! Reflow sparse mobile category grids into five-column rows.

use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{Map, Value};

const MAX_ROOT_WIDTH: f64 = 480.0;
const ROW_GAP: f64 = 12.0;

/// Reflow eligible category grids after the density patches have landed.
pub(super) fn repair_category_grid_reflow(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = super::super::find_root(sink.state(), root_id) else {
        return 0;
    };
    if !root
        .width_px()
        .is_some_and(|width| width.is_finite() && width <= MAX_ROOT_WIDTH)
    {
        return 0;
    }
    let Ok(root_value) = serde_json::to_value(root) else {
        return 0;
    };

    let mut grid_ids = Vec::new();
    collect_grid_ids(&root_value, None, &[], &root_value, root_id, &mut grid_ids);

    grid_ids
        .into_iter()
        .filter(|grid_id| reflow_grid(sink, grid_id))
        .count()
}

fn collect_grid_ids(
    node: &Value,
    parent: Option<&Value>,
    ancestors: &[&Value],
    root: &Value,
    root_id: &str,
    grid_ids: &mut Vec<String>,
) {
    if !super::is_excluded(node, parent, ancestors, root, root_id)
        && category_grid_is_reflow_candidate(node)
    {
        if let Some(id) = node.get("id").and_then(Value::as_str) {
            grid_ids.push(id.to_string());
        }
    }

    let mut next_ancestors = ancestors.to_vec();
    next_ancestors.push(node);
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            collect_grid_ids(child, Some(node), &next_ancestors, root, root_id, grid_ids);
        }
    }
}

fn category_grid_is_reflow_candidate(node: &Value) -> bool {
    let Some(rows) = super::category_grid_rows(node) else {
        return false;
    };
    if rows.len() < 2
        || !rows.iter().all(row_justify_is_allowed)
        || rows
            .iter()
            .flat_map(|row| {
                row.get("children")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
            })
            .any(|tile| super::has_fill(tile) || contains_image(tile))
    {
        return false;
    }

    let tile_count = rows
        .iter()
        .filter_map(|row| row.get("children").and_then(Value::as_array))
        .map(Vec::len)
        .sum::<usize>();
    if tile_count < 6
        || rows
            .iter()
            .any(|row| row["children"].as_array().unwrap().len() > 4)
    {
        return false;
    }

    // Two complete four-column rows are already a legitimate compact grid.
    !(rows.len() == 2
        && rows.iter().all(|row| {
            row["children"]
                .as_array()
                .is_some_and(|tiles| tiles.len() == 4)
        }))
}

fn row_justify_is_allowed(row: &Value) -> bool {
    match row.get("justifyContent") {
        None | Some(Value::Null) => true,
        Some(Value::String(value)) => {
            matches!(value.as_str(), "start" | "space_between" | "space_around")
        }
        Some(_) => false,
    }
}

fn contains_image(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) == Some("image") {
        return true;
    }
    node.get("children")
        .and_then(Value::as_array)
        .is_some_and(|children| children.iter().any(contains_image))
}

fn reflow_grid(sink: &mut dyn DocSink, grid_id: &str) -> bool {
    let Some(grid) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(grid_id.to_string()),
    ) else {
        return false;
    };
    let Ok(grid_value) = serde_json::to_value(grid) else {
        return false;
    };
    let Some(rows) = super::category_grid_rows(&grid_value) else {
        return false;
    };
    if !category_grid_is_reflow_candidate(&grid_value) {
        return false;
    }

    let tiles = rows
        .iter()
        .filter_map(|row| row.get("children").and_then(Value::as_array))
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let row_lengths = target_row_lengths(tiles.len());
    let mut next_rows = Vec::with_capacity(row_lengths.len());
    let mut tile_offset = 0usize;
    for (row_index, row_length) in row_lengths.into_iter().enumerate() {
        let mut next_row = rows
            .get(row_index)
            .cloned()
            .unwrap_or_else(|| clone_row_with_fresh_id(sink, &rows[0], grid_id, row_index));
        next_row["gap"] = Value::from(ROW_GAP);
        next_row["justifyContent"] = Value::String("start".to_string());
        next_row["width"] = Value::String("fill_container".to_string());
        next_row["height"] = Value::String("fit_content".to_string());

        let row_tiles = tiles[tile_offset..tile_offset + row_length]
            .iter()
            .cloned()
            .map(|mut tile| {
                tile["width"] = Value::String("fill_container".to_string());
                tile
            })
            .collect::<Vec<_>>();
        next_row["children"] = Value::Array(row_tiles);
        next_rows.push(next_row);
        tile_offset += row_length;
    }

    if grid_value.get("children") == Some(&Value::Array(next_rows.clone())) {
        return false;
    }

    let mut fields = Map::new();
    fields.insert("children".to_string(), Value::Array(next_rows));
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new(grid_id.to_string()),
        patch_json: Value::Object(fields).to_string(),
        page_id: None,
    })
}

fn target_row_lengths(tile_count: usize) -> Vec<usize> {
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

fn clone_row_with_fresh_id(
    sink: &dyn DocSink,
    first_row: &Value,
    grid_id: &str,
    row_index: usize,
) -> Value {
    let mut row = first_row.clone();
    let id = crate::hero_bleed::unique_id(
        sink.state(),
        &format!("{grid_id}-reflow-row-{}", row_index + 1),
    );
    row["id"] = Value::String(id);
    row
}

#[cfg(test)]
#[path = "category_grid_reflow_tests.rs"]
mod tests;
