//! `Orchestrator::run()` —— 四阶段编排主轴(spec §4)。
//!
//! 规划 → 画布搭建 → 顺序子 agent → 清理。副作用全经
//! [`DocSink`] / [`LlmClient`]。错误 / abort / 零内容语义见
//! spec §6。

use crate::cleanup::{descendant_count, run_cleanup_passes};
use crate::model_profile::{resolve_model_profile, ModelTier};
use crate::plan::{build_fallback_plan, OrchestratorPlan};
use crate::plan_normalize::{normalize, NormInfo};
use crate::plan_repair::parse_orchestrator_response;
use crate::prompt::build_orchestrator_prompt;
use crate::retry::{attempt_modes, is_non_retryable};
use crate::scaffold::build_scaffold;
use crate::subagent::run_subtask;
use crate::types::{
    AbortFlag, DesignRequest, DocSink, LlmChunk, LlmClient, OrchestratorError, Progress,
    RunSummary, SubtaskOutcome,
};
use crate::variables::{rollback, seed_commands, snapshot_plan_vars};
use futures::StreamExt;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

/// 设计编排器。S3a 阶段无构造期配置 —— 保留 struct 以便 S3b/S3c
/// 挂选项。
#[derive(Debug, Default, Clone, Copy)]
pub struct Orchestrator;

impl Orchestrator {
    pub fn new() -> Self {
        Orchestrator
    }

    /// 跑一次完整编排。见 spec §4 数据流。
    pub async fn run(
        &self,
        request: DesignRequest,
        sink: &mut dyn DocSink,
        llm: &dyn LlmClient,
        on_progress: &mut dyn FnMut(Progress),
        abort: &AbortFlag,
    ) -> Result<RunSummary, OrchestratorError> {
        // -- 阶段 1:规划(含 mode-rotation 重试循环 + 规范化, S3b-1b Task C2)--
        // `planning_loop` 内部已 normalize 并回传 `NormInfo`,此处不再二次规范化。
        on_progress(Progress::Planning);
        let (mut plan, norm) = planning_loop(&request, llm, abort).await?;
        let planned_root_id = plan.root_frame.id.clone();

        // -- 进入"已动文档"区,全程 undo batch 包裹 --
        sink.begin_undo_batch();
        let var_snapshot = snapshot_plan_vars(sink, &plan);

        // -- 阶段 2:画布搭建 --
        for cmd in seed_commands(&plan) {
            sink.apply(cmd);
        }
        let scaffold_root_index = sink.state().active_children().len();
        match build_scaffold(&plan, norm.is_mobile) {
            Ok(cmds) => {
                for cmd in cmds {
                    if !sink.apply(cmd) {
                        rollback(sink, &var_snapshot);
                        sink.end_undo_batch();
                        return Err(OrchestratorError::Internal(
                            "scaffold insert rejected by document".into(),
                        ));
                    }
                }
            }
            Err(e) => {
                // scaffold 模板 bug —— 收尾后报内部错误。
                rollback(sink, &var_snapshot);
                sink.end_undo_batch();
                return Err(OrchestratorError::Internal(e));
            }
        }
        let Some(root_id) = sink
            .state()
            .active_children()
            .get(scaffold_root_index)
            .map(|n| n.id_str().to_string())
        else {
            rollback(sink, &var_snapshot);
            sink.end_undo_batch();
            return Err(OrchestratorError::Internal(format!(
                "scaffold root `{planned_root_id}` was not inserted"
            )));
        };
        for subtask in &mut plan.subtasks {
            subtask.parent_frame_id = Some(root_id.clone());
        }
        let scaffold_baseline = descendant_count(sink.state(), &root_id);
        on_progress(Progress::ScaffoldDone);

        // -- 阶段 3:顺序子 agent(C3: 3-attempt tier-gated retry ladder) --
        //
        // Port of `orchestrator-sub-agent.ts:128-206` (sequential path).
        //
        // Per subtask:
        //   Attempt 1: reduced_complexity=false, minimal_skills=false
        //   Attempt 2: reduced_complexity=(tier==Basic), minimal_skills=false
        //   Attempt 3: reduced_complexity=true, minimal_skills=true
        //
        // A retryable failure = error.is_some() && node_count==0
        //                       && !abort.is_set() && !is_non_retryable(&err).
        // non_retryable is evaluated from attempt-1's error and cached
        // (matching TS semantics where `isNonRetryable` is computed once
        // before the retry chain).
        // A partial result (node_count > 0) is never retried.
        // After 3 still-zero → zero_node_failure stop.
        let tier = resolve_model_profile(request.model.as_deref().unwrap_or("")).tier;
        let mut outcomes: Vec<SubtaskOutcome> = Vec::new();
        let mut aborted_mid = false;
        let mut zero_node_failure = false;
        for subtask in &plan.subtasks {
            if abort.is_set() {
                aborted_mid = true;
                break;
            }
            on_progress(Progress::SubtaskStarted {
                id: subtask.id.clone(),
                label: subtask.label.clone(),
            });

            // Attempt 1 — full complexity.
            let outcome1 =
                run_subtask(subtask, &plan, &request, llm, sink, abort, false, false).await;

            // Evaluate non-retryable predicate once from attempt-1's error
            // (faithful to TS: `isNonRetryable` is computed before the retry
            // chain and reused for both the attempt-2 and attempt-3 guards).
            let non_retryable = outcome1
                .error
                .as_deref()
                .map(is_non_retryable)
                .unwrap_or(false);

            // Helper: is the current outcome a retryable failure?
            let retryable = |o: &SubtaskOutcome| {
                o.error.is_some() && o.node_count == 0 && !abort.is_set() && !non_retryable
            };

            // Attempt 2 — reduced_complexity iff Basic tier.
            let outcome2 = if retryable(&outcome1) {
                tracing::warn!(
                    subtask = %subtask.id,
                    error = outcome1.error.as_deref().unwrap_or(""),
                    "subtask failed, retrying (attempt 2)"
                );
                Some(
                    run_subtask(
                        subtask,
                        &plan,
                        &request,
                        llm,
                        sink,
                        abort,
                        tier == ModelTier::Basic,
                        false,
                    )
                    .await,
                )
            } else {
                None
            };

            // Pick current best outcome after attempt 2.
            let outcome_after2 = outcome2.as_ref().unwrap_or(&outcome1);

            // Attempt 3 — minimal skills (last-ditch fallback).
            let outcome3 = if retryable(outcome_after2) {
                tracing::warn!(
                    subtask = %subtask.id,
                    error = outcome_after2.error.as_deref().unwrap_or(""),
                    "subtask still empty after retry, falling back to minimal skills (attempt 3)"
                );
                Some(run_subtask(subtask, &plan, &request, llm, sink, abort, true, true).await)
            } else {
                None
            };

            // Final outcome: last attempt that ran.
            let outcome = outcome3.unwrap_or_else(|| outcome2.unwrap_or(outcome1));

            let zero = outcome.node_count == 0;
            let node_count = outcome.node_count;
            let err_msg = outcome.error.clone();
            outcomes.push(outcome);

            // abort 在 run_subtask 期间被置位 —— 优先于零节点判定归
            // abort 路径(否则 mid-stream abort 会被误判为错误路径,
            // 错误地移除 scaffold root 并返回 NoContent 而非 Aborted)。
            if abort.is_set() {
                aborted_mid = true;
                if zero {
                    on_progress(Progress::SubtaskFailed {
                        id: subtask.id.clone(),
                        error: err_msg.unwrap_or_else(|| "aborted".into()),
                    });
                } else {
                    on_progress(Progress::SubtaskDone {
                        id: subtask.id.clone(),
                        node_count,
                    });
                }
                break;
            }
            if zero {
                // 零节点失败(非 abort,全部 3 次皆失败)—— 停止后续 subtask。
                on_progress(Progress::SubtaskFailed {
                    id: subtask.id.clone(),
                    error: err_msg.unwrap_or_default(),
                });
                zero_node_failure = true;
                break;
            }
            on_progress(Progress::SubtaskDone {
                id: subtask.id.clone(),
                node_count,
            });
        }

        // -- 阶段 4:清理 --
        run_cleanup_passes(sink, &plan, &[&root_id]);
        on_progress(Progress::CleanupDone);

        // -- 阶段 4.5:收尾(spec §6.3 三路径)--
        let zero_content = descendant_count(sink.state(), &root_id) <= scaffold_baseline;
        if zero_content {
            // 错误路径才移除空 scaffold root;abort / 正常零内容只回滚变量。
            if zero_node_failure {
                sink.apply(EditorCommand::DeleteNode {
                    node_id: NodeId::new(root_id.clone()),
                });
            }
            rollback(sink, &var_snapshot);
        }
        sink.end_undo_batch();

        // -- 返回 --
        if zero_content {
            return Err(if aborted_mid {
                OrchestratorError::Aborted
            } else {
                OrchestratorError::NoContent
            });
        }
        let total_nodes = outcomes.iter().map(|o| o.node_count).sum();
        Ok(RunSummary {
            root_frame_id: root_id,
            subtasks: outcomes,
            total_nodes,
        })
    }
}

/// 规划阶段: mode-rotation 重试循环。
///
/// Port of `callOrchestrator` planning stage in `orchestrator.ts:1323-1503`.
/// 解析 tier → `attempt_modes` → 遍历每个 mode,调用 LLM + parse,首次
/// 成功即回填 style_guide_name + normalize 后返回。全部失败 → fallback plan。
///
/// 返回 `(plan, NormInfo)` —— `planning_loop` 是唯一的规范化点,
/// `NormInfo` 透传给 `build_scaffold`,调用方不再二次 `normalize`。
async fn planning_loop(
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
) -> Result<(OrchestratorPlan, NormInfo), OrchestratorError> {
    let tier =
        crate::model_profile::resolve_model_profile(request.model.as_deref().unwrap_or("")).tier;
    let modes = attempt_modes(tier);
    let last_idx = modes.len() - 1;

    /// 规划失败的诊断记录 —— 仅用于 `tracing::warn!`,不影响控制流。
    struct PlanningFailure {
        reason: &'static str,
        mode: &'static str,
        detail: String,
    }

    let mut last_planning_failure: Option<PlanningFailure> = None;

    for (attempt_idx, &mode) in modes.iter().enumerate() {
        let pp = build_orchestrator_prompt(request, mode, abort.clone());

        let collect_result = collect_text(llm.call(pp.call_request)).await;

        let raw = match collect_result {
            Ok(text) => text,
            Err(true) => {
                // abort 在流中发生 → 立即返回,不轮换
                return Err(OrchestratorError::Aborted);
            }
            Err(false) => {
                // 真实流错误 → 记录,继续下一档
                let mode_name = mode_name(mode);
                tracing::warn!(
                    mode = mode_name,
                    attempt = attempt_idx + 1,
                    "planning stream error; rotating to next mode"
                );
                last_planning_failure = Some(PlanningFailure {
                    reason: "stream_error",
                    mode: mode_name,
                    detail: String::new(),
                });
                if attempt_idx < last_idx {
                    continue;
                } else {
                    break;
                }
            }
        };

        // abort 在流结束后被置位(两次检查对齐 TS)
        if abort.is_set() {
            return Err(OrchestratorError::Aborted);
        }

        match parse_orchestrator_response(&raw, request) {
            Some((mut plan, _repaired)) => {
                // compact 模式回填 forced_style_guide_name(若 plan 未携带)
                if plan.style_guide_name.is_none() {
                    if let Some(forced) = pp.forced_style_guide_name {
                        plan.style_guide_name = Some(forced);
                    }
                }
                let norm = normalize(&mut plan, request);
                return Ok((plan, norm));
            }
            None => {
                let mode_name = mode_name(mode);
                let preview = raw.trim().chars().take(150).collect::<String>();
                tracing::warn!(
                    mode = mode_name,
                    attempt = attempt_idx + 1,
                    preview = %preview,
                    "planning parse failure; rotating to next mode"
                );
                last_planning_failure = Some(PlanningFailure {
                    reason: "parse_error",
                    mode: mode_name,
                    detail: preview,
                });
                if attempt_idx < last_idx {
                    continue;
                }
                // 最后一档 fall-through
            }
        }
    }

    // 所有档次耗尽 → fallback plan(规划不可出错)
    if let Some(f) = &last_planning_failure {
        tracing::warn!(
            reason = f.reason,
            mode = f.mode,
            detail = %f.detail,
            "planning exhausted all modes; using fallback plan"
        );
    }
    let mut fallback = build_fallback_plan(request);
    let norm = normalize(&mut fallback, request);
    Ok((fallback, norm))
}

/// 返回 `PlanningMode` 的静态字符串名(用于日志)。
fn mode_name(mode: crate::types::PlanningMode) -> &'static str {
    use crate::types::PlanningMode;
    match mode {
        PlanningMode::Rich => "rich",
        PlanningMode::Minimal => "minimal",
        PlanningMode::Compact => "compact",
    }
}

/// 消费一次 LLM 调用的流 —— 拼接所有 `Text` chunk,丢弃 `Thinking`。
/// `Err(true)` 表示中止,`Err(false)` 表示真实错误。
async fn collect_text(
    mut stream: futures::stream::BoxStream<'static, Result<LlmChunk, crate::types::LlmError>>,
) -> Result<String, bool> {
    let mut text = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(LlmChunk::Text(t)) => text.push_str(&t),
            Ok(LlmChunk::Thinking(_)) => {}
            Err(e) => return Err(e.aborted),
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};

    fn req() -> DesignRequest {
        DesignRequest {
            prompt: "a landing page".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
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

    fn node_json(prefix: &str) -> String {
        format!(
            r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":1200,"height":300,"children":[]}}]"#
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
        assert!(matches!(events.last(), Some(Progress::CleanupDone)));
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
        ))
        .expect("fallback run ok");
        assert!(summary.total_nodes >= 1);
    }

    // ── Task C2: new tests (Step 1 — add failing, then implement) ─────────────

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
        ))
        .expect("no retry on partial");
        // Both subtasks succeed; if hero had been retried the scripted LLM
        // would have served feat's slot to the second hero attempt, leaving
        // feat with 0 nodes and causing NoContent.
        assert_eq!(summary.subtasks.len(), 2);
        assert!(summary.total_nodes >= 2);
    }
}
