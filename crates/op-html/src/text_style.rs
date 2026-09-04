use jian_ops_schema::node::text::{FontStyleKind as TextFontStyle, FontWeight, TextNode};
use jian_ops_schema::style::FontStyleKind as SegmentFontStyle;

use crate::css::cascade::ComputedStyle;

use super::{decoration, solid_fill, SegStyle};

pub(super) fn segment_style(
    style: &ComputedStyle,
    href: Option<String>,
    fill_override: Option<&str>,
) -> SegStyle {
    SegStyle {
        weight: super::parse_weight(style.get("font-weight")),
        style: match style.get("font-style") {
            Some("italic" | "oblique") => Some(SegmentFontStyle::Italic),
            Some("normal") => Some(SegmentFontStyle::Normal),
            _ => None,
        },
        underline: decoration(style, "underline").then_some(true),
        strike: decoration(style, "line-through").then_some(true),
        fill: crate::mapper::text_paint_color(style, fill_override),
        href,
        font_size: Some(style.font_size as f32),
        font_family: style.get("font-family").map(str::to_string),
    }
}

/// Keep a single rich-text run's effective typography on the node as well.
/// Layout and fallback render paths consult node-level fields, while the rich
/// text painter consults the segment, so a one-run node must agree at both
/// levels. Multiple-run nodes deliberately retain their common block style.
pub(super) fn sync_single_run_style(text: &mut TextNode, style: &SegStyle) {
    if let Some(family) = style.font_family.as_ref() {
        text.font_family = Some(family.clone());
    }
    if let Some(size) = style.font_size {
        text.font_size = Some(f64::from(size));
    }
    if let Some(weight) = style.weight {
        text.font_weight = Some(FontWeight::Number(weight));
    }
    if let Some(font_style) = style.style.as_ref() {
        text.font_style = Some(match font_style {
            SegmentFontStyle::Normal => TextFontStyle::Normal,
            SegmentFontStyle::Italic => TextFontStyle::Italic,
        });
    }
    if let Some(fill) = style.fill.as_ref() {
        text.fill = Some(vec![solid_fill(fill.clone())]);
    }
    if style.underline.is_some() {
        text.underline = style.underline;
    }
    if style.strike.is_some() {
        text.strikethrough = style.strike;
    }
}
