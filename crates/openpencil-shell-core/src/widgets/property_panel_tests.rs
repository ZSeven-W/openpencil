//! Tests for `widgets::property_panel` — moved to a sibling file to
//! keep `property_panel.rs` under the 800-line cap.

use super::property_panel::{PropertyPanel, PropertyPanelAction, SectionCapabilities};
use super::property_panel_sections as sections;
use crate::document::{Document, Node, NodeId, NodeKind, PropertyFocus};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget};
use crate::{Color, Point2D, Rect};


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
    doc.set_single_selection(NodeId::new("n9999"));
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
    doc.set_single_selection(NodeId::new("n12"));
    let panel = PropertyPanel::for_selection(&doc).unwrap();
    assert_eq!(panel.snapshot.kind, "Group");
    assert_eq!(panel.snapshot.x, 60);
    assert_eq!(panel.snapshot.y, 130);
    assert!(panel.snapshot.width > 0);
    assert!(panel.snapshot.height > 0);
}

#[test]
fn hit_test_action_export_section_returns_open_dialog() {
    // Single-frame selection paints every section + Export.
    // The walker now extends through Stroke + Effects so the
    // Export row's hit-test rect resolves to OpenExportDialog.
    let mut doc = Document::sample();
    // NodeId 10 is the Frame at the root of the sample document
    // (mutators.rs::sample). Frame paints every section including
    // Stroke + Effects + Export so the walker has to advance past
    // all of them to reach the Export row.
    doc.set_single_selection(NodeId::new("n10"));
    let panel = PropertyPanel::for_selection(&doc).expect("frame panel");
    // Tall panel rect so every section fits without clipping.
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1600.0),
    };
    // The Export section is the last section painted. Walk
    // the action-button rects looking for the OpenExportDialog
    // rect, then click its center and assert we get back the
    // OpenExportDialog action.
    let caps = SectionCapabilities::for_kind(&panel.snapshot.kind_variant);
    let visible = sections::VisibleSections {
        flex_layout: caps.flex_layout,
        size_options: caps.size_options,
        opacity: caps.opacity,
        fill: caps.fill,
        stroke: caps.stroke,
        effects: caps.effects,
        export: caps.export,
        fill_type: panel.fill_type,
    };
    let rects = sections::action_button_rects_with_fill_picker(rect, visible, false);
    let export_rect = rects
        .iter()
        .find(|(action, _)| {
            matches!(action, PropertyPanelAction::OpenExportDialog)
        })
        .map(|(_, r)| *r)
        .expect("export section must emit an OpenExportDialog rect");
    let center = Point2D::new(
        export_rect.origin.x + export_rect.size.x / 2.0,
        export_rect.origin.y + export_rect.size.y / 2.0,
    );
    let hit = panel.hit_test_action(rect, center);
    assert!(
        matches!(hit, Some(PropertyPanelAction::OpenExportDialog)),
        "click in Export section should resolve to OpenExportDialog, got {:?}",
        hit
    );
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
    doc.set_single_selection(NodeId::new("n11"));
    doc.toggle_selection(NodeId::new("n12"));
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
    doc.set_single_selection(NodeId::new("n11"));
    doc.toggle_selection(NodeId::new("n12"));
    let panel_multi = PropertyPanel::for_selection(&doc).expect("multi");
    doc.set_single_selection(NodeId::new("n10"));
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
    doc.set_single_selection(NodeId::new("n11"));
    doc.toggle_selection(NodeId::new("n12"));
    let panel = PropertyPanel::for_selection(&doc).expect("multi-select panel");
    assert!(panel.is_multi);
    let caps = panel.capabilities();
    assert!(caps.size_options, "multi-select must paint W/H");
    assert!(!caps.fill, "multi-select must hide fill section");
    assert!(!caps.stroke, "multi-select must hide stroke section");
    assert!(!caps.flex_layout, "multi-select hides flex");
    // Cross-check the single-select fallback: a Rect selection
    // routes through `for_kind`, which exposes fill/stroke.
    doc.set_single_selection(NodeId::new("n13")); // Button background (Rect)
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
        Node::leaf("n50", NodeKind::Rect, "A"),
        Node::leaf("n51", NodeKind::Rect, "B"),
    ];
    doc.set_single_selection(NodeId::new("n50"));
    doc.toggle_selection(NodeId::new("n51"));
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
    doc.set_single_selection(NodeId::new("n11"));
    assert!(doc.property_panel_visible());
    // Multi with valid union bounds → visible.
    doc.toggle_selection(NodeId::new("n12"));
    assert!(doc.property_panel_visible());
}
