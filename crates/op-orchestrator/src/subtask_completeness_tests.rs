use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::run::Orchestrator;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress, ValidationProviders};
use futures::executor::block_on;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use serde_json::{json, Value};

fn subtask(label: &str, elements: Option<&str>) -> Subtask {
    Subtask {
        id: "merchants".into(),
        label: label.into(),
        region: Region {
            width: 375.0,
            height: 400.0,
        },
        bleed_hero: false,
        id_prefix: "merchants".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: elements.map(str::to_owned),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn sink_with_section(children: Vec<Value>) -> VecDocSink {
    let document = serde_json::from_value(json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "section", "name": "商家列表",
            "layout": "vertical", "children": children
        }]
    }))
    .expect("synthetic completeness document");
    VecDocSink {
        state: EditorState::from_document(document),
        applied: Vec::new(),
        batch_depth: 0,
    }
}

fn sink_with_roots(roots: Vec<Value>) -> VecDocSink {
    let document = serde_json::from_value(json!({
        "version": "1.0",
        "children": roots
    }))
    .expect("synthetic root-family document");
    VecDocSink {
        state: EditorState::from_document(document),
        applied: Vec::new(),
        batch_depth: 0,
    }
}

fn card(index: usize) -> Value {
    json!({
        "type": "frame", "id": format!("card-{index}"), "name": format!("商家 {index}"),
        "children": [{
            "type": "text", "id": format!("name-{index}"), "content": format!("商家 {index}")
        }]
    })
}

fn image(index: usize) -> Value {
    json!({
        "type": "image", "id": format!("image-{index}"), "src": "merchant.png",
        "width": 64, "height": 64
    })
}

#[test]
fn expected_item_count_parses_cjk_and_english_promises() {
    assert_eq!(
        expected_item_count(&subtask("商家列表五个（图+名称+评分+配送费+时长）", None)),
        Some(5)
    );
    assert_eq!(expected_item_count(&subtask("推荐视频六卡", None)), Some(6));
    assert_eq!(expected_item_count(&subtask("顶部地址与搜索", None)), None);
    assert_eq!(
        expected_item_count(&subtask("Recent transactions list of 6", None)),
        Some(6)
    );
    assert_eq!(expected_item_count(&subtask("375×812 暗色", None)), None);
    assert_eq!(
        expected_item_count(&subtask("24px · 09:30 · ¥15 · 2026", None)),
        None
    );
    assert_eq!(
        expected_item_count(&subtask("2×3 product grid", None)),
        Some(6)
    );
}

#[test]
fn expected_item_count_uses_largest_promise_and_range_lower_bound() {
    assert_eq!(
        expected_item_count(&subtask("商家 3-5 个，另有 2 条标签", None)),
        Some(3)
    );
    assert_eq!(expected_item_count(&subtask("list of 6", None)), Some(6));
    assert_eq!(
        expected_item_count(&subtask("Recent", Some("show 4 entries"))),
        Some(4)
    );
}

#[test]
fn delivered_item_count_finds_nested_card_families() {
    let mut children = vec![json!({
        "type": "text", "id": "header", "content": "附近商家"
    })];
    children.extend((0..5).map(card));
    let sink = sink_with_section(children);
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 5);
}

#[test]
fn delivered_item_count_ignores_a_header_only_section() {
    let sink = sink_with_section(vec![json!({
        "type": "text", "id": "header", "content": "附近商家"
    })]);
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 0);
}

#[test]
fn delivered_item_count_counts_image_siblings_without_card_frames() {
    let mut children = vec![json!({
        "type": "text", "id": "header", "content": "附近商家"
    })];
    children.extend((0..5).map(image));
    let sink = sink_with_section(children);
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 5);
}

#[test]
fn delivered_item_count_counts_families_across_inserted_roots() {
    let sink = sink_with_roots((0..5).map(card).collect());
    let ids = (0..5)
        .map(|index| format!("card-{index}"))
        .collect::<Vec<_>>();
    assert_eq!(delivered_item_count(&sink, &ids), 5);
}

#[test]
fn delivered_item_count_excludes_status_bar_subtrees() {
    let status_bar = json!({
        "type": "frame", "id": "status", "role": "status-bar",
        "children": (0..5).map(card).collect::<Vec<_>>()
    });
    let sink = sink_with_section(vec![status_bar]);
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 0);
}

fn request() -> DesignRequest {
    DesignRequest {
        prompt: "商家列表五个（图+名称+评分+配送费+时长）".into(),
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
    }
}

fn plan(task: &Subtask) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![task.clone()],
        style_guide_name: None,
    }
}

const HEADER_ONLY: &str = r#"I(null,{"type":"frame","name":"商家列表","children":[{"type":"text","content":"附近商家"}]});"#;

fn cards_script() -> String {
    let cards = (0..5)
        .map(|index| {
            format!(
                r#"I(sec,{{"type":"frame","name":"商家 {index}","children":[{{"type":"text","content":"商家 {index}"}}]}});"#
            )
        })
        .collect::<String>();
    format!(r#"const sec=I(null,{{"type":"frame","name":"商家列表"}});{cards}"#)
}

fn run_ladder(responses: Vec<&str>) -> (SubtaskOutcome, Vec<Progress>, Vec<String>, Vec<PenNode>) {
    let task = subtask("商家列表五个（图+名称+评分+配送费+时长）", None);
    let plan = plan(&task);
    let llm = ScriptedLlm::new(
        responses
            .into_iter()
            .map(|response| ScriptResponse::Text(response.into()))
            .collect(),
    );
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |progress| events.push(progress);
    let outcome =
        futures::executor::block_on(crate::concurrent::run_subtask_retry_ladder_with_outcomes(
            &task,
            &plan,
            &request(),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            crate::model_profile::ModelTier::Full,
            None,
            &GeometryEchoBudget::new(0),
            &mut on_progress,
            &[],
        ));
    (
        outcome,
        events,
        llm.user_prompts(),
        sink.state.active_children().to_vec(),
    )
}

#[test]
fn incomplete_first_attempt_is_rolled_back_and_retried_with_exact_feedback() {
    let (outcome, events, prompts, roots) = run_ladder(vec![HEADER_ONLY, &cards_script()]);
    assert_eq!(outcome.node_count, 1);
    assert_eq!(
        roots.len(),
        1,
        "the incomplete first subtree must be removed"
    );
    assert_eq!(
        delivered_item_count(
            &VecDocSink {
                state: EditorState::from_document(
                    serde_json::from_value(json!({
                        "version": "1.0", "children": roots
                    }))
                    .unwrap()
                ),
                applied: Vec::new(),
                batch_depth: 0,
            },
            &outcome.inserted_root_ids
        ),
        5
    );
    let expected_feedback = completeness_feedback(5, 0);
    assert!(events.iter().any(|event| matches!(event,
        Progress::SubtaskRetry { attempt: 2, reason, .. } if reason == &expected_feedback
    )));
    assert!(prompts[1].contains(&expected_feedback));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskIncomplete { .. })));
}

#[test]
fn final_incomplete_rung_keeps_only_the_last_non_empty_result_and_emits_once() {
    let (outcome, events, prompts, roots) = run_ladder(vec![HEADER_ONLY, HEADER_ONLY, HEADER_ONLY]);
    assert!(outcome.node_count > 0);
    assert_eq!(
        roots.len(),
        1,
        "only the last incomplete result is retained"
    );
    assert_eq!(
        delivered_item_count(
            &VecDocSink {
                state: EditorState::from_document(
                    serde_json::from_value(json!({
                        "version": "1.0", "children": roots
                    }))
                    .unwrap()
                ),
                applied: Vec::new(),
                batch_depth: 0,
            },
            &outcome.inserted_root_ids
        ),
        0
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Progress::SubtaskIncomplete { .. }))
            .count(),
        1
    );
    assert!(
        matches!(events.iter().find(|event| matches!(event, Progress::SubtaskIncomplete { .. })),
            Some(Progress::SubtaskIncomplete { id, expected: 5, delivered: 0 }) if id == "merchants"
        )
    );
    assert_eq!(
        outcome.error.as_deref(),
        Some(completeness_feedback(5, 0).as_str())
    );
    let feedback = completeness_feedback(5, 0);
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                Progress::SubtaskRetry { reason, .. } => Some(reason),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![&feedback, &feedback]
    );
    assert!(prompts[1].contains(&feedback));
    assert!(prompts[2].contains(&feedback));
}

#[test]
fn completeness_rollback_uses_delete_commands_for_inserted_roots() {
    let mut sink = sink_with_section(Vec::new());
    let roots = vec!["section".to_string()];
    rollback_inserted_roots(&mut sink, &roots);
    assert!(sink.applied.iter().any(|command| matches!(
        command,
        EditorCommand::DeleteNode { node_id, .. } if node_id == &NodeId::new("section")
    )));
}

#[test]
fn orchestrator_summary_counts_a_final_incomplete_subtask() {
    const PLAN: &str = r##"{
        "rootFrame": {"id":"root","name":"Page","width":375,"height":812,"layout":"vertical"},
        "subtasks": [{"id":"merchants","label":"商家列表五个（图+名称+评分+配送费+时长）","region":{"width":375,"height":400}}]
    }"##;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN.into()),
        ScriptResponse::Text(HEADER_ONLY.into()),
        ScriptResponse::Text(HEADER_ONLY.into()),
        ScriptResponse::Text(HEADER_ONLY.into()),
    ]);
    let mut sink = VecDocSink::new();
    let mut request = request();
    request.validation_enabled = false;
    let providers = ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    };
    let mut events = Vec::new();
    let mut emit = |event| events.push(event);
    let summary = block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut emit,
        &AbortFlag::new(),
        &providers,
    ))
    .expect("a non-empty final incomplete section remains usable");

    assert!(summary.incomplete_subtask_failure);
    assert_eq!(summary.subtasks.len(), 1);
    assert!(summary.subtasks[0].node_count > 0);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Progress::SubtaskIncomplete { .. }))
            .count(),
        1
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskDone { id, .. } if id == "merchants")));
}
