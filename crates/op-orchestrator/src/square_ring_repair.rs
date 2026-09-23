//! Square ring-wrapper repair (motion50 fix 4).
//!
//! A tiny square frame with a non-transparent fill whose ONLY child is a
//! square ellipse (equal or smaller) is a status/presence dot or swatch ring
//! whose author forgot the corner radius: the frame paints OVER the ellipse as
//! a hard-cornered square (measured: lane0/app-08 `置顶头像容器/在线绿点` —
//! a 16×16 `$--background` frame over a 10×10 `#16A34A` ellipse rendering as a
//! white square stamped on the avatar corner; same shape in lane1/app-09's
//! five `swatch-ring-*` nodes). The fix is deliberately narrow —
//! `cornerRadius = width/2` on the frame, nothing else — and purely
//! STRUCTURAL: node names never gate the predicate, they only feed the
//! diagnostics line. When in doubt, decline (`宁可漏不可误`).

use std::collections::HashMap;

use op_editor_core::{EditorCommand, NodeId};
use serde_json::Value;

use crate::types::DocSink;

/// Wrappers above this edge are boards/cards, not dots or rings.
const MAX_EDGE: f64 = 40.0;

/// Round every square ring wrapper under `root_id`. Returns whether any edit
/// was applied.
pub fn repair_square_ring_wrappers(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let Some(root) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        return false;
    };
    let Ok(v) = serde_json::to_value(root) else {
        return false;
    };
    let sizes = resolved_sizes(sink.state());
    let mut hits: Vec<(String, f64)> = Vec::new();
    collect_square_ring_ids(&v, &sizes, &mut hits);
    if hits.is_empty() {
        return false;
    }
    for (id, radius) in hits {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(id),
            patch_json: serde_json::json!({ "cornerRadius": radius }).to_string(),
            page_id: None,
        });
    }
    true
}

/// Lint-side twin of the repair predicate — same shape proof, reported instead
/// of fixed, mirroring `stub_repair::empty_decorated_stub_diagnostic`.
pub(crate) fn square_ring_wrapper_diagnostic(
    v: &Value,
    resolved_size: Option<(f64, f64)>,
) -> Option<String> {
    let width = numeric(v, "width").or_else(|| resolved_size.map(|size| size.0))?;
    is_square_ring_wrapper(v, resolved_size).then(|| {
        format!(
            "{}: square-ring-wrapper — a square frame painted over a single smaller ellipse \
             reads as a hard-cornered square; set cornerRadius to half the frame width \
             ({}) or drop the frame",
            diag_label(v),
            width / 2.0
        )
    })
}

fn collect_square_ring_ids(
    v: &Value,
    sizes: &HashMap<String, (f64, f64)>,
    out: &mut Vec<(String, f64)>,
) {
    let resolved_size = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| sizes.get(id))
        .copied();
    if is_square_ring_wrapper(v, resolved_size) {
        if let (Some(id), Some(width)) = (
            v.get("id").and_then(Value::as_str),
            numeric(v, "width").or_else(|| resolved_size.map(|size| size.0)),
        ) {
            out.push((id.to_string(), width / 2.0));
        }
        // A wrapper holds exactly one ellipse child — no nested hits.
        return;
    }
    for child in children(v) {
        collect_square_ring_ids(child, sizes, out);
    }
}

/// The full structural predicate — every clause must hold (宁可漏不可误):
/// square frame ≤ 40px, non-transparent fill, exactly one child which is a
/// square ellipse no larger than the frame, and no corner radius yet.
fn is_square_ring_wrapper(v: &Value, resolved_size: Option<(f64, f64)>) -> bool {
    if v.get("type").and_then(Value::as_str) != Some("frame")
        || v.get("visible").and_then(Value::as_bool) == Some(false)
    {
        return false;
    }
    let (Some(width), Some(height)) = (
        numeric(v, "width").or_else(|| resolved_size.map(|size| size.0)),
        numeric(v, "height").or_else(|| resolved_size.map(|size| size.1)),
    ) else {
        return false;
    };
    if width <= 0.0
        || height <= 0.0
        || width > MAX_EDGE
        || height > MAX_EDGE
        || (width - height).abs() > 0.5
    {
        return false;
    }
    if !has_non_transparent_fill(v) {
        return false;
    }
    let kids = children(v);
    let Some((only, others)) = kids.split_first() else {
        return false;
    };
    if !others.is_empty() || only.get("type").and_then(Value::as_str) != Some("ellipse") {
        return false;
    }
    let (Some(kid_w), Some(kid_h)) = (numeric(only, "width"), numeric(only, "height")) else {
        return false;
    };
    if (kid_w - kid_h).abs() > 0.5 || kid_w > width {
        return false;
    }
    match numeric(v, "cornerRadius") {
        // Already circular (or rounder) — leave it alone.
        Some(radius) => radius < width / 2.0,
        None => true,
    }
}

fn has_non_transparent_fill(v: &Value) -> bool {
    match v.get("fill") {
        Some(Value::Array(paints)) => paints.iter().any(is_opaque_paint),
        Some(paint @ Value::Object(_)) => is_opaque_paint(paint),
        _ => false,
    }
}

fn is_opaque_paint(paint: &Value) -> bool {
    if paint
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.eq_ignore_ascii_case("none"))
    {
        return false;
    }
    paint
        .get("opacity")
        .and_then(Value::as_f64)
        .map(|opacity| opacity > 0.0)
        .unwrap_or(true)
}

fn resolved_sizes(state: &op_editor_core::EditorState) -> HashMap<String, (f64, f64)> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let mut out = HashMap::new();
    if let Some(page) = scene.active_page() {
        collect_sizes(&page.children, &mut out);
    }
    out
}

fn collect_sizes(
    nodes: &[jian_scene::layout_scene::SceneNode],
    out: &mut HashMap<String, (f64, f64)>,
) {
    for node in nodes {
        let bounds = node.aggregate_bounds();
        out.insert(
            node.id.clone(),
            (f64::from(bounds.size.x), f64::from(bounds.size.y)),
        );
        collect_sizes(&node.children, out);
    }
}

fn numeric(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn diag_label(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("frame");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
    format!("{name} ({id})")
}

#[cfg(test)]
#[path = "square_ring_repair_tests.rs"]
mod tests;
