use super::*;
use crate::plan::{parse_plan, RetryFeedback};
use crate::prompt::{build_orchestrator_prompt, build_subagent_prompt};
use crate::types::{AbortFlag, PlanningMode};
use op_editor_core::ComponentLibrary;
use serde_json::{json, Value};

const BRIEF: &str = "Design a 375×812 mobile product detail and cart flow. Include product photos, a price, selected color and size, two populated cart items and a total. The brand is Luma.";

fn context() -> Value {
    json!({
        "sourceFacts":["The brand is Luma."],
        "exampleEntities":[
            {"id":"main-product","name":"Sage linen midi dress",
             "imageSubject":"sage-green linen midi dress, identical garment and fabric in every gallery angle and cart thumbnail",
             "attributes":{"price":"¥329","selectedSpecification":"Sage green / M"},
             "usedBy":["gallery","info","options","cart"]},
            {"id":"second-product","name":"Linen scarf",
             "imageSubject":"oat-colored linen scarf",
             "attributes":{"price":"¥128","selectedSpecification":"Oat / One size"},
             "usedBy":["cart"]}
        ]
    })
}

fn plan_value() -> Value {
    let subtasks: Vec<_> = [
        ("gallery", "Product Gallery", "Product"),
        ("info", "Product Information", "Product"),
        ("options", "Product Options", "Product"),
        ("cart", "Cart Items", "Cart"),
        ("summary", "Cart Summary", "Cart"),
    ]
    .into_iter()
    .map(|(id, label, screen)| {
        json!({
            "id":id,"label":label,"screen":screen,"region":{"width":375,"height":140}
        })
    })
    .collect();
    json!({
        "rootFrame":{"id":"product","name":"Shop","width":375,"height":812},
        "subtasks":subtasks,"sharedContext":context()
    })
}

fn request() -> DesignRequest {
    DesignRequest {
        prompt: BRIEF.into(),
        model: Some("glm-5.3-flash".into()),
        provider: None,
        design_md: None,
        concurrency: 2,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

fn block(prompt: &str) -> &str {
    prompt
        .split_once("SHARED CONTENT CONTEXT —")
        .unwrap()
        .1
        .split_once("\nEND SHARED CONTENT CONTEXT")
        .unwrap()
        .0
}

#[test]
fn shared_context_old_json_and_empty_optional_context_remain_compatible() {
    for empty in [None, Some(Value::Null), Some(json!({}))] {
        let mut value = plan_value();
        value.as_object_mut().unwrap().remove("sharedContext");
        if let Some(empty) = empty {
            value["sharedContext"] = empty;
        }
        let plan = parse_plan(&value.to_string()).unwrap();
        assert!(plan.shared_context.is_none());
        assert!(!serde_json::to_value(plan)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("sharedContext"));
    }
}

#[test]
fn shared_context_survives_strict_parse_normalization_and_plan_repair() {
    let value = plan_value();
    let plan = parse_plan(&value.to_string()).unwrap();
    assert_eq!(
        serde_json::to_value(plan.shared_context.as_ref().unwrap()).unwrap(),
        context()
    );
    let mut repaired_value = value.clone();
    let tasks = repaired_value
        .as_object_mut()
        .unwrap()
        .remove("subtasks")
        .unwrap();
    repaired_value["sections"] = tasks;
    let (repaired, used_repair) =
        crate::plan_repair::parse_orchestrator_response(&repaired_value.to_string(), &request())
            .unwrap();
    assert!(used_repair);
    assert_eq!(repaired.shared_context, plan.shared_context);
    let mut normalized = plan.clone();
    crate::plan_normalize::normalize(&mut normalized, &request());
    assert_eq!(normalized.shared_context, plan.shared_context);
}

#[test]
fn shared_context_all_pages_images_specs_and_retries_receive_the_same_full_data() {
    let mut value = plan_value();
    // Exceeds the existing 1200-character same-page copy budget. The suffix is
    // deliberate: losing it would silently change a selected variant.
    let long_subject = format!(
        "{} EXACT SUBJECT SUFFIX",
        "linen material detail ".repeat(80)
    );
    value["sharedContext"]["exampleEntities"][0]["imageSubject"] = json!(long_subject);
    let plan = parse_plan(&value.to_string()).unwrap();
    let mut expected = None;
    for model in ["glm-4.6", "glm-5.3-flash"] {
        let mut req = request();
        req.model = Some(model.into());
        for task in &plan.subtasks {
            for (reduced, minimal, feedback) in [
                (false, false, None),
                (
                    true,
                    false,
                    Some(RetryFeedback::Completeness("missing an item".into())),
                ),
                (
                    true,
                    true,
                    Some(RetryFeedback::SelfCheck(
                        "retry the malformed section".into(),
                    )),
                ),
                (
                    false,
                    false,
                    Some(RetryFeedback::Geometry("fix wrapping".into())),
                ),
            ] {
                let mut retried = task.clone();
                retried.retry_feedback = feedback;
                let (call, _) = build_subagent_prompt(
                    &retried,
                    &plan,
                    &req,
                    AbortFlag::new(),
                    reduced,
                    minimal,
                    &ComponentLibrary::default(),
                );
                let shared = block(&call.user_prompt);
                assert!(call
                    .user_prompt
                    .contains(&format!("\"subtaskId\":\"{}\"", task.id)));
                assert!(shared.contains("Sage linen midi dress"));
                assert!(shared.contains("Sage green / M"));
                assert!(shared.contains("¥329") && shared.contains("¥128"));
                assert!(shared.contains(&long_subject));
                assert!(shared.contains("imagePrompt/imageSearchQuery"));
                assert!(shared.contains("FICTIONAL EXAMPLE SETTINGS"));
                assert!(call.user_prompt.contains(BRIEF));
                if let Some(expected) = &expected {
                    assert_eq!(shared, expected);
                } else {
                    expected = Some(shared.to_string());
                }
            }
        }
    }
}

#[test]
fn shared_context_does_not_promote_a_planner_claim_or_unknown_real_data() {
    let mut value = plan_value();
    value["sharedContext"]["sourceFacts"] = json!([
        "The brand is Luma.",
        "The real account has a verified balance of ¥50000."
    ]);
    let plan = parse_plan(&value.to_string()).unwrap();
    let mut prompt = String::new();
    append_instruction(&plan, &request(), &plan.subtasks[0], &mut prompt);
    assert!(prompt.contains("The brand is Luma."));
    assert!(!prompt.contains("¥50000"));
    assert!(prompt.contains("not user-provided or verified facts"));
    assert!(prompt.contains("unknown-real-data placeholders always take priority"));
    for restriction in [
        "不要示例数据，未知金额待填写。",
        "Real data only; do not invent missing values.",
    ] {
        let mut req = request();
        req.prompt.push_str(restriction);
        let mut prompt = String::new();
        append_instruction(&plan, &req, &plan.subtasks[0], &mut prompt);
        assert!(prompt.contains("The brand is Luma."));
        assert!(!prompt.contains("Sage linen midi dress"));
        assert!(!prompt.contains("¥329"));
        assert!(!prompt.contains("¥50000"));
    }
}

#[test]
fn shared_context_reference_content_keeps_priority_over_new_example_choices() {
    let plan = parse_plan(&plan_value().to_string()).unwrap();
    for source in ["复刻参考图的商品与金额。", "Reproduce the uploaded image."] {
        let mut req = request();
        req.prompt.push_str(source);
        let mut prompt = String::new();
        append_instruction(&plan, &req, &plan.subtasks[0], &mut prompt);
        assert!(prompt.contains("The brand is Luma."));
        assert!(!prompt.contains("Sage linen midi dress"));
        assert!(!prompt.contains("¥329"));
    }
}

#[test]
fn shared_context_limits_reject_whole_invalid_data_instead_of_truncating() {
    for invalid in [
        json!({"exampleEntities":[{"id":"x","name":"x"},{"id":"x","name":"y"}]}),
        json!({"exampleEntities":[{"id":"x","name":"x".repeat(MAX_CONTEXT_BYTES)}]}),
        json!({"exampleEntities":[{"id":"x","name":"x","verifiedRealPrice":"¥50000"}]}),
    ] {
        let mut value = plan_value();
        value["sharedContext"] = invalid;
        assert!(parse_plan(&value.to_string()).is_err());
        assert!(
            crate::plan_repair::parse_orchestrator_response(&value.to_string(), &request())
                .is_none()
        );
    }
}

#[test]
fn shared_context_planner_contract_reaches_every_planning_mode() {
    for mode in [
        PlanningMode::Rich,
        PlanningMode::Minimal,
        PlanningMode::Compact,
    ] {
        let call = build_orchestrator_prompt(&request(), mode, AbortFlag::new()).call_request;
        assert!(call.system_prompt.contains("\"sharedContext\""));
        assert!(call.system_prompt.contains("decide its identity ONCE"));
        assert!(call
            .system_prompt
            .contains("NOT user facts or verified business data"));
        assert!(call
            .system_prompt
            .contains("placeholders for requested real fields whose values are unknown"));
        assert!(call.user_prompt.contains(BRIEF));
    }
}
