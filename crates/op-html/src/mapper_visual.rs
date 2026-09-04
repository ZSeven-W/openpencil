use crate::import_warning::ImportWarning;
use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::node::ImageSrc;
use jian_ops_schema::style::{
    BlendMode, BlurBody, GradientStop, ImageFillBody, LinearGradientBody, PenEffect, PenFill,
    PenStroke, RadialGradientBody, ShadowBody, SolidFillBody, StrokeAlign, StrokeThickness,
};

use crate::color::parse_css_color;
use crate::css::cascade::ComputedStyle;
use crate::length::{parse_length, LengthCtx};

use crate::mapper::MapCtx;

#[path = "mapper_visual_syntax.rs"]
mod syntax;
use syntax::{
    exact_function_body, parse_gradient_angle, parse_radial_position, split_top_level,
    split_whitespace_top_level, strip_interpolation_method,
};

#[path = "mapper_background.rs"]
mod background;

#[path = "mapper_text_paint.rs"]
mod text_paint;
pub(crate) use text_paint::{background_clips_text, text_paint_has_partial_alpha};
pub(crate) use text_paint::{fill_glyph_color, take_text_clip_fill, text_paint_color};
use text_paint::{is_fully_transparent, text_clip_glyph_color};

/// `node` is the element's used border-box size, needed to resolve
/// `background-size` / `background-position` into an image-fill placement.
/// `definite` reports per axis whether that size is real or a stand-in for an
/// unresolved `auto`; a fabricated axis must not become a crop transform.
pub(super) fn map_fill(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    node: (f64, f64),
    definite: (bool, bool),
    text_clip_will_transfer: bool,
) -> Option<Vec<PenFill>> {
    let mut fills = Vec::new();
    let mut position_dropped = false;
    if let Some(images) = style.get("background-image") {
        for (index, image) in split_top_level(images, ',').into_iter().enumerate() {
            if image.eq_ignore_ascii_case("none") {
                continue;
            }
            let blend = layer_blend_mode(style, index, context);
            if let Some(mut gradient) = map_gradient(image, style, context) {
                set_fill_blend(&mut gradient, blend);
                fills.push(gradient);
                // A gradient paints across the whole box; it has no placement
                // the schema could offset.
                position_dropped |= background::position_is_dropped(style, index);
            } else if let Some(url) = extract_url(image) {
                if url.is_empty() {
                    context.warn_once(ImportWarning::BackgroundImageUrlEmpty);
                    continue;
                }
                let geometry = background::image_layer(style, index, node, definite, context);
                position_dropped |= geometry.position_dropped;
                fills.push(PenFill::Image(ImageFillBody {
                    url: ImageSrc::from(url),
                    mode: geometry.mode,
                    original_size: None,
                    transform: geometry.transform,
                    tile_scale: None,
                    explain: None,
                    opacity: None,
                    blend_mode: blend,
                    exposure: None,
                    contrast: None,
                    saturation: None,
                    temperature: None,
                    tint: None,
                    highlights: None,
                    shadows: None,
                }));
            } else {
                if image.to_ascii_lowercase().contains("conic-gradient(") {
                    context.warn_once(ImportWarning::ConicGradientIgnored);
                } else if resolve_color(image, style).is_none() {
                    context.warn_once(ImportWarning::BackgroundImageLayerUnsupported);
                }
                position_dropped |= background::position_is_dropped(style, index);
            }
        }
    }
    if let Some(value) = style.get("background-color") {
        if let Some(color) = resolve_color(value, style) {
            if !is_fully_transparent(&color) {
                let blend = mix_blend_mode(style, context);
                fills.push(solid_fill(color, blend));
            }
        } else if !value.eq_ignore_ascii_case("transparent") {
            context.warn_once(ImportWarning::BackgroundColorUnresolved);
        }
    }
    if position_dropped {
        context.warn_once(ImportWarning::BackgroundPositionDropped);
    }
    let text_clip_handled =
        text_clip_will_transfer && text_clip_glyph_color(style, &fills).is_some();
    warn_background_geometry(style, context, text_clip_handled);
    (!fills.is_empty()).then_some(fills)
}

pub(super) fn map_stroke(style: &ComputedStyle, context: &mut MapCtx<'_>) -> Option<PenStroke> {
    let sides = ["top", "right", "bottom", "left"];
    let styles = sides.map(|side| {
        style
            .get(&format!("border-{side}-style"))
            .or_else(|| style.get("border-style"))
            .unwrap_or("none")
    });
    let widths = super::box_model::border_widths(style, context).map(|width| width as f32);
    if widths.iter().all(|width| *width <= 0.0) {
        return None;
    }
    let colors = sides.map(|side| {
        style
            .get(&format!("border-{side}-color"))
            .or_else(|| style.get("border-color"))
            .unwrap_or("currentcolor")
    });
    let visible_side_count = widths.iter().filter(|width| **width > 0.0).count();
    let resolved_colors: Vec<_> = colors
        .iter()
        .enumerate()
        .filter(|(index, _)| widths[*index] > 0.0)
        .filter_map(|(_, value)| resolve_color(value, style))
        .collect();
    let color = resolved_colors
        .iter()
        .find(|color| !is_fully_transparent(color))
        .cloned()
        .or_else(|| (resolved_colors.len() < visible_side_count).then(|| "#000000".into()))?;
    if resolved_colors.iter().any(|candidate| candidate != &color) {
        context.warn_once(ImportWarning::BorderColorsApproximated);
    }
    let uniform = widths.iter().all(|width| *width == widths[0]);
    let thickness = if uniform {
        StrokeThickness::Uniform(widths[0])
    } else {
        StrokeThickness::PerSide(widths)
    };
    let mut visible_styles = styles
        .iter()
        .enumerate()
        .filter(|(index, _)| widths[*index] > 0.0)
        .map(|(_, value)| value.to_ascii_lowercase());
    let style_name = visible_styles.next().unwrap_or_else(|| "solid".into());
    if visible_styles.any(|candidate| candidate != style_name) {
        context.warn_once(ImportWarning::BorderStylesApproximated);
    }
    let dash_pattern = match style_name.to_ascii_lowercase().as_str() {
        "dotted" => Some(vec![1.0, widths.iter().copied().fold(1.0, f32::max) * 2.0]),
        "dashed" => Some(vec![
            widths.iter().copied().fold(1.0, f32::max) * 3.0,
            widths.iter().copied().fold(1.0, f32::max) * 2.0,
        ]),
        "double" | "groove" | "ridge" | "inset" | "outset" => {
            context.warn_once(ImportWarning::BorderStyleComplex);
            None
        }
        "solid" => None,
        _ => {
            context.warn_once(ImportWarning::BorderStyleUnsupported);
            None
        }
    };
    Some(PenStroke {
        thickness,
        // CSS borders occupy the inside edge of the border box. Jian strokes
        // are centered by default, so make the inward alignment explicit.
        align: Some(StrokeAlign::Inside),
        join: None,
        cap: None,
        dash_pattern,
        dash_offset: None,
        fill: Some(vec![solid_fill(color, None)]),
    })
}

pub(super) fn map_corner_radius(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    width_reference: f64,
    height_reference: f64,
) -> Option<CornerRadius> {
    let reference = width_reference.min(height_reference).max(0.0);
    let names = [
        "border-top-left-radius",
        "border-top-right-radius",
        "border-bottom-right-radius",
        "border-bottom-left-radius",
    ];
    let fallback = style.get("border-radius");
    let mut any = false;
    let mut radii = [0.0; 4];
    for (index, name) in names.iter().enumerate() {
        let Some(value) = style.get(name).or(fallback) else {
            continue;
        };
        any = true;
        if value.contains('/') {
            context.warn_once(ImportWarning::BorderRadiusElliptical);
        }
        if let Some(radius) = parse_length(
            value.split('/').next().unwrap_or(value),
            &length_context(style, context),
        ) {
            radii[index] = radius.resolve(reference).max(0.0);
        } else {
            context.warn_once(ImportWarning::BorderRadiusUnsupported);
        }
    }
    if !any {
        return None;
    }
    if radii.iter().all(|radius| *radius == radii[0]) {
        Some(CornerRadius::Uniform(radii[0]))
    } else {
        Some(CornerRadius::PerCorner(radii))
    }
}

pub(super) fn map_effects(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
) -> Option<Vec<PenEffect>> {
    let mut effects = Vec::new();
    if let Some(shadows) = style.get("box-shadow") {
        for shadow in split_top_level(shadows, ',') {
            if shadow.eq_ignore_ascii_case("none") {
                continue;
            }
            if let Some(shadow) = map_shadow(shadow, style, context, true) {
                effects.push(PenEffect::Shadow(shadow));
            } else {
                context.warn_once(ImportWarning::BoxShadowLayerUnsupported);
            }
        }
    }
    if let Some(filter) = style.get("filter") {
        map_filter(filter, false, style, context, &mut effects);
    }
    if let Some(filter) = style.get("backdrop-filter") {
        map_filter(filter, true, style, context, &mut effects);
    }
    (!effects.is_empty()).then_some(effects)
}

/// CSS `text-shadow` → a text node's shadow effect.
///
/// `text-shadow` shares `box-shadow`'s `<offset-x> <offset-y> <blur> <color>`
/// grammar minus `spread` and `inset`, so it reuses the same parser — with the
/// stricter grammar enforced: a fourth length or an `inset` keyword makes the
/// layer invalid CSS, which a browser drops rather than paints. The schema
/// keeps one effect list per text node and the text painter honours a single
/// shadow, so only the first layer survives — CSS paints the first layer on top,
/// which makes it the one worth keeping.
pub(crate) fn map_text_shadow(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
) -> Option<Vec<PenEffect>> {
    let value = style.get("text-shadow")?;
    if value.trim().eq_ignore_ascii_case("none") {
        return None;
    }
    let layers = split_top_level(value, ',');
    let mut shadow = None;
    for layer in &layers {
        match map_shadow(layer, style, context, false) {
            Some(parsed) if shadow.is_none() => shadow = Some(parsed),
            Some(_) => {}
            None => context.warn_once(ImportWarning::TextShadowLayerUnsupported),
        }
    }
    let shadow = shadow?;
    if layers.len() > 1 {
        context.warn_once(ImportWarning::TextShadowExtraLayersIgnored);
    }
    Some(vec![PenEffect::Shadow(shadow)])
}

/// Report a `text-shadow` authored on an inline element inside a text block.
///
/// The schema carries effects per TEXT NODE, not per styled segment, so only
/// the block's own shadow reaches the document; a `<span style="text-shadow:…">`
/// inside a paragraph is silently dropped without this. `text-shadow` inherits,
/// so a segment that merely repeats its parent's value is not a drop.
pub(crate) fn warn_segment_text_shadow(
    context: &mut MapCtx<'_>,
    parent: &ComputedStyle,
    segment: &ComputedStyle,
) {
    let value = segment.get("text-shadow");
    if value != parent.get("text-shadow")
        && value.is_some_and(|value| !value.trim().eq_ignore_ascii_case("none"))
    {
        context.warn_once(ImportWarning::TextShadowOnInlineIgnored);
    }
}

fn map_gradient(value: &str, style: &ComputedStyle, context: &mut MapCtx<'_>) -> Option<PenFill> {
    let value = value.trim();
    let (radial, repeating, body) =
        if let Some(body) = exact_function_body(value, "linear-gradient") {
            (false, false, body)
        } else if let Some(body) = exact_function_body(value, "repeating-linear-gradient") {
            (false, true, body)
        } else if let Some(body) = exact_function_body(value, "radial-gradient") {
            (true, false, body)
        } else if let Some(body) = exact_function_body(value, "repeating-radial-gradient") {
            (true, true, body)
        } else {
            return None;
        };
    let mut parts = split_top_level(body, ',');
    let mut angle = 180.0;
    let mut center = (None, None);
    let mut radius = None;
    if radial {
        if parts
            .first()
            .is_some_and(|part| parse_color_stops(part, style).is_none())
        {
            if let Some(prelude) = parts.first() {
                (center.0, center.1, radius) = parse_radial_prelude(prelude, context);
            }
            parts.remove(0);
        }
    } else if let Some(prelude) = parts.first().copied() {
        if parse_color_stops(prelude, style).is_none() {
            let (direction, interpolation) = strip_interpolation_method(prelude);
            if interpolation {
                context.warn_once(ImportWarning::GradientInterpolationIgnored);
            }
            if direction.is_empty() {
                // The default CSS direction is top to bottom.
            } else if let Some(parsed) = parse_gradient_angle(direction) {
                angle = parsed;
            } else {
                context.warn_once(ImportWarning::LinearGradientDirectionUnsupported);
            }
            parts.remove(0);
        }
    }
    let mut stops = Vec::new();
    for part in parts {
        if let Some(mut parsed) = parse_color_stops(part, style) {
            stops.append(&mut parsed);
        } else if parse_stop_position(part).is_some() {
            context.warn_once(ImportWarning::GradientColorHintsIgnored);
        } else {
            context.warn_once(ImportWarning::GradientColorStopUnsupported);
        }
    }
    if stops.len() < 2 {
        context.warn_once(ImportWarning::GradientTooFewStops);
        return None;
    }
    distribute_stops(&mut stops);
    if repeating {
        context.warn_once(ImportWarning::GradientRepeatingApproximated);
    }
    if stops.iter().any(|stop| !(0.0..=1.0).contains(&stop.offset)) {
        context.warn_once(ImportWarning::GradientStopsClamped);
        stops
            .iter_mut()
            .for_each(|stop| stop.offset = stop.offset.clamp(0.0, 1.0));
    }
    if radial {
        Some(PenFill::RadialGradient(RadialGradientBody {
            cx: center.0,
            cy: center.1,
            radius,
            stops: stops
                .into_iter()
                .map(|stop| GradientStop {
                    offset: stop.offset,
                    color: stop.color,
                })
                .collect(),
            explain: None,
            opacity: None,
            blend_mode: None,
        }))
    } else {
        Some(PenFill::LinearGradient(LinearGradientBody {
            // CSS angles and canonical `.op` angles share the same
            // convention: 0deg points bottom-to-top, 90deg left-to-right,
            // and the CSS default is 180deg (top-to-bottom). The renderer
            // performs the screen-coordinate projection itself.
            angle: Some(angle.rem_euclid(360.0)),
            stops: stops
                .into_iter()
                .map(|stop| GradientStop {
                    offset: stop.offset,
                    color: stop.color,
                })
                .collect(),
            explain: None,
            opacity: None,
            blend_mode: None,
        }))
    }
}

#[derive(Clone)]
struct ParsedStop {
    color: String,
    offset: f32,
    explicit: bool,
}

fn parse_color_stops(value: &str, style: &ComputedStyle) -> Option<Vec<ParsedStop>> {
    let tokens = split_whitespace_top_level(value);
    let color = resolve_color(tokens.first()?, style)?;
    let positions: Vec<_> = tokens[1..]
        .iter()
        .take(2)
        .map(|position| parse_stop_position(position))
        .collect::<Option<_>>()?;
    if tokens.len() > 3 {
        return None;
    }
    if positions.is_empty() {
        return Some(vec![ParsedStop {
            color,
            offset: 0.0,
            explicit: false,
        }]);
    }
    Some(
        positions
            .into_iter()
            .map(|offset| ParsedStop {
                color: color.clone(),
                offset,
                explicit: true,
            })
            .collect(),
    )
}

fn parse_stop_position(value: &str) -> Option<f32> {
    let value = value.trim();
    if value == "0" {
        return Some(0.0);
    }
    value
        .strip_suffix('%')?
        .trim()
        .parse::<f32>()
        .ok()
        .map(|percent| percent / 100.0)
}

fn distribute_stops(stops: &mut [ParsedStop]) {
    if stops.is_empty() {
        return;
    }
    if !stops[0].explicit {
        stops[0].offset = 0.0;
        stops[0].explicit = true;
    }
    let last = stops.len() - 1;
    if !stops[last].explicit {
        stops[last].offset = 1.0;
        stops[last].explicit = true;
    }
    let mut anchor = 0;
    while anchor < last {
        let next = (anchor + 1..=last)
            .find(|index| stops[*index].explicit)
            .unwrap_or(last);
        let start = stops[anchor].offset;
        let end = stops[next].offset.max(start);
        let span = (next - anchor) as f32;
        for (offset, stop) in stops[anchor + 1..next].iter_mut().enumerate() {
            stop.offset = start + (end - start) * (offset as f32 + 1.0) / span;
        }
        stops[next].offset = end;
        anchor = next;
    }
}

/// `spread_and_inset_allowed` is the `box-shadow` grammar; `text-shadow` uses
/// the same production without the fourth length and without `inset`, and a
/// layer that uses either is invalid rather than approximable.
fn map_shadow(
    value: &str,
    style: &ComputedStyle,
    context: &MapCtx<'_>,
    spread_and_inset_allowed: bool,
) -> Option<ShadowBody> {
    let mut inner = false;
    let mut color = resolve_color("currentcolor", style).unwrap_or_else(|| "#000000".into());
    let mut lengths = Vec::new();
    for token in split_whitespace_top_level(value) {
        if token.eq_ignore_ascii_case("inset") {
            if inner || !spread_and_inset_allowed {
                return None;
            }
            inner = true;
        } else if let Some(parsed) = resolve_color(token, style) {
            color = parsed;
        } else if let Some(length) = length_px(token, style, context) {
            lengths.push(length as f32);
        } else {
            return None;
        }
    }
    let maximum_lengths = if spread_and_inset_allowed { 4 } else { 3 };
    if !(2..=maximum_lengths).contains(&lengths.len()) {
        return None;
    }
    Some(ShadowBody {
        inner: inner.then_some(true),
        visible: None,
        offset_x: lengths[0],
        offset_y: lengths[1],
        blur: lengths.get(2).copied().unwrap_or(0.0).max(0.0),
        spread: lengths.get(3).copied().unwrap_or(0.0),
        color,
    })
}

fn map_filter(
    value: &str,
    backdrop: bool,
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    effects: &mut Vec<PenEffect>,
) {
    if value.trim().eq_ignore_ascii_case("none") {
        return;
    }
    for function in split_whitespace_top_level(value) {
        if let Some(body) = exact_function_body(function, "blur") {
            if let Some(radius) = length_px(body, style, context) {
                let blur = BlurBody {
                    radius: radius.max(0.0) as f32,
                    visible: None,
                };
                effects.push(if backdrop {
                    PenEffect::BackgroundBlur(blur)
                } else {
                    PenEffect::Blur(blur)
                });
            } else {
                context.warn_once(ImportWarning::BlurRadiusUnsupported);
            }
        } else if !backdrop {
            if let Some(body) = exact_function_body(function, "drop-shadow") {
                if let Some(shadow) = map_shadow(body, style, context, true) {
                    effects.push(PenEffect::Shadow(shadow));
                } else {
                    context.warn_once(ImportWarning::FilterDropShadowUnsupported);
                }
            } else {
                context.warn_once(ImportWarning::FilterFunctionUnsupported);
            }
        } else {
            context.warn_once(ImportWarning::BackdropFilterUnsupported);
        }
    }
}

fn resolve_color(value: &str, style: &ComputedStyle) -> Option<String> {
    resolve_color_inner(value, style, 0)
}

fn resolve_color_inner(value: &str, style: &ComputedStyle, depth: usize) -> Option<String> {
    if depth > 16 {
        return None;
    }
    let value = value.trim();
    if value.eq_ignore_ascii_case("currentcolor") {
        style
            .get("color")
            .filter(|color| !color.eq_ignore_ascii_case("currentcolor"))
            .and_then(|color| resolve_color_inner(color, style, depth + 1))
    } else if let Some(color) = parse_css_color(value) {
        Some(color)
    } else if let Some(body) = exact_function_body(value, "var") {
        let parts = split_top_level(body, ',');
        let name = parts.first()?.trim();
        style
            .get(name)
            .and_then(|color| resolve_color_inner(color, style, depth + 1))
            .or_else(|| {
                parts
                    .get(1)
                    .and_then(|fallback| resolve_color_inner(fallback, style, depth + 1))
            })
    } else {
        None
    }
}

fn length_px(value: &str, style: &ComputedStyle, context: &MapCtx<'_>) -> Option<f64> {
    Some(parse_length(value, &length_context(style, context))?.resolve(context.opts.viewport_width))
}

fn length_context(style: &ComputedStyle, context: &MapCtx<'_>) -> LengthCtx {
    LengthCtx {
        font_size: style.font_size,
        root_font_size: context.opts.base_font_size,
        viewport_w: context.opts.viewport_width,
        viewport_h: context.opts.viewport_height(),
    }
}

fn solid_fill(color: String, blend_mode: Option<BlendMode>) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color,
        explain: None,
        opacity: None,
        blend_mode,
    })
}

fn set_fill_blend(fill: &mut PenFill, blend: Option<BlendMode>) {
    match fill {
        PenFill::Solid(body) => body.blend_mode = blend,
        PenFill::LinearGradient(body) => body.blend_mode = blend,
        PenFill::RadialGradient(body) => body.blend_mode = blend,
        PenFill::MeshGradient(body) => body.blend_mode = blend,
        PenFill::Shader(body) => body.blend_mode = blend,
        PenFill::Image(body) => body.blend_mode = blend,
    }
}

fn layer_blend_mode(
    style: &ComputedStyle,
    index: usize,
    context: &mut MapCtx<'_>,
) -> Option<BlendMode> {
    if let Some(value) = style
        .get("background-blend-mode")
        .and_then(|value| layer_value(value, index))
    {
        let blend = map_blend_mode(value);
        if blend.is_none() {
            context.warn_once(ImportWarning::BackgroundBlendModeUnsupported);
        }
        blend
    } else {
        mix_blend_mode(style, context)
    }
}

fn mix_blend_mode(style: &ComputedStyle, context: &mut MapCtx<'_>) -> Option<BlendMode> {
    let value = style.get("mix-blend-mode")?;
    if !value.eq_ignore_ascii_case("normal") {
        context.warn_once(ImportWarning::MixBlendModeOnFills);
    }
    let blend = map_blend_mode(value);
    if blend.is_none() {
        context.warn_once(ImportWarning::MixBlendModeUnsupported);
    }
    blend
}

pub(crate) fn map_blend_mode(value: &str) -> Option<BlendMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "normal" => Some(BlendMode::Normal),
        "darken" => Some(BlendMode::Darken),
        "multiply" => Some(BlendMode::Multiply),
        "screen" => Some(BlendMode::Screen),
        "overlay" => Some(BlendMode::Overlay),
        "lighten" => Some(BlendMode::Lighten),
        "difference" => Some(BlendMode::Difference),
        "hue" => Some(BlendMode::Hue),
        "saturation" => Some(BlendMode::Saturation),
        "color" => Some(BlendMode::Color),
        "luminosity" => Some(BlendMode::Luminosity),
        "soft-light" => Some(BlendMode::SoftLight),
        "color-dodge" => Some(BlendMode::ColorDodge),
        "color-burn" => Some(BlendMode::ColorBurn),
        "hard-light" => Some(BlendMode::HardLight),
        "exclusion" => Some(BlendMode::Exclusion),
        _ => None,
    }
}

fn layer_value(value: &str, index: usize) -> Option<&str> {
    let layers = split_top_level(value, ',');
    (!layers.is_empty()).then(|| layers[index % layers.len()])
}

fn warn_background_geometry(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    text_clip_handled: bool,
) {
    for (name, default) in [
        ("background-origin", "padding-box"),
        ("background-clip", "border-box"),
        ("background-attachment", "scroll"),
    ] {
        if !(name == "background-clip" && text_clip_handled)
            && style
                .get(name)
                .is_some_and(|value| !layers_match(value, &[default]))
        {
            context.warn_once(ImportWarning::PropertyNotRepresentable {
                property: name.to_string(),
            });
        }
    }
    if style
        .get("background-image")
        .is_some_and(|value| value.to_ascii_lowercase().contains("gradient("))
        && background::gradient_size_is_dropped(style)
    {
        context.warn_once(ImportWarning::GradientBackgroundSizeIgnored);
    }
}

fn layers_match(value: &str, choices: &[&str]) -> bool {
    split_top_level(value, ',')
        .into_iter()
        .all(|layer| matches_ignore_ascii_case(layer.trim(), choices))
}

fn parse_radial_prelude(
    value: &str,
    context: &mut MapCtx<'_>,
) -> (Option<f32>, Option<f32>, Option<f32>) {
    let (value, interpolation) = strip_interpolation_method(value);
    if interpolation {
        context.warn_once(ImportWarning::GradientInterpolationIgnored);
    }
    let lower = value.to_ascii_lowercase();
    let (geometry, position) = lower.find(" at ").map_or((lower.as_str(), None), |index| {
        (&lower[..index], Some(&lower[index + 4..]))
    });
    let (cx, cy) = position
        .and_then(parse_radial_position)
        .map_or((None, None), |(x, y)| (Some(x), Some(y)));
    if position.is_some() && cx.is_none() {
        context.warn_once(ImportWarning::RadialGradientPositionUnsupported);
    }
    let mut radius = None;
    for token in geometry.split_whitespace() {
        match token {
            "" | "circle" => {}
            "ellipse" => context.warn_once(ImportWarning::RadialGradientElliptical),
            "closest-side" | "closest-corner" | "farthest-side" | "farthest-corner" => {
                context.warn_once(ImportWarning::RadialGradientExtentApproximated)
            }
            _ => {
                if let Some(percent) = token
                    .strip_suffix('%')
                    .and_then(|number| number.parse::<f32>().ok())
                {
                    radius = Some((percent / 100.0).max(0.0));
                } else {
                    context.warn_once(ImportWarning::RadialGradientSizeUnsupported);
                }
            }
        }
    }
    (cx, cy, radius)
}

fn extract_url(value: &str) -> Option<String> {
    Some(
        exact_function_body(value, "url")?
            .trim()
            .trim_matches(['\'', '"'])
            .to_string(),
    )
}

fn matches_ignore_ascii_case(value: &str, choices: &[&str]) -> bool {
    choices
        .iter()
        .any(|choice| value.eq_ignore_ascii_case(choice))
}

#[cfg(test)]
#[path = "mapper_visual_tests.rs"]
mod tests;
