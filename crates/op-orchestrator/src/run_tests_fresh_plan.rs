//! Fresh response protocol checks run before normalization and section work.

use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm};
use serde_json::json;

fn request() -> DesignRequest {
    DesignRequest {
        prompt: "Create a two-screen catalog and detail flow. The brand is Luma.".into(),
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
    }
}

fn plan_value() -> serde_json::Value {
    json!({
        "rootFrame":{"id":"root","name":"Page","width":375,"height":812},
        "subtasks":[
            {"id":"s1","label":"Gallery","elements":"subject photo","screen":"Catalog","region":{"width":375,"height":240}},
            {"id":"s2","label":"Information","elements":"subject name and price","screen":"Detail","region":{"width":375,"height":240}}
        ],
        "sharedContext":{
            "sourceFacts":["The brand is Luma.","The price is ¥169."],
            "exampleEntities":[{"id":"item","name":"Linen tote","imageSubject":"oat linen tote bag",
                "attributes":{"price":"¥169","selectedSize":"Large"},"usedBy":["s1","s2"]}]
        }
    })
}

#[test]
fn fresh_plan_missing_field_retries_with_precise_feedback_and_accepts_explicit_empty() {
    let mut old = plan_value();
    old.as_object_mut().unwrap().remove("sharedContext");
    let mut corrected = old.clone();
    corrected["sharedContext"] = json!({});
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(old.to_string()),
        ScriptResponse::Text(corrected.to_string()),
    ]);
    let (plan, _) = futures::executor::block_on(planning_loop(
        &request(),
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
    ))
    .unwrap();
    assert!(plan.shared_context.is_none());
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 2);
    assert!(!prompts[0].contains("PLAN PROTOCOL CORRECTION REQUIRED"));
    assert!(prompts[1].contains("missing top-level sharedContext"));
    assert!(prompts[1].contains("Keep the requested content, pages and dimensions"));
}

#[test]
fn fresh_plan_stale_id_retries_before_sections_and_filters_sources_on_acceptance() {
    let mut wrong = plan_value();
    wrong["sharedContext"]["exampleEntities"][0]["usedBy"] = json!(["s1", "price-spec"]);
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(wrong.to_string()),
        ScriptResponse::Text(plan_value().to_string()),
    ]);
    let (plan, _) = futures::executor::block_on(planning_loop(
        &request(),
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
    ))
    .unwrap();
    let context = plan.shared_context.unwrap();
    assert_eq!(context.source_facts, ["The brand is Luma."]);
    assert_eq!(context.example_entities[0].used_by, ["s1", "s2"]);
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 2);
    assert!(prompts[1].contains("price-spec") && prompts[1].contains("undeclared section"));
    for expected in [
        "validFinalSubtaskIds",
        "s1",
        "s2",
        "Check every",
        "Correct stale usedBy references",
        "Do not rename final subtask IDs",
    ] {
        assert!(prompts[1].contains(expected), "missing {expected}");
    }
}

#[test]
fn fresh_plan_leaf_type_failure_retries_with_path_without_replacing_context() {
    let mut wrong = plan_value();
    wrong["sharedContext"]["exampleEntities"][0]["attributes"]["colors"] = json!(["oat", "green"]);
    let mut corrected = wrong.clone();
    corrected["sharedContext"]["exampleEntities"][0]["attributes"]["colors"] = json!("oat / green");
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(wrong.to_string()),
        ScriptResponse::Text(corrected.to_string()),
    ]);
    let (plan, _) = futures::executor::block_on(planning_loop(
        &request(),
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
    ))
    .unwrap();
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 2);
    for expected in [
        "$.sharedContext.exampleEntities[0].attributes.colors",
        "string",
        "array",
        "Keep sharedContext as an object",
    ] {
        assert!(prompts[1].contains(expected), "missing {expected}");
    }
    assert_eq!(
        serde_json::to_value(plan.shared_context.unwrap().example_entities).unwrap(),
        corrected["sharedContext"]["exampleEntities"]
    );
}

#[test]
fn fresh_plan_repeated_protocol_errors_never_accept_the_invalid_context() {
    let mut wrong = plan_value();
    wrong["sharedContext"]["exampleEntities"][0]["usedBy"] = json!(["missing"]);
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(wrong.to_string()),
        ScriptResponse::Text(wrong.to_string()),
    ]);
    let (plan, _) = futures::executor::block_on(planning_loop(
        &request(),
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
    ))
    .unwrap();
    // The invalid context is dropped, never accepted; the modelled sections
    // survive instead of being replaced by the heuristic fallback skeleton.
    assert!(plan.shared_context.is_none());
    let ids: Vec<_> = plan.subtasks.iter().map(|task| task.id.as_str()).collect();
    assert_eq!(ids, ["s1", "s2"]);
    assert_eq!(llm.user_prompts().len(), 2);
}

// 1019 A/B: GLM-5.3-Flash twice put a stray key into sharedContext (`usedBy`,
// `fonts`, `note`); the heuristic fallback then lost the calendar header.
#[test]
fn fresh_plan_unknown_shared_context_field_keeps_the_modelled_sections() {
    let mut wrong = plan_value();
    wrong["sharedContext"]["fonts"] = json!(["Inter"]);
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(wrong.to_string()),
        ScriptResponse::Text(wrong.to_string()),
    ]);
    let (plan, _) = futures::executor::block_on(planning_loop(
        &request(),
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
    ))
    .unwrap();
    assert!(plan.shared_context.is_none());
    let labels: Vec<_> = plan
        .subtasks
        .iter()
        .map(|task| task.label.as_str())
        .collect();
    assert_eq!(labels, ["Gallery", "Information"]);
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 2);
    assert!(prompts[1].contains("unknown field `fonts`"));
}

#[test]
fn fresh_plan_unparseable_twice_still_uses_the_heuristic_fallback() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text("not a plan".into()),
        ScriptResponse::Text("still not a plan".into()),
    ]);
    let (plan, _) = futures::executor::block_on(planning_loop(
        &request(),
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
    ))
    .unwrap();
    assert!(plan.subtasks.iter().all(|task| task.label != "Gallery"));
}

#[test]
fn fresh_plan_retained_multi_error_response_is_corrected_without_reselecting_subjects() {
    const RAW: &str =
        include_str!("fixtures/planner_protocol_1018/product-review-count-attempt1.json");
    let mut corrected: serde_json::Value = serde_json::from_str(RAW).unwrap();
    let entities = corrected["sharedContext"]["exampleEntities"]
        .as_array_mut()
        .unwrap();
    entities[0]["attributes"]["colors"] = json!("米白 / 燕麦 / 雾蓝");
    entities[0]["attributes"]["sizes"] = json!("S / M / L / XL");
    for entity in entities {
        for id in entity["usedBy"].as_array_mut().unwrap() {
            if id == "cart-page" {
                *id = json!("cart-items");
            }
        }
    }
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(RAW.into()),
        ScriptResponse::Text(corrected.to_string()),
    ]);
    let (plan, _) = futures::executor::block_on(planning_loop(
        &request(),
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
    ))
    .unwrap();
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 2);
    for expected in [
        "attributes.colors",
        "attributes.sizes",
        "cart-page",
        "cart-items",
        "Repair this plan, do not redesign it",
    ] {
        assert!(prompts[1].contains(expected), "missing {expected}");
    }
    assert_eq!(
        serde_json::to_value(
            plan.shared_context
                .as_ref()
                .unwrap()
                .example_entities
                .clone()
        )
        .unwrap(),
        corrected["sharedContext"]["exampleEntities"]
    );
    for task in &plan.subtasks {
        let mut prompt = String::new();
        crate::plan_shared_context::append_instruction(&plan, &request(), task, &mut prompt);
        for expected in [
            "云感针织卫衣",
            "复古直筒牛仔裤",
            "¥299",
            "¥399",
            "米白 / 燕麦 / 雾蓝",
            "S / M / L / XL",
        ] {
            assert!(prompt.contains(expected), "{} lost {expected}", task.id);
        }
    }
}

#[test]
fn fresh_plan_coverage_replan_cannot_replace_an_accepted_contract_with_missing_context() {
    let request = request();
    let (original, _) =
        parse_fresh_orchestrator_response(&plan_value().to_string(), &request).unwrap();
    let expected = original.shared_context.clone();
    let mut invalid = plan_value();
    invalid.as_object_mut().unwrap().remove("sharedContext");
    invalid["rootFrame"]["name"] = json!("Rejected replacement");
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(invalid.to_string())]);
    let retained = futures::executor::block_on(replan_once_for_coverage(
        &request,
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
        original,
        &["Pricing".into()],
    ))
    .unwrap();
    assert_eq!(retained.shared_context, expected);
    assert_eq!(retained.root_frame.name, "Page");
    assert_eq!(llm.user_prompts().len(), 1);
}

#[test]
fn fresh_plan_source_copy_replan_rejects_stale_entity_refs_even_with_complete_copy() {
    const FIRST: &str = "先做1页试稿，核对原始资料，再决定是否扩展为多页。";
    const SECOND: &str = "正文不要靠缩小字号塞进版面；可以调整留白、行长和分组。";
    let request = DesignRequest {
        prompt: format!(
            "做2张1080×1440中文图文卡。保留全部原文，以下内容逐条分配。\n{FIRST}\n{SECOND}"
        ),
        ..request()
    };
    let mut original = plan_value();
    original["rootFrame"]["width"] = json!(1080);
    original["rootFrame"]["height"] = json!(1440);
    original["subtasks"][0]["elements"] = json!("先做一页");
    original["subtasks"][1]["elements"] = json!(SECOND);
    for task in original["subtasks"].as_array_mut().unwrap() {
        task["region"] = json!({"width":1080,"height":1440});
    }
    let (original_plan, _) =
        parse_fresh_orchestrator_response(&original.to_string(), &request).unwrap();
    let mut corrected_copy = original;
    corrected_copy["subtasks"][0]["elements"] = json!(FIRST);
    corrected_copy["sharedContext"]["exampleEntities"][0]["usedBy"] = json!(["stale-section"]);
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(corrected_copy.to_string())]);
    let result = futures::executor::block_on(source_copy_plan::ensure(
        &request,
        &llm,
        &AbortFlag::new(),
        &mut |_| {},
        original_plan,
    ));
    assert!(
        matches!(result, Err(OrchestratorError::AllFailed(reason)) if reason.contains("source-copy-missing") && reason.contains("stale-section") && reason.contains("undeclared section"))
    );
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("SOURCE COPY FIX REQUIRED"));
}
