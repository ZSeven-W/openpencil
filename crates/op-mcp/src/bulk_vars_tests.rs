//! Bulk variables/themes tool tests.

use std::collections::BTreeMap;

use super::{set_themes_snapshot, set_variables_snapshot, EditorCommand, McpTool, ToolOutcome};

#[test]
fn set_variables_parses_ts_json_payload_and_emits_bulk_command() {
    let tool = set_variables_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "variables".into(),
        r##"{"brand":{"type":"color","value":"#ff0000"},"gap":{"type":"number","value":8}}"##
            .into(),
    );
    args.insert("replace".into(), "true".into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, EditorCommand::SetVariables { variables, replace }) => {
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            assert!(replace);
            assert_eq!(variables.len(), 2);
            assert!(variables.contains_key("brand"));
            assert!(variables.contains_key("gap"));
        }
        other => panic!("expected SetVariables command, got {other:?}"),
    }
}

#[test]
fn set_themes_parses_ts_json_payload_and_emits_bulk_command() {
    let tool = set_themes_snapshot();
    let mut args = BTreeMap::new();
    args.insert("themes".into(), r#"{"Mode":["Light","Dark"]}"#.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, EditorCommand::SetThemes { themes, replace }) => {
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            assert!(!replace);
            assert_eq!(
                themes.get("Mode"),
                Some(&vec!["Light".to_string(), "Dark".to_string()])
            );
        }
        other => panic!("expected SetThemes command, got {other:?}"),
    }
}
