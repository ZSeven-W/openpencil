//! Pieces of the Motion section that are not plain rows: the curve row's
//! geometry and thumbnail, and the keyframe-values header.
//!
//! The thumbnail samples `jian_core::motion::ease`, the curve Preview and
//! the canvas play, so a custom cubic-bezier shows exactly the shape (and
//! the [0, 1] clamp) the runtime will use.

use crate::theme::Theme;
use crate::widgets::property_panel_inputs::INPUT_HEIGHT;
use crate::widgets::property_panel_interactions::tr;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use jian_core::action::services::Easing as RuntimeEasing;
use jian_core::motion::ease;
use jian_ops_schema::motion::Easing;
use op_editor_core::motion_summary::KeyframeSummary;
use op_editor_core::Locale;

const CELL_GAP: f32 = 4.0;
/// Segments the thumbnail's curve is drawn with.
const SAMPLES: usize = 24;

/// Split a curve row into its thumbnail square and the four control
/// inputs (x1, y1, x2, y2).
pub(crate) fn cells(row: Rect) -> (Rect, [Rect; 4]) {
    let thumb = Rect::xywh(row.origin.x, row.origin.y, INPUT_HEIGHT, INPUT_HEIGHT);
    let left = row.origin.x + INPUT_HEIGHT + CELL_GAP;
    let cell_w = ((row.origin.x + row.size.x - left) - CELL_GAP * 3.0).max(0.0) / 4.0;
    let cell = |slot: usize| {
        Rect::xywh(
            left + slot as f32 * (cell_w + CELL_GAP),
            row.origin.y,
            cell_w,
            INPUT_HEIGHT,
        )
    };
    (thumb, [cell(0), cell(1), cell(2), cell(3)])
}

/// A control value with only the decimals it needs: `0`, `0.2`, `0.05`.
pub(crate) fn format_control(value: f32) -> String {
    let fixed = format!("{value:.2}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// The runtime curve in `rect`: progress across, eased output up.
pub(crate) fn paint_thumbnail(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, easing: &Easing) {
    cx.backend.fill_round_rect(rect, 6.0, theme.muted);
    let inset = 5.0;
    let (x0, y0) = (rect.origin.x + inset, rect.origin.y + rect.size.y - inset);
    let (w, h) = (rect.size.x - inset * 2.0, rect.size.y - inset * 2.0);
    let guide = Color {
        a: 0.35,
        ..theme.muted_foreground
    };
    cx.backend.stroke_line(
        Point2D::new(x0, y0),
        Point2D::new(x0 + w, y0 - h),
        guide,
        1.0,
    );
    let runtime = RuntimeEasing::from(easing);
    let point = |step: usize| {
        let progress = step as f32 / SAMPLES as f32;
        Point2D::new(x0 + progress * w, y0 - ease(runtime, progress) * h)
    };
    for step in 0..SAMPLES {
        cx.backend
            .stroke_line(point(step), point(step + 1), theme.primary, 1.5);
    }
}

/// `◆ Keyframe 2 of 3 · 0.20 s`, as plain text: the row names what the
/// inputs below edit.
pub(crate) fn paint_keyframe_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    locale: Locale,
    keyframe: &KeyframeSummary,
) {
    let center = Point2D::new(rect.origin.x + 8.0, rect.origin.y + rect.size.y / 2.0);
    let r = 4.5;
    cx.backend.fill_polygon(
        &[
            Point2D::new(center.x, center.y - r),
            Point2D::new(center.x + r, center.y),
            Point2D::new(center.x, center.y + r),
            Point2D::new(center.x - r, center.y),
        ],
        theme.primary,
    );
    let label = keyframe_title(locale, keyframe);
    let label = crate::util::ellipsize_to_width(&label, rect.size.x - 24.0, |text| {
        crate::widgets::text_metrics::measure_chrome(cx.backend, text, 12.0)
    });
    let layout = TextLayout::single_run(
        &label,
        "system-ui",
        12.0,
        theme.foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            rect.origin.x + 20.0,
            jian_widgets::centered_text_baseline_y(rect, 12.0),
        ),
    );
}

/// `Keyframe 2 of 3 · 0.20 s`.
pub(crate) fn keyframe_title(locale: Locale, keyframe: &KeyframeSummary) -> String {
    let title = tr(
        locale,
        ("motion.keyframe.title", "Keyframe {{index}} of {{count}}"),
    )
    .replace("{{index}}", &(keyframe.keyframe + 1).to_string())
    .replace("{{count}}", &keyframe.count.to_string());
    format!("{title} · {:.2} s", keyframe.time_ms / 1000.0)
}
