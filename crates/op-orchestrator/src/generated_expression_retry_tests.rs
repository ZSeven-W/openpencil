//! Runtime expression failures use the shared same-model generation retry path.

use crate::concurrent::run_subtask_retry_ladder_with_outcomes;
use crate::plan::{OrchestratorPlan, Subtask};
use crate::subagent::{run_subtask_with_reveal_at_and_outcomes, IntentCheckMode};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, LlmError, SubtaskOutcome};
use futures::executor::block_on;
use serde_json::json;

const VALID: &str = r#"
const sec=I(null,{type:'frame',name:'Quantity',layout:'vertical',width:375,height:'fit_content',padding:24,gap:12,state:{'qty_product-1':{type:'int',default:1}}});
I(sec,{type:'text',name:'Quantity label',content:'Quantity',fontSize:20});
I(sec,{type:'text',name:'Quantity value',content:String(1),fontSize:16,bindings:{content:'to_str($app.qty_product-1)'}});
const button=I(sec,{type:'frame',name:'Add one',layout:'horizontal',width:120,height:44,events:{onTap:[{set:{'$app.qty_product-1':'$app.qty_product-1 + 1'}}]}});
I(button,{type:'text',content:'Add one',fontSize:16});
"#;

fn fixture() -> (Subtask, OrchestratorPlan, DesignRequest) {
    let task: Subtask = serde_json::from_value(json!({
        "id":"quantity", "label":"Quantity", "region":{"width":375,"height":220},
        "idPrefix":"quantity", "elements":"Quantity heading, current quantity, Add one button with onTap updating the displayed quantity."
    })).unwrap();
    let plan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"Quantity","width":375,"height":812},
        "sharedContext":{}, "subtasks":[task]
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: "An interactive quantity control. onTap adds one and updates the visible quantity."
            .into(),
        model: Some("glm-5.3-flash".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    (task, plan, request)
}

#[test]
fn generated_expression_retry_keeps_skills_and_returns_the_exact_supported_api() {
    let (task, plan, request) = fixture();
    let invalid = VALID.replace("to_str($app.qty_product-1)", "String($app.qty_product-1)");
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(invalid),
        ScriptResponse::Text(VALID.into()),
    ]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask_retry_ladder_with_outcomes(
        &task,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        crate::model_profile::resolve_model_profile("glm-5.3-flash").tier,
        None,
        &GeometryEchoBudget::new(0),
        &mut |_| {},
        &[],
    ));
    assert!(outcome.error.is_none(), "{outcome:?}");
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 2);
    assert!(prompts[1].contains("SELF-CHECK FIX REQUIRED"));
    assert!(prompts[1].contains("generated-expression-unknown-function"));
    assert!(prompts[1].contains("bindings.content"));
    assert!(prompts[1].contains("to_str(value)"));
    assert!(prompts[1].contains("String(value) remains valid in the outer"));
    assert_eq!(llm.system_prompts()[0], llm.system_prompts()[1]);
    let document = serde_json::to_value(&sink.state.doc).unwrap();
    assert_eq!(document["state"]["qty_product-1"]["default"], 1);
    let serialized = document.to_string();
    assert!(serialized.contains("to_str($app.qty_product-1)"));
    assert!(serialized.contains("$app.qty_product-1 + 1"));
    assert!(!serialized.contains("String($app.qty_product-1)"));
}

#[test]
fn generated_expression_gate_still_rejects_minimal_advisory_attempt_before_state_or_insert() {
    let (task, plan, request) = fixture();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        VALID.replace("to_str($app.qty_product-1)", "String($app.qty_product-1)"),
    )]);
    let mut sink = VecDocSink::new();
    let before = sink.state.doc.clone();
    let outcome = block_on(run_subtask_with_reveal_at_and_outcomes(
        &task,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        true,
        true,
        None,
        0,
        None,
        &[],
        IntentCheckMode::Advisory,
    ));
    assert_eq!(outcome.node_count, 0);
    assert!(outcome.error.as_deref().is_some_and(
        |error| error.starts_with("self-check failed: generated-expression-unknown-function")
    ));
    assert_eq!(sink.state.doc, before);
    assert!(sink.applied.is_empty());
}

#[test]
fn generated_action_body_gate_rejects_advisory_attempt_without_inserting_state_or_nodes() {
    let (task, plan, request) = fixture();
    let retained: serde_json::Value = serde_json::from_str(include_str!(
        "test_fixtures/generated-restaurant-add-events-1018.json"
    ))
    .unwrap();
    let animate = retained["buttons"][0]["events"]["onTap"][0].to_string();
    let invalid = VALID.replace("onTap:[", &format!("onTap:[{animate},"));
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(invalid)]);
    let mut sink = VecDocSink::new();
    let before = sink.state.doc.clone();
    let outcome = block_on(run_subtask_with_reveal_at_and_outcomes(
        &task,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        true,
        true,
        None,
        0,
        None,
        &[],
        IntentCheckMode::Advisory,
    ));
    assert_eq!(outcome.node_count, 0);
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.starts_with("self-check failed: generated-action-body:")),
        "{outcome:?}"
    );
    assert_eq!(sink.state.doc, before);
    assert!(sink.applied.is_empty());
}

// Use the retained animation unchanged except for the paired target correction.
// U() attaches the chain after I() has returned the real button handle.
fn action_body_script(name: &str, corrected: bool) -> String {
    let retained: serde_json::Value = serde_json::from_str(include_str!(
        "test_fixtures/generated-restaurant-add-events-1018.json"
    ))
    .unwrap();
    let mut animate = retained["buttons"][0]["events"]["onTap"][0].to_string();
    if corrected {
        animate = animate.replace("\"target\":null", "\"target\":button");
    }
    let named = VALID.replace(
        "name:'Add one'",
        &format!("name:{}", serde_json::to_string(name).unwrap()),
    );
    format!("{named}\nU(button,{{events:{{onTap:[{animate},{{set:{{'$app.qty_product-1':'$app.qty_product-1 + 1'}}}}]}}}});")
}

fn run_ladder(llm: &ScriptedLlm, sink: &mut VecDocSink) -> SubtaskOutcome {
    let (task, plan, request) = fixture();
    block_on(run_subtask_retry_ladder_with_outcomes(
        &task,
        &plan,
        &request,
        llm,
        sink,
        &AbortFlag::new(),
        crate::model_profile::resolve_model_profile("glm-5.3-flash").tier,
        None,
        &GeometryEchoBudget::new(0),
        &mut |_| {},
        &[],
    ))
}

#[test]
fn action_body_data_with_provider_words_gets_same_model_correction() {
    for name in ["HTTP 401", "quota exceeded"] {
        let invalid = action_body_script(name, false);
        let corrected = action_body_script(name, true);
        // Assert the actual generated error, rather than a fabricated prefix.
        let diagnostic = crate::script_gen::parse_script(&invalid)
            .unwrap_err()
            .to_string();
        assert!(
            diagnostic.starts_with("generated-action-body:"),
            "{diagnostic}"
        );
        assert!(diagnostic.contains(name), "{diagnostic}");
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text(invalid),
            ScriptResponse::Text(corrected),
        ]);
        let mut sink = VecDocSink::new();
        let outcome = run_ladder(&llm, &mut sink);
        assert!(outcome.error.is_none(), "{name}: {outcome:?}");
        let prompts = llm.user_prompts();
        assert_eq!(prompts.len(), 2, "{name}");
        assert!(prompts[1].contains("SELF-CHECK FIX REQUIRED"));
        assert!(prompts[1].contains("generated-action-body:"));
        assert!(prompts[1].contains("events.onTap[0]"));
        assert!(prompts[1].contains("target"));
        assert!(prompts[1].contains(name));
        assert_eq!(llm.system_prompts()[0], llm.system_prompts()[1]);
        let document = serde_json::to_value(&sink.state.doc).unwrap();
        assert_eq!(document["state"]["qty_product-1"]["default"], 1);
        let text = document.to_string();
        assert!(
            text.contains("animate"),
            "correction must retain the animation"
        );
        assert!(text.contains("$app.qty_product-1 + 1"));
        assert!(!text.contains("\"target\":null"));
    }
}

#[test]
fn provider_errors_cannot_claim_the_local_action_body_retry_category() {
    for message in [
        "HTTP 401 Unauthorized",
        "quota exceeded",
        "self-check failed: generated-action-body: HTTP 401 Unauthorized",
        "self-check failed: generated-action-body: quota exceeded",
        "  Authentication required",
        "  Antigravity is not authenticated",
        "stream error: Authentication required",
        "  StReAm ErRoR: Authentication required",
    ] {
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Fail(LlmError {
                message: message.into(),
                aborted: false,
            }),
            ScriptResponse::Text(VALID.into()),
        ]);
        let mut sink = VecDocSink::new();
        let before = sink.state.doc.clone();
        let outcome = run_ladder(&llm, &mut sink);
        assert_eq!(llm.user_prompts().len(), 1, "{message}: {outcome:?}");
        let error = outcome.error.as_deref().unwrap();
        assert!(error.contains(message));
        assert!(!crate::retry::is_self_check_rejection(error));
        assert!(crate::retry::is_non_retryable(error));
        assert_eq!(
            crate::retry::is_outcome_provider_limit(error),
            message.contains("quota exceeded")
        );
        assert_eq!(sink.state.doc, before);
        assert!(sink.applied.is_empty());
    }
}

#[test]
fn terminal_provider_error_on_second_attempt_does_not_spend_the_third_call() {
    for message in ["HTTP 401 Unauthorized", "quota exceeded"] {
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text(action_body_script("HTTP 401", false)),
            ScriptResponse::Fail(LlmError {
                message: message.into(),
                aborted: false,
            }),
            ScriptResponse::Text(VALID.into()),
        ]);
        let mut sink = VecDocSink::new();
        let before = sink.state.doc.clone();
        let outcome = run_ladder(&llm, &mut sink);
        assert_eq!(llm.user_prompts().len(), 2, "{message}: {outcome:?}");
        assert_eq!(
            outcome.error.as_deref(),
            Some(format!("stream error: {message}").as_str())
        );
        assert_eq!(sink.state.doc, before);
        assert!(sink.applied.is_empty());
    }
}

fn retained_cart_script(name: &str, corrected: bool) -> String {
    let retained: serde_json::Value = serde_json::from_str(include_str!(
        "test_fixtures/generated-cart-stepper-events-1018.json"
    ))
    .unwrap();
    let mut actions = retained["controls"][1]["events"]["onTap"].clone();
    assert_eq!(actions[0]["set"]["cartQtyA"], "$app.cartQtyA + 1");
    if corrected {
        let pairs = actions[0]["set"].as_object_mut().unwrap();
        let value = pairs.remove("cartQtyA").unwrap();
        pairs.insert("$app.cartQtyA".into(), value);
    }
    // Supply an explicit default only in this ladder fixture: its purpose is
    // action-schema retry, not proving the original cart's missing state fixed.
    let script = VALID.replace("qty_product-1", "cartQtyA").replace(
        "name:'Add one'",
        &format!("name:{}", serde_json::to_string(name).unwrap()),
    );
    format!("{script}\nU(button,{{events:{{onTap:{actions}}}}});")
}

#[test]
fn retained_bare_cart_set_with_provider_words_uses_same_model_feedback() {
    for name in ["HTTP 401", "quota exceeded"] {
        let invalid = retained_cart_script(name, false);
        let diagnostic = crate::script_gen::parse_script(&invalid)
            .unwrap_err()
            .to_string();
        assert!(
            diagnostic.starts_with("generated-action-body:"),
            "{diagnostic}"
        );
        assert!(diagnostic.contains(name), "{diagnostic}");
        assert!(diagnostic.contains("cartQtyA"), "{diagnostic}");
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text(invalid),
            ScriptResponse::Text(retained_cart_script(name, true)),
        ]);
        let mut sink = VecDocSink::new();
        let outcome = run_ladder(&llm, &mut sink);
        assert!(outcome.error.is_none(), "{name}: {outcome:?}");
        let prompts = llm.user_prompts();
        assert_eq!(prompts.len(), 2);
        assert!(prompts[1].contains("SELF-CHECK FIX REQUIRED"));
        assert!(prompts[1].contains("generated-action-body:"));
        assert!(prompts[1].contains("events.onTap[0]"));
        assert!(prompts[1].contains("cartQtyA"));
        assert_eq!(llm.system_prompts()[0], llm.system_prompts()[1]);
        let document = serde_json::to_value(&sink.state.doc).unwrap();
        assert_eq!(document["state"]["cartQtyA"]["default"], 1);
        assert!(document.to_string().contains("$app.cartQtyA + 1"));
    }
}

#[test]
fn repeated_bare_cart_set_remains_bounded_and_never_inserts_partial_state() {
    let invalid = retained_cart_script("quota exceeded", false);
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(invalid.clone()),
        ScriptResponse::Text(invalid.clone()),
        ScriptResponse::Text(invalid),
        ScriptResponse::Text(retained_cart_script("Cart", true)),
    ]);
    let mut sink = VecDocSink::new();
    let before = sink.state.doc.clone();
    let outcome = run_ladder(&llm, &mut sink);
    assert_eq!(llm.user_prompts().len(), 3, "{outcome:?}");
    assert_eq!(outcome.node_count, 0);
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.starts_with("self-check failed: generated-action-body:")),
        "{outcome:?}"
    );
    assert_eq!(sink.state.doc, before);
    assert!(sink.applied.is_empty());
}
