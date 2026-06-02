//! `EditorCommand::DeleteNode` page targeting tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{rect, sample, state_with};
use crate::walkers::find_node;
use jian_ops_schema::page::PenPage;

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn delete_node_removes_subtree() {
    let mut s = sample();
    assert!(s.apply(EditorCommand::DeleteNode {
        node_id: id("n12"),
        page_id: None,
    }));
    let frame = find_node(s.active_children(), &id("n10")).unwrap();
    assert!(frame
        .children()
        .unwrap()
        .iter()
        .all(|child| child.id_str() != "n12"));
}

#[test]
fn delete_node_rejects_unknown_id() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::DeleteNode {
        node_id: id("ghost"),
        page_id: None,
    }));
}

#[test]
fn delete_node_can_remove_from_requested_page_without_switching_active_page() {
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
            children: vec![rect("n2", "Other", 0.0, 0.0, 10.0, 10.0)],
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;

    assert!(s.apply(EditorCommand::DeleteNode {
        node_id: id("n2"),
        page_id: Some("page-2".into()),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    assert_eq!(pages[0].children.len(), 1);
    assert!(pages[1].children.is_empty());
    assert_eq!(s.ui.active_page_index, 0);
}
