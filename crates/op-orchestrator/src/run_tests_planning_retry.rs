//! Single-mode planning failures and the sub-agent 3-attempt tier-gated
//! retry ladder.

use super::*;

// ── single-mode planning tests ───────────────────────────────────────────

/// Planning parse failure → heuristic fallback plan used, run succeeds.
/// (Single-mode planning: one bad response falls straight through to the
/// fallback plan — no mode rotation. The fallback plan has one subtask.)
#[test]
fn planning_parse_failure_uses_fallback_plan() {
    let llm = ScriptedLlm::new(vec![
        // planning attempts 1 + 2 → bad JSON (the retry consumes one more)
        ScriptResponse::Text("not valid json at all".into()),
        ScriptResponse::Text("not valid json at all".into()),
        // fallback plan's single subtask
        ScriptResponse::Text(node_json("section-1")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_standard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("fallback after parse failure ok");
    assert!(summary.total_nodes >= 1);
}

/// Planning stream error → heuristic fallback plan used, run succeeds.
#[test]
fn planning_stream_error_uses_fallback_plan() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        // planning attempts 1 + 2 → stream error (non-abort); the retry
        // consumes the second before the heuristic fallback engages.
        ScriptResponse::Fail(LlmError {
            message: "HTTP 500 upstream".into(),
            aborted: false,
        }),
        ScriptResponse::Fail(LlmError {
            message: "HTTP 500 upstream".into(),
            aborted: false,
        }),
        // fallback plan's single subtask
        ScriptResponse::Text(node_json("section-1")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_standard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("fallback on stream error ok");
    assert!(summary.total_nodes >= 1);
}

/// Abort during planning stream → `OrchestratorError::Aborted`.
#[test]
fn planning_abort_during_stream_returns_aborted() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
        message: "user aborted".into(),
        aborted: true,
    })]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let abort = AbortFlag::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &abort,
        &stub_providers(),
    ));
    assert!(matches!(result, Err(OrchestratorError::Aborted)));
    // undo batch 在 abort 路径前返回,文档不应已进入批
    assert_eq!(sink.batch_depth, 0);
}

// ── Task C3: sub-agent 3-attempt tier-gated retry ladder ──────────────────

/// Subtask returns zero nodes on attempt 1 but succeeds on attempt 2 →
/// the subtask's nodes land (ladder retries once).
/// Uses Full tier (attempt 2: reduced_complexity=false, minimal_skills=false).
#[test]
fn subtask_retries_on_attempt1_zero_succeeds_on_attempt2() {
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: garbage (0 nodes, retryable)
        ScriptResponse::Text("the model gave garbage".into()),
        // subtask hero — attempt 2: success
        ScriptResponse::Text(node_json("hero")),
        // subtask feat — attempt 1: success
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(), // Full tier → reduced_complexity=false on attempt 2
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("retry succeeded");
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    assert_eq!(sink.batch_depth, 0);
}

/// A self-check quality rejection on attempt 1 must NOT downgrade attempt
/// 2's skill tier, even on a Basic-tier model that would otherwise always
/// narrow to `retryAllowed` on retry — the content was real, just flagged
/// for one geometry issue, so throwing skills away only makes the rest of
/// the design worse. Attempt 2's prompt must also carry the rejection
/// reason so the model can fix exactly that issue.
#[test]
fn self_check_rejection_keeps_full_skills_and_injects_feedback_on_attempt2() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        // hero attempt 1 (Basic tier, full complexity): parses fine, but
        // self-check fatally rejects the mismatched ring — zero nodes land.
        ScriptResponse::Text(radial_reject_script()),
        // hero attempt 2: must stay full complexity despite Basic tier.
        ScriptResponse::Text(node_json("hero")),
        // feat attempt 1: succeeds normally.
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_basic(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("attempt 2 recovers after the self-check rejection");
    assert_eq!(summary.subtasks.len(), 2);

    // Call order: [0] planning, [1] hero attempt 1, [2] hero attempt 2, [3] feat.
    let prompts = llm.system_prompts();
    assert_eq!(prompts.len(), 4, "unexpected call count: {prompts:?}");
    assert_eq!(
        prompts[1], prompts[2],
        "attempt 2 after a self-check rejection must resolve the IDENTICAL \
         (full) skill set attempt 1 used — a Basic-tier model would \
         otherwise narrow this to the retryAllowed set"
    );
}

/// The mirror case: an attempt-1 TRANSPORT failure (not a self-check
/// rejection) on a Basic-tier model must still downgrade attempt 2 to
/// `reduced_complexity`, exactly as before this task — skill downgrade
/// stays reserved for failures that suggest the model is struggling with
/// the full prompt, not for a quality gate on otherwise-fine content.
#[test]
fn transport_failure_still_downgrades_skills_on_attempt2_for_basic_tier() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        // hero attempt 1: a stream error — NOT a self-check rejection.
        ScriptResponse::Fail(LlmError {
            message: "stream disconnected before completion".into(),
            aborted: false,
        }),
        // hero attempt 2: reduced_complexity narrows the skill set.
        ScriptResponse::Text(node_json("hero")),
        // feat attempt 1: succeeds normally.
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_basic(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("attempt 2 recovers after the transport failure");
    assert_eq!(summary.subtasks.len(), 2);

    let prompts = llm.system_prompts();
    assert_eq!(prompts.len(), 4, "unexpected call count: {prompts:?}");
    assert!(
        prompts[2].len() < prompts[1].len(),
        "attempt 2 after a plain transport failure must still narrow to the \
         reduced-complexity skill set on Basic tier (attempt 1: {} chars, \
         attempt 2: {} chars)",
        prompts[1].len(),
        prompts[2].len()
    );
}

const COVERING_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "pricing", "label": "Pricing Table", "region": { "width": 1200, "height": 400 } },
    { "id": "faq", "label": "FAQ", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

fn enumerated_req() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page with hero, pricing table and FAQ".into(),
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

fn planning_prompt_count(llm: &ScriptedLlm, brief: &str) -> usize {
    llm.user_prompts()
        .iter()
        .filter(|prompt| prompt.starts_with(brief))
        .count()
}

/// First plan omits enumerated sections → exactly one re-plan carrying the
/// coverage feedback paragraph.
#[test]
fn missing_plan_section_issues_one_replan_with_feedback() {
    let brief = enumerated_req().prompt;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(COVERING_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("pricing")),
        ScriptResponse::Text(node_json("faq")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        enumerated_req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("coverage re-plan run succeeds");

    assert_eq!(planning_prompt_count(&llm, &brief), 2);
    let feedback = crate::plan_coverage::coverage_feedback(&["pricing table".into(), "FAQ".into()]);
    assert!(
        llm.user_prompts()[1].contains(&feedback),
        "second planning prompt must carry coverage feedback, got {}",
        llm.user_prompts()[1]
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::PlanCoverageRetry { missing }
            if missing == &["pricing table".to_string(), "FAQ".to_string()]
    )));
}

/// A plan that already covers every enumerated section is requested once.
#[test]
fn covering_plan_issues_exactly_one_plan_request() {
    let brief = enumerated_req().prompt;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(COVERING_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("pricing")),
        ScriptResponse::Text(node_json("faq")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        enumerated_req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("covering plan run succeeds");

    assert_eq!(planning_prompt_count(&llm, &brief), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::PlanCoverageRetry { .. })));
}

/// A second plan that still misses sections is kept; the gate never loops.
#[test]
fn second_incomplete_plan_is_kept_without_another_retry() {
    let brief = enumerated_req().prompt;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        enumerated_req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("second incomplete plan still runs");

    assert_eq!(planning_prompt_count(&llm, &brief), 2);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Progress::PlanCoverageRetry { .. }))
            .count(),
        1
    );
}
