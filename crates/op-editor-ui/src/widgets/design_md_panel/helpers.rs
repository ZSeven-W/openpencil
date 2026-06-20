use crate::Color;

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
