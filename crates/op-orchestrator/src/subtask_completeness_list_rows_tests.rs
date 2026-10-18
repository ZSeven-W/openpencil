//! Real six-dish scripts rejected as five because only one dish had an old price.

use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use op_editor_core::EditorCommand;
use serde_json::json;

const ELEMENTS: &str = "商家详情(头图+菜品分类横滑+菜品列表六条+购物车条) — build exactly what this names, with every count it states; do not add content that belongs to the brief's other sections";
const SCRIPTS: [&str; 2] = [
    include_str!("../tests/fixtures/subtask_completeness/dish-list-1018-attempt1.js"),
    include_str!("../tests/fixtures/subtask_completeness/dish-list-1018-attempt2.js"),
];

fn task() -> Subtask {
    serde_json::from_value(json!({
        "id":"section-2", "label":"商家详情", "elements":ELEMENTS,
        "idPrefix":"section-2", "region":{"width":375,"height":812}
    }))
    .unwrap()
}

fn inserted(script: &str) -> (VecDocSink, SubtaskOutcome) {
    let (nodes, _) =
        crate::script_gen::parse_script(script).expect("retained production script parses");
    let mut sink = VecDocSink::new();
    let ids = sink
        .insert_subtree_returning_root_ids(nodes, &NodeId::NONE)
        .unwrap();
    let outcome = SubtaskOutcome {
        id: "section-2".into(),
        node_count: ids.len(),
        inserted_root_ids: ids,
        error: None,
        headline: None,
        subtask: None,
    };
    (sink, outcome)
}

fn dish_rows(node: &PenNode) -> Vec<String> {
    let Some(children) = node.children() else {
        return vec![];
    };
    // The retained list has six independent rows, each containing one photo.
    if children.len() == 6 && children.iter().all(|child| image_count(child) == 1) {
        return children
            .iter()
            .map(|child| child.id_str().to_owned())
            .collect();
    }
    children.iter().flat_map(dish_rows).collect()
}

fn image_count(node: &PenNode) -> usize {
    usize::from(matches!(node, PenNode::Image(_)))
        + node
            .children()
            .into_iter()
            .flatten()
            .map(image_count)
            .sum::<usize>()
}

#[test]
fn both_retained_six_dish_scripts_add_evidence_to_the_original_false_five() {
    let task = task();
    let text = local_item_text(&task);
    assert_eq!(expected_item_count(&task), Some(6));
    assert!(!media_items::has_local_media_card_contract(&text, 6));
    assert!(list_rows::has_local_list_contract(&text, 6));
    for (attempt, script) in SCRIPTS.iter().enumerate() {
        let (sink, outcome) = inserted(script);
        let rows: Vec<_> = sink
            .state
            .active_children()
            .iter()
            .flat_map(dish_rows)
            .collect();
        assert_eq!(rows.len(), 6, "retained attempt {}", attempt + 1);
        assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 5);
        assert_eq!(
            list_rows::delivered_list_row_count(&sink, &outcome.inserted_root_ids),
            6
        );
        assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
    }
}

#[test]
fn removing_any_dish_from_either_retained_script_still_rejects_five_of_six() {
    for (attempt, script) in SCRIPTS.iter().enumerate() {
        let (sink, outcome) = inserted(script);
        let rows: Vec<_> = sink
            .state
            .active_children()
            .iter()
            .flat_map(dish_rows)
            .collect();
        assert_eq!(rows.len(), 6);
        for row in rows {
            let mut missing = VecDocSink::new();
            missing.state = EditorState::from_document(sink.state.doc.clone());
            assert!(missing.apply(EditorCommand::DeleteNode {
                node_id: NodeId::new(&row),
                page_id: None,
            }));
            assert_eq!(
                list_rows::delivered_list_row_count(&missing, &outcome.inserted_root_ids),
                5
            );
            let failure = incomplete_attempt(&missing, &task(), &outcome)
                .expect("five actual dishes cannot satisfy six");
            assert_eq!(
                (failure.expected, failure.delivered),
                (6, 5),
                "attempt {}, deleted {row}",
                attempt + 1
            );
        }
    }
}

#[test]
fn retained_dish_response_completes_without_a_quota_retry() {
    let task = task();
    let plan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"商家详情","width":375,"height":812},
        "subtasks":[task]
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: "商家详情：头图、菜品分类横滑、菜品列表六条、购物车条；中文界面".into(),
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
