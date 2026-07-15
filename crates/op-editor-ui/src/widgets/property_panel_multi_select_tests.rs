//! Multi-selection behaviour for `widgets::property_panel`: the
//! aggregate snapshot, inert hit-testing, the reduced capability mask,
//! and the selection-scoped padding-mode pin.

use super::property_panel::PropertyPanel;
use super::property_panel_test_support::{paint_and_count, state_from};
use crate::{Point2D, Rect};
use op_editor_core::{EditorState, NodeId};

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
        r##"{ "version": "1.0.0", "children": [
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

#[test]
fn padding_pin_does_not_leak_into_a_different_selection() {
    use op_editor_core::PaddingEditMode;
    let mut state = state_from(
        r#"{"version":"1.0.0","children":[
          {"type":"frame","id":"a","name":"A","x":0,"y":0,"width":100,"height":100},
          {"type":"frame","id":"b","name":"B","x":200,"y":0,"width":100,"height":100}
        ]}"#,
    );
    // The user pins "Individual" padding mode for node A.
    state.editor_ui.padding_edit_mode = Some(PaddingEditMode::Individual);
    state.editor_ui.padding_edit_mode_anchor = "a".to_string();
    // While A is selected the pin applies.
    state.set_single_selection(NodeId::new("a"));
    let panel_a = PropertyPanel::for_selection(&state).unwrap();
    assert_eq!(panel_a.padding_edit_mode, PaddingEditMode::Individual);
    // Selecting B must NOT inherit A's pin — B has no padding, so the
    // panel derives Single. Regression for the leaked-padding-mode bug.
    state.set_single_selection(NodeId::new("b"));
    let panel_b = PropertyPanel::for_selection(&state).unwrap();
    assert_eq!(panel_b.padding_edit_mode, PaddingEditMode::Single);
}
