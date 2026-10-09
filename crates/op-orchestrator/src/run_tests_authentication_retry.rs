//! A later authentication failure must not spend another model-call rung.

use super::*;
use crate::LlmError;

fn progress_event(progress: &Progress) -> &Progress {
    match progress {
        Progress::WorkerScoped(worker) => progress_event(worker.event.as_ref()),
        event => event,
    }
}

fn authentication_failure() -> ScriptResponse {
    ScriptResponse::Fail(LlmError {
        message: "Antigravity is not authenticated. Run `agy` once in a terminal.".into(),
        aborted: false,
    })
}

#[test]
fn planner_authentication_failure_does_not_retry_or_create_a_fallback() {
    let llm = ScriptedLlm::new(vec![authentication_failure()]);
    let mut sink = VecDocSink::new();
    let mut events = vec![];
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut |p| events.push(p),
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert!(result.is_err());
    assert_eq!(llm.system_prompts().len(), 1);
    assert!(!events.iter().any(|p| matches!(
        progress_event(p),
        Progress::ScaffoldDone | Progress::SubtaskStarted { .. }
    )));
    assert!(sink.state.active_children().is_empty());
    assert_eq!(sink.batch_depth, 0);
}

#[test]
fn transient_then_authentication_failure_stops_before_attempt_three_and_salvage() {
    let mut plan: serde_json::Value = serde_json::from_str(PLAN_JSON).unwrap();
    plan["subtasks"].as_array_mut().unwrap().truncate(1);
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(plan.to_string()),
        ScriptResponse::Fail(LlmError {
            message: "socket closed unexpectedly".into(),
            aborted: false,
        }),
        authentication_failure(),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = vec![];
    let _ = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut |p| events.push(p),
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert_eq!(
        llm.system_prompts().len(),
        3,
        "planning + transient + auth failure only"
    );
    assert!(events
        .iter()
        .any(|p| matches!(progress_event(p), Progress::SubtaskRetry { attempt: 2, .. })));
    assert!(!events
        .iter()
        .any(|p| matches!(progress_event(p), Progress::SubtaskRetry { attempt: 3, .. })));
    assert_eq!(sink.batch_depth, 0);
}
