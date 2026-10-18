//! Retained dish responses must deliver their explicitly promised cart bar.

use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use serde_json::json;

const ELEMENTS: &str = "商家详情(头图+菜品分类横滑+菜品列表六条+购物车条) — build exactly what this names, with every count it states; do not add content that belongs to the brief's other sections";
const SCRIPTS: [&str; 3] = [
    include_str!("../tests/fixtures/subtask_completeness/dish-list-1018-attempt1.js"),
    include_str!("../tests/fixtures/subtask_completeness/dish-list-1018-attempt2.js"),
    include_str!("../tests/fixtures/subtask_completeness/dish-list-1018-attempt3.js"),
];

fn task() -> Subtask {
    serde_json::from_value(json!({
        "id":"section-2", "label":"商家详情", "elements":ELEMENTS,
        "idPrefix":"section-2", "region":{"width":375,"height":812}
    }))
    .unwrap()
}

fn inserted(script: &str) -> (VecDocSink, SubtaskOutcome) {
    // Preserve the old response bytes. New expression syntax validation rejects
    // bare toast copy; the shared builder/executor still reconstructs that tree.
    let (nodes, _) = match crate::script_gen::parse_script(script) {
        Ok(result) => result,
        Err(error) => {
            assert!(error.to_string().contains("toast"), "{error}");
            let program = op_mcp::script_runner::run_generation_script_to_program(script).unwrap();
            crate::program_gen::run_program_to_forest(&program).unwrap()
        }
    };
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

#[test]
fn bottom_action_both_retained_cart_bars_supply_presence_evidence() {
    for (index, script) in SCRIPTS[..2].iter().enumerate() {
        let (sink, outcome) = inserted(script);
        // The second response is static and absolutely positioned. Presence
        // does not certify quantity, amount, state bindings, or event behavior.
        assert!(
            bottom_action::inspect(&sink, &task(), &outcome).is_none(),
            "retained attempt {} contains a real cart bar",
            index + 1
        );
        assert!(incomplete_attempt(&sink, &task(), &outcome).is_none());
    }
}

#[test]
fn bottom_action_six_delivered_dishes_cannot_substitute_for_a_missing_cart_bar() {
    let (sink, outcome) = inserted(SCRIPTS[2]);
    assert_eq!(expected_item_count(&task()), Some(6));
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 6);
    let failure = bottom_action::inspect(&sink, &task(), &outcome)
        .expect("dish add buttons and an empty bottom spacer are not a cart bar");
    assert_eq!((failure.expected, failure.delivered), (1, 0));
    assert!(failure.feedback.contains("bottom business bar"));
    let shared_failure = incomplete_attempt(&sink, &task(), &outcome)
        .expect("the shared gate must preserve the independent presence failure");
    assert_eq!(shared_failure, failure);
}

fn ladder(responses: &[&str]) -> (VecDocSink, SubtaskOutcome, Vec<Progress>, Vec<String>) {
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
    let llm = ScriptedLlm::new(
        responses
            .iter()
            .map(|script| ScriptResponse::Text(valid_actions(script)))
            .collect(),
    );
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
    (sink, outcome, events, llm.user_prompts())
}

#[test]
fn bottom_action_presence_feedback_retries_then_accepts_the_retained_first_response() {
    let (missing_sink, missing_outcome) = inserted(SCRIPTS[2]);
    let failure = bottom_action::inspect(&missing_sink, &task(), &missing_outcome).unwrap();
    // Exactly two responses are supplied: a valid second attempt must not ask
    // for another retry or consume a geometry echo response.
    let (sink, outcome, events, prompts) = ladder(&[SCRIPTS[2], SCRIPTS[0]]);
    assert!(outcome.error.is_none(), "{outcome:?}");
    assert!(outcome.node_count > 0);
    assert_eq!(prompts.len(), 2);
    assert!(prompts[1].contains(&failure.feedback));
    assert_eq!(
        sink.state.active_children().len(),
        1,
        "the incomplete first response must be rolled back"
    );
    let retries: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Progress::SubtaskRetry {
                attempt, reason, ..
            } => Some((*attempt, reason.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(retries, vec![(2, failure.feedback.as_str())]);
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskIncomplete { .. })));
    assert!(bottom_action::inspect(&sink, &task(), &outcome).is_none());
    assert!(incomplete_attempt(&sink, &task(), &outcome).is_none());
}

#[test]
fn bottom_action_three_missing_attempts_retain_content_with_an_incomplete_terminal_state() {
    let (sink, outcome, events, prompts) = ladder(&[SCRIPTS[2], SCRIPTS[2], SCRIPTS[2]]);
    assert_eq!(prompts.len(), 3);
    assert!(outcome.node_count > 0);
    assert!(outcome.subtask.is_none());
    assert!(is_incomplete_outcome(&outcome));
    assert_eq!(
        sink.state.active_children().len(),
        1,
        "only the final incomplete response remains"
    );
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 6);
    let failure = bottom_action::inspect(&sink, &task(), &outcome).unwrap();
    assert!(failure.feedback.contains("bottom business bar"));
    assert_eq!(outcome.error.as_deref(), Some(failure.feedback.as_str()));
    for prompt in &prompts[1..] {
        assert!(prompt.contains(&failure.feedback));
    }
    let retries: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Progress::SubtaskRetry {
                attempt, reason, ..
            } => Some((*attempt, reason.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        retries,
        vec![
            (2, failure.feedback.as_str()),
            (3, failure.feedback.as_str())
        ]
    );
    let incomplete: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Progress::SubtaskIncomplete {
                expected,
                delivered,
                ..
            } => Some((*expected, *delivered)),
            _ => None,
        })
        .collect();
    assert_eq!(incomplete, vec![(1, 0)]);
}

#[test]
fn retained_bare_toast_is_rejected_before_the_independent_structural_check() {
    let error = crate::script_gen::parse_script(SCRIPTS[0]).unwrap_err();
    assert!(error.to_string().contains("toast"), "{error}");
    let error = crate::script_gen::parse_script(SCRIPTS[2]).unwrap_err();
    assert!(error.to_string().contains("toast"), "{error}");
    for script in SCRIPTS {
        crate::script_gen::parse_script(&valid_actions(script)).unwrap();
    }
}

fn valid_actions(script: &str) -> String {
    // Only quote historical literal toast messages as runtime string literals.
    Regex::new(r#"toast:"([^"]*)""#)
        .unwrap()
        .replace_all(script, r#"toast:"'$1'""#)
        .into_owned()
}
