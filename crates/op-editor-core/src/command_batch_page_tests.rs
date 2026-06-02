//! Batch write command page targeting tests.

#![cfg(test)]

use crate::command::{BatchInsertItem, EditorCommand};
use crate::node_id::NodeId;
use crate::pen_node_ext::{make_group, PenNodeExt};
use crate::test_support::{rect, state_with};
use jian_ops_schema::page::PenPage;

#[test]
fn batch_insert_can_target_requested_page_without_switching_active_page() {
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
            children: Vec::new(),
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;

    assert!(s.apply(EditorCommand::BatchInsert {
        items: vec![BatchInsertItem {
            kind: "rect".into(),
            name: "Other Page".into(),
            x: 1,
            y: 2,
            width: 3,
            height: 4,
            fill_hex: None,
        }],
        page_id: Some("page-2".into()),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    assert_eq!(pages[0].children[0].base().name.as_deref(), Some("Current"));
    assert_eq!(
        pages[1].children[0].base().name.as_deref(),
        Some("Other Page")
    );
    assert_eq!(s.ui.active_page_index, 0);
}

#[test]
fn insert_subtree_can_target_requested_page_without_switching_active_page() {
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
            children: Vec::new(),
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;

    assert!(s.apply(EditorCommand::InsertSubtree {
        nodes: vec![make_group("external".into(), "Card", vec![])],
        parent_id: NodeId::NONE,
        page_id: Some("page-2".into()),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    assert_eq!(pages[0].children[0].base().name.as_deref(), Some("Current"));
    assert_eq!(pages[1].children[0].base().name.as_deref(), Some("Card"));
    assert_eq!(s.ui.active_page_index, 0);
}

#[test]
fn add_page_can_use_supplied_initial_children() {
    let mut s = state_with(vec![rect("n1", "Existing", 0.0, 0.0, 10.0, 10.0)]);

    assert!(s.apply(EditorCommand::AddPage {
        name: Some("Landing".into()),
        children: Some(vec![make_group("external".into(), "Hero", vec![])]),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[1].name, "Landing");
    assert_eq!(pages[1].children.len(), 1);
    assert_eq!(pages[1].children[0].base().name.as_deref(), Some("Hero"));
    assert_ne!(
        pages[1].children[0].id_str(),
        "external",
        "externally-authored child ids should be remapped like InsertSubtree"
    );
    assert_eq!(
        pages[0].children[0].base().name.as_deref(),
        Some("Existing"),
        "single-page children should still migrate to Page 1"
    );
}
