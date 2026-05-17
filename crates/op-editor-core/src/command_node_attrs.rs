//! Per-node attribute command application — `SetNodeRotation` /
//! `SetNodeText` / `SetNodeCornerRadius` / `SetNodeFontSize` /
//! `SetNodeFontWeight` / `SetNodeStrokeHex` / `SetNodeStrokeWidth` /
//! `SetNodeFillHex` / `SetNodeName` / `SetNodeFlag`.
//!
//! Ported from shell-core's `mcp_apply_node_attrs.rs`, retargeted onto
//! the canonical `jian_ops_schema::PenNode`. shell-core's flat `Node`
//! carried a single `fill` / `stroke` / `corner_radius`; `PenNode`
//! spreads those across per-variant fields, so these helpers route
//! through [`crate::fills`] + per-variant matches.
//!
//! Each helper keeps the validate-then-mutate discipline: kind / range
//! / hex checks happen BEFORE the mutable borrow + write.

use crate::command::{EffectField, NodeFlag};
use crate::fills::{set_primary_fill_hex, set_primary_stroke_hex};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers::find_node_mut;
use jian_ops_schema::node::{CornerRadius, FontWeight, PenNode, TextContent};
use jian_ops_schema::style::{BlurBody, PenEffect, PenStroke, ShadowBody, StrokeThickness};

/// Write a literal corner radius onto whatever variant carries one.
/// Frame / Group / Rectangle store `CornerRadius` on `container`;
/// Ellipse / Polygon carry an `f64`. Other kinds accept the call as a
/// silent no-op (parity with shell-core, where the radius was simply
/// invisible on non-rounded kinds). True when a field was written.
fn write_corner_radius(node: &mut PenNode, radius: f64) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.corner_radius = Some(CornerRadius::Uniform(radius));
            true
        }
        PenNode::Group(n) => {
            n.container.corner_radius = Some(CornerRadius::Uniform(radius));
            true
        }
        PenNode::Rectangle(n) => {
            n.container.corner_radius = Some(CornerRadius::Uniform(radius));
            true
        }
        PenNode::Ellipse(n) => {
            n.corner_radius = Some(radius);
            true
        }
        PenNode::Polygon(n) => {
            n.corner_radius = Some(radius);
            true
        }
        // Other kinds have no corner-radius field; the write is a
        // silent no-op so the command still reports success.
        _ => true,
    }
}

/// Mutably borrow whatever variant's `effects` field. Frame / Group /
/// Rectangle keep it on `container`; the leaf kinds carry it directly.
/// `None` for IconFont / Ref (no effects field in the schema).
fn node_effects_slot(node: &mut PenNode) -> Option<&mut Option<Vec<PenEffect>>> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container.effects),
        PenNode::Group(n) => Some(&mut n.container.effects),
        PenNode::Rectangle(n) => Some(&mut n.container.effects),
        PenNode::Ellipse(n) => Some(&mut n.effects),
        PenNode::Polygon(n) => Some(&mut n.effects),
        PenNode::Path(n) => Some(&mut n.effects),
        PenNode::Line(n) => Some(&mut n.effects),
        PenNode::Text(n) => Some(&mut n.effects),
        PenNode::TextInput(n) => Some(&mut n.effects),
        PenNode::Image(n) => Some(&mut n.effects),
        PenNode::IconFont(_) | PenNode::Ref(_) => None,
    }
}

/// Mutably borrow whatever variant's `stroke` field. Mirrors the
/// `fills::node_stroke_mut` arm set. `None` for Text / Image / Ref.
fn node_stroke_slot(node: &mut PenNode) -> Option<&mut Option<PenStroke>> {
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

impl EditorState {
    /// `SetNodeRotation` — write rotation (degrees) on a node.
    pub(crate) fn cmd_set_node_rotation(&mut self, node_id: &NodeId, degrees: f32) -> bool {
        if !node_id.is_real() || !degrees.is_finite() {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        node.base_mut().rotation = Some(degrees as f64);
        true
    }

    /// `SetNodeText` — set the plain-text content of a Text node.
    /// Rejects non-Text kinds (parity with shell-core).
    pub(crate) fn cmd_set_node_text(&mut self, node_id: &NodeId, text: &str) -> bool {
        if !node_id.is_real() {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        match node {
            PenNode::Text(t) => {
                t.content = TextContent::Plain(text.to_string());
                true
            }
            _ => false,
        }
    }

    /// `SetNodeCornerRadius` — write corner radius on a node. Rejects
    /// negative / non-finite values.
    pub(crate) fn cmd_set_node_corner_radius(&mut self, node_id: &NodeId, radius: f32) -> bool {
        if !node_id.is_real() || !radius.is_finite() || radius < 0.0 {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        write_corner_radius(node, radius as f64)
    }

    /// `SetNodeFontSize` — set the font size on a Text node. Rejects
    /// non-Text kinds + non-positive sizes.
    pub(crate) fn cmd_set_node_font_size(&mut self, node_id: &NodeId, font_size: f32) -> bool {
        if !node_id.is_real() || !font_size.is_finite() || font_size <= 0.0 {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        match node {
            PenNode::Text(t) => {
                t.font_size = Some(font_size as f64);
                true
            }
            _ => false,
        }
    }

    /// `SetNodeFontWeight` — set the font weight (1..=1000) on a Text
    /// node. Rejects non-Text kinds + out-of-range weights.
    pub(crate) fn cmd_set_node_font_weight(&mut self, node_id: &NodeId, font_weight: u16) -> bool {
        if !node_id.is_real() || font_weight == 0 || font_weight > 1000 {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        match node {
            PenNode::Text(t) => {
                t.font_weight = Some(FontWeight::Number(font_weight as u32));
                true
            }
            _ => false,
        }
    }

    /// `SetNodeStrokeHex` — set the stroke color on a node. A node with
    /// no stroke gets a fresh 1-px stroke so the color always lands.
    pub(crate) fn cmd_set_node_stroke_hex(&mut self, node_id: &NodeId, hex: &str) -> bool {
        if !node_id.is_real() || crate::color_picker::parse_hex_rgb(hex).is_none() {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        set_primary_stroke_hex(node, hex)
    }

    /// `SetNodeStrokeWidth` — set the stroke width (doc-px) on a node.
    /// Width 0 clears the stroke; width > 0 on a node with no stroke
    /// attaches a fresh stroke at that width.
    pub(crate) fn cmd_set_node_stroke_width(&mut self, node_id: &NodeId, width: f32) -> bool {
        if !node_id.is_real() || !width.is_finite() || width < 0.0 {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        let Some(slot) = node_stroke_slot(node) else {
            return false;
        };
        if width == 0.0 {
            *slot = None;
        } else {
            match slot {
                Some(stroke) => stroke.thickness = StrokeThickness::Uniform(width),
                none @ None => {
                    *none = Some(PenStroke {
                        thickness: StrokeThickness::Uniform(width),
                        align: None,
                        join: None,
                        cap: None,
                        dash_pattern: None,
                        dash_offset: None,
                        fill: None,
                    });
                }
            }
        }
        true
    }

    /// `SetNodeFillHex` — set the fill color on a node by id.
    pub(crate) fn cmd_set_node_fill_hex(&mut self, node_id: &NodeId, hex: &str) -> bool {
        if !node_id.is_real() || crate::color_picker::parse_hex_rgb(hex).is_none() {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        set_primary_fill_hex(node, hex)
    }

    /// `SetNodeName` — rename a node by id. Rejects whitespace-only
    /// names.
    pub(crate) fn cmd_set_node_name(&mut self, node_id: &NodeId, name: &str) -> bool {
        if !node_id.is_real() {
            return false;
        }
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        node.base_mut().name = Some(trimmed.to_string());
        true
    }

    /// `SetNodeFlag` — flip a boolean flag on a node. `Collapsed` has
    /// no canonical-schema field (it is editor-UI-only state), so
    /// the applier rejects it; `Hidden` writes `visible`, `Locked`
    /// writes `locked`.
    pub(crate) fn cmd_set_node_flag(
        &mut self,
        node_id: &NodeId,
        flag: NodeFlag,
        value: bool,
    ) -> bool {
        if !node_id.is_real() {
            return false;
        }
        if matches!(flag, NodeFlag::Collapsed) {
            // No `collapsed` field on `PenNodeBase` — collapse is a
            // layer-panel UI flag, not part of the `.op` document.
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        match flag {
            // `visible == false` is the hidden state, so a `Hidden`
            // write inverts the sense.
            NodeFlag::Hidden => node.base_mut().visible = Some(!value),
            NodeFlag::Locked => node.base_mut().locked = Some(value),
            NodeFlag::Collapsed => unreachable!("rejected above"),
        }
        true
    }

    /// `SetNodeFlip` — write the horizontal / vertical mirror flags on
    /// a node. Either axis `None` leaves that flag untouched. Returns
    /// `false` only on a missing node or when nothing was supplied.
    pub(crate) fn cmd_set_node_flip(
        &mut self,
        node_id: &NodeId,
        flip_x: Option<bool>,
        flip_y: Option<bool>,
    ) -> bool {
        if !node_id.is_real() || (flip_x.is_none() && flip_y.is_none()) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        if let Some(fx) = flip_x {
            node.base_mut().flip_x = Some(fx);
        }
        if let Some(fy) = flip_y {
            node.base_mut().flip_y = Some(fy);
        }
        true
    }

    /// `SetEllipseArc` — write arc geometry on an Ellipse node.
    /// `start_angle` / `sweep_angle` are degrees; `inner_radius` is a
    /// 0.0..=1.0 fraction. Rejects non-Ellipse kinds, non-finite
    /// values, an out-of-range `inner_radius`, and a call that
    /// supplies nothing.
    pub(crate) fn cmd_set_ellipse_arc(
        &mut self,
        node_id: &NodeId,
        start_angle: Option<f64>,
        sweep_angle: Option<f64>,
        inner_radius: Option<f64>,
    ) -> bool {
        if !node_id.is_real() || !self.is_editable(node_id) {
            return false;
        }
        if start_angle.is_none() && sweep_angle.is_none() && inner_radius.is_none() {
            return false;
        }
        if let Some(a) = start_angle {
            if !a.is_finite() {
                return false;
            }
        }
        if let Some(a) = sweep_angle {
            if !a.is_finite() {
                return false;
            }
        }
        if let Some(r) = inner_radius {
            if !r.is_finite() || !(0.0..=1.0).contains(&r) {
                return false;
            }
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        match node {
            PenNode::Ellipse(e) => {
                if let Some(a) = start_angle {
                    e.start_angle = Some(a);
                }
                if let Some(a) = sweep_angle {
                    // A sweep beyond a full turn just over-draws —
                    // clamp to a single revolution either direction.
                    e.sweep_angle = Some(a.clamp(-360.0, 360.0));
                }
                if let Some(r) = inner_radius {
                    e.inner_radius = Some(r);
                }
                true
            }
            _ => false,
        }
    }

    /// `AddNodeEffect` — append a visual effect with default
    /// parameters. `kind` is `"shadow"` / `"blur"` / `"background_blur"`.
    /// Rejects an unknown kind or a node variant with no effects field
    /// (IconFont / Ref).
    pub(crate) fn cmd_add_node_effect(&mut self, node_id: &NodeId, kind: &str) -> bool {
        if !node_id.is_real() {
            return false;
        }
        let effect = match kind {
            "shadow" => PenEffect::Shadow(ShadowBody {
                inner: None,
                offset_x: 4.0,
                offset_y: 4.0,
                blur: 8.0,
                spread: 0.0,
                color: "#00000040".to_string(),
            }),
            "blur" => PenEffect::Blur(BlurBody { radius: 4.0 }),
            "background_blur" => PenEffect::BackgroundBlur(BlurBody { radius: 8.0 }),
            _ => return false,
        };
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        let Some(slot) = node_effects_slot(node) else {
            return false;
        };
        slot.get_or_insert_with(Vec::new).push(effect);
        true
    }

    /// `RemoveNodeEffect` — drop the effect at `index`. Rejects an
    /// out-of-range index; clears the list to `None` once empty so the
    /// serialized `.op` carries no empty `effects` array.
    pub(crate) fn cmd_remove_node_effect(&mut self, node_id: &NodeId, index: u32) -> bool {
        if !node_id.is_real() {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        let Some(slot) = node_effects_slot(node) else {
            return false;
        };
        let Some(effects) = slot.as_mut() else {
            return false;
        };
        let i = index as usize;
        if i >= effects.len() {
            return false;
        }
        effects.remove(i);
        if effects.is_empty() {
            *slot = None;
        }
        true
    }

    /// `SetEffectParam` — write one scalar param of the effect at
    /// `index`. Blur values are clamped to ≥ 0. Rejects a non-finite
    /// value, an out-of-range index, or a field that doesn't match
    /// the effect variant (e.g. `Radius` on a Shadow).
    pub(crate) fn cmd_set_effect_param(
        &mut self,
        node_id: &NodeId,
        index: u32,
        field: EffectField,
        value: f32,
    ) -> bool {
        if !node_id.is_real() || !value.is_finite() {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), node_id) else {
            return false;
        };
        let Some(slot) = node_effects_slot(node) else {
            return false;
        };
        let Some(effects) = slot.as_mut() else {
            return false;
        };
        let Some(effect) = effects.get_mut(index as usize) else {
            return false;
        };
        match (effect, field) {
            (PenEffect::Shadow(s), EffectField::OffsetX) => s.offset_x = value,
            (PenEffect::Shadow(s), EffectField::OffsetY) => s.offset_y = value,
            (PenEffect::Shadow(s), EffectField::Blur) => s.blur = value.max(0.0),
            (PenEffect::Shadow(s), EffectField::Spread) => s.spread = value,
            (PenEffect::Blur(b), EffectField::Radius)
            | (PenEffect::BackgroundBlur(b), EffectField::Radius) => b.radius = value.max(0.0),
            _ => return false,
        }
        true
    }
}
