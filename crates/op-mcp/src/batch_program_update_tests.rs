//! Transaction regression tests for `U()` sizing updates in mixed programs.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, NodeId};

use super::batch_design_snapshot;
use super::test_fixtures::sample;
use super::{McpTool, ToolOutcome};

#[test]
fn batch_program_transaction_applies_sizing_keywords() {
    let mut state = sample();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        concat!(
            "U(\"n10\", {\"x\":48})\n",
            "U(\"n10\", {\"width\":\"fill_container\",\"height\":\"fit_content\"})"
        )
        .into(),
    );

    let ToolOutcome::OkJsonWithCommand(json, command) = tool.call(&args) else {
        panic!("successful transaction must emit a command");
    };
    let envelope: serde_json::Value = serde_json::from_str(&json).expect("result envelope");
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let EditorCommand::Batch { commands } = &command else {
        panic!("two updates must remain one atomic batch, got {command:?}");
    };
    assert!(matches!(commands[0], EditorCommand::UpdateNode { .. }));
    assert!(matches!(commands[1], EditorCommand::PatchNodeData { .. }));

    assert!(state.apply(command), "complete transaction must apply");
    let node = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n10"))
        .expect("updated frame");
    let value = serde_json::to_value(node).expect("updated node json");
    assert_eq!(value["x"].as_f64(), Some(48.0));
    assert_eq!(value["width"], "fill_container");
    assert_eq!(value["height"], "fit_content");
}

#[test]
fn batch_program_transaction_applies_token_only_fill_patch() {
    let mut state = sample();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        concat!(
            "U(\"n10\", {\"x\":48})\n",
            "U(\"n10\", {\"fill\":[{\"type\":\"solid\",\"color\":\"$--primary\"}]})"
        )
        .into(),
    );

    let ToolOutcome::OkJsonWithCommand(json, command) = tool.call(&args) else {
        panic!("successful transaction must emit a command");
    };
    let envelope: serde_json::Value = serde_json::from_str(&json).expect("result envelope");
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let EditorCommand::Batch { commands } = &command else {
        panic!("two updates must remain one atomic batch, got {command:?}");
    };
    assert!(matches!(commands[0], EditorCommand::UpdateNode { .. }));
    assert!(matches!(commands[1], EditorCommand::PatchNodeData { .. }));

    assert!(state.apply(command), "complete transaction must apply");
    let node = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n10"))
        .expect("updated frame");
    let value = serde_json::to_value(node).expect("updated node json");
    assert_eq!(value["x"].as_f64(), Some(48.0));
    assert_eq!(value["fill"][0]["color"], "$--primary");
}

#[test]
fn batch_program_applies_fractional_geometry_without_other_fields() {
    let mut state = sample();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        "U(\"n10\", {\"x\":10.5,\"y\":20.25,\"width\":100.5,\"height\":60.25})".into(),
    );

    let outcome = tool.call(&args);
    let ToolOutcome::OkWithCommand(_, command @ EditorCommand::PatchNodeData { .. }) = outcome
    else {
        panic!("fractional geometry update must emit PatchNodeData, got {outcome:?}");
    };

    assert!(
        state.apply(command),
        "fractional geometry update must apply"
    );
    let node = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n10"))
        .expect("updated frame");
    let value = serde_json::to_value(node).expect("updated node json");
    assert_eq!(value["x"].as_f64(), Some(10.5));
    assert_eq!(value["y"].as_f64(), Some(20.25));
    assert_eq!(value["width"].as_f64(), Some(100.5));
    assert_eq!(value["height"].as_f64(), Some(60.25));
}
