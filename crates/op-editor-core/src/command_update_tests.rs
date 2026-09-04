//! `EditorCommand::UpdateNode` field and page targeting tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{rect, state_with, text};
use crate::walkers::find_node;
use jian_ops_schema::node::{PenNode, TextContent};
use jian_ops_schema::page::PenPage;

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn update_node_patches_fields() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::UpdateNode {
        node_id: id("n1"),
        x: Some(50),
        y: None,
        width: Some(80),
        height: None,
        name: Some("Renamed".into()),
        fill_hex: None,
        page_id: None,
    }));
    let n = find_node(s.active_children(), &id("n1")).unwrap();
    assert_eq!(n.base().x, Some(50.0));
    assert_eq!(n.base().y, Some(0.0));
    assert_eq!(n.width_px(), Some(80.0));
    assert_eq!(n.base().name.as_deref(), Some("Renamed"));
}

#[test]
fn update_node_atomic_on_negative_width() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::UpdateNode {
        node_id: id("n1"),
        x: Some(99),
        y: None,
        width: Some(-5),
        height: None,
        name: None,
        fill_hex: None,
        page_id: None,
    }));
    let n = find_node(s.active_children(), &id("n1")).unwrap();
    assert_eq!(n.base().x, Some(0.0));
}

#[test]
fn update_node_rejects_unknown_id() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::UpdateNode {
        node_id: id("ghost"),
        x: Some(1),
        y: None,
        width: None,
        height: None,
        name: None,
        fill_hex: None,
        page_id: None,
    }));
}

#[test]
fn update_node_can_patch_requested_page_without_switching_active_page() {
    let mut s = state_with(vec![]);
    s.doc.pages = Some(vec![
        PenPage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![rect("n1", "Current", 0.0, 0.0, 10.0, 10.0)],
            background_color: None,
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "page-2".into(),
            name: "Page 2".into(),
            children: vec![rect("n2", "Other", 0.0, 0.0, 10.0, 10.0)],
            background_color: None,
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;

    assert!(s.apply(EditorCommand::UpdateNode {
        node_id: id("n2"),
        x: Some(42),
        y: None,
        width: None,
        height: None,
        name: Some("Other Updated".into()),
        fill_hex: None,
        page_id: Some("page-2".into()),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    assert_eq!(pages[0].children[0].base().name.as_deref(), Some("Current"));
    assert_eq!(
        pages[1].children[0].base().name.as_deref(),
        Some("Other Updated")
    );
    assert_eq!(pages[1].children[0].base().x, Some(42.0));
    assert_eq!(s.ui.active_page_index, 0);
}

#[test]
fn patch_node_data_shallow_merges_ts_text_fields() {
    let mut s = state_with(vec![text("n1", "Title", 0.0, 0.0, 100.0, 24.0, "Old")]);

    assert!(s.apply(EditorCommand::PatchNodeData {
        node_id: id("n1"),
        patch_json: r#"{"content":"New","fontSize":24}"#.into(),
        page_id: None,
    }));

    let PenNode::Text(text) = find_node(s.active_children(), &id("n1")).unwrap() else {
        panic!("expected text node");
    };
    assert_eq!(text.content, TextContent::Plain("New".into()));
    assert_eq!(text.font_size, Some(24.0));
}

#[test]
fn patch_node_data_rebuilds_a_same_id_leaf_as_another_leaf_type() {
    let mut s = state_with(vec![text(
        "action",
        "See all",
        0.0,
        0.0,
        100.0,
        24.0,
        "View all >",
    )]);

    assert!(s.apply(EditorCommand::PatchNodeData {
        node_id: id("action"),
        patch_json: r#"{
          "type":"icon_font",
          "iconFontName":"chevron-right",
          "width":20,
          "height":20,
          "content":null,
          "fontSize":null
        }"#
        .into(),
        page_id: None,
    }));

    let replacement = find_node(s.active_children(), &id("action")).unwrap();
    let PenNode::IconFont(icon) = replacement else {
        panic!("expected same-id icon_font replacement");
    };
    assert_eq!(icon.base.id, "action");
    assert_eq!(icon.icon_font_name, "chevron-right");
    let canonical = serde_json::to_value(replacement).unwrap();
    assert!(canonical.get("content").is_none());
    assert!(canonical.get("fontSize").is_none());
}
