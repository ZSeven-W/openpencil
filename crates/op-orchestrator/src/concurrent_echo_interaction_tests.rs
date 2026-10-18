//! Exercise the real echo, parser and atomic replay, including a toast regression.

use super::*;
use jian_core::{expression::Expression, Runtime};
use jian_ops_schema::node::PenNode;

fn run(script: String, local: bool, reject_replay: bool) -> (VecDocSink, SubtaskOutcome) {
    crate::script_gen::parse_script(&script).expect("the echo candidate must be a valid script");
    let (mut sink, task, plan, req, outcome) = fixture(false);
    let schema = json!({"quantity":{"type":"number","default":1}});
    let node = op_editor_core::walkers::find_node_mut(
        sink.state.active_children_mut(),
        &NodeId::new("name-0"),
    )
    .unwrap();
    let mut value = serde_json::to_value(&*node).unwrap();
    let scope = if local { "$state" } else { "$app" };
    value["bindings"] = json!({"content":format!("{scope}.quantity + ''")});
    value["events"] = json!({"onTap":[{"set":{
        format!("{scope}.quantity"):format!("{scope}.quantity + 1")
    }}]});
    *node = serde_json::from_value(value).unwrap();
    if local {
        let PenNode::Frame(root) = &mut sink.state.doc.children[0] else {
            panic!()
        };
        root.base.name = Some("Recovered list".into());
        root.state = Some(serde_json::from_value(schema).unwrap());
    } else {
        sink.state.doc.state = Some(serde_json::from_value(schema).unwrap());
    }
    let before = sink.state.doc.clone();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(script)]);
    struct Reject<'a>(&'a mut VecDocSink, bool);
    impl DocSink for Reject<'_> {
        fn state(&self) -> &EditorState {
            self.0.state()
        }
        fn apply(&mut self, cmd: EditorCommand) -> bool {
            if self.1 {
                false
            } else {
                self.0.apply(cmd)
            }
        }
        fn begin_undo_batch(&mut self) {
            self.0.begin_undo_batch();
        }
        fn end_undo_batch(&mut self) {
            self.0.end_undo_batch();
        }
    }
    let result = futures::executor::block_on(maybe_geometry_echo_with_outcomes(
        &task,
        &plan,
        &req,
        &llm,
        &mut Reject(&mut sink, reject_replay),
        &AbortFlag::new(),
        false,
        false,
        None,
        &GeometryEchoBudget::new(1),
        &mut |_| {},
        &[],
        outcome,
    ));
    assert_eq!(llm.user_prompts().len(), 1);
    assert!(llm.user_prompts()[0].contains("INTERACTION LOCK"));
    if result.inserted_root_ids == ["original"] {
        assert_eq!(
            sink.state.doc, before,
            "rejected echo must leave no mutations"
        );
        assert!(
            sink.applied.is_empty(),
            "rejected candidates never reach the live sink"
        );
    } else {
        assert_eq!(sink.applied.len(), 1, "adopt as one atomic batch");
        assert!(matches!(&sink.applied[0], EditorCommand::Batch { .. }));
    }
    (sink, result)
}

fn preserving_script(local: bool) -> String {
    let scope = if local { "$state" } else { "$app" };
    let state = if local {
        r#",state:{quantity:{type:"number",default:1}}"#
    } else {
        ""
    };
    replacement(5, true)
        .replacen("height:\"fit_content\"}", &format!("height:\"fit_content\"{state}}}"), 1)
        .replacen(
            "type:\"text\",content:\"Alexander Wellington Montgomery item 0\"",
            &format!(r#"type:"text",name:"Name",content:"Alexander Wellington Montgomery item 0",bindings:{{content:"{scope}.quantity + ''"}},events:{{onTap:[{{set:{{"{scope}.quantity":"{scope}.quantity + 1"}}}}]}}"#),
            1,
        )
}

#[test]
fn toast_only_geometry_rewrite_cannot_erase_real_quantity_interaction() {
    let script = preserving_script(false).replace(
        r#"bindings:{content:"$app.quantity + ''"},events:{onTap:[{set:{"$app.quantity":"$app.quantity + 1"}}]}"#,
        r#"events:{onTap:[{toast:"'商品数量已更新'"}]}"#,
    );
    let (_, result) = run(script, false, false);
    assert_eq!(result.inserted_root_ids, ["original"]);
}

#[test]
fn preserved_quantity_binding_and_action_survive_atomic_geometry_replacement() {
    let (sink, result) = run(preserving_script(false), false, false);
    assert_ne!(result.inserted_root_ids, ["original"]);
    let mut nodes = Vec::new();
    fn collect(node: &PenNode, out: &mut Vec<serde_json::Value>) {
        out.push(serde_json::to_value(node).unwrap());
        for child in node.children().into_iter().flatten() {
            collect(child, out);
        }
    }
    for node in sink.state.active_children() {
        collect(node, &mut nodes);
    }
    let quantity = nodes
        .iter()
        .find(|node| node["bindings"].is_object())
        .unwrap();
    let runtime = Runtime::new();
    runtime.state.app_set(
        "quantity",
        sink.state.doc.state.as_ref().unwrap()["quantity"]
            .default
            .clone()
            .unwrap(),
    );
    let chain = runtime
        .actions
        .borrow()
        .parse_list(&quantity["events"]["onTap"])
        .unwrap();
    let ctx = runtime.make_action_ctx();
    futures::executor::block_on(chain.run_serial(&ctx)).unwrap();
    assert!(ctx.take_warnings().is_empty());
    let (value, diagnostics) =
        Expression::compile(quantity["bindings"]["content"].as_str().unwrap())
            .unwrap()
            .eval(&runtime.state, None, None);
    assert!(diagnostics.is_empty());
    assert_eq!(value.0, "2");
}

#[test]
fn layout_echo_cannot_pollute_app_state_even_when_copy_and_actions_match() {
    let script = preserving_script(false).replacen(
        "height:\"fit_content\"}",
        r#"height:"fit_content",state:{unrelated:{type:"number",default:99}}}"#,
        1,
    );
    let (_, result) = run(script, false, false);
    assert_eq!(result.inserted_root_ids, ["original"]);
}

#[test]
fn layout_echo_cannot_hoist_or_drop_an_existing_local_state_scope() {
    // The generation protocol hoists authored state; that transformation is
    // not allowed when replacing an existing locally scoped interaction.
    let (_, hoisted) = run(preserving_script(true), true, false);
    assert_eq!(hoisted.inserted_root_ids, ["original"]);
    let (_, rejected) = run(
        preserving_script(true).replace(r#",state:{quantity:{type:"number",default:1}}"#, ""),
        true,
        false,
    );
    assert_eq!(rejected.inserted_root_ids, ["original"]);
}

#[test]
fn interaction_snapshot_allows_geometry_but_not_moving_a_control_out_of_local_scope() {
    let doc = jian_ops_schema::load_str(
        &json!({"version":"1.0.0","children":[{
            "type":"frame","id":"root","name":"Scope","width":220,"height":100,
            "state":{"quantity":{"type":"number","default":1}},
            "children":[{"type":"text","id":"quantity","name":"Quantity","content":"1",
                "bindings":{"content":"$state.quantity + ''"}}]
        }]})
        .to_string(),
    )
    .unwrap()
    .value;
    let mut state = EditorState::from_document(doc);
    let snapshot = interaction::InteractionSnapshot::capture(&state, &["root".into()]).unwrap();
    let PenNode::Frame(root) = &mut state.doc.children[0] else {
        panic!()
    };
    root.container.width = Some(serde_json::from_value(json!(300)).unwrap());
    assert!(snapshot.matches(&state, &["root".into()]));
    let PenNode::Frame(root) = &mut state.doc.children[0] else {
        panic!()
    };
    let control = root.children.as_mut().unwrap().remove(0);
    state.doc.children.push(control);
    assert!(!snapshot.matches(&state, &["root".into(), "quantity".into()]));
}

#[test]
fn rejected_real_sink_replay_keeps_original_and_app_defaults() {
    let (_, result) = run(preserving_script(false), false, true);
    assert_eq!(result.inserted_root_ids, ["original"]);
}

#[test]
fn renamed_control_or_changed_counter_expression_is_not_a_geometry_repair() {
    for script in [
        preserving_script(false).replace("name:\"Name\"", "name:\"Different control\""),
        preserving_script(false).replace("$app.quantity + 1", "$app.quantity + 2"),
    ] {
        let (_, result) = run(script, false, false);
        assert_eq!(result.inserted_root_ids, ["original"]);
    }
}
