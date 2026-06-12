//! Tests for the multi-op `batch_design` DSL program executor
//! (`batch_program.rs`) — mixed programs, bindings, slash paths, and
//! TS `runBatchDesignDsl` error semantics, exercised through the
//! public `BatchDesign` tool exactly as the MCP host drives it.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::Value;

use super::batch_design_snapshot;
use super::test_fixtures::{frame, sample, state_with};
use super::{McpTool, ToolOutcome};

fn call_operations(
    state: &op_editor_core::EditorState,
    operations: &str,
) -> (Value, Option<EditorCommand>) {
    let tool = batch_design_snapshot(state);
    let mut args = BTreeMap::new();
    args.insert("operations".into(), operations.into());
    match tool.call(&args) {
        ToolOutcome::OkJson(json) => (serde_json::from_str(&json).expect("json"), None),
        ToolOutcome::OkJsonWithCommand(json, cmd) => {
            (serde_json::from_str(&json).expect("json"), Some(cmd))
        }
        other => panic!("expected a TS result envelope, got {other:?}"),
    }
}

fn binding_id(envelope: &Value, binding: &str) -> String {
    envelope["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|r| r["binding"] == binding)
        .unwrap_or_else(|| panic!("binding {binding} missing in {envelope}"))["nodeId"]
        .as_str()
        .expect("nodeId")
        .to_string()
}

#[test]
fn mixed_program_executes_all_ops_with_shared_bindings_and_slash_paths() {
    let mut state = sample();
    let program = r##"card=I("n10", {"type":"frame","name":"Card","width":200,"height":120,"children":[{"type":"text","id":"title","name":"Title","content":"Hi","width":100,"height":24}]})
U(card+"/title", {"content":"Hello"})
copy=C(card, null, {"name":"Card Copy"})
D("n14")"##;

    let (envelope, cmd) = call_operations(&state, program);

    // TS envelope: results for I + C (U/D push nothing), no errors key.
    let results = envelope["results"].as_array().expect("results");
    assert_eq!(results.len(), 2, "{envelope}");
    assert!(envelope.get("errors").is_none(), "{envelope}");
    assert!(envelope["nodeCount"].as_u64().is_some(), "{envelope}");
    let card_id = binding_id(&envelope, "card");
    let copy_id = binding_id(&envelope, "copy");
    assert_ne!(card_id, copy_id);

    // The four surviving ops ride home as ONE atomic Batch command.
    let cmd = cmd.expect("program with successful lines must emit a command");
    let EditorCommand::Batch { ref commands } = cmd else {
        panic!("expected Batch, got {cmd:?}");
    };
    assert_eq!(commands.len(), 4, "{commands:?}");

    // Apply against the SAME doc the snapshot was taken from — the
    // predicted binding ids must be the ids that actually land.
    assert!(state.apply(cmd));
    let children = state.active_children();
    let card = op_editor_core::walkers::find_node(children, &NodeId::new(&card_id)).expect("card");
    assert_eq!(card.base().name.as_deref(), Some("Card"));
    // U(card+"/title") resolved the AUTHORED child id through the
    // alias table and patched the final node.
    let title = &card.children().expect("card children")[0];
    let title_json = serde_json::to_value(title).unwrap();
    assert_eq!(title_json["content"], "Hello", "{title_json}");
    // C cloned the card (with its child) under the page root.
    let copy = op_editor_core::walkers::find_node(children, &NodeId::new(&copy_id)).expect("copy");
    assert_eq!(copy.base().name.as_deref(), Some("Card Copy"));
    assert_eq!(copy.children().expect("copy children").len(), 1);
    // D removed n14.
    assert!(op_editor_core::walkers::find_node(children, &NodeId::new("n14")).is_none());
}

#[test]
fn failing_line_is_collected_and_execution_continues() {
    // TS `runBatchDesignDsl`: a thrown line lands in errors[] (with a
    // line preview) and the remaining lines still execute.
    let mut state = sample();
    let program = r##"a=I("n10", {"type":"rectangle","name":"A","width":10,"height":10})
U("missing-node", {"x":5})
b=I("n10", {"type":"rectangle","name":"B","width":10,"height":10})"##;

    let (envelope, cmd) = call_operations(&state, program);
    let errors = envelope["errors"].as_array().expect("errors present");
    assert_eq!(errors.len(), 1, "{envelope}");
    assert_eq!(errors[0]["error"], "Update target not found: missing-node");
    assert_eq!(
        errors[0]["line"], r##"U("missing-node", {"x":5})"##,
        "{envelope}"
    );
    let results = envelope["results"].as_array().expect("results");
    assert_eq!(results.len(), 2);

    assert!(state.apply(cmd.expect("surviving lines emit a command")));
    let frame = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n10"))
        .expect("n10");
    let names: Vec<_> = frame
        .children()
        .expect("children")
        .iter()
        .filter_map(|c| c.base().name.as_deref())
        .collect();
    assert!(names.contains(&"A") && names.contains(&"B"), "{names:?}");
}

#[test]
fn unparseable_program_returns_ts_errors_without_a_command() {
    let state = sample();
    let program = "X(1)\nfoo bar";
    let (envelope, cmd) = call_operations(&state, program);
    assert!(cmd.is_none(), "no surviving line, no command");
    assert_eq!(envelope["results"], serde_json::json!([]));
    let errors = envelope["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0]["error"], "Cannot parse operation: X(1)");
    assert_eq!(errors[1]["error"], "Cannot parse operation: foo bar");
}

#[test]
fn lenient_json_accepts_unquoted_keys_single_quotes_and_trailing_commas() {
    // TS `parseJsonArg` fallback pipeline.
    let mut state = sample();
    let program = "a=I(null, {type:'frame', name:'Lenient', width:100, height:50,})\nD(\"ghost\")";
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let id = binding_id(&envelope, "a");
    assert!(state.apply(cmd.expect("command")));
    let node = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&id))
        .expect("lenient node");
    assert_eq!(node.base().name.as_deref(), Some("Lenient"));
}

#[test]
fn delete_of_unknown_id_is_a_silent_noop_like_ts_remove_node_from_tree() {
    let state = sample();
    // Two lines so the program executor (not the single-op path) runs.
    let program = "D(\"ghost\")\nD(\"phantom\")";
    let (envelope, cmd) = call_operations(&state, program);
    assert!(cmd.is_none(), "nothing to apply");
    assert!(envelope.get("errors").is_none(), "{envelope}");
    assert_eq!(envelope["results"], serde_json::json!([]));
}

#[test]
fn root_frame_insert_replaces_the_first_empty_frame_and_inherits_position() {
    // TS auto-replace: `I(null, frame)` swaps in for an empty root
    // frame instead of creating a sibling. Lenient JSON keeps this
    // program on the executor path.
    let mut state = state_with(vec![frame("f1", "Blank", 30.0, 40.0, 100.0, 100.0, vec![])]);
    let program = "page=I(null, {type:'frame', name:'Page', width:320, height:240})";
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let page_id = binding_id(&envelope, "page");

    assert!(state.apply(cmd.expect("command")));
    let children = state.active_children();
    assert!(
        op_editor_core::walkers::find_node(children, &NodeId::new("f1")).is_none(),
        "the empty frame must be replaced"
    );
    let page = op_editor_core::walkers::find_node(children, &NodeId::new(&page_id))
        .expect("inserted page");
    assert_eq!(page.base().x, Some(30.0), "inherits the empty frame's x");
    assert_eq!(page.base().y, Some(40.0), "inherits the empty frame's y");
}

#[test]
fn bound_move_records_the_binding_and_honors_the_index() {
    let mut state = sample();
    let program = r##"box=I("n10", {"type":"rectangle","name":"Box","width":10,"height":10})
moved=M(box, "n12", 0)"##;
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let box_id = binding_id(&envelope, "box");
    assert_eq!(binding_id(&envelope, "moved"), box_id);

    assert!(state.apply(cmd.expect("command")));
    let group = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n12"))
        .expect("group");
    let first = &group.children().expect("children")[0];
    assert_eq!(first.id_str(), box_id, "M(.., 0) inserts at the front");
}

#[test]
fn move_of_unknown_target_reports_ts_error_message() {
    let state = sample();
    let program = "M(\"nope\", null)\nD(\"ghost\")";
    let (envelope, _) = call_operations(&state, program);
    let errors = envelope["errors"].as_array().expect("errors");
    assert_eq!(errors[0]["error"], "Move target not found: nope");
}

#[test]
fn replace_swaps_the_node_in_place_and_binds_the_fresh_id() {
    let mut state = sample();
    let program = r##"swap=R("n13", {"type":"text","name":"Swapped","content":"New","width":50,"height":20})
D("ghost")"##;
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let new_id = binding_id(&envelope, "swap");
    assert_ne!(new_id, "n13");

    assert!(state.apply(cmd.expect("command")));
    let children = state.active_children();
    assert!(op_editor_core::walkers::find_node(children, &NodeId::new("n13")).is_none());
    let group = op_editor_core::walkers::find_node(children, &NodeId::new("n12")).expect("n12");
    // Same slot: the replacement sits where n13 (first child) was.
    let first = &group.children().expect("children")[0];
    assert_eq!(first.id_str(), new_id, "predicted id must land in place");
    assert_eq!(first.base().name.as_deref(), Some("Swapped"));
}

#[test]
fn image_op_requires_ts_quoted_syntax_and_resolves_binding_parents() {
    let mut state = sample();
    let program = r##"wrap=I(null, {"type":"frame","name":"Wrap","x":500,"y":0,"width":400,"height":300,"children":[{"type":"text","name":"caption","content":"x","width":10,"height":10}]})
img=G("wrap", "search", "sunset photo")
G(wrap, "search", "bad syntax")"##;
    let (envelope, cmd) = call_operations(&state, program);
    let errors = envelope["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1, "{envelope}");
    assert!(
        errors[0]["error"]
            .as_str()
            .unwrap()
            .starts_with("Invalid G() syntax:"),
        "{envelope}"
    );
    let wrap_id = binding_id(&envelope, "wrap");
    let img_id = binding_id(&envelope, "img");

    assert!(state.apply(cmd.expect("command")));
    let wrap = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&wrap_id))
        .expect("wrap");
    let img = wrap
        .children()
        .expect("children")
        .iter()
        .find(|c| c.id_str() == img_id)
        .expect("image under wrap");
    assert!(matches!(img, jian_ops_schema::node::PenNode::Image(_)));
    assert_eq!(img.base().name.as_deref(), Some("sunset photo"));
}

#[test]
fn post_process_flag_marks_the_envelope() {
    let state = sample();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("postProcess".into(), "true".into());
    args.insert(
        "operations".into(),
        "a=I(\"n10\", {\"type\":\"rectangle\",\"name\":\"A\",\"width\":10,\"height\":10})\nD(\"ghost\")"
            .into(),
    );
    let ToolOutcome::OkJsonWithCommand(json, _) = tool.call(&args) else {
        panic!("expected envelope");
    };
    let envelope: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(envelope["postProcessed"], Value::Bool(true), "{envelope}");
}

#[test]
fn long_failing_lines_are_previewed_at_200_chars() {
    let state = sample();
    let filler = "y".repeat(400);
    let program = format!("Z({filler})\nD(\"ghost\")");
    let (envelope, _) = call_operations(&state, &program);
    let line = envelope["errors"][0]["line"].as_str().expect("line");
    assert_eq!(line.chars().count(), 203, "200 chars + ellipsis");
    assert!(line.ends_with("..."));
}
