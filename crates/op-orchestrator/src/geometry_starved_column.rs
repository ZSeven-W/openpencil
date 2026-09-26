//! Starved text-column repair — a horizontal row whose `fill_container` TEXT
//! column the fixed siblings squeezed below a readable width.
//!
//! Measured (GLM-5.3-Flash `arena-m03`): a 375-wide phone holdings table with
//! rows `{gap: 24, padding: [12, 16]}` holding a `fill_container` name cell
//! ("易方达蓝筹精选" + a tag), two fixed number texts (64 / 78) and a 56px
//! pill. The name column resolved to ~25px — fewer than two CJK glyphs at
//! 14px — so its text wrapped one glyph per line, every row grew a tower and
//! the fixed-height table card clipped to one and a half rows. Nothing
//! OVERFLOWS in that failure, so the overflow repairs are blind; the spill
//! diagnostics only report the column once it falls under 24px, and
//! `geometry_starved_row` covers the opposite case (a row of rigid columns).
//!
//! Contract tier: a text column shredded into a vertical tower is never an
//! authored intent.
//!
//! **Readable floor.** A text needs `min(natural, max(3.5 × fontSize,
//! longest unbreakable Latin run))`: three and a half glyphs per line for CJK
//! (which wraps per glyph), never a mid-word break for Latin, and never more
//! than the text's own single-line width. Both widths are MEASURED with the
//! real jian layout on a scratch document — no advance-width guesses.
//!
//! **Target.** The floor only decides WHETHER a column is starved. Once it
//! is, the fix aims for the widest natural single-line width among the
//! column's texts across the table's rows: stopping at the floor still split
//! a four-glyph name 3+1 ("贵州茅/台"), which reads worse than the model's
//! intent. What cannot be reached is not forced — the repair takes all the
//! width the two steps below can give, which covers the floor whenever the
//! floor is reachable at all.
//!
//! **Fix.** The deficit is recovered from the row's own spacing before any
//! content is touched:
//! 1. reduce `gap` (never below [`MIN_GAP`]) by just enough;
//! 2. if still short, shrink the numeric-width TEXT columns toward their
//!    measured natural single-line width (never below it), in proportion to
//!    their slack. Pills, icons, images and frames keep their widths.
//!
//! When both steps together fall short the partial fix still lands and the
//! model-facing starvation diagnostic stays in place.
//!
//! **Tables.** Sibling rows of the same table (same parent, same flow child
//! count, same gap, same per-column width pattern — header row included)
//! receive the SAME gap and width edits, sized from the worst row, so the
//! columns stay aligned.

use super::*;

/// Gaps are never reduced below this — tighter columns read as one run.
pub(super) const MIN_GAP: f64 = 8.0;
/// Glyphs per line a text column must hold (CJK wraps per glyph).
const READABLE_EMS: f64 = 3.5;
/// A row this narrow is a chip / control row; its columns are not prose.
const MIN_ROW_W: f64 = 200.0;
/// Deficits within this many px are rounding, not starvation.
const DEFICIT_EPS: f64 = 1.0;
/// Jian measures an omitted font size as 14px.
const DEFAULT_FONT_SIZE: f64 = 14.0;
/// Id prefixes of the scratch measurement copies.
const NATURAL_PREFIX: &str = "__starved_col_n_";
const WORD_PREFIX: &str = "__starved_col_w_";

/// Sibling rows that must move together: same parent, same flow child count,
/// same gap, same width pattern.
struct RowGroup<'a> {
    rows: Vec<&'a Value>,
    gap: f64,
}

/// Collect the gap / width commands that give each starved text column its
/// readable floor. Runs from the PARENT's vantage point so the table's other
/// rows are in view.
pub(super) fn collect_starved_text_column_fixes(
    state: &EditorState,
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let mut groups = Vec::new();
    collect_candidate_groups(state, v, rects, &mut groups);
    if groups.is_empty() {
        return;
    }
    let texts = measured_texts(&groups);
    let measured = measure_texts(state, &texts);
    for group in &groups {
        fix_group(group, rects, &measured, cmds);
    }
}

// ── Grouping + cheap candidacy ──

fn flow_children(row: &Value) -> Vec<&Value> {
    children(row)
        .iter()
        .filter(|c| !has_authored_position(c))
        .collect()
}

fn width_pattern(row: &Value) -> Vec<String> {
    flow_children(row)
        .iter()
        .map(|c| match fixed_width(c) {
            Some(w) => format!("{}", w.round()),
            None => c
                .get("width")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
        .collect()
}

fn is_row(v: &Value) -> bool {
    layout_str(v) == Some("horizontal")
        && v.get("clipContent").and_then(Value::as_bool) != Some(true)
        && !crate::cleanup::is_status_bar_from_json(v)
        && flow_children(v).len() >= 2
}

fn is_fill_text_column(c: &Value) -> bool {
    c.get("width").and_then(Value::as_str) == Some("fill_container") && bears_text(c)
}

fn collect_candidate_groups<'a>(
    state: &EditorState,
    v: &'a Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<RowGroup<'a>>,
) {
    // The table column scaler owns rows whose FIXED columns overflow while it
    // still has slack to take; two passes editing the same gaps in one round
    // would fight. Once its cells sit on their content floors it stops, and
    // a still-starved name column is this pass's to reclaim.
    let scaler_owns = table_overflow_scale(v, rects).is_some()
        && !super::geometry_scale_ops::table_scale_plan(state, v, rects).is_empty();
    if !scaler_owns && !crate::cleanup::is_status_bar_from_json(v) {
        let mut groups: Vec<(Vec<String>, RowGroup<'a>)> = Vec::new();
        for c in children(v).iter().filter(|c| is_row(c)) {
            let key = width_pattern(c);
            let gap = num(c, "gap");
            // The width pattern carries the flow child count.
            match groups
                .iter_mut()
                .find(|(k, g)| *k == key && (g.gap - gap).abs() < 0.5)
            {
                Some((_, g)) => g.rows.push(c),
                None => groups.push((key, RowGroup { rows: vec![c], gap })),
            }
        }
        for (_, group) in groups {
            if group.rows.iter().any(|row| row_may_be_starved(row, rects)) {
                out.push(group);
            }
        }
    }
    for c in children(v) {
        collect_candidate_groups(state, c, rects, out);
    }
}

/// Cheap upper-bound check before any scratch layout: some wrap-capable text
/// inside a fill column resolved narrower than the most its floor could be.
fn row_may_be_starved(row: &Value, rects: &HashMap<String, Rect>) -> bool {
    let Some(rw) = rect_of(row, rects).map(|r| r.w) else {
        return false;
    };
    if rw < MIN_ROW_W {
        return false;
    }
    flow_children(row)
        .into_iter()
        .filter(|c| is_fill_text_column(c))
        .flat_map(column_texts)
        .any(|t| {
            let fs = font_size(t);
            let word_bound = longest_latin_run(t).chars().count() as f64 * fs;
            let bound = (READABLE_EMS * fs).max(word_bound);
            rect_of(t, rects).is_some_and(|r| r.w + DEFICIT_EPS < bound)
        })
}

/// Texts whose width the column dictates: the walk stops at any descendant
/// with its own numeric width, and numeric-width texts are skipped.
fn column_texts(column: &Value) -> Vec<&Value> {
    fn walk<'a>(v: &'a Value, is_root: bool, out: &mut Vec<&'a Value>) {
        if !is_root && fixed_width(v).is_some() {
            return;
        }
        if v.get("type").and_then(Value::as_str) == Some("text") {
            out.push(v);
            return;
        }
        for c in children(v) {
            walk(c, false, out);
        }
    }
    let mut out = Vec::new();
    walk(column, true, &mut out);
    out
}

/// Numeric-width text columns, by flow index, that every row of the group
/// agrees on — the only columns step 2 may narrow.
fn shrinkable_columns(group: &RowGroup<'_>) -> Vec<usize> {
    let n = flow_children(group.rows[0]).len();
    (0..n)
        .filter(|&i| {
            group.rows.iter().all(|row| {
                flow_children(row).get(i).is_some_and(|c| {
                    c.get("type").and_then(Value::as_str) == Some("text")
                        && fixed_width(c).is_some()
                })
            })
        })
        .collect()
}

fn rect_of<'r>(v: &Value, rects: &'r HashMap<String, Rect>) -> Option<&'r Rect> {
    v.get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
}

fn font_size(t: &Value) -> f64 {
    let fs = num(t, "fontSize");
    if fs.is_finite() && fs > 0.0 {
        fs
    } else {
        DEFAULT_FONT_SIZE
    }
}

/// The longest run of non-space ASCII characters — a word the line breaker
/// may not split. CJK characters break anywhere and end a run.
fn longest_latin_run(t: &Value) -> String {
    let content = t.get("content").and_then(Value::as_str).unwrap_or("");
    content
        .split(|c: char| c.is_whitespace() || !c.is_ascii())
        .max_by_key(|run| run.chars().count())
        .unwrap_or("")
        .to_string()
}

// ── Measurement ──

/// Every text the fix needs measured: the fill columns' texts (natural +
/// longest word) and the shrinkable columns' texts (natural).
fn measured_texts<'a>(groups: &[RowGroup<'a>]) -> Vec<&'a Value> {
    let mut out: Vec<&'a Value> = Vec::new();
    for group in groups {
        let shrinkable = shrinkable_columns(group);
        for row in &group.rows {
            let kids = flow_children(row);
            for (i, c) in kids.iter().enumerate() {
                if is_fill_text_column(c) {
                    out.extend(column_texts(c));
                } else if shrinkable.contains(&i) {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Natural single-line width (`n`) and longest-word width (`w`) per text id.
#[derive(Default)]
pub(super) struct Measured {
    pub(super) natural: HashMap<String, f64>,
    word: HashMap<String, f64>,
}

/// Lay copies of `texts` out as hugging single-line text in an unconstrained
/// scratch document (same variables / themes) and read their widths back
/// from the real jian layout.
pub(super) fn measure_texts(state: &EditorState, texts: &[&Value]) -> Measured {
    let mut kids = Vec::new();
    for t in texts {
        let Some(id) = t.get("id").and_then(Value::as_str) else {
            continue;
        };
        kids.push(scratch_text(t, &format!("{NATURAL_PREFIX}{id}"), None));
        let word = longest_latin_run(t);
        if word.chars().count() >= 2 {
            kids.push(scratch_text(t, &format!("{WORD_PREFIX}{id}"), Some(&word)));
        }
    }
    let mut measured = Measured::default();
    let doc = serde_json::json!({
        "version": state.doc.version,
        "variables": state.doc.variables,
        "themes": state.doc.themes,
        "children": [{
            "type": "frame", "id": "__starved_col_root", "width": 100000,
            "height": "fit_content", "layout": "vertical", "alignItems": "start",
            "children": kids,
        }],
    });
    let Ok(doc) = serde_json::from_value::<jian_ops_schema::PenDocument>(doc) else {
        return measured;
    };
    let scene =
        op_pen_loader::pen_document_to_layout_scene(&doc, &std::collections::BTreeMap::new(), 0);
    let mut rects = HashMap::new();
    if let Some(page) = scene.active_page() {
        collect_rects(&page.children, &mut rects);
    }
    for (id, r) in rects {
        if !r.w.is_finite() || r.w <= 0.0 {
            continue;
        }
        if let Some(orig) = id.strip_prefix(NATURAL_PREFIX) {
            measured.natural.insert(orig.to_string(), r.w);
        } else if let Some(orig) = id.strip_prefix(WORD_PREFIX) {
            measured.word.insert(orig.to_string(), r.w);
        }
    }
    measured
}

fn scratch_text(t: &Value, id: &str, content: Option<&str>) -> Value {
    let mut copy = t.clone();
    if let Some(obj) = copy.as_object_mut() {
        for key in ["x", "y", "height", "animations", "children"] {
            obj.remove(key);
        }
        obj.insert("id".into(), Value::String(id.to_string()));
        obj.insert("width".into(), Value::String("fit_content".into()));
        obj.insert("textGrowth".into(), Value::String("auto".into()));
        if let Some(content) = content {
            obj.insert("content".into(), Value::String(content.to_string()));
        }
    }
    copy
}

// ── Fix ──

/// Width a text must reach to stay readable, per the module rule.
fn readable_floor(t: &Value, measured: &Measured) -> Option<f64> {
    let id = t.get("id").and_then(Value::as_str)?;
    let natural = *measured.natural.get(id)?;
    let word = measured.word.get(id).copied().unwrap_or(0.0);
    Some(natural.min((READABLE_EMS * font_size(t)).max(word)))
}

/// Natural single-line width of a text: the width at which it stops wrapping,
/// and the repair's TARGET once its column is starved.
fn natural_width(t: &Value, measured: &Measured) -> Option<f64> {
    let id = t.get("id").and_then(Value::as_str)?;
    measured.natural.get(id).copied()
}

/// Which width a text is measured against: the readable floor decides
/// WHETHER a column is starved, the natural width sizes the fix.
type Goal = fn(&Value, &Measured) -> Option<f64>;

/// Width this row's fill columns must gain, in total, so every text inside
/// them reaches `goal`. Free space splits evenly between the row's fill
/// children, so a column's deficit costs that many times over.
fn row_deficit(row: &Value, rects: &HashMap<String, Rect>, measured: &Measured, goal: Goal) -> f64 {
    if rect_of(row, rects).is_none_or(|r| r.w < MIN_ROW_W) {
        return 0.0;
    }
    let kids = flow_children(row);
    let fill_count = kids
        .iter()
        .filter(|c| c.get("width").and_then(Value::as_str) == Some("fill_container"))
        .count()
        .max(1) as f64;
    let worst = kids
        .iter()
        .filter(|c| is_fill_text_column(c))
        .flat_map(|c| column_texts(c))
        .filter_map(|t| Some(goal(t, measured)? - rect_of(t, rects)?.w))
        .fold(0.0_f64, f64::max);
    worst * fill_count
}

/// The worst row's deficit — every row of the group gets the same edits.
fn group_deficit(
    group: &RowGroup<'_>,
    rects: &HashMap<String, Rect>,
    measured: &Measured,
    goal: Goal,
) -> f64 {
    group
        .rows
        .iter()
        .map(|row| row_deficit(row, rects, measured, goal))
        .fold(0.0_f64, f64::max)
}

fn fix_group(
    group: &RowGroup<'_>,
    rects: &HashMap<String, Rect>,
    measured: &Measured,
    cmds: &mut Vec<EditorCommand>,
) {
    // Trigger on the readable floor; once starved, aim for no wrap at all
    // and take whatever part of that the two steps below can reach.
    if group_deficit(group, rects, measured, readable_floor) <= DEFICIT_EPS {
        return;
    }
    let deficit = group_deficit(group, rects, measured, natural_width);
    if deficit <= DEFICIT_EPS {
        return;
    }
    let gaps = (flow_children(group.rows[0]).len() - 1) as f64;

    // Step 1: the row's own spacing.
    let mut new_gap = group.gap;
    if group.gap > MIN_GAP {
        new_gap = (group.gap - (deficit / gaps).ceil()).max(MIN_GAP);
    }
    let remaining = deficit - (group.gap - new_gap) * gaps;

    // Step 2: numeric text columns toward their natural width.
    let mut widths: Vec<(usize, f64)> = Vec::new();
    if remaining > DEFICIT_EPS {
        let columns: Vec<(usize, f64, f64)> = shrinkable_columns(group)
            .into_iter()
            .filter_map(|i| column_slack(group, i, measured))
            .filter(|&(_, width, natural)| width > natural)
            .collect();
        let total: f64 = columns.iter().map(|&(_, w, n)| w - n).sum();
        if total > 0.0 {
            let take = remaining.min(total);
            for (i, width, natural) in columns {
                let cut = (width - natural) * take / total;
                let target = (width - cut).floor().max(natural);
                if target < width {
                    widths.push((i, target));
                }
            }
        }
    }

    for row in &group.rows {
        let Some(row_id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        if new_gap < group.gap {
            cmds.push(EditorCommand::SetNodeLayoutProp {
                node_id: NodeId::new(row_id.to_string()),
                property: "gap".to_string(),
                value: LayoutPropValue::Number(new_gap),
            });
        }
        let kids = flow_children(row);
        for &(i, width) in &widths {
            if let Some(id) = kids
                .get(i)
                .and_then(|c| c.get("id"))
                .and_then(Value::as_str)
            {
                cmds.push(EditorCommand::UpdateNode {
                    node_id: NodeId::new(id.to_string()),
                    x: None,
                    y: None,
                    width: Some(width as i32),
                    height: None,
                    name: None,
                    fill_hex: None,
                    page_id: None,
                });
            }
        }
    }
}

/// `(index, authored width, natural width)` of a shrinkable column: the
/// widest measured text across every row decides its natural width.
fn column_slack(group: &RowGroup<'_>, i: usize, measured: &Measured) -> Option<(usize, f64, f64)> {
    let mut width = f64::INFINITY;
    let mut natural = 0.0_f64;
    for row in &group.rows {
        let kids = flow_children(row);
        let cell = kids.get(i)?;
        width = width.min(fixed_width(cell)?);
        let id = cell.get("id").and_then(Value::as_str)?;
        natural = natural.max(*measured.natural.get(id)?);
    }
    Some((i, width, natural.ceil()))
}

#[cfg(test)]
#[path = "geometry_starved_column_tests.rs"]
mod tests;
