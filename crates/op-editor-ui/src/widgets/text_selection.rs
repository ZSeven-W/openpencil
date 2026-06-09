//! Small paint helpers for whole-input text selection feedback.

use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

pub(super) fn paint_single_line_selection(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    text: &str,
    text_x: f32,
    baseline_y: f32,
    font_size: f32,
    max_x: f32,
) {
    if text.is_empty() {
        return;
    }
    let width = cx.backend.measure_text(text, font_size).min(max_x - text_x);
    if width <= 0.0 {
        return;
    }
    cx.backend.fill_round_rect(
        Rect {
            origin: Point2D::new(text_x - 2.0, baseline_y - font_size - 2.0),
            size: Point2D::new(width + 4.0, font_size + 6.0),
        },
        3.0,
        selection_color(theme),
    );
}

pub(super) fn selection_color(theme: &Theme) -> Color {
    Color {
        a: theme.primary.a * 0.28,
        ..theme.primary
    }
}
