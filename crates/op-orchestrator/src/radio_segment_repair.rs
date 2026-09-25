//! A `radio_group` authored into a single-row box renders its options on top
//! of each other.
//!
//! The radio widget stacks one row per option down its box (row height =
//! box height / option count, capped at 28px) and draws each label at 14px.
//! A model that wants a period switcher — "近1月 / 近3月 / 今年" beside a
//! chart title — writes a `radio_group` sized like a segmented control
//! (arena-m03: 194×36 with a muted fill, border and radius 8 for three
//! options). Three rows in 36px are 12px apart, so the three 14px labels
//! paint over one another. No author intends that: it is a CONTRACT defect,
//! provable from the declared numbers alone.
//!
//! The authored box is the one thing the author did state, and only a
//! horizontal single-choice control fits it, so the repair re-types the node
//! as the schema's segmented control (`tabs` with no panels): same id, name,
//! box, fill, stroke, radius, options and selected value. A radio group whose
//! box can hold its stacked rows is left alone.

use jian_ops_schema::node::PenNode;
use serde_json::Value;

/// The smallest stacked row a 14px radio label can occupy without touching
/// its neighbour's glyphs.
const MIN_STACKED_ROW: f64 = 20.0;
/// The smallest segment that can carry a short label in a single row.
const MIN_SEGMENT_WIDTH: f64 = 40.0;

/// Whole-root transform (see `cleanup_root_transform::apply_root_transform`).
pub(crate) fn segment_single_row_radio_groups(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !segment_in_value(&mut v) {
        return false;
    }
    match serde_json::from_value::<PenNode>(v) {
        Ok(node) => {
            *root = node;
            true
        }
        Err(_) => false,
    }
}

fn numeric(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn is_squeezed_radio_row(v: &Value) -> bool {
    if v.get("type").and_then(Value::as_str) != Some("radio_group") {
        return false;
    }
    let options = v
        .get("options")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if options < 2 {
        return false;
    }
    let n = options as f64;
    let Some(height) = numeric(v, "height") else {
        // A hugging / filling height is sized by the widget or its parent;
        // only a declared number proves the squeeze.
        return false;
    };
    if height >= n * MIN_STACKED_ROW {
        return false;
    }
    // The single-row reading must actually fit: a fixed width wide enough
    // for one segment per option, or a width the parent provides.
    match v.get("width") {
        Some(Value::Number(w)) => w.as_f64().is_some_and(|w| w >= n * MIN_SEGMENT_WIDTH),
        Some(Value::String(s)) => s == "fill_container",
        _ => false,
    }
}

fn segment_in_value(v: &mut Value) -> bool {
    let mut changed = false;
    if is_squeezed_radio_row(v) {
        if let Some(obj) = v.as_object_mut() {
            obj.insert("type".into(), Value::String("tabs".into()));
            if let Some(options) = obj.remove("options") {
                obj.insert("tabs".into(), options);
            }
            changed = true;
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for kid in kids {
            changed |= segment_in_value(kid);
        }
    }
    changed
}

#[cfg(test)]
#[path = "radio_segment_repair_tests.rs"]
mod tests;
