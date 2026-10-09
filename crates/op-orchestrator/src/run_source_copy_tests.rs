use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use serde_json::json;

const BRIEF: &str = "请把以下资料排成活动卡，保留全部原文，不添加事实。\n晴日咖啡 · 静安店\n2026年10月17日\n14:30–16:00\n拿铁 28元\n美式 22元\n燕麦拿铁 32元\n到店自取，不含配送。";

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

fn plan() -> String {
    json!({"rootFrame":{"id":"root","name":"活动卡","width":1080,"height":1440,"layout":"vertical"},"subtasks":[{"id":"card","label":"活动卡","region":{"width":1080,"height":1440}}]}).to_string()
}

fn script(complete: bool) -> String {
    let copy = if complete {
        BRIEF.split_once('\n').unwrap().1
    } else {
        "晴日咖啡 · 静安店"
    };
    let children: Vec<_> = copy.lines().map(|line| json!({"type":"text","content":line,"fontSize":36,"width":"fill_container","height":"fit_content","textGrowth":"fixed-width"})).collect();
    format!(
        "I(null, {});",
        json!({"type":"frame","name":"活动卡内容","width":"fill_container","height":"fit_content","layout":"vertical","gap":36,"children":children})
    )
}

#[test]
fn missing_source_copy_retries_with_exact_facts_then_delivers_one_complete_design() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(plan()),
        ScriptResponse::Text(script(false)),
        ScriptResponse::Text(script(true)),
    ]);
    let request = DesignRequest {
        prompt: BRIEF.into(),
        model: Some("glm-5.3-flash".into()),
        validation_enabled: false,
        ..Default::default()
    };
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let summary = futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut |p| events.push(p),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .unwrap();
    assert!(summary.subtasks.iter().all(|s| s.error.is_none()));
    let prompts = llm.user_prompts();
    assert_eq!(
        prompts.len(),
        3,
        "one planning call and two generation attempts"
    );
    assert!(prompts[2].contains("source-copy-missing") && prompts[2].contains("燕麦拿铁 32元"));
    assert!(events.iter().any(|p| matches!(p, Progress::SubtaskRetry { attempt:2, reason, .. } if reason.contains("source-copy-missing"))));
    let audit = crate::quality_audit::audit_final_quality_with_brief(
        &sink.state,
        &[summary.root_frame_id],
        BRIEF,
    );
    assert!(!audit
        .remaining
        .iter()
        .any(|item| item.source == "source-copy-missing"));
}

#[test]
fn exhausted_source_copy_failures_cannot_be_reported_as_success() {
    let llm = ScriptedLlm::new(
        std::iter::once(ScriptResponse::Text(plan()))
            .chain((0..4).map(|_| ScriptResponse::Text(script(false))))
            .collect(),
    );
    let request = DesignRequest {
        prompt: BRIEF.into(),
        validation_enabled: false,
        ..Default::default()
    };
    let mut sink = VecDocSink::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert!(
        matches!(result, Err(OrchestratorError::AllFailed(ref reason)) if reason.contains("source-copy-missing")),
        "{result:?}"
    );
    assert!(
        sink.state.active_children().is_empty(),
        "incomplete board must not survive as a completed work"
    );
}

#[test]
fn denied_model_access_never_spends_subtask_or_fallback_calls() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(crate::types::LlmError {
        message: "Model access denied by the current subscription (provider code 1311, HTTP 429)"
            .into(),
        aborted: false,
    })]);
    let request = DesignRequest {
        prompt: "a simple landing page".into(),
        ..Default::default()
    };
    let mut sink = VecDocSink::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert!(
        matches!(result, Err(OrchestratorError::AllFailed(ref reason)) if reason.contains("Model access denied"))
    );
    assert_eq!(
        llm.user_prompts().len(),
        1,
        "permission cannot recover through a re-plan"
    );
    assert!(
        sink.applied.is_empty(),
        "do not draw an empty fallback on access failure"
    );
}
