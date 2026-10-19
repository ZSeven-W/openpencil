//! Retained production scripts: optional merchant badges do not change quota.

use super::*;
use crate::plan::OrchestratorPlan;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use op_editor_core::EditorCommand;
use serde_json::json;

#[path = "subtask_completeness_media_details_tests.rs"]
mod details;

const ELEMENTS: &str = "区块标题「附近商家」+ 筛选 tab（综合排序/销量/距离）；5 条商家行卡片：左 88px 方形美食实拍图，右侧商家名、月售与配送时长行、起送/配送费行、右侧 2 个活动小标签；商家行 onTap 导航至「商家详情」；列表卡片进入视口时交错 fade-up（每条延迟 60ms）；底部 padding 预留";
const SCRIPTS: [&str; 3] = [
    include_str!("../tests/fixtures/subtask_completeness/merchant-list-1018-attempt1.js"),
    include_str!("../tests/fixtures/subtask_completeness/merchant-list-1018-attempt2.js"),
    include_str!("../tests/fixtures/subtask_completeness/merchant-list-1018-attempt3.js"),
];

fn task() -> Subtask {
    serde_json::from_value(json!({
        "id":"home-store-list", "label":"首页-商家列表", "elements":ELEMENTS,
        "idPrefix":"home-store-list", "region":{"width":375,"height":0}
    }))
    .unwrap()
}

fn inserted(script: &str) -> (VecDocSink, SubtaskOutcome) {
    // Inspect the unchanged historical trees independently of action validity.
    // The public generation gate rejects attempt 2's malformed push below;
    // the real builder/executor still reconstructs its five merchant cards.
    let program = op_mcp::script_runner::run_generation_script_to_program(script)
        .expect("retained JavaScript builds a program");
    let (nodes, _) = crate::program_gen::run_program_to_forest(&program)
        .expect("retained program builds schema nodes");
    let mut sink = VecDocSink::new();
    let ids = sink
        .insert_subtree_returning_root_ids(nodes, &NodeId::NONE)
        .expect("retained script inserts");
    let outcome = SubtaskOutcome {
        id: "home-store-list".into(),
        node_count: 1,
        inserted_root_ids: ids,
        error: None,
        headline: None,
        subtask: None,
    };
    (sink, outcome)
}

fn card_ids(nodes: &[PenNode], out: &mut Vec<String>) {
    for node in nodes {
        if let Some(children) = node.children() {
            if matches!(node, PenNode::Frame(_))
                && children
                    .iter()
                    .any(|child| matches!(child, PenNode::Image(_)))
            {
                out.push(node.id_str().to_owned());
            }
            card_ids(children, out);
        }
    }
}

#[test]
fn retained_extra_quote_push_is_rejected_before_independent_structure_check() {
    // Keep all three original response files byte-for-byte. Only attempt 2 has
    // the extra quote; the two valid historical action chains still pass.
    crate::script_gen::parse_script(SCRIPTS[0]).unwrap();
    crate::script_gen::parse_script(SCRIPTS[2]).unwrap();
    let error = crate::script_gen::parse_script(SCRIPTS[1]).unwrap_err();
    assert!(matches!(
        &error,
        crate::script_gen::ScriptGenError::RuntimeExpression(_)
    ));
    let message = error.to_string();
    assert!(message.starts_with("generated-action-body:"), "{message}");
    assert!(message.contains("events.onTap[0]"), "{message}");
    assert!(message.contains("push"), "{message}");
    assert!(message.contains("unterminated string"), "{message}");
}

#[test]
fn all_three_retained_merchant_scripts_deliver_five_despite_optional_badges() {
    let task = task();
    assert_eq!(expected_item_count(&task), Some(5));
    assert!(media_items::has_local_media_card_contract(
        &local_item_text(&task),
        5
    ));
    for (attempt, script) in SCRIPTS.iter().enumerate() {
        let (sink, outcome) = inserted(script);
        let mut cards = Vec::new();
        card_ids(sink.state.active_children(), &mut cards);
        assert_eq!(cards.len(), 5, "attempt {}", attempt + 1);
        // Retain the exact old failure, proving this is a counting correction.
        assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 3);
        assert_eq!(
            media_items::delivered_media_card_count(&sink, &outcome.inserted_root_ids),
            Some(5)
        );
        assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
    }
}

#[test]
fn removing_any_real_merchant_from_each_retained_script_is_still_four_of_five() {
    for (attempt, script) in SCRIPTS.iter().enumerate() {
        let (sink, outcome) = inserted(script);
        let mut cards = Vec::new();
        card_ids(sink.state.active_children(), &mut cards);
        assert_eq!(cards.len(), 5);
        for card in cards {
            let mut missing = VecDocSink::new();
            missing.state = EditorState::from_document(sink.state.doc.clone());
            assert!(missing.apply(EditorCommand::DeleteNode {
                node_id: NodeId::new(&card),
                page_id: None,
            }));
            let failure = incomplete_attempt(&missing, &task(), &outcome)
                .expect("four real cards cannot satisfy five");
            assert_eq!(
                (failure.expected, failure.delivered),
                (5, 4),
                "attempt {}, removed {card}",
                attempt + 1
            );
        }
    }
}

#[test]
fn retained_merchant_list_finishes_retry_ladder_without_a_count_retry() {
    let task = task();
    let plan: OrchestratorPlan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"首页","width":375,"height":812},
        "subtasks":[task]
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: "外卖首页，附近商家列表，中文界面".into(),
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
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(SCRIPTS[0].into())]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let outcome =
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
    assert!(outcome.error.is_none(), "{outcome:?}");
    assert!(outcome.node_count > 0);
    assert_eq!(llm.user_prompts().len(), 1);
    assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
    assert!(!events.iter().any(|event| matches!(
        event,
        Progress::SubtaskRetry { .. } | Progress::SubtaskIncomplete { .. }
    )));
}
