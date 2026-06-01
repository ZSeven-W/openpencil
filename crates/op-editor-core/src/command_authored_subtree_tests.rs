//! `EditorCommand::InsertAuthoredSubtree` tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{frame, rect, state_with};

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
