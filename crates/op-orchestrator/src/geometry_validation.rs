//! Geometry-driven structural validation — the deterministic analogue of
//! Pencil's per-batch `snapshot_layout` feedback.
//!
//! The tree-shape post-passes (`role_layout_post_pass`, `table_repair`, …) reason
//! about the AUTHORED tree; they can't see where a node actually LANDS once jian
//! resolves flex sizing. A weak model that ignores the prompt's
//! "total_width ≤ parent_inner_width" rule (glm routinely does) emits fixed table
//! columns that sum WIDER than their row — jian then overflows them past the
//! right edge (columns overlap, the last column is clipped). No amount of
//! tree-shape heuristics catch it, because the widths are individually valid.
//!
//! This module runs the REAL jian layout (`editor_state_to_layout_scene`, the
//! same pass `snapshot_layout` uses), reads each node's RESOLVED absolute rect,
//! and fixes what the resolved geometry proves is wrong. First detector: a table
//! row whose fixed columns overflow its resolved width → scale the fixed columns
//! (and the column gap) down to fit, keeping proportions and column alignment.

use std::collections::HashMap;

use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, LayoutPropValue, NodeId};
use serde_json::Value;

use crate::types::DocSink;

#[derive(Clone, Copy)]
struct Rect {
    w: f64,
    h: f64,
}

/// EditorState → resolved absolute rect (w, h) per node id, via the SAME jian
/// flex pass `snapshot_layout` uses.
fn resolved_rects(state: &EditorState) -> HashMap<String, Rect> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let mut map = HashMap::new();
    for page in &scene.pages {
        collect_rects(&page.children, &mut map);
    }
    map
}

fn collect_rects(nodes: &[SceneNode], map: &mut HashMap<String, Rect>) {
    for n in nodes {
        let b = n.aggregate_bounds();
        map.insert(
            n.id.clone(),
            Rect {
                w: f64::from(b.size.x),
                h: f64::from(b.size.y),
            },
        );
        collect_rects(&n.children, map);
    }
}

// ── tolerant Value readers ──

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn layout_str(v: &Value) -> Option<&str> {
    v.get("layout").and_then(Value::as_str)
}

fn ident_text(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("");
    format!("{name} {id}").to_lowercase()
}

/// A cell's fixed (numeric) width, or `None` when it is a keyword sizing
/// (`fill_container` / `fit_content`).
fn fixed_width(v: &Value) -> Option<f64> {
    match v.get("width") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn num(v: &Value, key: &str) -> f64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn is_table_named(t: &str) -> bool {
    t.contains("table") || t.contains("data grid") || t.contains("datagrid")
}

// ── Table column-overflow fix ──

/// Reserved width for each `fill_container` column so scaling leaves it room.
const MIN_FILL_COL: f64 = 40.0;
/// Slack so a table that fits by a hair isn't needlessly rescaled.
const OVERFLOW_EPS: f64 = 4.0;
/// Leave a little breathing room after scaling.
const FIT_MARGIN: f64 = 0.97;
/// Never shrink columns below this fraction of their authored width — beyond it
/// the table is unsalvageable by scaling and needs a structural rethink (left to
/// a later detector); clamp so we don't produce 5px columns.
const MIN_SCALE: f64 = 0.35;

/// Detect + fix table rows whose FIXED columns overflow their resolved width.
/// Returns `true` iff any column was rescaled. A numeric width is set via
/// `UpdateNode` (`SetNodeLayoutProp` only accepts KEYWORD widths); the column gap
/// via `SetNodeLayoutProp`. Both mutate props in place — node ids never change,
/// so no `ReplaceSubtree` id churn.
pub fn fix_table_column_overflow(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let rects = resolved_rects(sink.state());
    let ops: Vec<EditorCommand> = {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return false;
        };
        let Ok(v) = serde_json::to_value(root) else {
            return false;
        };
        let mut ops = Vec::new();
        collect_scale_ops(&v, &rects, &mut ops);
        ops
    };
    if ops.is_empty() {
        return false;
    }
    for cmd in ops {
        sink.apply(cmd);
    }
    true
}

/// Max detect→fix→relayout rounds (Pencil's multi-round `snapshot_layout`
/// feedback, bounded so a hard-to-fix design can't spin).
const MAX_ROUNDS: usize = 3;

/// Geometry-driven validation LOOP: recompute the REAL layout, detect + fix the
/// structural violations the resolved rects prove (table column overflow,
/// collapsed fill containers), and repeat until a round finds nothing to do or
/// `MAX_ROUNDS` is hit. The deterministic analogue of Pencil's per-batch
/// `snapshot_layout` feedback. Returns the number of rounds that applied a fix.
pub fn geometry_validate_and_fix(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let mut rounds = 0;
    for _ in 0..MAX_ROUNDS {
        let rects = resolved_rects(sink.state());
        let cmds = {
            let Some(root) = op_editor_core::walkers::find_node(
                sink.state().active_children(),
                &NodeId::new(root_id.to_string()),
            ) else {
                break;
            };
            let Ok(v) = serde_json::to_value(root) else {
                break;
            };
            let mut cmds = Vec::new();
            collect_scale_ops(&v, &rects, &mut cmds);
            collect_collapse_fixes(&v, &rects, &mut cmds);
            cmds
        };
        if cmds.is_empty() {
            break;
        }
        for cmd in cmds {
            sink.apply(cmd);
        }
        rounds += 1;
    }
    rounds
}

/// A container resolves to ~0 height while declaring `fill_container` — it asked
/// to fill an ancestor that HUGS (a transitive `fit_content` chain the tree-shape
/// `fix_circular_fill_height` misses, since that pass only checks the DIRECT
/// parent). Its sized children then pile up / overlap.
const COLLAPSE_H: f64 = 6.0;
/// A child must carry real height for the parent's 0-height to read as a collapse
/// (not an intentionally-empty spacer).
const CHILD_MIN_H: f64 = 12.0;

fn collect_collapse_fixes(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>) {
    if is_collapsed_fill_container(v, rects) {
        if let Some(id) = v.get("id").and_then(Value::as_str) {
            // Make it hug its content instead of filling a hugging ancestor.
            cmds.push(EditorCommand::SetNodeLayoutProp {
                node_id: NodeId::new(id.to_string()),
                property: "height".to_string(),
                value: LayoutPropValue::Keyword("fit_content".to_string()),
            });
            // A now-hug vertical column can't distribute on the main axis — jian
            // collapses `space_between` there to an overlap. Top-pack + gap.
            let distributes = matches!(
                v.get("justifyContent").and_then(Value::as_str),
                Some("space_between" | "space_around" | "space_evenly")
            );
            if distributes && layout_str(v) == Some("vertical") {
                cmds.push(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(id.to_string()),
                    property: "justifyContent".to_string(),
                    value: LayoutPropValue::Keyword("start".to_string()),
                });
                if num(v, "gap") <= 0.0 {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_string()),
                        property: "gap".to_string(),
                        value: LayoutPropValue::Number(8.0),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_collapse_fixes(c, rects, cmds);
    }
}

fn is_collapsed_fill_container(v: &Value, rects: &HashMap<String, Rect>) -> bool {
    if v.get("height").and_then(Value::as_str) != Some("fill_container") {
        return false;
    }
    let Some(id) = v.get("id").and_then(Value::as_str) else {
        return false;
    };
    let Some(rect) = rects.get(id) else {
        return false;
    };
    if rect.h >= COLLAPSE_H {
        return false;
    }
    // A real child with height proves the 0-height parent is a collapse.
    children(v).iter().any(|c| {
        c.get("id")
            .and_then(Value::as_str)
            .and_then(|cid| rects.get(cid))
            .map(|r| r.h >= CHILD_MIN_H)
            .unwrap_or(false)
    })
}

fn collect_scale_ops(v: &Value, rects: &HashMap<String, Rect>, ops: &mut Vec<EditorCommand>) {
    if let Some(scale) = table_overflow_scale(v, rects) {
        // Apply the same scale to EVERY row's fixed cells (columns stay aligned)
        // and to each row's gap.
        for row in children(v) {
            if layout_str(row) != Some("horizontal") {
                continue;
            }
            let cells = children(row);
            if cells.len() < 3 {
                continue;
            }
            for cell in cells {
                if let (Some(w), Some(id)) =
                    (fixed_width(cell), cell.get("id").and_then(Value::as_str))
                {
                    ops.push(EditorCommand::UpdateNode {
                        node_id: NodeId::new(id.to_string()),
                        x: None,
                        y: None,
                        width: Some((w * scale).round() as i32),
                        height: None,
                        name: None,
                        fill_hex: None,
                        page_id: None,
                    });
                }
            }
            let gap = num(row, "gap");
            if gap > 0.0 {
                if let Some(id) = row.get("id").and_then(Value::as_str) {
                    ops.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_string()),
                        property: "gap".to_string(),
                        value: LayoutPropValue::Number((gap * scale).round()),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_scale_ops(c, rects, ops);
    }
}

/// If `v` is a table container (a `table`-named vertical frame with ≥2 horizontal
/// rows of ≥3 cells) whose columns overflow the row's RESOLVED width, return the
/// scale factor (< 1.0) to apply to its fixed columns + gap. `None` when it isn't
/// a table or doesn't overflow.
fn table_overflow_scale(v: &Value, rects: &HashMap<String, Rect>) -> Option<f64> {
    if layout_str(v) == Some("horizontal") {
        return None;
    }
    if !is_table_named(&ident_text(v)) {
        return None;
    }
    // Representative row: the first horizontal child with ≥3 cells.
    let row = children(v)
        .iter()
        .find(|r| layout_str(r) == Some("horizontal") && children(r).len() >= 3)?;
    // Need at least a header + one data row to be a real table.
    let data_rows = children(v)
        .iter()
        .filter(|r| layout_str(r) == Some("horizontal") && children(r).len() >= 3)
        .count();
    if data_rows < 2 {
        return None;
    }
    let cells = children(row);
    let n_gaps = (cells.len() - 1) as f64;
    let gap = num(row, "gap");
    let mut fixed_sum = 0.0;
    let mut fill_count = 0.0;
    for cell in cells {
        match fixed_width(cell) {
            Some(w) => fixed_sum += w,
            None => fill_count += 1.0, // fill_container / fit_content
        }
    }
    if fixed_sum <= 0.0 {
        return None; // all-flex table can't overflow via fixed widths
    }
    let row_id = row.get("id").and_then(Value::as_str)?;
    let row_w = rects.get(row_id)?.w;
    if row_w <= 1.0 {
        return None;
    }
    // Minimum width the row NEEDS: fixed columns + gaps + a floor for each flex
    // column. If that already fits the resolved row width, nothing to do.
    let needed = fixed_sum + gap * n_gaps + fill_count * MIN_FILL_COL;
    if needed <= row_w + OVERFLOW_EPS {
        return None;
    }
    // Scale the fixed budget (columns + gaps) so it fits alongside the flex floor.
    let flex_floor = fill_count * MIN_FILL_COL;
    let fixed_budget = (row_w - flex_floor) * FIT_MARGIN;
    let scalable = fixed_sum + gap * n_gaps;
    if scalable <= 0.0 {
        return None;
    }
    let scale = (fixed_budget / scalable).clamp(MIN_SCALE, 1.0);
    if scale >= 1.0 - 0.001 {
        return None;
    }
    Some(scale)
}

#[cfg(test)]
#[path = "geometry_validation_tests.rs"]
mod tests;
