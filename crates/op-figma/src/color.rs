//! Figma colour conversion — ports `figma-color-utils.ts`.

use crate::figma_types::FigColor;

/// JS `Math.round` on a non-negative product — half rounds up.
fn round_channel(v: f64) -> u8 {
    (v * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8
}

/// Figma 0–1 float colour → `#RRGGBB` (or `#RRGGBBAA` when alpha is
/// present and below 1.0). No gamma correction — straight linear scale.
pub fn figma_color_to_hex(c: &FigColor) -> String {
    let mut s = format!(
        "#{:02x}{:02x}{:02x}",
        round_channel(c.r),
        round_channel(c.g),
        round_channel(c.b),
    );
    if let Some(a) = c.a {
        if a < 1.0 {
            s.push_str(&format!("{:02x}", round_channel(a)));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(r: f64, g: f64, b: f64, a: Option<f64>) -> FigColor {
        FigColor { r, g, b, a }
    }

    #[test]
    fn opaque_colors_are_six_digit() {
        assert_eq!(figma_color_to_hex(&color(1.0, 0.0, 0.0, None)), "#ff0000");
        assert_eq!(
            figma_color_to_hex(&color(0.0, 0.0, 0.0, Some(1.0))),
            "#000000"
        );
        assert_eq!(
            figma_color_to_hex(&color(1.0, 1.0, 1.0, Some(1.0))),
            "#ffffff"
        );
    }

    #[test]
    fn translucent_colors_append_alpha() {
        // 0.25 * 255 + 0.5 = 64.25 → floor 64 → 0x40.
        assert_eq!(
            figma_color_to_hex(&color(0.0, 0.0, 0.0, Some(0.25))),
            "#00000040"
        );
    }

    #[test]
    fn rounds_half_up() {
        // 0.5 * 255 = 127.5 → +0.5 = 128 → 0x80.
        assert_eq!(figma_color_to_hex(&color(0.5, 0.5, 0.5, None)), "#808080");
    }
}
