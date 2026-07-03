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

/// Recursively search a command (unwrapping `Batch`) for a leaked
/// `MergeAppState` — the regression guard for a failed stateful
/// `I()`/`R()` line (see `failed_insert_line_does_not_leak_merge_app_state`
/// / `failed_replace_line_does_not_leak_merge_app_state`).
fn contains_merge_app_state(cmd: &EditorCommand) -> bool {
    match cmd {
        EditorCommand::MergeAppState { .. } => true,
        EditorCommand::Batch { commands } => commands.iter().any(contains_merge_app_state),
        _ => false,
    }
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
fn replace_program_hoists_node_state() {
    let state = sample();
    let program = r##"swap=R("n13", {"type":"frame","name":"Counter","width":120,"height":24,"state":{"n":{"type":"int","default":0}}})
D("ghost")"##;
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    match cmd.expect("command") {
        EditorCommand::Batch { commands } => {
            assert!(
                matches!(&commands[0], EditorCommand::MergeAppState { plan_idx, state }
                if *plan_idx == usize::MAX && state.contains_key("n"))
            );
            let replacement = commands
                .iter()
                .find_map(|c| match c {
                    EditorCommand::ReplaceSubtree { node, .. } => Some(node),
                    _ => None,
                })
                .expect("ReplaceSubtree in batch");
            let v = serde_json::to_value(replacement.as_ref()).expect("json");
            assert!(
                v.get("state").is_none(),
                "replacement state must be stripped"
            );
        }
        other => panic!("expected Batch, got {other:?}"),
    }
}

#[test]
fn failed_insert_line_does_not_leak_merge_app_state() {
    // A stateful `I()` line whose parent doesn't resolve to a container
    // fails at the `InsertAuthoredSubtree` emit — AFTER the node's
    // `state` would already have been hoisted. The line's error is
    // collected and the line dropped (TS best-effort semantics), so its
    // hoisted `MergeAppState` must never survive into the surviving
    // command. `U(...)` (rather than a second `I()`) is the second line
    // so `parse_operations` can't take the single-shot Insert-only fast
    // path and this program is guaranteed to run through the mixed-DSL
    // executor (`run_batch_design_program`) this fix targets.
    let mut state = sample();
    let program = r##"a=I("nonexistent-parent", {"type":"frame","name":"Ghost","width":100,"height":50,"state":{"n":{"type":"int","default":0}}})
U("n11", {"name":"Renamed"})"##;
    let (envelope, cmd) = call_operations(&state, program);
    let errors = envelope["errors"].as_array().expect("errors present");
    assert_eq!(errors.len(), 1, "{envelope}");
    assert!(
        errors[0]["error"]
            .as_str()
            .unwrap()
            .starts_with("Insert parent not found or not a container"),
        "{envelope}"
    );
    let cmd = cmd.expect("the surviving U() line still emits a command");
    assert!(
        !contains_merge_app_state(&cmd),
        "failed insert line must not leak orphan MergeAppState: {cmd:?}"
    );
    assert!(
        state.apply(cmd),
        "surviving command must still apply cleanly"
    );
}

#[test]
fn failed_replace_line_does_not_leak_merge_app_state() {
    // The insert case above fails at the emit (parent-not-container is
    // only checked inside `InsertAuthoredSubtree`'s apply). A plain
    // `R("no-such-id", ...)` instead fails EARLIER, in `find_node_by_path`
    // — before `hoist_generation_state` ever runs — so it can't exercise
    // the vulnerable emit-ordering window this fix closes. To reach that
    // window for replace, force the `ReplaceSubtree` emit ITSELF to fail
    // post-hoist: a `pageId` that resolves to no page. The program's
    // initial page-pin silently no-ops on the same unresolvable id
    // (leaving the sim on its real, default active page), so
    // `find_node_by_path` still finds "n13" and the node's `state` is
    // hoisted — but the ReplaceSubtree command's own page stamp is then
    // rejected at apply, reproducing exactly the failure shape codex
    // flagged for `I()`.
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "does-not-exist".into());
    args.insert(
        "operations".into(),
        r##"swap=R("n13", {"type":"frame","name":"Ghost","width":100,"height":50,"state":{"n":{"type":"int","default":0}}})
D("ghost")"##
            .into(),
    );
    let (envelope, cmd): (Value, Option<EditorCommand>) = match tool.call(&args) {
        ToolOutcome::OkJson(json) => (serde_json::from_str(&json).expect("json"), None),
        ToolOutcome::OkJsonWithCommand(json, cmd) => {
            (serde_json::from_str(&json).expect("json"), Some(cmd))
        }
        other => panic!("expected a TS result envelope, got {other:?}"),
    };
    let errors = envelope["errors"].as_array().expect("errors present");
    assert_eq!(errors.len(), 1, "{envelope}");
    assert!(
        errors[0]["error"]
            .as_str()
            .unwrap()
            .starts_with("Replace failed for:"),
        "{envelope}"
    );
    // `D("ghost")` is a silent no-op (unknown id), so nothing else could
    // legitimately contribute a command here — any surviving command
    // must NOT be (or contain) the orphaned MergeAppState.
    assert!(
        cmd.as_ref()
            .map(|c| !contains_merge_app_state(c))
            .unwrap_or(true),
        "failed replace line must not leak orphan MergeAppState: {cmd:?}"
    );
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

#[test]
fn stray_quote_line_does_not_swallow_following_operations() {
    // A weak model fused a value's close-quote with the trailing comma
    // (`"fontWeight":"700,"fill"` — `fill` ends up unquoted, an ODD number of
    // quotes on the line). The old quote/bracket state machine leaked the open
    // string across the newline and ate every following op into one malformed
    // blob. `split_operations` now anchors boundaries to the operation-start
    // grammar, and `parse_json_arg` repairs the fused quote, so the broken line
    // recovers AND the next op survives.
    let mut state = sample();
    let program = "a=I(null, {\"type\":\"text\",\"content\":\"x\",\"fontWeight\":\"700,\"fill\":[{\"type\":\"solid\",\"color\":\"#111111\"}]})\nb=I(null, {\"type\":\"frame\",\"name\":\"Kept\",\"width\":50,\"height\":50})";
    let (envelope, cmd) = call_operations(&state, program);
    assert!(
        envelope.get("errors").is_none(),
        "stray quote recovered: {envelope}"
    );
    let b_id = binding_id(&envelope, "b");
    binding_id(&envelope, "a");
    assert!(state.apply(cmd.expect("command")));
    assert!(
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&b_id)).is_some(),
        "the op after the stray-quote line must survive"
    );
}

#[test]
fn unclosed_root_node_is_auto_closed_so_children_survive() {
    // A weak model dropped the node's closing `}` before `)` (classic with a
    // nested `"stroke":{...}`). When that node is the ROOT binding, the failure
    // used to cascade — every `I(sec, ...)` child then can't find `sec`. The
    // brace-balancing repair recovers the root so its children land.
    let mut state = sample();
    let program = "sec=I(null, {\"type\":\"frame\",\"name\":\"Sec\",\"layout\":\"vertical\",\"stroke\":{\"thickness\":1,\"fill\":[{\"type\":\"solid\",\"color\":\"#111111\"}]})\nc=I(sec, {\"type\":\"text\",\"content\":\"Hi\"})";
    let (envelope, cmd) = call_operations(&state, program);
    let sec_id = binding_id(&envelope, "sec");
    binding_id(&envelope, "c");
    assert!(state.apply(cmd.expect("command")));
    let sec = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&sec_id))
        .expect("recovered root");
    assert!(
        sec.children().map(|c| !c.is_empty()).unwrap_or(false),
        "child must nest under the recovered root"
    );
}

#[test]
fn missing_opening_quote_on_string_value_is_repaired() {
    // `"width":fill_container"` — the weak model dropped the value's leading `"`.
    let mut state = sample();
    let program =
        "a=I(null, {\"type\":\"frame\",\"name\":\"S\",\"width\":fill_container\",\"height\":40})";
    let (envelope, cmd) = call_operations(&state, program);
    assert!(
        envelope.get("errors").is_none(),
        "missing-open-quote repaired: {envelope}"
    );
    let id = binding_id(&envelope, "a");
    assert!(state.apply(cmd.expect("command")));
    assert!(
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&id)).is_some()
    );
}

#[test]
fn empty_stroke_array_is_tolerated_as_no_stroke() {
    // A weak model emitted `"stroke":[]`, which deserializes as a 0-length
    // `PenStroke` and failed the whole node (cascading to its children).
    // `normalize_node_shape` drops an empty stroke so the node still lands.
    let mut state = sample();
    let program =
        "a=I(null, {\"type\":\"frame\",\"name\":\"S\",\"width\":40,\"height\":40,\"stroke\":[]})";
    let (envelope, cmd) = call_operations(&state, program);
    assert!(
        envelope.get("errors").is_none(),
        "empty stroke tolerated: {envelope}"
    );
    let id = binding_id(&envelope, "a");
    assert!(state.apply(cmd.expect("command")));
    assert!(
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&id)).is_some()
    );
}

/// Count nodes named `name` anywhere in the forest.
fn count_named(nodes: &[jian_ops_schema::node::PenNode], name: &str) -> usize {
    nodes
        .iter()
        .map(|n| {
            let own = usize::from(n.base().name.as_deref() == Some(name));
            own + n.children().map(|c| count_named(c, name)).unwrap_or(0)
        })
        .sum()
}

#[test]
fn redrafted_binding_converges_to_the_last_draft() {
    // A weak model deliberating in-channel re-emits its section several times
    // ("Let me redo…") with the SAME binding, parent, type, and name — one
    // minimax-m3 response stacked SEVEN navbars this way. The re-insert must
    // supersede the earlier draft, not sibling it.
    let mut state = sample();
    let program = concat!(
        "nav=I(\"n10\", {\"type\":\"frame\",\"name\":\"Nav\",\"layout\":\"horizontal\",\"children\":[{\"type\":\"text\",\"name\":\"Brand\",\"content\":\"draft one\"}]})\n",
        "nav=I(\"n10\", {\"type\":\"frame\",\"name\":\"Nav\",\"layout\":\"horizontal\",\"children\":[{\"type\":\"text\",\"name\":\"Brand\",\"content\":\"draft two\"},{\"type\":\"text\",\"name\":\"Links\",\"content\":\"Features\"}]})"
    );
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    assert!(state.apply(cmd.expect("command")));
    assert_eq!(
        count_named(state.active_children(), "Nav"),
        1,
        "the redraft must replace draft one"
    );
    // The survivor is the LAST draft (two children, updated copy).
    assert_eq!(count_named(state.active_children(), "Links"), 1);
}

#[test]
fn scratch_binding_reuse_under_different_parents_keeps_both_nodes() {
    // Lazy binding reuse is NOT a redraft: `t=I(cardA, …)` then
    // `t=I(cardB, …)` targets different parents — both must survive.
    let mut state = sample();
    let program = concat!(
        "a=I(\"n10\", {\"type\":\"frame\",\"name\":\"Card A\",\"layout\":\"vertical\"})\n",
        "b=I(\"n10\", {\"type\":\"frame\",\"name\":\"Card B\",\"layout\":\"vertical\"})\n",
        "t=I(a, {\"type\":\"text\",\"name\":\"Label\",\"content\":\"one\"})\n",
        "t=I(b, {\"type\":\"text\",\"name\":\"Label\",\"content\":\"two\"})"
    );
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    assert!(state.apply(cmd.expect("command")));
    assert_eq!(
        count_named(state.active_children(), "Label"),
        2,
        "different parents → both scratch inserts survive"
    );
}

#[test]
fn children_after_a_redraft_attach_to_the_new_draft() {
    // Draft 1 inserts nav + a child under it; draft 2 re-emits nav (deleting
    // draft 1's subtree INCLUDING the child), then re-emits the child whose
    // previous node is already gone — the stale-binding delete must be a
    // no-op and the child must land under the NEW nav.
    let mut state = sample();
    let program = concat!(
        "nav=I(\"n10\", {\"type\":\"frame\",\"name\":\"Nav\",\"layout\":\"vertical\"})\n",
        "u=I(nav, {\"type\":\"frame\",\"name\":\"Utility\",\"layout\":\"horizontal\"})\n",
        "nav=I(\"n10\", {\"type\":\"frame\",\"name\":\"Nav\",\"layout\":\"vertical\"})\n",
        "u=I(nav, {\"type\":\"frame\",\"name\":\"Utility\",\"layout\":\"horizontal\"})"
    );
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    assert!(state.apply(cmd.expect("command")));
    assert_eq!(count_named(state.active_children(), "Nav"), 1);
    assert_eq!(count_named(state.active_children(), "Utility"), 1);
    let nav_id = binding_id(&envelope, "nav");
    let nav = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&nav_id))
        .expect("final nav");
    assert_eq!(
        nav.children().map(|c| c.len()).unwrap_or(0),
        1,
        "utility must nest under the final draft"
    );
}
