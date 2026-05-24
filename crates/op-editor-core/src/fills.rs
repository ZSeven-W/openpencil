//! Fill / stroke / effect read-write helpers for `PenNode`.
//!
//! The canonical model stores rich `Vec<PenFill>` payloads; the
//! property panel mostly edits a primary colour/fill/stroke/effect
//! surface. These helpers preserve non-target fills and gradient/image
//! bodies instead of flattening the node into shell-core's old scalar
//! colour model.

use crate::editor_ui_state::{FillType, ImageAdjustmentField, ImageFillMode};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{
    GradientStop, ImageFillBody, LinearGradientBody, PenEffect, PenFill, PenStroke,
    RadialGradientBody, ShadowBody, SolidFillBody, StrokeThickness,
};

/// Display/edit summary of the primary image fill.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageFillSummary {
    pub mode: ImageFillMode,
    pub has_image: bool,
    /// Current primary image URL. Native picked images are stored as
    /// inline `data:image/...;base64,...` URLs so the property popover
    /// can decode and preview them without another host-side fetch.
    pub image_url: Option<String>,
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub tint: f32,
    pub highlights: f32,
    pub shadows: f32,
}

impl ImageFillSummary {
    pub fn adjustment(&self, field: ImageAdjustmentField) -> f32 {
        match field {
            ImageAdjustmentField::Exposure => self.exposure,
            ImageAdjustmentField::Contrast => self.contrast,
            ImageAdjustmentField::Saturation => self.saturation,
            ImageAdjustmentField::Temperature => self.temperature,
            ImageAdjustmentField::Tint => self.tint,
            ImageAdjustmentField::Highlights => self.highlights,
            ImageAdjustmentField::Shadows => self.shadows,
        }
    }

    pub fn has_adjustments(&self) -> bool {
        ImageAdjustmentField::ALL
            .iter()
            .any(|field| self.adjustment(*field).abs() > f32::EPSILON)
    }
}

/// Borrow a node's `fill` list, if the variant carries one. Frame /
/// Group fills live on `container.fill`; the leaf paintable variants
/// (Rectangle / Ellipse / Polygon / Path / Text / TextInput /
/// IconFont) carry their own `fill`. Line / Image / Ref have none.
pub fn node_fills(node: &PenNode) -> Option<&Vec<PenFill>> {
    match node {
        PenNode::Frame(n) => n.container.fill.as_ref(),
        PenNode::Group(n) => n.container.fill.as_ref(),
        PenNode::Rectangle(n) => n.container.fill.as_ref(),
        PenNode::Ellipse(n) => n.fill.as_ref(),
        PenNode::Polygon(n) => n.fill.as_ref(),
        PenNode::Path(n) => n.fill.as_ref(),
        PenNode::Text(n) => n.fill.as_ref(),
        PenNode::TextInput(n) => n.fill.as_ref(),
        PenNode::IconFont(n) => n.fill.as_ref(),
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

/// Mutably borrow a node's `fill` list, creating an empty one when
/// the variant supports fills but has none yet. `None` for the
/// variants that have no `fill` field at all.
pub fn node_fills_mut(node: &mut PenNode) -> Option<&mut Vec<PenFill>> {
    match node {
        PenNode::Frame(n) => Some(n.container.fill.get_or_insert_with(Vec::new)),
        PenNode::Group(n) => Some(n.container.fill.get_or_insert_with(Vec::new)),
        PenNode::Rectangle(n) => Some(n.container.fill.get_or_insert_with(Vec::new)),
        PenNode::Ellipse(n) => Some(n.fill.get_or_insert_with(Vec::new)),
        PenNode::Polygon(n) => Some(n.fill.get_or_insert_with(Vec::new)),
        PenNode::Path(n) => Some(n.fill.get_or_insert_with(Vec::new)),
        PenNode::Text(n) => Some(n.fill.get_or_insert_with(Vec::new)),
        PenNode::TextInput(n) => Some(n.fill.get_or_insert_with(Vec::new)),
        PenNode::IconFont(n) => Some(n.fill.get_or_insert_with(Vec::new)),
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

/// Borrow a node's `stroke`, if the variant carries one.
fn node_stroke_mut(node: &mut PenNode) -> Option<&mut Option<PenStroke>> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container.stroke),
        PenNode::Group(n) => Some(&mut n.container.stroke),
        PenNode::Rectangle(n) => Some(&mut n.container.stroke),
        PenNode::Ellipse(n) => Some(&mut n.stroke),
        PenNode::Polygon(n) => Some(&mut n.stroke),
        PenNode::Path(n) => Some(&mut n.stroke),
        PenNode::Line(n) => Some(&mut n.stroke),
        PenNode::TextInput(n) => Some(&mut n.stroke),
        PenNode::IconFont(n) => Some(&mut n.stroke),
        PenNode::Text(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

/// Shared stroke accessor for reads.
fn node_stroke(node: &PenNode) -> Option<&PenStroke> {
    match node {
        PenNode::Frame(n) => n.container.stroke.as_ref(),
        PenNode::Group(n) => n.container.stroke.as_ref(),
        PenNode::Rectangle(n) => n.container.stroke.as_ref(),
        PenNode::Ellipse(n) => n.stroke.as_ref(),
        PenNode::Polygon(n) => n.stroke.as_ref(),
        PenNode::Path(n) => n.stroke.as_ref(),
        PenNode::Line(n) => n.stroke.as_ref(),
        PenNode::TextInput(n) => n.stroke.as_ref(),
        PenNode::IconFont(n) => n.stroke.as_ref(),
        PenNode::Text(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

/// Mutably borrow a node's `effects` list, creating an empty one
/// when the variant supports effects but has none yet.
fn node_effects_mut(node: &mut PenNode) -> Option<&mut Vec<PenEffect>> {
    match node {
        PenNode::Frame(n) => Some(n.container.effects.get_or_insert_with(Vec::new)),
        PenNode::Group(n) => Some(n.container.effects.get_or_insert_with(Vec::new)),
        PenNode::Rectangle(n) => Some(n.container.effects.get_or_insert_with(Vec::new)),
        PenNode::Ellipse(n) => Some(n.effects.get_or_insert_with(Vec::new)),
        PenNode::Polygon(n) => Some(n.effects.get_or_insert_with(Vec::new)),
        PenNode::Path(n) => Some(n.effects.get_or_insert_with(Vec::new)),
        PenNode::Line(n) => Some(n.effects.get_or_insert_with(Vec::new)),
        PenNode::Text(n) => Some(n.effects.get_or_insert_with(Vec::new)),
        PenNode::TextInput(n) => Some(n.effects.get_or_insert_with(Vec::new)),
        PenNode::Image(n) => Some(n.effects.get_or_insert_with(Vec::new)),
        PenNode::IconFont(_) | PenNode::Ref(_) => None,
    }
}

/// First `Solid` fill's hex string, when the node has one.
pub fn first_solid_fill_hex(node: &PenNode) -> Option<&str> {
    let fills = node_fills(node)?;
    fills.iter().find_map(|f| match f {
        PenFill::Solid(body) => Some(body.color.as_str()),
        _ => None,
    })
}

/// Read-only view of a node's visual effects. Frame / Group /
/// Rectangle carry them on `container`; the leaf shapes carry them
/// directly. Returns an empty slice for a node with no effects (or a
/// kind — IconFont / Ref — with no effects field).
pub fn node_effects(node: &PenNode) -> &[jian_ops_schema::style::PenEffect] {
    let slot = match node {
        PenNode::Frame(n) => n.container.effects.as_deref(),
        PenNode::Group(n) => n.container.effects.as_deref(),
        PenNode::Rectangle(n) => n.container.effects.as_deref(),
        PenNode::Ellipse(n) => n.effects.as_deref(),
        PenNode::Polygon(n) => n.effects.as_deref(),
        PenNode::Path(n) => n.effects.as_deref(),
        PenNode::Line(n) => n.effects.as_deref(),
        PenNode::Text(n) => n.effects.as_deref(),
        PenNode::TextInput(n) => n.effects.as_deref(),
        PenNode::Image(n) => n.effects.as_deref(),
        PenNode::IconFont(_) | PenNode::Ref(_) => None,
    };
    slot.unwrap_or(&[])
}

/// First `Solid` fill's hex string on the node's stroke.
pub fn first_solid_stroke_hex(node: &PenNode) -> Option<&str> {
    let stroke = node_stroke(node)?;
    stroke.fill.as_ref()?.iter().find_map(|f| match f {
        PenFill::Solid(body) => Some(body.color.as_str()),
        _ => None,
    })
}

/// Uniform stroke width (doc-px) for the node, when it carries a
/// stroke. A `PerSide` thickness reads as its first (top) edge — the
/// editor's property panel exposes a single scalar width. `None`
/// when the variant carries no stroke or the node has none set.
pub fn node_stroke_width(node: &PenNode) -> Option<f64> {
    use jian_ops_schema::style::StrokeThickness;
    match &node_stroke(node)?.thickness {
        StrokeThickness::Uniform(w) => Some(*w as f64),
        StrokeThickness::PerSide(sides) => Some(sides[0] as f64),
        StrokeThickness::Sided(s) => Some(s.top.unwrap_or(0.0) as f64),
    }
}

/// Build a bare `Solid` fill from a hex string.
fn solid_fill(hex: String) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: hex,
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

/// Opacity of the node's **primary** fill — whatever kind it is
/// (Solid / LinearGradient / RadialGradient / Image). `1.0` when
/// the node has no fill at all or the body's opacity is `None`.
/// The Fill section's `100 %` input drives this regardless of
/// fill kind, so a gradient / image fill reports its own opacity.
pub fn first_solid_fill_opacity(node: &PenNode) -> f32 {
    node_fills(node)
        .and_then(|fills| fills.first())
        .map(|fill| match fill {
            PenFill::Solid(b) => b.opacity.unwrap_or(1.0),
            PenFill::LinearGradient(b) => b.opacity.unwrap_or(1.0),
            PenFill::RadialGradient(b) => b.opacity.unwrap_or(1.0),
            PenFill::Image(b) => b.opacity.unwrap_or(1.0),
        })
        .unwrap_or(1.0)
}

/// Summary of the node's primary image fill. `None` when the first
/// fill isn't `Image`.
pub fn first_image_fill_summary(node: &PenNode) -> Option<ImageFillSummary> {
    let PenFill::Image(body) = node_fills(node)?.first()? else {
        return None;
    };
    let trimmed_url = body.url.trim();
    Some(ImageFillSummary {
        mode: ImageFillMode::from_schema(body.mode.as_ref()),
        has_image: !trimmed_url.is_empty(),
        image_url: (!trimmed_url.is_empty()).then(|| body.url.clone()),
        exposure: body.exposure.unwrap_or(0.0),
        contrast: body.contrast.unwrap_or(0.0),
        saturation: body.saturation.unwrap_or(0.0),
        temperature: body.temperature.unwrap_or(0.0),
        tint: body.tint.unwrap_or(0.0),
        highlights: body.highlights.unwrap_or(0.0),
        shadows: body.shadows.unwrap_or(0.0),
    })
}

fn primary_image_fill_mut(node: &mut PenNode) -> Option<&mut ImageFillBody> {
    if node_fills(node).map(|f| f.is_empty()).unwrap_or(true) {
        return None;
    }
    let fills = node_fills_mut(node)?;
    match fills.first_mut()? {
        PenFill::Image(body) => Some(body),
        _ => None,
    }
}

/// Set the primary image fill's fit mode.
pub fn set_primary_image_fill_mode(node: &mut PenNode, mode: ImageFillMode) -> bool {
    let Some(body) = primary_image_fill_mut(node) else {
        return false;
    };
    body.mode = Some(mode.to_schema());
    true
}

/// Set one primary image-fill adjustment, clamped to the TS slider
/// range `[-100, 100]`.
pub fn set_primary_image_adjustment(
    node: &mut PenNode,
    field: ImageAdjustmentField,
    value: f32,
) -> bool {
    let Some(body) = primary_image_fill_mut(node) else {
        return false;
    };
    let value = value.clamp(-100.0, 100.0);
    match field {
        ImageAdjustmentField::Exposure => body.exposure = Some(value),
        ImageAdjustmentField::Contrast => body.contrast = Some(value),
        ImageAdjustmentField::Saturation => body.saturation = Some(value),
        ImageAdjustmentField::Temperature => body.temperature = Some(value),
        ImageAdjustmentField::Tint => body.tint = Some(value),
        ImageAdjustmentField::Highlights => body.highlights = Some(value),
        ImageAdjustmentField::Shadows => body.shadows = Some(value),
    }
    true
}

/// Reset every primary image-fill adjustment to zero.
pub fn reset_primary_image_adjustments(node: &mut PenNode) -> bool {
    let Some(body) = primary_image_fill_mut(node) else {
        return false;
    };
    body.exposure = Some(0.0);
    body.contrast = Some(0.0);
    body.saturation = Some(0.0);
    body.temperature = Some(0.0);
    body.tint = Some(0.0);
    body.highlights = Some(0.0);
    body.shadows = Some(0.0);
    true
}

/// Write the primary fill's `opacity` (clamped to `[0.0, 1.0]`),
/// matching on whatever variant the first fill is. Touches no
/// other field — a gradient / image fill keeps its stops, image
/// url, etc. `false` (no-op) when the variant carries no `fill`
/// field or the node has no fills.
pub fn set_primary_fill_opacity(node: &mut PenNode, opacity: f32) -> bool {
    // Read-only probe first: `node_fills_mut` would `get_or_insert_with`
    // an empty Vec, silently mutating `fill: None` into
    // `fill: Some([])`. Bail before touching the document when
    // there's nothing to update.
    if node_fills(node).map(|f| f.is_empty()).unwrap_or(true) {
        return false;
    }
    let opacity = opacity.clamp(0.0, 1.0);
    let Some(fills) = node_fills_mut(node) else {
        return false;
    };
    let Some(first) = fills.first_mut() else {
        return false;
    };
    match first {
        PenFill::Solid(b) => b.opacity = Some(opacity),
        PenFill::LinearGradient(b) => b.opacity = Some(opacity),
        PenFill::RadialGradient(b) => b.opacity = Some(opacity),
        PenFill::Image(b) => b.opacity = Some(opacity),
    }
    true
}

/// Set the LinearGradient body's `angle` (degrees, canonical
/// `.op` convention — 0° = bottom→top). No-op when the first fill
/// isn't a linear gradient; returns `false` so callers can detect
/// the silent rejection (panel input clears without mutation).
pub fn set_primary_gradient_angle(node: &mut PenNode, angle_deg: f32) -> bool {
    if node_fills(node).map(|f| f.is_empty()).unwrap_or(true) {
        return false;
    }
    let Some(fills) = node_fills_mut(node) else {
        return false;
    };
    let Some(first) = fills.first_mut() else {
        return false;
    };
    match first {
        PenFill::LinearGradient(b) => {
            b.angle = Some(angle_deg);
            true
        }
        _ => false,
    }
}

/// Replace gradient stop `index`'s colour with `hex` (already
/// validated `#RRGGBB`). Linear + Radial both accepted. No-op when
/// the first fill isn't a gradient or `index` is out of range.
pub fn set_primary_gradient_stop_hex(node: &mut PenNode, index: usize, hex: &str) -> bool {
    if node_fills(node).map(|f| f.is_empty()).unwrap_or(true) {
        return false;
    }
    let Some(fills) = node_fills_mut(node) else {
        return false;
    };
    let Some(first) = fills.first_mut() else {
        return false;
    };
    let stops = match first {
        PenFill::LinearGradient(b) => &mut b.stops,
        PenFill::RadialGradient(b) => &mut b.stops,
        _ => return false,
    };
    let Some(stop) = stops.get_mut(index) else {
        return false;
    };
    stop.color = hex.to_string();
    true
}

/// Replace gradient stop `index`'s offset with `frac` (0.0..=1.0).
/// Linear + Radial both accepted. Same no-op rules as the hex
/// setter; offset is clamped before write so the canonical schema's
/// invariant (`0 ≤ offset ≤ 1`) holds.
pub fn set_primary_gradient_stop_offset(node: &mut PenNode, index: usize, frac: f32) -> bool {
    if node_fills(node).map(|f| f.is_empty()).unwrap_or(true) {
        return false;
    }
    let Some(fills) = node_fills_mut(node) else {
        return false;
    };
    let Some(first) = fills.first_mut() else {
        return false;
    };
    let stops = match first {
        PenFill::LinearGradient(b) => &mut b.stops,
        PenFill::RadialGradient(b) => &mut b.stops,
        _ => return false,
    };
    let Some(stop) = stops.get_mut(index) else {
        return false;
    };
    stop.offset = frac.clamp(0.0, 1.0);
    true
}

fn primary_gradient_stops_mut(node: &mut PenNode) -> Option<&mut Vec<GradientStop>> {
    if node_fills(node).map(|f| f.is_empty()).unwrap_or(true) {
        return None;
    }
    let fills = node_fills_mut(node)?;
    match fills.first_mut()? {
        PenFill::LinearGradient(b) => Some(&mut b.stops),
        PenFill::RadialGradient(b) => Some(&mut b.stops),
        _ => None,
    }
}

pub fn add_primary_gradient_stop(node: &mut PenNode) -> bool {
    let Some(stops) = primary_gradient_stops_mut(node) else {
        return false;
    };
    let last_offset = stops.last().map(|s| s.offset).unwrap_or(0.5);
    stops.push(GradientStop {
        offset: (last_offset + 0.1).min(1.0),
        color: "#888888".to_string(),
    });
    true
}

pub fn remove_primary_gradient_stop(node: &mut PenNode, index: usize) -> bool {
    let Some(stops) = primary_gradient_stops_mut(node) else {
        return false;
    };
    if stops.len() <= 2 || index >= stops.len() {
        return false;
    }
    stops.remove(index);
    true
}

/// Replace the first `Solid` fill's colour with `hex`, leaving any
/// gradient / image fills untouched. When the node has no solid fill,
/// a fresh one is prepended so it paints on top. `false` when the
/// variant carries no `fill` field at all.
pub fn set_primary_fill_hex(node: &mut PenNode, hex: &str) -> bool {
    let Some(fills) = node_fills_mut(node) else {
        return false;
    };
    if let Some(slot) = fills.iter_mut().find_map(|f| match f {
        PenFill::Solid(body) => Some(body),
        _ => None,
    }) {
        slot.color = hex.to_string();
    } else {
        fills.insert(0, solid_fill(hex.to_string()));
    }
    true
}

/// Read the node's primary fill kind as a [`FillType`]. The canonical
/// model has no scalar `fill_type` field — the kind is the variant of
/// the first `PenFill`. A node with no fills reports `Solid` (the
/// neutral default the property panel paints).
pub fn first_fill_type(node: &PenNode) -> FillType {
    match node_fills(node).and_then(|f| f.first()) {
        Some(PenFill::Solid(_)) | None => FillType::Solid,
        Some(PenFill::LinearGradient(_)) => FillType::LinearGradient,
        Some(PenFill::RadialGradient(_)) => FillType::RadialGradient,
        Some(PenFill::Image(_)) => FillType::Image,
    }
}

/// Build a default `PenFill` of the given `FillType`, seeding it with
/// `hex` where the variant carries a single colour (Solid) or a stop
/// list (gradients). `Image` has no colour, so it gets an empty `url`.
fn default_fill_of_type(kind: FillType, hex: &str) -> PenFill {
    match kind {
        FillType::Solid => solid_fill(hex.to_string()),
        FillType::LinearGradient => PenFill::LinearGradient(LinearGradientBody {
            angle: None,
            stops: default_stops(hex),
            explain: None,
            opacity: None,
            blend_mode: None,
        }),
        FillType::RadialGradient => PenFill::RadialGradient(RadialGradientBody {
            cx: None,
            cy: None,
            radius: None,
            stops: default_stops(hex),
            explain: None,
            opacity: None,
            blend_mode: None,
        }),
        FillType::Image => PenFill::Image(ImageFillBody {
            url: String::new(),
            mode: None,
            original_size: None,
            transform: None,
            explain: None,
            opacity: None,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
        }),
    }
}

/// Two-stop gradient default — the picked colour at 0.0, transparent
/// black at 1.0, mirroring the property panel's 2-stop gradient body.
fn default_stops(hex: &str) -> Vec<GradientStop> {
    vec![
        GradientStop {
            offset: 0.0,
            color: hex.to_string(),
        },
        GradientStop {
            offset: 1.0,
            color: "#00000000".to_string(),
        },
    ]
}

/// Carry a representative hex colour out of a `PenFill` so flipping
/// fill types keeps the node's colour where one exists. Solid → its
/// colour; gradient → the first stop's colour; image → none.
fn fill_hex(fill: &PenFill) -> Option<&str> {
    match fill {
        PenFill::Solid(body) => Some(body.color.as_str()),
        PenFill::LinearGradient(body) => body.stops.first().map(|s| s.color.as_str()),
        PenFill::RadialGradient(body) => body.stops.first().map(|s| s.color.as_str()),
        PenFill::Image(_) => None,
    }
}

/// Convert an existing first `PenFill` to fill-type `kind`, preserving
/// as much of the existing body as the target variant structurally
/// allows. This mirrors shell-core's `set_selected_fill_type`, which
/// only flipped a scalar `Node.fill_type` discriminant and never
/// discarded the fill body: shell-core kept the gradient stops / image
/// payload while the discriminant moved. The canonical model has no
/// scalar discriminant — type IS the `PenFill` variant — so a faithful
/// port carries the body across the variant flip by hand:
///
///   - already the target variant → returned unchanged (no-op).
///   - LinearGradient ⇄ RadialGradient → carry `stops`, `opacity`,
///     `blend_mode`, `explain`; only the angle / centre fields that
///     have no counterpart are dropped.
///   - Solid → gradient → seed stops from the solid colour.
///   - gradient → Solid → carry the first stop's colour.
///   - anything → Image → fresh image body (no shared structure to
///     carry; the previous body cannot become a URL).
fn convert_fill(existing: PenFill, kind: FillType) -> PenFill {
    match (kind, existing) {
        // Already the requested variant — keep the body verbatim.
        (FillType::Solid, f @ PenFill::Solid(_)) => f,
        (FillType::LinearGradient, f @ PenFill::LinearGradient(_)) => f,
        (FillType::RadialGradient, f @ PenFill::RadialGradient(_)) => f,
        (FillType::Image, f @ PenFill::Image(_)) => f,
        // Linear → Radial — carry every shared field.
        (FillType::RadialGradient, PenFill::LinearGradient(body)) => {
            PenFill::RadialGradient(RadialGradientBody {
                cx: None,
                cy: None,
                radius: None,
                stops: body.stops,
                explain: body.explain,
                opacity: body.opacity,
                blend_mode: body.blend_mode,
            })
        }
        // Radial → Linear — carry every shared field.
        (FillType::LinearGradient, PenFill::RadialGradient(body)) => {
            PenFill::LinearGradient(LinearGradientBody {
                angle: None,
                stops: body.stops,
                explain: body.explain,
                opacity: body.opacity,
                blend_mode: body.blend_mode,
            })
        }
        // Cross-family flips — carry the representative colour only.
        (kind, other) => {
            let hex = fill_hex(&other).unwrap_or("#000000").to_string();
            default_fill_of_type(kind, &hex)
        }
    }
}

/// Set the node's primary fill kind to `kind`. The canonical model
/// encodes fill type as the first `PenFill` variant, so this converts
/// the first fill to the requested variant via [`convert_fill`] —
/// preserving as much of the existing body (gradient stops, opacity,
/// blend mode) as the target variant allows — or prepends a default
/// body when the node has no fills. Non-first fills are left untouched.
/// `false` for variants that carry no `fill` field at all.
pub fn set_primary_fill_type(node: &mut PenNode, kind: FillType) -> bool {
    let Some(fills) = node_fills_mut(node) else {
        return false;
    };
    if fills.is_empty() {
        fills.push(default_fill_of_type(kind, "#000000"));
    } else {
        let existing = fills.remove(0);
        fills.insert(0, convert_fill(existing, kind));
    }
    true
}

pub fn clear_primary_fills(node: &mut PenNode) -> bool {
    let Some(fills) = node_fills_mut(node) else {
        return false;
    };
    fills.clear();
    true
}

/// Stroke parallel to [`set_primary_fill_hex`]. Creates a default
/// 1-px stroke when the node has none, so a colour write always
/// lands a visible stroke. `false` for variants without a stroke.
pub fn set_primary_stroke_hex(node: &mut PenNode, hex: &str) -> bool {
    let Some(slot) = node_stroke_mut(node) else {
        return false;
    };
    let stroke = slot.get_or_insert_with(|| PenStroke {
        thickness: StrokeThickness::Uniform(1.0),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: None,
    });
    let fills = stroke.fill.get_or_insert_with(Vec::new);
    if let Some(body) = fills.iter_mut().find_map(|f| match f {
        PenFill::Solid(body) => Some(body),
        _ => None,
    }) {
        body.color = hex.to_string();
    } else {
        fills.insert(0, solid_fill(hex.to_string()));
    }
    true
}

/// Append a default drop-shadow effect — mirrors a common CSS card
/// shadow (`0 4px 8px rgba(0,0,0,0.25)`). `false` for variants that
/// carry no `effects` field.
pub fn push_drop_shadow(node: &mut PenNode) -> bool {
    let Some(effects) = node_effects_mut(node) else {
        return false;
    };
    effects.push(PenEffect::Shadow(ShadowBody {
        inner: None,
        offset_x: 0.0,
        offset_y: 4.0,
        blur: 8.0,
        spread: 0.0,
        color: "#00000040".to_string(),
    }));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare rectangle node fixture, parsed from `.op` JSON so it
    /// stays robust to schema growth.
    fn rect_node() -> PenNode {
        let src = r#"{"version":"0.8.0","children":[
            {"type":"rectangle","id":"r1","name":"R",
             "x":0,"y":0,"width":10,"height":10}
        ]}"#;
        jian_ops_schema::load_str(src)
            .expect("fixture parses")
            .value
            .children
            .into_iter()
            .next()
            .expect("one node")
    }

    /// Seed a node with a custom 3-stop linear gradient as its first
    /// fill so the conversion has a non-default body to preserve.
    fn seed_linear_gradient(node: &mut PenNode) {
        let fills = node_fills_mut(node).expect("rect carries fills");
        fills.clear();
        fills.push(PenFill::LinearGradient(LinearGradientBody {
            angle: Some(45.0),
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: "#ff0000".into(),
                },
                GradientStop {
                    offset: 0.5,
                    color: "#00ff00".into(),
                },
                GradientStop {
                    offset: 1.0,
                    color: "#0000ff".into(),
                },
            ],
            explain: None,
            opacity: Some(0.75),
            blend_mode: None,
        }));
    }

    #[test]
    fn linear_to_radial_preserves_the_gradient_body() {
        // Fix 6: a fill-type discriminant change must not discard the
        // existing gradient payload — shell-core's `set_selected_fill_type`
        // only flipped a scalar `Node.fill_type` and kept the body, so
        // the canonical port carries the stops / opacity across.
        let mut node = rect_node();
        seed_linear_gradient(&mut node);

        assert!(set_primary_fill_type(&mut node, FillType::RadialGradient));

        let fills = node_fills(&node).expect("rect carries fills");
        match fills.first().expect("a first fill") {
            PenFill::RadialGradient(body) => {
                // The full 3-stop list survived the variant flip.
                assert_eq!(body.stops.len(), 3);
                assert_eq!(body.stops[0].color, "#ff0000");
                assert_eq!(body.stops[1].color, "#00ff00");
                assert_eq!(body.stops[2].color, "#0000ff");
                // Opacity carried across too — not reset to default.
                assert_eq!(body.opacity, Some(0.75));
            }
            other => panic!("expected RadialGradient, got {other:?}"),
        }
    }

    #[test]
    fn flipping_back_and_forth_keeps_the_stops() {
        // Linear → Radial → Linear round-trip must still carry the
        // custom stops (angle has no radial counterpart, so it is the
        // one field allowed to drop).
        let mut node = rect_node();
        seed_linear_gradient(&mut node);

        assert!(set_primary_fill_type(&mut node, FillType::RadialGradient));
        assert!(set_primary_fill_type(&mut node, FillType::LinearGradient));

        let fills = node_fills(&node).expect("rect carries fills");
        match fills.first().expect("a first fill") {
            PenFill::LinearGradient(body) => {
                assert_eq!(body.stops.len(), 3);
                assert_eq!(body.stops[0].color, "#ff0000");
                assert_eq!(body.opacity, Some(0.75));
            }
            other => panic!("expected LinearGradient, got {other:?}"),
        }
    }

    #[test]
    fn same_type_is_a_no_op_keeping_the_exact_body() {
        // Setting the type the node already has must leave the body
        // byte-for-byte identical (no default-body overwrite).
        let mut node = rect_node();
        seed_linear_gradient(&mut node);
        let before = node_fills(&node).unwrap().first().cloned();

        assert!(set_primary_fill_type(&mut node, FillType::LinearGradient));

        let after = node_fills(&node).unwrap().first().cloned();
        assert_eq!(before, after);
    }

    #[test]
    fn solid_to_gradient_seeds_stops_from_the_solid_colour() {
        // Cross-family flip: there is no gradient body to carry, so the
        // representative colour seeds the first stop.
        let mut node = rect_node();
        {
            let fills = node_fills_mut(&mut node).unwrap();
            fills.clear();
            fills.push(solid_fill("#abcdef".into()));
        }
        assert!(set_primary_fill_type(&mut node, FillType::LinearGradient));
        match node_fills(&node).unwrap().first().unwrap() {
            PenFill::LinearGradient(body) => {
                assert_eq!(
                    body.stops.first().map(|s| s.color.as_str()),
                    Some("#abcdef")
                );
            }
            other => panic!("expected LinearGradient, got {other:?}"),
        }
    }
}
