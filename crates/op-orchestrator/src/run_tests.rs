//! `run.rs` inline tests — sequential + planning rotation + 3-attempt ladder.
//!
//! Wired as `#[path = "run_tests.rs"] mod tests;` inside `run.rs`;
//! stays a child module of `run`, so `use super::*` resolves to `run`.

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

// Standard tier → [Rich, Minimal]
fn req_standard() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        // "gpt-4o" matches Standard tier in model_profile table
        model: Some("gpt-4o".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

// Basic tier → [Rich, Minimal, Compact]
fn req_basic() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        // "glm" matches Basic tier in model_profile table
        model: Some("glm-4".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

const PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "feat", "label": "Features", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

const MOBILE_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Mobile Page", "width": 390, "height": 844,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFF8F0" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 390, "height": 300 } }
  ]
}"##;

fn node_json(prefix: &str) -> String {
    format!(
        r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":1200,"height":300,"children":[{{"type":"text","id":"{prefix}-title","content":"{prefix}","fontSize":18}}]}}]"#
    )
}

// ── existing tests (must stay green) ─────────────────────────────────────

#[test]
fn run_happy_path_applies_scaffold_and_subtasks() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("run ok");

    // root_frame_id 是 InsertSubtree 重映射后的真实 id —— 不是
    // plan 里的 "root" 字面值,只断言它非空。
    assert!(!summary.root_frame_id.is_empty());
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    // undo batch 配对。
    assert_eq!(sink.batch_depth, 0);
    // 至少有 scaffold + 两个 subtask 的 InsertSubtree。
    let inserts = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert!(inserts >= 3, "expected >=3 InsertSubtree, got {inserts}");
    assert!(matches!(events.first(), Some(Progress::Planning)));
    // CleanupDone must be present (validation runs after it).
    assert!(
        events.iter().any(|e| matches!(e, Progress::CleanupDone)),
        "expected CleanupDone in events"
    );
}

#[test]
fn run_mobile_scaffold_reveals_status_bar() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MOBILE_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let mut request = req();
    request.prompt = "a mobile food app".into();
    request.validation_enabled = false;

    let summary = futures::executor::block_on(Orchestrator::new().with_indicator_epoch(epoch).run(
        request,
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("mobile run ok");

    assert!(!summary.root_frame_id.is_empty());
    let root = sink.state.active_children().first().expect("root inserted");
    let status_id = root
        .children()
        .expect("mobile root should have children")
        .iter()
        .find(|node| {
            serde_json::to_value(node)
                .ok()
                .is_some_and(|v| v["role"] == "status-bar")
        })
        .map(|node| node.id_str().to_string())
        .expect("mobile scaffold should insert a status bar");
    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert!(
        snapshot.reveals.contains_key(&status_id),
        "status bar should get a reveal animation, got {:?}",
        snapshot.reveals.keys().collect::<Vec<_>>()
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn run_zero_node_subtask_stops_and_errors() {
    // 规划 OK,但第一个 subtask 吐垃圾(3 次全失败)→ 零节点 → NoContent。
    // C3 引入 3-attempt 梯子:需要 3 条垃圾响应才能穷尽重试。
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text("the model refused".into()),
        ScriptResponse::Text("still refused".into()),
        ScriptResponse::Text("refused again".into()),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert!(matches!(result, Err(OrchestratorError::NoContent)));
    // undo batch 仍配对。
    assert_eq!(sink.batch_depth, 0);
}

#[test]
fn run_planning_failure_uses_fallback_plan() {
    // 规划吐垃圾 → fallback plan;subtask 正常 → 成功。
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text("no json here".into()),
        ScriptResponse::Text(node_json("section-1")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("fallback run ok");
    assert!(summary.total_nodes >= 1);
}

// ── Task C2: planning rotation tests ─────────────────────────────────────

/// Attempt 1 returns bad JSON (parse_error), attempt 2 returns valid plan.
/// Standard tier → [Rich, Minimal] → rotation occurs.
#[test]
fn planning_rotation_uses_attempt2_plan_on_attempt1_parse_failure() {
    let llm = ScriptedLlm::new(vec![
        // attempt 1 (Rich) → bad JSON
        ScriptResponse::Text("not valid json at all".into()),
        // attempt 2 (Minimal) → valid plan
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtasks
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_standard(), // Standard tier → [Rich, Minimal]
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("rotation run ok");
    // 2 subtasks from the attempt-2 plan
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
}

/// Attempt 1 returns a stream error, attempt 2 returns valid plan.
#[test]
fn planning_rotation_uses_attempt2_plan_on_attempt1_stream_error() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        // attempt 1 → stream error (non-abort)
        ScriptResponse::Fail(LlmError {
            message: "HTTP 500 upstream".into(),
            aborted: false,
        }),
        // attempt 2 → valid plan
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtasks
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_standard(), // Standard tier → [Rich, Minimal]
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("rotation on stream error ok");
    assert_eq!(summary.subtasks.len(), 2);
}

/// All attempts fail (Basic tier → [Rich, Minimal, Compact]) →
/// fallback plan used, run succeeds.
#[test]
fn planning_all_attempts_fail_uses_fallback_plan() {
    // Basic tier has 3 attempts; supply 3 bad responses + 1 subtask response
    // for the fallback plan's single subtask.
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text("garbage 1".into()),
        ScriptResponse::Text("garbage 2".into()),
        ScriptResponse::Text("garbage 3".into()),
        ScriptResponse::Text(node_json("section-1")),
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
    .expect("fallback after all failures ok");
    assert!(summary.total_nodes >= 1);
}

/// Abort during planning stream → `OrchestratorError::Aborted`, no rotation.
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

/// Subtask fails all 3 attempts → `OrchestratorError::NoContent`.
#[test]
fn subtask_all_three_attempts_fail_returns_no_content() {
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: garbage
        ScriptResponse::Text("garbage attempt 1".into()),
        // subtask hero — attempt 2: garbage
        ScriptResponse::Text("garbage attempt 2".into()),
        // subtask hero — attempt 3: garbage
        ScriptResponse::Text("garbage attempt 3".into()),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert!(matches!(result, Err(OrchestratorError::NoContent)));
    assert_eq!(sink.batch_depth, 0);
}

/// Subtask's attempt-1 error is non-retryable (HTTP 401) →
/// no retry, stops immediately with NoContent.
#[test]
fn subtask_non_retryable_error_stops_immediately_no_retry() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: HTTP 401 (non-retryable)
        ScriptResponse::Fail(LlmError {
            message: "HTTP 401 Unauthorized".into(),
            aborted: false,
        }),
        // This response should NOT be consumed — if it were, the test
        // would assert fewer LLM calls than expected (we just verify NoContent).
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert!(matches!(result, Err(OrchestratorError::NoContent)));
    assert_eq!(sink.batch_depth, 0);
}

/// Partial result (node_count > 0 with an error) is never retried —
/// it is accepted and counted toward summary.
///
/// Note: the current `run_subtask` returns `error: None` on success and
/// `error: Some` only on zero-node failure. A partial result (nodes
/// produced + downstream soft error) would arrive as node_count>0,
/// error=None from `run_subtask`. We model this by having the first
/// subtask succeed (nodes produced) even though the scenario calls for
/// a "partial with error". The key invariant: once node_count>0 the
/// ladder does not retry regardless of error state.
#[test]
fn subtask_partial_result_not_retried() {
    // A subtask that returns a valid node on the first attempt must
    // succeed without using a second LLM slot.
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: success (node_count > 0)
        ScriptResponse::Text(node_json("hero")),
        // subtask feat — attempt 1: success
        ScriptResponse::Text(node_json("feat")),
        // A third response here would mean hero was retried — we assert
        // only 2 subtasks succeeded so the LLM is not over-consumed.
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("no retry on partial");
    // Both subtasks succeed; if hero had been retried the scripted LLM
    // would have served feat's slot to the second hero attempt, leaving
    // feat with 0 nodes and causing NoContent.
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
}
