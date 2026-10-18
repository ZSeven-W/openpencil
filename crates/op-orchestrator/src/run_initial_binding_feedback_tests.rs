use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{CallRequest, LlmChunk, LlmError};
use futures::stream::BoxStream;
use serde_json::{json, Value};

fn section() -> Value {
    json!({"type":"frame","id":"original","name":"Quantity selector",
    "layout":"vertical","width":"fill_container","height":"fit_content","gap":12,
    "children":[
        {"type":"text","id":"quantity","name":"Quantity","content":"1","fontSize":16,
         "bindings":{"content":"'' + $app.units"}},
        {"type":"frame","id":"increase","name":"Increase","layout":"horizontal",
         "width":100,"height":44,"padding":12,
         "events":{"onTap":[{"set":{"$app.units":"$app.units + 1"}}]},
         "children":[{"type":"text","id":"button-label","content":"Add","fontSize":14}]}
    ]})
}

fn fixture() -> (
    VecDocSink,
    OrchestratorPlan,
    DesignRequest,
    Vec<SubtaskOutcome>,
) {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        jian_ops_schema::load_str(
            &json!({
                "version":"1.0.0","children":[{"type":"frame","id":"page","name":"Page",
                    "layout":"vertical","width":375,"height":812,"children":[
                        {"type":"text","id":"before","content":"Keep before"}, section(),
                        {"type":"text","id":"after","content":"Keep after"}
                    ]}]
            })
            .to_string(),
        )
        .unwrap()
        .value,
    );
    let plan = serde_json::from_value(json!({
        "rootFrame":{"id":"page","name":"Page","width":375,"height":812},
        "sharedContext":{"sourceFacts":[],"exampleEntities":[{
            "id":"cup","name":"Workshop cup","attributes":{"quantity":"1"},
            "usedBy":["selector"]
        }]},
        "subtasks":[{"id":"selector","label":"Quantity selector","idPrefix":"selector",
            "region":{"width":375,"height":120},"parentFrameId":"page",
            "elements":"Quantity value and increase control; retain the existing interactive UI."}]
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: "Create a product detail App with English copy and a quantity selector".into(),
        model: Some("glm-5.3-flash".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let outcomes = vec![SubtaskOutcome {
        id: "selector".into(),
        node_count: 1,
        error: None,
        inserted_root_ids: vec!["original".into()],
        headline: None,
        subtask: None,
    }];
    assert!(!missing_initial_bindings(&sink.state, &outcomes[0].inserted_root_ids).is_empty());
    (sink, plan, request, outcomes)
}

fn proposal(change: impl FnOnce(&mut Value)) -> String {
    let mut value = json!({"state":{"units":{"type":"number","default":1}}});
    change(&mut value);
    value.to_string()
}

fn repair(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    outcomes: &mut [SubtaskOutcome],
    llm: &dyn LlmClient,
    abort: &AbortFlag,
) -> (bool, Vec<Progress>) {
    let mut events = Vec::new();
    let failed = futures::executor::block_on(repair_initial_bindings(
        sink,
        plan,
        request,
        llm,
        abort,
        outcomes,
        &mut |event| events.push(event),
    ));
    (failed, events)
}

#[test]
fn text_converted_quantity_repairs_only_the_missing_default() {
    let (mut sink, plan, request, mut outcomes) = fixture();
    assert!(sink.state.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("quantity"),
        patch_json: json!({"bindings":{"content":"to_str($app.units)"}}).to_string(),
        page_id: None,
    }));
    let before = sink.state.doc.children.clone();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(proposal(|_| {}))]);
    let (failed, _) = repair(
        &mut sink,
        &plan,
        &request,
        &mut outcomes,
        &llm,
        &AbortFlag::new(),
    );
    assert!(!failed, "{outcomes:?}");
    assert_eq!(llm.user_prompts().len(), 1);
    assert_eq!(sink.state.doc.children, before);
    assert_eq!(
        sink.state.doc.state.as_ref().unwrap()["units"].default,
        Some(json!(1))
    );
    assert!(missing_initial_bindings(&sink.state, &outcomes[0].inserted_root_ids).is_empty());
}

#[test]
fn repairs_only_owner_with_full_context_and_commits_state_and_order_atomically() {
    let (mut sink, plan, request, mut outcomes) = fixture();
    let before_nodes = sink.state.doc.children.clone();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(proposal(|_| {}))]);
    let (failed, events) = repair(
        &mut sink,
        &plan,
        &request,
        &mut outcomes,
        &llm,
        &AbortFlag::new(),
    );
    assert!(!failed, "{outcomes:?}");
    assert!(outcomes[0].error.is_none());
    assert_eq!(llm.user_prompts().len(), 1);
    let prompt = &llm.user_prompts()[0];
    for contract in [
        "INITIAL BINDING FIX REQUIRED",
        "Preserve the original",
        "Workshop cup",
        "originalSection",
    ] {
        assert!(prompt.contains(contract), "missing {contract}");
    }
    assert!(events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskDone { id, .. } if id == "selector")));
    assert_eq!(sink.applied.len(), 1);
    let EditorCommand::Batch { commands } = &sink.applied[0] else {
        panic!("expected atomic state batch")
    };
    assert!(commands
        .iter()
        .all(|command| matches!(command, EditorCommand::MergeAppState { .. })));
    assert_eq!(sink.state.doc.children, before_nodes);
    assert_eq!(
        sink.state.doc.state.as_ref().unwrap()["units"].default,
        Some(json!(1))
    );
    let siblings = crate::types::child_ids_under(&sink.state, &NodeId::new("page"));
    assert_eq!(siblings[0], "before");
    assert_eq!(siblings[2], "after");
    assert_eq!(&siblings[1..2], outcomes[0].inserted_root_ids.as_slice());
    assert_eq!(siblings, vec!["before", "original", "after"]);
    assert!(missing_initial_bindings(&sink.state, &outcomes[0].inserted_root_ids).is_empty());
}

#[test]
fn invalid_or_unrelated_defaults_are_discarded() {
    for candidate in [
        proposal(|root| root["state"]["units"]["default"] = json!(2)),
        proposal(|root| root["state"]["units"]["default"] = json!("1")),
        proposal(|root| root["state"]["units"]["default"] = Value::Null),
        proposal(|root| {
            root["state"]["unrelated"] = json!({"type":"string","default":"extra"});
        }),
        proposal(|root| root["state"] = json!({})),
    ] {
        let (mut sink, plan, request, mut outcomes) = fixture();
        let before = sink.state.doc.clone();
        outcomes[0].error = Some("previous visible warning".into());
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(candidate)]);
        let (failed, events) = repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new(),
        );
        assert!(failed);
        assert_eq!(llm.user_prompts().len(), 1);
        assert_eq!(sink.state.doc, before);
        assert!(sink.applied.is_empty());
        assert_eq!(outcomes[0].node_count, 1);
        assert_eq!(outcomes[0].inserted_root_ids, vec!["original"]);
        let error = outcomes[0].error.as_deref().unwrap();
        assert!(error.contains("previous visible warning") && error.contains(UNRESOLVED));
        assert!(outcomes[0].subtask.is_none());
        assert!(events.iter().any(|event| matches!(event, Progress::SubtaskFailed { error, .. } if error.contains(UNRESOLVED))));
    }
}

struct AbortAfterCall {
    llm: ScriptedLlm,
    abort: AbortFlag,
}

impl LlmClient for AbortAfterCall {
    fn call(&self, request: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let stream = self.llm.call(request);
        self.abort.set();
        stream
    }
}

#[test]
fn abort_after_model_call_never_deletes_original_or_merges_candidate_state() {
    let (mut sink, plan, request, mut outcomes) = fixture();
    let before = sink.state.doc.clone();
    let abort = AbortFlag::new();
    let llm = AbortAfterCall {
        llm: ScriptedLlm::new(vec![ScriptResponse::Text(proposal(|_| {}))]),
        abort: abort.clone(),
    };
    assert!(repair(&mut sink, &plan, &request, &mut outcomes, &llm, &abort).0);
    assert_eq!(llm.llm.user_prompts().len(), 1);
    assert_eq!(sink.state.doc, before);
    assert!(sink.applied.is_empty());
    assert_eq!(outcomes[0].node_count, 1);
}

struct RejectFinalBatch(VecDocSink);

impl DocSink for RejectFinalBatch {
    fn state(&self) -> &EditorState {
        &self.0.state
    }
    fn apply(&mut self, command: EditorCommand) -> bool {
        let EditorCommand::Batch { mut commands } = command else {
            panic!("only the final atomic batch may touch live state");
        };
        commands.push(EditorCommand::SetNodeText {
            node_id: NodeId::new("missing-node"),
            text: "reject".into(),
        });
        self.0.state.apply(EditorCommand::Batch { commands })
    }
    fn begin_undo_batch(&mut self) {}
    fn end_undo_batch(&mut self) {}
}

#[test]
fn rejected_real_batch_restores_original_and_generation_state_ownership() {
    let (sink, plan, request, mut outcomes) = fixture();
    let before = sink.state.doc.clone();
    let mut sink = RejectFinalBatch(sink);
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(proposal(|_| {}))]);
    assert!(
        repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new()
        )
        .0
    );
    assert_eq!(sink.0.state.doc, before);
    assert_eq!(outcomes[0].inserted_root_ids, vec!["original"]);
    assert!(sink.0.state.apply(EditorCommand::MergeAppState {
        plan_idx: 99,
        state: serde_json::from_value(json!({"units":{"type":"number","default":7}})).unwrap(),
    }));
    assert_eq!(
        sink.0.state.doc.state.as_ref().unwrap()["units"].default,
        Some(json!(7))
    );
}

#[test]
fn state_from_another_section_or_append_document_needs_no_model_call() {
    let (mut sink, plan, request, mut outcomes) = fixture();
    sink.state.doc.state = Some(
        serde_json::from_value(json!({
            "units":{"type":"number","default":4}
        }))
        .unwrap(),
    );
    let before = sink.state.doc.clone();
    let llm = ScriptedLlm::new(vec![]);
    assert!(
        !repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new()
        )
        .0
    );
    assert!(llm.user_prompts().is_empty());
    assert_eq!(sink.state.doc, before);
    assert!(sink.applied.is_empty());
}

#[test]
fn malformed_or_node_proposals_are_rejected_after_one_format_correction() {
    for candidate in [
        format!("I(null, {});", section()),
        "[]".into(),
        "{}".into(),
        proposal(|value| value["nodes"] = json!([section()])),
        proposal(|value| value["children"] = json!([])),
        proposal(|value| value["events"] = json!({"onTap":[]})),
        proposal(|value| value["width"] = json!(900)),
        proposal(|value| value["state"]["units"]["extra"] = json!(true)),
        proposal(|value| {
            value["state"]["units"].as_object_mut().unwrap().remove("type");
        }),
        proposal(|value| {
            value["state"]["units"].as_object_mut().unwrap().remove("default");
        }),
        r#"{"state":{},"state":{"units":{"type":"number","default":1}}}"#.into(),
        r#"{"state":{"units":{"type":"number","default":1},"units":{"type":"number","default":1}}}"#.into(),
        r#"{"state":{"units":{"type":"number","default":2,"default":1}}}"#.into(),
    ] {
        let (mut sink, plan, request, mut outcomes) = fixture();
        let before = sink.state.doc.clone();
        outcomes[0].error = Some("previous visible warning".into());
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text(candidate.clone()),
            ScriptResponse::Text(candidate.clone()),
        ]);
        let (failed, _) = repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new(),
        );
        assert!(failed, "{candidate}");
        assert_eq!(llm.user_prompts().len(), 2, "{candidate}");
        assert_eq!(sink.state.doc, before, "{candidate}");
        assert!(sink.applied.is_empty());
        assert_eq!(outcomes[0].inserted_root_ids, vec!["original"]);
        assert_eq!(outcomes[0].node_count, 1);
        assert!(outcomes[0].subtask.is_none());
        let error = outcomes[0].error.as_deref().unwrap();
        assert!(error.contains("previous visible warning") && error.contains(UNRESOLVED));
    }
}

#[test]
fn lower_owner_cannot_change_existing_state_and_equal_redeclaration_cannot_steal_ownership() {
    for proposed_price in [Some(99), Some(100), None] {
        let (mut sink, plan, request, mut outcomes) = fixture();
        assert!(sink.state.apply(EditorCommand::MergeAppState {
            plan_idx: 1,
            state: serde_json::from_value(json!({"price":{"type":"number","default":100}}))
                .unwrap(),
        }));
        let before = sink.state.doc.clone();
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(proposal(|root| {
            if let Some(price) = proposed_price {
                root["state"]["price"] = json!({"type":"number","default":price});
            }
        }))]);
        let failed = repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new(),
        )
        .0;
        assert_eq!(failed, proposed_price.is_some());
        assert_eq!(llm.user_prompts().len(), 1);
        assert_eq!(
            sink.state.doc.state.as_ref().unwrap()["price"].default,
            Some(json!(100))
        );
        if failed {
            assert_eq!(sink.state.doc, before);
            assert!(sink.applied.is_empty());
        } else {
            assert_eq!(sink.state.doc.children, before.children);
            assert_eq!(
                sink.state.doc.state.as_ref().unwrap()["units"].default,
                Some(json!(1))
            );
        }
        // The state-only correction must not claim an unchanged pre-existing key.
        assert!(sink.state.apply(EditorCommand::MergeAppState {
            plan_idx: 0,
            state: serde_json::from_value(json!({"price":{"type":"number","default":101}}))
                .unwrap(),
        }));
        assert_eq!(
            sink.state.doc.state.as_ref().unwrap()["price"].default,
            Some(json!(101))
        );
        let prompt = &llm.user_prompts()[0];
        assert!(prompt.contains("existingAppState"));
        assert!(
            !prompt.contains("\"default\":100"),
            "unrelated default leaked to model context"
        );
    }
}

#[test]
fn false_zero_and_empty_string_remain_legitimate_explicit_defaults() {
    for value in [json!(0), json!(false), json!("")] {
        let (mut sink, plan, request, mut outcomes) = fixture();
        let authored = match &value {
            Value::String(text) => text.clone(),
            _ => value.to_string(),
        };
        let mut document = serde_json::to_value(&sink.state.doc).unwrap();
        document["children"][0]["children"][1]["children"][0]["content"] = json!(authored);
        document["children"][0]["children"][1]["children"][0]["bindings"]["content"] =
            json!("$app.units");
        sink.state = EditorState::from_document(serde_json::from_value(document).unwrap());
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(proposal(|root| {
            root["state"]["units"]["type"] = json!(if value.is_boolean() {
                "bool"
            } else if value.is_string() {
                "string"
            } else {
                "number"
            });
            root["state"]["units"]["default"] = value.clone();
        }))]);
        assert!(
            !repair(
                &mut sink,
                &plan,
                &request,
                &mut outcomes,
                &llm,
                &AbortFlag::new()
            )
            .0,
            "{outcomes:?}"
        );
        assert_eq!(
            sink.state.doc.state.as_ref().unwrap()["units"].default,
            Some(value)
        );
    }
}

#[test]
fn selection_default_must_match_an_original_option_and_change_selected_paint() {
    for (default, should_fail) in [("cloud", false), ("invented", true)] {
        let (mut sink, plan, request, mut outcomes) = fixture();
        let mut original = section();
        original["children"] = json!([
            {"type":"frame","id":"cloud-option","name":"Cloud","width":100,"height":44,
             "fill":[{"type":"solid","color":"#ffffff"}],"bindings":{"fill":"$app.tone == 'cloud' ? '#000000' : '#ffffff'"},
             "events":{"onTap":[{"set":{"$app.tone":"'cloud'"}}]},
             "children":[{"type":"text","id":"cloud-label","content":"Cloud"}]},
            {"type":"frame","id":"night-option","name":"Night","width":100,"height":44,
             "fill":[{"type":"solid","color":"#ffffff"}],"bindings":{"fill":"$app.tone == 'night' ? '#000000' : '#ffffff'"},
             "events":{"onTap":[{"set":{"$app.tone":"'night'"}}]},
             "children":[{"type":"text","id":"night-label","content":"Night"}]}
        ]);
        let mut document = serde_json::to_value(&sink.state.doc).unwrap();
        document["children"][0]["children"][1] = original.clone();
        sink.state = EditorState::from_document(serde_json::from_value(document).unwrap());
        assert_eq!(
            missing_initial_bindings(&sink.state, &outcomes[0].inserted_root_ids).len(),
            2
        );
        let before = sink.state.doc.clone();
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            json!({"state":{"tone":{"type":"string","default":default}}}).to_string(),
        )]);
        assert_eq!(
            repair(
                &mut sink,
                &plan,
                &request,
                &mut outcomes,
                &llm,
                &AbortFlag::new()
            )
            .0,
            should_fail,
            "{outcomes:?}"
        );
        if should_fail {
            assert_eq!(sink.state.doc, before)
        } else {
            assert_eq!(sink.state.doc.children, before.children)
        }
    }
}

#[test]
fn later_owner_default_does_not_clear_an_earlier_incompatible_initial_copy_failure() {
    let (mut sink, mut plan, request, mut outcomes) = fixture();
    let mut document = serde_json::to_value(&sink.state.doc).unwrap();
    let mut other = section();
    other["id"] = json!("other-original");
    other["children"][0]["id"] = json!("other-quantity");
    other["children"][0]["content"] = json!("2");
    other["children"][1]["id"] = json!("other-increase");
    other["children"][1]["children"][0]["id"] = json!("other-label");
    document["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(other);
    sink.state = EditorState::from_document(serde_json::from_value(document).unwrap());
    let mut task = plan.subtasks[0].clone();
    task.id = "other-selector".into();
    task.id_prefix = "other-selector".into();
    plan.subtasks.push(task);
    let mut outcome = outcomes[0].clone();
    outcome.id = "other-selector".into();
    outcome.inserted_root_ids = vec!["other-original".into()];
    outcomes.push(outcome);
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(proposal(|root| root["state"] = json!({}))),
        ScriptResponse::Text(proposal(|root| {
            root["state"]["units"]["default"] = json!(2);
        })),
    ]);
    assert!(
        repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new()
        )
        .0
    );
    assert_eq!(llm.user_prompts().len(), 2);
    assert!(outcomes[0].error.as_deref().unwrap().contains(UNRESOLVED));
    assert!(outcomes[1].error.is_none());
    assert!(missing_initial_bindings(&sink.state, &outcomes[0].inserted_root_ids).is_empty());
}

#[test]
fn earlier_success_cannot_hide_a_later_owners_conflicting_shared_initial_value() {
    let (mut sink, mut plan, request, mut outcomes) = fixture();
    let mut document = serde_json::to_value(&sink.state.doc).unwrap();
    let mut other = section();
    other["id"] = json!("other-original");
    other["children"][0]["id"] = json!("other-quantity");
    other["children"][0]["content"] = json!("2");
    other["children"][1]["id"] = json!("other-increase");
    other["children"][1]["children"][0]["id"] = json!("other-label");
    document["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(other);
    sink.state = EditorState::from_document(serde_json::from_value(document).unwrap());
    let mut task = plan.subtasks[0].clone();
    task.id = "other-selector".into();
    task.id_prefix = "other-selector".into();
    plan.subtasks.push(task);
    let mut outcome = outcomes[0].clone();
    outcome.id = "other-selector".into();
    outcome.inserted_root_ids = vec!["other-original".into()];
    outcomes.push(outcome);
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(proposal(|_| {}))]);
    let (failed, _) = repair(
        &mut sink,
        &plan,
        &request,
        &mut outcomes,
        &llm,
        &AbortFlag::new(),
    );
    assert!(failed);
    assert_eq!(llm.user_prompts().len(), 1);
    assert!(outcomes[0].error.is_none());
    assert!(outcomes[1].error.as_deref().unwrap().contains(UNRESOLVED));
    assert_eq!(outcomes[1].node_count, 1);
    assert_eq!(outcomes[1].inserted_root_ids, vec!["other-original"]);
}

#[test]
fn styled_initial_copy_is_verified_using_the_original_segments() {
    for proposed in [1, 2] {
        let (mut sink, plan, request, mut outcomes) = fixture();
        let mut document = serde_json::to_value(&sink.state.doc).unwrap();
        document["children"][0]["children"][1]["children"][0]["content"] = json!([
            {"text":"1","fontWeight":700}
        ]);
        sink.state = EditorState::from_document(serde_json::from_value(document).unwrap());
        let before = sink.state.doc.clone();
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(proposal(|root| {
            root["state"]["units"]["default"] = json!(proposed);
        }))]);
        let failed = repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new(),
        )
        .0;
        assert_eq!(failed, proposed != 1);
        assert_eq!(sink.state.doc.children, before.children);
        if failed {
            assert_eq!(sink.state.doc, before)
        }
    }
}

fn two_quantity_fixture() -> (
    VecDocSink,
    OrchestratorPlan,
    DesignRequest,
    Vec<SubtaskOutcome>,
) {
    let (mut sink, plan, request, outcomes) = fixture();
    let rows: Vec<_> = ["qty1", "qty2"].into_iter().map(|key| json!({
        "type":"frame","id":format!("{key}-row"),"layout":"horizontal",
        "children":[
            {"type":"text","id":format!("{key}-value"),"content":"1",
             "bindings":{"content":format!("to_str($app.{key})")}},
            {"type":"frame","id":format!("{key}-increase"),
             "events":{"onTap":[{"set":{format!("$app.{key}"):format!("$app.{key} + 1")}}]}},
            {"type":"frame","id":format!("{key}-decrease"),
             "events":{"onTap":[{"set":{format!("$app.{key}"):format!("$app.{key} - 1")}}]}}
        ]
    })).collect();
    let mut document = serde_json::to_value(&sink.state.doc).unwrap();
    document["children"][0]["children"][1]["children"] = json!(rows);
    sink.state = EditorState::from_document(serde_json::from_value(document).unwrap());
    assert_eq!(
        missing_initial_bindings(&sink.state, &outcomes[0].inserted_root_ids).len(),
        2
    );
    (sink, plan, request, outcomes)
}

fn two_quantity_proposal() -> Value {
    json!({"state":{
        "qty1":{"type":"number","default":1},
        "qty2":{"type":"number","default":1}
    }})
}

#[test]
fn two_quantity_defaults_preserve_original_text_bindings_and_stepper_actions() {
    let (mut sink, plan, request, mut outcomes) = two_quantity_fixture();
    let before = sink.state.doc.children.clone();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        two_quantity_proposal().to_string(),
    )]);
    assert!(
        !repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new()
        )
        .0
    );
    assert_eq!(llm.user_prompts().len(), 1);
    assert_eq!(sink.state.doc.children, before);
    assert_eq!(outcomes[0].inserted_root_ids, vec!["original"]);
    assert_eq!(outcomes[0].node_count, 1);
    assert_eq!(sink.applied.len(), 1);
    let EditorCommand::Batch { commands } = &sink.applied[0] else {
        panic!("expected one atomic state batch")
    };
    assert!(commands
        .iter()
        .all(|command| matches!(command, EditorCommand::MergeAppState { .. })));
    let state = sink.state.doc.state.as_ref().unwrap();
    assert_eq!(state.len(), 2);
    for key in ["qty1", "qty2"] {
        assert_eq!(state[key].default, Some(json!(1)));
    }
    assert!(missing_initial_bindings(&sink.state, &outcomes[0].inserted_root_ids).is_empty());
}

#[test]
fn invalid_second_quantity_never_partially_commits_the_first_default() {
    for second in [None, Some(json!("1")), Some(json!(2)), Some(Value::Null)] {
        let (mut sink, plan, request, mut outcomes) = two_quantity_fixture();
        let before = sink.state.doc.clone();
        let mut candidate = two_quantity_proposal();
        if let Some(value) = second {
            candidate["state"]["qty2"]["default"] = value;
        } else {
            candidate["state"].as_object_mut().unwrap().remove("qty2");
        }
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(candidate.to_string())]);
        assert!(
            repair(
                &mut sink,
                &plan,
                &request,
                &mut outcomes,
                &llm,
                &AbortFlag::new()
            )
            .0
        );
        assert_eq!(llm.user_prompts().len(), 1);
        assert_eq!(sink.state.doc, before);
        assert!(sink.applied.is_empty());
        assert_eq!(outcomes[0].inserted_root_ids, vec!["original"]);
        assert_eq!(outcomes[0].node_count, 1);
        assert!(outcomes[0].subtask.is_none());
        assert!(outcomes[0].error.as_deref().unwrap().contains(UNRESOLVED));
    }
}

#[test]
fn one_format_correction_can_supply_a_valid_state_only_proposal() {
    let (mut sink, plan, request, mut outcomes) = fixture();
    let before = sink.state.doc.children.clone();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(proposal(|value| value["nodes"] = json!([section()]))),
        ScriptResponse::Text(proposal(|_| {})),
    ]);
    assert!(
        !repair(
            &mut sink,
            &plan,
            &request,
            &mut outcomes,
            &llm,
            &AbortFlag::new()
        )
        .0
    );
    assert_eq!(llm.user_prompts().len(), 2);
    assert_eq!(sink.state.doc.children, before);
    assert_eq!(sink.applied.len(), 1);
    assert_eq!(
        sink.state.doc.state.as_ref().unwrap()["units"].default,
        Some(json!(1))
    );
    assert!(outcomes[0].error.is_none());
}

#[test]
fn transport_or_abort_errors_do_not_trigger_format_correction() {
    for aborted in [false, true] {
        let (mut sink, plan, request, mut outcomes) = fixture();
        let before = sink.state.doc.clone();
        let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
            message: "proposal stream failed".into(),
            aborted,
        })]);
        assert!(
            repair(
                &mut sink,
                &plan,
                &request,
                &mut outcomes,
                &llm,
                &AbortFlag::new()
            )
            .0
        );
        assert_eq!(llm.user_prompts().len(), 1);
        assert_eq!(sink.state.doc, before);
        assert!(sink.applied.is_empty());
    }
}

#[path = "run_initial_binding_feedback_guard_tests.rs"]
mod guards;
