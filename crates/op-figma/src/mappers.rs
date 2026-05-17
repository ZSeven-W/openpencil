//! Figma style mappers — ports `figma-fill-mapper.ts`,
//! `figma-stroke-mapper.ts`, `figma-effect-mapper.ts` and
//! `figma-layout-mapper.ts`. Each turns Figma `.fig` node fields
//! (decoded [`FigValue`] objects) into canonical `jian_ops_schema`
//! style values.

use crate::color::figma_color_to_hex;
use crate::figma_types::{FigColor, FigMatrix};
use crate::kiwi::FigValue;
use jian_ops_schema::node::container::{AlignItems, JustifyContent, LayoutMode, Padding};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{
    BlurBody, GradientStop, ImageFillBody, ImageFillMode, ImageOriginalSize, ImageTransform,
    LinearGradientBody, PenEffect, PenFill, PenStroke, RadialGradientBody, ShadowBody,
    SolidFillBody, StrokeAlign, StrokeCap, StrokeJoin, StrokeThickness,
};

/// Identity-matrix epsilon for image-fill transforms.
const IMAGE_TRANSFORM_EPSILON: f64 = 1e-6;

// ── Fills ─────────────────────────────────────────────────────────

/// Map a Figma paint array to `PenFill[]`. Invisible paints
/// (`visible == false`) and unmappable types are dropped; an empty
/// result is `None`.
pub fn map_figma_fills(paints: Option<&[FigValue]>) -> Option<Vec<PenFill>> {
    let paints = paints?;
    if paints.is_empty() {
        return None;
    }
    let fills: Vec<PenFill> = paints
        .iter()
        .filter(|p| p.get_bool("visible") != Some(false))
        .filter_map(map_single_fill)
        .collect();
    if fills.is_empty() {
        None
    } else {
        Some(fills)
    }
}

fn fill_color_hex(v: &FigValue) -> String {
    v.get("color")
        .and_then(FigColor::from_value)
        .map(|c| figma_color_to_hex(&c))
        .unwrap_or_else(|| "#000000".to_string())
}

fn gradient_stops(paint: &FigValue) -> Vec<GradientStop> {
    paint
        .get_array("stops")
        .map(|stops| {
            stops
                .iter()
                .map(|s| GradientStop {
                    offset: s.get_f64("position").unwrap_or(0.0) as f32,
                    color: fill_color_hex(s),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn map_single_fill(paint: &FigValue) -> Option<PenFill> {
    let opacity = paint.get_f64("opacity").map(|o| o as f32);
    match paint.get_str("type")? {
        "SOLID" => {
            paint.get("color")?;
            Some(PenFill::Solid(SolidFillBody {
                color: fill_color_hex(paint),
                explain: None,
                opacity,
                blend_mode: None,
            }))
        }
        "GRADIENT_LINEAR" => {
            paint.get("stops")?;
            let angle = paint
                .get("transform")
                .and_then(FigMatrix::from_value)
                .map(|m| gradient_angle_from_transform(&m))
                .unwrap_or(0);
            Some(PenFill::LinearGradient(LinearGradientBody {
                angle: Some(angle as f32),
                stops: gradient_stops(paint),
                explain: None,
                opacity,
                blend_mode: None,
            }))
        }
        "GRADIENT_RADIAL" | "GRADIENT_ANGULAR" | "GRADIENT_DIAMOND" => {
            paint.get("stops")?;
            Some(PenFill::RadialGradient(RadialGradientBody {
                cx: Some(0.5),
                cy: Some(0.5),
                radius: Some(0.5),
                stops: gradient_stops(paint),
                explain: None,
                opacity,
                blend_mode: None,
            }))
        }
        "IMAGE" => {
            let image = paint.get("image");
            let url = image
                .and_then(|i| i.get("hash"))
                .and_then(|h| h.as_bytes())
                .filter(|b| !b.is_empty())
                .map(|bytes| {
                    let mut s = String::from("__hash:");
                    for b in bytes {
                        s.push_str(&format!("{b:02x}"));
                    }
                    s
                })
                .or_else(|| {
                    image
                        .and_then(|i| i.get_f64("dataBlob"))
                        .map(|n| format!("__blob:{}", n as i64))
                })
                .unwrap_or_default();
            Some(PenFill::Image(ImageFillBody {
                url,
                mode: Some(map_scale_mode(paint.get_str("imageScaleMode"))),
                original_size: normalize_original_size(
                    paint.get_f64("originalImageWidth"),
                    paint.get_f64("originalImageHeight"),
                ),
                transform: paint
                    .get("transform")
                    .and_then(FigMatrix::from_value)
                    .and_then(normalize_image_transform),
                explain: None,
                opacity,
                exposure: None,
                contrast: None,
                saturation: None,
                temperature: None,
                tint: None,
                highlights: None,
                shadows: None,
            }))
        }
        _ => None,
    }
}

/// Gradient direction vector is column 0 of the paint transform;
/// CSS-convention angle = `90° − math-angle`, rounded to an integer.
fn gradient_angle_from_transform(m: &FigMatrix) -> i32 {
    let math_angle = m.m10.atan2(m.m00) * (180.0 / std::f64::consts::PI);
    (90.0 - math_angle).round() as i32
}

fn normalize_original_size(w: Option<f64>, h: Option<f64>) -> Option<ImageOriginalSize> {
    let (w, h) = (w?, h?);
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some(ImageOriginalSize {
        width: w as f32,
        height: h as f32,
    })
}

fn normalize_image_transform(m: FigMatrix) -> Option<ImageTransform> {
    let e = IMAGE_TRANSFORM_EPSILON;
    let is_identity = (m.m00 - 1.0).abs() <= e
        && m.m01.abs() <= e
        && m.m02.abs() <= e
        && m.m10.abs() <= e
        && (m.m11 - 1.0).abs() <= e
        && m.m12.abs() <= e;
    if is_identity {
        return None;
    }
    Some(ImageTransform {
        m00: m.m00 as f32,
        m01: m.m01 as f32,
        m02: m.m02 as f32,
        m10: m.m10 as f32,
        m11: m.m11 as f32,
        m12: m.m12 as f32,
    })
}

fn map_scale_mode(mode: Option<&str>) -> ImageFillMode {
    match mode {
        Some("FIT") => ImageFillMode::Fit,
        Some("STRETCH") => ImageFillMode::Stretch,
        _ => ImageFillMode::Fill,
    }
}

// ── Stroke ────────────────────────────────────────────────────────

/// Map a Figma node's stroke properties to a `PenStroke`. `None` when
/// the node has no visible stroke paint.
pub fn map_figma_stroke(node: &FigValue) -> Option<PenStroke> {
    let stroke_paints = node.get_array("strokePaints").filter(|p| !p.is_empty())?;
    let has_visible = stroke_paints
        .iter()
        .any(|s| s.get_bool("visible") != Some(false));
    if !has_visible {
        return None;
    }

    let thickness = if node.get_bool("borderStrokeWeightsIndependent") == Some(true) {
        StrokeThickness::PerSide([
            node.get_f64("borderTopWeight").unwrap_or(0.0) as f32,
            node.get_f64("borderRightWeight").unwrap_or(0.0) as f32,
            node.get_f64("borderBottomWeight").unwrap_or(0.0) as f32,
            node.get_f64("borderLeftWeight").unwrap_or(0.0) as f32,
        ])
    } else {
        StrokeThickness::Uniform(node.get_f64("strokeWeight").unwrap_or(1.0) as f32)
    };

    let dash_pattern = node
        .get_array("dashPattern")
        .filter(|d| !d.is_empty())
        .map(|d| {
            d.iter()
                .filter_map(|v| v.as_f64())
                .map(|v| v as f32)
                .collect()
        });

    Some(PenStroke {
        thickness,
        align: map_stroke_align(node.get_str("strokeAlign")),
        join: map_stroke_join(node.get_str("strokeJoin")),
        cap: map_stroke_cap(node.get_str("strokeCap")),
        dash_pattern,
        dash_offset: None,
        // `mapFigmaFills` re-applies the visible filter — harmless.
        fill: map_figma_fills(Some(stroke_paints)),
    })
}

fn map_stroke_align(v: Option<&str>) -> Option<StrokeAlign> {
    match v {
        Some("INSIDE") => Some(StrokeAlign::Inside),
        Some("OUTSIDE") => Some(StrokeAlign::Outside),
        Some("CENTER") => Some(StrokeAlign::Center),
        _ => None,
    }
}

fn map_stroke_join(v: Option<&str>) -> Option<StrokeJoin> {
    match v {
        Some("MITER") => Some(StrokeJoin::Miter),
        Some("BEVEL") => Some(StrokeJoin::Bevel),
        Some("ROUND") => Some(StrokeJoin::Round),
        _ => None,
    }
}

fn map_stroke_cap(v: Option<&str>) -> Option<StrokeCap> {
    match v {
        Some("NONE") => Some(StrokeCap::None),
        Some("ROUND") => Some(StrokeCap::Round),
        Some("SQUARE") => Some(StrokeCap::Square),
        _ => None,
    }
}

// ── Effects ───────────────────────────────────────────────────────

/// Map a Figma effect array to `PenEffect[]`.
pub fn map_figma_effects(effects: Option<&[FigValue]>) -> Option<Vec<PenEffect>> {
    let effects = effects?;
    if effects.is_empty() {
        return None;
    }
    let mapped: Vec<PenEffect> = effects
        .iter()
        .filter(|e| e.get_bool("visible") != Some(false))
        .filter_map(map_single_effect)
        .collect();
    if mapped.is_empty() {
        None
    } else {
        Some(mapped)
    }
}

fn map_single_effect(effect: &FigValue) -> Option<PenEffect> {
    let ty = effect.get_str("type")?;
    let radius = effect.get_f64("radius").unwrap_or(0.0) as f32;
    match ty {
        "DROP_SHADOW" | "INNER_SHADOW" => {
            let offset = effect.get("offset");
            Some(PenEffect::Shadow(ShadowBody {
                inner: Some(ty == "INNER_SHADOW"),
                offset_x: offset.and_then(|o| o.get_f64("x")).unwrap_or(0.0) as f32,
                offset_y: offset.and_then(|o| o.get_f64("y")).unwrap_or(0.0) as f32,
                blur: radius,
                spread: effect.get_f64("spread").unwrap_or(0.0) as f32,
                color: effect
                    .get("color")
                    .and_then(FigColor::from_value)
                    .map(|c| figma_color_to_hex(&c))
                    .unwrap_or_else(|| "#00000040".to_string()),
            }))
        }
        "FOREGROUND_BLUR" => Some(PenEffect::Blur(BlurBody { radius })),
        "BACKGROUND_BLUR" => Some(PenEffect::BackgroundBlur(BlurBody { radius })),
        _ => None,
    }
}

// ── Auto-layout ───────────────────────────────────────────────────

/// Partial layout props lifted off a Figma auto-layout (stack) node.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LayoutProps {
    pub layout: Option<LayoutMode>,
    pub gap: Option<f64>,
    pub padding: Option<Padding>,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub clip_content: Option<bool>,
}

/// Map Figma stack fields → `LayoutProps`.
pub fn map_figma_layout(node: &FigValue) -> LayoutProps {
    let mut result = LayoutProps::default();

    if let Some(mode) = node.get_str("stackMode") {
        if mode != "NONE" {
            result.layout = Some(if mode == "HORIZONTAL" {
                LayoutMode::Horizontal
            } else {
                LayoutMode::Vertical
            });
        }
    }

    if let Some(align) = node.get_str("stackPrimaryAlignItems") {
        result.justify_content = map_justify_content(align);
    }

    if let Some(sp) = node.get_f64("stackSpacing") {
        if sp != 0.0 && result.justify_content != Some(JustifyContent::SpaceBetween) {
            result.gap = Some(sp);
        }
    }

    result.padding = map_padding(node);

    if let Some(align) = node.get_str("stackCounterAlignItems") {
        result.align_items = map_align_items(align);
    }

    if node.get_bool("frameMaskDisabled") != Some(true) {
        result.clip_content = Some(true);
    }

    result
}

fn map_padding(node: &FigValue) -> Option<Padding> {
    let h = node.get_f64("stackHorizontalPadding");
    let v = node.get_f64("stackVerticalPadding");
    let right = node.get_f64("stackPaddingRight");
    let bottom = node.get_f64("stackPaddingBottom");

    if h.is_none() && v.is_none() && right.is_none() && bottom.is_none() {
        // Uniform-only path.
        return match node.get_f64("stackPadding") {
            Some(p) if p > 0.0 => Some(Padding::Uniform(p)),
            _ => None,
        };
    }

    let uniform = node.get_f64("stackPadding");
    let v_pad = v.or(uniform).unwrap_or(0.0);
    let h_pad = h.or(uniform).unwrap_or(0.0);
    let top = v_pad;
    let bottom = bottom.unwrap_or(v_pad);
    let left = h_pad;
    let right = right.unwrap_or(h_pad);

    if top == 0.0 && right == 0.0 && bottom == 0.0 && left == 0.0 {
        return None;
    }
    if top == right && right == bottom && bottom == left {
        return Some(Padding::Uniform(top));
    }
    if top == bottom && left == right {
        return Some(Padding::XY([top, right]));
    }
    Some(Padding::LtrB([top, right, bottom, left]))
}

fn map_justify_content(align: &str) -> Option<JustifyContent> {
    match align {
        "MIN" => Some(JustifyContent::Start),
        "CENTER" => Some(JustifyContent::Center),
        "MAX" => Some(JustifyContent::End),
        "SPACE_EVENLY" => Some(JustifyContent::SpaceBetween),
        _ => None,
    }
}

fn map_align_items(align: &str) -> Option<AlignItems> {
    match align {
        "MIN" => Some(AlignItems::Start),
        "CENTER" => Some(AlignItems::Center),
        "MAX" => Some(AlignItems::End),
        _ => None,
    }
}

// ── Sizing ────────────────────────────────────────────────────────

/// Resolve a node's width as a `SizingBehavior` in `openpencil` layout
/// mode (auto-layout-aware). `parent_stack_mode` is the parent frame's
/// raw `stackMode`.
pub fn map_width_sizing(node: &FigValue, parent_stack_mode: Option<&str>) -> SizingBehavior {
    let stack_mode = node.get_str("stackMode");
    if node.get_str("stackPrimarySizing") == Some("RESIZE_TO_FIT")
        && stack_mode == Some("HORIZONTAL")
    {
        return SizingBehavior::Keyword(SizingKeyword::FitContent);
    }
    if node.get_str("stackCounterSizing") == Some("RESIZE_TO_FIT") && stack_mode == Some("VERTICAL")
    {
        return SizingBehavior::Keyword(SizingKeyword::FitContent);
    }
    if node.get_f64("stackChildPrimaryGrow") == Some(1.0) && parent_stack_mode == Some("HORIZONTAL")
    {
        return SizingBehavior::Keyword(SizingKeyword::FillContainer);
    }
    if node.get_str("stackChildAlignSelf") == Some("STRETCH")
        && parent_stack_mode == Some("VERTICAL")
    {
        return SizingBehavior::Keyword(SizingKeyword::FillContainer);
    }
    SizingBehavior::Number(
        node.get("size")
            .and_then(|s| s.get_f64("x"))
            .unwrap_or(100.0),
    )
}

/// Resolve a node's height — the width logic with the axes swapped.
pub fn map_height_sizing(node: &FigValue, parent_stack_mode: Option<&str>) -> SizingBehavior {
    let stack_mode = node.get_str("stackMode");
    if node.get_str("stackPrimarySizing") == Some("RESIZE_TO_FIT") && stack_mode == Some("VERTICAL")
    {
        return SizingBehavior::Keyword(SizingKeyword::FitContent);
    }
    if node.get_str("stackCounterSizing") == Some("RESIZE_TO_FIT")
        && stack_mode == Some("HORIZONTAL")
    {
        return SizingBehavior::Keyword(SizingKeyword::FitContent);
    }
    if node.get_f64("stackChildPrimaryGrow") == Some(1.0) && parent_stack_mode == Some("VERTICAL") {
        return SizingBehavior::Keyword(SizingKeyword::FillContainer);
    }
    if node.get_str("stackChildAlignSelf") == Some("STRETCH")
        && parent_stack_mode == Some("HORIZONTAL")
    {
        return SizingBehavior::Keyword(SizingKeyword::FillContainer);
    }
    SizingBehavior::Number(
        node.get("size")
            .and_then(|s| s.get_f64("y"))
            .unwrap_or(100.0),
    )
}

#[cfg(test)]
mod tests;
