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
        stroke: None,
        text: None,
        font_family: String::new(),
        rotation: (base.rotation.unwrap_or(0.0) as f32).to_radians(),
        flip_x: base.flip_x.unwrap_or(false),
        flip_y: base.flip_y.unwrap_or(false),
        opacity: base_opacity(base),
        corner_radius: 0.0,
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
        image_src: None,
        image_fit: None,
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
    p.corner_radius = match corner_radius {
        Some(CornerRadius::Uniform(r)) => *r as f32,
        Some(CornerRadius::PerCorner(corners)) => corners[0] as f32,
        None => 0.0,
    };
}

pub(crate) fn assign_first_fill(p: &mut NodePayload, fills: Option<&[PenFill]>) {
    p.fill = first_solid_color(fills);
    p.fill_type = first_fill_type(fills);
    p.gradient = first_gradient(fills);
    p.shader = first_shader(fills);
    if let Some((url, fit, adjustments)) = first_image_fill(fills) {
        p.image_src = Some(url);
        p.image_fit = Some(fit);
        p.image_adjustments = adjustments;
    }
}

fn first_image_fill(
    fills: Option<&[PenFill]>,
) -> Option<(String, String, Option<ImageAdjustmentPayload>)> {
    let body = fills?.first().and_then(|f| match f {
        PenFill::Image(b) => Some(b),
        _ => None,
    })?;
    if body.url.trim().is_empty() {
        None
    } else {
        Some((
            body.url.to_string(),
            image_fill_mode_to_payload(body.mode.as_ref()),
            image_fill_adjustments(body),
        ))
    }
}

fn image_fill_mode_to_payload(mode: Option<&ImageFillMode>) -> String {
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

fn image_fill_adjustments(
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
    let fills = fills?;
    let first = fills.first()?;
    match first {
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
    let PenFill::Shader(body) = fills?.first()? else {
        return None;
    };
    if body.sksl.trim().is_empty() {
        return None;
    }
    let mut uniforms: Vec<ShaderUniformPayload> = Vec::new();
    let mut fallback: Option<[f32; 4]> = None;
    if let Some(map) = &body.uniforms {
        for (name, val) in map {
            match val {
                ShaderUniformValue::Float(f) => uniforms.push(ShaderUniformPayload {
                    name: name.clone(),
                    values: vec![*f],
                }),
                ShaderUniformValue::Vec(v) => {
                    if !v.is_empty() {
                        uniforms.push(ShaderUniformPayload {
                            name: name.clone(),
                            values: v.clone(),
                        });
                    }
                }
                ShaderUniformValue::Color(hex) => {
                    if let Some(rgba) = parse_hex(hex) {
                        // Premultiply for the vec4 binding (matches the
                        // jian-core scene walker's color-uniform rule).
                        let a = rgba[3];
                        uniforms.push(ShaderUniformPayload {
                            name: name.clone(),
                            values: vec![rgba[0] * a, rgba[1] * a, rgba[2] * a, a],
                        });
                        if fallback.is_none() {
                            fallback = Some(rgba);
                        }
                    }
                }
            }
        }
    }
    Some(ShaderPayload {
        sksl: body.sksl.clone(),
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
        if let Some(rgba) = parse_hex(&s.color) {
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
                color: parse_hex(&s.color).unwrap_or([0.0, 0.0, 0.0, 1.0]),
            })
            .collect(),
    )
}

fn first_solid_color(fills: Option<&[PenFill]>) -> Option<[f32; 4]> {
    let fills = fills?;
    for fill in fills {
        match fill {
            PenFill::Solid(body) => {
                if let Some(rgba) = parse_hex(&body.color) {
                    return Some(apply_alpha(rgba, body.opacity));
                }
            }
            PenFill::LinearGradient(body) => {
                if let Some(stop) = body.stops.first() {
                    if let Some(rgba) = parse_hex(&stop.color) {
                        return Some(apply_alpha(rgba, body.opacity));
                    }
                }
            }
            PenFill::RadialGradient(body) => {
                if let Some(stop) = body.stops.first() {
                    if let Some(rgba) = parse_hex(&stop.color) {
                        return Some(apply_alpha(rgba, body.opacity));
                    }
                }
            }
            PenFill::MeshGradient(body) => {
                // First-vertex colour is the documented solid fallback
                // baked into `node.fill` (backends without per-vertex
                // support paint this flat).
                if let Some(stop) = body.stops.first() {
                    if let Some(rgba) = parse_hex(&stop.color) {
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
                        ShaderUniformValue::Color(hex) => parse_hex(hex),
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
    let color = first_solid_color(s.fill.as_deref()).unwrap_or([0.0, 0.0, 0.0, 1.0]);
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

fn apply_alpha(rgba: [f32; 4], opacity: Option<f32>) -> [f32; 4] {
    let a = opacity.unwrap_or(1.0).clamp(0.0, 1.0);
    [rgba[0], rgba[1], rgba[2], rgba[3] * a]
}

pub(crate) fn parse_hex(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    let (r, g, b, a) = match s.len() {
        3 => (
            u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?,
            255u8,
        ),
        6 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            255u8,
        ),
        8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            u8::from_str_radix(&s[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some([
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
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
