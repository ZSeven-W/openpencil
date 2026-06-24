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
use crate::test_support::{frame, rect, state_with};
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
        page_id: None,
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
fn insert_subtree_root_frame_replaces_empty_root_frame() {
    let mut s = state_with(vec![frame(
        "default",
        "Frame",
        30.0,
        40.0,
        100.0,
        100.0,
        vec![],
    )]);

    assert!(s.apply(EditorCommand::InsertSubtree {
        nodes: vec![frame(
            "ext-root",
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
    assert_ne!(children[0].id_str(), "default");
    assert_eq!(children[0].base().name.as_deref(), Some("Food App Home"));
    assert_eq!(children[0].base().x, Some(30.0));
    assert_eq!(children[0].base().y, Some(40.0));
}

#[test]
fn insert_subtree_rejects_empty() {
    let mut s = state_with(vec![]);
    assert!(!s.apply(EditorCommand::InsertSubtree {
        nodes: vec![],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    assert_eq!(s.active_children().len(), 0);
}

#[test]
fn insert_subtree_is_undoable() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::InsertSubtree {
        nodes: vec![make_group("ext-1".into(), "card", vec![])],
        parent_id: NodeId::NONE,
        page_id: None,
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
        page_id: None,
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
        page_id: None,
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
        page_id: None,
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
        page_id: None,
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

// --- insert_subtree_returning_root_ids ---------------------------------

/// This test exercises the DFS-ordering correctness of
/// `insert_subtree_returning_root_ids`. The buggy implementation sliced
/// `mapping[0..root_count]` from `remap_subtree_ids_mapping`, which allocates
/// in DFS order (root0, child0a, child0b, root1, ...). For a forest where
/// root0 has children, the slice [root0, child0a, ...] contains child ids,
/// not [root0, root1, ...]. The fix reads root ids directly from the
/// top-level nodes after in-place remap.
///
/// Specifically, this test ensures:
/// - The returned Vec has exactly `root_count` entries (not more).
/// - Every returned id matches a TOP-LEVEL doc node (not a child).
/// - No returned id is a child's id.
/// - All incoming placeholder ids are remapped (forced by collision).
#[test]
fn insert_subtree_returning_root_ids_yields_post_remap_roots() {
    let mut s = state_with(vec![]);
    // Pre-insert one node so the allocator has an occupied id slot and any
    // trivial "first id = same" coincidence is ruled out.
    assert!(s.apply(EditorCommand::InsertSubtree {
        nodes: vec![make_group("a".into(), "Existing", vec![])],
        parent_id: NodeId::NONE,
        page_id: None,
    }));
    // Record the id that was assigned so we can use a colliding incoming id.
    let existing_id = s.active_children()[0].id_str().to_string();

    let before: std::collections::HashSet<String> = s
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();

    // Insert a 2-root forest where:
    //   root0 ("r1") has a CHILD ("child-a") — this is the key DFS trap.
    //   root1 ("r2") has no children.
    // root0's incoming id deliberately matches the existing doc id to force
    // a remap (ensures the returned id != "r1" and != existing_id).
    let child_placeholder = "child-a".to_string();
    let roots = s
        .insert_subtree_returning_root_ids(
            vec![
                make_group(
                    existing_id.clone(), // collide with live doc id → forces remap
                    "Root0WithChild",
                    vec![make_path(
                        child_placeholder.clone(),
                        "PathChild",
                        (0.0, 0.0),
                    )],
                ),
                make_group("r2".into(), "Root1NoChild", vec![]),
            ],
            &NodeId::NONE,
        )
        .expect("accepted");

    // Exactly 2 root ids returned (not 3 = root0 + child + root1).
    assert_eq!(
        roots.len(),
        2,
        "must return exactly root_count=2 ids, not child ids"
    );

    // Neither root id is the placeholder.
    assert!(
        roots.iter().all(|id| id != &existing_id && id != "r2"),
        "returned ids must be remapped, got {roots:?}"
    );

    // Collect what is actually in the top-level doc now.
    let after_top: Vec<String> = s
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();

    // Collect child ids of inserted nodes.
    let child_ids: Vec<String> = s
        .active_children()
        .iter()
        .flat_map(|n| {
            n.children()
                .into_iter()
                .flatten()
                .map(|c| c.id_str().to_string())
        })
        .collect();

    for id in &roots {
        // Each returned id must be a top-level node.
        assert!(
            after_top.contains(id),
            "root id {id} missing from doc top-level"
        );
        // Each returned id must NOT be a child id (the DFS bug would return child ids).
        assert!(
            !child_ids.contains(id),
            "root id {id} is a child id, not a root — DFS bug!"
        );
        // Each returned id must be new (not present before insert).
        assert!(
            !before.contains(id),
            "root id {id} was already in doc before insert"
        );
    }

    // child-a's placeholder must not appear in roots (DFS bug symptom).
    assert!(
        !roots.contains(&child_placeholder),
        "child placeholder {child_placeholder} must not appear in returned root ids"
    );

    // No duplicate ids anywhere.
    assert!(s.find_duplicate_id().is_none());
}

#[test]
fn insert_subtree_returning_root_ids_returns_none_on_reject() {
    // Rejected insert (empty nodes list) returns None and leaves doc unchanged.
    let mut s = state_with(vec![make_group("existing".into(), "E", vec![])]);
    let result = s.insert_subtree_returning_root_ids(vec![], &NodeId::NONE);
    assert!(result.is_none());
    // Doc is unmodified.
    assert_eq!(s.active_children().len(), 1);
    assert_eq!(s.active_children()[0].id_str(), "existing");
}

#[test]
fn insert_subtree_returning_root_ids_rejects_missing_parent() {
    let mut s = state_with(vec![]);
    let result = s.insert_subtree_returning_root_ids(
        vec![make_group("x".into(), "X", vec![])],
        &NodeId::new("ghost"),
    );
    assert!(result.is_none());
    assert_eq!(s.active_children().len(), 0);
}
