//! `command::affected_node_ids` tests — the id set the MCP-driven
//! canvas indicators use to focus the agent cursor on EXISTING nodes
//! a write touched (fresh ids are the before/after diff's job, so
//! insert-family variants must report nothing here).

#![cfg(test)]

use crate::command::{
    affected_node_ids, EditorCommand, NodeFlag, StylePropValue, StylePropertyReplacement,
};
use crate::node_id::NodeId;

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn node_id_carrying_variants_report_their_target() {
    assert_eq!(
        affected_node_ids(&EditorCommand::UpdateNode {
            node_id: id("n7"),
            x: Some(10),
            y: None,
            width: None,
            height: None,
            name: None,
            fill_hex: None,
            page_id: None,
        }),
        vec!["n7".to_string()]
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::PatchNodeData {
            node_id: id("n7"),
            patch_json: "{}".to_string(),
            page_id: None,
        }),
        vec!["n7".to_string()]
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::DeleteNode {
            node_id: id("n7"),
            page_id: None,
        }),
        vec!["n7".to_string()]
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::MoveNode {
            node_id: id("n7"),
            target_parent: id("n1"),
            page_id: None,
            index: None,
        }),
        vec!["n7".to_string()],
        "the moved node is the focus target, not the parent it lands under"
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::CopyNode {
            node_id: id("n7"),
            target_parent: NodeId::NONE,
            overrides_json: None,
            page_id: None,
        }),
        vec!["n7".to_string()]
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::SetNodeFlag {
            node_id: id("n7"),
            flag: NodeFlag::Hidden,
            value: true,
        }),
        vec!["n7".to_string()]
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::SetNodeText {
            node_id: id("n7"),
            text: "hi".to_string(),
        }),
        vec!["n7".to_string()]
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::SetNodeLayoutProp {
            node_id: id("n7"),
            property: "gap".to_string(),
            value: crate::command::LayoutPropValue::Number(4.0),
        }),
        vec!["n7".to_string()]
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::CreateComponent {
            node_id: id("n7"),
            name: "C".to_string(),
        }),
        vec!["n7".to_string()]
    );
    assert_eq!(
        affected_node_ids(&EditorCommand::RefineDesign {
            root_id: id("root-a"),
            canvas_width: None,
            page_id: None,
        }),
        vec!["root-a".to_string()]
    );
}

#[test]
fn replace_all_matching_properties_reports_every_parent() {
    assert_eq!(
        affected_node_ids(&EditorCommand::ReplaceAllMatchingProperties {
            page_id: None,
            parent_ids: vec![id("p1"), id("p2"), id("p1")],
            replacements: vec![StylePropertyReplacement {
                property: "fill".to_string(),
                from: StylePropValue::String("#000".to_string()),
                to: StylePropValue::String("#fff".to_string()),
            }],
        }),
        vec!["p1".to_string(), "p2".to_string()],
        "order preserved, duplicates dropped"
    );
}

#[test]
fn batch_reports_the_union_of_sub_command_targets() {
    assert_eq!(
        affected_node_ids(&EditorCommand::Batch {
            commands: vec![
                EditorCommand::SetNodeText {
                    node_id: id("a"),
                    text: "x".to_string(),
                },
                EditorCommand::SetNodeFlag {
                    node_id: id("b"),
                    flag: NodeFlag::Locked,
                    value: true,
                },
                // Same node twice across ops → one id in the output.
                EditorCommand::SetNodeFillHex {
                    node_id: id("a"),
                    hex: "#fff".to_string(),
                },
            ],
        }),
        vec!["a".to_string(), "b".to_string()]
    );
}

#[test]
fn node_id_none_is_dropped() {
    assert!(affected_node_ids(&EditorCommand::SetNodeFlag {
        node_id: NodeId::NONE,
        flag: NodeFlag::Hidden,
        value: true,
    })
    .is_empty());
}

#[test]
fn insert_family_reports_nothing_the_id_diff_owns_fresh_ids() {
    assert!(affected_node_ids(&EditorCommand::InsertNode {
        kind: "rectangle".to_string(),
        name: "r".to_string(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        target_parent: id("p1"),
        page_id: None,
    })
    .is_empty());
    assert!(affected_node_ids(&EditorCommand::InsertSubtree {
        nodes: vec![],
        parent_id: id("p1"),
        page_id: None,
    })
    .is_empty());
    assert!(affected_node_ids(&EditorCommand::BatchInsert {
        items: vec![],
        page_id: None,
    })
    .is_empty());
}

#[test]
fn selection_page_viewport_and_clipboard_commands_report_nothing() {
    for cmd in [
        EditorCommand::ClearSelection,
        EditorCommand::SetSelection { node_id: id("n7") },
        EditorCommand::SetSelectionSet {
            node_ids: vec![id("n7")],
        },
        EditorCommand::ToggleNodeSelection { node_id: id("n7") },
        EditorCommand::SetActivePage { index: 1 },
        EditorCommand::SetViewport {
            pan_x: Some(0),
            pan_y: None,
            zoom_percent: None,
        },
        EditorCommand::CopySelected,
        EditorCommand::NudgeSelected { dx: 4, dy: 0 },
        EditorCommand::SetActiveTool {
            tool: "pen".to_string(),
        },
    ] {
        assert!(
            affected_node_ids(&cmd).is_empty(),
            "{cmd:?} must not report a node it does not carry"
        );
    }
}
