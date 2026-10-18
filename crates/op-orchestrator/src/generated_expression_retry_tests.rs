//! Runtime expression failures use the shared same-model generation retry path.

use crate::concurrent::run_subtask_retry_ladder_with_outcomes;
use crate::plan::{OrchestratorPlan, Subtask};
use crate::subagent::{run_subtask_with_reveal_at_and_outcomes, IntentCheckMode};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget};
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
