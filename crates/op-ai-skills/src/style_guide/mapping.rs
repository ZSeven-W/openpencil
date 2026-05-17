//! Style-switch mapping — port of `style-guide-mapping.ts`.
//!
//! Diffs two [`StyleGuideValues`] into a [`PropertyReplacement`] —
//! the `from → to` pairs the `replace_all_matching_properties` MCP
//! tool consumes to re-skin an existing design.

use crate::style_guide::parser::StyleGuideValues;

/// A `from → to` string replacement (colours, font families).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorReplacement {
    pub from: String,
    pub to: String,
}

/// A `from → to` corner-radius replacement (scalar px).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusReplacement {
    pub from: i64,
    pub to: i64,
}

/// The replacement bundle [`build_style_mapping`] produces. Colours
/// are routed to the channel they apply to (fill / text / stroke).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PropertyReplacement {
    pub fill_color: Vec<ColorReplacement>,
    pub text_color: Vec<ColorReplacement>,
    pub stroke_color: Vec<ColorReplacement>,
    pub font_family: Vec<ColorReplacement>,
    pub corner_radius: Vec<RadiusReplacement>,
}

/// Push a colour replacement when both ends are set and differ.
fn push_color(out: &mut Vec<ColorReplacement>, from: &Option<String>, to: &Option<String>) {
    if let (Some(f), Some(t)) = (from, to) {
        if f != t {
            out.push(ColorReplacement {
                from: f.clone(),
                to: t.clone(),
            });
        }
    }
}

/// Build the `from → to` property mapping between two style guides.
pub fn build_style_mapping(from: &StyleGuideValues, to: &StyleGuideValues) -> PropertyReplacement {
    let mut out = PropertyReplacement::default();

    // Fill colours — backgrounds, surfaces, accents.
    push_color(&mut out.fill_color, &from.colors.background, &to.colors.background);
    push_color(&mut out.fill_color, &from.colors.surface, &to.colors.surface);
    push_color(&mut out.fill_color, &from.colors.accent, &to.colors.accent);

    // Text colours.
    push_color(&mut out.text_color, &from.colors.text_primary, &to.colors.text_primary);
    push_color(&mut out.text_color, &from.colors.text_secondary, &to.colors.text_secondary);
    push_color(&mut out.text_color, &from.colors.text_muted, &to.colors.text_muted);

    // Border / stroke colour.
    push_color(&mut out.stroke_color, &from.colors.border, &to.colors.border);

    // Font families.
    push_color(&mut out.font_family, &from.typography.display_font, &to.typography.display_font);
    push_color(&mut out.font_family, &from.typography.body_font, &to.typography.body_font);
    push_color(&mut out.font_family, &from.typography.data_font, &to.typography.data_font);

    // Corner radii (scalar px).
    for (f, t) in [
        (from.radius.card, to.radius.card),
        (from.radius.button, to.radius.button),
    ] {
        if let (Some(f), Some(t)) = (f, t) {
            if f != t {
                out.corner_radius.push(RadiusReplacement { from: f, to: t });
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style_guide::parser::{StyleColors, StyleRadius, StyleTypography};

    fn values(bg: &str, accent: &str, font: &str, card: i64) -> StyleGuideValues {
        StyleGuideValues {
            colors: StyleColors {
                background: Some(bg.into()),
                accent: Some(accent.into()),
                ..Default::default()
            },
            typography: StyleTypography {
                display_font: Some(font.into()),
                ..Default::default()
            },
            radius: StyleRadius {
                card: Some(card),
                button: None,
            },
        }
    }

    #[test]
    fn maps_changed_properties_only() {
        let from = values("#000000", "#FF0000", "Inter", 8);
        let to = values("#FFFFFF", "#FF0000", "Roboto", 12);
        let m = build_style_mapping(&from, &to);
        // background changed → one fill replacement; accent unchanged.
        assert_eq!(m.fill_color.len(), 1);
        assert_eq!(m.fill_color[0].from, "#000000");
        assert_eq!(m.fill_color[0].to, "#FFFFFF");
        // display font changed.
        assert_eq!(m.font_family.len(), 1);
        assert_eq!(m.font_family[0].to, "Roboto");
        // card radius changed.
        assert_eq!(m.corner_radius, vec![RadiusReplacement { from: 8, to: 12 }]);
    }

    #[test]
    fn identical_guides_map_to_nothing() {
        let v = values("#000000", "#FF0000", "Inter", 8);
        let m = build_style_mapping(&v, &v);
        assert_eq!(m, PropertyReplacement::default());
    }
}
