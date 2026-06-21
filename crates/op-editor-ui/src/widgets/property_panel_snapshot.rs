//! Snapshot extraction for the right-rail `PropertyPanel`.

use crate::layout_scene::{NodeKind, SceneStroke};
use crate::widgets::property_panel_action::{
    LayoutAlignValue, LayoutJustifyValue, TextAlignValue, TextGrowthValue, TextVerticalAlignValue,
};
use crate::Color;
use jian_ops_schema::node::base::{BoolOrExpression, NumberOrExpression};
use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::container::{AlignItems, JustifyContent, Padding};
use jian_ops_schema::node::text::{FontWeight, TextAlign, TextAlignVertical, TextGrowth};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

/// Map a `PenNode` variant onto shell-core's `document::NodeKind`,
/// which drives the per-kind section-capability filtering. The
/// canonical schema's extra variants degrade onto the closest
/// shell-core kind (TextInput → Text; Image / IconFont / Ref →
/// `Other(tag)` so the section mask treats them structurally).
fn node_kind_of(node: &PenNode) -> NodeKind {
    match node {
        PenNode::Frame(_) => NodeKind::Frame,
        PenNode::Group(_) => NodeKind::Group,
        PenNode::Rectangle(_) => NodeKind::Rect,
        PenNode::Ellipse(_) => NodeKind::Ellipse,
        PenNode::Polygon(_) => NodeKind::Polygon,
        PenNode::Line(_) => NodeKind::Line,
        PenNode::Path(_) => NodeKind::Path,
        PenNode::Text(_) | PenNode::TextInput(_) | PenNode::TextArea(_) => NodeKind::Text,
        PenNode::Select(_)
        | PenNode::Switch(_)
        | PenNode::Checkbox(_)
        | PenNode::Slider(_)
        | PenNode::RadioGroup(_)
        | PenNode::NumberInput(_)
        | PenNode::Progress(_)
        | PenNode::Tabs(_) => NodeKind::Rect,
        PenNode::Image(_) => NodeKind::Other("image".to_string()),
        PenNode::IconFont(_) => NodeKind::Other("icon_font".to_string()),
        PenNode::Ref(_) => NodeKind::Other("ref".to_string()),
    }
}

/// Parse a `#RRGGBB` / `#RGB` hex string into a `Color`. Reuses the
/// editor-state colour parser; 8-char `#RRGGBBAA` is honoured so
/// gradient stop swatches (and any other authored alpha) round-trip
/// transparency into paint instead of always reading as opaque.
pub(crate) fn color_from_hex(hex: &str) -> Option<Color> {
    let (r, g, b) = op_editor_core::parse_hex_rgb(hex)?;
    let a = op_editor_core::parse_hex_alpha(hex);
    Some(Color { r, g, b, a })
}

pub(crate) fn stroke_sides_for_scene(node: &PenNode) -> Option<[f32; 4]> {
    let sides = op_editor_core::fills::node_stroke_side_widths(node)?;
    let first = sides[0];
    if sides
        .iter()
        .all(|side| (*side - first).abs() < f32::EPSILON)
    {
        None
    } else {
        Some(sides)
    }
}

/// Snapshot of the selected node's editable fields, formatted for
/// display. Built once per `for_selection` call so all paint
/// helpers can read pre-computed strings instead of re-formatting.
#[derive(Debug, Clone)]
pub struct NodeSnapshot {
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Rotation in degrees (clockwise positive).
    pub rotation_deg: f32,
    /// Uniform corner radius in doc-px.
    pub corner_radius: f32,
    /// Polygon side count, only present for Polygon selections.
    pub polygon_sides: Option<u32>,
    /// Ellipse arc controls, only present for Ellipse selections.
    pub ellipse_arc: Option<EllipseArcSummary>,
    pub flex_layout: op_editor_core::FlexLayout,
    pub layout_justify: LayoutJustifyValue,
    pub layout_align: LayoutAlignValue,
    pub layout_gap: f32,
    pub layout_padding: LayoutPaddingSummary,
    pub size_fill_width: bool,
    pub size_fill_height: bool,
    pub size_hug_width: bool,
    pub size_hug_height: bool,
    pub size_clip_content: bool,
    pub can_clip_content: bool,
    pub has_corner_radius: bool,
    pub can_create_component: bool,
    pub is_image_node: bool,
    pub icon: Option<IconSummary>,
    pub text: Option<TextSummary>,
    /// Form-widget props, `Some` only when the selection is one of the
    /// widget `PenNode` variants. Drives the Widget section's
    /// visibility + rows.
    pub widget: Option<WidgetSummary>,
    pub fill: Option<Color>,
    /// Primary solid-fill opacity in `[0.0, 1.0]` — the Fill
    /// section's `100 %` paints `fill_opacity * 100`.
    pub fill_opacity: f32,
    /// One summary per `PenFill`, in authored order. The Fill section
    /// stacks one editable row per entry (head row + body). The
    /// single-fill `fill` / `fill_opacity` / `gradient_*` / `image_fill`
    /// fields above stay derived from `fills[0]` so the gradient /
    /// image / colour-variable subsystems (which key off the primary
    /// fill) keep compiling unchanged. An old single-fill `.op` loads
    /// as exactly one entry.
    pub fills: Vec<FillSummary>,
    pub stroke: Option<SceneStroke>,
    /// LinearGradient angle in degrees (canonical `.op` convention,
    /// 0° = bottom→top). `None` when the primary fill isn't a
    /// linear gradient — the Fill section hides the angle row in
    /// that case.
    pub gradient_angle: Option<f32>,
    /// Resolved primary-fill gradient stops, in authored order.
    /// Populated for Linear + Radial fills; empty for Solid / Image
    /// / no-fill. Each entry carries the schema hex string (so the
    /// panel input can paint exactly what the file authored) plus
    /// the parsed paint colour for the stop swatch.
    pub gradient_stops: Vec<GradientStopSummary>,
    /// Primary image-fill mode + adjustment values. `None` unless
    /// the selected node's first fill is `PenFill::Image`.
    pub image_fill: Option<op_editor_core::ImageFillSummary>,
    /// The node's visual effects, in paint order — drives the
    /// Effects section's rows + param inputs.
    pub effects: Vec<EffectSummary>,
    /// Drives per-kind section filtering (Line hides fill, etc.).
    pub kind_variant: crate::layout_scene::NodeKind,
    /// True when the selection is a component INSTANCE (`Ref`) shown
    /// through its merged display node — drives the purple badge +
    /// the Go-to-component / Detach-instance rows.
    pub is_instance: bool,
    /// True when the selection is a reusable COMPONENT definition —
    /// drives the purple badge + the Detach-component button.
    pub is_reusable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseArcSummary {
    pub start_deg: f32,
    pub sweep_deg: f32,
    pub inner_percent: f32,
}

/// Which form-widget variant the selected node is — drives the
/// Widget section's per-kind field set. Mirrors the schema's widget
/// `PenNode` variants (`states` overrides are out of scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    TextInput,
    TextArea,
    NumberInput,
    Select,
    RadioGroup,
    Switch,
    Checkbox,
    Slider,
    Progress,
    Tabs,
}

impl WidgetKind {
    /// Whether the kind exposes a `placeholder` field.
    pub fn has_placeholder(self) -> bool {
        matches!(
            self,
            WidgetKind::TextInput
                | WidgetKind::TextArea
                | WidgetKind::NumberInput
                | WidgetKind::Select
        )
    }

    /// Whether the kind exposes a text-typed `value` field the panel
    /// edits as a string (numeric `value` on Slider / NumberInput /
    /// Progress is edited through the min/max/step + numeric inputs,
    /// not the text value row).
    pub fn has_text_value(self) -> bool {
        matches!(
            self,
            WidgetKind::TextInput
                | WidgetKind::TextArea
                | WidgetKind::Select
                | WidgetKind::RadioGroup
        )
    }

    /// Whether the kind carries a `checked` toggle.
    pub fn has_checked(self) -> bool {
        matches!(self, WidgetKind::Switch | WidgetKind::Checkbox)
    }

    /// Whether the kind carries a `label` text field (Checkbox only).
    pub fn has_label(self) -> bool {
        matches!(self, WidgetKind::Checkbox)
    }

    /// Whether the kind exposes the min / max / step numeric trio.
    pub fn has_range(self) -> bool {
        matches!(self, WidgetKind::Slider | WidgetKind::NumberInput)
    }

    /// Whether the kind exposes `leadingIcon` / `trailingIcon` name
    /// fields (icon-bearing input widgets, Phase 1).
    pub fn has_icons(self) -> bool {
        matches!(
            self,
            WidgetKind::TextInput | WidgetKind::TextArea | WidgetKind::NumberInput
        )
    }

    /// Whether the kind exposes the `bind:value` state-key field. The
    /// schema carries `bindings` on every form widget; Phase 1 surfaces
    /// the editor only on the text-bearing input kinds (parity with
    /// where the value/placeholder rows show).
    pub fn has_bind_value(self) -> bool {
        matches!(
            self,
            WidgetKind::TextInput | WidgetKind::TextArea | WidgetKind::NumberInput
        )
    }
}

/// Snapshot of a form-widget node's editable props, formatted for
/// the Widget section. `None` on the snapshot when the selection
/// isn't a widget. The `options` / `tabs` list-editor is deferred
/// (see `property_panel_widget`); the lists are surfaced read-only
/// as a count so the user can see what's authored.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetSummary {
    pub kind: WidgetKind,
    /// `placeholder`, formatted for the input (empty when unset).
    pub placeholder: String,
    /// String `value` (text widgets) — empty when unset / numeric.
    pub value: String,
    /// `label` (Checkbox) — empty when unset.
    pub label: String,
    /// `checked` literal (Switch / Checkbox). `false` for an
    /// expression-bound or unset value.
    pub checked: bool,
    /// `min` / `max` / `step`, formatted (empty when unset).
    pub min: String,
    pub max: String,
    pub step: String,
    /// Count of authored `options` (Select / RadioGroup) — read-only
    /// affordance; the row editor is a follow-up.
    pub option_count: usize,
    /// Count of authored `tabs` (Tabs) — read-only affordance.
    pub tab_count: usize,
    /// `leadingIcon` / `trailingIcon` lucide glyph names (input
    /// widgets) — empty when unset. The Widget section edits these.
    pub leading_icon: String,
    pub trailing_icon: String,
    /// The `bind:value` state key, with the `$state.` prefix stripped
    /// for display (e.g. `email` for `bindings."bind:value" =
    /// "$state.email"`). Empty when no value binding is authored.
    pub bind_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSummary {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub line_height_percent: f32,
    pub letter_spacing: f32,
    pub align: TextAlignValue,
    pub vertical_align: TextVerticalAlignValue,
    pub growth: TextGrowthValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconSummary {
    pub family: String,
    pub name: String,
    pub icon_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutPaddingSummary {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl LayoutPaddingSummary {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub fn value_for(self, focus: op_editor_core::PropertyFocus) -> Option<f32> {
        use op_editor_core::PropertyFocus as F;
        match focus {
            F::PaddingTop => Some(self.top),
            F::PaddingRight => Some(self.right),
            F::PaddingBottom => Some(self.bottom),
            F::PaddingLeft => Some(self.left),
            _ => None,
        }
    }
}

/// One fill's head-row summary for the multi-fill Fill section.
/// The Fill section stacks one row per `PenFill`; each carries its
/// own type / colour / opacity so the row paints its head + body
/// independently. Built from `node_fills` in authored order.
#[derive(Debug, Clone)]
pub struct FillSummary {
    /// The fill kind — drives the per-row type dropdown + which body
    /// (solid hex / gradient stops / image) paints below the head row.
    pub fill_type: op_editor_core::FillType,
    /// Representative paint colour for the head-row swatch + (for a
    /// Solid fill) the hex input. Solid → its colour; gradient → the
    /// first stop's colour; image → white placeholder.
    pub color: Color,
    /// This fill's opacity in `[0.0, 1.0]` — the head row's `%` input
    /// paints `opacity * 100`.
    pub opacity: f32,
    /// Bound colour-variable name, when this fill's colour follows a
    /// `$ref`. `None` for a literal colour. Only meaningful for the
    /// primary (index 0) fill today (the variable subsystem keys off
    /// the primary fill); carried per-fill for forward compatibility.
    pub variable_ref: Option<String>,
}

/// One gradient stop summary for the Fill section.
#[derive(Debug, Clone)]
pub struct GradientStopSummary {
    /// Offset 0.0..=1.0 — the Fill panel paints `offset * 100` as
    /// the per-stop `%` input.
    pub offset: f32,
    /// Schema hex string (`#RRGGBB` or `#RRGGBBAA`). The panel
    /// paints this verbatim so a freshly-typed user value isn't
    /// silently re-cased by `format_color_hex` round-trips.
    pub hex: String,
    /// Parsed paint colour for the per-row swatch. Falls back to
    /// black when the hex fails to parse.
    pub color: Color,
}

/// Which visual-effect variant a row represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    Shadow,
    Blur,
    BackgroundBlur,
}

impl EffectKind {
    /// Human-readable row label.
    pub fn label(self) -> &'static str {
        match self {
            EffectKind::Shadow => "Drop Shadow",
            EffectKind::Blur => "Layer Blur",
            EffectKind::BackgroundBlur => "Background Blur",
        }
    }
}

/// One effect's editable scalar parameters, formatted for the
/// Effects section. Shadow uses all four; the blur kinds use `blur`
/// as the radius and leave offset / spread at 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectSummary {
    pub kind: EffectKind,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    /// Effect colour — Shadow carries an authored hex string; the
    /// blur kinds don't have a colour field, so paint reads
    /// `Color::TRANSPARENT` (and the colour row is hidden by the
    /// effects-section painter when alpha is zero).
    pub color: Color,
}

impl EffectSummary {
    /// Current value of one editable parameter — Blur / BackgroundBlur
    /// keep their radius in `blur`, so `Blur` and `Radius` both read
    /// that field.
    pub fn param_value(&self, field: op_editor_core::EffectField) -> f32 {
        use op_editor_core::EffectField as F;
        match field {
            F::OffsetX => self.offset_x,
            F::OffsetY => self.offset_y,
            F::Blur | F::Radius => self.blur,
            F::Spread => self.spread,
        }
    }

    /// Summarise a canonical `PenEffect` for the panel.
    fn from_pen_effect(e: &jian_ops_schema::style::PenEffect) -> Self {
        use jian_ops_schema::style::PenEffect;
        match e {
            PenEffect::Shadow(s) => EffectSummary {
                kind: EffectKind::Shadow,
                offset_x: s.offset_x,
                offset_y: s.offset_y,
                blur: s.blur,
                spread: s.spread,
                color: color_from_hex(&s.color).unwrap_or(Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.25,
                }),
            },
            PenEffect::Blur(b) => EffectSummary {
                kind: EffectKind::Blur,
                offset_x: 0.0,
                offset_y: 0.0,
                blur: b.radius,
                spread: 0.0,
                color: Color::TRANSPARENT,
            },
            PenEffect::BackgroundBlur(b) => EffectSummary {
                kind: EffectKind::BackgroundBlur,
                offset_x: 0.0,
                offset_y: 0.0,
                blur: b.radius,
                spread: 0.0,
                color: Color::TRANSPARENT,
            },
        }
    }
}

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
            rotation_deg: 0.0,
            corner_radius: 0.0,
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
            icon: None,
            text: None,
            widget: None,
            fill: None,
            fill_opacity: 1.0,
            fills: Vec::new(),
            stroke: None,
            gradient_angle: None,
            gradient_stops: Vec::new(),
            image_fill: None,
            effects: Vec::new(),
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
            rotation_deg: 0.0,
            corner_radius: 0.0,
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
            icon: None,
            text: None,
            widget: None,
            fill: None,
            fill_opacity: 1.0,
            // Multi-select hides the Fill section (see `for_multi`),
            // so it carries no per-fill rows.
            fills: Vec::new(),
            stroke: None,
            gradient_angle: None,
            gradient_stops: Vec::new(),
            image_fill: None,
            // Multi-select shows no per-effect rows — the Effects
            // section paints just its header + the add affordance.
            effects: Vec::new(),
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
    pub(crate) fn from_node(node: &PenNode) -> Self {
        let base = node.base();
        let kind = node_kind_of(node);
        let bounds = op_editor_core::aggregate_bounds(node);
        // Corner radius — only the container variants carry one;
        // a `PerCorner` radius reports its top-left corner.
        let corner_radius = container_corner_radius(node);
        let fill = op_editor_core::first_solid_fill_hex(node).and_then(color_from_hex);
        let stroke = op_editor_core::first_solid_stroke_hex(node)
            .and_then(color_from_hex)
            .map(|color| SceneStroke {
                color,
                width: op_editor_core::fills::node_stroke_width(node).unwrap_or(1.0) as f32,
                sides: stroke_sides_for_scene(node),
            });
        Self {
            kind: kind.label().to_string(),
            name: base.name.clone().unwrap_or_default(),
            x: bounds.x.round() as i32,
            y: bounds.y.round() as i32,
            width: bounds.w.round() as i32,
            height: bounds.h.round() as i32,
            // `base.rotation` is stored in degrees by the canonical
            // schema; the snapshot's `rotation_deg` wants degrees.
            rotation_deg: base.rotation.unwrap_or(0.0) as f32,
            corner_radius,
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
            icon: icon_summary_of(node),
            text: text_summary_of(node),
            widget: widget_summary_of(node),
            fill,
            fill_opacity: op_editor_core::first_solid_fill_opacity(node),
            fills: fills_of(node),
            stroke,
            gradient_angle: gradient_angle_of(node),
            gradient_stops: gradient_stops_of(node),
            image_fill: op_editor_core::first_image_fill_summary(node)
                .or_else(|| op_editor_core::image_node_summary(node)),
            effects: op_editor_core::node_effects(node)
                .iter()
                .map(EffectSummary::from_pen_effect)
                .collect(),
            kind_variant: kind,
            is_instance: false,
            is_reusable: matches!(node, PenNode::Frame(f) if f.reusable == Some(true)),
        }
    }
}

fn polygon_sides_of(node: &PenNode) -> Option<u32> {
    match node {
        PenNode::Polygon(n) => Some(n.polygon_count.clamp(3, 100)),
        _ => None,
    }
}

fn ellipse_arc_of(node: &PenNode) -> Option<EllipseArcSummary> {
    match node {
        PenNode::Ellipse(n) => Some(EllipseArcSummary {
            start_deg: n.start_angle.unwrap_or(0.0) as f32,
            sweep_deg: n.sweep_angle.unwrap_or(360.0) as f32,
            inner_percent: (n.inner_radius.unwrap_or(0.0).clamp(0.0, 0.99) * 100.0) as f32,
        }),
        _ => None,
    }
}

fn container_layout(node: &PenNode) -> Option<&LayoutMode> {
    match node {
        PenNode::Frame(n) => n.container.layout.as_ref(),
        PenNode::Group(n) => n.container.layout.as_ref(),
        PenNode::Rectangle(n) => n.container.layout.as_ref(),
        _ => None,
    }
}

fn flex_layout_of(node: &PenNode) -> op_editor_core::FlexLayout {
    match container_layout(node) {
        Some(LayoutMode::Vertical) => op_editor_core::FlexLayout::Vertical,
        Some(LayoutMode::Horizontal) => op_editor_core::FlexLayout::Horizontal,
        Some(LayoutMode::None) | None => op_editor_core::FlexLayout::Free,
    }
}

fn layout_justify_of(node: &PenNode) -> LayoutJustifyValue {
    let value = match node {
        PenNode::Frame(n) => n.container.justify_content.as_ref(),
        PenNode::Group(n) => n.container.justify_content.as_ref(),
        PenNode::Rectangle(n) => n.container.justify_content.as_ref(),
        _ => None,
    };
    match value.unwrap_or(&JustifyContent::Start) {
        JustifyContent::Start => LayoutJustifyValue::Start,
        JustifyContent::Center => LayoutJustifyValue::Center,
        JustifyContent::End => LayoutJustifyValue::End,
        JustifyContent::SpaceBetween => LayoutJustifyValue::SpaceBetween,
        JustifyContent::SpaceAround => LayoutJustifyValue::SpaceAround,
    }
}

fn layout_align_of(node: &PenNode) -> LayoutAlignValue {
    let value = match node {
        PenNode::Frame(n) => n.container.align_items.as_ref(),
        PenNode::Group(n) => n.container.align_items.as_ref(),
        PenNode::Rectangle(n) => n.container.align_items.as_ref(),
        _ => None,
    };
    match value.unwrap_or(&AlignItems::Start) {
        AlignItems::Start => LayoutAlignValue::Start,
        AlignItems::Center => LayoutAlignValue::Center,
        AlignItems::End => LayoutAlignValue::End,
        // `stretch` renders as start (see jian resolve_align), so the
        // panel surfaces Start — matching the rendered alignment.
        AlignItems::Stretch => LayoutAlignValue::Start,
    }
}

fn layout_gap_of(node: &PenNode) -> f32 {
    let gap = match node {
        PenNode::Frame(n) => n.container.gap.as_ref(),
        PenNode::Group(n) => n.container.gap.as_ref(),
        PenNode::Rectangle(n) => n.container.gap.as_ref(),
        _ => None,
    };
    match gap {
        Some(NumberOrExpression::Number(v)) => *v as f32,
        Some(NumberOrExpression::Expression(_)) | None => 0.0,
    }
}

fn layout_padding_of(node: &PenNode) -> LayoutPaddingSummary {
    let padding = match node {
        PenNode::Frame(n) => n.container.padding.as_ref(),
        PenNode::Group(n) => n.container.padding.as_ref(),
        PenNode::Rectangle(n) => n.container.padding.as_ref(),
        _ => None,
    };
    match padding {
        Some(Padding::Uniform(v)) => {
            let v = *v as f32;
            LayoutPaddingSummary {
                top: v,
                right: v,
                bottom: v,
                left: v,
            }
        }
        Some(Padding::XY(v)) => LayoutPaddingSummary {
            top: v[0] as f32,
            right: v[1] as f32,
            bottom: v[0] as f32,
            left: v[1] as f32,
        },
        Some(Padding::LtrB(v)) => LayoutPaddingSummary {
            top: v[0] as f32,
            right: v[1] as f32,
            bottom: v[2] as f32,
            left: v[3] as f32,
        },
        Some(Padding::Expression(_)) | None => LayoutPaddingSummary::ZERO,
    }
}

fn node_width_sizing(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(n) => n.container.width.as_ref(),
        PenNode::Group(n) => n.container.width.as_ref(),
        PenNode::Rectangle(n) => n.container.width.as_ref(),
        PenNode::Ellipse(n) => n.width.as_ref(),
        PenNode::Polygon(n) => n.width.as_ref(),
        PenNode::Path(n) => n.width.as_ref(),
        PenNode::Text(n) => n.width.as_ref(),
        PenNode::TextInput(n) => n.width.as_ref(),
        PenNode::TextArea(n) => n.width.as_ref(),
        PenNode::Select(n) => n.width.as_ref(),
        PenNode::Switch(n) => n.width.as_ref(),
        PenNode::Checkbox(n) => n.width.as_ref(),
        PenNode::Slider(n) => n.width.as_ref(),
        PenNode::RadioGroup(n) => n.width.as_ref(),
        PenNode::NumberInput(n) => n.width.as_ref(),
        PenNode::Progress(n) => n.width.as_ref(),
        PenNode::Tabs(n) => n.width.as_ref(),
        PenNode::Image(n) => n.width.as_ref(),
        PenNode::IconFont(n) => n.width.as_ref(),
        PenNode::Line(_) | PenNode::Ref(_) => None,
    }
}

fn node_height_sizing(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(n) => n.container.height.as_ref(),
        PenNode::Group(n) => n.container.height.as_ref(),
        PenNode::Rectangle(n) => n.container.height.as_ref(),
        PenNode::Ellipse(n) => n.height.as_ref(),
        PenNode::Polygon(n) => n.height.as_ref(),
        PenNode::Path(n) => n.height.as_ref(),
        PenNode::Text(n) => n.height.as_ref(),
        PenNode::TextInput(n) => n.height.as_ref(),
        PenNode::TextArea(n) => n.height.as_ref(),
        PenNode::Select(n) => n.height.as_ref(),
        PenNode::Switch(n) => n.height.as_ref(),
        PenNode::Checkbox(n) => n.height.as_ref(),
        PenNode::Slider(n) => n.height.as_ref(),
        PenNode::RadioGroup(n) => n.height.as_ref(),
        PenNode::NumberInput(n) => n.height.as_ref(),
        PenNode::Progress(n) => n.height.as_ref(),
        PenNode::Tabs(n) => n.height.as_ref(),
        PenNode::Image(n) => n.height.as_ref(),
        PenNode::IconFont(n) => n.height.as_ref(),
        PenNode::Line(_) | PenNode::Ref(_) => None,
    }
}

fn sizing_is(sizing: Option<&SizingBehavior>, keyword: SizingKeyword) -> bool {
    matches!(sizing, Some(SizingBehavior::Keyword(k)) if *k == keyword)
}

fn clip_content_of(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => n.container.clip_content.unwrap_or(false),
        PenNode::Group(n) => n.container.clip_content.unwrap_or(false),
        PenNode::Rectangle(n) => n.container.clip_content.unwrap_or(false),
        _ => false,
    }
}

fn can_clip_content(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_)
    )
}

fn icon_summary_of(node: &PenNode) -> Option<IconSummary> {
    match node {
        PenNode::IconFont(n) => {
            let family = n
                .icon_font_family
                .clone()
                .unwrap_or_else(|| "lucide".to_string());
            Some(IconSummary {
                icon_id: format!("{}:{}", family, n.icon_font_name),
                family,
                name: n.icon_font_name.clone(),
            })
        }
        PenNode::Path(n) => {
            let icon_id = n.icon_id.as_ref()?;
            let (family, name) = icon_id
                .split_once(':')
                .map(|(family, name)| (family.to_string(), name.to_string()))
                .unwrap_or_else(|| ("lucide".to_string(), icon_id.clone()));
            Some(IconSummary {
                family,
                name,
                icon_id: icon_id.clone(),
            })
        }
        _ => None,
    }
}

fn has_corner_radius(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_)
            | PenNode::Rectangle(_)
            | PenNode::Ellipse(_)
            | PenNode::Polygon(_)
            | PenNode::Image(_)
    )
}

fn can_create_component(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_) | PenNode::Ref(_)
    )
}

/// Format an optional `f64` widget prop for an input box, dropping a
/// trailing `.0` so `5.0` paints as `5`. Empty when unset.
fn fmt_widget_num(v: Option<f64>) -> String {
    match v {
        Some(n) if n.fract() == 0.0 => format!("{}", n as i64),
        Some(n) => format!("{n}"),
        None => String::new(),
    }
}

/// Read a `NumberOrExpression` value as a display string — a literal
/// number drops trailing `.0`; an expression paints verbatim (the
/// panel can't edit a binding numerically, but should still show it).
fn fmt_widget_value(v: Option<&NumberOrExpression>) -> String {
    match v {
        Some(NumberOrExpression::Number(n)) if n.fract() == 0.0 => format!("{}", *n as i64),
        Some(NumberOrExpression::Number(n)) => format!("{n}"),
        Some(NumberOrExpression::Expression(s)) => s.clone(),
        None => String::new(),
    }
}

fn checked_literal(v: Option<&BoolOrExpression>) -> bool {
    matches!(v, Some(BoolOrExpression::Bool(true)))
}

/// Display key for the `bind:value` two-way binding — the raw
/// expression with the `$state.` prefix stripped so the panel input
/// shows `email` for `$state.email`. Empty when no value binding is
/// authored. A non-`$state` expression is shown verbatim.
fn bind_value_key(bindings: Option<&jian_ops_schema::events::Bindings>) -> String {
    bindings
        .and_then(|b| b.get("bind:value"))
        .map(|e| e.0.trim().trim_start_matches("$state.").to_string())
        .unwrap_or_default()
}

/// Build the Widget-section summary for a form-widget `PenNode`.
/// `None` for every non-widget kind so the section stays hidden.
fn widget_summary_of(node: &PenNode) -> Option<WidgetSummary> {
    let base = |kind: WidgetKind| WidgetSummary {
        kind,
        placeholder: String::new(),
        value: String::new(),
        label: String::new(),
        checked: false,
        min: String::new(),
        max: String::new(),
        step: String::new(),
        option_count: 0,
        tab_count: 0,
        leading_icon: String::new(),
        trailing_icon: String::new(),
        bind_key: String::new(),
    };
    match node {
        PenNode::TextInput(n) => Some(WidgetSummary {
            placeholder: n.placeholder.clone().unwrap_or_default(),
            value: n.value.clone().unwrap_or_default(),
            leading_icon: n.leading_icon.clone().unwrap_or_default(),
            trailing_icon: n.trailing_icon.clone().unwrap_or_default(),
            bind_key: bind_value_key(n.bindings.as_ref()),
            ..base(WidgetKind::TextInput)
        }),
        PenNode::TextArea(n) => Some(WidgetSummary {
            placeholder: n.placeholder.clone().unwrap_or_default(),
            value: n.value.clone().unwrap_or_default(),
            leading_icon: n.leading_icon.clone().unwrap_or_default(),
            trailing_icon: n.trailing_icon.clone().unwrap_or_default(),
            bind_key: bind_value_key(n.bindings.as_ref()),
            ..base(WidgetKind::TextArea)
        }),
        PenNode::NumberInput(n) => Some(WidgetSummary {
            placeholder: n.placeholder.clone().unwrap_or_default(),
            value: fmt_widget_value(n.value.as_ref()),
            min: fmt_widget_num(n.min),
            max: fmt_widget_num(n.max),
            step: fmt_widget_num(n.step),
            leading_icon: n.leading_icon.clone().unwrap_or_default(),
            trailing_icon: n.trailing_icon.clone().unwrap_or_default(),
            bind_key: bind_value_key(n.bindings.as_ref()),
            ..base(WidgetKind::NumberInput)
        }),
        PenNode::Select(n) => Some(WidgetSummary {
            placeholder: n.placeholder.clone().unwrap_or_default(),
            value: n.value.clone().unwrap_or_default(),
            option_count: n.options.as_ref().map_or(0, |o| o.len()),
            ..base(WidgetKind::Select)
        }),
        PenNode::RadioGroup(n) => Some(WidgetSummary {
            value: n.value.clone().unwrap_or_default(),
            option_count: n.options.as_ref().map_or(0, |o| o.len()),
            ..base(WidgetKind::RadioGroup)
        }),
        PenNode::Switch(n) => Some(WidgetSummary {
            checked: checked_literal(n.checked.as_ref()),
            ..base(WidgetKind::Switch)
        }),
        PenNode::Checkbox(n) => Some(WidgetSummary {
            label: n.label.clone().unwrap_or_default(),
            checked: checked_literal(n.checked.as_ref()),
            ..base(WidgetKind::Checkbox)
        }),
        PenNode::Slider(n) => Some(WidgetSummary {
            value: fmt_widget_value(n.value.as_ref()),
            min: fmt_widget_num(n.min),
            max: fmt_widget_num(n.max),
            step: fmt_widget_num(n.step),
            ..base(WidgetKind::Slider)
        }),
        PenNode::Progress(n) => Some(WidgetSummary {
            value: fmt_widget_value(n.value.as_ref()),
            max: fmt_widget_num(n.max),
            ..base(WidgetKind::Progress)
        }),
        PenNode::Tabs(n) => Some(WidgetSummary {
            value: n.value.clone().unwrap_or_default(),
            tab_count: n.tabs.as_ref().map_or(0, |t| t.len()),
            ..base(WidgetKind::Tabs)
        }),
        _ => None,
    }
}

fn text_summary_of(node: &PenNode) -> Option<TextSummary> {
    let PenNode::Text(t) = node else {
        return None;
    };
    Some(TextSummary {
        font_family: t
            .font_family
            .clone()
            .unwrap_or_else(|| "Inter, sans-serif".to_string()),
        font_size: t.font_size.unwrap_or(16.0) as f32,
        font_weight: font_weight_value(t.font_weight.as_ref()),
        line_height_percent: (t.line_height.unwrap_or(1.2) * 100.0) as f32,
        letter_spacing: t.letter_spacing.unwrap_or(0.0) as f32,
        align: match t.text_align.as_ref().unwrap_or(&TextAlign::Left) {
            TextAlign::Left => TextAlignValue::Left,
            TextAlign::Center => TextAlignValue::Center,
            TextAlign::Right => TextAlignValue::Right,
            TextAlign::Justify => TextAlignValue::Justify,
        },
        vertical_align: match t
            .text_align_vertical
            .as_ref()
            .unwrap_or(&TextAlignVertical::Top)
        {
            TextAlignVertical::Top => TextVerticalAlignValue::Top,
            TextAlignVertical::Middle => TextVerticalAlignValue::Middle,
            TextAlignVertical::Bottom => TextVerticalAlignValue::Bottom,
        },
        growth: match t.text_growth.as_ref().unwrap_or(&TextGrowth::FixedWidth) {
            TextGrowth::Auto => TextGrowthValue::Auto,
            TextGrowth::FixedWidth => TextGrowthValue::FixedWidth,
            TextGrowth::FixedWidthHeight => TextGrowthValue::FixedWidthHeight,
        },
    })
}

fn font_weight_value(weight: Option<&FontWeight>) -> u16 {
    match weight {
        Some(FontWeight::Number(n)) => (*n).clamp(1, 1000) as u16,
        Some(FontWeight::Keyword(s)) => match s.as_str() {
            "thin" => 100,
            "light" => 300,
            "medium" => 500,
            "semibold" => 600,
            "bold" => 700,
            "black" => 900,
            _ => 400,
        },
        None => 400,
    }
}

/// LinearGradient `angle` for the node's first fill, when it has
/// one. Falls back to `0.0` (canonical default, bottom→top) when
/// the body omits an explicit angle. `None` for non-linear primary
/// fills — the Fill section uses that to hide the angle row.
fn gradient_angle_of(node: &PenNode) -> Option<f32> {
    use jian_ops_schema::style::PenFill;
    match op_editor_core::fills::node_fills(node).and_then(|f| f.first())? {
        PenFill::LinearGradient(body) => Some(body.angle.unwrap_or(0.0)),
        _ => None,
    }
}

/// Resolved stops for the primary Linear / Radial gradient — empty
/// list for Solid / Image / no-fill nodes.
fn gradient_stops_of(node: &PenNode) -> Vec<GradientStopSummary> {
    use jian_ops_schema::style::PenFill;
    let Some(first) = op_editor_core::fills::node_fills(node).and_then(|f| f.first()) else {
        return Vec::new();
    };
    let raw = match first {
        PenFill::LinearGradient(b) => &b.stops,
        PenFill::RadialGradient(b) => &b.stops,
        _ => return Vec::new(),
    };
    raw.iter()
        .map(|s| GradientStopSummary {
            offset: s.offset.clamp(0.0, 1.0),
            hex: s.color.clone(),
            color: color_from_hex(&s.color).unwrap_or(Color::BLACK),
        })
        .collect()
}

/// Build one [`FillSummary`] per `PenFill` on the node, in authored
/// order. Each entry's representative colour mirrors `fills::fill_hex`
/// (Solid → its colour; gradient → first stop; image → white). An old
/// single-fill node yields exactly one entry; a node with no `fill`
/// field / no fills yields an empty list so the Fill section paints
/// just its header + "+".
fn fills_of(node: &PenNode) -> Vec<FillSummary> {
    use jian_ops_schema::style::PenFill;
    let Some(fills) = op_editor_core::fills::node_fills(node) else {
        return Vec::new();
    };
    fills
        .iter()
        .map(|fill| {
            let fill_type = op_editor_core::fills::fill_type_of(fill);
            let (hex, opacity) = match fill {
                PenFill::Solid(b) => (Some(b.color.as_str()), b.opacity.unwrap_or(1.0)),
                PenFill::LinearGradient(b) => (
                    b.stops.first().map(|s| s.color.as_str()),
                    b.opacity.unwrap_or(1.0),
                ),
                PenFill::RadialGradient(b) => (
                    b.stops.first().map(|s| s.color.as_str()),
                    b.opacity.unwrap_or(1.0),
                ),
                PenFill::Image(b) => (None, b.opacity.unwrap_or(1.0)),
            };
            FillSummary {
                fill_type,
                color: hex.and_then(color_from_hex).unwrap_or(Color::WHITE),
                opacity,
                variable_ref: None,
            }
        })
        .collect()
}

/// Uniform corner radius (doc-px) for a container variant — Frame /
/// Group / Rectangle carry a `CornerRadius`. A `PerCorner` radius
/// reports its top-left value. Non-container variants read 0.
fn container_corner_radius(node: &PenNode) -> f32 {
    use jian_ops_schema::node::container::CornerRadius;
    let cr = match node {
        PenNode::Frame(n) => n.container.corner_radius.as_ref(),
        PenNode::Group(n) => n.container.corner_radius.as_ref(),
        PenNode::Rectangle(n) => n.container.corner_radius.as_ref(),
        PenNode::Image(n) => n.corner_radius.as_ref(),
        _ => None,
    };
    match cr {
        Some(CornerRadius::Uniform(r)) => *r as f32,
        Some(CornerRadius::PerCorner(c)) => c[0] as f32,
        None => match node {
            PenNode::Ellipse(n) => n.corner_radius.unwrap_or(0.0) as f32,
            PenNode::Polygon(n) => n.corner_radius.unwrap_or(0.0) as f32,
            _ => 0.0,
        },
    }
}
