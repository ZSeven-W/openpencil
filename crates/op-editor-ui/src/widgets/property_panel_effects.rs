//! Effects-section paint helpers for [`crate::widgets::PropertyPanel`].
//!
//! Card-style layout (one card per effect): title row with the
//! `投影` label + remove `—` glyph, a 2-column grid of editable
//! parameter inputs, and an optional colour row (`颜色` label +
//! swatch + `rgba(...)` text). Geometry is shared with
//! `property_panel_layout` so paint + hit-test never drift.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::EffectSummary;
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label_with_add, INPUT_RADIUS, PAD_X, SECTION_GAP,
};
use crate::widgets::property_panel_layout::{
    effect_block_height, effect_color_rect, effect_has_color_row, effect_param_fields,
    effect_param_rect, effect_param_row_count, EFFECT_CARD_GAP, EFFECT_CARD_PAD,
    EFFECT_TITLE_ROW_HEIGHT,
};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::EffectParamFocus;

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_effects_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    effects: &[EffectSummary],
    edit: &EditContext<'_>,
    effect_focus: Option<EffectParamFocus>,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut row_y = paint_section_label_with_add(cx, theme, labels.effects, x, y, width);
    if effects.is_empty() {
        row_y += 8.0;
    } else {
        for (ei, eff) in effects.iter().enumerate() {
            paint_effect_card(cx, theme, eff, ei, edit, effect_focus, x, row_y, width);
            row_y += effect_block_height(eff.kind) + EFFECT_CARD_GAP;
        }
    }
    paint_section_divider(cx, theme, x, row_y, width);
    row_y + SECTION_GAP
}

#[allow(clippy::too_many_arguments)]
fn paint_effect_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    eff: &EffectSummary,
    effect_index: usize,
    edit: &EditContext<'_>,
    effect_focus: Option<EffectParamFocus>,
    x: f32,
    y: f32,
    width: f32,
) {
    let card_x = x + PAD_X;
    let card_w = width - PAD_X * 2.0;
    let card_rect = Rect {
        origin: Point2D::new(card_x, y),
        size: Point2D::new(card_w, effect_block_height(eff.kind)),
    };
    cx.backend
        .fill_round_rect(card_rect, INPUT_RADIUS, theme.muted);
    cx.backend
        .stroke_round_rect(card_rect, INPUT_RADIUS, theme.border, 1.0);

    // Title row — effect-kind label on the left, remove `—` on the right.
    let title = TextLayout::single_run(
        eff.kind.label(),
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(card_x + EFFECT_CARD_PAD + 4.0, y + 18.0),
    );
    draw_icon(
        cx.backend,
        Icon::Minus,
        Point2D::new(
            card_x + card_w - EFFECT_CARD_PAD - 16.0,
            y + (EFFECT_TITLE_ROW_HEIGHT - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.4,
    );

    // Parameter grid.
    let card_inner_y = y + EFFECT_CARD_PAD;
    for (i, &(field, label)) in effect_param_fields(eff.kind).iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let rect = effect_param_rect(card_x, card_inner_y, card_w, col, row);
        let focused = effect_focus
            == Some(EffectParamFocus {
                effect: effect_index,
                field,
            });
        let caret = if focused && edit.caret_blink_on() {
            Some(edit.caret.min(edit.draft.len()))
        } else {
            None
        };
        paint_param_input(
            cx,
            theme,
            label,
            eff.param_value(field),
            focused,
            edit.draft,
            caret,
            rect,
        );
    }

    // Colour row (Shadow only).
    if effect_has_color_row(eff.kind) {
        let row_count = effect_param_row_count(eff.kind);
        let cr = effect_color_rect(card_x, card_inner_y, card_w, row_count);
        paint_effect_color_row(cx, theme, eff.color, cr);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_param_input(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    value: f32,
    focused: bool,
    draft: &str,
    caret: Option<usize>,
    rect: Rect,
) {
    cx.backend
        .fill_round_rect(rect, INPUT_RADIUS, theme.background);
    if focused {
        cx.backend
            .stroke_round_rect(rect, INPUT_RADIUS, theme.primary, 1.5);
    } else {
        cx.backend
            .stroke_round_rect(rect, INPUT_RADIUS, theme.border, 1.0);
    }
    // Label sits on the left at the muted-foreground colour; the
    // editable value follows it left-aligned (matches Figma + the
    // image-spec'd "X 4" pattern).
    let label_layout = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let label_x = rect.origin.x + 10.0;
    let baseline_y = rect.origin.y + 19.0;
    cx.backend
        .draw_text(&label_layout, Point2D::new(label_x, baseline_y));
    let label_w = cx.backend.measure_text(label, 12.0);
    let value_text_owned = format!("{value:.0}");
    let text: &str = if focused {
        draft
    } else {
        value_text_owned.as_str()
    };
    let value_x = label_x + label_w + 8.0;
    let value_layout = TextLayout::single_run(
        text,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&value_layout, Point2D::new(value_x, baseline_y));
    if let Some(pos) = caret {
        let caret_w = cx.backend.measure_text(&text[..pos.min(text.len())], 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(value_x + caret_w, rect.origin.y + 6.0),
                size: Point2D::new(1.5, rect.size.y - 12.0),
            },
            theme.foreground,
        );
    }
}

fn paint_effect_color_row(cx: &mut PaintCx<'_>, theme: &Theme, color: Color, rect: Rect) {
    cx.backend
        .fill_round_rect(rect, INPUT_RADIUS, theme.background);
    cx.backend
        .stroke_round_rect(rect, INPUT_RADIUS, theme.border, 1.0);
    let label_layout = TextLayout::single_run(
        "颜色",
        "system-ui",
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let label_x = rect.origin.x + 10.0;
    let baseline_y = rect.origin.y + 19.0;
    cx.backend
        .draw_text(&label_layout, Point2D::new(label_x, baseline_y));
    // Colour swatch — same alpha-checker treatment as gradient stops
    // so a translucent shadow colour reads correctly.
    let swatch = Rect {
        origin: Point2D::new(rect.origin.x + 38.0, rect.origin.y + 7.0),
        size: Point2D::new(16.0, 16.0),
    };
    super::property_panel_fill::paint_alpha_checker_public(cx, swatch, 3.0);
    cx.backend.fill_round_rect(swatch, 3.0, color);
    // `rgba(r,g,b,a)` text after the swatch — matches the spec.
    let text = format!(
        "rgba({},{},{},{:.2})",
        (color.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        color.a.clamp(0.0, 1.0)
    );
    let value_layout = TextLayout::single_run(
        &text,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &value_layout,
        Point2D::new(rect.origin.x + 62.0, baseline_y),
    );
}
