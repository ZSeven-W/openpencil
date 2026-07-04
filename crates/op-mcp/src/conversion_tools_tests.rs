use std::collections::BTreeMap;

use crate::conversion_tools::*;
use crate::{McpTool, ToolErrorCode, ToolOutcome};
use op_editor_core::{EditorCommand, PenNodeExt};

#[test]
fn upsert_variables_emits_command() {
    let tool = upsert_variables_snapshot();
    let mut args = BTreeMap::new();
    args.insert("key".into(), "tokens:theme.css".into());
    args.insert(
        "variables".into(),
        r##"{"color/primary":{"type":"color","value":"#3366ff"}}"##.into(),
    );
    args.insert("sourcePath".into(), "src/theme.css".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::UpsertVariables {
                variables,
                key,
                source_path,
                ..
            },
        ) => {
            assert_eq!(out.get("wrote").map(String::as_str), Some("true"));
            assert_eq!(out.get("key").map(String::as_str), Some("tokens:theme.css"));
            assert_eq!(out.get("count").map(String::as_str), Some("1"));
            assert_eq!(key, "tokens:theme.css");
            assert!(variables.contains_key("color/primary"));
            assert_eq!(source_path.as_deref(), Some("src/theme.css"));
        }
        other => panic!("expected UpsertVariables, got {other:?}"),
    }
}

#[test]
fn upsert_variables_rejects_bad_json() {
    let tool = upsert_variables_snapshot();
    let mut args = BTreeMap::new();
    args.insert("key".into(), "k".into());
    args.insert("variables".into(), "not-json".into());
    match tool.call(&args) {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _) => {}
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn upsert_component_requires_key_name_node() {
    let tool = upsert_component_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(ToolErrorCode::MissingArgument, msg) => assert!(msg.contains("key")),
        other => panic!("expected MissingArgument, got {other:?}"),
    }
}

#[test]
fn upsert_component_emits_command() {
    let tool = upsert_component_snapshot();
    let mut args = BTreeMap::new();
    args.insert("key".into(), "src/Button.tsx#Button".into());
    args.insert("name".into(), "Button".into());
    args.insert(
        "node_json".into(),
        r#"{"type":"frame","id":"button","name":"Button"}"#.into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::UpsertComponent {
                key, name, root, ..
            },
        ) => {
            assert_eq!(out.get("wrote").map(String::as_str), Some("true"));
            assert_eq!(key, "src/Button.tsx#Button");
            assert_eq!(name, "Button");
            assert_eq!(root.base().id, "button");
        }
        other => panic!("expected UpsertComponent, got {other:?}"),
    }
}

#[test]
fn upsert_component_rejects_bad_json() {
    let tool = upsert_component_snapshot();
    let mut args = BTreeMap::new();
    args.insert("key".into(), "k".into());
    args.insert("name".into(), "Button".into());
    args.insert("node_json".into(), "not-json".into());
    match tool.call(&args) {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _) => {}
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn upsert_screen_emits_command() {
    let tool = upsert_screen_snapshot();
    let mut args = BTreeMap::new();
    args.insert("key".into(), "route:/".into());
    args.insert(
        "node_json".into(),
        r#"{"type":"frame","id":"home","name":"Home"}"#.into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, EditorCommand::UpsertScreen { key, root, .. }) => {
            assert_eq!(out.get("wrote").map(String::as_str), Some("true"));
            assert_eq!(key, "route:/");
            assert_eq!(root.base().id, "home");
        }
        other => panic!("expected UpsertScreen, got {other:?}"),
    }
}

#[test]
fn upsert_screen_rejects_bad_json() {
    let tool = upsert_screen_snapshot();
    let mut args = BTreeMap::new();
    args.insert("key".into(), "route:/".into());
    args.insert("node_json".into(), "not-json".into());
    match tool.call(&args) {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _) => {}
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}
