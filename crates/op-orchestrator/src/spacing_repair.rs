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
//!   stacks (gap 0) may legitimately breathe through wrapper padding — AND
//!   only when that gap can stand in for the padding (see
//!   [`gap_absorbs_vertical_padding`]) — AND never off the top-level sections of a
//!   scrolling page, whose vertical padding IS the page's rhythm (see
//!   `sections_own_the_rhythm` in [`strip_in_value`]).

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

use crate::design_type::{classify_root_form_value, DesignForm};

/// How far past the parent's gap a wrapper's vertical padding may reach and
/// still count as a DUPLICATE of that gap.
///
/// Deliberately NOT 1.0. This pass's original job — collapsing a weak model's
/// doubled wrapper insets on a phone screen — routinely sees a wrapper padded
/// slightly deeper than the column gaps it duplicates, and a strict `<= gap`
/// would disarm it there. The scrolling-page guard below (`sections_own_the_rhythm`)
/// is what protects a web page's own 24px sections; this constant only has to
/// refuse insets that DWARF the gap, which is a separate and much safer call.
const GAP_DUPLICATE_FACTOR: f64 = 1.5;

/// Can the parent column's `gap` stand in for a wrapper's vertical padding?
///
/// The vertical half of this pass removes double-spacing: padding that repeats
/// separation the column's gap ALREADY provides. That reading holds only while
/// the two are of the same order. Padding that dwarfs the gap is not repeating
/// it, it is carrying rhythm the gap cannot express, and zeroing it deletes
/// authored spacing instead of de-duplicating it.
///
/// Measured on `0808-gm-1.op`: a marketing page root gapped 20 with eight
/// sections each carrying 24–80px of their own vertical inset came back with
/// ALL EIGHT at `[0, H, 0, H]`, collapsing the page's section rhythm into a
/// flat 20px stack — the user's "web 顶部和底部应该有空间" report.
///
/// BOTH sides must be absorbable: a pair that only half-fits the gap is
/// rhythm, and flattening one edge of it would skew the wrapper rather than
/// de-duplicate it.
fn gap_absorbs_vertical_padding(top: f64, bottom: f64, gap: f64) -> bool {
    let absorbable = gap * GAP_DUPLICATE_FACTOR;
    top <= absorbable && bottom <= absorbable
}

/// Strip redundant wrapper padding under padded/gapped columns. Returns
/// `true` iff any padding changed. Same `Value` round-trip as the
/// `table_repair` passes.
pub(crate) fn strip_wrapper_double_inset(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    // The artboard is the only design-type signal a pass gets on the agentic
    // loop path (no plan exists there), so classify it once at the root and
    // thread it down rather than re-deriving it per level.
    let form = classify_root_form_value(&v);
    if !strip_in_value(&mut v, false, form, 0) {
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

/// `gutter_above` — some ancestor reached through an unbroken chain of
/// NON-PAINTING frames already owns a horizontal gutter, so this level's
/// children may not re-add one either. Without it a chain of transparent
/// wrappers only lost its outermost duplicate: stripping the first layer left
/// the second one's parent unpadded, which read as "nobody owns the gutter
/// here" and let the third layer keep an inset it had no claim to — the same
/// misalignment this pass exists to remove, one level deeper. It does NOT
/// cross a painting surface: a card's own padding is its inner inset, not a
/// rail gutter, so `is_painting_surface` resets the chain.
fn strip_in_value(v: &mut Value, gutter_above: bool, form: DesignForm, depth: usize) -> bool {
    let mut changed = false;
    let is_column = v.get("layout").and_then(Value::as_str) == Some("vertical");
    let (pt, pr, pb, pl) = padding_sides(v);
    let parent_pads_h = (pr >= 16.0 && pl >= 16.0) || gutter_above;
    let gap = num(v, "gap");
    let parent_gaps = gap >= 12.0;
    let _ = (pt, pb);
    // A SCROLLING PAGE whose root column establishes no inset of its own has
    // delegated the whole frame to its sections: their vertical padding IS the
    // page's section rhythm, and the root gap is a separator between sections,
    // not a substitute for them. A dashboard's main column — which pads itself
    // (`parent_pads_h`) — keeps the original reading, and so does every level
    // below the root, where an interior strip really can duplicate the column.
    let sections_own_the_rhythm = form.is_scrolling_page() && depth == 0 && !parent_pads_h;
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
                // Canonical bottom-nav padding is internal control clearance;
                // the parent's gap separates chrome from content. Removing it
                // here makes first finalize disagree with chrome normalization.
                let owns_chrome_inset =
                    child.get("role").and_then(Value::as_str) == Some("bottom-tab-bar");
                if parent_gaps
                    && transparent_v
                    && !owns_chrome_inset
                    && gap_absorbs_vertical_padding(t, b, gap)
                    && !sections_own_the_rhythm
                    && (t > 0.0 || b > 0.0)
                {
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
            let inherits = parent_pads_h && !is_painting_surface(c);
            changed |= strip_in_value(c, inherits, form, depth + 1);
        }
    }
    changed
}

/// Does this node paint a surface of its own — a fill, a clip, or any stroke
/// beyond a top/bottom rule? Such a node re-establishes the inset frame for
/// everything below it, so an ancestor's gutter claim stops here.
fn is_painting_surface(v: &Value) -> bool {
    let paints_fill = v
        .get("fill")
        .map(|f| match f {
            Value::Array(a) => !a.is_empty(),
            Value::Null => false,
            _ => true,
        })
        .unwrap_or(false);
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    let strokes = match v.get("stroke") {
        None | Some(Value::Null) => false,
        Some(stroke) => !stroke_is_horizontal_rule_only(stroke),
    };
    paints_fill || clips || strokes
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
