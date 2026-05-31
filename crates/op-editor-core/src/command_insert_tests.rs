//! `EditorCommand::InsertNode` parent/page targeting tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{frame, state_with};
use jian_ops_schema::page::PenPage;

#[test]
fn insert_node_appends_a_fresh_leaf() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Box".into(),
        x: 10,
        y: 20,
        width: 100,
        height: 60,
        fill_hex: Some("#ff0000".into()),
        target_parent: NodeId::NONE,
        page_id: None,
    }));
    assert_eq!(s.active_children().len(), 1);
    let node = &s.active_children()[0];
    assert_eq!(node.base().name.as_deref(), Some("Box"));
    assert_eq!(node.base().x, Some(10.0));
}

#[test]
fn insert_node_rejects_unknown_kind() {
    let mut s = state_with(vec![]);
    assert!(!s.apply(EditorCommand::InsertNode {
        kind: "bogus".into(),
        name: "X".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        target_parent: NodeId::NONE,
        page_id: None,
    }));
    assert!(s.active_children().is_empty());
}

#[test]
fn insert_node_atomic_on_bad_fill_hex() {
    let mut s = state_with(vec![]);
    assert!(!s.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "X".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: Some("not-hex".into()),
        target_parent: NodeId::NONE,
        page_id: None,
    }));
    assert!(s.active_children().is_empty());
    assert_eq!(s.max_node_id(), 0);
}

#[test]
fn insert_node_can_insert_under_parent_container() {
    let mut s = state_with(vec![frame("n1", "Frame", 0.0, 0.0, 100.0, 100.0, vec![])]);

    assert!(s.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Child".into(),
        x: 1,
        y: 2,
        width: 3,
        height: 4,
        fill_hex: None,
        target_parent: NodeId::new("n1"),
        page_id: None,
    }));

    assert_eq!(s.active_children().len(), 1);
    let children = s.active_children()[0].children().expect("parent children");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].base().name.as_deref(), Some("Child"));
}

#[test]
fn insert_node_can_insert_on_requested_page_without_switching_active_page() {
    let mut s = state_with(vec![]);
    s.doc.pages = Some(vec![
        PenPage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![frame("n1", "Current", 0.0, 0.0, 100.0, 100.0, vec![])],
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "page-2".into(),
            name: "Page 2".into(),
            children: Vec::new(),
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;

    assert!(s.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Other Page".into(),
        x: 1,
        y: 2,
        width: 3,
        height: 4,
        fill_hex: None,
        target_parent: NodeId::NONE,
        page_id: Some("page-2".into()),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    assert_eq!(pages[0].children.len(), 1);
    assert_eq!(pages[1].children.len(), 1);
    assert_eq!(
        pages[1].children[0].base().name.as_deref(),
        Some("Other Page")
    );
    assert_eq!(s.ui.active_page_index, 0);
}
