use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, NodeId};
use serde_json::{json, Value};

pub(super) const ELEMENTS: &str = "「颜色」分组标题 + 3 个规格胶囊（月白/燕麦/苔绿），胶囊 onTap 切换选中态（选中为深色填充白字，未选中浅灰描边），切换时带 200ms 尺寸/颜色 transition；「数量」步进器（−/数字/+）；胶囊组与数量之间留 16px 间距";
const INITIAL: &str =
    include_str!("../tests/fixtures/subtask_completeness/product-specs-initial-third-block.js");
const RETRY2: &str = include_str!("../tests/fixtures/subtask_completeness/product-specs-retry2.js");
const RETRY3: &str = include_str!("../tests/fixtures/subtask_completeness/product-specs-retry3.js");
const ECHO: &str =
    include_str!("../tests/fixtures/subtask_completeness/product-specs-rejected-geometry-echo.js");

pub(super) fn task(elements: &str) -> Subtask {
    Subtask {
        id: "specs".into(),
        label: "商品规格选择".into(),
        region: Region {
            width: 375.0,
            height: 0.0,
        },
        bleed_hero: false,
        id_prefix: "specs".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: Some(elements.into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    }
}

pub(super) fn sink(roots: Vec<Value>) -> VecDocSink {
    VecDocSink {
        state: EditorState::from_document(
            serde_json::from_value(json!({"version":"1.0", "children":roots})).unwrap(),
        ),
        applied: Vec::new(),
        batch_depth: 0,
    }
}

pub(super) fn outcome(roots: &[&str]) -> SubtaskOutcome {
    SubtaskOutcome {
        id: "specs".into(),
        node_count: roots.len(),
        inserted_root_ids: roots.iter().map(|id| (*id).into()).collect(),
        error: None,
        headline: None,
        subtask: Some(task(ELEMENTS)),
    }
}

pub(super) fn pill(id: &str, label: &str) -> Value {
    json!({"type":"frame", "id":id, "layout":"horizontal", "children":[
        {"type":"text", "id":format!("{id}-label"), "content":label}
    ]})
}

pub(super) fn section(choices: Vec<Value>) -> Value {
    json!({"type":"frame", "id":"section", "layout":"vertical", "children":[
        {"type":"frame", "id":"color-group", "layout":"horizontal", "children":[
            {"type":"text", "id":"heading", "content":"颜色"},
            {"type":"frame", "id":"choices", "layout":"horizontal", "children":choices}
        ]}
    ]})
}

fn inserted(script: &str) -> (VecDocSink, SubtaskOutcome) {
    // Decode the exact historical bytes through the shared builder/executor.
    // Runtime-expression validation is tested separately below: retained copy
    // includes an old String(...) binding that the new guard now rejects.
    let program = op_mcp::script_runner::run_generation_script_to_program(script)
        .expect("retained builder script parses");
    let (nodes, _) = crate::program_gen::run_program_to_forest(&program)
        .expect("retained program produces its original forest");
    let mut sink = VecDocSink::new();
    let ids = sink
        .insert_subtree_returning_root_ids(nodes, &NodeId::NONE)
        .expect("retained roots inserted");
    let result = outcome(&ids.iter().map(String::as_str).collect::<Vec<_>>());
    (sink, result)
}

#[test]
fn retained_empty_pills_and_unrelated_stepper_cannot_satisfy_named_specifications() {
    for (script, generic_count) in [(RETRY2, 2), (RETRY3, 3)] {
        let (sink, result) = inserted(script);
        assert_eq!(
            delivered_item_count(&sink, &result.inserted_root_ids),
            generic_count
        );
        let failure = incomplete_attempt(&sink, &task(ELEMENTS), &result).unwrap();
        assert_eq!((failure.expected, failure.delivered), (3, 0));
        assert!(failure
            .feedback
            .starts_with("The plan asks for 3 named specification options"));
        for label in ["月白", "燕麦", "苔绿"] {
            assert!(failure.feedback.contains(label));
        }
    }
}

#[test]
fn retained_complete_script_and_echo_copy_deliver_all_three_labels() {
    // The initial third block is tested alone; the original multi-block response
    // remains invalid under the independent script parser contract.
    for script in [INITIAL, ECHO] {
        let error = crate::script_gen::parse_script(script).unwrap_err();
        assert!(error
            .to_string()
            .contains("generated-expression-unknown-function"));
        let (sink, result) = inserted(script);
        assert_eq!(
            named_options::inspect(&sink, &task(ELEMENTS), &result).unwrap(),
            3
        );
        assert!(incomplete_attempt(&sink, &task(ELEMENTS), &result).is_none());
    }
}

fn ladder(responses: &[&str]) -> (SubtaskOutcome, Vec<Progress>, Vec<String>, Vec<PenNode>) {
    ladder_for_task(task(ELEMENTS), responses)
}

fn ladder_for_task(
    task: Subtask,
    responses: &[&str],
) -> (SubtaskOutcome, Vec<Progress>, Vec<String>, Vec<PenNode>) {
    let plan = OrchestratorPlan {
        shared_context: None,
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "商品详情".into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![task.clone()],
        style_guide_name: None,
        palette: None,
    };
    let request = DesignRequest {
        prompt: "生成商品详情的颜色规格和数量选择".into(),
        model: None,
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
    let llm = ScriptedLlm::new(
        responses
            .iter()
            .map(|s| ScriptResponse::Text((*s).into()))
            .collect(),
    );
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let result =
        futures::executor::block_on(crate::concurrent::run_subtask_retry_ladder_with_outcomes(
            &task,
            &plan,
            &request,
            &llm,
            &mut sink,
            &AbortFlag::new(),
            crate::model_profile::ModelTier::Full,
            None,
            &GeometryEchoBudget::new(0),
            &mut |event| events.push(event),
            &[],
        ));
    (
        result,
        events,
        llm.user_prompts(),
        sink.state.active_children().to_vec(),
    )
}

#[test]
fn named_feedback_repairs_the_real_missing_labels_in_the_existing_retry_ladder() {
    // Keep the retained fixture byte-exact; the successful live response must
    // also satisfy the new runtime-expression contract for the quantity label.
    let repaired = INITIAL.replace("String($state.qty)", "to_str($state.qty)");
    let (result, events, prompts, roots) = ladder(&[RETRY2, RETRY3, &repaired]);
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(
        roots.len(),
        1,
        "both incomplete attempts must be rolled back"
    );
    assert_eq!(prompts.len(), 3);
    let feedback =
        named_options::inspect(&inserted(RETRY3).0, &task(ELEMENTS), &inserted(RETRY3).1)
            .unwrap_err()
            .feedback;
    for prompt in &prompts[1..] {
        assert!(prompt.contains(&feedback));
    }
    assert_eq!(
        events
            .iter()
            .filter(
                |event| matches!(event, Progress::SubtaskRetry{reason,..} if reason == &feedback)
            )
            .count(),
        2
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskIncomplete { .. })));
    let final_sink = sink(
        roots
            .iter()
            .map(|node| serde_json::to_value(node).unwrap())
            .collect(),
    );
    assert_eq!(
        named_options::inspect(&final_sink, &task(ELEMENTS), &result).unwrap(),
        3
    );
}

#[test]
fn exhausted_named_option_retries_retain_only_the_last_incomplete_attempt() {
    let (result, events, prompts, roots) = ladder(&[RETRY3, RETRY3, RETRY3]);
    assert_eq!(roots.len(), 1);
    assert_eq!(prompts.len(), 3);
    assert!(is_incomplete_outcome(&result));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                Progress::SubtaskIncomplete {
                    expected: 3,
                    delivered: 0,
                    ..
                }
            ))
            .count(),
        1
    );
}

#[path = "subtask_completeness_unnumbered_options_tests.rs"]
mod unnumbered;
