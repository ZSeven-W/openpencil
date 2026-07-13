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

#[test]
fn apply_design_system_lands_full_preset_and_resolves_themed_tokens() {
    use std::collections::BTreeMap;

    let tool = super::bulk_vars::apply_design_system_snapshot();
    let mut args = BTreeMap::new();
    args.insert("name".into(), "halo".into());
    let ToolOutcome::OkWithCommand(out, command) = tool.call(&args) else {
        panic!("expected OkWithCommand");
    };
    assert_eq!(out.get("applied").map(String::as_str), Some("halo"));

    let mut state = op_editor_core::EditorState::new();
    assert!(state.apply(command), "batch applies");
    let variables = state.doc.variables.as_ref().expect("variables written");
    assert!(
        variables.len() >= 40,
        "full halo table: {}",
        variables.len()
    );
    for token in [
        "--background",
        "--primary",
        "--sidebar-accent",
        "--font-primary",
    ] {
        assert!(variables.contains_key(token), "missing {token}");
    }
    let themes = state.doc.themes.as_ref().expect("themes written");
    assert_eq!(
        themes.get("Mode").map(Vec::as_slice),
        Some(["Light".to_string(), "Dark".to_string()].as_slice())
    );
    // Themed resolution: --background flips across the Mode axis.
    let light = state
        .resolve_color_variable_hex("--background")
        .expect("light value resolves");
    state.set_active_axis_value("Mode", "Dark");
    let dark = state
        .resolve_color_variable_hex("--background")
        .expect("dark value resolves");
    assert_ne!(light, dark, "Mode axis flips the token: {light} vs {dark}");

    // Unknown preset name is a friendly error.
    let mut bad = BTreeMap::new();
    bad.insert("name".into(), "nope".into());
    assert!(matches!(tool.call(&bad), ToolOutcome::Err(_, _)));
}
