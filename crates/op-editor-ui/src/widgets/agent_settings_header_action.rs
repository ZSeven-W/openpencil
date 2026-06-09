use crate::{Point2D, Rect};

pub(super) const HEADER_ACTION_RIGHT_INSET: f32 = 12.0;
const HEADER_ACTION_MIN_W: f32 = 96.0;
const HEADER_ACTION_PAD_X: f32 = 12.0;
const HEADER_ACTION_H: f32 = 24.0;

pub(super) fn header_action_rect(content: Rect, y: f32, text_w: f32) -> Rect {
    let w = (text_w + HEADER_ACTION_PAD_X * 2.0).max(HEADER_ACTION_MIN_W);
    Rect {
        origin: Point2D::new(
            content.origin.x + content.size.x - HEADER_ACTION_RIGHT_INSET - w,
            y,
        ),
        size: Point2D::new(w, HEADER_ACTION_H),
    }
}

pub(super) fn header_action_text_x(rect: Rect, text_w: f32) -> f32 {
    rect.origin.x + (rect.size.x - text_w).max(0.0) / 2.0
}

pub(super) fn header_action_text_baseline_y(rect: Rect) -> f32 {
    rect.origin.y + rect.size.y / 2.0 + 4.0
}
