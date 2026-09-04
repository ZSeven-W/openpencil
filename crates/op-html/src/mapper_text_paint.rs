use jian_ops_schema::style::PenFill;

use crate::color::parse_css_color;
use crate::css::cascade::ComputedStyle;

/// Collapse a paintable `background-clip: text` background to the single
/// colour the schema can put on glyphs. Per-glyph gradients are not supported,
/// so gradients contribute their first stop, matching snapshot import.
pub(crate) fn fill_glyph_color(fills: &[PenFill]) -> Option<String> {
    if fills.iter().any(|fill| {
        matches!(
            fill,
            PenFill::Image(_) | PenFill::MeshGradient(_) | PenFill::Shader(_)
        )
    }) {
        return None;
    }
    fills.iter().find_map(|fill| match fill {
        PenFill::Solid(body) => Some(body.color.clone()),
        PenFill::LinearGradient(body) => body.stops.first().map(|stop| stop.color.clone()),
        PenFill::RadialGradient(body) => body.stops.first().map(|stop| stop.color.clone()),
        PenFill::MeshGradient(_) | PenFill::Shader(_) | PenFill::Image(_) => None,
    })
}

pub(super) fn text_clip_glyph_color(style: &ComputedStyle, fills: &[PenFill]) -> Option<String> {
    background_clips_text(style).then(|| fill_glyph_color(fills))?
}

/// Remove a successfully representable text-clipped background from its box
/// and return the colour that should replace transparent descendant glyphs.
pub(crate) fn take_text_clip_fill(
    style: &ComputedStyle,
    fills: &mut Option<Vec<PenFill>>,
) -> Option<String> {
    let color = text_clip_glyph_color(style, fills.as_deref()?)?;
    *fills = None;
    Some(color)
}

pub(crate) fn text_paint_has_partial_alpha(style: &ComputedStyle) -> bool {
    text_paint_color(style, None).is_some_and(|color| {
        color.len() == 9 && !color.ends_with("00") && !color.to_ascii_lowercase().ends_with("ff")
    })
}

/// Resolve the colour that actually paints text. WebKit's text-fill property
/// wins over `color`; a fully transparent paint only takes the active clipped
/// background override, while transparent text elsewhere remains transparent.
pub(crate) fn text_paint_color(
    style: &ComputedStyle,
    fill_override: Option<&str>,
) -> Option<String> {
    let text_fill = style
        .get("text-fill-color")
        // Accept hand-built ComputedStyle values as well as parser-normalized
        // declarations used by ordinary import.
        .or_else(|| style.get("-webkit-text-fill-color"));
    let color = text_fill
        .and_then(|value| parse_text_fill(value, style))
        .or_else(|| style.get("color").and_then(parse_css_color))?;
    if is_fully_transparent(&color) {
        fill_override.map(str::to_string).or(Some(color))
    } else {
        Some(color)
    }
}

pub(super) fn is_fully_transparent(color: &str) -> bool {
    color.len() == 9 && color.ends_with("00")
}

pub(crate) fn background_clips_text(style: &ComputedStyle) -> bool {
    style.get("background-clip").is_some_and(|value| {
        super::syntax::split_top_level(value, ',')
            .into_iter()
            .all(|layer| layer.trim().eq_ignore_ascii_case("text"))
    })
}

fn parse_text_fill(value: &str, style: &ComputedStyle) -> Option<String> {
    if value.trim().eq_ignore_ascii_case("currentcolor") {
        style.get("color").and_then(parse_css_color)
    } else {
        parse_css_color(value)
    }
}
