//! `Orchestrator::run()` —— 四阶段编排主轴(spec §4)。
//!
//! 规划 → 画布搭建 → 顺序子 agent(或并发 screen-group) → 清理。
//! 副作用全经 [`DocSink`] / [`LlmClient`]。
//! 错误 / abort / 零内容语义见 spec §6。
//!
//! ## S3b-2 Task C2: concurrent path
//! After planning, `effective_concurrency` decides whether to take the
//! concurrent multi-screen path (N-root scaffold + `run_concurrent`) or
//! the existing sequential single-screen path.  The sequential path is
//! completely unchanged.
//!
//! ## S3b-3 Task C3: dashboard column layout
//! In the sequential path, after planning, `should_use_dashboard_columns`
//! decides whether to use the dashboard scaffold (sidebar + main columns)
//! or the existing single-root vertical scaffold.  The concurrent path is
//! NEVER given the dashboard treatment — it is sequential-only (spec §2).
//! The dashboard path implementation lives in `run_dashboard.rs` (split to
//! keep this file under the 800-line ceiling).

use crate::append::apply_append_context_to_plan;
use crate::cleanup::{
    aggregate_concurrent_verdict, cleanup_concurrent_roots, descendant_count, run_cleanup_passes,
};
use crate::concurrent::{effective_concurrency, group_subtasks_by_screen, run_concurrent};
use crate::dashboard_columns::should_use_dashboard_columns;
use crate::model_profile::{resolve_model_profile, ModelTier};
use crate::plan::{build_fallback_plan, OrchestratorPlan};
use crate::plan_normalize::{normalize, NormInfo};
use crate::plan_repair::parse_orchestrator_response;
use crate::prompt::build_orchestrator_prompt;
use crate::retry::{attempt_modes, is_non_retryable};
use crate::run_dashboard::run_dashboard_path;
use crate::scaffold::{
    build_scaffold_at, build_scaffold_concurrent_mobile, build_scaffold_reusing,
};
use crate::subagent::{apply_command_with_reveal, reveal_now_millis, run_subtask_with_reveal_at};
use crate::types::{
    AbortFlag, DesignRequest, DocSink, LlmChunk, LlmClient, OrchestratorError, Progress,
    RunSummary, SubtaskOutcome, ValidationProviders,
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

/// Remove the fresh-canvas starter frame (a single empty top-level
/// container) so a path that builds its OWN root(s) doesn't leave it
/// orphaned beside the design. The sequential path REUSES it instead
/// (see [`detect_reusable_empty_frame`] + `build_scaffold_reusing`); the
/// concurrent (N roots) + dashboard (bespoke sidebar+main root) paths
/// can't reuse a single starter, so they clear it here. This mirrors the
/// host's former `clear_fresh_starter_frame_for_design` — now owned
/// orchestrator-side so the host can leave the starter in place for the
/// reuse path. No-op on a headless / empty canvas (op-smoke), where
/// there is no starter to detect.
fn clear_reusable_empty_frame(sink: &mut dyn DocSink) {
    if let Some(id) = detect_reusable_empty_frame(sink.state()) {
        sink.apply(EditorCommand::DeleteNode {
            node_id: NodeId::new(id),
            page_id: None,
        });
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
    /// ## S3b-2 分支决策
    /// 规划完成后检查 `effective_concurrency`:
    /// - `> 1` → 并发多屏路径(N-root scaffold + `run_concurrent`)。
    /// - `<= 1` → 原有顺序路径,完全不变。
    pub async fn run(
        &self,
        request: DesignRequest,
        sink: &mut dyn DocSink,
        llm: &dyn LlmClient,
        on_progress: &mut dyn FnMut(Progress),
        abort: &AbortFlag,
        providers: &ValidationProviders<'_>,
    ) -> Result<RunSummary, OrchestratorError> {
        // -- 阶段 1:规划(含 mode-rotation 重试循环 + 规范化, S3b-1b Task C2)--
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

        // -- S3b-2 Task C2: concurrency branch decision --
        // Port of `orchestrator.ts:780-810`.
        let screen_groups = group_subtasks_by_screen(&plan.subtasks);

        // -- S3b-4 Task B2 call site 3: effective concurrency gate (TS :806-810) --
        // Append mode is forced sequential — the concurrent branch creates multiple
        // root frames which conflicts with reusing an existing content-root.
        let effective = if append_result.skip_root_insertion {
            1
        } else {
            effective_concurrency(request.concurrency, screen_groups.len())
        };

        if effective > 1 {
            // Concurrent builds N screen roots — can't reuse the single
            // fresh-canvas starter, so clear it (host no longer does).
            clear_reusable_empty_frame(sink);
            return run_concurrent_path(
                request,
                plan,
                norm,
                &screen_groups,
                sink,
                llm,
                on_progress,
                abort,
                providers,
                self.agent_indicator_epoch,
            )
            .await;
        }

        let planned_root_id = plan.root_frame.id.clone();

        // -- S3b-3 Task C3: dashboard branch decision --
        let use_dashboard = should_use_dashboard_columns(&request.prompt, &plan);

        // -- 进入"已动文档"区,全程 undo batch 包裹 --
        sink.begin_undo_batch();
        let var_snapshot = snapshot_plan_vars(sink, &plan);

        // -- 阶段 2:画布搭建 --
        for cmd in seed_commands(&plan, &var_snapshot) {
            sink.apply(cmd);
        }
        // The dashboard path builds a bespoke sidebar+main root that can't
        // reuse the fresh-canvas starter; clear it BEFORE indexing so the
        // dashboard root lands at index 0 (not orphaned beside the starter).
        // The sequential path below keeps the starter and REUSES it via
        // ReplaceSubtree, so it must NOT be cleared here.
        if !append_result.skip_root_insertion && use_dashboard {
            clear_reusable_empty_frame(sink);
        }
        let scaffold_root_index = sink.state().active_children().len();
        let scaffold_root_ids_before: Vec<String> = sink
            .state()
            .active_children()
            .iter()
            .map(|n| n.id_str().to_string())
            .collect();

        // -- S3b-4 Task B2: dashboard / append mutex (spec §2) --
        // Both concurrent and dashboard paths create new root structures that
        // conflict with reusing an existing content-root.  The concurrency gate
        // above already enforces the concurrent/append mutex; here we enforce
        // the dashboard/append mutex by preferring the append fast-path when
        // both would otherwise fire.  Mirrors TS positional precedence — the
        // append fast-path lives inside the sequential else block, before the
        // dashboard sub-branch.
        if !append_result.skip_root_insertion && use_dashboard {
            // ── Dashboard path (extracted to run_dashboard.rs) ────────────
            return run_dashboard_path(
                plan,
                request,
                scaffold_root_index,
                sink,
                llm,
                &var_snapshot,
                on_progress,
                abort,
                providers,
                self.agent_indicator_epoch,
            )
            .await;
        }

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
            run_cleanup_passes(sink, &plan, &new_roots);
        } else {
            run_cleanup_passes(sink, &plan, &[&root_id]);
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

// ── S3b-2 Task C2: concurrent multi-screen path ───────────────────────────────

/// 并发多屏路径(S3b-2 Task C2)。
///
/// Port of `orchestrator.ts:856-1158` concurrent branch.
/// 只在 `effective_concurrency > 1` 时调用;顺序路径不碰此函数。
#[allow(clippy::too_many_arguments)]
async fn run_concurrent_path(
    request: DesignRequest,
    mut plan: OrchestratorPlan,
    norm: NormInfo,
    screen_groups: &[crate::concurrent::ScreenGroup],
    sink: &mut dyn DocSink,
    llm: &dyn LlmClient,
    on_progress: &mut dyn FnMut(Progress),
    abort: &AbortFlag,
    providers: &ValidationProviders<'_>,
    host_epoch: Option<u64>,
) -> Result<RunSummary, OrchestratorError> {
    // -- 进入"已动文档"区 --
    sink.begin_undo_batch();
    let var_snapshot = snapshot_plan_vars(sink, &plan);

    // -- 阶段 2 (并发):变量播种 + N-root scaffold --
    for cmd in seed_commands(&plan, &var_snapshot) {
        sink.apply(cmd);
    }

    // Build N scaffold roots (one per screen group).
    let (scaffold_cmds, _original_root_ids, baselines) =
        match build_scaffold_concurrent_mobile(&plan, screen_groups, norm.is_mobile) {
            Ok(r) => r,
            Err(e) => {
                rollback(sink, &var_snapshot);
                sink.end_undo_batch();
                return Err(OrchestratorError::Internal(e));
            }
        };

    // Record page-child count before inserting N roots.
    let roots_start_index = sink.state().active_children().len();

    for cmd in &scaffold_cmds {
        if !apply_command_with_reveal(sink, cmd.clone(), host_epoch, reveal_now_millis()) {
            rollback(sink, &var_snapshot);
            sink.end_undo_batch();
            return Err(OrchestratorError::Internal(
                "concurrent scaffold insert rejected by document".into(),
            ));
        }
    }

    // Resolve the actual (remapped) root IDs from the live document.
    // Each InsertSubtree appended one child to the active page — capture them
    // in insertion order.
    let actual_root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .skip(roots_start_index)
        .take(screen_groups.len())
        .map(|n| n.id_str().to_string())
        .collect();

    if actual_root_ids.len() != screen_groups.len() {
        rollback(sink, &var_snapshot);
        sink.end_undo_batch();
        return Err(OrchestratorError::Internal(format!(
            "expected {} concurrent scaffold roots, got {}",
            screen_groups.len(),
            actual_root_ids.len()
        )));
    }

    // Assign parent_frame_id for each group's subtasks.
    for (g, group) in screen_groups.iter().enumerate() {
        let root_id = &actual_root_ids[g];
        for &idx in &group.indices {
            if let Some(subtask) = plan.subtasks.get_mut(idx) {
                subtask.parent_frame_id = Some(root_id.clone());
            }
        }
    }

    // Tag each group's root frame with a distinct agent identity so the
    // canvas can paint a per-agent breathing border while the team works.
    let identities = crate::agent_identity::assign_agent_identities(actual_root_ids.len());
    // Adopt the host-minted epoch when present — the host already called
    // `begin` (bumping the epoch + clearing the prior run) at turn start so
    // it can clear immediately on stop. Headless / test callers pass `None`
    // and we mint our own. Either way the guard below clears on every exit
    // path (finish / error / cancelled worker), but only while this run is
    // still the active epoch — a newer run keeps its own indicators.
    let epoch = host_epoch.unwrap_or_else(op_editor_core::agent_indicators::begin);
    for (root_id, identity) in actual_root_ids.iter().zip(identities.iter()) {
        op_editor_core::agent_indicators::add_frame(
            epoch,
            root_id,
            &identity.color,
            &identity.name,
        );
    }
    // Graceful drain on drop: the run's queued reveals keep playing at
    // the queue cadence and the overlay clears itself once the last one
    // lands. A user STOP still kills instantly — the host calls
    // `end_if_epoch` / `clear_if_epoch` directly on that path.
    struct IndicatorGuard(u64);
    impl Drop for IndicatorGuard {
        fn drop(&mut self) {
            op_editor_core::agent_indicators::finish_if_epoch(self.0);
        }
    }
    let _indicator_guard = IndicatorGuard(epoch);

    on_progress(Progress::ScaffoldDone);

    // -- 阶段 3 (并发):run_concurrent --
    // Take a snapshot of current state for worker BufferDocSinks.
    let state_snapshot = sink.state().clone();
    let all_outcomes = run_concurrent(
        screen_groups,
        &plan,
        &request,
        llm,
        abort,
        state_snapshot,
        sink,
        on_progress,
        host_epoch,
    )
    .await;

    // Collect non-None outcomes (workers that ran at least one subtask).
    let collected: Vec<crate::types::SubtaskOutcome> =
        all_outcomes.iter().filter_map(|o| o.clone()).collect();

    // -- 阶段 4 (并发):清理 --
    let root_id_strs: Vec<&str> = actual_root_ids.iter().map(|s| s.as_str()).collect();
    run_cleanup_passes(sink, &plan, &root_id_strs);
    on_progress(Progress::CleanupDone);

    sink.end_undo_batch();

    // -- Run-all-aggregate failure policy (Task C1) --
    // Check AFTER cleanup so the cleanup pass still runs on partial results.
    if let Err(e) = aggregate_concurrent_verdict(&collected) {
        // All workers failed → clean up N roots + roll back variables.
        sink.begin_undo_batch();
        cleanup_concurrent_roots(sink, &root_id_strs, &baselines, &var_snapshot);
        sink.end_undo_batch();
        return Err(e);
    }

    // -- Abort check --
    if abort.is_set() && collected.iter().map(|o| o.node_count).sum::<usize>() == 0 {
        return Err(OrchestratorError::Aborted);
    }

    // -- 阶段 5 (并发):视觉校验 (S3c D1) --
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

    // -- Success: build RunSummary --
    // Use the first surviving root as the "primary" root_frame_id.
    let primary_root_id = actual_root_ids.first().cloned().unwrap_or_default();
    let total_nodes = collected.iter().map(|o| o.node_count).sum();
    Ok(RunSummary {
        root_frame_id: primary_root_id,
        subtasks: collected,
        total_nodes,
    })
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
#[path = "run_tests.rs"]
mod tests;

// Task C2 tests are in a sibling file to keep run.rs under the 800-line cap.
#[cfg(test)]
#[path = "run_tests_c2.rs"]
mod tests_c2;

// Task C3 tests — dashboard column wiring.
#[cfg(test)]
#[path = "run_tests_c3.rs"]
mod tests_c3;

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
