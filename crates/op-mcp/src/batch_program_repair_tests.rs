//! Malformed-program repair + redraft-convergence tests for the
//! `batch_design` DSL executor — the weak-model resilience half of
//! `batch_program_tests.rs`, carved off to keep both files under the
//! 800-line cap.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::Value;

use super::batch_design_snapshot;
use super::batch_program_test_support::{binding_id, call_operations};
use super::test_fixtures::sample;
use super::{McpTool, ToolOutcome};

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
fn post_process_materializes_icon_font_dimensions_before_the_batch_applies() {
    let mut state = sample();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("postProcess".into(), "true".into());
    args.insert(
        "operations".into(),
        "icon=I(\"n10\", {\"type\":\"icon_font\",\"name\":\"Search\",\"iconFontName\":\"search\",\"fontSize\":32})"
            .into(),
    );
    let ToolOutcome::OkJsonWithCommand(json, command) = tool.call(&args) else {
        panic!("expected post-processed command");
    };
    let envelope: Value = serde_json::from_str(&json).expect("valid envelope");
    let icon_id = binding_id(&envelope, "icon");
    let EditorCommand::InsertAuthoredSubtree { nodes, .. } = &command else {
        panic!("expected one authored insert: {command:?}");
    };
    assert_eq!(envelope["postProcessed"], Value::Bool(true), "{envelope}");
    assert_eq!(
        (nodes[0].width_px(), nodes[0].height_px()),
        (Some(24.0), Some(24.0)),
        "postProcess must materialize dimensions in the emitted command"
    );

    assert!(state.apply(command));
    let icon = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&icon_id))
        .expect("icon applies");
    assert!(
        matches!(icon, jian_ops_schema::node::PenNode::IconFont(_)),
        "post-processed node must stay an icon_font: {icon:?}"
    );
    assert_eq!(
        (icon.width_px(), icon.height_px()),
        (Some(24.0), Some(24.0))
    );
}

fn category_rail_json() -> &'static str {
    r#"{"type":"frame","name":"Category rail","width":"fill_container","height":232,"layout":"horizontal","children":[
            {"type":"frame","name":"Category 1","width":160,"height":160},
            {"type":"frame","name":"Category 2","width":160,"height":160},
            {"type":"frame","name":"Category 3","width":160,"height":160},
            {"type":"frame","name":"Category 4","width":160,"height":160},
            {"type":"frame","name":"Category 5","width":160,"height":160}
        ]}"#
}

fn category_rail_program(parent: &str) -> String {
    format!("rail=I({parent}, {})\nD(\"ghost\")", category_rail_json())
}

#[test]
fn flat_child_insert_post_process_preserves_authored_category_rail_height() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("postProcess".into(), "true".into());
    args.insert(
        "operations".into(),
        format!("rail=I(\"n10\", {})", category_rail_json()),
    );

    let ToolOutcome::OkJsonWithCommand(_, command) = tool.call(&args) else {
        panic!("expected post-processed flat insert command");
    };
    let EditorCommand::InsertAuthoredSubtree {
        nodes, parent_id, ..
    } = &command
    else {
        panic!("expected one flat insert: {command:?}");
    };
    assert_eq!(parent_id.as_str(), "n10");
    assert_eq!(
        nodes[0].height_px(),
        Some(232.0),
        "flat child insert must share the child-safe postProcess path"
    );
}

#[test]
fn child_insert_post_process_preserves_authored_category_rail_height() {
    let mut state = sample();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("postProcess".into(), "true".into());
    // The harmless direct op forces this mixed program through
    // `run_batch_design_program`, rather than the flat insert parser.
    args.insert("operations".into(), category_rail_program(r#""n10""#));

    let ToolOutcome::OkJsonWithCommand(json, command) = tool.call(&args) else {
        panic!("expected post-processed program command");
    };
    let envelope: Value = serde_json::from_str(&json).expect("valid envelope");
    let rail_id = binding_id(&envelope, "rail");
    let EditorCommand::InsertAuthoredSubtreePreservingRoots {
        nodes, parent_id, ..
    } = &command
    else {
        panic!("expected one program insert: {command:?}");
    };
    assert_eq!(parent_id.as_str(), "n10");
    assert_eq!(nodes[0].height_px(), Some(232.0));

    assert!(state.apply(command));
    let rail = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&rail_id))
        .expect("category rail applies");
    assert_eq!(
        rail.height_px(),
        Some(232.0),
        "child postProcess must not sum horizontal card heights into 800 px"
    );
}

#[test]
fn root_insert_post_process_keeps_root_height_adjustment() {
    let tool = batch_design_snapshot(&sample());
    let mut args = BTreeMap::new();
    args.insert("postProcess".into(), "true".into());
    args.insert("operations".into(), category_rail_program("null"));

    let ToolOutcome::OkJsonWithCommand(_, command) = tool.call(&args) else {
        panic!("expected post-processed program command");
    };
    let EditorCommand::InsertAuthoredSubtreePreservingRoots {
        nodes, parent_id, ..
    } = &command
    else {
        panic!("expected one program insert: {command:?}");
    };
    assert!(!parent_id.is_real(), "root insert must target the document");
    assert_eq!(
        nodes[0].height_px(),
        Some(800.0),
        "real document roots retain the established height-to-content pass"
    );
}

#[test]
fn child_copy_post_process_preserves_authored_category_rail_height() {
    let mut state = sample();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("postProcess".into(), "true".into());
    args.insert(
        "operations".into(),
        format!(
            "source=I(\"n10\", {})\ncopy=C(source, \"n10\")\nD(\"ghost\")",
            category_rail_json()
        ),
    );

    let ToolOutcome::OkJsonWithCommand(json, command) = tool.call(&args) else {
        panic!("expected post-processed copy program command");
    };
    let envelope: Value = serde_json::from_str(&json).expect("valid envelope");
    let source_id = binding_id(&envelope, "source");
    let copy_id = binding_id(&envelope, "copy");
    assert!(state.apply(command));
    for node_id in [source_id, copy_id] {
        let rail =
            op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&node_id))
                .unwrap_or_else(|| panic!("category rail {node_id} applies"));
        assert_eq!(
            rail.height_px(),
            Some(232.0),
            "child C() postProcess must not run the root-only height pass"
        );
    }
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

#[test]
fn logical_text_align_is_normalized_without_touching_container_alignment() {
    // Text alignment is a physical enum (`left`/`right`), while layout-axis
    // alignment legitimately uses logical `start`/`end`. A generated forecast
    // used the latter spelling for both HIGH/LOW labels and lost every text
    // node during deserialization.
    let mut state = sample();
    let program = concat!(
        "row=I(null, {\"type\":\"frame\",\"name\":\"Forecast Row\",\"layout\":\"horizontal\",\"justifyContent\":\"end\",\"alignItems\":\"start\"})\n",
        "high=I(row, {\"type\":\"text\",\"name\":\"High\",\"content\":\"72°\",\"textAlign\":\"start\"})\n",
        "low=I(row, {\"type\":\"text\",\"name\":\"Low\",\"content\":\"54°\",\"textAlign\":\"end\"})"
    );
    let (envelope, cmd) = call_operations(&state, program);
    assert!(
        envelope.get("errors").is_none(),
        "logical text alignments must not drop their nodes: {envelope}"
    );
    let row_id = binding_id(&envelope, "row");
    assert!(state.apply(cmd.expect("alignment program emits a command")));

    let row = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&row_id))
        .expect("forecast row");
    let value = serde_json::to_value(row).expect("row json");
    assert_eq!(value["justifyContent"], "end");
    assert_eq!(value["alignItems"], "start");
    let children = value["children"].as_array().expect("row children");
    assert_eq!(children.len(), 2, "both HIGH/LOW labels survive");
    assert_eq!(children[0]["textAlign"], "left");
    assert_eq!(children[1]["textAlign"], "right");
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

#[test]
fn shadow_missing_spread_does_not_cascade_into_the_whole_card() {
    // 2026-07-28 production log (desktop built-in agent, program-gen): a
    // "Challenge Card" frame carried a shadow written without `spread`, serde
    // rejected the node payload, `b9` never got a binding, and all 19
    // following `I(b9, …)` lines died with "Insert parent not found" — the
    // entire card vanished from the design without a visible error.
    let mut state = sample();
    let program = concat!(
        "b9=I(\"n10\", {\"type\":\"frame\",\"name\":\"Challenge Card\",\"width\":320,\"height\":200,\"layout\":\"vertical\",",
        "\"effects\":[{\"type\":\"shadow\",\"offsetX\":0,\"offsetY\":4,\"blur\":12,\"color\":\"#00000014\"}]})\n",
        "b10=I(b9, {\"type\":\"text\",\"content\":\"Daily Challenge\",\"fontSize\":18})\n",
        "b11=I(b9, {\"type\":\"text\",\"content\":\"3 of 5 complete\",\"fontSize\":14})"
    );
    let (envelope, cmd) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let card_id = binding_id(&envelope, "b9");
    binding_id(&envelope, "b10");
    binding_id(&envelope, "b11");
    assert!(state.apply(cmd.expect("command")));
    let card = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&card_id))
        .expect("the card itself must land");
    assert_eq!(
        card.children().map(|c| c.len()).unwrap_or(0),
        2,
        "both descendant lines must attach to the recovered card"
    );
    // The authored shadow keeps its full semantics — the repair fills the
    // missing field, it does not drop the effect.
    let shadow = &serde_json::to_value(card).expect("card json")["effects"][0];
    assert_eq!(shadow["type"], "shadow");
    assert_eq!(shadow["offsetY"], 4.0);
    assert_eq!(shadow["blur"], 12.0);
    assert_eq!(shadow["spread"], 0.0);
    assert_eq!(shadow["color"], "#00000014");
}
