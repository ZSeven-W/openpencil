//! Supplied copy is owned, retried and delivered across actual screen groups.

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use serde_json::json;

const FIRST: &str = "先做1页试稿，核对原始资料，再决定是否扩展为多页。";
const SECOND: &str = "正文不要靠缩小字号塞进版面；可以调整留白、行长和分组。";

fn request() -> DesignRequest {
    DesignRequest {
        prompt: format!(
            "做2张1080×1440中文图文卡。保留全部原文，以下内容逐条分配。\n{FIRST}\n{SECOND}"
        ),
        model: Some("glm-5.3-flash".into()),
        validation_enabled: false,
        ..Default::default()
    }
}

fn plan(complete: bool) -> String {
    json!({"rootFrame":{"id":"root","name":"图文卡","width":1080,"height":1440,"layout":"vertical"},
        "subtasks":[
            {"id":"first","label":"第一张","screen":"first","elements":if complete {FIRST} else {"先做一页"},"region":{"width":1080,"height":1440}},
            {"id":"second","label":"第二张","screen":"second","elements":SECOND,"region":{"width":1080,"height":1440}}
        ]}).to_string()
}

fn script(line: &str) -> String {
    format!(
        "I(null, {});",
        json!({
            "type":"frame","width":"fill_container","height":"fit_content","layout":"vertical",
            "children":[{"type":"text","content":line,"fontFamily":"Noto Sans SC","fontSize":40,"width":"fill_container","height":"fit_content","textGrowth":"fixed-width"}],
        })
    )
}

fn run(
    llm: &ScriptedLlm,
    sink: &mut VecDocSink,
    events: &mut Vec<Progress>,
) -> Result<RunSummary, OrchestratorError> {
    futures::executor::block_on(Orchestrator::new().run(
        request(),
        sink,
        llm,
        &mut |p| events.push(p),
        &AbortFlag::new(),
        &ValidationProviders {
            pre_validator: &SkippedPreValidator,
            screenshot: &SkippedScreenshotProvider,
            vision: &SkippedVisionLlmClient,
            system_prompt: String::new(),
        },
    ))
}

#[test]
fn missing_plan_copy_is_replanned_and_only_the_faulty_page_is_regenerated() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(plan(false)),
        ScriptResponse::Text(plan(true)),
        ScriptResponse::Text(script("先做一页")),
        ScriptResponse::Text(script(FIRST)),
        ScriptResponse::Text(script(SECOND)),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let summary = run(&llm, &mut sink, &mut events).expect("complete multi-page copy");
    assert_eq!(sink.state.active_children().len(), 2);
    assert!(summary.subtasks.iter().all(|task| task.error.is_none()));
    assert!(events
        .iter()
        .any(|p| matches!(p, Progress::PlanCoverageRetry { missing } if missing == &[FIRST])));
    assert!(events.iter().any(|p| matches!(p, Progress::SubtaskRetry { id, reason, .. } if id == "first" && reason.contains("source-copy-missing"))));
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 5, "one re-plan and one page retry only");
    assert!(prompts[1].contains("SOURCE COPY FIX REQUIRED"));
    let assigned = prompts[4]
        .split("THIS SECTION'S SUPPLIED COPY:")
        .nth(1)
        .unwrap();
    assert!(assigned.contains(SECOND));
    assert!(
        !assigned.contains(FIRST),
        "do not repeat all source on every page"
    );
    let copy = crate::source_copy::SourceCopy::from_brief(&request().prompt);
    assert!(copy
        .missing(&sink.state, sink.state.active_children())
        .is_empty());
    let first = &sink.state.active_children()[0];
    assert_eq!(
        copy.missing(&sink.state, std::slice::from_ref(first)),
        [SECOND]
    );
}

#[test]
fn exhausted_plan_copy_failure_leaves_the_existing_document_untouched() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(plan(false)),
        ScriptResponse::Text(plan(false)),
    ]);
    let mut sink = VecDocSink::new();
    sink.state.doc.name = Some("Existing work".into());
    let before = serde_json::to_value(&sink.state.doc).unwrap();
    let result = run(&llm, &mut sink, &mut Vec::new());
    assert!(
        matches!(result, Err(OrchestratorError::AllFailed(reason)) if reason.contains("source-copy-missing"))
    );
    assert_eq!(serde_json::to_value(&sink.state.doc).unwrap(), before);
    assert!(sink.applied.is_empty());
    assert_eq!(
        llm.user_prompts().len(),
        2,
        "no generation or fallback after a failed copy re-plan"
    );
}
