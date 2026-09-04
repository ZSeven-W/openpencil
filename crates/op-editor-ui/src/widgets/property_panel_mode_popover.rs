//! Shared gear-anchored PaddingEditMode popover chrome.
//!
//! The flex padding section and the stroke section both anchor a
//! "Single / Axis / Individual" radio popover to a gear icon; the box
//! geometry and the popover paint live here once, parameterized by
//! each section's radio size/gutter.

use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::PaddingEditMode;

const POPOVER_PAD: f32 = 6.0;
const POPOVER_TITLE_H: f32 = 22.0;
const POPOVER_ROW_H: f32 = 26.0;
const TOUCH_POPOVER_ROW_H: f32 = 30.0;

/// Popup chrome derived from the gear emitted by the shared action
/// walker. Paint and host containment both call this helper.
pub(crate) fn mode_popover_rect_from_gear(gear: Rect, width: f32, touch_controls: bool) -> Rect {
    let pop_w = 190.0_f32.min(width - crate::widgets::property_panel_inputs::PAD_X * 2.0);
    let row_h = if touch_controls {
        TOUCH_POPOVER_ROW_H
    } else {
        POPOVER_ROW_H
    };
    Rect {
        origin: Point2D::new(
            gear.origin.x + gear.size.x - pop_w,
            gear.origin.y + gear.size.y / 2.0 + 13.0,
        ),
        size: Point2D::new(pop_w, POPOVER_PAD * 2.0 + POPOVER_TITLE_H + row_h * 3.0),
    }
}

/// The three mode-radio row rects inside `pop_box`.
pub(crate) fn mode_popover_rows(pop_box: Rect, touch_controls: bool) -> [Rect; 3] {
    let row_h = if touch_controls {
        TOUCH_POPOVER_ROW_H
    } else {
        POPOVER_ROW_H
    };
    let first_row = pop_box.origin.y + POPOVER_PAD + POPOVER_TITLE_H;
    let row = |i: usize| Rect {
        origin: Point2D::new(pop_box.origin.x + POPOVER_PAD, first_row + i as f32 * row_h),
        size: Point2D::new(pop_box.size.x - POPOVER_PAD * 2.0, row_h),
    };
    [row(0), row(1), row(2)]
}

/// Radio circle — a jian `Radio` (ring + selected dot) at `size` px.
pub(crate) fn paint_radio_circle(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    size: f32,
    selected: bool,
) {
    jian_widgets::components::radio::Radio {
        selected,
        enabled: true,
    }
    .paint(
        cx.backend,
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(size, size),
        },
        &crate::widgets::button::tokens_from_theme(theme),
    );
}

/// Paint the popover box, title, and the three mode-radio rows.
/// `radio_size` / `radio_gutter` carry each caller's radio geometry.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_mode_popover(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    title_key: &'static str,
    active_mode: PaddingEditMode,
    hover: Option<usize>,
    pop_box: Rect,
    rows: &[Rect; 3],
    radio_size: f32,
    radio_gutter: f32,
) {
    cx.backend.fill_round_rect(pop_box, 8.0, theme.popover);
    cx.backend
        .stroke_round_rect(pop_box, 8.0, theme.border, 1.0);
    let title = TextLayout::single_run(
        op_i18n::translate(locale, title_key),
        "system-ui",
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(pop_box.origin.x + 12.0, pop_box.origin.y + 16.0),
    );
    for (i, rect) in rows.iter().enumerate() {
        let mode = PaddingEditMode::ALL[i];
        if hover == Some(i) {
            // jian-standard button_hover row wash, matching the other dropdowns.
            cx.backend.fill_round_rect(*rect, 6.0, theme.button_hover);
        }
        paint_radio_circle(
            cx,
            theme,
            rect.origin.x + 4.0,
            rect.origin.y + (rect.size.y - radio_size) / 2.0,
            radio_size,
            mode == active_mode,
        );
        let label = TextLayout::single_run(
            op_i18n::translate(locale, mode.label_key()),
            "system-ui",
            11.0,
            (theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label,
            Point2D::new(
                rect.origin.x + 4.0 + radio_gutter,
                rect.origin.y + rect.size.y / 2.0 + 4.0,
            ),
        );
    }
}
