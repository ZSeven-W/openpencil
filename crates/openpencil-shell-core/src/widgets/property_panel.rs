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
//!   1. Tab strip (设计 / 代码)
//!   2. Header (kind label) + 创建组件 button
//!   3. 位置 — X / Y / rotation / R
//!   4. 弹性布局 — 3 layout-mode buttons
//!   5. 尺寸 — W / H + 5 sizing checkboxes
//!   6. 图层 — opacity row
//!   7. 填充 — solid color rows + add affordance
//!   8. 描边 — color + width row
//!   9. 效果 — empty list + add affordance
//!  10. 导出 — scale + format dropdowns
//!
//! Conditional rendering: TS app does `{hasSelection && <RightPanel/>}`.
//! Host calls [`PropertyPanel::for_selection`] which returns
//! `Option<Self>`; `None` = panel hidden entirely.

use crate::document::{Document, Node, PropertyFocus, Stroke};
use crate::theme::Theme;
use crate::widgets::property_panel_sections as sections;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub const PROPERTY_PANEL_WIDTH: f32 = 280.0;

/// Button / checkbox actions in the property panel that don't
/// map to a text input. The host dispatches these in `apply_press`
/// after the text-input hit-test misses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyPanelAction {
    SetFlexLayout(crate::document::FlexLayout),
    ToggleSizeFillWidth,
    ToggleSizeFillHeight,
    ToggleSizeHugWidth,
    ToggleSizeHugHeight,
    ToggleSizeClipContent,
    /// User clicked the Fill section's "纯色" dropdown — host
    /// toggles `Document.ui.fill_type_picker_open`.
    ToggleFillTypePicker,
    /// User picked a fill type from the dropdown.
    SetFillType(crate::document::FillType),
    /// User clicked a colour swatch (Fill or Stroke section). Host
    /// opens the floating colour picker tied to that target.
    OpenColorPicker(crate::document::ColorTarget),
    /// User clicked anywhere in the Export section — host queues
    /// `FileAction::ExportImage` so the picker dialog opens.
    OpenExportDialog,
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

    fn for_kind(kind: &crate::document::NodeKind) -> Self {
        use crate::document::NodeKind as K;
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
    pub stroke: Option<Stroke>,
    /// Drives per-kind section filtering (Line hides fill, etc.).
    pub kind_variant: crate::document::NodeKind,
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
    fn from_multi_selection(doc: &Document) -> Option<Self> {
        // Confirm at least one selected id resolves on the active
        // page — bails on cross-page selections (where nothing
        // resolves) but NOT on all-zero-size selections (matches
        // single-select semantics, which paint the panel even
        // for a 0x0 node).
        if !doc.property_panel_visible() || doc.selection_count() < 2 {
            return None;
        }
        let bounds = doc.selection_bounds().unwrap_or(crate::Rect::ZERO);
        let n = doc.selection_count();
        Some(Self {
            kind: format!("{} items", n),
            name: format!("{} selected", n),
            x: bounds.origin.x.round() as i32,
            y: bounds.origin.y.round() as i32,
            width: bounds.size.x.round() as i32,
            height: bounds.size.y.round() as i32,
            rotation_deg: 0.0,
            corner_radius: 0.0,
            fill: None,
            stroke: None,
            // `kind_variant` is informational for the snapshot
            // header label only — the paint capability mask is
            // driven by `SectionCapabilities::for_multi()` instead
            // of `for_kind`, see `paint`. Frame chosen so any
            // future kind-specific lookups paint a neutral default.
            kind_variant: crate::document::NodeKind::Frame,
        })
    }

    fn from_node(node: &Node) -> Self {
        // Use `aggregate_bounds` so Group / unbounded container
        // nodes (Frame without explicit bounds, Other(_)) report
        // the visual extent of their subtree instead of "0 × 0"
        // (codex Step 6 stop-hook fix).
        let bounds = node.aggregate_bounds();
        Self {
            kind: node.kind.label().to_string(),
            name: node.name.clone(),
            x: bounds.origin.x.round() as i32,
            y: bounds.origin.y.round() as i32,
            width: bounds.size.x.round() as i32,
            height: bounds.size.y.round() as i32,
            rotation_deg: node.rotation.to_degrees(),
            corner_radius: node.corner_radius,
            fill: node.fill,
            stroke: node.stroke,
            kind_variant: node.kind.clone(),
        }
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
    pub flex_layout: crate::document::FlexLayout,
    /// 5 size checkboxes — fill / hug / clip.
    pub size_flags: sections::SizeFlags,
    /// Active fill type — drives the dropdown label + picker.
    pub fill_type: crate::document::FillType,
    pub fill_type_picker_open: bool,
    /// True for multi-select aggregate (inputs inert, "N items").
    pub is_multi: bool,
    /// Active header tab — toggled by Cmd+Shift+C.
    pub tab: crate::document::PropertyTab,
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
    /// Conditional builder — returns `Some` only when the document
    /// has an active selection. Mirrors TS `{hasSelection && ...}`.
    pub fn for_selection(doc: &Document) -> Option<Self> {
        Self::for_selection_at(doc, 0)
    }

    /// Same as [`for_selection`] but threads the host's monotonic
    /// millisecond clock through so the focused-input caret can
    /// blink off the same animation timer as the chat input.
    pub fn for_selection_at(doc: &Document, now_ms: u64) -> Option<Self> {
        if doc.selection_count() == 1 {
            let node = doc.selected_node()?;
            return Some(Self::build_from_snapshot(
                doc,
                NodeSnapshot::from_node(node),
                node.fill_type,
                now_ms,
                false,
            ));
        }
        if doc.selection_count() >= 2 {
            let snapshot = NodeSnapshot::from_multi_selection(doc)?;
            return Some(Self::build_from_snapshot(
                doc,
                snapshot,
                crate::document::FillType::Solid,
                now_ms,
                true,
            ));
        }
        None
    }

    fn build_from_snapshot(
        doc: &Document,
        snapshot: NodeSnapshot,
        fill_type: crate::document::FillType,
        now_ms: u64,
        is_multi: bool,
    ) -> Self {
        Self {
            id: WidgetId::new(2000),
            snapshot,
            theme: doc.theme(),
            labels: sections::PropertyLabels::for_document(doc),
            // Multi-select inputs are inert in v1 — broadcast edits
            // to all selected nodes lands later. Force focus to None
            // so the panel paints all values muted and hit_test
            // returns None (see `hit_test` is_multi short-circuit).
            focus: if is_multi {
                None
            } else {
                doc.ui.property_focus
            },
            draft: if is_multi {
                String::new()
            } else {
                doc.ui.property_input_draft.clone()
            },
            caret_anchor_ms: doc.ui.property_caret_anchor_ms,
            now_ms,
            flex_layout: doc.ui.flex_layout,
            size_flags: sections::SizeFlags {
                fill_width: doc.ui.size_fill_width,
                fill_height: doc.ui.size_fill_height,
                hug_width: doc.ui.size_hug_width,
                hug_height: doc.ui.size_hug_height,
                clip_content: doc.ui.size_clip_content,
            },
            fill_type,
            fill_type_picker_open: doc.ui.fill_type_picker_open,
            is_multi,
            tab: doc.ui.property_tab,
            export_format: doc.ui.export_format,
            export_scale: doc.ui.export_scale,
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
            fill_type: self.fill_type,
        };
        let rects = sections::action_button_rects_with_fill_picker(
            panel_rect,
            visible,
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
        use crate::document::NodeKind;
        let caps = self.capabilities();
        y = sections::paint_tab_strip(cx, &self.theme, &self.labels, self.tab, x, y, w);
        if matches!(self.tab, crate::document::PropertyTab::Code) {
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
            y = sections::paint_effects_section(cx, &self.theme, &self.labels, x, y, w);
        }
        if caps.export {
            let _ = sections::paint_export_section(
                cx, &self.theme, &self.labels,
                self.export_format, self.export_scale,
                x, y, w,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{NodeId, NodeKind};

    #[test]
    fn for_selection_with_real_node_builds_snapshot() {
        let doc = Document::sample();
        let panel = PropertyPanel::for_selection(&doc).expect("sample doc has a selection");
        assert_eq!(panel.snapshot.kind, "Text");
        assert_eq!(panel.snapshot.name, "Title");
        // Title node bounds: (60, 60, 240, 28).
        assert_eq!(panel.snapshot.x, 60);
        assert_eq!(panel.snapshot.y, 60);
        assert_eq!(panel.snapshot.width, 240);
        assert_eq!(panel.snapshot.height, 28);
    }

    #[test]
    fn for_selection_without_selection_returns_none() {
        let doc = Document::empty();
        assert!(PropertyPanel::for_selection(&doc).is_none());
    }

    #[test]
    fn for_selection_with_stale_selection_returns_none() {
        let mut doc = Document::sample();
        doc.set_single_selection(NodeId::new(9999));
        assert!(PropertyPanel::for_selection(&doc).is_none());
    }

    #[test]
    fn access_node_advertises_group_with_kind_label() {
        let doc = Document::sample();
        let panel = PropertyPanel::for_selection(&doc).unwrap();
        let node = panel.access_node();
        assert_eq!(node.role(), accesskit::Role::Group);
        assert_eq!(node.label(), Some("Text"));
    }

    #[test]
    fn group_snapshot_aggregates_child_bounds() {
        // Codex Step 6 stop-hook fix: a Group has bounds = ZERO,
        // so `from_node` must derive W/H from children — else
        // the panel shows "0 × 0" for any container.
        let doc = Document::sample();
        // Select the "Button" group (id 12). Its children:
        //   - Button background rect (60, 130, 180, 36)
        //   - Click me text       (76, 152, 160, 16)
        // Aggregate bounds: (60, 130, 240-60=180, 168-130=38).
        let mut doc = doc;
        doc.set_single_selection(NodeId::new(12));
        let panel = PropertyPanel::for_selection(&doc).unwrap();
        assert_eq!(panel.snapshot.kind, "Group");
        assert_eq!(panel.snapshot.x, 60);
        assert_eq!(panel.snapshot.y, 130);
        assert!(panel.snapshot.width > 0);
        assert!(panel.snapshot.height > 0);
    }

    #[test]
    fn format_color_hex_pads_to_six_chars() {
        use crate::widgets::property_panel_inputs::format_color_hex;
        assert_eq!(format_color_hex(Color::WHITE), "#FFFFFF");
        assert_eq!(format_color_hex(Color::BLACK), "#000000");
        assert_eq!(format_color_hex(Color::RED), "#FF0000");
    }

    #[test]
    fn multi_selection_panel_shows_union_bounds_and_is_inert() {
        let mut doc = Document::sample();
        // Select Title (id 11, bounds 60,60,240,28) + Button (id 12,
        // aggregate bounds 60,130,180,38). Union: x=60, y=60,
        // w=240-60+? = 60+240→300; the button right edge is 60+180=240
        // → max_x = 300 (from title). Union: x=60, y=60, w=240, h=108.
        doc.set_single_selection(NodeId::new(11));
        doc.toggle_selection(NodeId::new(12));
        assert_eq!(doc.selection_count(), 2);

        let panel = PropertyPanel::for_selection(&doc).expect("multi-select must paint");
        assert!(panel.is_multi);
        assert_eq!(panel.snapshot.kind, "2 items");
        assert_eq!(panel.snapshot.x, 60);
        assert_eq!(panel.snapshot.y, 60);
        // Union is at least as wide / tall as the larger node.
        assert!(panel.snapshot.width >= 240);
        assert!(panel.snapshot.height >= 108);
        // Inputs inert.
        assert!(panel.focus.is_none());
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(280.0, 600.0),
        };
        // Center of the panel — over an input row in single-select.
        // In multi-select, hit_test must return None.
        assert!(panel.hit_test(rect, Point2D::new(140.0, 100.0)).is_none());
        assert!(panel
            .hit_test_action(rect, Point2D::new(140.0, 100.0))
            .is_none());
    }

    /// Minimal `RenderBackend` that counts paint ops — used by the
    /// next test to observe what `paint` actually emits. Mirrors the
    /// canvas_viewport tests' `RecordingBackend`.
    #[derive(Default)]
    struct CountingBackend {
        text: usize,
        round_rects: usize,
    }
    impl crate::RenderBackend for CountingBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &crate::TextLayout, _: Point2D) {
            self.text += 1;
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
            self.round_rects += 1;
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn multi_select_paint_diverges_from_full_section_paint() {
        // Paint-output integration test: multi-select hides
        // fill+stroke+flex via `for_multi`; single-Frame paints all
        // sections via `for_kind`. If `paint` regressed to bypass
        // `capabilities()` for the multi panel, the two would emit
        // identical ops.
        let mut doc = Document::sample();
        doc.set_single_selection(NodeId::new(11));
        doc.toggle_selection(NodeId::new(12));
        let panel_multi = PropertyPanel::for_selection(&doc).expect("multi");
        doc.set_single_selection(NodeId::new(10));
        let panel_frame = PropertyPanel::for_selection(&doc).expect("frame");
        assert!(!panel_frame.is_multi);

        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(280.0, 1200.0),
        };
        let multi = paint_and_count(&panel_multi, rect);
        let frame = paint_and_count(&panel_frame, rect);
        assert_ne!(multi, frame, "multi must paint fewer ops than single-Frame");
        assert!(multi.0 > 5 && multi.1 > 0, "Size section must paint");
    }

    fn paint_and_count(panel: &PropertyPanel, rect: Rect) -> (usize, usize) {
        let mut backend = CountingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            panel.paint(&mut cx, rect);
        }
        (backend.text, backend.round_rects)
    }

    #[test]
    fn multi_select_caps_keep_size_hide_fill_and_stroke() {
        // Codex CONCERN: asserting `SectionCapabilities::for_multi()`
        // directly would self-verify the function — a regression in
        // `paint` that swapped back to `for_kind` would still pass.
        // Instead drive through `panel.capabilities()`, which is the
        // single source of truth that `paint` calls.
        let mut doc = Document::sample();
        doc.set_single_selection(NodeId::new(11));
        doc.toggle_selection(NodeId::new(12));
        let panel = PropertyPanel::for_selection(&doc).expect("multi-select panel");
        assert!(panel.is_multi);
        let caps = panel.capabilities();
        assert!(caps.size_options, "multi-select must paint W/H");
        assert!(!caps.fill, "multi-select must hide fill section");
        assert!(!caps.stroke, "multi-select must hide stroke section");
        assert!(!caps.flex_layout, "multi-select hides flex");
        // Cross-check the single-select fallback: a Rect selection
        // routes through `for_kind`, which exposes fill/stroke.
        doc.set_single_selection(NodeId::new(13)); // Button background (Rect)
        let single = PropertyPanel::for_selection(&doc).expect("single-select panel");
        let caps_single = single.capabilities();
        assert!(caps_single.fill, "single Rect must paint fill");
        assert!(caps_single.stroke, "single Rect must paint stroke");
    }

    #[test]
    fn multi_select_panel_shows_even_when_all_zero_size() {
        // Symmetry with single-select: a 0x0 node still shows the
        // panel, so two 0x0 nodes selected together must too.
        let mut doc = Document::empty();
        let p = doc.active_page_index;
        // Two leaf nodes whose `aggregate_bounds` is `Rect::ZERO`
        // (no bounds, no children). `Node::leaf` defaults to
        // zero-sized bounds.
        doc.pages[p].children = vec![
            Node::leaf(50, NodeKind::Rect, "A"),
            Node::leaf(51, NodeKind::Rect, "B"),
        ];
        doc.set_single_selection(NodeId::new(50));
        doc.toggle_selection(NodeId::new(51));
        assert_eq!(doc.selection_count(), 2);
        // Visible despite the union being None.
        assert!(doc.property_panel_visible());
        let panel = PropertyPanel::for_selection(&doc).expect("0x0 multi-select must paint");
        assert!(panel.is_multi);
        assert_eq!(panel.snapshot.width, 0);
        assert_eq!(panel.snapshot.height, 0);
    }

    #[test]
    fn property_panel_visible_handles_multi() {
        let mut doc = Document::sample();
        // Empty → not visible.
        doc.clear_selection();
        assert!(!doc.property_panel_visible());
        // Single → visible (existing behavior).
        doc.set_single_selection(NodeId::new(11));
        assert!(doc.property_panel_visible());
        // Multi with valid union bounds → visible.
        doc.toggle_selection(NodeId::new(12));
        assert!(doc.property_panel_visible());
    }
}
