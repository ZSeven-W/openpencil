//! Tests for the canvas-drag P1 gaps: selection resolution (parent
//! promotion + enter-group), container-resize descendant scaling, and
//! drag-end reorder / reparent mutators.

#![cfg(test)]

use crate::drag_mutators::{
    auto_layout_direction, parent_of, should_auto_reparent_outside_parent, DragDropTarget,
    FlexDirection,
};
use crate::geometry::DocRect;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::selection_resolve::{resolve_canvas_selection_target, SelectionResolution};
use crate::test_support::{flex_frame, flow_rect, frame, group, rect, state_with, text};
use crate::walkers::find_node;
use jian_ops_schema::node::text::TextGrowth;
use jian_ops_schema::node::PenNode;

/// `frame f1 > group g1 > rect r1` plus a sibling top-level rect.
fn nested_state() -> crate::EditorState {
    let r1 = rect("r1", "Leaf", 10.0, 10.0, 20.0, 20.0);
    let g1 = group("g1", "Inner", vec![r1]);
    let f1 = frame("f1", "Card", 100.0, 100.0, 200.0, 200.0, vec![g1]);
    let other = rect("n9", "Other", 500.0, 500.0, 40.0, 40.0);
    state_with(vec![f1, other])
}

// --- Selection resolution (GAP A) -------------------------------------

#[test]
fn click_on_nested_child_promotes_to_top_level_container() {
    let s = nested_state();
    let resolved = resolve_canvas_selection_target(
        s.active_children(),
        &NodeId::new("r1"),
        None,
        &s.selection.set,
    );
    assert_eq!(resolved, SelectionResolution::Select(NodeId::new("f1")));
}

#[test]
fn click_on_child_of_selected_container_keeps_selection() {
    let mut s = nested_state();
    s.set_single_selection(NodeId::new("f1"));
    let resolved = resolve_canvas_selection_target(
        s.active_children(),
        &NodeId::new("r1"),
        None,
        &s.selection.set,
    );
    assert_eq!(resolved, SelectionResolution::Keep);
}

#[test]
fn click_on_already_selected_node_reselects_it() {
    let mut s = nested_state();
    s.set_single_selection(NodeId::new("r1"));
    let resolved = resolve_canvas_selection_target(
        s.active_children(),
        &NodeId::new("r1"),
        None,
        &s.selection.set,
    );
    assert_eq!(resolved, SelectionResolution::Select(NodeId::new("r1")));
}

#[test]
fn entered_container_caps_promotion_at_its_child() {
    let s = nested_state();
    // Entered f1: a hit on the deep leaf promotes only to g1 (f1's
    // child on the ancestor chain), not back up to f1.
    let entered = NodeId::new("f1");
    let resolved = resolve_canvas_selection_target(
        s.active_children(),
        &NodeId::new("r1"),
        Some(&entered),
        &s.selection.set,
    );
    assert_eq!(resolved, SelectionResolution::Select(NodeId::new("g1")));
}

#[test]
fn click_on_entered_container_itself_selects_it() {
    let s = nested_state();
    let entered = NodeId::new("f1");
    let resolved = resolve_canvas_selection_target(
        s.active_children(),
        &NodeId::new("f1"),
        Some(&entered),
        &s.selection.set,
    );
    assert_eq!(resolved, SelectionResolution::Select(NodeId::new("f1")));
}

#[test]
fn top_level_hit_resolves_to_itself() {
    let s = nested_state();
    let resolved = resolve_canvas_selection_target(
        s.active_children(),
        &NodeId::new("n9"),
        None,
        &s.selection.set,
    );
    assert_eq!(resolved, SelectionResolution::Select(NodeId::new("n9")));
}

#[test]
fn image_visual_hit_skips_promotion() {
    // TS `hasImageVisual` guard: an image-filled child selects
    // directly instead of promoting to its frame.
    let mut img = rect("img", "Photo", 10.0, 10.0, 50.0, 50.0);
    crate::fills::set_primary_fill_type(&mut img, crate::FillType::Image);
    let f1 = frame("f1", "Card", 0.0, 0.0, 200.0, 200.0, vec![img]);
    let s = state_with(vec![f1]);
    let resolved = resolve_canvas_selection_target(
        s.active_children(),
        &NodeId::new("img"),
        None,
        &s.selection.set,
    );
    assert_eq!(resolved, SelectionResolution::Select(NodeId::new("img")));
}

#[test]
fn sync_exits_entered_container_when_selection_lands_outside() {
    let mut s = nested_state();
    s.editor_ui.entered_container = Some(NodeId::new("f1"));
    s.set_single_selection(NodeId::new("n9"));
    s.sync_entered_container_with_selection();
    assert_eq!(s.editor_ui.entered_container, None);
}

#[test]
fn sync_keeps_entered_container_for_inside_or_self_selection() {
    let mut s = nested_state();
    s.editor_ui.entered_container = Some(NodeId::new("f1"));
    s.set_single_selection(NodeId::new("r1"));
    s.sync_entered_container_with_selection();
    assert_eq!(s.editor_ui.entered_container, Some(NodeId::new("f1")));
    // The container itself still counts as inside.
    s.set_single_selection(NodeId::new("f1"));
    s.sync_entered_container_with_selection();
    assert_eq!(s.editor_ui.entered_container, Some(NodeId::new("f1")));
}

#[test]
fn sync_exits_entered_container_on_cleared_selection() {
    let mut s = nested_state();
    s.editor_ui.entered_container = Some(NodeId::new("f1"));
    s.clear_selection();
    s.sync_entered_container_with_selection();
    assert_eq!(s.editor_ui.entered_container, None);
}

// --- Drag-end reorder / reparent (GAP B) -------------------------------

#[test]
fn reorder_child_to_index_moves_within_auto_layout_parent() {
    let f = flex_frame(
        "f1",
        "Stack",
        0.0,
        0.0,
        200.0,
        300.0,
        vec![
            flow_rect("a", "A", 100.0, 40.0),
            flow_rect("b", "B", 100.0, 40.0),
            flow_rect("c", "C", 100.0, 40.0),
        ],
    );
    let mut s = state_with(vec![f]);
    // Index counted among the siblings WITHOUT the dragged node:
    // dropping `a` after `b` (before `c`) is index 1.
    assert!(s.reorder_child_to_index(&NodeId::new("f1"), &NodeId::new("a"), 1));
    let parent = find_node(s.active_children(), &NodeId::new("f1")).unwrap();
    let order: Vec<&str> = parent
        .children()
        .unwrap()
        .iter()
        .map(|c| c.id_str())
        .collect();
    assert_eq!(order, vec!["b", "a", "c"]);
}

#[test]
fn reorder_child_to_index_clamps_to_tail_and_reports_noop() {
    let f = flex_frame(
        "f1",
        "Stack",
        0.0,
        0.0,
        200.0,
        300.0,
        vec![
            flow_rect("a", "A", 100.0, 40.0),
            flow_rect("b", "B", 100.0, 40.0),
        ],
    );
    let mut s = state_with(vec![f]);
    // Same position → no order change.
    assert!(!s.reorder_child_to_index(&NodeId::new("f1"), &NodeId::new("a"), 0));
    // Out-of-range index clamps to the tail.
    assert!(s.reorder_child_to_index(&NodeId::new("f1"), &NodeId::new("a"), 99));
    let parent = find_node(s.active_children(), &NodeId::new("f1")).unwrap();
    let order: Vec<&str> = parent
        .children()
        .unwrap()
        .iter()
        .map(|c| c.id_str())
        .collect();
    assert_eq!(order, vec!["b", "a"]);
}

#[test]
fn layout_arrow_down_moves_selection_forward_in_vertical_container() {
    let f = flex_frame(
        "f1",
        "Stack",
        0.0,
        0.0,
        200.0,
        300.0,
        vec![
            flow_rect("a", "A", 100.0, 40.0),
            flow_rect("b", "B", 100.0, 40.0),
            flow_rect("c", "C", 100.0, 40.0),
        ],
    );
    let mut s = state_with(vec![f]);
    s.set_single_selection(NodeId::new("b"));

    assert!(s.move_selected_in_layout_direction(0.0, 1.0));

    let parent = find_node(s.active_children(), &NodeId::new("f1")).unwrap();
    let order: Vec<&str> = parent
        .children()
        .unwrap()
        .iter()
        .map(|c| c.id_str())
        .collect();
    assert_eq!(order, vec!["a", "c", "b"]);
    assert_eq!(s.selection.anchor, NodeId::new("b"));
}

#[test]
fn layout_arrow_left_moves_selection_backward_in_horizontal_container() {
    use jian_ops_schema::node::container::LayoutMode;

    let mut f = flex_frame(
        "f1",
        "Row",
        0.0,
        0.0,
        300.0,
        100.0,
        vec![
            flow_rect("a", "A", 80.0, 40.0),
            flow_rect("b", "B", 80.0, 40.0),
            flow_rect("c", "C", 80.0, 40.0),
        ],
    );
    if let PenNode::Frame(frame) = &mut f {
        frame.container.layout = Some(LayoutMode::Horizontal);
    }
    let mut s = state_with(vec![f]);
    s.set_single_selection(NodeId::new("b"));

    assert!(s.move_selected_in_layout_direction(-1.0, 0.0));

    let parent = find_node(s.active_children(), &NodeId::new("f1")).unwrap();
    let order: Vec<&str> = parent
        .children()
        .unwrap()
        .iter()
        .map(|c| c.id_str())
        .collect();
    assert_eq!(order, vec!["b", "a", "c"]);
    assert_eq!(s.selection.anchor, NodeId::new("b"));
}

#[test]
fn reparent_to_page_root_preserves_visual_position() {
    let mut s = nested_state();
    assert!(s.reparent_to_page_root(&NodeId::new("r1"), 400.0, 50.0));
    // Lands as the FIRST top-level child (TS `moveNode(id, null, 0)`).
    assert_eq!(s.active_children()[0].id_str(), "r1");
    let n = find_node(s.active_children(), &NodeId::new("r1")).unwrap();
    assert_eq!(n.base().x, Some(400.0));
    assert_eq!(n.base().y, Some(50.0));
    // The old parent no longer carries it.
    let g1 = find_node(s.active_children(), &NodeId::new("g1")).unwrap();
    assert!(g1.children().unwrap().is_empty());
}

#[test]
fn reparent_to_page_root_is_noop_for_top_level_nodes() {
    let mut s = nested_state();
    assert!(!s.reparent_to_page_root(&NodeId::new("n9"), 0.0, 0.0));
}

#[test]
fn reparent_policy_allows_shape_container_and_content_nodes() {
    let r = rect("r", "R", 0.0, 0.0, 10.0, 10.0);
    let t = text("t", "T", 0.0, 0.0, 10.0, 10.0, "hi");
    let f = frame("f", "F", 0.0, 0.0, 10.0, 10.0, vec![]);
    let g = group("g", "G", vec![]);
    assert!(should_auto_reparent_outside_parent(&r));
    assert!(should_auto_reparent_outside_parent(&f));
    assert!(should_auto_reparent_outside_parent(&g));
    assert!(should_auto_reparent_outside_parent(&t));
}

#[test]
fn drop_into_free_container_preserves_visual_position_as_relative_xy() {
    let src_child = rect("box", "Box", 20.0, 30.0, 50.0, 40.0);
    let src = frame("src", "Source", 100.0, 100.0, 200.0, 200.0, vec![src_child]);
    let target = frame("target", "Target", 400.0, 200.0, 240.0, 180.0, vec![]);
    let mut s = state_with(vec![src, target]);

    assert!(s.move_node_to_drop_target(
        &NodeId::new("box"),
        DragDropTarget::Container {
            parent_id: NodeId::new("target"),
            parent_abs_x: 400.0,
            parent_abs_y: 200.0,
            index: 0,
        },
        450.0,
        260.0,
        50.0,
        40.0,
    ));

    let src = find_node(s.active_children(), &NodeId::new("src")).unwrap();
    assert!(src.children().unwrap().is_empty());
    let target = find_node(s.active_children(), &NodeId::new("target")).unwrap();
    let moved = target
        .children()
        .unwrap()
        .iter()
        .find(|node| node.id_str() == "box")
        .expect("box moved into target");
    assert_eq!(moved.base().x, Some(50.0));
    assert_eq!(moved.base().y, Some(60.0));
}

#[test]
fn drop_to_page_root_makes_nested_node_a_root_at_absolute_position() {
    let src_child = rect("box", "Box", 20.0, 30.0, 50.0, 40.0);
    let src = frame("src", "Source", 100.0, 100.0, 200.0, 200.0, vec![src_child]);
    let target = frame("target", "Target", 400.0, 200.0, 240.0, 180.0, vec![]);
    let mut s = state_with(vec![src, target]);

    assert!(s.move_node_to_drop_target(
        &NodeId::new("box"),
        DragDropTarget::PageRoot { index: 1 },
        700.0,
        80.0,
        50.0,
        40.0,
    ));

    let ids: Vec<&str> = s
        .active_children()
        .iter()
        .map(|node| node.id_str())
        .collect();
    assert_eq!(ids, vec!["src", "box", "target"]);
    let moved = find_node(s.active_children(), &NodeId::new("box")).unwrap();
    assert_eq!(moved.base().x, Some(700.0));
    assert_eq!(moved.base().y, Some(80.0));
}

#[test]
fn drop_root_into_free_container_preserves_visual_position() {
    let root = rect("box", "Box", 100.0, 120.0, 50.0, 40.0);
    let target = frame("target", "Target", 400.0, 200.0, 240.0, 180.0, vec![]);
    let mut s = state_with(vec![root, target]);

    assert!(s.move_node_to_drop_target(
        &NodeId::new("box"),
        DragDropTarget::Container {
            parent_id: NodeId::new("target"),
            parent_abs_x: 400.0,
            parent_abs_y: 200.0,
            index: 0,
        },
        440.0,
        230.0,
        50.0,
        40.0,
    ));

    let root_ids: Vec<&str> = s
        .active_children()
        .iter()
        .map(|node| node.id_str())
        .collect();
    assert_eq!(root_ids, vec!["target"]);
    let target = find_node(s.active_children(), &NodeId::new("target")).unwrap();
    let moved = &target.children().unwrap()[0];
    assert_eq!(moved.id_str(), "box");
    assert_eq!(moved.base().x, Some(40.0));
    assert_eq!(moved.base().y, Some(30.0));
}

#[test]
fn drop_root_into_flex_container_clears_xy_and_inserts_at_index() {
    let root = rect("box", "Box", 100.0, 120.0, 50.0, 40.0);
    let target = flex_frame(
        "stack",
        "Stack",
        400.0,
        200.0,
        240.0,
        180.0,
        vec![
            flow_rect("a", "A", 100.0, 40.0),
            flow_rect("b", "B", 100.0, 40.0),
        ],
    );
    let mut s = state_with(vec![root, target]);

    assert!(s.move_node_to_drop_target(
        &NodeId::new("box"),
        DragDropTarget::Container {
            parent_id: NodeId::new("stack"),
            parent_abs_x: 400.0,
            parent_abs_y: 200.0,
            index: 1,
        },
        430.0,
        245.0,
        50.0,
        40.0,
    ));

    let stack = find_node(s.active_children(), &NodeId::new("stack")).unwrap();
    let children = stack.children().unwrap();
    let ids: Vec<&str> = children.iter().map(|node| node.id_str()).collect();
    assert_eq!(ids, vec!["a", "box", "b"]);
    let moved = children.iter().find(|node| node.id_str() == "box").unwrap();
    assert_eq!(moved.base().x, None);
    assert_eq!(moved.base().y, None);
}

#[test]
fn drop_into_own_descendant_is_rejected_without_detaching() {
    let inner = frame("inner", "Inner", 20.0, 20.0, 100.0, 100.0, vec![]);
    let root = frame("root", "Root", 100.0, 100.0, 200.0, 200.0, vec![inner]);
    let mut s = state_with(vec![root]);

    assert!(!s.move_node_to_drop_target(
        &NodeId::new("root"),
        DragDropTarget::Container {
            parent_id: NodeId::new("inner"),
            parent_abs_x: 120.0,
            parent_abs_y: 120.0,
            index: 0,
        },
        130.0,
        130.0,
        200.0,
        200.0,
    ));

    assert_eq!(s.active_children().len(), 1);
    assert!(find_node(s.active_children(), &NodeId::new("root")).is_some());
    assert!(find_node(s.active_children(), &NodeId::new("inner")).is_some());
}

#[test]
fn parent_of_and_direction_helpers_walk_the_tree() {
    let s = nested_state();
    assert_eq!(
        parent_of(s.active_children(), &NodeId::new("r1")),
        Some(NodeId::new("g1"))
    );
    assert_eq!(parent_of(s.active_children(), &NodeId::new("f1")), None);
    let flex = flex_frame("fx", "Stack", 0.0, 0.0, 100.0, 100.0, vec![]);
    assert_eq!(auto_layout_direction(&flex), Some(FlexDirection::Vertical));
    let free = frame("fr", "Free", 0.0, 0.0, 100.0, 100.0, vec![]);
    assert_eq!(auto_layout_direction(&free), None);
}

// --- Container resize scales descendants (GAP C) -----------------------

#[test]
fn container_resize_scales_free_descendants_recursively() {
    let leaf = rect("leaf", "Leaf", 5.0, 5.0, 10.0, 10.0);
    let inner = frame("inner", "Inner", 20.0, 40.0, 100.0, 50.0, vec![leaf]);
    let root = frame("root", "Root", 100.0, 100.0, 200.0, 100.0, vec![inner]);
    let mut s = state_with(vec![root]);
    s.set_single_selection(NodeId::new("root"));
    // 200×100 → 400×300: sx = 2, sy = 3.
    s.set_selected_bounds(DocRect {
        x: 100.0,
        y: 100.0,
        w: 400.0,
        h: 300.0,
    });
    let inner = find_node(s.active_children(), &NodeId::new("inner")).unwrap();
    assert_eq!(inner.base().x, Some(40.0));
    assert_eq!(inner.base().y, Some(120.0));
    assert_eq!(inner.width_px(), Some(200.0));
    assert_eq!(inner.height_px(), Some(150.0));
    let leaf = find_node(s.active_children(), &NodeId::new("leaf")).unwrap();
    assert_eq!(leaf.base().x, Some(10.0));
    assert_eq!(leaf.base().y, Some(15.0));
    assert_eq!(leaf.width_px(), Some(20.0));
    assert_eq!(leaf.height_px(), Some(30.0));
}

#[test]
fn container_resize_scales_flex_children_sizes_only() {
    let f = flex_frame(
        "f1",
        "Stack",
        0.0,
        0.0,
        200.0,
        100.0,
        vec![flow_rect("a", "A", 100.0, 40.0)],
    );
    let mut s = state_with(vec![f]);
    s.set_single_selection(NodeId::new("f1"));
    s.set_selected_bounds(DocRect {
        x: 0.0,
        y: 0.0,
        w: 400.0,
        h: 200.0,
    });
    let a = find_node(s.active_children(), &NodeId::new("a")).unwrap();
    // Flow children keep layout-engine positions (no materialized x/y)…
    assert_eq!(a.base().x, None);
    assert_eq!(a.base().y, None);
    // …but their explicit sizes scale with the container.
    assert_eq!(a.width_px(), Some(200.0));
    assert_eq!(a.height_px(), Some(80.0));
}

#[test]
fn incremental_resize_steps_compose_to_the_total_scale() {
    let leaf = rect("leaf", "Leaf", 10.0, 10.0, 10.0, 10.0);
    let root = frame("root", "Root", 0.0, 0.0, 100.0, 100.0, vec![leaf]);
    let mut s = state_with(vec![root]);
    s.set_single_selection(NodeId::new("root"));
    // Two drag steps: 100 → 150 → 200 must equal one 100 → 200 step.
    for w in [150.0, 200.0] {
        s.set_selected_bounds(DocRect {
            x: 0.0,
            y: 0.0,
            w,
            h: 100.0,
        });
    }
    let leaf = find_node(s.active_children(), &NodeId::new("leaf")).unwrap();
    assert_eq!(leaf.base().x, Some(20.0));
    assert_eq!(leaf.width_px(), Some(20.0));
    assert_eq!(leaf.base().y, Some(10.0));
    assert_eq!(leaf.height_px(), Some(10.0));
}

#[test]
fn resizing_auto_grow_text_pins_fixed_width_growth() {
    let t = text("t1", "Title", 0.0, 0.0, 100.0, 20.0, "Hello");
    let mut s = state_with(vec![t]);
    s.set_single_selection(NodeId::new("t1"));
    s.set_selected_bounds(DocRect {
        x: 0.0,
        y: 0.0,
        w: 160.0,
        h: 20.0,
    });
    let n = find_node(s.active_children(), &NodeId::new("t1")).unwrap();
    match n {
        PenNode::Text(t) => assert_eq!(t.text_growth, Some(TextGrowth::FixedWidth)),
        other => panic!("expected text node, got {other:?}"),
    }
}

#[test]
fn resizing_text_with_explicit_growth_keeps_it() {
    let mut t = text("t1", "Title", 0.0, 0.0, 100.0, 20.0, "Hello");
    if let PenNode::Text(tn) = &mut t {
        tn.text_growth = Some(TextGrowth::Auto);
    }
    let mut s = state_with(vec![t]);
    s.set_single_selection(NodeId::new("t1"));
    s.set_selected_bounds(DocRect {
        x: 0.0,
        y: 0.0,
        w: 160.0,
        h: 20.0,
    });
    let n = find_node(s.active_children(), &NodeId::new("t1")).unwrap();
    match n {
        PenNode::Text(t) => assert_eq!(t.text_growth, Some(TextGrowth::Auto)),
        other => panic!("expected text node, got {other:?}"),
    }
}
