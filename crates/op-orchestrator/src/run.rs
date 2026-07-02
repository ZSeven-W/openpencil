//! `Orchestrator::run()` —— 四阶段编排主轴(spec §4)。
//!
//! 规划 → 画布搭建 → 顺序子 agent → 清理。
//! 副作用全经 [`DocSink`] / [`LlmClient`]。
//! 错误 / abort / 零内容语义见 spec §6。
//!
//! 单一顺序路径:多屏(原并发 screen-group)与 dashboard(原 sidebar+main
//! 专用 scaffold)都收敛进这条路径 —— 它们的 per-subtask 产出与确定性后处理
//! 完全一致,仅差并发并行度 / scaffold 形状(默认 + 基准路径都没用上)。
//! `concurrency` 字段仍由独立的 `spawn_agents` 扇出(`spawn_concurrent.rs`)消费。

use crate::append::apply_append_context_to_plan;
use crate::cleanup::{descendant_count, finalize_design};
use crate::model_profile::{resolve_model_profile, ModelTier};
use crate::plan::{build_fallback_plan, OrchestratorPlan};
use crate::plan_normalize::{normalize, NormInfo};
use crate::plan_repair::parse_orchestrator_response;
use crate::prompt::build_orchestrator_prompt;
use crate::retry::is_non_retryable;
use crate::scaffold::{build_scaffold_at, build_scaffold_reusing};
use crate::subagent::{apply_command_with_reveal, reveal_now_millis, run_subtask_with_reveal_at};
use crate::types::{
    AbortFlag, DesignRequest, DocSink, LlmChunk, LlmClient, OrchestratorError, PlanningMode,
    Progress, RunSummary, SubtaskOutcome, ValidationProviders,
};
use crate::validation::run_post_generation_validation;
use crate::variables::{rollback, seed_commands, snapshot_plan_vars};
use futures::StreamExt;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

/// TS `replaceEmptyFrame` parity: detect a single EMPTY top-level frame (the
/// fresh-canvas starter) that can be REUSED as the design root instead of
/// inserting a brand-new root. Returns its id when the active page holds
/// exactly one empty container; `None` otherwise (multi-node canvas, filled
/// frame, or non-container) so the normal insert path runs.
fn detect_reusable_empty_frame(state: &EditorState) -> Option<String> {
    let kids = state.active_children();
    if kids.len() != 1 {
        return None;
    }
    let node = &kids[0];
    if node.is_container() && node.children().map(|c| c.is_empty()).unwrap_or(true) {
        Some(node.id_str().to_string())
    } else {
        None
    }
}

const FOLLOW_ON_ROOT_GAP: f64 = 80.0;
const DEFAULT_ROOT_X: f64 = 80.0;
const DEFAULT_ROOT_Y: f64 = 40.0;

fn next_root_insert_position(state: &EditorState, planned_width: f64) -> (f64, f64) {
    let mut rightmost: Option<f64> = None;
    let mut top: Option<f64> = None;
    for node in state.active_children() {
        let x = node.base().x.unwrap_or(DEFAULT_ROOT_X);
        let y = node.base().y.unwrap_or(DEFAULT_ROOT_Y);
        let width = node.width_px().unwrap_or(planned_width).max(1.0);
        rightmost = Some(rightmost.map_or(x + width, |current| current.max(x + width)));
        top = Some(top.map_or(y, |current| current.min(y)));
    }
    match rightmost {
        Some(right) => (right + FOLLOW_ON_ROOT_GAP, top.unwrap_or(DEFAULT_ROOT_Y)),
        None => (DEFAULT_ROOT_X, DEFAULT_ROOT_Y),
    }
}

/// 设计编排器。
#[derive(Debug, Default, Clone, Copy)]
pub struct Orchestrator {
    /// Run epoch for the agent-team canvas indicators. The host owns the
    /// design-turn lifecycle, so it mints the epoch (`agent_indicators::
    /// begin`) and clears via `clear_if_epoch` the instant the turn is
    /// stopped — registration in the concurrent path must run under that
    /// same epoch. `None` for headless / test callers, which then let the
    /// concurrent path mint its own epoch.
    agent_indicator_epoch: Option<u64>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adopt a host-owned indicator epoch. The host clears with
    /// `agent_indicators::clear_if_epoch(epoch)` on stop / new-chat, so
    /// the concurrent path registers under this epoch instead of minting
    /// its own — otherwise the host couldn't target the right run.
    pub fn with_indicator_epoch(mut self, epoch: u64) -> Self {
        self.agent_indicator_epoch = Some(epoch);
        self
    }

    /// 跑一次完整编排。见 spec §4 数据流。
    ///
    /// 规划 → 单根 scaffold → 顺序子 agent → 清理 的单一路径。
    pub async fn run(
        &self,
        request: DesignRequest,
        sink: &mut dyn DocSink,
        llm: &dyn LlmClient,
        on_progress: &mut dyn FnMut(Progress),
        abort: &AbortFlag,
        providers: &ValidationProviders<'_>,
    ) -> Result<RunSummary, OrchestratorError> {
        // -- 阶段 1:规划(单档 Rich + 规范化)--
        // `planning_loop` 内部已 normalize 并回传 `NormInfo`,此处不再二次规范化。
        on_progress(Progress::Planning);
        let (mut plan, norm) = planning_loop(&request, llm, abort).await?;

        // -- S3b-4 Task B2 call site 1: apply append context (TS :737) --
        // Must run AFTER planning_loop (which calls normalize) so root_frame.id
        // and subtasks are already normalized before we repoint them.
        let append_result =
            apply_append_context_to_plan(&mut plan, request.append_context.as_ref());

        // Surface the FULL planned task list upfront (TS parity) so the UI can
        // render the complete checklist immediately, rather than revealing
        // subtasks one-by-one as each starts.
        on_progress(Progress::Planned {
            subtasks: plan
                .subtasks
                .iter()
                .map(|s| (s.id.clone(), s.label.clone()))
                .collect(),
        });

        // The orchestrator runs a single sequential path. Multi-screen designs
        // (formerly the concurrent branch) and dashboards (formerly a bespoke
        // sidebar+main scaffold) flow through this same single-root sequential
        // pipeline — they produced byte-identical per-subtask output, only
        // differing in wall-clock parallelism / scaffold shape, neither of which
        // the default + benchmarked path used. The `concurrency` field is still
        // honored by the separate `spawn_agents` fan-out (`spawn_concurrent.rs`).
        let planned_root_id = plan.root_frame.id.clone();

        // -- 进入"已动文档"区,全程 undo batch 包裹 --
        sink.begin_undo_batch();
        let var_snapshot = snapshot_plan_vars(sink, &plan);

        // -- 阶段 2:画布搭建 --
        for cmd in seed_commands(&plan, &var_snapshot) {
            sink.apply(cmd);
        }
        let scaffold_root_ids_before: Vec<String> = sink
            .state()
            .active_children()
            .iter()
            .map(|n| n.id_str().to_string())
            .collect();

        let sequential_identity = if append_result.skip_root_insertion {
            None
        } else {
            crate::agent_identity::assign_agent_identities(1)
                .into_iter()
                .next()
        };

        let (root_id, scaffold_baseline) = if append_result.skip_root_insertion {
            let target_id = plan.root_frame.id.clone();
            for subtask in &mut plan.subtasks {
                subtask.parent_frame_id = Some(target_id.clone());
            }
            let baseline = descendant_count(sink.state(), &target_id);
            on_progress(Progress::ScaffoldDone);
            (target_id, baseline)
        } else {
            let effective_is_mobile = norm.is_mobile && !append_result.skip_status_bar;
            // TS `replaceEmptyFrame` parity: when the canvas is a single empty
            // top-level frame (the fresh-canvas starter), REUSE it as the design
            // root (ReplaceSubtree in place) instead of inserting a brand-new
            // root — which the host would otherwise clear + re-add, the visible
            // "delete then re-draw" flash the user flagged.
            let reuse_id = detect_reusable_empty_frame(sink.state());
            let (insert_x, insert_y) =
                next_root_insert_position(sink.state(), plan.root_frame.width);
            let scaffold_cmds = match reuse_id.as_deref() {
                Some(id) => build_scaffold_reusing(&plan, effective_is_mobile, id),
                None => build_scaffold_at(&plan, effective_is_mobile, insert_x, insert_y),
            };
            match scaffold_cmds {
                Ok(cmds) => {
                    for cmd in cmds {
                        if !apply_command_with_reveal(
                            sink,
                            cmd,
                            self.agent_indicator_epoch,
                            reveal_now_millis(),
                        ) {
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
            let Some(rid) = sink.state().active_children().iter().find_map(|n| {
                let id = n.id_str();
                (!scaffold_root_ids_before.iter().any(|old| old == id)).then(|| id.to_string())
            }) else {
                rollback(sink, &var_snapshot);
                sink.end_undo_batch();
                return Err(OrchestratorError::Internal(format!(
                    "scaffold root `{planned_root_id}` was not inserted"
                )));
            };
            for subtask in &mut plan.subtasks {
                subtask.parent_frame_id = Some(rid.clone());
            }
            if let (Some(epoch), Some(identity)) =
                (self.agent_indicator_epoch, sequential_identity.as_ref())
            {
                op_editor_core::agent_indicators::add_frame(
                    epoch,
                    &rid,
                    &identity.color,
                    &identity.name,
                );
            }
            let baseline = descendant_count(sink.state(), &rid);
            on_progress(Progress::ScaffoldDone);
            (rid, baseline)
        };

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

            // Attempt 1 — full complexity. SubtaskSkills fires via on_progress.
            let outcome1 = run_subtask_with_reveal_at(
                subtask,
                &plan,
                &request,
                llm,
                sink,
                abort,
                false,
                false,
                self.agent_indicator_epoch,
                reveal_now_millis(),
                Some(&mut *on_progress),
            )
            .await;

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
                on_progress(Progress::SubtaskRetry {
                    id: subtask.id.clone(),
                    attempt: 2,
                    reason: outcome1
                        .error
                        .clone()
                        .unwrap_or_else(|| "zero nodes generated".into()),
                });
                Some(
                    run_subtask_with_reveal_at(
                        subtask,
                        &plan,
                        &request,
                        llm,
                        sink,
                        abort,
                        tier == ModelTier::Basic,
                        false,
                        self.agent_indicator_epoch,
                        reveal_now_millis(),
                        None,
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
                on_progress(Progress::SubtaskRetry {
                    id: subtask.id.clone(),
                    attempt: 3,
                    reason: outcome_after2
                        .error
                        .clone()
                        .unwrap_or_else(|| "zero nodes generated".into()),
                });
                Some(
                    run_subtask_with_reveal_at(
                        subtask,
                        &plan,
                        &request,
                        llm,
                        sink,
                        abort,
                        true,
                        true,
                        self.agent_indicator_epoch,
                        reveal_now_millis(),
                        None,
                    )
                    .await,
                )
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
                // 零节点失败(非 abort,全部 3 次皆失败)。**不 break** —— 一个
                // section 失败不该放弃后续所有 subtask。各 subtask 独立
                // InsertSubtree 到 root、互不依赖;break 会把失败点之后的必要
                // 内容(bottom nav 等)全丢掉(用户报的"管线丢内容")。跳过这个、
                // 继续后面的;`zero_node_failure` 仍标记"至少一个失败",最终若
                // **全部**零内容(zero_content)才删 scaffold root。
                on_progress(Progress::SubtaskFailed {
                    id: subtask.id.clone(),
                    error: err_msg.unwrap_or_default(),
                });
                zero_node_failure = true;
                continue;
            }
            on_progress(Progress::SubtaskDone {
                id: subtask.id.clone(),
                node_count,
            });
        }

        // -- 阶段 4:清理 --
        // Append mode (skip_root_insertion): scope cleanup to ONLY the roots
        // this run inserted (post-remap ids from each outcome) so pre-existing
        // nodes under the target frame are never restyled (Component 11b).
        // If inserted_root_ids is empty (nothing inserted, or buffered sink),
        // the empty slice is a safe no-op — do NOT fall back to the whole target
        // root, which would reprocess old nodes.
        //
        // Fresh-document mode reuses the single page/target root — every node
        // under it is new — so the behaviour is unchanged there.
        if append_result.skip_root_insertion {
            let new_roots: Vec<&str> = outcomes
                .iter()
                .flat_map(|o| o.inserted_root_ids.iter().map(String::as_str))
                .collect();
            finalize_design(sink, &plan, &new_roots);
        } else {
            finalize_design(sink, &plan, &[&root_id]);
        }
        on_progress(Progress::CleanupDone);

        // -- 阶段 4.5:收尾(spec §6.3 三路径)--
        let zero_content = descendant_count(sink.state(), &root_id) <= scaffold_baseline;
        if zero_content {
            // 错误路径才移除空 scaffold root;abort / 正常零内容只回滚变量。
            if zero_node_failure {
                sink.apply(EditorCommand::DeleteNode {
                    node_id: NodeId::new(root_id.clone()),
                    page_id: None,
                });
            }
            rollback(sink, &var_snapshot);
        }
        sink.end_undo_batch();

        // -- 返回 --
        if zero_content {
            return Err(if aborted_mid {
                OrchestratorError::Aborted
            } else if let Some(first_error) = outcomes
                .iter()
                .find_map(|o| o.error.as_deref().filter(|s| !s.is_empty()))
            {
                OrchestratorError::AllFailed(first_error.to_string())
            } else {
                OrchestratorError::NoContent
            });
        }

        // -- 阶段 5:视觉校验 (S3c D1) — 在 cleanup 后、返回 RunSummary 前 --
        // Port of `orchestrator.ts:1247-1292`.
        // 守卫: request.validation_enabled && !abort.is_set().
        if request.validation_enabled && !abort.is_set() {
            let _ = run_post_generation_validation(
                sink,
                providers.pre_validator,
                providers.screenshot,
                providers.vision,
                &providers.system_prompt,
                &request,
                on_progress,
                abort,
            );
        }

        let total_nodes = outcomes.iter().map(|o| o.node_count).sum();
        Ok(RunSummary {
            root_frame_id: root_id,
            subtasks: outcomes,
            total_nodes,
        })
    }
}

/// 规划阶段: 单档(Rich)规划 + fallback。
///
/// Port of `callOrchestrator` planning stage in `orchestrator.ts:1323-1503`,
/// simplified to a SINGLE planning mode (`Rich` — the full prompt). The former
/// tier-driven mode-rotation ladder (Rich→Minimal→Compact) was machinery around
/// the deterministic core, not part of it: one LLM call builds the plan, and any
/// failure (stream error / unparseable) falls straight through to the
/// heuristic `build_fallback_plan`. The per-subtask retry ladder (the actual
/// weak-model quality lever) and `build_orchestrator_prompt`'s prompt
/// construction are untouched.
///
/// 返回 `(plan, NormInfo)` —— `planning_loop` 是唯一的规范化点,
/// `NormInfo` 透传给 `build_scaffold`,调用方不再二次 `normalize`。
async fn planning_loop(
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
) -> Result<(OrchestratorPlan, NormInfo), OrchestratorError> {
    let pp = build_orchestrator_prompt(request, PlanningMode::Rich, abort.clone());
    let forced_style_guide_name = pp.forced_style_guide_name.clone();

    match collect_text(llm.call(pp.call_request)).await {
        Ok(raw) => {
            // abort 在流结束后被置位(两次检查对齐 TS)
            if abort.is_set() {
                return Err(OrchestratorError::Aborted);
            }
            if let Some((mut plan, _repaired)) = parse_orchestrator_response(&raw, request) {
                // 回填 forced_style_guide_name(若 plan 未携带)
                if plan.style_guide_name.is_none() {
                    if let Some(forced) = forced_style_guide_name {
                        plan.style_guide_name = Some(forced);
                    }
                }
                let norm = normalize(&mut plan, request);
                return Ok((plan, norm));
            }
            let preview = raw.trim().chars().take(150).collect::<String>();
            tracing::warn!(
                preview = %preview,
                "planning parse failure; using fallback plan"
            );
        }
        Err(true) => {
            // abort 在流中发生 → 立即返回
            return Err(OrchestratorError::Aborted);
        }
        Err(false) => {
            tracing::warn!("planning stream error; using fallback plan");
        }
    }

    // 规划失败 → fallback plan(规划不可出错)
    let mut fallback = build_fallback_plan(request);
    let norm = normalize(&mut fallback, request);
    Ok((fallback, norm))
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
#[path = "run_tests.rs"]
mod tests;

// Task B2 (S3b-4) tests — append-to-document mode wiring.
#[cfg(test)]
#[path = "run_tests_b4.rs"]
mod tests_b4;

// Task D1 (S3c) tests — vision validation wiring across all paths.
#[cfg(test)]
#[path = "run_tests_d1.rs"]
mod tests_d1;

// Task F5 — backward-compat regression: append leaves pre-existing styled node byte-identical.
#[cfg(test)]
#[path = "run_tests_f5.rs"]
mod tests_f5;
