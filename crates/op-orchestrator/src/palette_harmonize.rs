//! Palette hue harmonization.
//!
//! The semantic palette ([`crate::semantic_palette`]) ships a FIXED cool-slate
//! neutral ramp (`color-surface-2` = `#F1F5F9`, `color-bg-deep` = `#F8FAFC`,
//! borders in `#E2E8F0`/`#CBD5E1` …). Those blue-tinted grays clash badly on a
//! WARM design — a cream/orange food-delivery app gets a cool-gray search bar,
//! cool-gray category chips and a cool-gray avatar circle that read as "not part
//! of the palette" (the user flagged exactly this).
//!
//! The TS design-system generator tints its neutrals toward the brand hue; the
//! Rust port used the raw slate table. This module closes that gap: given the
//! page background color, it re-tints the neutral tokens to share the page's hue
//! while preserving each token's own lightness and chroma magnitude (so the ramp
//! stays as subtle as before — only its temperature changes). A neutral page
//! (white / true gray) has no hue to harmonize toward, so the slate ramp is left
//! untouched; a cool design therefore keeps its cool grays.

use jian_ops_schema::variable::{VariableDefinition, VariableScalar, VariableValue};
use std::collections::BTreeMap;

/// Neutral surface + border tokens that should follow the design temperature.
/// `color-surface` (`#FFFFFF`) is intentionally omitted — pure white reads clean
/// on any palette, and it has zero chroma to rotate anyway.
const NEUTRAL_TINT_TOKENS: &[&str] = &[
    "color-surface-2",
    "color-surface-3",
    "color-bg-deep",
    "color-border",
    "color-border-strong",
];

fn parse_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    if !h.is_ascii() || h.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f64;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f64;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f64;
    Some((r, g, b))
}

/// The page background's warm/cool direction — each channel's deviation from the
/// color's own gray (mean), normalized so the strongest channel is ±1. `None`
/// when the bg is effectively neutral (nothing to harmonize toward).
fn page_hue_direction(page_bg_hex: &str) -> Option<[f64; 3]> {
    let (r, g, b) = parse_rgb(page_bg_hex)?;
    let mean = (r + g + b) / 3.0;
    let dev = [r - mean, g - mean, b - mean];
    let max_abs = dev.iter().fold(0.0_f64, |m, &d| m.max(d.abs()));
    // A delta below ~3/255 is a true gray/white — keep the slate ramp.
    if max_abs < 3.0 {
        return None;
    }
    Some([dev[0] / max_abs, dev[1] / max_abs, dev[2] / max_abs])
}

/// Re-tint a neutral hex to the page hue `dir`, preserving its lightness (mean)
/// and chroma magnitude (max−min). A pure gray (chroma 0) returns unchanged.
/// Non-6-digit values (e.g. an `#RRGGBBAA` scrim) are returned untouched.
fn retint_neutral(hex: &str, dir: [f64; 3]) -> String {
    let Some((r, g, b)) = parse_rgb(hex) else {
        return hex.to_string();
    };
    let mean = (r + g + b) / 3.0;
    let chroma = r.max(g).max(b) - r.min(g).min(b);
    let half = chroma / 2.0;
    let ch = |i: usize| (mean + dir[i] * half).round().clamp(0.0, 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", ch(0), ch(1), ch(2))
}

fn retint_definition(def: &mut VariableDefinition, dir: [f64; 3]) {
    match &mut def.value {
        VariableValue::Themed(vals) => {
            for tv in vals.iter_mut() {
                if let VariableScalar::Str(s) = &mut tv.value {
                    *s = retint_neutral(s, dir);
                }
            }
        }
        VariableValue::Scalar(VariableScalar::Str(s)) => {
            *s = retint_neutral(s, dir);
        }
        _ => {}
    }
}

/// Re-tint the neutral surface + border tokens of `palette` to share
/// `page_bg_hex`'s hue. No-op when the page background is neutral.
pub fn harmonize_palette_neutrals(
    palette: &mut BTreeMap<String, VariableDefinition>,
    page_bg_hex: &str,
) {
    let Some(dir) = page_hue_direction(page_bg_hex) else {
        return;
    };
    for name in NEUTRAL_TINT_TOKENS {
        if let Some(def) = palette.get_mut(*name) {
            retint_definition(def, dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light_hex(def: &VariableDefinition) -> String {
        match &def.value {
            VariableValue::Themed(vals) => match &vals[0].value {
                VariableScalar::Str(s) => s.clone(),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn warm(r: u8, _g: u8, b: u8) -> bool {
        // Warm = red channel above blue channel.
        r > b
    }

    #[test]
    fn warm_page_warms_the_cool_neutrals() {
        let mut palette = crate::semantic_palette::palette_variables();
        // Cream warm page bg.
        harmonize_palette_neutrals(&mut palette, "#FFF8F0");
        // surface-2 was #F1F5F9 (cool: B>R) → now warm (R>B).
        let s2 = light_hex(palette.get("color-surface-2").unwrap());
        let (r, g, b) = parse_rgb(&s2).unwrap();
        assert!(
            warm(r as u8, g as u8, b as u8),
            "surface-2 should be warm after harmonizing to a cream page, got {s2}"
        );
        // Lightness (mean) preserved within a couple points of the original 245.
        let mean = (r + g + b) / 3.0;
        assert!(
            (mean - 245.0).abs() <= 2.0,
            "lightness preserved (~245), got mean {mean}"
        );
    }

    #[test]
    fn neutral_page_leaves_slate_untouched() {
        let mut palette = crate::semantic_palette::palette_variables();
        let before = light_hex(palette.get("color-surface-2").unwrap());
        // A true-gray page → nothing to harmonize toward.
        harmonize_palette_neutrals(&mut palette, "#FAFAFA");
        let after = light_hex(palette.get("color-surface-2").unwrap());
        assert_eq!(before, after, "neutral page keeps the cool slate ramp");
    }

    #[test]
    fn cool_page_keeps_cool_neutrals() {
        let mut palette = crate::semantic_palette::palette_variables();
        // A cool blue-gray page → neutrals stay cool (B≥R).
        harmonize_palette_neutrals(&mut palette, "#EEF2FF");
        let s2 = light_hex(palette.get("color-surface-2").unwrap());
        let (r, _g, b) = parse_rgb(&s2).unwrap();
        assert!(
            b as u8 >= r as u8,
            "cool page keeps cool neutrals, got {s2}"
        );
    }

    #[test]
    fn white_surface_token_is_not_in_the_tint_set() {
        // color-surface (#FFFFFF) must stay pure white.
        let mut palette = crate::semantic_palette::palette_variables();
        harmonize_palette_neutrals(&mut palette, "#FFF8F0");
        assert_eq!(light_hex(palette.get("color-surface").unwrap()), "#FFFFFF");
    }
}
