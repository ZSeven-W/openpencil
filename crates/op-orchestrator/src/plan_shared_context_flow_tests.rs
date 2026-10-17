//! Shared entities must survive real concurrent replay, parse retries and salvage.

use crate::agent_identity::AgentIdentity;
use crate::model_profile::ModelTier;
use crate::plan::OrchestratorPlan;
use crate::screen_groups::{group_subtasks_by_screen, ScreenGroup};
use crate::test_support::{
    SkippedPreValidator, SkippedScreenshotProvider, SkippedVisionLlmClient, VecDocSink,
};
use crate::types::{
    AbortFlag, CallRequest, DesignRequest, GeometryEchoBudget, LlmChunk, LlmClient, LlmError,
    Progress, SubtaskOutcome, ValidationProviders,
};
use crate::Orchestrator;
use futures::stream::BoxStream;
use op_editor_core::{EditorState, NodeId};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const SECTIONS: [&str; 4] = [
    "Product gallery",
    "Product information",
    "Selected specification",
    "Cart item",
];
const PRODUCT: &str = "Sage linen dress";
const SUBJECT: &str = "sage green linen midi dress on a neutral studio background";
const SPECIFICATION: &str = "sage green / M";
const PRICE: &str = "¥329";

fn fixture() -> (
    VecDocSink,
    OrchestratorPlan,
    DesignRequest,
    Vec<ScreenGroup>,
) {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({
            "version":"1.0.0", "children":[
                {"id":"detail-root","type":"frame","width":375,"height":812,"layout":"vertical"},
                {"id":"cart-root","type":"frame","width":375,"height":812,"layout":"vertical"}
            ]
        }))
        .unwrap(),
    );
    let subtasks: Vec<Value> = [
        ("gallery", SECTIONS[0], "detail-root", "Detail"),
        ("info", SECTIONS[1], "detail-root", "Detail"),
        ("spec", SECTIONS[2], "detail-root", "Detail"),
        ("cart", SECTIONS[3], "cart-root", "Cart"),
    ]
    .into_iter()
    .map(|(id, label, parent, screen)| {
        json!({
            "id":id,"label":label,"parentFrameId":parent,"screen":screen,
            "region":{"width":375,"height":200}
        })
    })
    .collect();
    let plan: OrchestratorPlan = serde_json::from_value(json!({
        "rootFrame":{"id":"detail-root","name":"Shop","width":375,"height":812},
        "subtasks":subtasks,
        "sharedContext":{
            "sourceFacts":[],
            "exampleEntities":[{
                "id":"selected-product","name":PRODUCT,"imageSubject":SUBJECT,
                "attributes":{"price":PRICE,"selectedSpecification":SPECIFICATION},
                "usedBy":["gallery","info","spec","cart"]
            }]
        }
    }))
    .unwrap();
    let groups = group_subtasks_by_screen(&plan.subtasks);
    let request = DesignRequest {
        prompt: "Design linked product detail and cart app screens with a product gallery, product information, selected specification, and cart item. Use a fictional populated example.".into(),
        model: Some("glm-5.3-flash".into()),
        concurrency: 2,
        validation_enabled: false,
        visual_ref_enabled: false,
        ..Default::default()
    };
    (sink, plan, request, groups)
}

fn context_json(prompt: &str) -> &str {
    prompt
        .split_once("\nEND SHARED CONTENT CONTEXT")
        .expect("every generation attempt must receive shared context")
        .0
        .rsplit_once('\n')
        .expect("the shared context must end with one complete JSON line")
        .1
}

struct RecordedCall {
    label: String,
    prompt: String,
}

struct ContractLlm {
    plan: String,
    calls: Mutex<Vec<RecordedCall>>,
    info_attempts: AtomicUsize,
    parse_failures: usize,
}

impl ContractLlm {
    fn new(plan: &OrchestratorPlan, parse_failures: usize) -> Self {
        Self {
            plan: serde_json::to_string(plan).unwrap(),
            calls: Mutex::default(),
            info_attempts: AtomicUsize::new(0),
            parse_failures,
        }
    }
}

impl LlmClient for ContractLlm {
    fn call(&self, request: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let Some(label) = SECTIONS.into_iter().find(|label| {
            request
                .user_prompt
                .contains(&format!("Generate ONLY \"{label}\""))
        }) else {
            assert!(!request.user_prompt.contains("Generate ONLY"));
            return Box::pin(futures::stream::iter([Ok(LlmChunk::Text(
                self.plan.clone(),
            ))]));
        };
        let context: Value = serde_json::from_str(context_json(&request.user_prompt)).unwrap();
        self.calls.lock().unwrap().push(RecordedCall {
            label: label.into(),
            prompt: request.user_prompt,
        });
        let script = if label == SECTIONS[1]
            && self.info_attempts.fetch_add(1, Ordering::SeqCst) < self.parse_failures
        {
            // A real script parser failure, not a mock retry or transport error.
            "I(null,".into()
        } else {
            section_script(label, &context["exampleEntities"][0])
        };
        Box::pin(futures::stream::iter([Ok(LlmChunk::Text(script))]))
    }
}

fn section_script(label: &str, entity: &Value) -> String {
    let text = |content: &Value| {
        json!({"type":"text","content":content,"fontSize":16,
            "width":"fill_container","height":"fit_content"})
    };
    let photo = || {
        json!({"type":"image","name":"Product photo","width":327,"height":120,
            "imagePrompt":entity["imageSubject"],"imageSearchQuery":entity["imageSubject"]})
    };
    let children = match label {
        // No visible copy exists for the following information section to copy.
        "Product gallery" => vec![photo()],
        "Product information" => vec![text(&entity["name"]), text(&entity["attributes"]["price"])],
        "Selected specification" => vec![text(&entity["attributes"]["selectedSpecification"])],
        "Cart item" => vec![
            photo(),
            text(&entity["name"]),
            text(&entity["attributes"]["selectedSpecification"]),
            text(&entity["attributes"]["price"]),
        ],
        _ => unreachable!(),
    };
    format!(
        "I(null, {});",
        json!({"type":"frame","name":label,"layout":"vertical","width":327,
            "height":"fit_content","gap":8,"children":children})
    )
}

fn has_value(value: &Value, key: &str, expected: &str) -> bool {
    match value {
        Value::Object(fields) => {
            fields.get(key).and_then(Value::as_str) == Some(expected)
                || fields.values().any(|value| has_value(value, key, expected))
        }
        Value::Array(values) => values.iter().any(|value| has_value(value, key, expected)),
        _ => false,
    }
}

fn assert_committed_entities(sink: &VecDocSink, outcomes: &[SubtaskOutcome]) {
    for id in ["gallery", "info", "spec", "cart"] {
        let outcome = outcomes.iter().find(|outcome| outcome.id == id).unwrap();
        assert!(outcome.error.is_none(), "{id}: {:?}", outcome.error);
        assert!(outcome.node_count > 0);
        assert_eq!(outcome.inserted_root_ids.len(), 1);
        let node = op_editor_core::walkers::find_node(
            sink.state.active_children(),
            &NodeId::new(&outcome.inserted_root_ids[0]),
        )
        .expect("successful outcomes must reference real committed nodes");
        let value = serde_json::to_value(node).unwrap();
        if matches!(id, "gallery" | "cart") {
            assert!(has_value(&value, "imagePrompt", SUBJECT), "{id}: {value}");
            assert!(
                has_value(&value, "imageSearchQuery", SUBJECT),
                "{id}: {value}"
            );
        }
        if matches!(id, "info" | "cart") {
            assert!(has_value(&value, "content", PRODUCT), "{id}: {value}");
            assert!(has_value(&value, "content", PRICE), "{id}: {value}");
        }
        if matches!(id, "spec" | "cart") {
            assert!(has_value(&value, "content", SPECIFICATION), "{id}: {value}");
        }
    }
}

fn assert_identical_contracts(llm: &ContractLlm, plan: &OrchestratorPlan, info_calls: usize) {
    let expected = serde_json::to_string(plan.shared_context.as_ref().unwrap()).unwrap();
    let calls = llm.calls.lock().unwrap();
    assert_eq!(calls.len(), 3 + info_calls);
    for label in SECTIONS {
        let matching: Vec<_> = calls.iter().filter(|call| call.label == label).collect();
        assert_eq!(
            matching.len(),
            if label == SECTIONS[1] { info_calls } else { 1 }
        );
        for call in matching {
            assert_eq!(
                context_json(&call.prompt).as_bytes(),
                expected.as_bytes(),
                "{label}"
            );
        }
    }
    for label in [SECTIONS[0], SECTIONS[3]] {
        let first = calls.iter().find(|call| call.label == label).unwrap();
        assert!(!first.prompt.contains("EXISTING SAME-PAGE COPY"), "{label}");
    }
}

fn unscoped(progress: &Progress) -> &Progress {
    match progress {
        Progress::WorkerScoped(worker) => unscoped(&worker.event),
        event => event,
    }
}

#[tokio::test]
async fn concurrent_sections_share_identity_without_prior_copy_and_after_parse_retry() {
    let (mut sink, plan, request, groups) = fixture();
    let llm = ContractLlm::new(&plan, 1);
    let identities: Vec<_> = ["Detail", "Cart"]
        .into_iter()
        .map(|name| AgentIdentity {
            name: name.into(),
            color: "#5B8DEF".into(),
        })
        .collect();
    let mut events = Vec::new();
    let result = crate::concurrent::run_screen_groups_concurrent(
        &groups,
        &identities,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        ModelTier::Basic,
        2,
        None,
        &GeometryEchoBudget::new(0),
        &mut |event| events.push(event),
    )
    .await;
    assert!(!result.aborted_mid);
    assert_eq!(llm.info_attempts.load(Ordering::SeqCst), 2);
    assert!(events.iter().any(|event| matches!(unscoped(event),
        Progress::SubtaskRetry { id, attempt: 2, .. } if id == "info")));
    assert_identical_contracts(&llm, &plan, 2);
    assert_committed_entities(&sink, &result.outcomes);
}

#[tokio::test]
async fn final_salvage_after_three_parse_failures_keeps_the_same_shared_entity() {
    let (_, plan, request, _) = fixture();
    let llm = ContractLlm::new(&plan, 3);
    let providers = ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    };
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let summary = Orchestrator::new()
        .run(
            request,
            &mut sink,
            &llm,
            &mut |event| events.push(event),
            &AbortFlag::new(),
            &providers,
        )
        .await
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::ConcurrentGroupsStarted {
            group_count: 2,
            workers: 2
        }
    )));
    assert_eq!(llm.info_attempts.load(Ordering::SeqCst), 4);
    assert!(events.iter().any(|event| matches!(unscoped(event),
        Progress::SubtaskRetry { id, attempt: 4, .. } if id == "info")));
    assert_identical_contracts(&llm, &plan, 4);
    assert_committed_entities(&sink, &summary.subtasks);
}
