use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

pub(super) fn paint_caret(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    now_ms: u64,
    anchor_ms: u64,
    x: f32,
    y: f32,
) {
    if !jian_core::anim::blink_visible(now_ms, anchor_ms, 500) {
        return;
    }
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(1.5, 15.0),
        },
        theme.foreground,
    );
}
