//! Effects-section paint helpers for [`crate::widgets::PropertyPanel`].
//! Split out of `property_panel_sections.rs` to honor the 800-line
//! file ceiling. Each effect block paints a type row plus one
//! parameter row per editable scalar field.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::EffectSummary;
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label_with_add, to_jian_color, INPUT_HEIGHT, INPUT_RADIUS,
    PAD_X, SECTION_GAP,
};
use crate::widgets::property_panel_layout::{
    effect_param_fields, effect_param_value_rect, EFFECT_PARAM_ROW_HEIGHT, EFFECT_ROW_HEIGHT,
};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::EffectParamFocus;

// ── Effects section ───────────────────────────────────────────────

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
            paint_effect_row(cx, theme, eff, x, row_y, width);
            row_y += EFFECT_ROW_HEIGHT;
            for &(field, label) in effect_param_fields(eff.kind) {
                let focused = effect_focus == Some(EffectParamFocus { effect: ei, field });
                let caret = if focused && edit.caret_blink_on() {
                    Some(edit.caret.min(edit.draft.len()))
                } else {
                    None
                };
                paint_effect_param_row(
                    cx,
                    theme,
                    label,
                    eff.param_value(field),
                    focused,
                    edit.draft,
                    caret,
                    x,
                    row_y,
                    width,
                );
                row_y += EFFECT_PARAM_ROW_HEIGHT;
            }
        }
    }
    paint_section_divider(cx, theme, x, row_y, width);
    row_y + SECTION_GAP
}

/// Paint one effect-parameter row: `<label>  [value]  [−] [+]`. The
/// value box is click-to-type; when `focused` it shows the live
/// `draft` + caret instead of the committed `value`. The "−"/"+"
/// stepper rects must match `action_button_rects`'s `AdjustEffectParam`
/// rects exactly so paint + hit-test agree.
#[allow(clippy::too_many_arguments)]
fn paint_effect_param_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    value: f32,
    focused: bool,
    draft: &str,
    caret: Option<usize>,
    x: f32,
    y: f32,
    width: f32,
) {
    let label_layout = TextLayout::single_run(
        label,
        "system-ui",
        11.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label_layout, Point2D::new(x + PAD_X + 4.0, y + 15.0));
    // Editable value box — shows the live draft while focused.
    let box_rect = effect_param_value_rect(x, y, width);
    cx.backend
        .fill_round_rect(box_rect, INPUT_RADIUS, theme.muted);
    if focused {
        cx.backend
            .stroke_round_rect(box_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    let value_owned = format!("{value:.0}");
    let text = if focused { draft } else { value_owned.as_str() };
    let text_x = box_rect.origin.x + 10.0;
    let value_layout = TextLayout::single_run(
        text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &value_layout,
        Point2D::new(text_x, box_rect.origin.y + 16.0),
    );
    if let Some(pos) = caret {
        let caret_w = cx.backend.measure_text(&text[..pos.min(text.len())], 12.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(text_x + caret_w, box_rect.origin.y + 4.0),
                size: Point2D::new(1.5, box_rect.size.y - 8.0),
            },
            theme.foreground,
        );
    }
    // "−" then "+" — geometry mirrors the `AdjustEffectParam` rects.
    for (icon, off) in [(Icon::Minus, 48.0_f32), (Icon::Plus, 22.0_f32)] {
        let r = Rect {
            origin: Point2D::new(x + width - PAD_X - off, y + 3.0),
            size: Point2D::new(22.0, INPUT_HEIGHT - 6.0),
        };
        cx.backend.fill_round_rect(r, 6.0, theme.muted);
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(r.origin.x + 5.0, r.origin.y + (r.size.y - 12.0) / 2.0),
            12.0,
            theme.foreground,
            1.4,
        );
    }
}

/// Paint one effect row — the effect-type label on the left + a
/// right-aligned "✕" remove glyph. The "✕" hit rect is emitted by
/// `action_button_rects` as `RemoveEffect(index)`, so the glyph
/// position here must match that rect.
fn paint_effect_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    eff: &EffectSummary,
    x: f32,
    y: f32,
    width: f32,
) {
    let label = TextLayout::single_run(
        eff.kind.label(),
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label, Point2D::new(x + PAD_X, y + 15.0));
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(x + width - PAD_X - 14.0, y + 3.0),
        14.0,
        theme.muted_foreground,
        1.4,
    );
}
