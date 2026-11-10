//! The expanded body of a step whose details are quality-repair records.
//!
//! The quality passes report each applied repair as one
//! `category · pass · node · field before → after` line. Painted as plain
//! grey text, a dozen of them read as one undifferentiated block. A step
//! whose details hold such records shows them here instead:
//!
//! ```text
//! Checked layout, overflow, hierarchy, structure, palette
//! (● layout 3) (● structure 7)                          ← one chip per category
//! ● Home  n1 · hero-bleed                               ← node, then the pass
//!   gap 0 → 16                                          ← the new value stands out
//! ```
//!
//! Every row has a fixed height, so the step's height is known when the
//! transcript is laid out; only horizontal fitting is measured at paint.
//! Lines that are not records (the checked scope, notes, the "… and N
//! more" notice) stay plain muted rows.

use super::ai_chat_transcript::ACTION_DETAIL_LINE_H;
use super::ai_chat_transcript_steps::{wrap_detail, DETAIL_FONT};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

/// The repair families the quality passes report, in their display order.
const CATEGORIES: [&str; 5] = ["layout", "overflow", "hierarchy", "structure", "palette"];
const CHIP_H: f32 = 18.0;
const CHIP_ROW_H: f32 = CHIP_H + 8.0;
const REPAIR_ROW_H: f32 = 34.0;
const DOT: f32 = 6.0;
/// Where a repair row's text starts, past its category dot.
const TEXT_INDENT: f32 = 13.0;
const FAMILY: &str = "system-ui";

/// One line of a step's details, as the repair body understands it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReceiptLine {
    Repair(Repair),
    /// `repairs: layout 3, structure 7`.
    Counts(Vec<(String, usize)>),
    /// Anything else, pre-wrapped into rows.
    Text(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Repair {
    pub category: String,
    pub pass: String,
    /// Node name (or bare id when it has none) and id, when the repair has a
    /// node.
    pub node: Option<(String, Option<String>)>,
    /// The changed field and its old value (`gap 0`), or the whole change
    /// when it has no `→`.
    pub before: String,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReceiptBody {
    pub lines: Vec<ReceiptLine>,
    pub height: f32,
}

/// The structured body for `details`, or `None` when they hold no repair
/// record (every other step keeps its plain detail rows).
pub(crate) fn receipt_body(details: &[String], budget: u32) -> Option<ReceiptBody> {
    let lines: Vec<ReceiptLine> = details
        .iter()
        .flat_map(|detail| detail.split('\n'))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| parse_line(line, budget))
        .collect();
    if !lines
        .iter()
        .any(|line| matches!(line, ReceiptLine::Repair(_)))
    {
        return None;
    }
    let height = lines.iter().map(line_height).sum::<f32>() + 4.0;
    Some(ReceiptBody { lines, height })
}

fn parse_line(line: &str, budget: u32) -> ReceiptLine {
    if let Some(repair) = parse_repair(line) {
        return ReceiptLine::Repair(repair);
    }
    if let Some(counts) = parse_counts(line) {
        return ReceiptLine::Counts(counts);
    }
    ReceiptLine::Text(wrap_detail(
        &super::ai_chat_transcript_receipts::pluralized(line),
        budget,
    ))
}

fn parse_repair(line: &str) -> Option<Repair> {
    let parts: Vec<&str> = line.split(" · ").collect();
    let (category, pass) = (*parts.first()?, *parts.get(1)?);
    if !CATEGORIES.contains(&category) || parts.len() < 3 {
        return None;
    }
    let (node, rest) = match parts.get(2) {
        Some(label) if parts.len() > 3 && label.ends_with(']') => {
            (Some(node_label(label)), &parts[3..])
        }
        _ => (None, &parts[2..]),
    };
    let change = rest.join(" · ");
    let (before, after) = match change.split_once(" → ") {
        Some((before, after)) => (before.to_owned(), Some(after.to_owned())),
        None => (change, None),
    };
    Some(Repair {
        category: category.to_owned(),
        pass: pass.to_owned(),
        node,
        before,
        after,
    })
}

/// `Home [n1]` → (`Home`, `n1`); `[n28]` → (`n28`, none).
fn node_label(label: &str) -> (String, Option<String>) {
    match label.rsplit_once(" [") {
        Some((name, id)) => (name.to_owned(), Some(id.trim_end_matches(']').to_owned())),
        None => (
            label
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned(),
            None,
        ),
    }
}

fn parse_counts(line: &str) -> Option<Vec<(String, usize)>> {
    let list = line.strip_prefix("repairs: ")?;
    list.split(", ")
        .map(|entry| {
            let (category, count) = entry.rsplit_once(' ')?;
            Some((category.to_owned(), count.parse().ok()?))
        })
        .collect()
}

fn line_height(line: &ReceiptLine) -> f32 {
    match line {
        ReceiptLine::Repair(_) => REPAIR_ROW_H,
        ReceiptLine::Counts(_) => CHIP_ROW_H,
        ReceiptLine::Text(rows) => rows.len() as f32 * ACTION_DETAIL_LINE_H,
    }
}

/// A category's colour: a fixed hue per family so the dot under a repair
/// matches its chip, legible on both themes.
fn category_color(category: &str) -> Color {
    match category {
        "layout" => Color::rgb_u8(0x3B, 0x82, 0xF6),
        "overflow" => Color::rgb_u8(0xEF, 0x44, 0x44),
        "hierarchy" => Color::rgb_u8(0xF5, 0x9E, 0x0B),
        "structure" => Color::rgb_u8(0x8B, 0x5C, 0xF6),
        "palette" => Color::rgb_u8(0xEC, 0x48, 0x99),
        _ => Color::rgb_u8(0x94, 0xA3, 0xB8),
    }
}

/// Paint `body` with its top-left at (`x`, `y`) inside `width`.
pub(crate) fn paint_receipt_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    body: &ReceiptBody,
    x: f32,
    mut y: f32,
    width: f32,
) {
    for line in &body.lines {
        match line {
            ReceiptLine::Text(rows) => {
                let mut baseline = y + 10.0;
                for row in rows {
                    text(
                        cx,
                        row,
                        x,
                        baseline,
                        DETAIL_FONT,
                        400,
                        theme.muted_foreground,
                    );
                    baseline += ACTION_DETAIL_LINE_H;
                }
            }
            ReceiptLine::Counts(counts) => paint_chips(cx, theme, counts, x, y + 3.0, width),
            ReceiptLine::Repair(repair) => paint_repair(cx, theme, repair, x, y, width),
        }
        y += line_height(line);
    }
}

fn paint_chips(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    counts: &[(String, usize)],
    x: f32,
    y: f32,
    width: f32,
) {
    let mut chip_x = x;
    for (category, count) in counts {
        let label = format!("{category} {count}");
        let label_w = text_width(cx, &label, DETAIL_FONT, 500);
        let chip_w = label_w + DOT + 18.0;
        if chip_x + chip_w > x + width {
            break;
        }
        let hue = category_color(category);
        cx.backend.fill_round_rect(
            Rect::xywh(chip_x, y, chip_w, CHIP_H),
            CHIP_H / 2.0,
            Color { a: 0.14, ..hue },
        );
        dot(cx, hue, chip_x + 7.0, y + CHIP_H / 2.0);
        text(
            cx,
            &label,
            chip_x + 11.0 + DOT,
            y + 12.5,
            DETAIL_FONT,
            500,
            theme.foreground,
        );
        chip_x += chip_w + 6.0;
    }
}

fn paint_repair(cx: &mut PaintCx<'_>, theme: &Theme, repair: &Repair, x: f32, y: f32, width: f32) {
    let text_x = x + TEXT_INDENT;
    let text_w = (width - TEXT_INDENT).max(1.0);
    let top = y + 12.0;
    dot(
        cx,
        category_color(&repair.category),
        x + DOT / 2.0,
        top - 4.0,
    );

    // Line one: who was repaired, then by which pass.
    let mut line_x = text_x;
    if let Some((name, id)) = &repair.node {
        let name = fit(cx, name, 11.0, 600, text_w * 0.6);
        text(cx, &name, line_x, top, 11.0, 600, theme.foreground);
        line_x += text_width(cx, &name, 11.0, 600) + 5.0;
        if let Some(id) = id {
            text(
                cx,
                id,
                line_x,
                top,
                DETAIL_FONT,
                400,
                theme.muted_foreground,
            );
            line_x += text_width(cx, id, DETAIL_FONT, 400) + 4.0;
        }
        text(
            cx,
            "·",
            line_x,
            top,
            DETAIL_FONT,
            400,
            theme.muted_foreground,
        );
        line_x += text_width(cx, "·", DETAIL_FONT, 400) + 4.0;
    }
    let pass_w = (text_x + text_w - line_x).max(1.0);
    let pass = fit(cx, &repair.pass, DETAIL_FONT, 400, pass_w);
    text(
        cx,
        &pass,
        line_x,
        top,
        DETAIL_FONT,
        400,
        theme.muted_foreground,
    );

    // Line two: the change, the new value emphasised.
    let second = top + 15.0;
    let after_w = repair.after.as_ref().map_or(0.0, |after| {
        text_width(cx, &format!(" → {after}"), DETAIL_FONT, 600)
    });
    let before_budget = if after_w > 0.0 {
        (text_w - after_w.min(text_w * 0.6)).max(text_w * 0.4)
    } else {
        text_w
    };
    let before = fit(cx, &repair.before, DETAIL_FONT, 400, before_budget);
    text(
        cx,
        &before,
        text_x,
        second,
        DETAIL_FONT,
        400,
        theme.muted_foreground,
    );
    if let Some(after) = &repair.after {
        let mut after_x = text_x + text_width(cx, &before, DETAIL_FONT, 400) + 4.0;
        text(
            cx,
            "→",
            after_x,
            second,
            DETAIL_FONT,
            400,
            theme.muted_foreground,
        );
        after_x += text_width(cx, "→", DETAIL_FONT, 400) + 4.0;
        let after = fit(
            cx,
            after,
            DETAIL_FONT,
            600,
            (text_x + text_w - after_x).max(1.0),
        );
        text(
            cx,
            &after,
            after_x,
            second,
            DETAIL_FONT,
            600,
            theme.foreground,
        );
    }
}

/// Width of `text` in the family it paints in (the family-blind measure
/// resolves a different default font on native, which made the arrow
/// overlap long values).
fn text_width(cx: &mut PaintCx<'_>, text: &str, size: f32, weight: u16) -> f32 {
    cx.backend
        .measure_text_family_styled(text, size, FAMILY, weight, false)
}

fn dot(cx: &mut PaintCx<'_>, color: Color, center_x: f32, center_y: f32) {
    cx.backend.fill_oval(
        Rect::xywh(center_x - DOT / 2.0, center_y - DOT / 2.0, DOT, DOT),
        color,
    );
}

fn text(
    cx: &mut PaintCx<'_>,
    text: &str,
    x: f32,
    baseline: f32,
    size: f32,
    weight: u16,
    color: Color,
) {
    let layout =
        TextLayout::single_run(text, FAMILY, size, color.to_jian(), Point2D::new(0.0, 0.0))
            .with_font_weight(weight);
    cx.backend.draw_text(&layout, Point2D::new(x, baseline));
}

/// `text` shortened with an ellipsis until it measures at most `max_w`.
fn fit(cx: &mut PaintCx<'_>, text: &str, size: f32, weight: u16, max_w: f32) -> String {
    if text_width(cx, text, size, weight) <= max_w {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0usize, chars.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate: String = chars[..mid].iter().collect::<String>() + "…";
        if text_width(cx, &candidate, size, weight) <= max_w {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    chars[..low].iter().collect::<String>() + "…"
}

#[cfg(test)]
#[path = "ai_chat_transcript_receipt_body_tests.rs"]
mod tests;
