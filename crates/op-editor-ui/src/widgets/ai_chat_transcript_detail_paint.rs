//! Paint a [`DetailBody`]. Row heights are fixed by the model
//! (`ai_chat_transcript_detail_body::line_height`); this only fits text
//! horizontally, measured in the font it paints in.

use super::ai_chat_transcript::ACTION_DETAIL_LINE_H;
use super::ai_chat_transcript_detail_body::{
    line_height, DetailBody, DetailLine, FieldValue, Repair, CHIP_H, KEY_W,
};
use super::ai_chat_transcript_steps::DETAIL_FONT;
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

const DOT: f32 = 6.0;
/// Where a repair row's text starts, past its category dot.
const TEXT_INDENT: f32 = 13.0;
const FAMILY: &str = "system-ui";

/// How a chip list is tinted.
#[derive(Clone, Copy)]
enum ChipTone {
    /// Facts about a step (`41 elements`).
    Neutral,
    /// A field's items (`skills`), in the accent colour.
    Accent,
}

/// Paint `body` with its top-left at (`x`, `y`) inside `width`.
pub(crate) fn paint_detail_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    body: &DetailBody,
    x: f32,
    mut y: f32,
    width: f32,
) {
    for line in &body.lines {
        match line {
            DetailLine::Text { rows, alert } => paint_rows(cx, theme, rows, x, y, *alert),
            DetailLine::Counts(counts) => {
                paint_category_chips(cx, theme, counts, x, y + 3.0, width)
            }
            DetailLine::Stats(stats) => {
                paint_chip_list(cx, theme, stats, x, y + 3.0, width, ChipTone::Neutral)
            }
            DetailLine::Change { before, after } => {
                paint_change(cx, theme, before, after, x, y + 11.0, width)
            }
            DetailLine::Repair(repair) => paint_repair(cx, theme, repair, x, y, width),
            DetailLine::Field { key, value, alert } => {
                paint_field(cx, theme, key, value, *alert, x, y, width)
            }
        }
        y += line_height(line);
    }
}

/// Plain rows; a failed step's rows are tinted and marked with a dot.
fn paint_rows(cx: &mut PaintCx<'_>, theme: &Theme, rows: &[String], x: f32, y: f32, alert: bool) {
    let (color, text_x) = if alert {
        dot(cx, theme.destructive, x + DOT / 2.0, y + 6.5);
        (theme.destructive, x + TEXT_INDENT)
    } else {
        (theme.muted_foreground, x)
    };
    let mut baseline = y + 10.0;
    for row in rows {
        text(cx, row, text_x, baseline, DETAIL_FONT, 400, color);
        baseline += ACTION_DETAIL_LINE_H;
    }
}

/// A key in its column, the value beside it.
#[allow(clippy::too_many_arguments)]
fn paint_field(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    key: &str,
    value: &FieldValue,
    alert: bool,
    x: f32,
    y: f32,
    width: f32,
) {
    let value_x = x + KEY_W + 6.0;
    let value_w = (width - KEY_W - 6.0).max(1.0);
    let key_baseline = match value {
        // Level with the chip labels (chips sit 3px down, text at 12.5).
        FieldValue::Chips(_) => y + 15.5,
        FieldValue::Change { .. } => y + 11.0,
        _ => y + 10.0,
    };
    let key_weight = if matches!(value, FieldValue::Fields { .. }) {
        600
    } else {
        500
    };
    let key_text = fit(cx, key, DETAIL_FONT, key_weight, KEY_W - 4.0);
    let key_color = if alert {
        theme.destructive
    } else {
        theme.muted_foreground
    };
    text(
        cx,
        &key_text,
        x,
        key_baseline,
        DETAIL_FONT,
        key_weight,
        key_color,
    );
    match value {
        FieldValue::Chips(items) => paint_chip_list(
            cx,
            theme,
            items,
            value_x,
            y + 3.0,
            value_w,
            ChipTone::Accent,
        ),
        FieldValue::Change { before, after } => {
            paint_change(cx, theme, before, after, value_x, y + 11.0, value_w)
        }
        FieldValue::Text(rows) => {
            let color = if alert {
                theme.destructive
            } else {
                theme.foreground
            };
            let mut baseline = y + 10.0;
            for row in rows {
                let row = fit(cx, row, DETAIL_FONT, 400, value_w);
                text(cx, &row, value_x, baseline, DETAIL_FONT, 400, color);
                baseline += ACTION_DETAIL_LINE_H;
            }
        }
        FieldValue::Fields { fields, more } => {
            let mut baseline = y + 10.0 + ACTION_DETAIL_LINE_H;
            for (name, field_value) in fields {
                let name = fit(cx, name, DETAIL_FONT, 400, KEY_W - 12.0);
                text(
                    cx,
                    &name,
                    x + 8.0,
                    baseline,
                    DETAIL_FONT,
                    400,
                    theme.muted_foreground,
                );
                let field_value = fit(cx, field_value, DETAIL_FONT, 500, value_w);
                text(
                    cx,
                    &field_value,
                    value_x,
                    baseline,
                    DETAIL_FONT,
                    500,
                    theme.foreground,
                );
                baseline += ACTION_DETAIL_LINE_H;
            }
            if *more > 0 {
                let label = format!("+{more} more");
                text(
                    cx,
                    &label,
                    x + 8.0,
                    baseline,
                    DETAIL_FONT,
                    400,
                    theme.muted_foreground,
                );
            }
        }
    }
}

/// `before → after`, the new value emphasised.
fn paint_change(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    before: &str,
    after: &str,
    x: f32,
    baseline: f32,
    width: f32,
) {
    let after_w = text_width(cx, &format!(" → {after}"), DETAIL_FONT, 600);
    let before_budget = (width - after_w.min(width * 0.6)).max(width * 0.4);
    let before = fit(cx, before, DETAIL_FONT, 400, before_budget);
    text(
        cx,
        &before,
        x,
        baseline,
        DETAIL_FONT,
        400,
        theme.muted_foreground,
    );
    let mut after_x = x + text_width(cx, &before, DETAIL_FONT, 400) + 4.0;
    text(
        cx,
        "→",
        after_x,
        baseline,
        DETAIL_FONT,
        400,
        theme.muted_foreground,
    );
    after_x += text_width(cx, "→", DETAIL_FONT, 400) + 4.0;
    let after = fit(cx, after, DETAIL_FONT, 600, (x + width - after_x).max(1.0));
    text(
        cx,
        &after,
        after_x,
        baseline,
        DETAIL_FONT,
        600,
        theme.foreground,
    );
}

/// Chips on one row; the ones that do not fit become a `+N` chip.
fn paint_chip_list(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    items: &[String],
    x: f32,
    y: f32,
    width: f32,
    tone: ChipTone,
) {
    let (fill, ink) = match tone {
        ChipTone::Neutral => (theme.muted, theme.foreground),
        ChipTone::Accent => (
            Color {
                a: 0.12,
                ..theme.primary
            },
            theme.primary,
        ),
    };
    let widths: Vec<f32> = items
        .iter()
        .map(|item| text_width(cx, item, DETAIL_FONT, 500) + 16.0)
        .collect();
    let mut chip_x = x;
    for (index, (item, chip_w)) in items.iter().zip(&widths).enumerate() {
        let left = items.len() - index - 1;
        let more_w = if left > 0 {
            text_width(cx, &format!("+{left}"), DETAIL_FONT, 500) + 16.0 + 6.0
        } else {
            0.0
        };
        if chip_x + chip_w + more_w > x + width {
            let label = format!("+{}", items.len() - index);
            paint_chip(cx, &label, chip_x, y, fill, ink);
            return;
        }
        paint_chip(cx, item, chip_x, y, fill, ink);
        chip_x += chip_w + 6.0;
    }
}

fn paint_chip(cx: &mut PaintCx<'_>, label: &str, x: f32, y: f32, fill: Color, ink: Color) {
    let chip_w = text_width(cx, label, DETAIL_FONT, 500) + 16.0;
    cx.backend
        .fill_round_rect(Rect::xywh(x, y, chip_w, CHIP_H), CHIP_H / 2.0, fill);
    text(cx, label, x + 8.0, y + 12.5, DETAIL_FONT, 500, ink);
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

fn paint_category_chips(
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
