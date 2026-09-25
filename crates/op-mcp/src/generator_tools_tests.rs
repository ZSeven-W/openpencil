//! MCP generator tools: each write returns ONE command the host applies,
//! failures return an error and no command.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::{json, Value};

use super::*;

fn args(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn ok_json(outcome: ToolOutcome) -> (Value, Option<EditorCommand>) {
    match outcome {
        ToolOutcome::OkJsonWithCommand(json, command) => {
            (serde_json::from_str(&json).unwrap(), Some(command))
        }
        ToolOutcome::OkJson(json) => (serde_json::from_str(&json).unwrap(), None),
        other => panic!("expected a JSON result, got {other:?}"),
    }
}

const PROGRAM: &str =
    "for (var i = 0; i < params.count; i++) I(root, {type: 'rectangle', name: 'Bar ' + i, width: 10 + i * params.step, height: 8, fill: [{type: 'solid', color: params.color}]});";

fn params_json() -> String {
    json!([
        {"name": "count", "kind": "number", "value": 3},
        {"name": "step", "kind": "number", "value": 5, "label": "Step"},
        {"name": "color", "kind": "color", "value": "#3366FF"}
    ])
    .to_string()
}

fn create_custom(state: &mut EditorState) -> NodeId {
    let tool = create_generator_snapshot(state);
    let (result, command) = ok_json(tool.call(&args(&[
        ("program", PROGRAM),
        ("params", &params_json()),
        ("name", "Bars"),
        ("x", "40"),
        ("y", "60"),
    ])));
    assert_eq!(result["childCount"], 3);
    assert!(state.apply(command.expect("create ships a command")));
    NodeId::new(result["nodeId"].as_str().unwrap())
}

fn children(state: &EditorState, id: &NodeId) -> Vec<String> {
    op_editor_core::walkers::find_node(state.active_children(), id)
        .and_then(|node| node.children().cloned())
        .unwrap_or_default()
        .iter()
        .map(|node| node.base().name.clone().unwrap_or_default())
        .collect()
}

#[test]
fn create_from_program_and_params_places_and_materializes() {
    let mut state = EditorState::new();
    let id = create_custom(&mut state);
    let node = op_editor_core::walkers::find_node(state.active_children(), &id).unwrap();
    assert_eq!(node.base().name.as_deref(), Some("Bars"));
    assert_eq!(node.base().x, Some(40.0));
    assert_eq!(children(&state, &id), vec!["Bar 0", "Bar 1", "Bar 2"]);
}

#[test]
fn create_from_a_starter() {
    let mut state = EditorState::new();
    let tool = create_generator_snapshot(&state);
    let (result, command) = ok_json(tool.call(&args(&[("starter", "month-calendar"), ("x", "5")])));
    assert!(state.apply(command.unwrap()));
    let id = NodeId::new(result["nodeId"].as_str().unwrap());
    let spec = state.generator_spec(&id).unwrap();
    assert_eq!(spec.starter.as_deref(), Some("month-calendar"));
    assert_eq!(children(&state, &id)[0], "Month");
    assert!(matches!(
        tool.call(&args(&[("starter", "nope")])),
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
    ));
}

#[test]
fn update_params_regenerates_in_one_command_and_is_idempotent() {
    let mut state = EditorState::new();
    let id = create_custom(&mut state);
    let tool = update_generator_params_snapshot(&state);
    let (result, command) = ok_json(tool.call(&args(&[
        ("node_id", id.as_str()),
        ("params", r##"{"count": "5", "color": "#abc"}"##),
    ])));
    assert_eq!(result["changed"], true);
    assert_eq!(result["childCount"], 5);
    let command = command.expect("one command");
    assert!(matches!(command, EditorCommand::PatchNodeData { .. }));
    assert!(state.apply(command));
    let spec = state.generator_spec(&id).unwrap();
    assert_eq!(spec.params[2].value, json!("#AABBCC"));
    // Same values again: nothing to write.
    let tool = update_generator_params_snapshot(&state);
    let (result, command) = ok_json(tool.call(&args(&[
        ("node_id", id.as_str()),
        ("params", r#"{"count": 5}"#),
    ])));
    assert_eq!(result["changed"], false);
    assert!(command.is_none());
}

#[test]
fn update_can_swap_the_program() {
    let mut state = EditorState::new();
    let id = create_custom(&mut state);
    let tool = update_generator_params_snapshot(&state);
    let (_, command) = ok_json(tool.call(&args(&[
        ("node_id", id.as_str()),
        (
            "program",
            "I(root, {type: 'text', name: 'Only', content: 'x'});",
        ),
    ])));
    assert!(state.apply(command.unwrap()));
    assert_eq!(children(&state, &id), vec!["Only"]);
    assert!(state.generator_spec(&id).unwrap().program.contains("Only"));
}

#[test]
fn failures_return_errors_and_no_command() {
    let mut state = EditorState::new();
    let id = create_custom(&mut state);
    let tool = update_generator_params_snapshot(&state);
    assert!(matches!(
        tool.call(&args(&[("node_id", id.as_str()), ("params", r#"{"nope": 1}"#)])),
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, message) if message.contains("nope")
    ));
    assert!(matches!(
        tool.call(&args(&[("node_id", id.as_str()), ("program", "while (true) {}")])),
        ToolOutcome::Err(ToolErrorCode::ToolFailed, message) if message.contains("time limit")
    ));
    assert!(matches!(
        tool.call(&args(&[("node_id", "missing")])),
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
    ));
    let create = create_generator_snapshot(&state);
    assert!(matches!(
        create.call(&args(&[])),
        ToolOutcome::Err(ToolErrorCode::MissingArgument, _)
    ));
}

#[test]
fn get_and_detach() {
    let mut state = EditorState::new();
    let id = create_custom(&mut state);
    let (info, _) =
        ok_json(get_generator_snapshot(&state).call(&args(&[("node_id", id.as_str())])));
    assert_eq!(info["params"][1]["label"], "Step");
    assert_eq!(info["childrenEdited"], false);
    assert!(info["program"].as_str().unwrap().contains("params.count"));
    let (result, command) =
        ok_json(detach_generator_snapshot(&state).call(&args(&[("node_id", id.as_str())])));
    assert_eq!(result["detached"], true);
    assert!(state.apply(command.unwrap()));
    assert_eq!(children(&state, &id).len(), 3);
    assert!(matches!(
        get_generator_snapshot(&state).call(&args(&[("node_id", id.as_str())])),
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, message) if message.contains("not a generator")
    ));
}
