//! Width budget for the column gap `ensure_table_column_gap` injects.
//!
//! The pass used to hand every gap-less table row a flat 24px gap. On a phone
//! that is what starved the GLM-5.3-Flash `arena-m03` holdings table: 3 × 24px
//! of injected gap beside 198px of fixed columns left the `fill_container`
//! name column 25px, and every name shredded one glyph per line. The model had
//! authored no gap at all; the starvation was ours.
//!
//! The least invasive correction keeps the 24px default and only LOWERS it
//! when the row's width is provable from the tree and the default would leave
//! a text-bearing fill column narrower than its text's estimated single-line
//! width. The gap then becomes the largest value in `[8, 24]` that seats that
//! text, or 8 when nothing in the range does. Rows whose width cannot be
//! proven (a `fit_content` ancestor, a keyword-sized sibling, expression
//! padding) and rows without a text fill column keep the default, so a desktop
//! table is spaced exactly as before. The gap is chosen once per table — the
//! tightest row decides — so the columns stay aligned.
//!
//! This is a tree-shape pass and runs before any layout, so the text width is
//! an advance-width ESTIMATE; the geometry loop's measured starved-column
//! repair (`geometry_starved_column`) remains the exact backstop.

use serde_json::Value;

/// Smallest gap the budget may choose — tighter columns read as one run.
const MIN_TABLE_GAP: f64 = 8.0;
/// Jian measures an omitted font size as 14px.
const DEFAULT_FONT_SIZE: f64 = 14.0;

/// Width a child resolves to inside a parent whose content box is
/// `parent_inner`: numeric widths are literal, `fill_container` inherits the
/// content box in a column layout, anything else is unknown.
pub(super) fn child_width(
    child: &Value,
    parent_inner: Option<f64>,
    parent_is_row: bool,
) -> Option<f64> {
    match child.get("width") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) if s == "fill_container" && !parent_is_row => parent_inner,
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// `width` minus the node's numeric horizontal padding.
pub(super) fn inner_width(v: &Value, width: Option<f64>) -> Option<f64> {
    Some(width? - horizontal_padding(v)?)
}

/// Numeric left + right padding; `None` for expression padding.
fn horizontal_padding(v: &Value) -> Option<f64> {
    match v.get("padding") {
        None | Some(Value::Null) => Some(0.0),
        Some(Value::Number(n)) => Some(n.as_f64()? * 2.0),
        Some(Value::Array(a)) => {
            let n: Vec<f64> = a.iter().map(Value::as_f64).collect::<Option<_>>()?;
            match n.as_slice() {
                [p] => Some(p * 2.0),
                [_, h] | [_, h, _] => Some(h * 2.0),
                [_, r, _, l] => Some(r + l),
                _ => None,
            }
        }
        _ => None,
    }
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn texts(v: &Value) -> Vec<&Value> {
    if v.get("type").and_then(Value::as_str) == Some("text") {
        return vec![v];
    }
    children(v).iter().flat_map(texts).collect()
}

/// Rough advance of one character in ems — enough to tell "four CJK glyphs"
/// from "a sliver", which is all the budget has to decide.
fn char_em(c: char) -> f64 {
    if !c.is_ascii() {
        1.0
    } else if c.is_ascii_digit() {
        0.62
    } else if c.is_ascii_uppercase() {
        0.68
    } else if c.is_ascii_lowercase() {
        0.55
    } else if c == ' ' {
        0.3
    } else {
        0.35
    }
}

/// Estimated single-line width of a text node.
fn text_width(t: &Value) -> f64 {
    let content = t.get("content").and_then(Value::as_str).unwrap_or("");
    let size = t
        .get("fontSize")
        .and_then(Value::as_f64)
        .filter(|s| *s > 0.0)
        .unwrap_or(DEFAULT_FONT_SIZE);
    let spacing = t
        .get("letterSpacing")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    // The longest line decides: an authored break is not a wrap.
    content
        .lines()
        .map(|line| {
            line.chars()
                .map(|c| size * char_em(c) + spacing)
                .sum::<f64>()
        })
        .fold(0.0, f64::max)
}

/// Width a text-bearing fill column needs so its widest text sits on one
/// line: the widest text plus the column's own padding.
fn column_need(column: &Value) -> Option<f64> {
    let widest = texts(column)
        .into_iter()
        .map(text_width)
        .fold(0.0, f64::max);
    (widest > 0.0).then(|| widest + horizontal_padding(column).unwrap_or(0.0))
}

/// The largest gap in `[MIN_TABLE_GAP, default]` that leaves every
/// text-bearing fill column of `row` its estimated single-line width.
/// `None` when the row's width cannot be proven or it has no text fill column
/// — the caller then keeps the default.
pub(super) fn row_gap_budget(row: &Value, row_width: Option<f64>, default: f64) -> Option<f64> {
    let kids = children(row);
    if kids.len() < 2 {
        return None;
    }
    let is_fill = |c: &&Value| c.get("width").and_then(Value::as_str) == Some("fill_container");
    let fills: Vec<&Value> = kids.iter().filter(is_fill).collect();
    let need = fills
        .iter()
        .filter_map(|c| column_need(c))
        .fold(0.0, f64::max);
    if need <= 0.0 {
        return None;
    }
    let mut fixed = 0.0;
    for kid in kids.iter().filter(|c| !is_fill(c)) {
        fixed += child_width(kid, None, true)?;
    }
    let inner = inner_width(row, row_width)?;
    // Free space splits evenly between fill columns.
    let free = inner - fixed - need * fills.len() as f64;
    let gap = (free / (kids.len() - 1) as f64).floor();
    Some(gap.clamp(MIN_TABLE_GAP, default))
}

#[cfg(test)]
#[path = "table_repair_gap_budget_tests.rs"]
mod tests;
