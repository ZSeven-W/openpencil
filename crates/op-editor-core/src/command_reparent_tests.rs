//! `EditorCommand::MoveNode` / `CopyNode` page targeting tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{frame, rect, state_with};
use jian_ops_schema::page::PenPage;

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn move_node_can_target_requested_page_without_switching_active_page() {
    let mut s = state_with(vec![]);
    s.doc.pages = Some(vec![
        PenPage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![rect("n1", "Current", 0.0, 0.0, 10.0, 10.0)],
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "page-2".into(),
            name: "Page 2".into(),
            children: vec![
                frame("n2", "Target", 0.0, 0.0, 100.0, 100.0, Vec::new()),
                rect("n3", "Moved", 0.0, 0.0, 10.0, 10.0),
            ],
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;

    assert!(s.apply(EditorCommand::MoveNode {
        node_id: id("n3"),
        target_parent: id("n2"),
        page_id: Some("page-2".into()),
        index: None,
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    let target_children = pages[1].children[0].children().expect("frame children");
    assert_eq!(target_children.len(), 1);
    assert_eq!(target_children[0].id_str(), "n3");
    assert_eq!(s.ui.active_page_index, 0);
}

#[test]
fn move_node_can_insert_at_requested_root_index() {
    let mut s = state_with(vec![
        rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "B", 0.0, 0.0, 10.0, 10.0),
        rect("n3", "C", 0.0, 0.0, 10.0, 10.0),
    ]);

    assert!(s.apply(EditorCommand::MoveNode {
        node_id: id("n3"),
        target_parent: NodeId::NONE,
        page_id: None,
        index: Some(1),
    }));

    let ids: Vec<&str> = s.active_children().iter().map(|n| n.id_str()).collect();
    assert_eq!(ids, vec!["n1", "n3", "n2"]);
}

#[test]
fn copy_node_can_target_requested_page_without_switching_active_page() {
    let mut s = state_with(vec![]);
    s.doc.pages = Some(vec![
        PenPage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![rect("n1", "Current", 0.0, 0.0, 10.0, 10.0)],
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "page-2".into(),
            name: "Page 2".into(),
            children: vec![
                frame("n2", "Target", 0.0, 0.0, 100.0, 100.0, Vec::new()),
                rect("n3", "Source", 0.0, 0.0, 10.0, 10.0),
            ],
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;

    assert!(s.apply(EditorCommand::CopyNode {
        node_id: id("n3"),
        target_parent: id("n2"),
        overrides_json: None,
        page_id: Some("page-2".into()),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    let target_children = pages[1].children[0].children().expect("frame children");
    assert_eq!(target_children.len(), 1);
    assert_ne!(target_children[0].id_str(), "n3");
    assert_eq!(target_children[0].base().name.as_deref(), Some("Source"));
    assert_eq!(s.ui.active_page_index, 0);
}

#[test]
fn copy_node_applies_root_overrides_without_overriding_fresh_id() {
    let mut s = state_with(vec![rect("n1", "Source", 0.0, 0.0, 10.0, 10.0)]);

    assert!(s.apply(EditorCommand::CopyNode {
        node_id: id("n1"),
        target_parent: NodeId::NONE,
        page_id: None,
        overrides_json: Some(r#"{"id":"override-id","name":"Copy","x":42,"width":88}"#.into()),
    }));

    let clone = s
        .active_children()
        .iter()
        .find(|node| node.id_str() != "n1")
        .expect("cloned node");
    assert_ne!(clone.id_str(), "override-id");
    assert_eq!(clone.base().name.as_deref(), Some("Copy"));
    assert_eq!(clone.base().x, Some(42.0));
    assert_eq!(clone.width_px(), Some(88.0));
}
