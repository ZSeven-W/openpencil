//! Snapshot extraction for the right-rail `PropertyPanel`.

use crate::layout_scene::{NodeKind, SceneStroke};
use crate::Color;
use jian_ops_schema::node::PenNode;
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
        PenNode::Text(_) | PenNode::TextInput(_) => NodeKind::Text,
        PenNode::Image(_) => NodeKind::Other("image".to_string()),
        PenNode::IconFont(_) => NodeKind::Other("icon_font".to_string()),
        PenNode::Ref(_) => NodeKind::Other("ref".to_string()),
    }
}

/// Parse a `#RRGGBB` / `#RGB` hex string into a `Color`. Reuses the
/// editor-state colour parser; 8-char `#RRGGBBAA` is honoured so
/// gradient stop swatches (and any other authored alpha) round-trip
/// transparency into paint instead of always reading as opaque.
fn color_from_hex(hex: &str) -> Option<Color> {
    let (r, g, b) = op_editor_core::parse_hex_rgb(hex)?;
    let a = op_editor_core::parse_hex_alpha(hex);
    Some(Color { r, g, b, a })
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
    pub fill: Option<Color>,
    /// Primary solid-fill opacity in `[0.0, 1.0]` — the Fill
    /// section's `100 %` paints `fill_opacity * 100`.
    pub fill_opacity: f32,
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseArcSummary {
    pub start_deg: f32,
    pub sweep_deg: f32,
    pub inner_percent: f32,
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
    /// Build an aggregate snapshot for a multi-node selection.
    /// Returns None when nothing on the active page resolves from
    /// `selected_set`. Uses `Document::selection_bounds` (the union
    /// of every selected node's `aggregate_bounds`) for x/y/w/h.
    /// Rotation / fill / stroke are zeroed in v1 — broadcasting
    /// "Mixed" or per-axis aggregation is a follow-up; the panel
    /// hides those inputs anyway since `is_multi` flips them
    /// inert.
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
            fill: None,
            fill_opacity: 1.0,
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
            fill,
            fill_opacity: op_editor_core::first_solid_fill_opacity(node),
            stroke,
            gradient_angle: gradient_angle_of(node),
            gradient_stops: gradient_stops_of(node),
            image_fill: op_editor_core::first_image_fill_summary(node),
            effects: op_editor_core::node_effects(node)
                .iter()
                .map(EffectSummary::from_pen_effect)
                .collect(),
            kind_variant: kind,
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

/// Uniform corner radius (doc-px) for a container variant — Frame /
/// Group / Rectangle carry a `CornerRadius`. A `PerCorner` radius
/// reports its top-left value. Non-container variants read 0.
fn container_corner_radius(node: &PenNode) -> f32 {
    use jian_ops_schema::node::container::CornerRadius;
    let cr = match node {
        PenNode::Frame(n) => n.container.corner_radius.as_ref(),
        PenNode::Group(n) => n.container.corner_radius.as_ref(),
        PenNode::Rectangle(n) => n.container.corner_radius.as_ref(),
        _ => None,
    };
    match cr {
        Some(CornerRadius::Uniform(r)) => *r as f32,
        Some(CornerRadius::PerCorner(c)) => c[0] as f32,
        None => 0.0,
    }
}
