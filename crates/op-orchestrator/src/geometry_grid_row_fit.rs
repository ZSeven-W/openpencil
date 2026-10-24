//! Uniform grid rows — a calendar week, a habit-tracker strip, a keypad —
//! whose identical cells and gap do not fit the row.
//!
//! Measured on a DeepSeek travel booking page: each calendar week was seven
//! identical day cells, six of them 42px and the last `fill_container`, in a
//! 303px row with an 8px gap. Six cells and six gaps already took 300px, so
//! the last cell resolved to 3px while its band, circle and number were
//! authored for 42px: the whole Saturday column painted outside the card and
//! the other days drifted away from the weekday header, which the same model
//! had laid out as seven 42px cells with no gap.
//!
//! The cells are peers by construction (one child-type sequence, one declared
//! width), so the collapsed cell is meant to be the same width as the rest,
//! and the only free variable is the gap. Repair: give the fill cells the
//! shared width and shrink the gap until the row fits. When even a zero gap
//! cannot seat the cells, the design needs fewer or narrower columns — an
//! intent call left alone.

use super::*;

/// A week, a keypad row: fewer cells are ordinary rows of controls.
const MIN_GRID_CELLS: usize = 5;
/// A fill cell must miss the shared width by more than this to count as
/// collapsed; keeps rounding out.
const COLLAPSE_EPS: f64 = 4.0;

/// The cell's child types as a multiset: the buried-overlay pass reorders an
/// endpoint day's number above its circle in the same round, and that day is
/// still the same cell as its neighbours.
fn signature(v: &Value) -> Vec<&str> {
    let mut types: Vec<&str> = children(v)
        .iter()
        .map(|c| c.get("type").and_then(Value::as_str).unwrap_or(""))
        .collect();
    types.sort_unstable();
    types
}

fn is_fill(v: &Value) -> bool {
    v.get("width").and_then(Value::as_str) == Some("fill_container")
}

/// The shared declared width of a row of identical cells, when the row is
/// one: at least `MIN_GRID_CELLS` frames with one child-type sequence, every
/// width either that one number or `fill_container`.
fn grid_cell_width(v: &Value) -> Option<f64> {
    if layout_str(v) != Some("horizontal") {
        return None;
    }
    let cells = children(v);
    if cells.len() < MIN_GRID_CELLS
        || !cells.iter().all(|c| {
            matches!(
                c.get("type").and_then(Value::as_str),
                Some("frame" | "group")
            ) && !children(c).is_empty()
        })
    {
        return None;
    }
    let first = signature(&cells[0]);
    if !cells.iter().all(|c| signature(c) == first) {
        return None;
    }
    let mut width = None;
    for cell in cells {
        match cell.get("width").and_then(Value::as_f64) {
            Some(w) if width.is_none_or(|shared: f64| (shared - w).abs() < 0.5) => width = Some(w),
            Some(_) => return None,
            None if is_fill(cell) => {}
            None => return None,
        }
    }
    width.filter(|w| *w > 0.0)
}

fn texts_centered(v: &Value) -> bool {
    children(v).iter().all(|c| {
        (c.get("type").and_then(Value::as_str) != Some("text")
            || c.get("textAlign").and_then(Value::as_str) == Some("center"))
            && texts_centered(c)
    })
}

/// Identical cells whose text is centred: the cells touching is the grid's
/// pitch, not two data columns reading as one word, because each glyph sits
/// mid-cell with the cell's own slack on both sides.
pub(super) fn is_centered_grid_row(v: &Value) -> bool {
    grid_cell_width(v).is_some() && children(v).iter().all(texts_centered)
}

pub(super) fn collect_grid_row_fit_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    if let (Some(width), Some(row)) = (
        grid_cell_width(v),
        v.get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id)),
    ) {
        let cells = children(v);
        let n = cells.len() as f64;
        let gap = num(v, "gap").max(0.0);
        let inner = row.w - horizontal_padding(v);
        let collapsed: Vec<&str> = cells
            .iter()
            .filter(|c| is_fill(c))
            .filter_map(|c| c.get("id").and_then(Value::as_str))
            .filter(|id| rects.get(*id).is_some_and(|r| r.w < width - COLLAPSE_EPS))
            .collect();
        let overflowing = n * width + (n - 1.0) * gap > inner + 0.5;
        if (!collapsed.is_empty() || overflowing) && n * width <= inner + 0.5 {
            for id in collapsed {
                cmds.push(EditorCommand::UpdateNode {
                    node_id: NodeId::new(id.to_string()),
                    x: None,
                    y: None,
                    width: Some(width.round() as i32),
                    height: None,
                    name: None,
                    fill_hex: None,
                    page_id: None,
                });
            }
            let fitted = ((inner - n * width) / (n - 1.0)).floor().max(0.0);
            if fitted < gap {
                if let Some(id) = v.get("id").and_then(Value::as_str) {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_string()),
                        property: "gap".to_string(),
                        value: LayoutPropValue::Number(fitted),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_grid_row_fit_fixes(c, rects, cmds);
    }
}

#[cfg(test)]
#[path = "geometry_grid_row_fit_tests.rs"]
mod tests;
