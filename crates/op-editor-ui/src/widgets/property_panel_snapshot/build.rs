//! `impl NodeSnapshot` — the stroke accessors plus the three snapshot
//! constructors (single node, multi-selection aggregate, and the
//! neutral Code-tab placeholder).
//!
//! Split out of `property_panel_snapshot.rs` to keep both files under
//! the openpencil 800-line cap.

use super::node_props::*;
use super::widget_props::{text_summary_of, widget_summary_of};
use super::{color_from_hex, EffectSummary, LayoutPaddingSummary, NodeSnapshot};
use super::{fill_props, node_kind_of, stroke_sides_for_scene};
use crate::layout_scene::{NodeKind, SceneStroke, SceneStrokeAlign};
use crate::widgets::property_panel_action::{LayoutAlignValue, LayoutJustifyValue};
use crate::Color;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingKeyword;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

impl NodeSnapshot {
    /// Slate-gray placeholder shown for the stroke swatch + hex input
    /// when the node has no parseable solid stroke (`#374151`). The paint
    /// routine AND the edit-seed path MUST read this same value so that
    /// clicking into the hex input doesn't change the displayed color.
    pub(crate) const DEFAULT_STROKE_SWATCH: Color = Color {
        r: 0x37 as f32 / 255.0,
        g: 0x41 as f32 / 255.0,
        b: 0x51 as f32 / 255.0,
        a: 1.0,
    };

    /// The color the stroke swatch + hex input should display: the
    /// node's solid stroke color, or the slate placeholder when unset.
    /// Single source of truth so paint and the click-to-edit seed can
    /// never drift (the bug where the seed defaulted to `#000000`).
    /// `pub` so the host seed path (op-host-native) reads the same value.
    pub fn stroke_swatch_color(&self) -> Color {
        self.stroke
            .map(|s| s.color)
            .unwrap_or(Self::DEFAULT_STROKE_SWATCH)
    }

    /// Stroke widths in `[top, right, bottom, left]` order for the
    /// side-specific stroke controls. Uniform strokes expand to all
    /// four sides so each input has a concrete value.
    pub fn stroke_side_widths(&self) -> [f32; 4] {
        self.stroke
            .map(|stroke| stroke.sides.unwrap_or([stroke.width; 4]))
            .unwrap_or([0.0; 4])
    }

    /// One side's stroke width for focus seeding.
    pub fn stroke_side_width_for(&self, focus: op_editor_core::PropertyFocus) -> Option<f32> {
        use op_editor_core::PropertyFocus as F;
        let widths = self.stroke_side_widths();
        match focus {
            F::StrokeTopWidth => Some(widths[0]),
            F::StrokeRightWidth => Some(widths[1]),
            F::StrokeBottomWidth => Some(widths[2]),
            F::StrokeLeftWidth => Some(widths[3]),
            _ => None,
        }
    }

    /// Build an aggregate snapshot for a multi-node selection.
    /// Returns None when nothing on the active page resolves from
    /// `selected_set`. Uses `Document::selection_bounds` (the union
    /// of every selected node's `aggregate_bounds`) for x/y/w/h.
    /// Rotation / fill / stroke are zeroed in v1 — broadcasting
    /// "Mixed" or per-axis aggregation is a follow-up; the panel
    /// hides those inputs anyway since `is_multi` flips them
    /// inert.
    /// Neutral placeholder snapshot for the selection-independent Code
    /// tab: the panel must stay alive with an EMPTY selection (the TS
    /// code-panel falls back to the active page's children), but the
    /// Design sections that read the snapshot are never painted on the
    /// Code tab, so every field takes the same inert defaults as the
    /// multi-select aggregate.
    pub(crate) fn empty_for_code_tab() -> Self {
        Self {
            kind: String::new(),
            name: String::new(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            opacity_percent: 100.0,
            blend_mode: None,
            mask_type: None,
            rotation_deg: 0.0,
            corner_radius: 0.0,
            corner_radii: [0.0; 4],
            supports_per_corner: false,
            polygon_sides: None,
            ellipse_arc: None,
            flex_layout: op_editor_core::FlexLayout::Free,
            layout_justify: LayoutJustifyValue::Start,
            layout_align: LayoutAlignValue::Start,
            layout_gap: 0.0,
            layout_padding: LayoutPaddingSummary::ZERO,
            size_fill_width: false,
            size_fill_height: false,
            size_hug_width: false,
            size_hug_height: false,
            size_clip_content: false,
            can_clip_content: false,
            has_corner_radius: false,
            can_create_component: false,
            is_image_node: false,
            video: None,
            icon: None,
            text: None,
            widget: None,
            fill: None,
            fill_opacity: 1.0,
            fills: Vec::new(),
            path_fill_rule: None,
            stroke: None,
            gradient_angle: None,
            gradient_stops: Vec::new(),
            image_fill: None,
            effects: Vec::new(),
            interactions: crate::widgets::property_panel_interactions::InteractionSummary::default(
            ),
            // Neutral default — the Code tab never consults the
            // kind-driven capability mask.
            kind_variant: NodeKind::Frame,
            is_instance: false,
            is_reusable: false,
        }
    }

    pub(crate) fn from_multi_selection(state: &EditorState) -> Option<Self> {
        // Confirm at least 2 selected ids resolve on the active
        // page — bails on cross-page selections but NOT on
        // all-zero-size selections (matches single-select
        // semantics, which paint the panel even for a 0x0 node).
        if state.selection_count() < 2 {
            return None;
        }
        // `selection_bounds` returns `None` when nothing resolves;
        // an empty union still paints (zeroed) like single-select.
        if state.selected_node().is_none() && state.selection_bounds().is_none() {
            return None;
        }
        let bounds = state
            .selection_bounds()
            .unwrap_or(op_editor_core::DocRect::ZERO);
        let n = state.selection_count();
        Some(Self {
            kind: format!("{} items", n),
            name: format!("{} selected", n),
            x: bounds.x.round() as i32,
            y: bounds.y.round() as i32,
            width: bounds.w.round() as i32,
            height: bounds.h.round() as i32,
            opacity_percent: 100.0,
            blend_mode: None,
            mask_type: None,
            rotation_deg: 0.0,
            corner_radius: 0.0,
            corner_radii: [0.0; 4],
            supports_per_corner: false,
            polygon_sides: None,
            ellipse_arc: None,
            flex_layout: op_editor_core::FlexLayout::Free,
            layout_justify: LayoutJustifyValue::Start,
            layout_align: LayoutAlignValue::Start,
            layout_gap: 0.0,
            layout_padding: LayoutPaddingSummary::ZERO,
            size_fill_width: false,
            size_fill_height: false,
            size_hug_width: false,
            size_hug_height: false,
            size_clip_content: false,
            can_clip_content: false,
            has_corner_radius: false,
            can_create_component: false,
            is_image_node: false,
            video: None,
            icon: None,
            text: None,
            widget: None,
            fill: None,
            fill_opacity: 1.0,
            // Multi-select hides the Fill section (see `for_multi`),
            // so it carries no per-fill rows.
            fills: Vec::new(),
            path_fill_rule: None,
            stroke: None,
            gradient_angle: None,
            gradient_stops: Vec::new(),
            image_fill: None,
            // Multi-select shows no per-effect rows — the Effects
            // section paints just its header + the add affordance.
            effects: Vec::new(),
            // Multi-select hides Interactions entirely (see
            // `SectionCapabilities::for_multi`) — editing one node's
            // `events` across a broadcast selection is a follow-up.
            interactions: crate::widgets::property_panel_interactions::InteractionSummary::default(
            ),
            // `kind_variant` is informational for the snapshot
            // header label only — the paint capability mask is
            // driven by `SectionCapabilities::for_multi()` instead
            // of `for_kind`, see `paint`. Frame chosen so any
            // future kind-specific lookups paint a neutral default.
            kind_variant: NodeKind::Frame,
            is_instance: false,
            is_reusable: false,
        })
    }

    /// Build the snapshot from a canonical `PenNode`. Geometry uses
    /// `aggregate_bounds` so Group / unbounded container nodes report
    /// the visual extent of their subtree instead of "0 × 0".
    /// `is_top_level` gates the Interactions section's Screen row —
    /// only a page-root child's `screen` marker is ever meaningful
    /// (see `wire_screen_navigation`'s contract); the caller alone
    /// knows the selection's position in the tree.
    pub(crate) fn from_node(node: &PenNode, is_top_level: bool) -> Self {
        let base = node.base();
        let kind = node_kind_of(node);
        let bounds = op_editor_core::aggregate_bounds(node);
        // Corner radius — only the container variants carry one;
        // a `PerCorner` radius reports its top-left corner.
        let corner_radius = container_corner_radius(node);
        let corner_radii = container_corner_radii(node).unwrap_or([corner_radius; 4]);
        let fill = op_editor_core::first_solid_fill_hex(node).and_then(color_from_hex);
        // A node can carry a stroke WIDTH with no solid color — e.g. set
        // via the width input before a color is chosen, where
        // `cmd_set_node_stroke_width` attaches a fresh stroke with
        // `fill: None`. Surface the stroke whenever it has any width so the
        // width input + side controls reflect it; fall back to the slate
        // placeholder color (matching `stroke_swatch_color`) when no solid
        // color is parseable. Gating `stroke` on a parseable color used to
        // drop a colorless stroke's width, so the width read back "0".
        let stroke_color = op_editor_core::first_solid_stroke_hex(node).and_then(color_from_hex);
        let stroke_width = op_editor_core::fills::node_stroke_width(node);
        let stroke = match (stroke_color, stroke_width) {
            (Some(color), _) => Some(SceneStroke {
                color,
                width: stroke_width.unwrap_or(1.0) as f32,
                sides: stroke_sides_for_scene(node),
                align: SceneStrokeAlign::Center,
            }),
            (None, Some(width)) if width > 0.0 => Some(SceneStroke {
                color: Self::DEFAULT_STROKE_SWATCH,
                width: width as f32,
                sides: stroke_sides_for_scene(node),
                align: SceneStrokeAlign::Center,
            }),
            _ => None,
        };
        Self {
            kind: kind.label().to_string(),
            name: base.name.clone().unwrap_or_default(),
            x: bounds.x.round() as i32,
            y: bounds.y.round() as i32,
            width: bounds.w.round() as i32,
            height: bounds.h.round() as i32,
            opacity_percent: node_opacity_percent(node),
            blend_mode: op_editor_core::node_blend_mode(node),
            mask_type: op_editor_core::node_mask_type(node),
            // `base.rotation` is stored in degrees by the canonical
            // schema; the snapshot's `rotation_deg` wants degrees.
            rotation_deg: base.rotation.unwrap_or(0.0) as f32,
            corner_radius,
            corner_radii,
            supports_per_corner: container_corner_radii(node).is_some(),
            polygon_sides: polygon_sides_of(node),
            ellipse_arc: ellipse_arc_of(node),
            flex_layout: flex_layout_of(node),
            layout_justify: layout_justify_of(node),
            layout_align: layout_align_of(node),
            layout_gap: layout_gap_of(node),
            layout_padding: layout_padding_of(node),
            size_fill_width: sizing_is(node_width_sizing(node), SizingKeyword::FillContainer),
            size_fill_height: sizing_is(node_height_sizing(node), SizingKeyword::FillContainer),
            size_hug_width: sizing_is(node_width_sizing(node), SizingKeyword::FitContent),
            size_hug_height: sizing_is(node_height_sizing(node), SizingKeyword::FitContent),
            size_clip_content: clip_content_of(node),
            can_clip_content: can_clip_content(node),
            has_corner_radius: has_corner_radius(node),
            can_create_component: can_create_component(node),
            is_image_node: matches!(node, PenNode::Image(_)),
            video: video_summary_of(node),
            icon: icon_summary_of(node),
            text: text_summary_of(node),
            widget: widget_summary_of(node),
            fill,
            fill_opacity: op_editor_core::first_solid_fill_opacity(node),
            fills: fill_props::fills_of(node),
            path_fill_rule: match node {
                PenNode::Path(path) => Some(
                    path.fill_rule
                        .unwrap_or(jian_ops_schema::node::path::PathFillRule::Nonzero),
                ),
                _ => None,
            },
            stroke,
            gradient_angle: fill_props::gradient_angle_of(node),
            gradient_stops: fill_props::gradient_stops_of(node),
            image_fill: op_editor_core::first_image_fill_summary(node)
                .or_else(|| op_editor_core::image_node_summary(node)),
            effects: op_editor_core::node_effects(node)
                .iter()
                .map(EffectSummary::from_pen_effect)
                .collect(),
            interactions: crate::widgets::property_panel_interactions::interactions_of(
                node,
                is_top_level,
            ),
            kind_variant: kind,
            is_instance: false,
            is_reusable: matches!(node, PenNode::Frame(f) if f.reusable == Some(true)),
        }
    }
}
