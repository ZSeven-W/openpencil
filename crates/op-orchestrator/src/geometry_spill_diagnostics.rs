//! Vertical-spill, starved-`fill_container` and sibling-jam diagnostics.

use super::*;

/// A frame declaring a NUMERIC height but resolving MUCH taller — an
/// oversized child inflated it (jian grows the parent instead of letting the
/// child spill, so no edge ever crosses another and the width-overflow echo
/// stays blind). Measured (GLM-5.2 test0711-1.op): a 300px image inside a
/// declared-42px "Avatar" strip blew the strip — and the whole header —
/// to 300px. A generous slack keeps line-height rounding out of the report;
/// a real defect overshoots by multiples.
pub(super) const VERTICAL_SPILL_SLACK: f64 = 24.0;

pub(super) fn collect_vertical_spill_diagnostics(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    if v.get("clipContent").and_then(Value::as_bool) == Some(true) {
        return;
    }
    let Some(declared) = v.get("height").and_then(Value::as_f64) else {
        return;
    };
    let Some(resolved) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .map(|r| r.h)
    else {
        return;
    };
    // Taffy treats a numeric size as the border box, but OpenPencil's
    // post-layout repair reconciles an open container to its children's
    // measured extent plus authored padding. A generated container whose
    // children already consume the declared height can therefore resolve
    // taller by exactly its numeric vertical padding. That is breathing room,
    // not an oversized child. Expression padding is deliberately not guessed.
    let padding_allowance = numeric_vertical_padding(v).unwrap_or(0.0);
    if resolved <= declared + padding_allowance + VERTICAL_SPILL_SLACK {
        return;
    }
    let culprit = children(v)
        .iter()
        .filter_map(|c| {
            let cr = c
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| rects.get(id))?;
            (cr.h > declared + VERTICAL_SPILL_SLACK).then(|| (diag_label(c), cr.h))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let blame = culprit
        .map(|(label, h)| {
            format!(
                " — its child {label} is {}px tall and inflates it",
                h.round()
            )
        })
        .unwrap_or_default();
    out.push(format!(
        "{}: declared {}px tall but resolved {}px{blame}; shrink the oversized content to fit the declared height (or grow the parent on purpose)",
        diag_label(v),
        declared.round(),
        resolved.round()
    ));
}

/// A text-bearing `fill_container` child squeezed to a sliver in a horizontal
/// row — rigid siblings ate the width, the flex column resolved to a few px,
/// and its text wraps one glyph per line into a tower that blows the row's
/// height (measured: a 6px email column inflated its table rows to 432px).
/// Nothing OVERFLOWS in this failure — width-exceeds-parent checks are blind
/// to it — so the starvation itself is the report.
pub(super) const STARVED_FILL_W: f64 = 24.0;

pub(super) fn collect_starved_fill_diagnostics(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    if out.len() >= MAX_DIAGNOSTICS || layout_str(v) != Some("horizontal") {
        return;
    }
    let Some(pw) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .map(|r| r.w)
    else {
        return;
    };
    if pw < 200.0 {
        return; // a narrow chip/control row can't meaningfully starve anyone
    }
    for c in children(v) {
        if out.len() >= MAX_DIAGNOSTICS {
            return;
        }
        if c.get("width").and_then(Value::as_str) != Some("fill_container") || !bears_text(c) {
            continue;
        }
        let Some(cr) = c
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        else {
            continue;
        };
        if cr.w > 0.0 && cr.w < STARVED_FILL_W {
            out.push(format!(
                "{}: fill_container column starved to {}px in a {}px row — its fixed-width siblings eat the space and its text shreds vertically; shrink the fixed column widths so this one gets room",
                diag_label(c),
                cr.w.round(),
                pw.round()
            ));
        }
    }
}

/// Two adjacent text-bearing siblings count as JAMMED below this many px of
/// resolved horizontal breathing room ("Oct 24, 2024" + "42" reading as
/// "202442" — measured on a gap-less table row).
pub(super) const SIBLING_JAM_GAP: f64 = 3.0;
/// Siblings whose resolved rects overlap deeper than this are reported as
/// OVERLAPPING (painted on top of each other), not merely jammed.
pub(super) const SIBLING_OVERLAP_EPS: f64 = 2.0;
/// The JAM rule targets ROW cells ("Oct 24" + "42" reading as one word) —
/// two PAGE-level columns (an app-shell's sidebar and content) legitimately
/// touch. Only pairs where at least one side is row-cell sized qualify.
pub(super) const ROW_CELL_MAX_H: f64 = 120.0;

/// Does this subtree carry visible text? A spacer / icon / image sibling
/// touching its neighbor is normal; two TEXT columns touching is a jam.
pub(super) fn bears_text(v: &Value) -> bool {
    if v.get("type").and_then(Value::as_str) == Some("text") {
        return true;
    }
    children(v).iter().any(bears_text)
}

/// A JAM participant must be a text-bearing CONTAINER cell. Two bare `text`
/// siblings set tight on purpose ("$29"+"/mo", value+unit pairs) are
/// typography, not a data-column jam — measured false positive on a pricing
/// card's price row.
pub(super) fn is_cell_like(v: &Value) -> bool {
    matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group")
    ) && bears_text(v)
}

/// Report adjacent siblings of a horizontal FLEX row that resolved jammed
/// (text columns touching) or overlapping. Report-only: flush layouts are
/// sometimes intentional (joined button groups), so the model — not a fixer —
/// arbitrates, and the name-gated table pass stays the deterministic repairer.
pub(super) fn collect_sibling_jam_diagnostics(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    let Some(layout @ ("horizontal" | "vertical")) = layout_str(v) else {
        return;
    };
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    let kids = children(v);
    for pair in kids.windows(2) {
        if out.len() >= MAX_DIAGNOSTICS {
            return;
        }
        let (a, b) = (&pair[0], &pair[1]);
        let (Some(ra), Some(rb)) = (
            a.get("id")
                .and_then(Value::as_str)
                .and_then(|i| rects.get(i)),
            b.get("id")
                .and_then(Value::as_str)
                .and_then(|i| rects.get(i)),
        ) else {
            continue;
        };
        if ra.w <= 0.0 || rb.w <= 0.0 {
            continue;
        }
        let breathing = if layout == "vertical" {
            rb.y - (ra.y + ra.h)
        } else {
            rb.x - (ra.x + ra.w)
        };
        // Intentional OVERLAY, not an accident: one sibling's center inside
        // the other's box — a number set on a ring (ellipse + short text), a
        // corner badge on an avatar. Layout can't express children for an
        // ellipse, so models stack a sibling on purpose; don't report it.
        let center_inside = |inner: &Rect, outer: &Rect| {
            let (cx, cy) = (inner.x + inner.w / 2.0, inner.y + inner.h / 2.0);
            cx > outer.x && cx < outer.x + outer.w && cy > outer.y && cy < outer.y + outer.h
        };
        // An ellipse + a SHORT text sibling is the ring/badge stack by
        // construction — an ellipse can't carry children, so models sibling
        // the number on purpose. The center-inside test alone misses the
        // boundary case (a "2" whose center lands EXACTLY on the ring's top
        // edge — measured, p44), so the pair's shape is the intent signal.
        let is_ellipse = |v: &Value| v.get("type").and_then(Value::as_str) == Some("ellipse");
        let is_short_text = |v: &Value| {
            v.get("type").and_then(Value::as_str) == Some("text")
                && v.get("content")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.trim().chars().count() <= 4)
        };
        let ring_badge = (is_ellipse(a) && is_short_text(b)) || (is_ellipse(b) && is_short_text(a));
        let overlay = ring_badge || center_inside(ra, rb) || center_inside(rb, ra);
        let all_fill_cells = crate::chip_repair::is_fill_container_frame(a)
            && crate::chip_repair::is_fill_container_frame(b);
        if breathing < -SIBLING_OVERLAP_EPS && !overlay {
            let axis = if layout == "vertical" { "stack" } else { "row" };
            out.push(format!(
                "{} and {}: siblings OVERLAP by {}px — their resolved boxes collide in the {axis}; resize or add spacing so they do not paint on top of each other",
                diag_label(a),
                diag_label(b),
                (-breathing).round()
            ));
        } else if layout == "horizontal"
            && breathing < SIBLING_JAM_GAP
            && !all_fill_cells
            && !super::geometry_grid_row_fit::is_centered_grid_row(v)
            && ra.h.min(rb.h) <= ROW_CELL_MAX_H
            && is_cell_like(a)
            && is_cell_like(b)
        {
            out.push(format!(
                "{} and {}: text columns touch (only {}px apart) — their contents read as one word; add a gap on the row (e.g. gap: 16)",
                diag_label(a),
                diag_label(b),
                breathing.max(0.0).round()
            ));
        }
    }
}
