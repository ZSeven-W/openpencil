//! `PropertyPanel` — right-rail node inspector (Step 6).
//!
//! Mirrors `apps/web/src/components/panels/right-panel.tsx` and the
//! per-section TS files (`*-section.tsx`). The bulk of the paint
//! logic lives in [`super::property_panel_sections`] — this file
//! holds the `PropertyPanel` struct, the `Widget` impl, and the
//! per-frame snapshot extraction. Splitting the file keeps both
//! pieces under the openpencil 800-line ceiling.
//!
//! Sections (top → bottom, mirroring TS order):
//!   1. Tab strip (Design / Code)
//!   2. Header (kind label) + Create component button
//!   3. Position — X / Y / rotation / R
//!   4. Flex layout — 3 layout-mode buttons
//!   5. Size — W / H + 5 sizing checkboxes
//!   6. Layer — opacity row
//!   7. Fill — solid color rows + add affordance
//!   8. Stroke — color + width row
//!   9. Effects — empty list + add affordance
//!  10. Export — scale + format dropdowns
//!
//! Conditional rendering: TS app does `{hasSelection && <RightPanel/>}`.
//! Host calls [`PropertyPanel::for_selection`] which returns
//! `Option<Self>`; `None` = panel hidden entirely.

use crate::layout_scene::{NodeKind, SceneStroke};
use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::property_panel_sections as sections;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect};
use op_editor_core::PropertyFocus;

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

pub const PROPERTY_PANEL_WIDTH: f32 = 280.0;

// `PropertyPanelAction` lives in `property_panel_action.rs` (split
// out for the 800-line ceiling); re-exported so every existing
// `widgets::PropertyPanelAction` / `property_panel::PropertyPanelAction`
// path is unchanged.
pub use crate::widgets::property_panel_action::PropertyPanelAction;

// `SectionCapabilities` lives in `property_panel_layout.rs`
// alongside `VisibleSections` (the section-visibility mask it
// feeds); re-exported so `property_panel::SectionCapabilities`
// resolves unchanged.
pub(crate) use crate::widgets::property_panel_layout::SectionCapabilities;

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
    /// The node's visual effects, in paint order — drives the
    /// Effects section's rows + param inputs.
    pub effects: Vec<EffectSummary>,
    /// Drives per-kind section filtering (Line hides fill, etc.).
    pub kind_variant: crate::layout_scene::NodeKind,
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
    fn from_multi_selection(state: &EditorState) -> Option<Self> {
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
            fill: None,
            fill_opacity: 1.0,
            stroke: None,
            gradient_angle: None,
            gradient_stops: Vec::new(),
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
    fn from_node(node: &PenNode) -> Self {
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
            fill,
            fill_opacity: op_editor_core::first_solid_fill_opacity(node),
            stroke,
            gradient_angle: gradient_angle_of(node),
            gradient_stops: gradient_stops_of(node),
            effects: op_editor_core::node_effects(node)
                .iter()
                .map(EffectSummary::from_pen_effect)
                .collect(),
            kind_variant: kind,
        }
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

pub struct PropertyPanel {
    pub id: WidgetId,
    pub snapshot: NodeSnapshot,
    pub theme: Theme,
    /// Localised chrome strings — `Document::t` lookups resolved
    /// once at construction time so every section paint hands
    /// straight to the renderer without re-walking the i18n table.
    pub labels: sections::PropertyLabels,
    /// Which input row the user is editing. `None` when no input
    /// is focused (panel paints all values from the snapshot).
    pub focus: Option<PropertyFocus>,
    /// Live edit-buffer for the focused input. Empty when nothing
    /// is focused. The host fills this on click + mutates on
    /// keystroke; the panel paints it as the field's value.
    pub draft: String,
    /// Caret byte-offset into `draft` (ASCII drafts → char index).
    pub caret_pos: usize,
    /// Caret-blink anchor (ms since host start) for the focused
    /// input. Drives the same `jian_core::anim::blink_visible`
    /// helper the chat caret uses.
    pub caret_anchor_ms: u64,
    /// Host clock ms; paired with `caret_anchor_ms` for caret blink.
    pub now_ms: u64,
    /// Active flex-layout button.
    pub flex_layout: op_editor_core::FlexLayout,
    /// 5 size checkboxes — fill / hug / clip.
    pub size_flags: sections::SizeFlags,
    /// Active fill type — drives the dropdown label + picker.
    pub fill_type: op_editor_core::FillType,
    pub fill_type_picker_open: bool,
    /// True for multi-select aggregate (inputs inert, "N items").
    pub is_multi: bool,
    /// Active header tab — toggled by Cmd+Shift+C.
    pub tab: op_editor_core::PropertyTab,
    /// Current export format + scale, shown on the Export section's
    /// two dropdowns. Clicking a dropdown opens its inline select
    /// popup (NOT the Export modal).
    pub export_format: op_editor_core::ExportFormat,
    pub export_scale: f32,
    /// Whether the Export section's scale / format inline select
    /// popups are open.
    pub export_scale_picker_open: bool,
    pub export_format_picker_open: bool,
    /// Row index the cursor is over in the open Export select
    /// popup — `None` when no popup is open or no row is hovered.
    pub export_picker_hover: Option<usize>,
    /// Vertical scroll offset (px, ≥ 0) — paint + hit-test shift the
    /// section content up by this so a tall inspector stays usable.
    pub scroll: f32,
    /// Active UI locale — threaded into the Fill section so its
    /// type label / picker / body sub-labels translate.
    pub locale: op_editor_core::Locale,
    /// Focused effect-parameter value, if any — drives the Effects
    /// section's editable value boxes.
    pub effect_param_focus: Option<op_editor_core::editor_ui_state::EffectParamFocus>,
}

impl PropertyPanel {
    /// Capability mask that drives which sections paint for this
    /// panel state. Multi-select uses a dedicated mask (`for_multi`)
    /// that keeps Size + Position + Layer + Effects + Export and
    /// hides Flex + Fill + Stroke; single-select falls back to the
    /// snapshot's `kind_variant` (`for_kind`). Paint + hit-test
    /// must call this rather than `SectionCapabilities::for_kind`
    /// directly so the multi-select carve-out can't regress
    /// silently.
    pub(crate) fn capabilities(&self) -> SectionCapabilities {
        if self.is_multi {
            SectionCapabilities::for_multi()
        } else {
            SectionCapabilities::for_kind(&self.snapshot.kind_variant)
        }
    }
}

impl PropertyPanel {
    /// Conditional builder — returns `Some` only when the editor
    /// has an active selection. Mirrors TS `{hasSelection && ...}`.
    pub fn for_selection(state: &EditorState) -> Option<Self> {
        Self::for_selection_at(state, 0)
    }

    /// Same as [`for_selection`] but threads the host's monotonic
    /// millisecond clock through so the focused-input caret can
    /// blink off the same animation timer as the chat input.
    pub fn for_selection_at(state: &EditorState, now_ms: u64) -> Option<Self> {
        if state.selection_count() == 1 {
            let node = state.selected_node()?;
            let fill_type = op_editor_core::first_fill_type(node);
            return Some(Self::build_from_snapshot(
                state,
                NodeSnapshot::from_node(node),
                fill_type,
                now_ms,
                false,
            ));
        }
        if state.selection_count() >= 2 {
            let snapshot = NodeSnapshot::from_multi_selection(state)?;
            return Some(Self::build_from_snapshot(
                state,
                snapshot,
                op_editor_core::FillType::Solid,
                now_ms,
                true,
            ));
        }
        None
    }

    fn build_from_snapshot(
        state: &EditorState,
        snapshot: NodeSnapshot,
        fill_type: op_editor_core::FillType,
        now_ms: u64,
        is_multi: bool,
    ) -> Self {
        let ui = &state.editor_ui;
        Self {
            id: WidgetId::new(2000),
            snapshot,
            theme: theme_for(ui),
            labels: sections::PropertyLabels::for_editor_ui(ui),
            // Multi-select inputs are inert in v1 — broadcast edits
            // to all selected nodes lands later. Force focus to None
            // so the panel paints all values muted and hit_test
            // returns None (see `hit_test` is_multi short-circuit).
            focus: if is_multi {
                None
            } else {
                state.ui.property_focus
            },
            draft: if is_multi {
                String::new()
            } else {
                state.ui.property_input_draft.clone()
            },
            caret_pos: if is_multi {
                0
            } else {
                state.ui.property_caret_pos
            },
            caret_anchor_ms: state.ui.property_caret_anchor_ms,
            now_ms,
            flex_layout: ui.flex_layout,
            size_flags: sections::SizeFlags {
                fill_width: ui.size_fill_width,
                fill_height: ui.size_fill_height,
                hug_width: ui.size_hug_width,
                hug_height: ui.size_hug_height,
                clip_content: ui.size_clip_content,
            },
            fill_type,
            fill_type_picker_open: ui.fill_type_picker_open,
            is_multi,
            tab: ui.property_tab,
            export_format: ui.export_format,
            export_scale: ui.export_scale,
            export_scale_picker_open: ui.export_scale_picker_open,
            export_format_picker_open: ui.export_format_picker_open,
            export_picker_hover: ui.export_picker_hover,
            scroll: ui.property_panel_scroll.max(0.0),
            locale: ui.locale,
            // Inert in the multi-select aggregate view.
            effect_param_focus: if is_multi {
                None
            } else {
                ui.effect_param_focus
            },
        }
    }

    /// `self.scroll` clamped to the current content's scrollable
    /// range. The host only re-clamps the stored offset on a wheel
    /// event, so selecting a shorter node (fewer sections / effects)
    /// could otherwise leave the panel scrolled past its end —
    /// every paint / hit-test reads through this so the view
    /// self-corrects on the very next frame.
    fn effective_scroll(&self, panel_rect: Rect) -> f32 {
        let max = (self.content_height(panel_rect) - panel_rect.size.y).max(0.0);
        self.scroll.clamp(0.0, max)
    }

    /// `panel_rect` shifted up by the (clamped) scroll offset. Both
    /// paint and every hit-test walker start their y-walk from this
    /// rect, so the panel scrolls as one piece and clicks stay
    /// aligned with what is drawn.
    fn scrolled_rect(&self, panel_rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                panel_rect.origin.x,
                panel_rect.origin.y - self.effective_scroll(panel_rect),
            ),
            size: panel_rect.size,
        }
    }

    /// Whether `point` is inside the scrolling section viewport —
    /// the panel below the pinned tab strip. A click in the tab-strip
    /// band must not fall through to a section row scrolled up
    /// under it (paint clips there; hit-test must agree).
    fn point_in_section_viewport(&self, panel_rect: Rect, point: Point2D) -> bool {
        point.y >= panel_rect.origin.y + crate::widgets::property_panel_inputs::TAB_HEIGHT
    }

    /// Total height (px) of the panel's section content — drives the
    /// scroll clamp so the inspector can't scroll past its end.
    pub fn content_height(&self, panel_rect: Rect) -> f32 {
        sections::property_panel_content_height(
            panel_rect,
            self.visible_sections(),
            &self.snapshot.effects,
        )
    }

    /// Section-visibility mask for the current selection, threaded
    /// into every layout walker so paint + hit-test stay aligned.
    fn visible_sections(&self) -> sections::VisibleSections {
        let caps = self.capabilities();
        sections::VisibleSections {
            flex_layout: caps.flex_layout,
            size_options: caps.size_options,
            opacity: caps.opacity,
            fill: caps.fill,
            stroke: caps.stroke,
            effects: caps.effects,
            export: caps.export,
            fill_type: self.fill_type,
            gradient_stop_count: self.snapshot.gradient_stops.len(),
        }
    }

    /// Hit-test the flex / size buttons + checkboxes. Returns the
    /// action the host should dispatch, or `None` if the cursor
    /// missed every clickable shape. Called AFTER `hit_test` so
    /// text inputs win over the action rects they overlap with.
    pub fn hit_test_action(&self, panel_rect: Rect, point: Point2D) -> Option<PropertyPanelAction> {
        if self.is_multi {
            // Multi-select inputs / toggles are inert in v1.
            return None;
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        let rects = sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            self.fill_type_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
        );
        // Picker rows live in `rects` AFTER the dropdown rect, so
        // a row hit takes priority — `rev()` makes the picker rows
        // tested first and short-circuits before the dropdown
        // toggle, otherwise clicking a row would just re-toggle.
        for (action, rect) in rects.into_iter().rev() {
            if rect_contains(rect, point) {
                return Some(action);
            }
        }
        None
    }

    /// Row index of the open Export select popup under `point`, or
    /// `None` when no popup is open / the cursor is off every row.
    /// The index counts only the option rows (`SetExportScale` /
    /// `SetExportFormat`), matching `paint_select_popup`'s row walk,
    /// so it can drive the popup's hover highlight.
    pub fn export_picker_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if !self.export_scale_picker_open && !self.export_format_picker_open {
            return None;
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            self.fill_type_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
        )
        .into_iter()
        .filter(|(a, _)| {
            matches!(
                a,
                PropertyPanelAction::SetExportScale(_) | PropertyPanelAction::SetExportFormat(_)
            )
        })
        .position(|(_, rect)| rect_contains(rect, point))
    }

    /// Hit-test the panel at `point` and return which input row
    /// (if any) contains the click. The layout walk mirrors the
    /// per-kind section filtering applied in `paint`, so rects
    /// after a skipped section don't drift out of alignment.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<PropertyFocus> {
        if self.is_multi {
            // Inputs inert in v1 multi-select aggregate view.
            return None;
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        for (focus, rect) in
            sections::editable_input_rects(self.scrolled_rect(panel_rect), self.visible_sections())
        {
            if rect_contains(rect, point) {
                return Some(focus);
            }
        }
        None
    }
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

use crate::widgets::property_panel_code::paint_code_placeholder;

impl Widget for PropertyPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Vertical extent is "as much as you give me" — the host
        // clips at the rail rect. Reporting 800 here is just a
        // placeholder for the abstract widget tree.
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, 800.0),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, self.theme.card);
        cx.backend.fill_rect(
            Rect {
                origin: rect.origin,
                size: Point2D::new(1.0, rect.size.y),
            },
            self.theme.border,
        );

        let x = rect.origin.x;
        let w = rect.size.x;
        // The Design / Code tab strip is pinned to the panel top —
        // painted fixed, above (and never scrolled with) the section
        // content.
        let tab_bottom =
            sections::paint_tab_strip(cx, &self.theme, &self.labels, self.tab, x, rect.origin.y, w);
        let edit_ctx = sections::EditContext {
            focus: self.focus,
            draft: self.draft.as_str(),
            caret: self.caret_pos,
            caret_anchor_ms: self.caret_anchor_ms,
            now_ms: self.now_ms,
        };
        use crate::layout_scene::NodeKind;
        let caps = self.capabilities();
        if matches!(self.tab, op_editor_core::PropertyTab::Code) {
            paint_code_placeholder(cx, &self.theme, &self.snapshot, x, tab_bottom, w);
            return;
        }
        // Section content scrolls below the pinned tab strip; clip it
        // so a scrolled-up section can't paint over the tabs or bleed
        // onto the neighbouring rail. Overlays (fill / export pickers)
        // anchor to `scrolled` — the same shifted rect the layout
        // walker uses (it adds `TAB_HEIGHT`), so paint + hit-test of
        // the sections agree.
        cx.backend.save();
        cx.backend.clip_rect(Rect {
            origin: Point2D::new(x, tab_bottom),
            size: Point2D::new(w, (rect.origin.y + rect.size.y - tab_bottom).max(0.0)),
        });
        let scroll = self.effective_scroll(rect);
        let scrolled = Rect {
            origin: Point2D::new(rect.origin.x, rect.origin.y - scroll),
            size: rect.size,
        };
        // First section sits just below the pinned tab strip:
        // `tab_bottom - scroll` == `scrolled.origin.y + TAB_HEIGHT`,
        // matching the layout walker's `+= TAB_HEIGHT` step.
        let mut y = tab_bottom - scroll;
        y = sections::paint_node_header(cx, &self.theme, &self.snapshot, x, y, w);
        y = sections::paint_create_component(cx, &self.theme, &self.labels, x, y, w);
        y = sections::paint_position_section(
            cx,
            &self.theme,
            &self.snapshot,
            &edit_ctx,
            &self.labels,
            x,
            y,
            w,
        );
        if caps.flex_layout {
            y = sections::paint_flex_section(
                cx,
                &self.theme,
                &self.labels,
                self.flex_layout,
                x,
                y,
                w,
            );
        }
        if caps.size_options {
            y = sections::paint_size_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.size_flags,
                x,
                y,
                w,
            );
        }
        if caps.opacity {
            y = sections::paint_layer_section(cx, &self.theme, &self.labels, &edit_ctx, x, y, w);
        }
        if caps.fill {
            y = sections::paint_fill_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.fill_type,
                self.fill_type_picker_open,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.stroke {
            y = sections::paint_stroke_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                x,
                y,
                w,
            );
        }
        if caps.effects {
            y = sections::paint_effects_section(
                cx,
                &self.theme,
                &self.labels,
                &self.snapshot.effects,
                &edit_ctx,
                self.effect_param_focus,
                x,
                y,
                w,
            );
        }
        if caps.export {
            let _ = sections::paint_export_section(
                cx,
                &self.theme,
                &self.labels,
                self.export_format,
                self.export_scale,
                x,
                y,
                w,
            );
        }
        // Fill-type picker overlay sits on top of everything below
        // the Fill section so it can extend past the section divider.
        if caps.fill && self.fill_type_picker_open {
            sections::paint_fill_type_picker(
                cx,
                &self.theme,
                scrolled,
                self.visible_sections(),
                self.fill_type,
                self.locale,
            );
        }
        // Export-section inline select popups — painted last so the
        // scale / format dropdown overlays sit above every section.
        if caps.export && (self.export_scale_picker_open || self.export_format_picker_open) {
            sections::paint_export_picker(
                cx,
                &self.theme,
                scrolled,
                self.visible_sections(),
                &self.snapshot.effects,
                self.export_scale_picker_open,
                self.export_format_picker_open,
                self.export_scale,
                self.export_format,
                self.export_picker_hover,
            );
        }
        cx.backend.restore();
        let _ = NodeKind::Frame; // ensure NodeKind is in scope above for tests
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(self.snapshot.kind.clone());
        node
    }
}
