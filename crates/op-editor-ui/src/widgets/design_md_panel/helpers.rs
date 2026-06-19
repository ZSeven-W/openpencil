use super::CHAR_W;
use crate::Color;

pub(super) fn label_char_w(ch: char) -> f32 {
    if ch.is_ascii() {
        CHAR_W
    } else {
        11.0
    }
}

pub(super) use crate::util::truncate_ellipsis as truncate;

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
