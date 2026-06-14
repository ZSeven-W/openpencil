use super::CHAR_W;
use crate::{Color, Point2D, Rect};

pub(super) fn contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

pub(super) fn label_char_w(ch: char) -> f32 {
    if ch.is_ascii() {
        CHAR_W
    } else {
        11.0
    }
}

pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

pub(super) fn hex_to_color(hex: &str) -> Color {
    match op_editor_core::parse_hex_rgb(hex) {
        Some((r, g, b)) => Color { r, g, b, a: 1.0 },
        None => Color {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        },
    }
}
