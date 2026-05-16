//! Fill / stroke / effect read-write helpers for `PenNode`.
//!
//! shell-core's flat `Node` carried `fill: Option<Color>` and a
//! `stroke: Option<Stroke>` — a single literal colour per channel.
//! The canonical `PenNode` is richer: every paintable variant carries
//! `fill: Option<Vec<PenFill>>` (where each `PenFill` is a tagged
//! `Solid` / gradient / `Image` body with hex `String` colours), and
//! the stroke colour lives inside `PenStroke::fill` as its own
//! `Vec<PenFill>`.
//!
//! The colour-picker + property-panel mutators only ever care about
//! "the node's primary solid colour" — a single hex. This module is
//! the shim that reads / writes exactly that:
//!
//!   - [`first_solid_fill_hex`] — read the first `Solid` fill's hex.
//!   - [`set_primary_fill_hex`] — replace the first `Solid` fill (or
//!     prepend one) with a new hex, keeping any non-solid fills.
//!   - the stroke parallel ([`first_solid_stroke_hex`] /
//!     [`set_primary_stroke_hex`]).
//!   - [`push_drop_shadow`] — append a default drop-shadow effect.
//!
//! Gradient / image fills are preserved verbatim — a hex write only
//! ever touches the *first solid* entry, mirroring shell-core's
//! single-colour behaviour without flattening the canonical model.

use crate::editor_ui_state::FillType;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{
    GradientStop, ImageFillBody, LinearGradientBody, PenEffect, PenFill, PenStroke,
    RadialGradientBody, ShadowBody, SolidFillBody, StrokeThickness,
};

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

/// First `Solid` fill's hex string on the node's stroke.
pub fn first_solid_stroke_hex(node: &PenNode) -> Option<&str> {
    let stroke = node_stroke(node)?;
    stroke.fill.as_ref()?.iter().find_map(|f| match f {
        PenFill::Solid(body) => Some(body.color.as_str()),
        _ => None,
    })
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

/// Set the node's primary fill kind to `kind`. The canonical model
/// encodes fill type as the first `PenFill` variant, so this replaces
/// the first fill with a fresh body of the requested variant
/// (preserving the old fill's representative colour), or prepends one
/// when the node has no fills. Non-first fills are left untouched.
/// `false` for variants that carry no `fill` field at all.
pub fn set_primary_fill_type(node: &mut PenNode, kind: FillType) -> bool {
    let Some(fills) = node_fills_mut(node) else {
        return false;
    };
    let hex = fills
        .first()
        .and_then(fill_hex)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "#000000".to_string());
    let fresh = default_fill_of_type(kind, &hex);
    if fills.is_empty() {
        fills.push(fresh);
    } else {
        fills[0] = fresh;
    }
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
