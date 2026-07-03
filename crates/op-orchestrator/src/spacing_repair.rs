//! Spacing-rhythm repairs — double-inset stripping.
//!
//! A TRANSPARENT wrapper (no fill, no stroke — invisible except through the
//! positions of its children) that carries padding INSIDE an already-padded /
//! gapped column doesn't add design, it adds drift: its horizontal padding
//! misaligns the section's left edge against sibling sections, and squeezes
//! its children (measured: a "Key Metrics" strip padded [24,32] inside a
//! [32,40]-padded main column starved four KPI cards to 187px each — the
//! 124px label + 16px icon left `space_between` ZERO slack, reading as
//! "title jammed into icon"); its vertical padding double-spaces against the
//! column's gap. Reference dashboards put NO padding on section wrapper rows
//! — the content column's padding is the single horizontal inset.
//!
//! Gates (both sides must prove redundancy):
//! - horizontal padding stripped only when the PARENT column already pads
//!   horizontally ≥16 — a landing page whose root column is unpadded keeps
//!   its self-padding `[0,24]` sections untouched;
//! - vertical padding stripped only when the parent column gaps ≥12 — flush
//!   stacks (gap 0) may legitimately breathe through wrapper padding.

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

/// Strip redundant wrapper padding under padded/gapped columns. Returns
/// `true` iff any padding changed. Same `Value` round-trip as the
/// `table_repair` passes.
pub(crate) fn strip_wrapper_double_inset(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !strip_in_value(&mut v) {
        return false;
    }
    match serde_json::from_value::<PenNode>(v) {
        Ok(new_node) => {
            *root = new_node;
            true
        }
        Err(_) => false,
    }
}

fn strip_in_value(v: &mut Value) -> bool {
    let mut changed = false;
    let is_column = v.get("layout").and_then(Value::as_str) == Some("vertical");
    let (pt, pr, pb, pl) = padding_sides(v);
    let parent_pads_h = pr >= 16.0 && pl >= 16.0;
    let parent_gaps = num(v, "gap") >= 12.0;
    let _ = (pt, pb);
    if is_column && (parent_pads_h || parent_gaps) {
        if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
            for child in kids.iter_mut() {
                let (transparent_h, transparent_v) = wrapper_transparency(child);
                if !transparent_h && !transparent_v {
                    continue;
                }
                let (t, r, b, l) = padding_sides(child);
                let (mut t, mut r, mut b, mut l) = (t, r, b, l);
                let mut touched = false;
                if parent_pads_h && transparent_h && (r > 0.0 || l > 0.0) {
                    r = 0.0;
                    l = 0.0;
                    touched = true;
                }
                if parent_gaps && transparent_v && (t > 0.0 || b > 0.0) {
                    t = 0.0;
                    b = 0.0;
                    touched = true;
                }
                if touched {
                    if let Some(obj) = child.as_object_mut() {
                        if t == 0.0 && r == 0.0 && b == 0.0 && l == 0.0 {
                            obj.remove("padding");
                        } else {
                            obj.insert("padding".into(), json!([t, r, b, l]));
                        }
                        changed = true;
                    }
                }
            }
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= strip_in_value(c);
        }
    }
    changed
}

/// Per-axis padding transparency of a padded layout wrapper —
/// `(horizontal, vertical)`. A wrapper that paints NOTHING is transparent on
/// both axes. A wrapper whose only paint is a TOP/BOTTOM hairline is a
/// DIVIDER BAR (a top header with its bottom rule): its horizontal padding
/// is still pure inset drift (strippable — a `[20,32]`-padded header inside
/// a `[32,40]`-padded column started its title 32px deeper than the cards
/// below it, measured), while its vertical padding is breathing against its
/// own rule line — kept. Any fill / side or uniform stroke / clip → a real
/// surface, opaque on both axes.
fn wrapper_transparency(v: &Value) -> (bool, bool) {
    if v.get("type").and_then(Value::as_str) != Some("frame") {
        return (false, false);
    }
    if !matches!(
        v.get("layout").and_then(Value::as_str),
        Some("vertical" | "horizontal")
    ) {
        return (false, false);
    }
    let (t, r, b, l) = padding_sides(v);
    if t <= 0.0 && r <= 0.0 && b <= 0.0 && l <= 0.0 {
        return (false, false);
    }
    let paints_fill = v
        .get("fill")
        .map(|f| match f {
            Value::Array(a) => !a.is_empty(),
            Value::Null => false,
            _ => true,
        })
        .unwrap_or(false);
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    let has_children = v
        .get("children")
        .and_then(Value::as_array)
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    if paints_fill || clips || !has_children {
        return (false, false);
    }
    match v.get("stroke") {
        None | Some(Value::Null) => (true, true),
        Some(stroke) => {
            if stroke_is_horizontal_rule_only(stroke) {
                (true, false)
            } else {
                (false, false)
            }
        }
    }
}

/// Does this stroke paint ONLY top/bottom rules (no left/right, no uniform
/// frame)? Accepts the `{"thickness": {...sides}}` and `[t, r, b, l]` forms.
fn stroke_is_horizontal_rule_only(stroke: &Value) -> bool {
    match stroke.get("thickness") {
        Some(Value::Object(sides)) => {
            let side = |k: &str| sides.get(k).and_then(Value::as_f64).unwrap_or(0.0);
            (side("top") > 0.0 || side("bottom") > 0.0)
                && side("left") <= 0.0
                && side("right") <= 0.0
        }
        Some(Value::Array(a)) if a.len() == 4 => {
            let side = |i: usize| a[i].as_f64().unwrap_or(0.0);
            (side(0) > 0.0 || side(2) > 0.0) && side(1) <= 0.0 && side(3) <= 0.0
        }
        _ => false,
    }
}

/// Padding as `(top, right, bottom, left)` — accepts number, `[v, h]`,
/// `[t, r, b, l]`, absent.
fn padding_sides(v: &Value) -> (f64, f64, f64, f64) {
    match v.get("padding") {
        Some(Value::Number(n)) => {
            let p = n.as_f64().unwrap_or(0.0);
            (p, p, p, p)
        }
        Some(Value::Array(a)) => match a.len() {
            1 => {
                let p = a[0].as_f64().unwrap_or(0.0);
                (p, p, p, p)
            }
            2 => {
                let pv = a[0].as_f64().unwrap_or(0.0);
                let ph = a[1].as_f64().unwrap_or(0.0);
                (pv, ph, pv, ph)
            }
            4 => (
                a[0].as_f64().unwrap_or(0.0),
                a[1].as_f64().unwrap_or(0.0),
                a[2].as_f64().unwrap_or(0.0),
                a[3].as_f64().unwrap_or(0.0),
            ),
            _ => (0.0, 0.0, 0.0, 0.0),
        },
        _ => (0.0, 0.0, 0.0, 0.0),
    }
}

fn num(v: &Value, key: &str) -> f64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

#[cfg(test)]
#[path = "spacing_repair_tests.rs"]
mod tests;
