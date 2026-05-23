//! `EditorCommand::InsertSubtree` tests — split from `command_tests.rs`
//! to keep that file under the 800-line cap.
//!
//! `InsertSubtree` carries fully-nested canonical `PenNode` subtrees
//! (frame + children + layout), unlike the flat-leaf `InsertNode` /
//! `BatchInsert`. Every incoming node id is remapped to a fresh editor
//! id, so an externally-authored subtree can't collide with live doc
//! ids.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::command_node::remap_subtree_ids;
use crate::node_id::NodeId;
use crate::pen_node_ext::{make_group, make_path, PenNodeExt};
use crate::test_support::state_with;
use std::collections::HashSet;

// --- remap_subtree_ids ----------------------------------------------

#[test]
fn remap_subtree_ids_reassigns_every_node_recursively() {
    // Foreign subtree: Group("xx") wrapping Path("xx") — ids collide
    // with each other and are unrelated to any document.
    let mut nodes = vec![make_group(
        "xx".into(),
        "card",
        vec![make_path("xx".into(), "line", (0.0, 0.0))],
    )];
    let mut next_id = 1u64;
    let mut taken: HashSet<NodeId> = HashSet::new();

    assert!(remap_subtree_ids(&mut nodes, &mut next_id, &mut taken));

    let group_id = nodes[0].id_str().to_string();
    let child_id = nodes[0].children().unwrap()[0].id_str().to_string();
    // Both ids became fresh, non-empty, distinct values.
    assert_ne!(group_id, "xx");
    assert_ne!(child_id, "xx");
    assert_ne!(group_id, child_id);
    assert!(!group_id.is_empty() && !child_id.is_empty());
}

// --- InsertSubtree: root insertion ----------------------------------

#[test]
fn insert_subtree_nests_children_under_root() {
    let mut s = state_with(vec![]);
    let subtree = make_group(
        "ext-1".into(),
        "card",
        vec![make_path("ext-2".into(), "line", (0.0, 0.0))],
    );
    assert!(s.apply(EditorCommand::InsertSubtree {
        nodes: vec![subtree],
        parent_id: NodeId::NONE,
    }));

    // The root gained one group; the group keeps its one child.
    assert_eq!(s.active_children().len(), 1);
    let g = &s.active_children()[0];
    assert!(g.is_group());
    assert_eq!(g.children().unwrap().len(), 1);
    // Ids were remapped (no longer the foreign "ext-*") and unique.
    assert_ne!(g.id_str(), "ext-1");
    assert!(s.find_duplicate_id().is_none());
}

#[test]
fn insert_subtree_rejects_empty() {
    let mut s = state_with(vec![]);
    assert!(!s.apply(EditorCommand::InsertSubtree {
        nodes: vec![],
        parent_id: NodeId::NONE,
    }));
    assert_eq!(s.active_children().len(), 0);
}

#[test]
fn insert_subtree_is_undoable() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::InsertSubtree {
        nodes: vec![make_group("ext-1".into(), "card", vec![])],
        parent_id: NodeId::NONE,
    }));
    assert_eq!(s.active_children().len(), 1);

    // The whole insert is a single undo step.
    assert!(s.apply(EditorCommand::Undo));
    assert_eq!(s.active_children().len(), 0);
}

// --- InsertSubtree: parent validation -------------------------------

#[test]
fn insert_subtree_under_parent_container() {
    // The document already holds an empty Group as the parent.
    let mut s = state_with(vec![make_group("parent".into(), "wrap", vec![])]);
    assert!(s.apply(EditorCommand::InsertSubtree {
        nodes: vec![make_group("ext-1".into(), "card", vec![])],
        parent_id: NodeId::new("parent"),
    }));
    // The subtree nested into `parent`, not the page root.
    assert_eq!(s.active_children().len(), 1);
    let parent = &s.active_children()[0];
    assert_eq!(parent.children().unwrap().len(), 1);
}

#[test]
fn insert_subtree_rejects_missing_parent() {
    let mut s = state_with(vec![]);
    assert!(!s.apply(EditorCommand::InsertSubtree {
        nodes: vec![make_group("ext-1".into(), "card", vec![])],
        parent_id: NodeId::new("nope"),
    }));
    assert_eq!(s.active_children().len(), 0);
}

#[test]
fn insert_subtree_rejects_non_container_parent() {
    // A Path is not a container.
    let mut s = state_with(vec![make_path("leaf".into(), "line", (0.0, 0.0))]);
    assert!(!s.apply(EditorCommand::InsertSubtree {
        nodes: vec![make_group("ext-1".into(), "card", vec![])],
        parent_id: NodeId::new("leaf"),
    }));
    // Document unchanged: still just that path.
    assert_eq!(s.active_children().len(), 1);
}

// --- InsertSubtree: id-collision remap ------------------------------

#[test]
fn insert_subtree_remaps_ids_colliding_with_live_doc() {
    // The document already has a node with id "n1".
    let mut s = state_with(vec![make_path("n1".into(), "existing", (0.0, 0.0))]);
    // The foreign subtree deliberately also uses "n1".
    assert!(s.apply(EditorCommand::InsertSubtree {
        nodes: vec![make_group("n1".into(), "card", vec![])],
        parent_id: NodeId::NONE,
    }));
    // Insert succeeded and the whole document has no duplicate ids.
    assert_eq!(s.active_children().len(), 2);
    assert!(s.find_duplicate_id().is_none());
    // At most one node still carries the literal id "n1".
    let n1_count = s
        .active_children()
        .iter()
        .filter(|n| n.id_str() == "n1")
        .count();
    assert!(n1_count <= 1);
}
