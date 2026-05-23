//! WCAG color helpers shared across diagnostic detectors.
//!
//! Ported behaviour-for-behaviour from
//! `packages/pen-ai-skills/src/diagnostics/color-utils.ts`. Any change here
//! must keep the contrast epsilon tests green (spec §8).

/// A parsed sRGB color. Alpha is dropped during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Parse `#rgb` / `#rrggbb` / `#rrggbbaa` to `Rgb` (alpha dropped).
/// Returns `None` on parse failure — mirrors TS `parseHexColor`.
pub fn parse_hex_color(s: &str) -> Option<Rgb> {
    let hex = s.trim().strip_prefix('#')?;
    if !(3..=8).contains(&hex.len()) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let expanded: String = if hex.len() == 3 {
        hex.chars().flat_map(|c| [c, c]).collect()
    } else {
        hex.to_string()
    };
    // TS rejects lengths 4/5/7 here — only 6 (rrggbb) or 8 (rrggbbaa) pass.
    if expanded.len() != 6 && expanded.len() != 8 {
        return None;
    }
    let r = u8::from_str_radix(&expanded[0..2], 16).ok()?;
    let g = u8::from_str_radix(&expanded[2..4], 16).ok()?;
    let b = u8::from_str_radix(&expanded[4..6], 16).ok()?;
    Some(Rgb { r, g, b })
}

/// WCAG 2.x relative luminance for sRGB. Returns 0.0–1.0.
pub fn relative_luminance(c: Rgb) -> f64 {
    fn lin(v: u8) -> f64 {
        let s = v as f64 / 255.0;
        if s <= 0.039_28 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG relative-luminance contrast ratio between two color strings.
/// Returns `1.0` for identical strings (before parsing — mirrors TS),
/// grows toward `21.0` as colors diverge, or `f64::INFINITY` if either
/// string fails to parse (e.g. unresolved variable refs).
pub fn color_contrast(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    let (Some(pa), Some(pb)) = (parse_hex_color(a), parse_hex_color(b)) else {
        return f64::INFINITY;
    };
    let lum_a = relative_luminance(pa);
    let lum_b = relative_luminance(pb);
    let lighter = lum_a.max(lum_b);
    let darker = lum_a.min(lum_b);
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    #[test]
    fn parse_hex_expands_shorthand_and_drops_alpha() {
        assert_eq!(
            parse_hex_color("#fff"),
            Some(Rgb {
                r: 255,
                g: 255,
                b: 255
            })
        );
        assert_eq!(parse_hex_color("#000000"), Some(Rgb { r: 0, g: 0, b: 0 }));
        assert_eq!(
            parse_hex_color("#11223344"),
            Some(Rgb {
                r: 0x11,
                g: 0x22,
                b: 0x33
            })
        );
        assert_eq!(
            parse_hex_color("  #abc  "),
            Some(Rgb {
                r: 0xaa,
                g: 0xbb,
                b: 0xcc
            })
        );
    }

    #[test]
    fn parse_hex_rejects_bad_input() {
        assert_eq!(parse_hex_color("fff"), None); // no '#'
        assert_eq!(parse_hex_color("#12"), None); // too short
        assert_eq!(parse_hex_color("#1234"), None); // length 4 not 6/8
        assert_eq!(parse_hex_color("#12345"), None); // length 5
        assert_eq!(parse_hex_color("#1234567"), None); // length 7
        assert_eq!(parse_hex_color("#ggg"), None); // non-hex
        assert_eq!(parse_hex_color("$color-1"), None); // unresolved ref
    }

    #[test]
    fn relative_luminance_endpoints() {
        assert!((relative_luminance(Rgb { r: 0, g: 0, b: 0 }) - 0.0).abs() < EPS);
        assert!(
            (relative_luminance(Rgb {
                r: 255,
                g: 255,
                b: 255
            }) - 1.0)
                .abs()
                < EPS
        );
    }

    #[test]
    fn relative_luminance_mid_gray_exercises_gamma_branch() {
        // 0x77 = 119; s = 119/255 ≈ 0.46667 > 0.03928 → gamma branch.
        // Expected value computed against the TS formula (threshold 0.03928,
        // gamma 2.4) — verified via Node.js to 1e-7.
        let lum = relative_luminance(Rgb {
            r: 0x77,
            g: 0x77,
            b: 0x77,
        });
        assert!((lum - 0.184_474_994_5).abs() < 1e-7);
    }

    #[test]
    fn color_contrast_known_vectors() {
        assert!((color_contrast("#000000", "#FFFFFF") - 21.0).abs() < EPS);
        assert!((color_contrast("#777777", "#FFFFFF") - 4.48).abs() < 0.01);
    }

    #[test]
    fn color_contrast_identical_returns_one_before_parsing() {
        // Identical strings short-circuit to 1.0 even when unparseable.
        assert_eq!(color_contrast("$same", "$same"), 1.0);
    }

    #[test]
    fn color_contrast_unparseable_returns_infinity() {
        assert_eq!(color_contrast("$color-1", "#FFFFFF"), f64::INFINITY);
        assert_eq!(color_contrast("#FFFFFF", "not-a-color"), f64::INFINITY);
    }
}
