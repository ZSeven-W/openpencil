//! Tests for component commands + selection / page tools.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. Component / selection / theme tools emit
//! `EditorCommand`s applied via `EditorState::apply`.

use super::component_tools::*;
use super::page_tools::*;
use super::test_fixtures::{add_theme_axis, rect, state_with};
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use std::collections::BTreeMap;

fn state_with_two_nodes() -> EditorState {
    state_with(vec![
        rect("n11", "a", 0.0, 0.0, 10.0, 10.0),
        rect("n22", "b", 20.0, 0.0, 10.0, 10.0),
    ])
}

fn state_with_two_pages() -> EditorState {
    let mut s = state_with_two_nodes();
    assert!(s.add_page_with_name(Some("Second".into())).is_some());
    s
}

// --- Component commands ----------------------------------------------

#[test]
fn instantiate_component_emits_command() {
    let tool = instantiate_component_snapshot();
    let mut args = BTreeMap::new();
    args.insert("component_id".into(), "n5".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InstantiateComponent { component_id }) => {
            assert_eq!(component_id.as_str(), "n5");
        }
        other => panic!("expected InstantiateComponent command, got {other:?}"),
    }
}

#[test]
fn create_component_emits_command() {
    let tool = create_component_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n11".into());
    args.insert("name".into(), "Card".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::CreateComponent { node_id, name }) => {
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(name, "Card");
        }
        other => panic!("expected CreateComponent command, got {other:?}"),
    }
}

#[test]
fn delete_component_emits_command() {
    let tool = delete_component_snapshot();
    let mut args = BTreeMap::new();
    args.insert("component_id".into(), "n5".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::DeleteComponent { component_id }) => {
            assert_eq!(component_id.as_str(), "n5");
        }
        other => panic!("expected DeleteComponent command, got {other:?}"),
    }
}

#[test]
fn rename_component_emits_command() {
    let tool = rename_component_snapshot();
    let mut args = BTreeMap::new();
    args.insert("component_id".into(), "n5".into());
    args.insert("name".into(), "Renamed".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::RenameComponent { component_id, name }) => {
            assert_eq!(component_id.as_str(), "n5");
            assert_eq!(name, "Renamed");
        }
        other => panic!("expected RenameComponent command, got {other:?}"),
    }
}

#[test]
fn component_tools_validate_arguments() {
    match create_component_snapshot().call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        other => panic!("expected MissingArgument, got {other:?}"),
    }
}

#[test]
fn set_node_collapsed_surfaces_clean_gap_error() {
    let tool = super::component_tools::set_node_collapsed_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n11".into());
    args.insert("value".into(), "true".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert!(msg.contains("collapsed"), "msg names the gap: {msg}");
        }
        other => panic!("expected ToolFailed gap error, got {other:?}"),
    }
}

#[test]
fn add_page_tool_accepts_optional_name() {
    let tool = add_page_snapshot();
    let mut args = BTreeMap::new();
    args.insert("name".into(), "Checkout".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::AddPage { name, children }) => {
            assert_eq!(name.as_deref(), Some("Checkout"));
            assert!(children.is_none());
        }
        other => panic!("expected AddPage command with name, got {other:?}"),
    }
}

#[test]
fn add_page_tool_accepts_ts_children_payload() {
    let tool = add_page_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "children".into(),
        r##"[{"id":"hero","type":"frame","name":"Hero","x":0,"y":0,"width":1200,"height":640,"fill":[{"type":"solid","color":"#FFFFFF"}],"children":[]}]"##
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::AddPage {
                children: Some(children),
                ..
            },
        ) => {
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].base().name.as_deref(), Some("Hero"));
        }
        other => panic!("expected AddPage command with children, got {other:?}"),
    }
}

#[test]
fn rename_page_tool_accepts_ts_page_id() {
    let s = state_with_two_pages();
    let page_id = s.doc.pages.as_ref().unwrap()[1].id.clone();
    let tool = rename_page_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), page_id);
    args.insert("name".into(), "Renamed".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::RenamePage { index, name }) => {
            assert_eq!(index, 1);
            assert_eq!(name, "Renamed");
        }
        other => panic!("expected RenamePage command from pageId, got {other:?}"),
    }
}

#[test]
fn remove_page_alias_accepts_ts_page_id() {
    let s = state_with_two_pages();
    let page_id = s.doc.pages.as_ref().unwrap()[1].id.clone();
    let tool = remove_page_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), page_id);
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::DeletePage { index }) => {
            assert_eq!(index, 1);
        }
        other => panic!("expected DeletePage command from remove_page alias, got {other:?}"),
    }
}

#[test]
fn reorder_page_tool_accepts_ts_page_id_and_index() {
    let s = state_with_two_pages();
    let page_id = s.doc.pages.as_ref().unwrap()[1].id.clone();
    let tool = reorder_page_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), page_id);
    args.insert("index".into(), "0".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::ReorderPage { from, to }) => {
            assert_eq!(from, 1);
            assert_eq!(to, 0);
        }
        other => panic!("expected ReorderPage command from pageId/index, got {other:?}"),
    }
}

#[test]
fn duplicate_page_tool_accepts_ts_page_id_and_optional_name() {
    let s = state_with_two_pages();
    let page_id = s.doc.pages.as_ref().unwrap()[1].id.clone();
    let tool = duplicate_page_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), page_id);
    args.insert("name".into(), "Second copy 2".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::DuplicatePage { index, name }) => {
            assert_eq!(index, 1);
            assert_eq!(name.as_deref(), Some("Second copy 2"));
        }
        other => panic!("expected DuplicatePage command from pageId/name, got {other:?}"),
    }
}

// --- set_selection_set -----------------------------------------------

#[test]
fn set_selection_set_drops_unknown_ids_keeps_known() {
    let mut s = state_with_two_nodes();
    assert!(s.apply(EditorCommand::SetSelectionSet {
        node_ids: vec![
            NodeId::new("n11"),
            NodeId::new("n999"),
            NodeId::new("n22"),
            NodeId::new("n7777"),
        ],
    }));
    let ids: Vec<&str> = s.selection.set.iter().map(|n| n.as_str()).collect();
    assert_eq!(ids, vec!["n11", "n22"], "unknown ids must drop");
    assert_eq!(s.selection.anchor.as_str(), "n22");
}

#[test]
fn set_selection_set_empty_clears_selection() {
    let mut s = state_with_two_nodes();
    s.set_single_selection(NodeId::new("n11"));
    assert!(s.apply(EditorCommand::SetSelectionSet { node_ids: vec![] }));
    assert!(s.selection.is_empty());
}

#[test]
fn set_selection_set_tool_accepts_empty_arg() {
    let tool = set_selection_set_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_ids".into(), "".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::SetSelectionSet { node_ids }) => {
            assert!(node_ids.is_empty());
        }
        other => panic!("expected OkWithCommand with empty list, got {other:?}"),
    }
}

#[test]
fn set_selection_set_tool_parses_comma_separated_ids() {
    let tool = set_selection_set_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_ids".into(), "n11, n22".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::SetSelectionSet { node_ids }) => {
            assert_eq!(node_ids.len(), 2);
            assert_eq!(node_ids[0].as_str(), "n11");
            assert_eq!(node_ids[1].as_str(), "n22");
        }
        other => panic!("expected OkWithCommand, got {other:?}"),
    }
}

// --- toggle_node_selection -------------------------------------------

#[test]
fn toggle_node_selection_command_applies() {
    let mut s = state_with_two_nodes();
    assert!(s.apply(EditorCommand::ToggleNodeSelection {
        node_id: NodeId::new("n11"),
    }));
    assert_eq!(s.selection.anchor.as_str(), "n11");
    // Unknown id rejects.
    assert!(!s.apply(EditorCommand::ToggleNodeSelection {
        node_id: NodeId::new("n9999"),
    }));
}

// --- cycle_active_axis_value -----------------------------------------

#[test]
fn cycle_active_axis_value_rejects_unknown_axis_at_call_time() {
    let mut s = state_with_two_nodes();
    add_theme_axis(&mut s, "mode", &["light", "dark"]);
    let tool = cycle_active_axis_value_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "nope".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert!(msg.contains("nope"));
        }
        other => panic!("expected ToolFailed for unknown axis, got {other:?}"),
    }
}

#[test]
fn cycle_active_axis_value_accepts_known_axis() {
    let mut s = state_with_two_nodes();
    add_theme_axis(&mut s, "mode", &["light", "dark"]);
    let tool = cycle_active_axis_value_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "mode".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::CycleActiveAxisValue { axis }) => {
            assert_eq!(axis, "mode");
        }
        other => panic!("expected OkWithCommand, got {other:?}"),
    }
}

#[test]
fn cycle_active_axis_value_excludes_empty_axes_from_snapshot() {
    let mut s = state_with_two_nodes();
    add_theme_axis(&mut s, "empty", &[]);
    let tool = cycle_active_axis_value_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "empty".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::ToolFailed),
        other => panic!("empty-values axis must reject, got {other:?}"),
    }
}
