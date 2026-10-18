//! Retained checkout-label regression through parsing and subtask retry.

use super::{parse_script, ScriptGenError};
use crate::plan::{OrchestratorPlan, Subtask};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use futures::executor::block_on;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde_json::json;

const RETAINED: &str = include_str!("test_fixtures/product-cart-summary-parent-1018.js");

fn corrected_parent_only() -> String {
    RETAINED
        .replace(
            "I(bar, {type:\"frame\", name:\"结算按钮\"",
            "const checkout = I(bar, {type:\"frame\", name:\"结算按钮\"",
        )
        .replace("I(\"结算按钮\",", "I(checkout,")
}

fn corrected() -> String {
    // Keep the retained bytes unchanged while repairing the independent toast
    // expression required by the successful parent-handle replay.
    corrected_parent_only().replace(
        r#"toast:"已提交结算 (2) 件商品""#,
        r#"toast:"'已提交结算 (2) 件商品'""#,
    )
}

fn find_named<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a PenNode> {
    nodes.iter().find_map(|node| {
        if node.base().name.as_deref() == Some(name) {
            Some(node)
        } else {
            find_named(node.children().map(Vec::as_slice).unwrap_or(&[]), name)
        }
    })
}

fn assert_checkout_label_is_a_child(nodes: &[PenNode]) {
    let button = find_named(nodes, "结算按钮").expect("checkout button");
    let children = button.children().expect("button children");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].base().name.as_deref(), Some("结算按钮文字"));
    let text = serde_json::to_value(&children[0]).unwrap();
    assert_eq!(text["content"], "结算 (2)");
}

#[test]
fn retained_parent_name_fails_atomically_and_corrected_handle_nests_the_label() {
    let error = parse_script(RETAINED).unwrap_err();
    assert!(matches!(&error, ScriptGenError::ParentReference(_)));
    assert!(error.to_string().contains("结算按钮"), "{error}");
    let error = parse_script(&corrected_parent_only()).unwrap_err();
    assert!(matches!(&error, ScriptGenError::RuntimeExpression(_)));
    assert!(error.to_string().contains("events.onTap[0].toast"));
    let (nodes, _) = parse_script(&corrected()).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_checkout_label_is_a_child(&nodes);
}

#[test]
fn authored_ids_were_not_parent_handles_in_the_scratch_executor() {
    let source = r#"const sec=I(null,{type:"frame",name:"Section"});
I(sec,{id:"authored-button",type:"frame",name:"Button"});
I("authored-button",{type:"text",name:"Label",content:"Checkout"});"#;
    // The existing pure-insert executor stamps bN bindings over authored ids.
    // Its external-parent fallback previously left this label at the root.
    let old_program = op_mcp::script_runner::run_script_to_program(source).unwrap();
    let (old_nodes, _) = crate::program_gen::run_program_to_forest(&old_program).unwrap();
    assert_eq!(old_nodes.len(), 2);
    let button = find_named(&old_nodes, "Button").unwrap();
    assert_ne!(button.id_str(), "authored-button");
    assert!(button.children().is_none_or(Vec::is_empty));
    assert_eq!(old_nodes[1].base().name.as_deref(), Some("Label"));
    assert!(matches!(
        parse_script(source),
        Err(ScriptGenError::ParentReference(_))
    ));
}

#[test]
fn explicit_null_state_declaration_still_hoists_without_an_external_parent() {
    let (nodes, state) = parse_script(
        r#"I(null,{type:"frame",name:"State declaration",state:{quantity:{type:"int",default:2}}});
const sec=I(null,{type:"frame",name:"Summary"});
const button=I(sec,{type:"frame",name:"Checkout"});
I(button,{type:"text",content:"Checkout",bindings:{content:"$app.quantity"}});"#,
    )
    .unwrap();
    assert!(state.contains_key("quantity"));
    assert!(find_named(&nodes, "Checkout")
        .unwrap()
        .children()
        .is_some_and(|children| children.len() == 1));
}

fn fixture() -> (Subtask, OrchestratorPlan, DesignRequest) {
    let task: Subtask = serde_json::from_value(json!({
        "id":"cart-summary", "label":"购物车页-金额汇总与结算",
        "region":{"width":375,"height":220}, "idPrefix":"cart-summary"
    }))
    .unwrap();
    let plan: OrchestratorPlan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"购物车","width":375,"height":812},
        "sharedContext":{}, "subtasks":[task]
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: "购物车金额汇总和结算按钮，中文界面".into(),
        model: None,
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
fn rejected_parent_does_not_insert_a_partial_section_or_leak_declared_state() {
    let (task, plan, request) = fixture();
    let source = format!(
        "I(null,{{type:'frame',state:{{pending:{{type:'bool',default:true}}}}}});\n{RETAINED}"
    );
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(source)]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(crate::subagent::run_subtask(
        &task,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert_eq!(outcome.node_count, 0);
    let error = outcome.error.unwrap();
    assert!(error.starts_with("self-check failed:"), "{error}");
    assert!(error.contains("OP_SCRIPT_PARENT_UNRESOLVED"), "{error}");
    assert!(
        sink.applied.is_empty(),
        "no commands from the rejected script"
    );
    assert!(sink
        .state
        .doc
        .state
        .as_ref()
        .is_none_or(|state| !state.contains_key("pending")));
}

#[test]
fn retry_receives_the_exact_parent_error_and_keeps_the_corrected_label_inside() {
    let (task, plan, request) = fixture();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(RETAINED.into()),
        ScriptResponse::Text(corrected()),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let outcome = block_on(crate::concurrent::run_subtask_retry_ladder_with_outcomes(
        &task,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        crate::model_profile::ModelTier::Basic,
        None,
        &GeometryEchoBudget::new(0),
        &mut |event| events.push(event),
        &[],
    ));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(llm.user_prompts().len(), 2);
    let error = format!("self-check failed: {}", parse_script(RETAINED).unwrap_err());
    assert!(llm.user_prompts()[1].contains(&error));
    assert!(events.iter().any(|event| matches!(event, Progress::SubtaskRetry { attempt:2, reason, .. } if reason == &error)));
    assert_eq!(
        llm.system_prompts()[0],
        llm.system_prompts()[1],
        "parent correction must retain the original skill tier"
    );
    assert_checkout_label_is_a_child(sink.state.active_children());
}
