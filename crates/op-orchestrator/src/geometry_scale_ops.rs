//! Table-overflow scaling: the scale-op collector and the per-table overflow
//! detector it is gated on.
//!
//! **Slack, not proportion.** The collector used to multiply every fixed
//! column and gap by one factor. The geometry loop re-runs it each round, and
//! nothing stopped it cutting a fixed number cell BELOW its own text: once
//! there the text still overflowed, the row still "needed" room, and the next
//! round cut again (measured, GLM-5.3-Flash `arena-m03`: a phone holdings
//! table's `12,480.55` cell went 64 → 49 → 20 and its gap 8 → 6 → 3, the
//! numbers spilled across their neighbours and the change badge was clipped).
//!
//! Each fixed cell now has a FLOOR — the measured single-line width of its
//! content plus its own horizontal padding (a cell whose content can't be
//! proven keeps its width) — and each gap floors at [`MIN_GAP`]. The deficit
//! is taken from the slack above those floors, in proportion to it, with the
//! same edits per column index on every row (header included) so columns
//! stay aligned. A cell authored narrower than its own content is raised to
//! its floor in the same edit, as long as every row's fixed columns still fit
//! its width. When the slack can't cover the deficit the cells land on their
//! floors and the collector stops there; a table already at its floors
//! yields no commands, so the loop converges and the residual is left to the
//! overflow diagnostic and the starved-column pass.

use super::geometry_starved_column::{measure_texts, Measured, MIN_GAP};
use super::*;

pub(super) fn collect_scale_ops(
    state: &EditorState,
    v: &Value,
    rects: &HashMap<String, Rect>,
    ops: &mut Vec<EditorCommand>,
) {
    ops.extend(table_scale_plan(state, v, rects));
    for c in children(v) {
        collect_scale_ops(state, c, rects, ops);
    }
}

/// The width / gap commands that bring `v`'s overflowing table rows back
/// inside their resolved width without cutting any fixed cell below its
/// content floor. Empty when `v` is not an overflowing table or has no slack
/// left to give.
pub(super) fn table_scale_plan(
    state: &EditorState,
    v: &Value,
    rects: &HashMap<String, Rect>,
) -> Vec<EditorCommand> {
    let mut ops = Vec::new();
    if table_overflow_scale(v, rects).is_none() {
        return ops;
    }
    let rows = table_rows(v);
    let floors = column_floors(state, &rows);
    // A cell authored narrower than its own content is raised to its floor
    // too — unless the raises would push a row's fixed columns past its
    // width, where the spill would just move to the row.
    let raised = Plan::new(&rows, &floors, rects, true);
    let plan = if raised.fits(&rows, &floors, rects) {
        raised
    } else {
        Plan::new(&rows, &floors, rects, false)
    };

    for row in rows {
        for (i, cell) in children(row).iter().enumerate() {
            let (Some(w), Some(id)) = (fixed_width(cell), cell.get("id").and_then(Value::as_str))
            else {
                continue;
            };
            let target = plan.width(w, floors.get(i).copied().flatten());
            if target != w.round() {
                ops.push(EditorCommand::UpdateNode {
                    node_id: NodeId::new(id.to_string()),
                    x: None,
                    y: None,
                    width: Some(target as i32),
                    height: None,
                    name: None,
                    fill_hex: None,
                    page_id: None,
                });
            }
        }
        let gap = num(row, "gap");
        let target = plan.gap(gap);
        if let (true, Some(id)) = (target < gap, row.get("id").and_then(Value::as_str)) {
            ops.push(EditorCommand::SetNodeLayoutProp {
                node_id: NodeId::new(id.to_string()),
                property: "gap".to_string(),
                value: LayoutPropValue::Number(target),
            });
        }
    }
    ops
}

/// One table's edit: the share of every slack given up (the worst row
/// decides, so each column index keeps one width) and whether cells below
/// their floor are raised to it.
struct Plan {
    share: f64,
    raise: bool,
}

impl Plan {
    fn new(
        rows: &[&Value],
        floors: &[Option<f64>],
        rects: &HashMap<String, Rect>,
        raise: bool,
    ) -> Self {
        let mut share = 0.0_f64;
        for row in rows {
            let Some(b) = row_budget(row, rects) else {
                continue;
            };
            let cells = children(row);
            let gap = num(row, "gap");
            let mut slack = (gap - MIN_GAP).max(0.0) * (cells.len() - 1) as f64;
            let mut raised = 0.0;
            for (i, c) in cells.iter().enumerate() {
                let (Some(w), Some(floor)) = (fixed_width(c), floors.get(i).copied().flatten())
                else {
                    continue;
                };
                slack += (w - floor).max(0.0);
                if raise {
                    raised += (floor - w).max(0.0);
                }
            }
            let deficit = b.scalable + raised - b.fixed_budget;
            if deficit > 0.0 && slack >= 1.0 {
                share = share.max((deficit / slack).min(1.0));
            }
        }
        Self { share, raise }
    }

    /// New width of a fixed cell authored `w` wide with content floor `floor`.
    fn width(&self, w: f64, floor: Option<f64>) -> f64 {
        match floor {
            None => w.round(),
            Some(f) if w < f => {
                if self.raise {
                    f
                } else {
                    w.round()
                }
            }
            Some(f) => (w - (w - f) * self.share).round().max(f),
        }
    }

    fn gap(&self, gap: f64) -> f64 {
        if gap > MIN_GAP {
            (gap - (gap - MIN_GAP) * self.share).round().max(MIN_GAP)
        } else {
            gap
        }
    }

    /// Every row's fixed columns + gaps, as edited, stay inside its resolved
    /// inner width (its flex columns may still shrink to make room).
    fn fits(&self, rows: &[&Value], floors: &[Option<f64>], rects: &HashMap<String, Rect>) -> bool {
        rows.iter().all(|row| {
            let Some(inner) = row
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| rects.get(id))
                .map(|r| r.w - horizontal_padding(row))
            else {
                return true;
            };
            let cells = children(row);
            let fixed: f64 = cells
                .iter()
                .enumerate()
                .filter_map(|(i, c)| Some(self.width(fixed_width(c)?, floors.get(i).copied()?)))
                .sum();
            fixed + self.gap(num(row, "gap")) * (cells.len() - 1) as f64 <= inner + 0.5
        })
    }
}

/// One row's overflow arithmetic, shared by the detector and the collector.
struct RowBudget {
    /// Fixed columns + gaps as authored.
    scalable: f64,
    /// What the fixed columns + gaps may occupy beside the flex floors.
    fixed_budget: f64,
}

/// `None` when the row has no fixed columns, no resolved width, fits, or is
/// UNSALVAGEABLE by scaling: even at `MIN_SCALE` the fixed budget can't fit
/// beside the flex floors (a 6-column table crammed into a half-width pane —
/// its five text-bearing fill columns alone need more than the row offers).
/// Leave the row alone and let the too-many-columns diagnostic speak.
fn row_budget(row: &Value, rects: &HashMap<String, Rect>) -> Option<RowBudget> {
    let cells = children(row);
    let n_gaps = (cells.len() - 1) as f64;
    let gap = num(row, "gap");
    let mut fixed_sum = 0.0;
    let mut flex_floor = 0.0;
    for cell in cells {
        match fixed_width(cell) {
            Some(w) => fixed_sum += w,
            // fill_container / fit_content — reserve room for it.
            None => {
                flex_floor += if bears_text(cell) {
                    MIN_FILL_TEXT_COL
                } else {
                    MIN_FILL_COL
                }
            }
        }
    }
    if fixed_sum <= 0.0 {
        return None; // all-flex row can't overflow via fixed widths
    }
    let row_id = row.get("id").and_then(Value::as_str)?;
    let row_w = rects.get(row_id).map(|r| r.w - horizontal_padding(row))?;
    if row_w <= 1.0 {
        return None;
    }
    // Minimum width the row NEEDS: fixed columns + gaps + the flex floors.
    // If that already fits the resolved inner width, this row is fine.
    if fixed_sum + gap * n_gaps + flex_floor <= row_w + OVERFLOW_EPS {
        return None;
    }
    let fixed_budget = (row_w - flex_floor) * FIT_MARGIN;
    let scalable = fixed_sum + gap * n_gaps;
    if scalable <= 0.0 || fixed_budget / scalable < MIN_SCALE {
        return None;
    }
    Some(RowBudget {
        scalable,
        fixed_budget,
    })
}

/// If `v` is a table-shaped container (a repeated contiguous run of ≥2
/// horizontal rows with ≥3 cells and matching width modes — the STRUCTURE is
/// the gate, not the name; "VIP Client List" shipped a starved 6px email column
/// because a name gate only trusted `table`-named frames) whose fixed columns
/// crowd out the rows' RESOLVED inner width, return the scale factor (< 1.0)
/// that would fit its fixed columns + gap. Each row is measured against its
/// own inner width (rect minus padding) and each text-bearing flex column
/// reserves a readable floor; the WORST row decides, so uneven header/data
/// column sets can't hide the deficit. `None` when the shape isn't a table or
/// everything fits. The collector does not apply this factor — it only takes
/// slack above each cell's content floor (see the module docs).
pub(super) fn table_overflow_scale(v: &Value, rects: &HashMap<String, Rect>) -> Option<f64> {
    let mut worst: Option<f64> = None;
    for row in table_rows(v) {
        let Some(b) = row_budget(row, rects) else {
            continue;
        };
        let scale = (b.fixed_budget / b.scalable).clamp(MIN_SCALE, 1.0);
        if scale < 1.0 - 0.001 {
            worst = Some(worst.map_or(scale, |w: f64| w.min(scale)));
        }
    }
    worst
}

// ── Content floors ──

/// Per column index: the widest content floor any row's fixed cell has there
/// (`None` = some row's cell at that index can't be proven, so the column
/// keeps its width). Flex columns get `None` — they are not edited.
fn column_floors(state: &EditorState, rows: &[&Value]) -> Vec<Option<f64>> {
    let mut texts = Vec::new();
    for row in rows {
        for cell in children(row).iter().filter(|c| fixed_width(c).is_some()) {
            collect_texts(cell, &mut texts);
        }
    }
    let measured = if texts.is_empty() {
        Measured::default()
    } else {
        measure_texts(state, &texts)
    };
    let n = rows.first().map_or(0, |r| children(r).len());
    (0..n)
        .map(|i| {
            let mut floor = 0.0_f64;
            for row in rows {
                let cell = children(row).get(i)?;
                fixed_width(cell)?;
                floor = floor.max(content_floor(cell, true, &measured)?);
            }
            Some(floor.ceil())
        })
        .collect()
}

fn collect_texts<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    if v.get("type").and_then(Value::as_str) == Some("text") {
        out.push(v);
        return;
    }
    for c in children(v) {
        collect_texts(c, out);
    }
}

/// Narrowest width `v` can take without its content spilling: text at its
/// measured single-line width, a nested fixed-width node at that width, a
/// flex container at its padding plus its flow children (summed with gaps
/// when horizontal, widest when stacked). `None` when that can't be proven
/// (an unmeasured text, a free-positioned layer, an unknown leaf), so the
/// owning cell keeps its authored width.
fn content_floor(v: &Value, is_cell: bool, measured: &Measured) -> Option<f64> {
    let kind = v.get("type").and_then(Value::as_str).unwrap_or("");
    if kind == "text" {
        if !is_cell {
            if let Some(w) = fixed_width(v) {
                return Some(w);
            }
        }
        let id = v.get("id").and_then(Value::as_str)?;
        return measured.natural.get(id).copied();
    }
    if !is_cell {
        if let Some(w) = fixed_width(v) {
            return Some(w);
        }
    }
    if !matches!(kind, "frame" | "group") {
        return None;
    }
    let kids = children(v);
    let padding = horizontal_padding(v);
    if kids.is_empty() {
        return Some(padding);
    }
    let horizontal = match layout_str(v) {
        Some("horizontal") => true,
        Some("vertical") => false,
        // Free-positioned children: their offsets decide the extent.
        _ => return None,
    };
    let mut total = 0.0_f64;
    let mut flow = 0usize;
    for c in kids {
        if has_authored_position(c) {
            return None;
        }
        let w = content_floor(c, false, measured)?;
        total = if horizontal { total + w } else { total.max(w) };
        flow += 1;
    }
    if horizontal && flow > 1 {
        total += num(v, "gap") * (flow - 1) as f64;
    }
    Some(total + padding)
}
