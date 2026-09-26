//! Grow-to-fit for a clipped fixed-height card that truncates a TABLE or LIST.
//!
//! Measured, arena-m03 (glm-5.3-flash): a holdings table card declared
//! `height: 282` with `clipContent: true`, stacking a header and six data rows
//! separated by 1px `row-divider` frames. The content resolved far taller than
//! 282, so the card showed two and a half rows and sliced the last visible row
//! mid-glyph. The small-overshoot grow rules in `geometry_grow_fit_fixes` stay
//! away from big overshoots (a cover image cropped to its slot is intended),
//! but a run of repeated text rows cut by a clip edge is never intended: the
//! rows are the content. Contract repair: the card hugs its content.

use super::*;
use crate::cleanup::is_status_bar_from_json;

/// Slack before a text's bottom counts as cut by the clip edge.
const CUT_TEXT_EPS: f64 = 1.0;
/// Fewest structurally similar siblings that make a table body / list.
const MIN_REPEATED_ROWS: usize = 3;
/// A childless leaf this thin (on its stacking axis) is a row divider.
const DIVIDER_MAX_THICKNESS: f64 = 2.0;

/// Walk the tree and grow every qualifying clipped card (see module docs).
/// The root itself is a screen / page and is never grown; status-bar chrome
/// is skipped whole.
pub(super) fn collect_clipped_rows_grow_fixes(
    root: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    for child in children(root) {
        visit(child, rects, cmds);
    }
}

fn visit(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>) {
    if is_status_bar_from_json(v) {
        return;
    }
    if let Some(cmd) = clipped_rows_fix(v, rects) {
        cmds.push(cmd);
    }
    for child in children(v) {
        visit(child, rects, cmds);
    }
}

/// The height edit for `v` when it is a qualifying clipped card.
fn clipped_rows_fix(v: &Value, rects: &HashMap<String, Rect>) -> Option<EditorCommand> {
    if v.get("clipContent").and_then(Value::as_bool) != Some(true)
        || layout_str(v) != Some("vertical")
        || has_scroller_semantics(v)
    {
        return None;
    }
    let declared = v
        .get("height")
        .and_then(Value::as_f64)
        .filter(|h| *h > 0.0)?;
    let id = v.get("id").and_then(Value::as_str)?;
    let frame = rects.get(id)?;
    let clip_bottom = frame.y + declared;
    // A media card (cover photo / video as a direct child) clips for its
    // media: the crop is authored, and the image passes own its sizing (the
    // oversized-image rule refits it to the card), so a hugging card would
    // collapse a `fill_container` cover. Table / list rows keep thumbnails
    // INSIDE the rows, never as the card's own child.
    if children(v).iter().any(is_media) {
        return None;
    }
    if !has_cut_repeated_row(v, rects, clip_bottom) {
        return None;
    }
    let node_id = NodeId::new(id.to_string());
    // A child that fills the card's height would collapse under a hugging
    // parent (circular sizing); pin the resolved content height instead.
    let fills_height = children(v)
        .iter()
        .any(|c| c.get("height").and_then(Value::as_str) == Some("fill_container"));
    if fills_height {
        let content_bottom = children(v)
            .iter()
            .filter_map(|c| bottom_of(c, rects))
            .fold(f64::MIN, f64::max);
        let padding_bottom = numeric_padding_sides(v)
            .map(|[_, _, bottom, _]| bottom.max(0.0))
            .unwrap_or(0.0);
        let required = content_bottom + padding_bottom - frame.y;
        if required <= declared {
            return None;
        }
        return Some(EditorCommand::UpdateNode {
            node_id,
            x: None,
            y: None,
            width: None,
            height: Some(required.ceil() as i32),
            name: None,
            fill_hex: None,
            page_id: None,
        });
    }
    Some(EditorCommand::SetNodeLayoutProp {
        node_id,
        property: "height".to_string(),
        value: LayoutPropValue::Keyword("fit_content".to_string()),
    })
}

/// Does `v`'s subtree hold a repeated-row run (table body / list) with at
/// least one row whose text the clip edge at `clip_bottom` cuts or hides?
/// Descent stops at nested fixed-height or horizontal clips — rows hidden by
/// THEIR edge are theirs to repair (or an authored scroller), not `v`'s — and
/// at status-bar chrome.
fn has_cut_repeated_row(v: &Value, rects: &HashMap<String, Rect>, clip_bottom: f64) -> bool {
    if repeated_row_run(v)
        .iter()
        .any(|row| text_bottom(row, rects).is_some_and(|b| b > clip_bottom + CUT_TEXT_EPS))
    {
        return true;
    }
    children(v).iter().any(|c| {
        !is_status_bar_from_json(c)
            && !is_nested_clip(c)
            && has_cut_repeated_row(c, rects, clip_bottom)
    })
}

fn is_nested_clip(v: &Value) -> bool {
    v.get("clipContent").and_then(Value::as_bool) == Some(true)
        && (v.get("height").and_then(Value::as_f64).is_some()
            || layout_str(v) == Some("horizontal"))
}

/// The longest contiguous run of structurally similar, text-bearing container
/// rows under a vertically stacked `v`. Unlike [`table_rows`], thin divider
/// leaves between rows (`row-divider` frames, rules) do not break the run, and
/// a list whose rows are not three-column table rows still counts.
pub(super) fn repeated_row_run(v: &Value) -> Vec<&Value> {
    if layout_str(v) == Some("horizontal") {
        return Vec::new();
    }
    let mut best: Vec<&Value> = Vec::new();
    let mut run: Vec<&Value> = Vec::new();
    let mut run_signature: Option<RowSignature> = None;
    for child in children(v) {
        if is_row_divider(child) {
            continue;
        }
        let signature = row_signature(child);
        if signature.is_some() && signature == run_signature {
            run.push(child);
            continue;
        }
        if run.len() > best.len() {
            best = std::mem::take(&mut run);
        } else {
            run.clear();
        }
        if signature.is_some() {
            run.push(child);
        }
        run_signature = signature;
    }
    if run.len() > best.len() {
        best = run;
    }
    if best.len() >= MIN_REPEATED_ROWS {
        best
    } else {
        Vec::new()
    }
}

#[derive(PartialEq)]
struct RowSignature<'a> {
    kind: &'a str,
    layout: Option<&'a str>,
    child_kinds: Vec<&'a str>,
}

/// Structure of a row: its own type + layout and its children's types. Only
/// text-bearing containers are rows.
fn row_signature(v: &Value) -> Option<RowSignature<'_>> {
    let kind = v.get("type").and_then(Value::as_str)?;
    if children(v).is_empty() || !bears_text(v) {
        return None;
    }
    Some(RowSignature {
        kind,
        layout: layout_str(v),
        child_kinds: children(v)
            .iter()
            .map(|c| c.get("type").and_then(Value::as_str).unwrap_or(""))
            .collect(),
    })
}

/// A childless, text-free leaf at most [`DIVIDER_MAX_THICKNESS`] tall: a
/// `row-divider` frame, a rule rectangle, or a line.
fn is_row_divider(v: &Value) -> bool {
    let kind = v.get("type").and_then(Value::as_str);
    if kind == Some("line") {
        return true;
    }
    matches!(kind, Some("frame" | "rectangle"))
        && children(v).is_empty()
        && v.get("height")
            .and_then(Value::as_f64)
            .is_some_and(|h| h <= DIVIDER_MAX_THICKNESS)
}

fn is_media(v: &Value) -> bool {
    matches!(
        v.get("type").and_then(Value::as_str),
        Some("image" | "video")
    )
}

fn bottom_of(v: &Value, rects: &HashMap<String, Rect>) -> Option<f64> {
    v.get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .map(|r| r.y + r.h)
}

/// Lowest resolved bottom of any text inside `v` (inclusive).
fn text_bottom(v: &Value, rects: &HashMap<String, Rect>) -> Option<f64> {
    let own = (v.get("type").and_then(Value::as_str) == Some("text"))
        .then(|| bottom_of(v, rects))
        .flatten();
    children(v)
        .iter()
        .filter_map(|c| text_bottom(c, rects))
        .fold(own, |acc, b| Some(acc.map_or(b, |a: f64| a.max(b))))
}
