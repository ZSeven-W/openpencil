//! Geometry mutator tests — translate / bounds / rotation /
//! property edits + the locked / hidden editability gates. Ported
//! from `openpencil-shell-core::document::tests_geometry`.

#![cfg(test)]

use crate::geometry::{aggregate_bounds, DocRect};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{frame, rect, state_with};
use crate::ui_draft::PropertyFocus;
use crate::walkers::find_node;

fn node_x(s: &crate::state::EditorState, id: &str) -> f64 {
    find_node(s.active_children(), &NodeId::new(id))
        .unwrap()
        .base()
        .x
        .unwrap_or(0.0)
}

#[test]
fn translate_selected_shifts_a_leaf() {
    let mut s = state_with(vec![rect("n1", "A", 10.0, 10.0, 50.0, 50.0)]);
    s.set_single_selection(NodeId::new("n1"));
    s.translate_selected(7.0, 3.0);
    let n = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert_eq!(n.base().x, Some(17.0));
    assert_eq!(n.base().y, Some(13.0));
}

#[test]
fn translate_selected_cascades_into_children() {
    let mut s = state_with(vec![frame(
        "n1",
        "Frame",
        10.0,
        10.0,
        100.0,
        100.0,
        vec![rect("n2", "Child", 20.0, 20.0, 10.0, 10.0)],
    )]);
    s.set_single_selection(NodeId::new("n1"));
    s.translate_selected(5.0, 5.0);
    assert_eq!(node_x(&s, "n1"), 15.0);
    // Child carries absolute coords — must shift with the parent.
    assert_eq!(node_x(&s, "n2"), 25.0);
}

#[test]
fn translate_selected_dedups_ancestor_and_descendant() {
    let mut s = state_with(vec![frame(
        "n1",
        "Frame",
        0.0,
        0.0,
        100.0,
        100.0,
        vec![rect("n2", "Child", 10.0, 10.0, 10.0, 10.0)],
    )]);
    // Both ancestor + descendant selected — child must shift once.
    s.clear_selection();
    s.toggle_selection(NodeId::new("n1"));
    s.toggle_selection(NodeId::new("n2"));
    s.translate_selected(5.0, 0.0);
    assert_eq!(node_x(&s, "n2"), 15.0);
}

#[test]
fn set_selected_bounds_overwrites_a_leaf_rect() {
    let mut s = state_with(vec![rect("n1", "A", 0.0, 0.0, 50.0, 50.0)]);
    s.set_single_selection(NodeId::new("n1"));
    s.set_selected_bounds(DocRect {
        x: 100.0,
        y: 200.0,
        w: 30.0,
        h: 40.0,
    });
    let n = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert_eq!(n.base().x, Some(100.0));
    assert_eq!(n.base().y, Some(200.0));
    assert_eq!(n.width_px(), Some(30.0));
    assert_eq!(n.height_px(), Some(40.0));
}

#[test]
fn set_selected_rotation_writes_degrees() {
    let mut s = state_with(vec![rect("n1", "A", 0.0, 0.0, 50.0, 50.0)]);
    s.set_single_selection(NodeId::new("n1"));
    s.set_selected_rotation(std::f32::consts::FRAC_PI_2); // 90 deg
    let rot = find_node(s.active_children(), &NodeId::new("n1"))
        .unwrap()
        .base()
        .rotation
        .unwrap();
    assert!((rot - 90.0).abs() < 1e-3);
}

#[test]
fn commit_property_edit_writes_position_and_size() {
    let mut s = state_with(vec![rect("n1", "A", 0.0, 0.0, 50.0, 50.0)]);
    s.set_single_selection(NodeId::new("n1"));
    assert!(s.commit_property_edit(PropertyFocus::PositionX, 123.0));
    assert!(s.commit_property_edit(PropertyFocus::SizeW, 88.0));
    let n = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert_eq!(n.base().x, Some(123.0));
    assert_eq!(n.width_px(), Some(88.0));
}

// --- Editability gates ----------------------------------------------

#[test]
fn locked_selection_blocks_translate_bounds_rotation_and_property_edit() {
    let mut locked = rect("n40", "locked", 10.0, 10.0, 50.0, 50.0);
    locked.base_mut().locked = Some(true);
    let mut s = state_with(vec![locked]);
    s.set_single_selection(NodeId::new("n40"));

    s.translate_selected(7.0, 3.0);
    assert_eq!(node_x(&s, "n40"), 10.0);
    s.set_selected_bounds(DocRect {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    });
    assert_eq!(node_x(&s, "n40"), 10.0);
    s.set_selected_rotation(1.5);
    assert!(
        find_node(s.active_children(), &NodeId::new("n40"))
            .unwrap()
            .base()
            .rotation
            .is_none()
    );
    assert!(!s.commit_property_edit(PropertyFocus::PositionX, 999.0));
    assert_eq!(node_x(&s, "n40"), 10.0);
}

#[test]
fn hidden_selection_blocks_translate_and_property_edit() {
    let mut hidden = rect("n41", "hidden", 10.0, 10.0, 50.0, 50.0);
    hidden.base_mut().visible = Some(false);
    let mut s = state_with(vec![hidden]);
    s.set_single_selection(NodeId::new("n41"));
    s.translate_selected(7.0, 3.0);
    assert_eq!(node_x(&s, "n41"), 10.0);
    assert!(!s.commit_property_edit(PropertyFocus::PositionX, 999.0));
}

// --- Aggregate bounds ------------------------------------------------

#[test]
fn aggregate_bounds_of_leaf_is_its_own_rect() {
    let n = rect("n1", "A", 5.0, 6.0, 30.0, 40.0);
    let b = aggregate_bounds(&n);
    assert_eq!(b, DocRect { x: 5.0, y: 6.0, w: 30.0, h: 40.0 });
}

#[test]
fn aggregate_bounds_of_bounded_frame_uses_own_rect() {
    let f = frame(
        "n1",
        "Frame",
        0.0,
        0.0,
        200.0,
        100.0,
        vec![rect("n2", "Child", 500.0, 500.0, 10.0, 10.0)],
    );
    // Bounded frame: own rect wins even though the child is outside.
    let b = aggregate_bounds(&f);
    assert_eq!(b.w, 200.0);
    assert_eq!(b.h, 100.0);
}

#[test]
fn selection_bounds_unions_multiple_nodes() {
    let mut s = state_with(vec![
        rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "B", 90.0, 90.0, 10.0, 10.0),
    ]);
    s.clear_selection();
    s.toggle_selection(NodeId::new("n1"));
    s.toggle_selection(NodeId::new("n2"));
    let b = s.selection_bounds().expect("union");
    assert_eq!(b, DocRect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 });
}
