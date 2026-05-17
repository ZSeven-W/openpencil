//! Effects-section paint helpers for [`crate::widgets::PropertyPanel`].
//! Split out of `property_panel_sections.rs` to honor the 800-line
//! file ceiling. Each effect block paints a type row plus one
//! parameter row per editable scalar field.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::EffectSummary;
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label_with_add, to_jian_color, INPUT_HEIGHT, PAD_X,
    SECTION_GAP,
};
use crate::widgets::property_panel_layout::{
    effect_param_fields, EFFECT_PARAM_ROW_HEIGHT, EFFECT_ROW_HEIGHT,
};
use crate::widgets::property_panel_sections::PropertyLabels;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

// ── Effects section ───────────────────────────────────────────────

pub fn paint_effects_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    effects: &[EffectSummary],
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut row_y = paint_section_label_with_add(cx, theme, labels.effects, x, y, width);
    if effects.is_empty() {
        row_y += 8.0;
    } else {
        for eff in effects {
            paint_effect_row(cx, theme, eff, x, row_y, width);
            row_y += EFFECT_ROW_HEIGHT;
            for &(field, label) in effect_param_fields(eff.kind) {
                paint_effect_param_row(cx, theme, label, eff.param_value(field), x, row_y, width);
                row_y += EFFECT_PARAM_ROW_HEIGHT;
            }
        }
    }
    paint_section_divider(cx, theme, x, row_y, width);
    row_y + SECTION_GAP
}

/// Paint one effect-parameter row: `<label>  <value>  [−] [+]`. The
/// "−"/"+" stepper rects must match `action_button_rects`'s
/// `AdjustEffectParam` rects exactly so paint + hit-test agree.
fn paint_effect_param_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    value: f32,
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
    let value_text = format!("{value:.0}");
    let value_layout = TextLayout::single_run(
        &value_text,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &value_layout,
        Point2D::new(x + width - PAD_X - 78.0, y + 15.0),
    );
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
