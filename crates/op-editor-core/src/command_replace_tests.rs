//! `EditorCommand::ReplaceNode` page targeting tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::{make_group, make_path, PenNodeExt};
use crate::test_support::{rect, state_with};
use jian_ops_schema::page::PenPage;

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn replace_node_can_target_requested_page_without_switching_active_page() {
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
            children: vec![rect("n2", "Old", 0.0, 0.0, 10.0, 10.0)],
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;

    assert!(s.apply(EditorCommand::ReplaceNode {
        node_id: id("n2"),
        kind: "rect".into(),
        name: "Replacement".into(),
        x: 5,
        y: 6,
        width: 70,
        height: 80,
        fill_hex: Some("#123456".into()),
        drop_children: false,
        page_id: Some("page-2".into()),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    assert_eq!(pages[0].children[0].base().name.as_deref(), Some("Current"));
    assert_eq!(
        pages[1].children[0].base().name.as_deref(),
        Some("Replacement")
    );
    assert_eq!(s.ui.active_page_index, 0);
}

#[test]
fn replace_subtree_swaps_in_nested_node_at_same_slot() {
    let mut s = state_with(vec![rect("n1", "Old", 0.0, 0.0, 10.0, 10.0)]);

    assert!(s.apply(EditorCommand::ReplaceSubtree {
        node_id: id("n1"),
        node: Box::new(make_group(
            "external-root".into(),
            "Card",
            vec![make_path("external-child".into(), "Line", (0.0, 0.0))],
        )),
        drop_children: false,
        page_id: None,
    }));

    assert_eq!(s.active_children().len(), 1);
    let root = &s.active_children()[0];
    assert_eq!(root.base().name.as_deref(), Some("Card"));
    assert_eq!(root.children().expect("children").len(), 1);
    assert_ne!(root.id_str(), "external-root");
    assert!(s.find_duplicate_id().is_none());
}
