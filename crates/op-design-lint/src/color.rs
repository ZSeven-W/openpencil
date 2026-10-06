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
    // TS parity: `#` required; 3 (shorthand) / 6 / 8 digits only — the
    // 4-digit `#rgba` shorthand stays rejected. Delegates to op-util.
    const OPTS: op_util::hex_color::HexOptions = op_util::hex_color::HexOptions {
        require_hash: true,
        allow_rgb_shorthand: true,
        allow_rgba_shorthand: false,
        allow_alpha: true,
    };
    let [r, g, b, _] = op_util::hex_color::parse_hex_rgba8(s, OPTS)?;
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

/// Standard sRGB → HSL conversion. Returns `(hue, saturation, lightness)`
/// with hue in `[0, 360)` degrees and saturation / lightness in `[0, 1]`.
/// Achromatic colors (`max == min`) report hue `0` and saturation `0`, so a
/// hue-range predicate never matches a gray.
pub fn hsl(c: Rgb) -> (f64, f64, f64) {
    let r = c.r as f64 / 255.0;
    let g = c.g as f64 / 255.0;
    let b = c.b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    let delta = max - min;
    if delta == 0.0 {
        return (0.0, 0.0, lightness);
    }
    let saturation = if lightness > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };
    let hue = if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    // `%` keeps the sign of the dividend; fold negative hues into [0, 360).
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    (hue, saturation, lightness)
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
    fn hsl_known_vectors() {
        // Pure primaries and a mid purple, cross-checked against the CSS
        // `hsl()` definition.
        let (h, s, l) = hsl(Rgb { r: 255, g: 0, b: 0 });
        assert!((h - 0.0).abs() < EPS && (s - 1.0).abs() < EPS && (l - 0.5).abs() < EPS);
        let (h, s, _l) = hsl(Rgb { r: 0, g: 255, b: 0 });
        assert!((h - 120.0).abs() < EPS && (s - 1.0).abs() < EPS);
        let (h, s, _l) = hsl(Rgb { r: 0, g: 0, b: 255 });
        assert!((h - 240.0).abs() < EPS && (s - 1.0).abs() < EPS);
        // #8B5CF6 (Tailwind violet-500) ≈ hsl(258.3, 89.5%, 66.3%).
        let (h, s, l) = hsl(Rgb {
            r: 0x8B,
            g: 0x5C,
            b: 0xF6,
        });
        assert!((h - 258.3).abs() < 0.1);
        assert!((s - 0.895).abs() < 0.01);
        assert!((l - 0.663).abs() < 0.01);
        // Achromatic gray: hue and saturation collapse to 0.
        let (h, s, _) = hsl(Rgb {
            r: 128,
            g: 128,
            b: 128,
        });
        assert!(h == 0.0 && s == 0.0);
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
