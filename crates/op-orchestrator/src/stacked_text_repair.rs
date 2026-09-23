//! Stacked overlapping-text repair (fake count-up roll).
//!
//! A `layout:none` window holding >=2 text children at the exact same
//! authored x/y (equal or absent widths) is a count-up "roll" the model
//! faked by stacking a start value under an end value and letting
//! animations swap them (measured: ring-fix-proof-0921/app-01
//! `本次时长卡-数字窗` — a `0` start node painted OVER the `12` end node,
//! because lower indexes paint on top in OpenPencil; app-05's
//! `Calorie Count Step` stack and motion50 other-06's `step-number-roller`
//! share the shape). In any STATIC state every layer paints, so the export
//! shows the counter's START instead of its result. The fix keeps the LAST
//! text (authors write roll frames in time order, so the last one carries
//! the final value) and hides the rest with `opacity: 0` — nodes stay (the
//! roll animation still plays), coordinates and order are untouched.
//! Purely STRUCTURAL: node names never gate the predicate, they only feed
//! the diagnostics line. When in doubt, decline (`宁可漏不可误`).

use op_editor_core::{EditorCommand, NodeId};
use serde_json::Value;

use crate::types::DocSink;

/// Hide every stacked roll start value under `root_id` (keep each window's
/// last text). Returns whether any edit was applied.
pub fn repair_stacked_overlapping_texts(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let Some(root) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        return false;
    };
    let Ok(v) = serde_json::to_value(root) else {
        return false;
    };
    let mut ids: Vec<String> = Vec::new();
    collect_hide_ids(&v, &mut ids);
    if ids.is_empty() {
        return false;
    }
    for id in ids {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(id),
            patch_json: serde_json::json!({ "opacity": 0.0 }).to_string(),
            page_id: None,
        });
    }
    true
}

/// Lint-side twin of the repair predicate — same shape proof, reported
/// instead of fixed, mirroring `stub_repair::empty_decorated_stub_diagnostic`.
pub(crate) fn stacked_overlapping_text_diagnostic(v: &Value) -> Option<String> {
    let texts = stacked_overlap(v)?;
    if !texts[..texts.len() - 1].iter().any(|t| needs_hide(t)) {
        // Every start layer is already hidden — the repair has run (or the
        // author hid them); stand down like the ring twin post-fix.
        return None;
    }
    let kept = texts.last().expect("len >= 2");
    let kept_content = kept.get("content").and_then(Value::as_str).unwrap_or("?");
    Some(format!(
        "{}: stacked-overlapping-text — {} text nodes share the same x/y inside a layout:none \
         window (a fake count-up roll); in a static export every layer paints and lower indexes \
         paint ON TOP, so the START value covers the result. Keep the last text (\"{}\") and set \
         opacity 0 on the rest, or use ONE Text node with the final value.",
        diag_label(v),
        texts.len(),
        kept_content
    ))
}

fn collect_hide_ids(v: &Value, out: &mut Vec<String>) {
    if let Some(texts) = stacked_overlap(v) {
        for t in &texts[..texts.len() - 1] {
            if needs_hide(t) {
                if let Some(id) = t.get("id").and_then(Value::as_str) {
                    out.push(id.to_string());
                }
            }
        }
    }
    for child in children(v) {
        collect_hide_ids(child, out);
    }
}

/// The full structural predicate — every clause must hold (宁可漏不可误):
/// a visible `layout:none` frame with >=2 direct text children whose
/// authored (x, y) are ALL present and exactly equal, and whose widths are
/// all equal or all absent. Returns the stacked text children.
fn stacked_overlap(v: &Value) -> Option<Vec<&Value>> {
    if v.get("type").and_then(Value::as_str) != Some("frame")
        || v.get("layout").and_then(Value::as_str) != Some("none")
        || v.get("visible").and_then(Value::as_bool) == Some(false)
    {
        return None;
    }
    let texts: Vec<&Value> = children(v)
        .iter()
        .filter(|c| c.get("type").and_then(Value::as_str) == Some("text"))
        .collect();
    if texts.len() < 2 {
        return None;
    }
    let (x0, y0) = (numeric(texts[0], "x")?, numeric(texts[0], "y")?);
    if texts[1..]
        .iter()
        .any(|t| numeric(t, "x") != Some(x0) || numeric(t, "y") != Some(y0))
    {
        return None;
    }
    let width0 = texts[0].get("width");
    if texts[1..].iter().any(|t| t.get("width") != width0) {
        return None;
    }
    Some(texts)
}

/// A roll layer needs hiding when nothing already pins it to invisible:
/// missing opacity → hide, exact 0 → done (idempotence), any other number
/// → the layer still paints statically, hide it. A non-numeric (expression)
/// opacity is an authored binding — leave it to its owner.
fn needs_hide(t: &Value) -> bool {
    match t.get("opacity") {
        None | Some(Value::Null) => true,
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::String(s)) => s.parse::<f64>().is_ok_and(|v| v != 0.0),
        _ => false,
    }
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn numeric(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn diag_label(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("frame");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
    format!("{name} ({id})")
}

#[cfg(test)]
#[path = "stacked_text_repair_tests.rs"]
mod tests;
