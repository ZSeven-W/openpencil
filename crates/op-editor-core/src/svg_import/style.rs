//! SVG presentation attributes: style-context inheritance, fill /
//! stroke resolution using the shared CSS color parser.

use super::*;

/// Merge inherited presentation properties with this element's values.
/// Inline `style="..."` takes precedence over matching attributes.
pub(super) fn merge_style_ctx(parent: &StyleCtx, attrs: &[(String, String)]) -> StyleCtx {
    let color = extract_style_or_attr(attrs, "color")
        .and_then(|raw| {
            if raw.trim().eq_ignore_ascii_case("currentcolor") {
                None
            } else {
                op_util::css_color::parse_css_color(&raw)
            }
        })
        .unwrap_or_else(|| parent.color.clone());
    StyleCtx {
        color,
        fill_opacity: extract_style_or_attr(attrs, "fill-opacity")
            .as_deref()
            .and_then(parse_paint_opacity)
            .unwrap_or(parent.fill_opacity),
        stroke_opacity: extract_style_or_attr(attrs, "stroke-opacity")
            .as_deref()
            .and_then(parse_paint_opacity)
            .unwrap_or(parent.stroke_opacity),
        fill: extract_style_or_attr(attrs, "fill").or_else(|| parent.fill.clone()),
        stroke: extract_style_or_attr(attrs, "stroke").or_else(|| parent.stroke.clone()),
        stroke_width: extract_style_or_attr(attrs, "stroke-width")
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(parent.stroke_width),
        fill_rule: extract_style_or_attr(attrs, "fill-rule")
            .as_deref()
            .and_then(parse_svg_fill_rule)
            .or(parent.fill_rule),
    }
}

pub(super) fn parse_svg_fill_rule(raw: &str) -> Option<PathFillRule> {
    match raw.trim() {
        value if value.eq_ignore_ascii_case("evenodd") => Some(PathFillRule::Evenodd),
        value if value.eq_ignore_ascii_case("nonzero") => Some(PathFillRule::Nonzero),
        _ => None,
    }
}

/// Look up `name` in an inline `style="..."` first, then fall back
/// to the named attribute. Mirrors `extractStyleOrAttr` from the TS
/// regex parser.
pub(super) fn extract_style_or_attr(attrs: &[(String, String)], name: &str) -> Option<String> {
    if let Some(style) = attrs.iter().find(|(k, _)| k == "style").map(|(_, v)| v) {
        // Naive CSS-ish split — values can't contain `;` because
        // SVG inline styles forbid it.
        for pair in style.split(';') {
            let trimmed = pair.trim();
            if let Some((k, v)) = trimmed.split_once(':') {
                if k.trim().eq_ignore_ascii_case(name) {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.to_string())
}

pub(super) fn resolve_svg_fill(attrs: &[(String, String)], ctx: &StyleCtx) -> Option<PenFill> {
    let raw = extract_style_or_attr(attrs, "fill").or_else(|| ctx.fill.clone());
    match raw.as_deref().map(str::trim) {
        None => Some(solid_fill([0, 0, 0, 255], ctx.fill_opacity)),
        Some(v) if v.eq_ignore_ascii_case("none") || v.eq_ignore_ascii_case("transparent") => None,
        Some(v) if v.to_ascii_lowercase().starts_with("url(") => {
            Some(solid_fill([0, 0, 0, 255], ctx.fill_opacity))
        }
        Some(v) => resolve_color(v, ctx).map(|rgba| solid_fill(rgba, ctx.fill_opacity)),
    }
}

pub(super) fn resolve_svg_stroke(
    attrs: &[(String, String)],
    ctx: &StyleCtx,
    scale: f64,
) -> Option<PenStroke> {
    let raw = extract_style_or_attr(attrs, "stroke").or_else(|| ctx.stroke.clone())?;
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("none")
        || raw.eq_ignore_ascii_case("transparent")
        || raw.to_ascii_lowercase().starts_with("url(")
    {
        return None;
    }
    let rgba = resolve_color(raw, ctx).unwrap_or([0, 0, 0, 255]);
    let width = (ctx.stroke_width * scale).max(0.0) as f32;
    if width <= 0.0 {
        return None;
    }
    Some(PenStroke {
        thickness: StrokeThickness::Uniform(width),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: Some(vec![solid_fill(rgba, ctx.stroke_opacity)]),
    })
}

fn solid_fill([r, g, b, a]: [u8; 4], opacity: f32) -> PenFill {
    let opacity = opacity * (f32::from(a) / 255.0);
    PenFill::Solid(SolidFillBody {
        color: format!("#{r:02x}{g:02x}{b:02x}"),
        explain: None,
        opacity: (opacity < 1.0).then_some(opacity),
        blend_mode: None,
    })
}

pub(super) fn set_node_stroke(node: &mut PenNode, stroke: PenStroke) {
    match node {
        PenNode::Frame(n) => n.container.stroke = Some(stroke),
        PenNode::Group(n) => n.container.stroke = Some(stroke),
        PenNode::Rectangle(n) => n.container.stroke = Some(stroke),
        PenNode::Ellipse(n) => n.stroke = Some(stroke),
        PenNode::Polygon(n) => n.stroke = Some(stroke),
        PenNode::Path(n) => n.stroke = Some(stroke),
        PenNode::Line(n) => n.stroke = Some(stroke),
        PenNode::TextInput(n) => n.stroke = Some(stroke),
        PenNode::TextArea(n) => n.stroke = Some(stroke),
        PenNode::Select(n) => n.stroke = Some(stroke),
        PenNode::Switch(n) => n.stroke = Some(stroke),
        PenNode::Checkbox(n) => n.stroke = Some(stroke),
        PenNode::Slider(n) => n.stroke = Some(stroke),
        PenNode::RadioGroup(n) => n.stroke = Some(stroke),
        PenNode::NumberInput(n) => n.stroke = Some(stroke),
        PenNode::Progress(n) => n.stroke = Some(stroke),
        PenNode::Tabs(n) => n.stroke = Some(stroke),
        PenNode::Text(_) | PenNode::IconFont(_) | PenNode::Image(_) | PenNode::Ref(_) => {}
    }
}

pub(super) fn clear_node_fill(node: &mut PenNode) {
    match node {
        PenNode::Frame(n) => n.container.fill = None,
        PenNode::Group(n) => n.container.fill = None,
        PenNode::Rectangle(n) => n.container.fill = None,
        PenNode::Ellipse(n) => n.fill = None,
        PenNode::Polygon(n) => n.fill = None,
        PenNode::Path(n) => n.fill = None,
        PenNode::Text(n) => n.fill = None,
        PenNode::TextInput(n) => n.fill = None,
        PenNode::IconFont(n) => n.fill = None,
        PenNode::TextArea(n) => n.fill = None,
        PenNode::Select(n) => n.fill = None,
        PenNode::Switch(n) => n.fill = None,
        PenNode::Checkbox(n) => n.fill = None,
        PenNode::Slider(n) => n.fill = None,
        PenNode::RadioGroup(n) => n.fill = None,
        PenNode::NumberInput(n) => n.fill = None,
        PenNode::Progress(n) => n.fill = None,
        PenNode::Tabs(n) => n.fill = None,
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => {}
    }
}

/// Resolve contextual color before delegating to the shared CSS parser.
fn resolve_color(raw: &str, ctx: &StyleCtx) -> Option<[u8; 4]> {
    let hex = if raw.trim().eq_ignore_ascii_case("currentcolor") {
        ctx.color.clone()
    } else {
        op_util::css_color::parse_css_color(raw)?
    };
    op_util::hex_color::parse_hex_rgba8(&hex, op_util::hex_color::HexOptions::default())
}

fn parse_paint_opacity(raw: &str) -> Option<f32> {
    let raw = raw.trim();
    let value = if let Some(percent) = raw.strip_suffix('%') {
        percent.parse::<f32>().ok()? / 100.0
    } else {
        raw.parse::<f32>().ok()?
    };
    value.is_finite().then(|| value.clamp(0.0, 1.0))
}
