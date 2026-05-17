//! Figma text mapper — ports `figma-text-mapper.ts`. Converts a
//! Figma TEXT node's style fields into canonical text props +
//! builds the `TextContent` (plain string or per-run styled
//! segments) from `characters` + `characterStyleIDs`.

use crate::color::figma_color_to_hex;
use crate::figma_types::FigColor;
use crate::kiwi::FigValue;
use jian_ops_schema::node::text::{TextAlign, TextAlignVertical, TextContent, TextGrowth};
use jian_ops_schema::style::{FontStyleKind, StyledTextSegment};

/// Mapped text props — spread onto a `TextNode` by the converter.
#[derive(Debug, Clone, PartialEq)]
pub struct TextProps {
    pub content: TextContent,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u32>,
    pub font_style: Option<FontStyleKind>,
    pub letter_spacing: Option<f64>,
    pub line_height: Option<f64>,
    pub text_align: Option<TextAlign>,
    pub text_align_vertical: Option<TextAlignVertical>,
    pub text_growth: Option<TextGrowth>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
}

/// Map a Figma TEXT node's fields to [`TextProps`].
pub fn map_figma_text_props(node: &FigValue) -> TextProps {
    let font_name = node.get("fontName");
    let style_lower = font_name
        .and_then(|f| f.get_str("style"))
        .unwrap_or("")
        .to_lowercase();

    let mut props = TextProps {
        content: apply_text_case(build_content(node), node.get_str("textCase")),
        font_family: font_name
            .and_then(|f| f.get_str("family"))
            .map(|s| s.to_string()),
        font_size: node.get_f64("fontSize"),
        font_weight: parse_font_weight(&style_lower),
        font_style: if style_lower.contains("italic") {
            Some(FontStyleKind::Italic)
        } else {
            None
        },
        letter_spacing: map_letter_spacing(node),
        line_height: map_line_height(node),
        text_align: map_text_align(node.get_str("textAlignHorizontal")),
        text_align_vertical: map_text_align_vertical(node.get_str("textAlignVertical")),
        text_growth: map_text_growth(node.get_str("textAutoResize")),
        underline: None,
        strikethrough: None,
    };
    match node.get_str("textDecoration") {
        Some("UNDERLINE") => props.underline = Some(true),
        Some("STRIKETHROUGH") => props.strikethrough = Some(true),
        _ => {}
    }
    props
}

/// Build the text content — plain string, or per-run styled segments
/// when `characterStyleIDs` + `styleOverrideTable` carry overrides.
fn build_content(node: &FigValue) -> TextContent {
    let Some(text_data) = node.get("textData") else {
        return TextContent::Plain(String::new());
    };
    let characters = text_data.get_str("characters").unwrap_or("");
    if characters.is_empty() {
        return TextContent::Plain(String::new());
    }
    let (Some(style_ids), Some(table)) = (
        text_data.get_array("characterStyleIDs"),
        text_data.get_array("styleOverrideTable"),
    ) else {
        return TextContent::Plain(characters.to_string());
    };
    if style_ids.is_empty() || table.is_empty() {
        return TextContent::Plain(characters.to_string());
    }

    // Figma indexes characterStyleIDs by UTF-16 code unit.
    let units: Vec<u16> = characters.encode_utf16().collect();
    let style_at = |i: usize| -> i32 {
        style_ids
            .get(i)
            .and_then(|v| v.as_f64())
            .map(|v| v as i32)
            .unwrap_or(-1)
    };

    let mut segments: Vec<StyledTextSegment> = Vec::new();
    let mut current = style_ids.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
    let mut seg_start = 0usize;
    for i in 1..=units.len() {
        let sid = style_at(i);
        if sid != current || i == units.len() {
            if i > seg_start {
                let seg_text = String::from_utf16_lossy(&units[seg_start..i]);
                segments.push(build_segment(&seg_text, current, table));
            }
            current = sid;
            seg_start = i;
        }
    }

    let has_override = segments.iter().any(|s| {
        s.font_family.is_some()
            || s.font_size.is_some()
            || s.font_weight.is_some()
            || s.fill.is_some()
    });
    if has_override {
        TextContent::Styled(segments)
    } else {
        TextContent::Plain(characters.to_string())
    }
}

/// Build one styled segment from a style-override-table entry.
fn build_segment(text: &str, style_id: i32, table: &[FigValue]) -> StyledTextSegment {
    let mut seg = StyledTextSegment {
        text: text.to_string(),
        font_family: None,
        font_size: None,
        font_weight: None,
        font_style: None,
        fill: None,
        underline: None,
        strikethrough: None,
        href: None,
    };
    if style_id <= 0 {
        return seg;
    }
    let idx = style_id as usize;
    let Some(ov) = table.get(idx).or_else(|| table.get(idx - 1)) else {
        return seg;
    };
    let font_name = ov.get("fontName");
    if let Some(family) = font_name.and_then(|f| f.get_str("family")) {
        if !family.is_empty() {
            seg.font_family = Some(family.to_string());
        }
    }
    if let Some(size) = ov.get_f64("fontSize") {
        if size != 0.0 {
            seg.font_size = Some(size as f32);
        }
    }
    let style_lower = font_name
        .and_then(|f| f.get_str("style"))
        .unwrap_or("")
        .to_lowercase();
    if let Some(w) = parse_font_weight(&style_lower) {
        seg.font_weight = Some(w);
    }
    if style_lower.contains("italic") {
        seg.font_style = Some(FontStyleKind::Italic);
    }
    match ov.get_str("textDecoration") {
        Some("UNDERLINE") => seg.underline = Some(true),
        Some("STRIKETHROUGH") => seg.strikethrough = Some(true),
        _ => {}
    }
    if let Some(color) = ov
        .get_array("fillPaints")
        .and_then(|p| p.first())
        .and_then(|p| p.get("color"))
        .and_then(FigColor::from_value)
    {
        seg.fill = Some(figma_color_to_hex(&color));
    }
    seg
}

/// Apply Figma `textCase` (UPPER / LOWER / TITLE) to the content.
fn apply_text_case(content: TextContent, text_case: Option<&str>) -> TextContent {
    let transform: fn(&str) -> String = match text_case {
        Some("UPPER") => |s| s.to_uppercase(),
        Some("LOWER") => |s| s.to_lowercase(),
        Some("TITLE") => title_case,
        _ => return content,
    };
    match content {
        TextContent::Plain(s) => TextContent::Plain(transform(&s)),
        TextContent::Styled(segs) => TextContent::Styled(
            segs.into_iter()
                .map(|mut s| {
                    s.text = transform(&s.text);
                    s
                })
                .collect(),
        ),
    }
}

/// Uppercase the first word character after every word boundary —
/// `\b\w` with ASCII `\w` (`[A-Za-z0-9_]`), matching JS.
fn title_case(s: &str) -> String {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut out = String::with_capacity(s.len());
    let mut prev_word = false;
    for c in s.chars() {
        if is_word(c) && !prev_word {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
        prev_word = is_word(c);
    }
    out
}

/// Figma font-style string → numeric weight. First substring match
/// wins; order matters (`extrabold` before `bold`, `extralight`
/// before `light`).
fn parse_font_weight(style: &str) -> Option<u32> {
    if style.contains("thin") || style.contains("hairline") {
        Some(100)
    } else if style.contains("extralight") || style.contains("ultralight") {
        Some(200)
    } else if style.contains("light") {
        Some(300)
    } else if style.contains("regular") || style.contains("normal") {
        Some(400)
    } else if style.contains("medium") {
        Some(500)
    } else if style.contains("semibold") || style.contains("demibold") {
        Some(600)
    } else if style.contains("extrabold") || style.contains("ultrabold") {
        Some(800)
    } else if style.contains("bold") {
        Some(700)
    } else if style.contains("black") || style.contains("heavy") {
        Some(900)
    } else {
        None
    }
}

/// Line height → unitless multiplier (3-decimal rounded).
fn map_line_height(node: &FigValue) -> Option<f64> {
    let lh = node.get("lineHeight")?;
    let value = lh.get_f64("value")?;
    if value == 0.0 {
        return None;
    }
    let font_size = node.get_f64("fontSize").unwrap_or(14.0);
    let mul = match lh.get_str("units")? {
        "PIXELS" => value / font_size,
        "PERCENT" => value / 100.0,
        "RAW" => value,
        _ => return None,
    };
    Some((mul * 1000.0).round() / 1000.0)
}

/// Letter spacing → pixels (percent units resolved against font size).
fn map_letter_spacing(node: &FigValue) -> Option<f64> {
    let ls = node.get("letterSpacing")?;
    let value = ls.get_f64("value")?;
    if value == 0.0 {
        return None;
    }
    match ls.get_str("units")? {
        "PIXELS" => Some(value),
        "PERCENT" => {
            let font_size = node.get_f64("fontSize").unwrap_or(14.0);
            Some(((font_size * value / 100.0) * 100.0).round() / 100.0)
        }
        _ => None,
    }
}

fn map_text_align(v: Option<&str>) -> Option<TextAlign> {
    match v {
        Some("LEFT") => Some(TextAlign::Left),
        Some("CENTER") => Some(TextAlign::Center),
        Some("RIGHT") => Some(TextAlign::Right),
        Some("JUSTIFIED") => Some(TextAlign::Justify),
        _ => None,
    }
}

fn map_text_align_vertical(v: Option<&str>) -> Option<TextAlignVertical> {
    match v {
        Some("TOP") => Some(TextAlignVertical::Top),
        Some("CENTER") => Some(TextAlignVertical::Middle),
        Some("BOTTOM") => Some(TextAlignVertical::Bottom),
        _ => None,
    }
}

fn map_text_growth(v: Option<&str>) -> Option<TextGrowth> {
    match v {
        Some("WIDTH_AND_HEIGHT") => Some(TextGrowth::Auto),
        Some("HEIGHT") => Some(TextGrowth::FixedWidth),
        Some("NONE") => Some(TextGrowth::FixedWidthHeight),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
