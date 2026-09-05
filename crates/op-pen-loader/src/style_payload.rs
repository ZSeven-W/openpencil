//! Canonical style/fill conversion helpers for payload nodes.

use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::node::{ImageFitMode, ImageNode};
use jian_ops_schema::style::{
    ImageFillMode, PenFill, PenStroke, ShaderUniformValue, StrokeAlign, StrokeThickness,
};

use crate::payload::{
    GradientPayload, GradientStopPayload, ImageAdjustmentPayload, NodePayload, ShaderPayload,
    ShaderUniformPayload, StrokePayload,
};

type ImageFillPayload = (
    jian_ops_schema::node::ImageSrc,
    String,
    Option<[f32; 6]>,
    Option<[f32; 2]>,
    Option<f32>,
    Option<ImageAdjustmentPayload>,
);

pub(crate) fn base_payload(base: &PenNodeBase, kind: &str) -> NodePayload {
    NodePayload {
        id: base.id.clone(),
        schema_id: base.id.clone(),
        kind: kind.to_string(),
        name: base.name.clone().unwrap_or_else(|| base.id.clone()),
        x: base.x.unwrap_or(0.0) as f32,
        y: base.y.unwrap_or(0.0) as f32,
        w: 0.0,
        h: 0.0,
        fill: None,
        fill_layers: Vec::new(),
        stroke: None,
        text: None,
        font_family: String::new(),
        rotation: (base.rotation.unwrap_or(0.0) as f32).to_radians(),
        flip_x: base.flip_x.unwrap_or(false),
        flip_y: base.flip_y.unwrap_or(false),
        opacity: base_opacity(base),
        blend_mode: base.blend_mode.clone(),
        corner_radius: 0.0,
        corner_radii: None,
        clip_content: false,
        arc_start_angle: None,
        arc_sweep_angle: None,
        arc_inner_radius: None,
        polygon_sides: 3,
        hidden: !base.visible.unwrap_or(true),
        locked: base.locked.unwrap_or(false),
        collapsed: false,
        fill_type: "solid".into(),
        gradient: None,
        shader: None,
        points: Vec::new(),
        path_anchors: Vec::new(),
        path_closed: false,
        is_mask: base.mask_type.is_some(),
        mask_type: base.mask_type,
        even_odd_fill: false,
        svg_path: None,
        font_size: 0.0,
        font_weight: 0,
        italic: false,
        underline: false,
        strikethrough: false,
        text_runs: Vec::new(),
        line_height: 0.0,
        letter_spacing: 0.0,
        text_align: String::new(),
        text_vertical_align: String::new(),
        text_wrap: false,
        effects: Vec::new(),
        layer_blur: None,
        background_blur: None,
        image_src: None,
        video: None,
        image_fit: None,
        image_blend_mode: None,
        image_transform: None,
        image_original_size: None,
        image_tile_scale: None,
        image_adjustments: None,
        widget: None,
        children: Vec::new(),
    }
}

fn base_opacity(base: &PenNodeBase) -> f32 {
    match &base.opacity {
        Some(jian_ops_schema::node::NumberOrExpression::Number(n)) => (*n as f32).clamp(0.0, 1.0),
        _ => 1.0,
    }
}

pub(crate) fn apply_container_style(
    p: &mut NodePayload,
    fill: Option<&[PenFill]>,
    stroke: Option<&PenStroke>,
    corner_radius: Option<&CornerRadius>,
) {
    assign_first_fill(p, fill);
    p.stroke = stroke_to_payload(stroke);
    p.corner_radii = match corner_radius {
        Some(CornerRadius::PerCorner(corners)) => Some(corners.map(|radius| radius as f32)),
        _ => None,
    };
    p.corner_radius = match corner_radius {
        Some(CornerRadius::Uniform(r)) => *r as f32,
        Some(CornerRadius::PerCorner(corners)) => {
            corners.iter().copied().fold(0.0_f64, f64::max) as f32
        }
        None => 0.0,
    };
}

pub(crate) fn assign_first_fill(p: &mut NodePayload, fills: Option<&[PenFill]>) {
    // Preserve the complete canonical stack for renderers that understand
    // layered fills. Keep populating the historical primary-fill projection
    // below so serialized payloads remain consumable by older clients.
    p.fill_layers = fills.unwrap_or_default().to_vec();
    p.fill = first_solid_color(fills);
    p.fill_type = first_fill_type(fills);
    p.gradient = first_gradient(fills);
    p.shader = first_shader(fills);
    if let Some((url, fit, transform, original_size, tile_scale, adjustments)) =
        first_image_fill(fills)
    {
        p.image_src = Some(url);
        p.image_fit = Some(fit);
        p.image_transform = transform;
        p.image_original_size = original_size;
        p.image_tile_scale = tile_scale;
        p.image_adjustments = adjustments;
    }
}

fn first_image_fill(fills: Option<&[PenFill]>) -> Option<ImageFillPayload> {
    let body = fills?.first().and_then(|f| match f {
        PenFill::Image(b) => Some(b),
        _ => None,
    })?;
    if body.url.trim().is_empty() {
        None
    } else {
        Some((
            // Arc bump — never copy the data-URL bytes into the payload.
            body.url.clone(),
            image_fill_mode_to_payload(body.mode.as_ref()),
            body.transform
                .as_ref()
                .map(|m| [m.m00, m.m01, m.m02, m.m10, m.m11, m.m12]),
            body.original_size.as_ref().and_then(|size| {
                (size.width.is_finite()
                    && size.height.is_finite()
                    && size.width > 0.0
                    && size.height > 0.0)
                    .then_some([size.width, size.height])
            }),
            body.tile_scale
                .filter(|scale| scale.is_finite() && *scale > 0.0),
            image_fill_adjustments(body),
        ))
    }
}

pub(crate) fn image_fill_mode_to_payload(mode: Option<&ImageFillMode>) -> String {
    match mode {
        Some(ImageFillMode::Fit) => "fit",
        Some(ImageFillMode::Crop) => "crop",
        Some(ImageFillMode::Tile) => "tile",
        Some(ImageFillMode::Stretch) => "stretch",
        Some(ImageFillMode::Fill) | None => "fill",
    }
    .into()
}

pub(crate) fn image_node_fit_to_payload(mode: &ImageFitMode) -> String {
    match mode {
        ImageFitMode::Fit => "fit",
        ImageFitMode::Crop => "crop",
        ImageFitMode::Tile => "tile",
        ImageFitMode::Fill => "fill",
    }
    .into()
}

pub(crate) fn image_fill_adjustments(
    body: &jian_ops_schema::style::ImageFillBody,
) -> Option<ImageAdjustmentPayload> {
    adjustments_payload(
        body.exposure,
        body.contrast,
        body.saturation,
        body.temperature,
        body.tint,
        body.highlights,
        body.shadows,
    )
}

pub(crate) fn image_node_adjustments(n: &ImageNode) -> Option<ImageAdjustmentPayload> {
    adjustments_payload(
        n.exposure.map(|v| v as f32),
        n.contrast.map(|v| v as f32),
        n.saturation.map(|v| v as f32),
        n.temperature.map(|v| v as f32),
        n.tint.map(|v| v as f32),
        n.highlights.map(|v| v as f32),
        n.shadows.map(|v| v as f32),
    )
}

#[allow(clippy::too_many_arguments)]
fn adjustments_payload(
    exposure: Option<f32>,
    contrast: Option<f32>,
    saturation: Option<f32>,
    temperature: Option<f32>,
    tint: Option<f32>,
    highlights: Option<f32>,
    shadows: Option<f32>,
) -> Option<ImageAdjustmentPayload> {
    let payload = ImageAdjustmentPayload {
        exposure: exposure.unwrap_or(0.0),
        contrast: contrast.unwrap_or(0.0),
        saturation: saturation.unwrap_or(0.0),
        temperature: temperature.unwrap_or(0.0),
        tint: tint.unwrap_or(0.0),
        highlights: highlights.unwrap_or(0.0),
        shadows: shadows.unwrap_or(0.0),
    };
    (payload.exposure != 0.0
        || payload.contrast != 0.0
        || payload.saturation != 0.0
        || payload.temperature != 0.0
        || payload.tint != 0.0
        || payload.highlights != 0.0
        || payload.shadows != 0.0)
        .then_some(payload)
}

fn first_gradient(fills: Option<&[PenFill]>) -> Option<GradientPayload> {
    gradient_payload(fills?.first()?)
}

/// Resolve one canonical gradient layer without projecting away siblings.
pub(crate) fn gradient_payload(fill: &PenFill) -> Option<GradientPayload> {
    match fill {
        PenFill::LinearGradient(body) => {
            let stops = gradient_stops(&body.stops)?;
            Some(GradientPayload::Linear {
                angle_deg: body.angle.unwrap_or(0.0),
                opacity: body.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
                stops,
            })
        }
        PenFill::RadialGradient(body) => {
            let stops = gradient_stops(&body.stops)?;
            Some(GradientPayload::Radial {
                cx: body.cx.unwrap_or(0.5).clamp(0.0, 1.0),
                cy: body.cy.unwrap_or(0.5).clamp(0.0, 1.0),
                radius: body.radius.unwrap_or(0.5).clamp(0.0, 1.0),
                opacity: body.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
                stops,
            })
        }
        PenFill::MeshGradient(body) => {
            let colors = mesh_colors(body)?;
            Some(GradientPayload::Mesh {
                rows: body.rows.max(2),
                cols: body.cols.max(2),
                colors,
                opacity: body.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
            })
        }
        _ => None,
    }
}

/// Resolve the first fill into a [`ShaderPayload`] when it is a
/// `Shader`. Uniforms are pre-resolved (a `color` hex → premultiplied
/// `vec4`); the fallback colour is the first `color` uniform, else
/// mid-gray, so a host that can't compile the program still paints a
/// visible block. SkSL source stays untrusted — not validated here.
fn first_shader(fills: Option<&[PenFill]>) -> Option<ShaderPayload> {
    shader_payload(fills?.first()?)
}

/// Resolve one canonical shader layer without projecting away siblings.
pub(crate) fn shader_payload(fill: &PenFill) -> Option<ShaderPayload> {
    let PenFill::Shader(body) = fill else {
        return None;
    };
    let expanded = crate::shader_preset::expand(body);
    let sksl = match &expanded {
        Some(preset) => preset.sksl.clone(),
        None => body
            .sksl
            .as_ref()
            .filter(|source| !source.trim().is_empty())?
            .clone(),
    };
    let mut uniforms: Vec<ShaderUniformPayload> = expanded
        .as_ref()
        .map(|preset| preset.uniforms.clone())
        .unwrap_or_default();
    let mut fallback: Option<[f32; 4]> = None;
    if let Some(map) = &body.uniforms {
        for (name, val) in map {
            match val {
                ShaderUniformValue::Float(f) if expanded.is_none() => {
                    uniforms.push(ShaderUniformPayload {
                        name: name.clone(),
                        values: vec![*f],
                    });
                }
                ShaderUniformValue::Vec(v) => {
                    if expanded.is_none() && !v.is_empty() {
                        uniforms.push(ShaderUniformPayload {
                            name: name.clone(),
                            values: v.clone(),
                        });
                    }
                }
                ShaderUniformValue::Color(hex) => {
                    if let Some(rgba) = parse_color(hex) {
                        // Premultiply for the vec4 binding (matches the
                        // jian-core scene walker's color-uniform rule).
                        let a = rgba[3];
                        if expanded.is_none() {
                            uniforms.push(ShaderUniformPayload {
                                name: name.clone(),
                                values: vec![rgba[0] * a, rgba[1] * a, rgba[2] * a, a],
                            });
                        }
                        if fallback.is_none() {
                            fallback = Some(rgba);
                        }
                    }
                }
                ShaderUniformValue::Float(_) => {}
            }
        }
    }
    Some(ShaderPayload {
        sksl,
        uniforms,
        opacity: body.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
        // Mid-gray when no colour uniform exists.
        fallback: fallback.unwrap_or([0.5, 0.5, 0.5, 1.0]),
    })
}

/// Resolve a mesh body's `stops[]` into a row-major `rows`×`cols`
/// colour grid (length == `rows * cols`). Vertices missing from the
/// sparse `stops[]` default to transparent black so the grid is always
/// fully populated for the triangulator. Returns `None` for a
/// degenerate (< 2×2) grid so the caller falls back to solid.
fn mesh_colors(body: &jian_ops_schema::style::MeshGradientBody) -> Option<Vec<[f32; 4]>> {
    let rows = body.rows.max(2);
    let cols = body.cols.max(2);
    if body.rows < 2 || body.cols < 2 {
        return None;
    }
    let mut colors = vec![[0.0, 0.0, 0.0, 0.0]; (rows * cols) as usize];
    for s in &body.stops {
        if s.row >= rows || s.col >= cols {
            continue;
        }
        if let Some(rgba) = parse_color(&s.color) {
            colors[(s.row * cols + s.col) as usize] = rgba;
        }
    }
    Some(colors)
}

fn gradient_stops(
    stops: &[jian_ops_schema::style::GradientStop],
) -> Option<Vec<GradientStopPayload>> {
    if stops.is_empty() {
        return None;
    }
    Some(
        stops
            .iter()
            .map(|s| GradientStopPayload {
                offset: s.offset.clamp(0.0, 1.0),
                color: parse_color(&s.color).unwrap_or([0.0, 0.0, 0.0, 1.0]),
            })
            .collect(),
    )
}

fn first_solid_color(fills: Option<&[PenFill]>) -> Option<[f32; 4]> {
    fills?.iter().find_map(fill_fallback_color)
}

/// Paintable solid fallback for one canonical fill layer.
pub(crate) fn fill_fallback_color(fill: &PenFill) -> Option<[f32; 4]> {
    match fill {
        PenFill::Solid(body) => {
            if let Some(rgba) = parse_color(&body.color) {
                return Some(apply_alpha(rgba, body.opacity));
            }
        }
        PenFill::LinearGradient(body) => {
            if let Some(stop) = body.stops.first() {
                if let Some(rgba) = parse_color(&stop.color) {
                    return Some(apply_alpha(rgba, body.opacity));
                }
            }
        }
        PenFill::RadialGradient(body) => {
            if let Some(stop) = body.stops.first() {
                if let Some(rgba) = parse_color(&stop.color) {
                    return Some(apply_alpha(rgba, body.opacity));
                }
            }
        }
        PenFill::MeshGradient(body) => {
            // First-vertex colour is the documented solid fallback
            // baked into `node.fill` (backends without per-vertex
            // support paint this flat).
            if let Some(stop) = body.stops.first() {
                if let Some(rgba) = parse_color(&stop.color) {
                    return Some(apply_alpha(rgba, body.opacity));
                }
            }
        }
        PenFill::Shader(body) => {
            // Fallback solid baked into `node.fill`: the first colour
            // uniform if any, else mid-gray. Backends that can't
            // compile the program paint this flat.
            let from_uniform = body.uniforms.as_ref().and_then(|m| {
                m.values().find_map(|v| match v {
                    ShaderUniformValue::Color(hex) => parse_color(hex),
                    _ => None,
                })
            });
            let rgba = from_uniform.unwrap_or([0.5, 0.5, 0.5, 1.0]);
            return Some(apply_alpha(rgba, body.opacity));
        }
        PenFill::Image(_) => {
            return Some([0.85, 0.86, 0.88, 1.0]);
        }
    }
    None
}

fn first_fill_type(fills: Option<&[PenFill]>) -> String {
    let Some(fills) = fills else {
        return "solid".into();
    };
    match fills.first() {
        Some(PenFill::LinearGradient(_)) => "linear".into(),
        Some(PenFill::RadialGradient(_)) => "radial".into(),
        Some(PenFill::MeshGradient(_)) => "mesh".into(),
        Some(PenFill::Shader(_)) => "shader".into(),
        Some(PenFill::Image(_)) => "image".into(),
        _ => "solid".into(),
    }
}

pub(crate) fn stroke_to_payload(s: Option<&PenStroke>) -> Option<StrokePayload> {
    let s = s?;
    let (width, sides) = match &s.thickness {
        StrokeThickness::Uniform(n) => (*n, None),
        StrokeThickness::PerSide(sides) => {
            (sides.iter().copied().fold(0.0_f32, f32::max), Some(*sides))
        }
        StrokeThickness::Sided(sided) => {
            let sides = [
                sided.top.unwrap_or(0.0),
                sided.right.unwrap_or(0.0),
                sided.bottom.unwrap_or(0.0),
                sided.left.unwrap_or(0.0),
            ];
            (sides.iter().copied().fold(0.0_f32, f32::max), Some(sides))
        }
    };
    // No fabricated fallback here: an unresolvable stroke paint stays `None`
    // so downstream consumers can tell "the author wrote no paint" apart from
    // "the author wrote black". Fabricating opaque black at this seam painted
    // black switch tracks / select borders for every control whose author
    // omitted `stroke.fill`.
    let color = first_solid_color(s.fill.as_deref());
    let align = match s.align {
        Some(StrokeAlign::Inside) => -1,
        Some(StrokeAlign::Outside) => 1,
        Some(StrokeAlign::Center) | None => 0,
    };
    Some(StrokePayload {
        color,
        width,
        sides,
        align,
    })
}

pub(crate) fn apply_alpha(rgba: [f32; 4], opacity: Option<f32>) -> [f32; 4] {
    let a = opacity.unwrap_or(1.0).clamp(0.0, 1.0);
    [rgba[0], rgba[1], rgba[2], rgba[3] * a]
}

/// `#RGB` / `#RRGGBB` / `#RRGGBBAA` (leading `#` optional) → normalized
/// RGBA. Delegates to the canonical op-util parser; the historical 3/6/8
/// digit set is kept (no 4-digit `#RGBA` shorthand).
pub(crate) fn parse_hex(s: &str) -> Option<[f32; 4]> {
    const OPTS: op_util::hex_color::HexOptions = op_util::hex_color::HexOptions {
        require_hash: false,
        allow_rgb_shorthand: true,
        allow_rgba_shorthand: false,
        allow_alpha: true,
    };
    op_util::hex_color::parse_hex_rgba_f32(s, OPTS)
}

/// Any colour string a canonical `.op` fill may carry → normalized RGBA:
/// hex (via [`parse_hex`]) OR the functional `rgb(r,g,b)` /
/// `rgba(r,g,b,a)` form (`r`/`g`/`b` 0-255, `a` 0..1). Every fill,
/// gradient-stop, mesh-cell, shader-uniform and text-colour site goes
/// through this so an `rgba()` stop never degrades to opaque black (the
/// hero-scrim bug: a three-stop `rgba(32,31,29,0.18..0.78)` gradient
/// painted as a solid black wall over the photo underneath).
pub(crate) fn parse_color(s: &str) -> Option<[f32; 4]> {
    let t = s.trim();
    if let Some(rgba) = parse_hex(t) {
        return Some(rgba);
    }
    let lower = t.to_ascii_lowercase();
    let inner = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let r = parts[0].parse::<f32>().ok()?;
    let g = parts[1].parse::<f32>().ok()?;
    let b = parts[2].parse::<f32>().ok()?;
    let a = if parts.len() >= 4 {
        parts[3].parse::<f32>().ok()?
    } else {
        1.0
    };
    Some([
        (r / 255.0).clamp(0.0, 1.0),
        (g / 255.0).clamp(0.0, 1.0),
        (b / 255.0).clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    ])
}

pub(crate) fn short_src(src: &str) -> String {
    let s = src.rsplit('/').next().unwrap_or(src);
    if s.chars().count() > 24 {
        let head: String = s.chars().take(24).collect();
        format!("{head}…")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use jian_ops_schema::style::GradientStop;

    fn stop(offset: f32, color: &str) -> GradientStop {
        GradientStop {
            offset,
            color: color.to_string(),
        }
    }

    #[test]
    fn parse_color_accepts_hex_and_functional_forms() {
        assert_eq!(
            parse_color("#201F1D"),
            Some([32.0 / 255.0, 31.0 / 255.0, 29.0 / 255.0, 1.0])
        );
        let rgba = parse_color("rgba(32, 31, 29, 0.18)").expect("rgba parses");
        assert!((rgba[0] - 32.0 / 255.0).abs() < 1e-6);
        assert!((rgba[3] - 0.18).abs() < 1e-6);
        assert_eq!(parse_color("rgb(255,128,0)").map(|c| c[3]), Some(1.0));
        assert_eq!(parse_color("not-a-colour"), None);
    }

    #[test]
    fn rgba_gradient_stops_keep_their_alpha_instead_of_opaque_black() {
        let stops = gradient_stops(&[
            stop(0.0, "rgba(32,31,29,0.18)"),
            stop(0.45, "rgba(32,31,29,0.42)"),
            stop(1.0, "rgba(32,31,29,0.78)"),
        ])
        .expect("stops");
        let alphas: Vec<f32> = stops.iter().map(|s| s.color[3]).collect();
        assert!((alphas[0] - 0.18).abs() < 1e-6, "alphas {alphas:?}");
        assert!((alphas[2] - 0.78).abs() < 1e-6, "alphas {alphas:?}");
        assert!(
            stops.iter().all(|s| s.color[0] > 0.0),
            "rgb must not collapse to black"
        );
    }
}
