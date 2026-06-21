//! `EditorCommand::InsertAuthoredSubtree` tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{frame, rect, state_with};

#[test]
fn insert_authored_subtree_rejects_empty_authored_id() {
    // Authored subtrees require a valid (non-empty) id on every node — the
    // ONLY accept/reject difference vs `cmd_insert_subtree` (which remaps and
    // so tolerates an empty id) besides id collisions. Callers that mint ids
    // (batch_design) never hit this; a malformed empty id is rejected.
    let mut s = state_with(vec![]);
    let mut node = rect("x", "X", 0.0, 0.0, 10.0, 10.0);
    node.base_mut().id = String::new();

    assert!(!s.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    assert!(s.active_children().is_empty());
}

#[test]
fn insert_authored_subtree_preserves_ids_for_layered_workflow() {
    let mut s = state_with(vec![]);

    assert!(s.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![frame(
            "root",
            "Root",
            0.0,
            0.0,
            375.0,
            812.0,
            vec![rect("hero", "Hero", 0.0, 0.0, 375.0, 240.0)],
        )],
        parent_id: NodeId::NONE,
        page_id: None,
    }));

    assert_eq!(s.active_children()[0].id_str(), "root");
    let section = &s.active_children()[0].children().expect("children")[0];
    assert_eq!(section.id_str(), "hero");
}

#[test]
fn insert_authored_root_frame_replaces_empty_root_frame() {
    let mut s = state_with(vec![frame(
        "default",
        "Frame",
        30.0,
        40.0,
        100.0,
        100.0,
        vec![],
    )]);

    assert!(s.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![frame(
            "food-home",
            "Food App Home",
            0.0,
            0.0,
            402.0,
            874.0,
            vec![rect("hero", "Hero", 0.0, 0.0, 402.0, 120.0)],
        )],
        parent_id: NodeId::NONE,
        page_id: None,
    }));

    let children = s.active_children();
    assert_eq!(children.len(), 1, "empty default frame should be replaced");
    assert_eq!(children[0].id_str(), "food-home");
    assert_eq!(children[0].base().x, Some(30.0));
    assert_eq!(children[0].base().y, Some(40.0));
}

#[test]
fn insert_authored_subtree_accepts_empty_container_parent() {
    // Regression: a container whose `children` is still `None` (here a `rect`,
    // which is a container) must accept an insert — `cmd_insert_subtree` does,
    // and the old `children().is_some()` check wrongly rejected it.
    let mut s = state_with(vec![rect("box", "Box", 0.0, 0.0, 100.0, 100.0)]);

    assert!(s.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![rect("child", "Child", 0.0, 0.0, 10.0, 10.0)],
        parent_id: NodeId::new("box"),
        page_id: None,
    }));

    let parent = &s.active_children()[0];
    assert_eq!(parent.id_str(), "box");
    let inserted = &parent.children().expect("children initialized")[0];
    assert_eq!(inserted.id_str(), "child");
}

#[test]
fn insert_authored_subtree_rejects_live_id_collision() {
    let mut s = state_with(vec![rect("root", "Existing", 0.0, 0.0, 10.0, 10.0)]);

    assert!(!s.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![rect("root", "Duplicate", 0.0, 0.0, 10.0, 10.0)],
        parent_id: NodeId::NONE,
        page_id: None,
    }));

    assert_eq!(s.active_children().len(), 1);
    assert_eq!(
        s.active_children()[0].base().name.as_deref(),
        Some("Existing")
    );
}
