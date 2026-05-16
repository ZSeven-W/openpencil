//! Tests for `widgets::property_panel` — moved to a sibling file to
//! keep `property_panel.rs` under the 800-line cap.
//!
//! Phase 6: the panel builds from `op_editor_core::EditorState`, so
//! the fixtures construct `EditorState` values.

use super::property_panel::{PropertyPanel, PropertyPanelAction, SectionCapabilities};
use super::property_panel_sections as sections;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect};
use op_editor_core::{EditorState, NodeId};

/// Build an `EditorState` from a canonical `.op` JSON string.
fn state_from(src: &str) -> EditorState {
    let doc = jian_ops_schema::load_str(src)
        .expect("property-panel fixture parses")
        .value;
    EditorState::from_document(doc)
}

#[test]
fn for_selection_with_real_node_builds_snapshot() {
    let state = EditorState::sample();
    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
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
    let state = EditorState::new();
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn for_selection_with_stale_selection_returns_none() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n9999"));
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn access_node_advertises_group_with_kind_label() {
    let state = EditorState::sample();
    let panel = PropertyPanel::for_selection(&state).unwrap();
    let node = panel.access_node();
    assert_eq!(node.role(), accesskit::Role::Group);
    assert_eq!(node.label(), Some("Text"));
}

#[test]
fn group_snapshot_aggregates_child_bounds() {
    // A Group has no own bounds, so `from_node` must derive W/H
    // from children — else the panel shows "0 × 0" for a container.
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n12"));
    let panel = PropertyPanel::for_selection(&state).unwrap();
    assert_eq!(panel.snapshot.kind, "Group");
    assert_eq!(panel.snapshot.x, 60);
    assert_eq!(panel.snapshot.y, 130);
    assert!(panel.snapshot.width > 0);
    assert!(panel.snapshot.height > 0);
}

#[test]
fn hit_test_action_export_section_returns_open_dialog() {
    // Single-frame selection paints every section + Export.
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n10"));
    let panel = PropertyPanel::for_selection(&state).expect("frame panel");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 1600.0),
    };
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
        .find(|(action, _)| matches!(action, PropertyPanelAction::OpenExportDialog))
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
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    state.toggle_selection(NodeId::new("n12"));
    assert_eq!(state.selection_count(), 2);

    let panel = PropertyPanel::for_selection(&state).expect("multi-select must paint");
    assert!(panel.is_multi);
    assert_eq!(panel.snapshot.kind, "2 items");
    assert_eq!(panel.snapshot.x, 60);
    assert_eq!(panel.snapshot.y, 60);
    // Union spans Title (y 60..88) + Button group (y 130..166) →
    // x=60, w=240, h≈106.
    assert!(panel.snapshot.width >= 240);
    assert!(panel.snapshot.height >= 100);
    assert!(panel.focus.is_none());
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 600.0),
    };
    assert!(panel.hit_test(rect, Point2D::new(140.0, 100.0)).is_none());
    assert!(panel
        .hit_test_action(rect, Point2D::new(140.0, 100.0))
        .is_none());
}

/// Minimal `RenderBackend` that counts paint ops.
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
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    state.toggle_selection(NodeId::new("n12"));
    let panel_multi = PropertyPanel::for_selection(&state).expect("multi");
    state.set_single_selection(NodeId::new("n10"));
    let panel_frame = PropertyPanel::for_selection(&state).expect("frame");
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
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n11"));
    state.toggle_selection(NodeId::new("n12"));
    let panel = PropertyPanel::for_selection(&state).expect("multi-select panel");
    assert!(panel.is_multi);
    let caps = panel.capabilities();
    assert!(caps.size_options, "multi-select must paint W/H");
    assert!(!caps.fill, "multi-select must hide fill section");
    assert!(!caps.stroke, "multi-select must hide stroke section");
    assert!(!caps.flex_layout, "multi-select hides flex");
    // A Rect selection routes through `for_kind`, exposing fill/stroke.
    state.set_single_selection(NodeId::new("n13"));
    let single = PropertyPanel::for_selection(&state).expect("single-select panel");
    let caps_single = single.capabilities();
    assert!(caps_single.fill, "single Rect must paint fill");
    assert!(caps_single.stroke, "single Rect must paint stroke");
}

#[test]
fn multi_select_panel_shows_even_when_all_zero_size() {
    // Symmetry with single-select: a 0x0 node still shows the panel.
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n50","name":"A"},
              {"type":"rectangle","id":"n51","name":"B"}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("n50"));
    state.toggle_selection(NodeId::new("n51"));
    assert_eq!(state.selection_count(), 2);
    let panel = PropertyPanel::for_selection(&state).expect("0x0 multi-select must paint");
    assert!(panel.is_multi);
    assert_eq!(panel.snapshot.width, 0);
    assert_eq!(panel.snapshot.height, 0);
}
