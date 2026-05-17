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
use crate::widgets::editor_state_ext::{doc_export_format, theme_for};
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
/// editor-state colour parser (RGB only — opacity stays 1.0).
fn color_from_hex(hex: &str) -> Option<Color> {
    let (r, g, b) = op_editor_core::parse_hex_rgb(hex)?;
    Some(Color { r, g, b, a: 1.0 })
}

pub const PROPERTY_PANEL_WIDTH: f32 = 280.0;

/// Button / checkbox actions in the property panel that don't
/// map to a text input. The host dispatches these in `apply_press`
/// after the text-input hit-test misses.
///
/// `PartialEq` only (not `Eq`) — `AdjustEffectParam` carries an `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyPanelAction {
    SetFlexLayout(op_editor_core::FlexLayout),
    ToggleSizeFillWidth,
    ToggleSizeFillHeight,
    ToggleSizeHugWidth,
    ToggleSizeHugHeight,
    ToggleSizeClipContent,
    /// User clicked the Fill section's fill-type dropdown — host
    /// toggles `Document.ui.fill_type_picker_open`.
    ToggleFillTypePicker,
    /// User picked a fill type from the dropdown.
    SetFillType(op_editor_core::FillType),
    /// User clicked a colour swatch (Fill or Stroke section). Host
    /// opens the floating colour picker tied to that target.
    OpenColorPicker(op_editor_core::ColorTarget),
    /// User clicked anywhere in the Export section — host queues
    /// `FileAction::ExportImage` so the picker dialog opens.
    OpenExportDialog,
    /// User clicked the Effects section's "+" — host appends a
    /// default drop shadow to the selected node.
    AddEffect,
    /// User clicked the "✕" on an effect row — host removes the
    /// effect at this index from the selected node.
    RemoveEffect(usize),
    /// User clicked a "−" / "+" stepper on an effect parameter row.
    /// `new_value` is the post-step value (the walker computed it
    /// from the current value ± the step); the host writes it via
    /// `EditorCommand::SetEffectParam`.
    AdjustEffectParam {
        effect: usize,
        field: op_editor_core::EffectField,
        new_value: f32,
    },
}

/// Per-NodeKind toggles for which property-panel sections render.
/// Mirrors the TS app's behaviour where a Line node hides the
/// fill picker, a Frame hides Text properties, etc. Sections that
/// always apply (Position / Layer / Export) aren't gated here.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SectionCapabilities {
    pub(crate) flex_layout: bool,
    pub(crate) size_options: bool,
    pub(crate) opacity: bool,
    pub(crate) fill: bool,
    pub(crate) stroke: bool,
    pub(crate) effects: bool,
    pub(crate) export: bool,
}

impl SectionCapabilities {
    /// Capability mask for the multi-select aggregate snapshot.
    /// Keeps Size (so the union W/H actually paint), hides
    /// fill/stroke (no aggregation in v1), keeps Layer/Effects/
    /// Export (paint safely with the zeroed snapshot fields).
    fn for_multi() -> Self {
        Self {
            flex_layout: false,
            size_options: true,
            opacity: true,
            fill: false,
            stroke: false,
            effects: true,
            export: true,
        }
    }

    pub(crate) fn for_kind(kind: &crate::layout_scene::NodeKind) -> Self {
        use crate::layout_scene::NodeKind as K;
        match kind {
            // Frame: full chrome — it can host children, take auto-
            // layout, fill / stroke / effects / export.
            K::Frame => Self {
                flex_layout: true,
                size_options: true,
                opacity: true,
                fill: true,
                stroke: true,
                effects: true,
                export: true,
            },
            // Group: structural — no fill / stroke, no flex slot
            // (children own layout). Opacity + export still apply.
            K::Group | K::Other(_) => Self {
                flex_layout: false,
                size_options: false,
                opacity: true,
                fill: false,
                stroke: false,
                effects: true,
                export: true,
            },
            // Rect / Ellipse / Polygon: full leaf — every paint
            // section applies; no flex (no children).
            K::Rect | K::Ellipse | K::Polygon => Self {
                flex_layout: false,
                size_options: true,
                opacity: true,
                fill: true,
                stroke: true,
                effects: true,
                export: true,
            },
            // Line / Path: only outline — fill doesn't apply.
            K::Line | K::Path => Self {
                flex_layout: false,
                size_options: true,
                opacity: true,
                fill: false,
                stroke: true,
                effects: true,
                export: true,
            },
            // Text: stroke is rare for text, but fill = ink colour.
            K::Text => Self {
                flex_layout: false,
                size_options: true,
                opacity: true,
                fill: true,
                stroke: false,
                effects: true,
                export: true,
            },
        }
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
    pub fill: Option<Color>,
    pub stroke: Option<SceneStroke>,
    /// The node's visual effects, in paint order — drives the
    /// Effects section's rows + param inputs.
    pub effects: Vec<EffectSummary>,
    /// Drives per-kind section filtering (Line hides fill, etc.).
    pub kind_variant: crate::layout_scene::NodeKind,
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
            },
            PenEffect::Blur(b) => EffectSummary {
                kind: EffectKind::Blur,
                offset_x: 0.0,
                offset_y: 0.0,
                blur: b.radius,
                spread: 0.0,
            },
            PenEffect::BackgroundBlur(b) => EffectSummary {
                kind: EffectKind::BackgroundBlur,
                offset_x: 0.0,
                offset_y: 0.0,
                blur: b.radius,
                spread: 0.0,
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
            stroke: None,
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
            stroke,
            effects: op_editor_core::node_effects(node)
                .iter()
                .map(EffectSummary::from_pen_effect)
                .collect(),
            kind_variant: kind,
        }
    }
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
    /// Current export format + scale, surfaced as preview pills in
    /// the Export section. Clicking the section opens the modal.
    pub export_format: crate::widgets::export_dialog::ExportFormat,
    pub export_scale: f32,
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
            export_format: doc_export_format(ui.export_format),
            export_scale: ui.export_scale,
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
        let caps = SectionCapabilities::for_kind(&self.snapshot.kind_variant);
        let visible = sections::VisibleSections {
            flex_layout: caps.flex_layout,
            size_options: caps.size_options,
            opacity: caps.opacity,
            fill: caps.fill,
            stroke: caps.stroke,
            effects: caps.effects,
            export: caps.export,
            fill_type: self.fill_type,
        };
        let rects = sections::action_button_rects_with_fill_picker(
            panel_rect,
            visible,
            &self.snapshot.effects,
            self.fill_type_picker_open,
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

    /// Hit-test the panel at `point` and return which input row
    /// (if any) contains the click. The layout walk mirrors the
    /// per-kind section filtering applied in `paint`, so rects
    /// after a skipped section don't drift out of alignment.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<PropertyFocus> {
        if self.is_multi {
            // Inputs inert in v1 multi-select aggregate view.
            return None;
        }
        let caps = SectionCapabilities::for_kind(&self.snapshot.kind_variant);
        let visible = sections::VisibleSections {
            flex_layout: caps.flex_layout,
            size_options: caps.size_options,
            opacity: caps.opacity,
            fill: caps.fill,
            stroke: caps.stroke,
            effects: caps.effects,
            export: caps.export,
            fill_type: self.fill_type,
        };
        for (focus, rect) in sections::editable_input_rects(panel_rect, visible) {
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
        let mut y = rect.origin.y;
        let edit_ctx = sections::EditContext {
            focus: self.focus,
            draft: self.draft.as_str(),
            caret_anchor_ms: self.caret_anchor_ms,
            now_ms: self.now_ms,
        };
        use crate::layout_scene::NodeKind;
        let caps = self.capabilities();
        y = sections::paint_tab_strip(cx, &self.theme, &self.labels, self.tab, x, y, w);
        if matches!(self.tab, op_editor_core::PropertyTab::Code) {
            paint_code_placeholder(cx, &self.theme, &self.snapshot, x, y, w);
            return;
        }
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
            let visible = sections::VisibleSections {
                flex_layout: caps.flex_layout,
                size_options: caps.size_options,
                opacity: caps.opacity,
                fill: caps.fill,
                stroke: caps.stroke,
                effects: caps.effects,
                export: caps.export,
                fill_type: self.fill_type,
            };
            sections::paint_fill_type_picker(cx, &self.theme, rect, visible, self.fill_type);
        }
        let _ = NodeKind::Frame; // ensure NodeKind is in scope above for tests
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(self.snapshot.kind.clone());
        node
    }
}
