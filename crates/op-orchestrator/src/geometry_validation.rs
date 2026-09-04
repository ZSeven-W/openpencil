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

use crate::design_type::DesignForm;
use crate::types::DocSink;

#[path = "geometry_echo.rs"]
mod geometry_echo;
pub(crate) use geometry_echo::geometry_diagnostics_for_roots;

#[path = "geometry_compact_status.rs"]
mod geometry_compact_status;
use geometry_compact_status::{
    compact_status_header_is_damaged, compact_trailing_status_pair, is_compact_status_badge,
    is_compact_status_badge_structure, stack_compact_status_header,
};

#[path = "text_collision.rs"]
mod text_collision;
use text_collision::push_text_collision_diagnostics;

// Cluster submodules: this file keeps `Rect` + the resolved-rect map + the
// public drivers (`fix_table_column_overflow` / `geometry_validate_and_fix` /
// `geometry_diagnostics` / `fix_rail_width_collapse`); each family of
// collectors lives in its own file and is re-imported here so the drivers (and
// the test modules mounted below) see the same flat namespace as before.
#[path = "geometry_bottom_gap.rs"]
mod geometry_bottom_gap;
use geometry_bottom_gap::push_mobile_bottom_gap_diagnostic;
pub(crate) use geometry_bottom_gap::{
    repair_mobile_bottom_breathing, repair_mobile_bottom_breathing_for_all_roots,
};
#[path = "geometry_interaction_backfill.rs"]
mod geometry_interaction_backfill;
use geometry_interaction_backfill::push_interaction_backfill_diagnostics;
pub(crate) use geometry_interaction_backfill::{
    screen_has_back_control_shape, wire_interaction_backfill,
};
#[path = "geometry_buried_overlay.rs"]
mod geometry_buried_overlay;
#[path = "geometry_card_rail_fixes.rs"]
mod geometry_card_rail_fixes;
#[path = "geometry_diagnostics_collect.rs"]
mod geometry_diagnostics_collect;
#[path = "geometry_grow_fit_fixes.rs"]
mod geometry_grow_fit_fixes;
#[path = "geometry_overflow_fixes.rs"]
mod geometry_overflow_fixes;
#[path = "geometry_row_fixes.rs"]
mod geometry_row_fixes;
#[path = "geometry_scale_ops.rs"]
mod geometry_scale_ops;
#[path = "geometry_spill_diagnostics.rs"]
mod geometry_spill_diagnostics;
#[path = "geometry_starved_row.rs"]
mod geometry_starved_row;
#[path = "geometry_value_readers.rs"]
mod geometry_value_readers;

use geometry_card_rail_fixes::*;
use geometry_diagnostics_collect::*;
use geometry_grow_fit_fixes::*;
use geometry_overflow_fixes::*;
use geometry_row_fixes::*;
use geometry_scale_ops::*;
use geometry_spill_diagnostics::*;
use geometry_value_readers::*;

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f64,
    /// Not read yet — vertical stacking-overlap detection will need it; kept
    /// so the resolved-rect map carries the full geometry.
    #[allow(dead_code)]
    y: f64,
    w: f64,
    h: f64,
}

/// EditorState → resolved absolute rect (w, h) per node id, via the SAME jian
/// flex pass `snapshot_layout` uses.
fn resolved_rects(state: &EditorState) -> HashMap<String, Rect> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let mut map = HashMap::new();
    if let Some(page) = scene.active_page() {
        collect_rects(&page.children, &mut map);
    }
    map
}

pub(crate) fn resolved_node_height(state: &EditorState, node_id: &str) -> Option<f64> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let page = scene.active_page()?;
    let root = find_scene_node(&page.children, node_id)?;
    resolved_subtree_height(root)
}

fn find_scene_node<'a>(nodes: &'a [SceneNode], node_id: &str) -> Option<&'a SceneNode> {
    for node in nodes {
        if node.id == node_id {
            return Some(node);
        }
        if let Some(found) = find_scene_node(&node.children, node_id) {
            return Some(found);
        }
    }
    None
}

/// Measure the resolved subtree against the node's own top edge.
///
/// `SceneNode::aggregate_bounds()` intentionally returns a bounded node's own
/// rectangle, so it cannot reveal descendants that overflow a fixed-height
/// root. Raw scene bounds are absolute; walking every descendant preserves the
/// real layout bottom even when an ancestor clips or has an authored height.
fn resolved_subtree_height(root: &SceneNode) -> Option<f64> {
    let root_top = f64::from(root.bounds.origin.y);
    if !root_top.is_finite() {
        return None;
    }
    let bottom = max_raw_bottom(root)?;
    let height = bottom - root_top;
    height.is_finite().then_some(height.max(0.0))
}

fn max_raw_bottom(node: &SceneNode) -> Option<f64> {
    let y = f64::from(node.bounds.origin.y);
    let height = f64::from(node.bounds.size.y);
    let own_bottom = if y.is_finite() && height.is_finite() && height >= 0.0 {
        let bottom = y + height;
        bottom.is_finite().then_some(bottom)
    } else {
        None
    };

    node.children
        .iter()
        .filter_map(max_raw_bottom)
        .fold(own_bottom, |max_bottom, child_bottom| {
            Some(
                max_bottom
                    .map(|current| current.max(child_bottom))
                    .unwrap_or(child_bottom),
            )
        })
}

fn collect_rects(nodes: &[SceneNode], map: &mut HashMap<String, Rect>) {
    for n in nodes {
        let b = n.aggregate_bounds();
        map.insert(
            n.id.clone(),
            Rect {
                x: f64::from(b.origin.x),
                y: f64::from(b.origin.y),
                w: f64::from(b.size.x),
                h: f64::from(b.size.y),
            },
        );
        collect_rects(&n.children, map);
    }
}

// ── Table column-overflow fix ──

/// Reserved width for each `fill_container` column so scaling leaves it room.
const MIN_FILL_COL: f64 = 40.0;
/// A flex column that CARRIES TEXT needs real room, not a sliver — 40px still
/// shreds "a.sterling@email.com" into a one-glyph-per-line tower that blows
/// the row to 400px tall. Reserve enough for a readable two-line wrap.
const MIN_FILL_TEXT_COL: f64 = 120.0;
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
    let form = root_design_form(sink.state(), root_id);
    geometry_validate_and_fix_for_form(sink, root_id, form)
}

/// The root's [`DesignForm`], read through the single classifier.
///
/// A root that has been replaced or removed since the caller last looked
/// classifies as `Unknown` — "no type information", never a default form.
pub fn root_design_form(state: &EditorState, root_id: &str) -> DesignForm {
    op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(root_id.to_string()))
        .map(crate::design_type::classify_root_form_node)
        .unwrap_or(DesignForm::Unknown)
}

/// [`geometry_validate_and_fix`] told which surface the root is.
///
/// The thresholds every collector below uses are tuned for a screen. A
/// projector board reads from the back row and takes its own margin, gap, and
/// line-height floors (deck-system spec §4.1). **No collector branches on
/// `form` yet** — the deck collectors land in a later batch, and this exists
/// so each of them takes the form from the single classifier instead of
/// measuring the root's width itself. The `debug` line makes the classification
/// visible while that is still true: "the deck floors did not fire" and "the
/// board was never classified as one" are otherwise the same symptom.
pub fn geometry_validate_and_fix_for_form(
    sink: &mut dyn DocSink,
    root_id: &str,
    form: DesignForm,
) -> usize {
    tracing::debug!(root_id, ?form, "geometry validation form");
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
            collect_text_overflow_fixes(&v, &rects, &mut cmds);
            collect_frame_overflow_fixes(&v, &rects, &mut cmds);
            collect_oversized_image_fixes(&v, &mut cmds);
            collect_absolute_fill_image_fixes(&v, &rects, &mut cmds);
            collect_grow_to_fit_fixes(&v, &rects, &mut cmds);
            collect_row_gap_fixes(&v, &rects, &mut cmds);
            collect_card_row_height_fixes(&v, &rects, &mut cmds, false);
            // BEFORE the inside-out overfull repair: a rigid row starved by
            // its own flex siblings must be widened at the ROW, not squeezed
            // through its columns (which have nothing to give).
            geometry_starved_row::collect_starved_rigid_row_fixes(&v, &rects, &mut cmds);
            geometry_buried_overlay::collect_buried_overlay_fixes(&v, &rects, &mut cmds);
            collect_row_overfull_fixes(&v, &rects, &mut cmds, false);
            collect_rail_width_collapse_fixes(&v, &rects, &mut cmds);
            collect_card_overflow_clips(&v, &rects, &mut cmds);
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

/// Cap on reported issues per call — enough to act on, small enough to not
/// drown the model's context.
const MAX_DIAGNOSTICS: usize = 8;

/// REPORT-mode counterpart of the fix loop: run the real jian layout over the
/// CURRENT document and describe — without fixing — what the resolved geometry
/// proves wrong (collapsed fill containers, table columns overflowing their
/// row, text overflowing its block). Attached to every `batch_design` tool
/// result (including its script-mode path, the default subagent generation
/// protocol) so the design agent SEES each batch's layout consequences
/// immediately and repairs them in-process — the deterministic analogue of
/// Pencil's per-batch `snapshot_layout` feedback, with the detection cost
/// paid in Rust instead of model turns.
pub fn geometry_diagnostics(state: &EditorState) -> Vec<String> {
    let rects = resolved_rects(state);
    let mut out = Vec::new();
    push_interaction_backfill_diagnostics(state, None, &mut out);
    for root in state.active_children() {
        if out.len() >= MAX_DIAGNOSTICS {
            break;
        }
        if let Ok(v) = serde_json::to_value(root) {
            bottom_nav_root_containment_diagnostic(&v, &rects, &mut out);
            bottom_nav_order_diagnostic(&v, &mut out);
            push_mobile_bottom_gap_diagnostic(&v, &rects, &mut out);
            push_text_collision_diagnostics(&v, &rects, &mut out);
            // Ring track/progress arcs left as flex siblings. Detection is
            // shared verbatim with the repair pass (`ring_repair`), so the
            // echo can never describe a violation the fixer disagrees with.
            crate::ring_repair::push_ring_fragment_diagnostics(&v, &mut out, MAX_DIAGNOSTICS);
            collect_diagnostics(&v, &rects, &mut out);
        }
    }
    out.truncate(MAX_DIAGNOSTICS);
    out
}

#[cfg(test)]
#[path = "geometry_chip_private_tests.rs"]
mod chip_private_tests;

#[cfg(test)]
#[path = "geometry_chip_pill_tests.rs"]
mod chip_pill_tests;
/// Detect + fix a card rail whose `fill_container` siblings collapsed
/// beside a fixed-width reference card. See
/// [`collect_rail_width_collapse_fixes`] for the failure mode. Returns
/// `true` iff any sibling was widened.
pub fn fix_rail_width_collapse(sink: &mut dyn DocSink, root_id: &str) -> bool {
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
        collect_rail_width_collapse_fixes(&v, &rects, &mut ops);
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

/// Table-shaped container whose TEXT-BEARING flex columns alone (at their
/// readable floors) exceed the row's inner width — unsalvageable by any
/// rescale; the design needs fewer columns. Returns `(columns, inner_px)`
/// of the worst row for the diagnostic.
fn table_columns_exceed_width(v: &Value, rects: &HashMap<String, Rect>) -> Option<(usize, i64)> {
    let rows = table_rows(v);
    if rows.is_empty() {
        return None;
    }
    for row in rows {
        let cells = children(row);
        let mut floor_sum = 0.0;
        for cell in cells {
            floor_sum += match fixed_width(cell) {
                Some(w) => w * MIN_SCALE,
                None => {
                    if bears_text(cell) {
                        MIN_FILL_TEXT_COL
                    } else {
                        MIN_FILL_COL
                    }
                }
            };
        }
        let Some(row_id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(inner) = rects.get(row_id).map(|r| r.w - horizontal_padding(row)) else {
            continue;
        };
        if inner > 1.0 && floor_sum > inner {
            return Some((cells.len(), inner.round() as i64));
        }
    }
    None
}

#[cfg(test)]
#[path = "geometry_validation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "geometry_rail_collapse_tests.rs"]
mod rail_collapse_tests;

#[cfg(test)]
#[path = "geometry_compact_status_tests.rs"]
mod compact_status_tests;

#[cfg(test)]
#[path = "geometry_resolved_extent_tests.rs"]
mod resolved_extent_tests;
