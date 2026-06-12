use crate::{Point2D, Rect};

pub(super) fn control_center_y(rect: Rect) -> f32 {
    rect.origin.y + super::HEADER_HEIGHT / 2.0
}

pub(super) fn icon_origin(rect: Rect, x_offset: f32, size: f32) -> Point2D {
    Point2D::new(
        rect.origin.x + x_offset,
        control_center_y(rect) - size / 2.0,
    )
}

pub(super) fn text_baseline(rect: Rect, font_size: f32) -> f32 {
    control_center_y(rect) + font_size * 0.36
}
