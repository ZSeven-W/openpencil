pub mod contrast;
pub mod oklch;
pub mod palettes;

pub use contrast::{on_color, scan_pairs, wcag, ContrastReport, ContrastViolation};
pub use oklch::{hex_saturation, hex_to_oklch, oklch_to_hex, scale12, Mode, Oklch};

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb_channels(hex: &str) -> [i32; 3] {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        [
            i32::from_str_radix(&hex[0..2], 16).expect("red channel"),
            i32::from_str_radix(&hex[2..4], 16).expect("green channel"),
            i32::from_str_radix(&hex[4..6], 16).expect("blue channel"),
        ]
    }

    fn assert_hex_close(actual: &str, expected: &str) {
        let actual = rgb_channels(actual);
        let expected = rgb_channels(expected);
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual - expected).abs() <= 1,
                "expected {actual:?} to be within 1 of {expected:?}"
            );
        }
    }

    #[test]
    fn oklch_roundtrip_stable() {
        for hex in [
            "#000000", "#FFFFFF", "#1E3A8A", "#3B82F6", "#FFEB3B", "#00FF00", "#64748B",
        ] {
            let oklch = hex_to_oklch(hex).expect("valid hex");
            let roundtripped = oklch_to_hex(oklch);
            assert_hex_close(&roundtripped, hex);
        }
    }

    #[test]
    fn on_color_dark_for_bright_yellow() {
        assert_eq!(on_color("#FFEB3B"), "#0F172A");
        let ratio = wcag("#000000", "#FFEB3B").expect("valid colors");
        assert!((ratio - 17.2).abs() <= 0.3, "ratio was {ratio}");
    }

    #[test]
    fn on_color_dark_for_pure_green() {
        assert_eq!(on_color("#00FF00"), "#0F172A");
        let ratio = wcag("#000000", "#00FF00").expect("valid colors");
        assert!((ratio - 15.3).abs() <= 0.3, "ratio was {ratio}");
    }

    #[test]
    fn on_color_white_for_deep_blue() {
        assert_eq!(on_color("#1E3A8A"), "#FFFFFF");
    }

    #[test]
    fn contrast_scan_flags_low_pair() {
        let low = scan_pairs(&[("#777777".to_string(), "#888888".to_string(), 4.5)]);
        assert_eq!(low.violations.len(), 1);
        assert_eq!(low.violations[0].fg, "#777777");
        assert_eq!(low.violations[0].bg, "#888888");
        assert!(low.violations[0].ratio < low.violations[0].target);

        let pass = scan_pairs(&[
            ("#000000".to_string(), "#FFFFFF".to_string(), 4.5),
            ("#FFFFFF".to_string(), "#1E3A8A".to_string(), 4.5),
        ]);
        assert!(pass.violations.is_empty());
    }
}
