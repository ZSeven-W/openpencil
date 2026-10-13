//! Tolerant `serde_json::Value` readers shared by every geometry pass —
//! children / layout / fixed width / numeric props / padding, plus the
//! intentional-horizontal-scroller escape hatch.

use super::*;

// ── tolerant Value readers ──

pub(super) fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub(super) fn layout_str(v: &Value) -> Option<&str> {
    v.get("layout").and_then(Value::as_str)
}

/// Is this node an authored horizontal-scroll VIEWPORT rather than an ordinary
/// clipped horizontal card? Protecting every `horizontal + clipContent` subtree
/// is too broad: a media card can clip its cover while still containing a real
/// text/layout overflow that geometry should repair.
///
/// Require all three signals:
/// - semantic intent, or a fit-content lane with fixed-width items,
/// - the canonical viewport → one horizontal lane structure (either viewport axis),
/// - resolved proof that the lane's direct items exceed the viewport.
pub(super) fn is_intentional_horizontal_scroller(v: &Value, rects: &HashMap<String, Rect>) -> bool {
    let fixed_lane = op_design_lint::horizontal_scroll::is_clipped_horizontal_lane_viewport(v);
    if !fixed_lane
        && (layout_str(v) != Some("horizontal")
            || v.get("clipContent").and_then(Value::as_bool) != Some(true))
    {
        return false;
    }
    let [lane] = children(v) else {
        return false;
    };
    if layout_str(lane) != Some("horizontal") || children(lane).len() < 2 {
        return false;
    }
    if !fixed_lane && !has_scroller_semantics(v) && !has_scroller_semantics(lane) {
        return false;
    }
    let Some(viewport) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
    else {
        return false;
    };
    let viewport_right = viewport.x + viewport.w;
    children(lane).iter().any(|item| {
        item.get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .is_some_and(|item_rect| item_rect.x + item_rect.w > viewport_right + TEXT_OVERFLOW_EPS)
    })
}

pub(super) fn has_scroller_semantics(v: &Value) -> bool {
    ["name", "role"].iter().any(|key| {
        v.get(*key).and_then(Value::as_str).is_some_and(|value| {
            let value = value.to_ascii_lowercase();
            ["scroll", "viewport", "rail", "carousel"]
                .iter()
                .any(|token| value.contains(token))
        })
    })
}

/// A cell's fixed (numeric) width, or `None` when it is a keyword sizing
/// (`fill_container` / `fit_content`).
pub(super) fn fixed_width(v: &Value) -> Option<f64> {
    match v.get("width") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

pub(super) fn num(v: &Value, key: &str) -> f64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Parse authored numeric padding into CSS-order `[top, right, bottom, left]`.
///
/// `None` means the padding is expression-backed or malformed. Callers that
/// mutate padding must preserve that distinction instead of replacing intent
/// they cannot evaluate; diagnostic callers may conservatively treat it as no
/// known allowance.
pub(super) fn numeric_padding_sides(v: &Value) -> Option<[f64; 4]> {
    match v.get("padding") {
        None | Some(Value::Null) => Some([0.0; 4]),
        Some(Value::Number(value)) => {
            let padding = value.as_f64()?;
            Some([padding; 4])
        }
        Some(Value::Array(values)) if values.len() == 1 => {
            let padding = values[0].as_f64()?;
            Some([padding; 4])
        }
        Some(Value::Array(values)) if values.len() == 2 => {
            let vertical = values[0].as_f64()?;
            let horizontal = values[1].as_f64()?;
            Some([vertical, horizontal, vertical, horizontal])
        }
        Some(Value::Array(values)) if values.len() == 4 => Some([
            values[0].as_f64()?,
            values[1].as_f64()?,
            values[2].as_f64()?,
            values[3].as_f64()?,
        ]),
        _ => None,
    }
}

/// Known vertical padding that can legitimately participate in post-layout
/// content reconciliation. Negative values are not useful breathing room and
/// therefore cannot widen a spill tolerance.
pub(super) fn numeric_vertical_padding(v: &Value) -> Option<f64> {
    let [top, _, bottom, _] = numeric_padding_sides(v)?;
    Some(top.max(0.0) + bottom.max(0.0))
}

/// Sum of a frame's LEFT + RIGHT padding — the schema authors `padding` as a
/// number (all sides), `[vertical, horizontal]`, or `[top, right, bottom,
/// left]`. The overflow math must compare column widths against the row's
/// INNER width; a `[12, 16]`-padded 860px row only offers 828px to its cells,
/// and ignoring that put a real table 2px on the "fits" side of the gate
/// while its flex column starved to 6px (measured: test0703.op).
pub(super) fn horizontal_padding(v: &Value) -> f64 {
    numeric_padding_sides(v)
        .map(|[_, right, _, left]| right + left)
        .unwrap_or(0.0)
}

#[cfg(test)]
#[path = "geometry_carousel_viewport_tests.rs"]
mod carousel_tests;
