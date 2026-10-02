//! Crop a stage-boxed path to the geometry it actually draws.
//!
//! The painter fits a path's tight `d` bounds into the node box, scaling each
//! axis independently (`fit_path_to_rect`). A model drawing a map or a chart
//! writes every path in the stage's own pixel space and gives each one the
//! stage's full box: the lines that span the stage survive, but a 15px
//! arrowhead boxed 888×612 is stretched into an 888×612 triangle, and five
//! district outlines each become a stage-sized polygon. Measured, showcase
//! s03 (Opus 5.5, 2026-10-02, both runs): 14 and 16 map paths all boxed
//! `(0, 0, W, H)` of their `layout: "none"` stage.
//!
//! **Contract, not taste**: a path whose box equals its absolute parent's
//! box but whose `d` covers only part of it was authored in node space —
//! nobody stretches a glyph to fill a whole stage. Setting the box to the
//! `d` bounds makes the fit an identity and leaves every other path alone.

use super::{find_root, is_status_bar_from_json};
use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::{json, Value};

/// Box sides must match the parent this closely.
const BOX_EPS: f64 = 0.5;
/// A `d` that already fills its box on both axes is left as authored.
const FILLS_RATIO: f64 = 0.98;
/// Degenerate bounds (a vertical hairline) have no box to crop to.
const MIN_BOUNDS_SIDE: f64 = 1.0;

pub(super) fn crop_stage_boxed_paths(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = find_root(sink.state(), root_id) else {
        return 0;
    };
    let Ok(root_value) = serde_json::to_value(root) else {
        return 0;
    };
    let mut patches = Vec::new();
    collect(&root_value, &mut patches);
    let mut applied = 0;
    for (id, patch) in patches {
        if sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(id),
            patch_json: patch.to_string(),
            page_id: None,
        }) {
            applied += 1;
        }
    }
    applied
}

fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

fn collect(node: &Value, patches: &mut Vec<(String, Value)>) {
    if is_status_bar_from_json(node) {
        return;
    }
    let children = node
        .get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let absolute = matches!(
        node.get("layout").and_then(Value::as_str),
        None | Some("none")
    );
    if absolute {
        if let (Some(w), Some(h)) = (number(node, "width"), number(node, "height")) {
            for child in children {
                if let Some(patch) = crop(child, w, h) {
                    patches.push(patch);
                }
            }
        }
    }
    for child in children {
        collect(child, patches);
    }
}

/// The box patch for a path boxed to its parent's full `(0, 0, w, h)` whose
/// `d` bounds lie inside that box and leave part of it empty.
fn crop(path: &Value, parent_w: f64, parent_h: f64) -> Option<(String, Value)> {
    if path.get("type").and_then(Value::as_str) != Some("path")
        || path
            .get("anchors")
            .and_then(Value::as_array)
            .is_some_and(|anchors| !anchors.is_empty())
    {
        return None;
    }
    let origin_at_zero = |key: &str| number(path, key).unwrap_or(0.0).abs() <= BOX_EPS;
    if !origin_at_zero("x") || !origin_at_zero("y") {
        return None;
    }
    let (w, h) = (number(path, "width")?, number(path, "height")?);
    if (w - parent_w).abs() > BOX_EPS || (h - parent_h).abs() > BOX_EPS {
        return None;
    }
    let d = path.get("d").and_then(Value::as_str)?.trim();
    let (bx, by, bw, bh) = op_editor_core::svg_path_data_bounds(d)?;
    let (bx, by, bw, bh) = (f64::from(bx), f64::from(by), f64::from(bw), f64::from(bh));
    if ![bx, by, bw, bh].iter().all(|v| v.is_finite())
        || bw < MIN_BOUNDS_SIDE
        || bh < MIN_BOUNDS_SIDE
        || bx < -BOX_EPS
        || by < -BOX_EPS
        || bx + bw > w + BOX_EPS
        || by + bh > h + BOX_EPS
    {
        return None;
    }
    if bw >= w * FILLS_RATIO && bh >= h * FILLS_RATIO {
        return None;
    }
    let id = path.get("id").and_then(Value::as_str)?;
    let round = |v: f64| (v * 100.0).round() / 100.0;
    Some((
        id.to_string(),
        json!({
            "x": round(bx),
            "y": round(by),
            "width": round(bw),
            "height": round(bh),
        }),
    ))
}

#[cfg(test)]
#[path = "cleanup_path_stage_coords_tests.rs"]
mod tests;
