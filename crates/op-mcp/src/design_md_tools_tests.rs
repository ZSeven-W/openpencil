//! `design.md` MCP parity tests.

use std::collections::BTreeMap;

use super::{
    export_design_md_snapshot, get_design_md_snapshot, set_design_md_snapshot, EditorCommand,
    McpTool, ToolOutcome,
};

#[test]
fn get_design_md_returns_persisted_spec_and_markdown() {
    let mut state = op_editor_core::EditorState::new();
    state.doc.design_md = Some(op_editor_core::parse_design_md(
        "# Design System: Aurora\n\n## Visual Theme\nCalm.",
    ));

    match get_design_md_snapshot(&state).call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("hasDesignMd"), Some(&"true".to_string()));
            assert!(out.get("markdown").is_some_and(|md| md.contains("Aurora")));
            let spec: serde_json::Value =
                serde_json::from_str(out.get("spec").expect("spec json")).expect("spec json");
            assert_eq!(spec["projectName"], "Aurora");
        }
        other => panic!("expected get ok, got {other:?}"),
    }
}

#[test]
fn set_design_md_markdown_emits_set_command() {
    let state = op_editor_core::EditorState::new();
    let mut args = BTreeMap::new();
    args.insert(
        "markdown".into(),
        "# Design System: Aurora\n\n## Visual Theme\nCalm.".into(),
    );

    match set_design_md_snapshot(&state).call(&args) {
        ToolOutcome::OkWithCommand(out, EditorCommand::SetDesignMd { spec }) => {
            assert_eq!(out.get("success"), Some(&"true".to_string()));
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            assert_eq!(spec.project_name.as_deref(), Some("Aurora"));
        }
        other => panic!("expected SetDesignMd command, got {other:?}"),
    }
}

#[test]
fn export_design_md_extracts_from_document_when_absent() {
    let mut state = op_editor_core::EditorState::new();
    state.doc.variables = Some(std::collections::BTreeMap::from([(
        "brand".into(),
        jian_ops_schema::variable::VariableDefinition {
            kind: jian_ops_schema::variable::VariableKind::Color,
            value: jian_ops_schema::variable::VariableValue::Scalar(
                jian_ops_schema::variable::VariableScalar::Str("#3366ff".into()),
            ),
        },
    )]));

    match export_design_md_snapshot(&state).call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            let markdown = out.get("markdown").expect("markdown");
            assert!(markdown.contains("# Design System: Untitled"));
            assert!(markdown.contains("#3366FF"));
        }
        other => panic!("expected export ok, got {other:?}"),
    }
}
