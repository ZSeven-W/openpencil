//! The width a text node is ALLOTTED by its flex ancestors, as opposed to
//! the width it resolved to.
//!
//! A `fill_container` text leaf keeps taffy's `min-size: auto` (only fill
//! CONTAINERS get `min-size: 0` in jian), so an unbreakable amount holds its
//! min-content width and spills out of its parent over the next sibling
//! while its own rect still equals its natural width (arena-m03: the
//! "+1,286.40" of the 今日收益 block painted across the "+38,640.18" of the
//! 累计收益 block). Nothing in the text's own rect overflows, so the width
//! the text must fit is recomputed from the layout the author wrote.

use super::super::*;

/// Walk `chain` (root … node) and return the width the last node is given:
/// a `fill_container` node gets its parent's allotted inner width — split,
/// in a horizontal row, among the fill siblings after the other in-flow
/// siblings and the gaps, the share taffy hands out with `min-size: 0`
/// (equal `100%` bases shrink equally). Any other sizing keeps its resolved
/// width, and the result never exceeds the resolved width.
pub(super) fn allotted_width(chain: &[&Value], rects: &HashMap<String, Rect>) -> Option<f64> {
    let (node, parents) = chain.split_last()?;
    let resolved = rects.get(node.get("id")?.as_str()?)?.w;
    if !resolved.is_finite() {
        return None;
    }
    let fill = node.get("width").and_then(Value::as_str) == Some("fill_container");
    let Some(parent) = parents.last().filter(|_| fill) else {
        return Some(resolved);
    };
    let Some(parent_width) = allotted_width(parents, rects) else {
        return Some(resolved);
    };
    let inner = parent_width - horizontal_padding(parent);
    let share = if layout_str(parent) == Some("horizontal") {
        row_share(parent, inner, rects)
    } else {
        inner
    };
    Some(resolved.min(share.max(0.0)))
}

/// One fill child's share of a horizontal row's inner width.
fn row_share(row: &Value, inner: f64, rects: &HashMap<String, Rect>) -> f64 {
    let in_flow: Vec<&Value> = children(row)
        .iter()
        .filter(|c| !has_explicit_position(c) && c.get("visible") != Some(&Value::Bool(false)))
        .collect();
    let gap = match row.get("gap") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    };
    let mut taken = gap * in_flow.len().saturating_sub(1) as f64;
    let mut fills = 0usize;
    for sibling in &in_flow {
        if sibling.get("width").and_then(Value::as_str) == Some("fill_container") {
            fills += 1;
        } else {
            taken += sibling
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| rects.get(id))
                .map(|r| r.w)
                .filter(|w| w.is_finite())
                .unwrap_or(0.0);
        }
    }
    (inner - taken) / fills.max(1) as f64
}

/// jian lays a child with an authored `x` / `y` out of flow.
fn has_explicit_position(v: &Value) -> bool {
    v.get("x").is_some_and(|x| !x.is_null()) || v.get("y").is_some_and(|y| !y.is_null())
}
