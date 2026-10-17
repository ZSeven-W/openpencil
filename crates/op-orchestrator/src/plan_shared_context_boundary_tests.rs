//! Keep shared examples scoped to the requested section and source authority.

use super::append_instruction;
use crate::plan::{OrchestratorPlan, Subtask};
use crate::types::DesignRequest;
use serde_json::{json, Value};

const SOURCE_FACT: &str = "The brand is Luma.";

fn fixture() -> (OrchestratorPlan, DesignRequest) {
    let plan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"Shop","width":375,"height":812},
        "subtasks":[
            {"id":"s1","label":"Section one","screen":"Detail",
             "region":{"width":375,"height":200}},
            {"id":"s2","label":"Section two","screen":"Cart",
             "region":{"width":375,"height":200}}
        ],
        "sharedContext":{
            "sourceFacts":[SOURCE_FACT],
            "exampleEntities":[
                {"id":"entity-a","name":"Sage linen dress",
                 "imageSubject":"sage green linen midi dress",
                 "attributes":{"selectedSpecification":"Sage / M"},"usedBy":["s1"]},
                {"id":"entity-b","name":"Oat linen scarf",
                 "imageSubject":"oat colored linen scarf",
                 "attributes":{"selectedSpecification":"Oat / One size"},"usedBy":["s2"]}
            ]
        }
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: format!("Create product detail and cart screens. {SOURCE_FACT}"),
        ..Default::default()
    };
    (plan, request)
}

fn render(plan: &OrchestratorPlan, request: &DesignRequest, task: &Subtask) -> String {
    let mut prompt = String::new();
    append_instruction(plan, request, task, &mut prompt);
    prompt
}

fn shared_json(prompt: &str) -> &str {
    prompt
        .split_once("\nEND SHARED CONTENT CONTEXT")
        .expect("verified source facts must remain in the prompt")
        .0
        .rsplit_once('\n')
        .expect("shared JSON must occupy a complete line")
        .1
}

fn section_binding(prompt: &str) -> Value {
    let json = prompt
        .lines()
        .find_map(|line| line.strip_prefix("SHARED CONTENT SECTION: "))
        .expect("opaque task ids need an explicit entity binding");
    serde_json::from_str(json).unwrap()
}

fn assert_source_only(prompt: &str) {
    let context: Value = serde_json::from_str(shared_json(prompt)).unwrap();
    assert_eq!(context["sourceFacts"], json!([SOURCE_FACT]));
    assert!(context
        .get("exampleEntities")
        .is_none_or(|entities| entities.as_array().is_some_and(Vec::is_empty)));
    for example in [
        "entity-a",
        "entity-b",
        "Sage linen dress",
        "Oat linen scarf",
    ] {
        assert!(
            !prompt.contains(example),
            "suppressed example leaked: {example}"
        );
    }
}

#[test]
fn opaque_task_ids_keep_identical_shared_data_and_distinct_entity_bindings() {
    let (plan, request) = fixture();
    let expected = serde_json::to_string(plan.shared_context.as_ref().unwrap()).unwrap();
    for (task, entity) in plan.subtasks.iter().zip(["entity-a", "entity-b"]) {
        let prompt = render(&plan, &request, task);
        assert_eq!(shared_json(&prompt).as_bytes(), expected.as_bytes());
        assert_eq!(
            section_binding(&prompt),
            json!({"subtaskId":task.id,"exampleEntityIds":[entity]})
        );
    }
}

#[test]
fn explicit_no_invention_and_unknown_blank_requests_preserve_only_source_facts() {
    let (plan, base) = fixture();
    for restriction in [
        "No fictional data.",
        "Do not make up data.",
        "Leave unknown fields blank.",
        "不要演示数据。",
        "不可编造商品或规格。",
    ] {
        let mut request = base.clone();
        request.prompt.push_str(restriction);
        for task in &plan.subtasks {
            assert_source_only(&render(&plan, &request, task));
        }
    }
}

#[test]
fn attached_reference_and_uploaded_photos_do_not_gain_fictional_subjects() {
    let (plan, base) = fixture();
    for source in [
        "Use the attached photo as the product reference.",
        "Recreate the item in the reference photo.",
        "Keep the product from the uploaded photo.",
    ] {
        let mut request = base.clone();
        request.prompt.push_str(source);
        for task in &plan.subtasks {
            assert_source_only(&render(&plan, &request, task));
        }
    }
}

#[test]
fn explicit_current_section_empty_state_does_not_clear_other_populated_sections() {
    let (base, request) = fixture();
    for (label, elements) in [
        ("Empty cart state", "Start shopping action"),
        ("Empty cart", "Browse products button"),
        ("Cart", "An empty state with a start shopping action"),
        (
            "Cart",
            "Empty state illustration and a Browse products button",
        ),
        ("No items", "Start shopping action"),
        ("Cart", "购物车空态，只显示引导文案"),
    ] {
        let mut plan = base.clone();
        plan.subtasks[0].label = label.into();
        plan.subtasks[0].elements = Some(elements.into());
        assert_source_only(&render(&plan, &request, &plan.subtasks[0]));

        let populated = render(&plan, &request, &plan.subtasks[1]);
        let expected = serde_json::to_string(plan.shared_context.as_ref().unwrap()).unwrap();
        assert_eq!(shared_json(&populated), expected);
        assert_eq!(
            section_binding(&populated),
            json!({"subtaskId":"s2","exampleEntityIds":["entity-b"]})
        );
        // Prompt construction must not mutate the shared plan for later work.
        assert_eq!(plan.shared_context, base.shared_context);
    }
}

#[test]
fn clear_controls_and_negated_empty_states_keep_populated_entities() {
    let (base, request) = fixture();
    for elements in [
        "Product list with a Clear cart button",
        "Product list with an Empty cart button",
        "商品列表，提供商品清空按钮",
        "Populated product list; no empty state",
        "非空列表，显示已有商品",
        "Show products; an empty-state variant is available when the list has no items",
    ] {
        let mut plan = base.clone();
        plan.subtasks[0].label = "Product list".into();
        plan.subtasks[0].elements = Some(elements.into());
        let prompt = render(&plan, &request, &plan.subtasks[0]);
        let expected = serde_json::to_string(plan.shared_context.as_ref().unwrap()).unwrap();
        assert_eq!(shared_json(&prompt), expected, "{elements}");
        assert_eq!(
            section_binding(&prompt),
            json!({"subtaskId":"s1","exampleEntityIds":["entity-a"]}),
            "{elements}"
        );
    }
}
