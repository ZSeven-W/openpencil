//! Tree-op mutator tests — delete / duplicate / reorder / group / ungroup.

use super::support::{root_ids, three_rects};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{ellipse, frame, group, rect, sample, state_with};
use crate::walkers::{find_node, ReorderDirection};

#[test]
fn delete_selected_removes_top_level_node_and_clears_selection() {
    let mut s = sample();
    s.set_single_selection(NodeId::new("n10"));
    assert!(find_node(s.active_children(), &NodeId::new("n10")).is_some());
    assert!(s.delete_selected());
    assert_eq!(s.selection.anchor, NodeId::NONE);
    assert!(find_node(s.active_children(), &NodeId::new("n10")).is_none());
}

#[test]
fn delete_selected_removes_nested_node() {
    let mut s = sample();
    s.set_single_selection(NodeId::new("n13"));
    assert!(s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n13")).is_none());
    assert!(find_node(s.active_children(), &NodeId::new("n10")).is_some());
}

#[test]
fn delete_selected_returns_false_when_unselected() {
    let mut s = sample();
    s.clear_selection();
    assert!(!s.delete_selected());
}

#[test]
fn delete_selected_removes_every_node_in_the_set() {
    let mut s = state_with(vec![
        rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "B", 20.0, 0.0, 10.0, 10.0),
        rect("n3", "C", 40.0, 0.0, 10.0, 10.0),
    ]);
    s.clear_selection();
    s.toggle_selection(NodeId::new("n1"));
    s.toggle_selection(NodeId::new("n2"));
    assert_eq!(s.selection_count(), 2);
    assert!(s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n1")).is_none());
    assert!(find_node(s.active_children(), &NodeId::new("n2")).is_none());
    assert!(find_node(s.active_children(), &NodeId::new("n3")).is_some());
    assert_eq!(s.active_children().len(), 1);
    assert!(s.selection.is_empty());
}

#[test]
fn delete_selected_removes_ancestor_of_hidden_descendant() {
    // HTML imports map `visibility: hidden` elements to `visible: false`
    // nodes; a hidden descendant must not make the imported frame
    // undeletable.
    let mut child = rect("n63", "child", 0.0, 0.0, 10.0, 10.0);
    child.base_mut().visible = Some(false);
    let parent = frame("n62", "parent", 0.0, 0.0, 50.0, 50.0, vec![child]);
    let mut s = state_with(vec![parent]);
    s.set_single_selection(NodeId::new("n62"));
    assert!(s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n62")).is_none());
}

#[test]
fn delete_selected_removes_hidden_root() {
    // A hidden layer selected in the layer panel must still delete —
    // visibility is a render state, not a protection state. This also
    // covers HTML imports whose whole body computed `display: none`.
    let mut node = rect("n64", "hidden", 0.0, 0.0, 10.0, 10.0);
    node.base_mut().visible = Some(false);
    let mut s = state_with(vec![node]);
    s.set_single_selection(NodeId::new("n64"));
    assert!(s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n64")).is_none());
}

#[test]
fn delete_selected_protects_locked_root() {
    let mut node = rect("n65", "locked", 0.0, 0.0, 10.0, 10.0);
    node.base_mut().locked = Some(true);
    let mut s = state_with(vec![node]);
    s.set_single_selection(NodeId::new("n65"));
    assert!(!s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n65")).is_some());
}

#[test]
fn delete_selected_protects_ancestor_of_locked_descendant() {
    let mut child = rect("n61", "child", 0.0, 0.0, 10.0, 10.0);
    child.base_mut().locked = Some(true);
    let parent = frame("n60", "parent", 0.0, 0.0, 50.0, 50.0, vec![child]);
    let mut s = state_with(vec![parent]);
    s.set_single_selection(NodeId::new("n60"));
    // Subtree contains a locked node → delete refused.
    assert!(!s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n60")).is_some());
}

// --- Duplicate -------------------------------------------------------

#[test]
fn duplicate_selected_clones_subtree_with_fresh_ids_and_selects_it() {
    let mut s = sample();
    s.set_single_selection(NodeId::new("n10"));
    let mut next_id = 1_000u64;
    let clone_id = s
        .duplicate_selected(&mut next_id, 10.0)
        .expect("duplicate should return new id");
    assert!(clone_id.is_real());
    assert_eq!(s.selection.anchor, clone_id);
    // Original survives.
    assert!(find_node(s.active_children(), &NodeId::new("n10")).is_some());
    // Clone present with fresh id.
    let original = find_node(s.active_children(), &NodeId::new("n10")).unwrap();
    let clone = find_node(s.active_children(), &clone_id).unwrap();
    // Clone offset by 10 px on both axes.
    assert!((clone.base().x.unwrap() - original.base().x.unwrap() - 10.0).abs() < 1e-3);
    assert!((clone.base().y.unwrap() - original.base().y.unwrap() - 10.0).abs() < 1e-3);
    // Descendant count preserved.
    assert_eq!(
        clone.children().map(|c| c.len()),
        original.children().map(|c| c.len())
    );
    assert!(s.validate().is_ok());
}

#[test]
fn duplicate_selected_returns_none_when_unselected() {
    let mut s = sample();
    s.clear_selection();
    let mut next_id = 1u64;
    assert!(s.duplicate_selected(&mut next_id, 10.0).is_none());
}

// --- Reorder ---------------------------------------------------------

#[test]
fn reorder_selected_up_moves_toward_front_index() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n2"));
    assert!(s.reorder_selected(ReorderDirection::Up));
    assert_eq!(root_ids(&s), vec!["n2", "n1", "n3"]);
}

#[test]
fn reorder_selected_down_moves_toward_back_index() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n2"));
    assert!(s.reorder_selected(ReorderDirection::Down));
    assert_eq!(root_ids(&s), vec!["n1", "n3", "n2"]);
}

#[test]
fn reorder_selected_at_edges_is_noop() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n1"));
    assert!(!s.reorder_selected(ReorderDirection::Up));
    s.set_single_selection(NodeId::new("n3"));
    assert!(!s.reorder_selected(ReorderDirection::Down));
}

#[test]
fn reorder_before_moves_source_to_anchor_position() {
    let mut s = three_rects();
    assert!(s.reorder_before(NodeId::new("n3"), NodeId::new("n1")));
    assert_eq!(root_ids(&s), vec!["n3", "n1", "n2"]);
}

#[test]
fn reorder_after_moves_source_after_anchor() {
    let mut s = three_rects();
    assert!(s.reorder_after(NodeId::new("n1"), NodeId::new("n2")));
    assert_eq!(root_ids(&s), vec!["n2", "n1", "n3"]);
}

#[test]
fn reorder_into_reparents_under_container() {
    let mut s = state_with(vec![
        frame("n1", "Frame", 0.0, 0.0, 100.0, 100.0, vec![]),
        rect("n2", "Loose", 0.0, 0.0, 10.0, 10.0),
    ]);
    assert!(s.reorder_into(NodeId::new("n2"), NodeId::new("n1")));
    assert_eq!(root_ids(&s), vec!["n1"]);
    let parent = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert_eq!(parent.children().unwrap().len(), 1);
}

#[test]
fn reorder_into_inserts_at_front() {
    let mut s = state_with(vec![
        frame(
            "n1",
            "Frame",
            0.0,
            0.0,
            100.0,
            100.0,
            vec![rect("c0", "Existing", 0.0, 0.0, 10.0, 10.0)],
        ),
        rect("n2", "Loose", 0.0, 0.0, 10.0, 10.0),
    ]);
    assert!(s.reorder_into(NodeId::new("n2"), NodeId::new("n1")));
    let parent = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    let ids: Vec<&str> = parent
        .children()
        .unwrap()
        .iter()
        .map(|n| n.id_str())
        .collect();
    // TS parity (layer-dnd-utils.ts): dropped node lands at index 0.
    assert_eq!(ids, vec!["n2", "c0"]);
}

#[test]
fn reorder_into_rejects_non_container_target() {
    let mut s = state_with(vec![
        ellipse("e1", "Circle", 0.0, 0.0, 50.0, 50.0),
        rect("n2", "Loose", 0.0, 0.0, 10.0, 10.0),
    ]);
    // An ellipse is not a container — the move must be refused and the source
    // must survive at the root (no extract-before-verify silent drop).
    assert!(!s.reorder_into(NodeId::new("n2"), NodeId::new("e1")));
    assert_eq!(root_ids(&s), vec!["e1", "n2"]);
}

#[test]
fn reorder_into_rejects_cycle() {
    let mut s = state_with(vec![frame(
        "n1",
        "Frame",
        0.0,
        0.0,
        100.0,
        100.0,
        vec![rect("n2", "Child", 0.0, 0.0, 10.0, 10.0)],
    )]);
    // Can't move the parent under its own child.
    assert!(!s.reorder_into(NodeId::new("n1"), NodeId::new("n2")));
}

// --- Grouping --------------------------------------------------------

#[test]
fn group_selected_wraps_siblings_in_a_group() {
    let mut s = three_rects();
    s.clear_selection();
    s.toggle_selection(NodeId::new("n1"));
    s.toggle_selection(NodeId::new("n2"));
    let mut next_id = 1u64;
    let group_id = s.group_selected(&mut next_id).expect("group");
    // Group replaces n1 + n2 at position 0; n3 stays.
    assert_eq!(root_ids(&s), vec![group_id.as_str(), "n3"]);
    let g = find_node(s.active_children(), &group_id).unwrap();
    assert!(g.is_group());
    assert_eq!(g.children().unwrap().len(), 2);
    assert_eq!(s.selection.anchor, group_id);
}

#[test]
fn group_selected_empty_is_none() {
    let mut s = three_rects();
    s.clear_selection();
    let mut next_id = 1u64;
    assert!(s.group_selected(&mut next_id).is_none());
}

#[test]
fn ungroup_selected_splices_children_inline() {
    let mut s = state_with(vec![
        group(
            "n9",
            "G",
            vec![
                rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
                rect("n2", "B", 0.0, 0.0, 10.0, 10.0),
            ],
        ),
        rect("n3", "C", 0.0, 0.0, 10.0, 10.0),
    ]);
    s.set_single_selection(NodeId::new("n9"));
    assert!(s.ungroup_selected());
    assert_eq!(root_ids(&s), vec!["n1", "n2", "n3"]);
    assert_eq!(s.selection.anchor, NodeId::new("n2"));
}

#[test]
fn ungroup_selected_rejects_non_group() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n1"));
    assert!(!s.ungroup_selected());
}

// --- History ---------------------------------------------------------
