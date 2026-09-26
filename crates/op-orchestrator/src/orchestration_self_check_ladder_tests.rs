//! Retry-ladder behaviour of intent- vs contract-class self-check findings:
//! `section-structure-drift` rejects attempts 1-2 (the retry nudge) but is
//! advisory on attempt 3, so a drifting section is never dropped; a
//! contract-class finding still fails the subtask on every rung.

use crate::concurrent::run_subtask_retry_ladder_with_outcomes;
use crate::model_profile::ModelTier;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress, SubtaskOutcome};

fn subtask(width: f64) -> Subtask {
    Subtask {
        id: "board".into(),
        label: "Task board".into(),
        region: Region {
            width,
            height: 600.0,
        },
        bleed_hero: false,
        id_prefix: "board".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    }
}

fn request() -> DesignRequest {
    DesignRequest {
        prompt: "A project task board".into(),
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

fn plan(task: &Subtask, width: f64) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width,
            height: 900.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![task.clone()],
        style_guide_name: None,
    }
}

fn run_ladder(width: f64, responses: Vec<String>) -> (SubtaskOutcome, Vec<Progress>) {
    let task = subtask(width);
    let plan = plan(&task, width);
    let llm = ScriptedLlm::new(responses.into_iter().map(ScriptResponse::Text).collect());
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |progress| events.push(progress);
    let outcome = futures::executor::block_on(run_subtask_retry_ladder_with_outcomes(
        &task,
        &plan,
        &request(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        ModelTier::Full,
        None,
        &GeometryEchoBudget::new(0),
        &mut on_progress,
        &[],
    ));
    (outcome, events)
}

fn retry_reasons(events: &[Progress]) -> Vec<(u32, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            Progress::SubtaskRetry {
                attempt, reason, ..
            } => Some((u32::from(*attempt), reason.clone())),
            _ => None,
        })
        .collect()
}

const CARD: &str = r#"{"type":"frame","name":"Task Card","layout":"vertical","children":[{"type":"text","content":"Write the spec"},{"type":"text","content":"Due Friday"}]}"#;
const CARD_WITH_TAG: &str = r#"{"type":"frame","name":"Task Card","layout":"vertical","children":[{"type":"text","content":"Write the spec"},{"type":"text","content":"Due Friday"},{"type":"icon_font","name":"Flag","iconFontName":"flag","width":16,"height":16}]}"#;

/// Four task cards, two of them with an optional flag icon: 2/4 structures
/// is drift under the 2/3 majority rule.
fn drifting_board() -> String {
    let cards = [CARD, CARD_WITH_TAG, CARD, CARD_WITH_TAG]
        .iter()
        .map(|card| format!("I(sec,{card});"))
        .collect::<String>();
    format!(
        r#"const sec=I(null,{{"type":"frame","name":"Task Board","layout":"horizontal"}});{cards}"#
    )
}

fn uniform_board() -> String {
    let cards = (0..4)
        .map(|_| format!("I(sec,{CARD});"))
        .collect::<String>();
    format!(
        r#"const sec=I(null,{{"type":"frame","name":"Task Board","layout":"horizontal"}});{cards}"#
    )
}

#[test]
fn drift_on_every_rung_is_accepted_on_the_final_attempt() {
    let (outcome, events) = run_ladder(1280.0, vec![drifting_board(); 3]);

    let retries = retry_reasons(&events);
    assert_eq!(
        retries
            .iter()
            .map(|(attempt, _)| *attempt)
            .collect::<Vec<_>>(),
        vec![2, 3],
        "attempts 1 and 2 must still reject on drift: {retries:?}"
    );
    assert!(
        retries
            .iter()
            .all(|(_, reason)| reason.contains("section-structure-drift")),
        "the retry nudge must echo the drift finding: {retries:?}"
    );
    assert!(
        outcome.error.is_none() && outcome.node_count > 0,
        "the final attempt must keep the section: {outcome:?}"
    );
}

#[test]
fn drift_still_rejects_a_non_final_attempt() {
    let (outcome, events) = run_ladder(1280.0, vec![drifting_board(), uniform_board()]);

    let retries = retry_reasons(&events);
    assert_eq!(retries.len(), 1, "{retries:?}");
    assert_eq!(retries[0].0, 2);
    assert!(
        retries[0].1.contains("section-structure-drift"),
        "{retries:?}"
    );
    assert!(
        outcome.error.is_none() && outcome.node_count > 0,
        "{outcome:?}"
    );
}

/// The 0815 "法则" lesion: five items, five structures — rejected on the
/// first attempt exactly as before.
#[test]
fn five_structure_rule_family_is_rejected_on_the_first_attempt() {
    let items = [
        r#"{"type":"frame","name":"Rule 01","layout":"vertical","children":[{"type":"text","content":"First"}]}"#,
        r#"{"type":"frame","name":"Rule 02","layout":"horizontal","children":[{"type":"text","content":"Second"},{"type":"frame","name":"Inner","layout":"vertical","children":[{"type":"text","content":"Line"}]}]}"#,
        r##"{"type":"frame","name":"Rule 03","layout":"vertical","children":[{"type":"rectangle","name":"Bar","width":40,"height":4,"fill":[{"type":"solid","color":"#111111"}]},{"type":"text","content":"Third"}]}"##,
        r#"{"type":"frame","name":"Rule 04","layout":"vertical","children":[{"type":"icon_font","name":"Mark","iconFontName":"check","width":16,"height":16},{"type":"text","content":"Fourth"}]}"#,
        r#"{"type":"frame","name":"Rule 05","layout":"vertical","children":[{"type":"text","content":"Fifth"},{"type":"icon_font","name":"Tail","iconFontName":"star","width":16,"height":16}]}"#,
    ]
    .iter()
    .map(|item| format!("I(sec,{item});"))
    .collect::<String>();
    let script = format!(
        r#"const sec=I(null,{{"type":"frame","name":"Rules","layout":"vertical"}});{items}"#
    );
    let (_, events) = run_ladder(1280.0, vec![script, uniform_board()]);

    let retries = retry_reasons(&events);
    assert!(
        retries.first().is_some_and(
            |(attempt, reason)| *attempt == 2 && reason.contains("section-structure-drift")
        ),
        "{retries:?}"
    );
}

/// Contract-class findings keep failing the subtask on the final attempt:
/// only intent findings became advisory.
#[test]
fn contract_finding_still_fails_the_final_attempt() {
    // A frame NAMED as a steps ring that renders no ring: a fact about the
    // screenshot, not a structure opinion.
    let script = r#"const sec=I(null,{"type":"frame","name":"Activity","layout":"vertical"});I(sec,{"type":"frame","name":"Steps Ring","width":124,"height":124,"layout":"vertical","alignItems":"center","justifyContent":"center","children":[{"type":"text","content":"8,432"},{"type":"text","content":"steps"}]});"#.to_string();
    let (outcome, events) = run_ladder(1280.0, vec![script; 4]);

    let retries = retry_reasons(&events);
    assert_eq!(
        retries
            .iter()
            .map(|(attempt, _)| *attempt)
            .collect::<Vec<_>>(),
        vec![2, 3],
        "{retries:?}"
    );
    assert_eq!(outcome.node_count, 0, "{outcome:?}");
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.contains("missing-progress-ring")),
        "{outcome:?}"
    );
}
