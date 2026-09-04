//! `Orchestrator::run()` —— 四阶段编排主轴(spec §4)。
//!
//! 规划 → 画布搭建 → 子 agent(屏组间可并发)→ 清理。
//! 副作用全经 [`DocSink`] / [`LlmClient`]。
//! 错误 / abort / 零内容语义见 spec §6。
//!
//! dashboard(原 sidebar+main 专用 scaffold)收敛进单根路径:它的 per-subtask
//! 产出与确定性后处理跟通用路径完全一致,只差 scaffold 形状(基准路径没用
//! 上)。**多屏不再收敛**(2026-07-17 修复 multiscreen-fanout-break item A):
//! plan 的 subtask 若带 ≥2 个不同 `screen` 标签,`insert_screen_group_roots`
//! 建 N 个顶层 root(每屏一个)。
//!
//! **屏组间并发(item D-lite,同日跟进)**:`effective_concurrency` =
//! `min(clamp(request.concurrency), groups.len())`;>1 时阶段 3 走
//! `concurrent::run_screen_groups_concurrent`(每屏一个 worker,
//! `FuturesUnordered` 驱动,组内仍是原 3 次重试梯;每个 subtask 通过独立
//! `BufferDocSink` 隔离,成功后按完成顺序以原子 Batch replay 进真实 sink),
//! =1(单组 / 单屏 / append 模式)原样走本文件
//! 未改动的顺序循环 —— 字节级回归锁。`agent_team_size` → `request.concurrency`
//! 至此才真正对经典路径生效;`spawn_agents` 扇出(`spawn_concurrent.rs`)是另
//! 一条独立消费方,不受此影响。

use crate::append::apply_append_context_to_plan;
use crate::cleanup::{descendant_count, finalize_design_with_summary_and_policy, CleanupPolicy};
use crate::model_profile::resolve_model_profile;
use crate::plan::{build_fallback_plan, OrchestratorPlan};
use crate::plan_normalize::{normalize, NormInfo};
use crate::plan_repair::parse_orchestrator_response;
use crate::prompt::build_orchestrator_prompt;
use crate::scaffold::{build_scaffold_at, build_scaffold_reusing};
use crate::screen_groups::group_subtasks_by_screen;
use crate::subagent::{
    apply_command_with_reveal, reveal_now_millis, run_subtask_with_reveal_at_and_outcomes,
};
use crate::types::{
    AbortFlag, DesignRequest, DocSink, LlmChunk, LlmClient, OrchestratorError, PlanningMode,
    Progress, RunSummary, SubtaskOutcome, ValidationProviders,
};
use crate::validation::run_post_generation_validation;
use crate::variables::{rollback, seed_commands, snapshot_plan_vars};
use futures::StreamExt;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

#[path = "run_screen_groups.rs"]
mod run_screen_groups;
use run_screen_groups::insert_screen_group_roots;

// `impl Orchestrator` lives in a sibling module so this file stays a slim
// spine of types + shared helpers; the test modules mounted below are
// unaffected because the impl is inherent (same type, same crate).
#[path = "run_orchestrator.rs"]
mod run_orchestrator;

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

/// Keep late lifecycle events (notably the phase-4.4 salvage pass) attached
/// to the same screen-group persona that owned the subtask during concurrent
/// generation. Sequential runs have no group identities and stay unscoped.
fn scope_progress_for_subtask(
    groups: &[crate::screen_groups::ScreenGroup],
    identities: &[crate::agent_identity::AgentIdentity],
    subtask_index: usize,
    event: Progress,
) -> Progress {
    let Some((group_idx, group)) = groups
        .iter()
        .enumerate()
        .find(|(_, group)| group.indices.contains(&subtask_index))
    else {
        return event;
    };
    let Some(identity) = identities.get(group_idx) else {
        return event;
    };
    Progress::worker_scoped(group_idx, group.screen.clone(), identity.clone(), event)
}

/// Find the id of a direct child of `parent_id` whose name matches `name`.
/// Used to re-resolve the pre-built two-column scaffold's column ids after the
/// `InsertSubtree` remap (the template ids no longer hold).
fn find_child_id_by_name(state: &EditorState, parent_id: &str, name: &str) -> Option<String> {
    let parent = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new(parent_id.to_string()),
    )?;
    parent
        .children()?
        .iter()
        .find(|c| c.base().name.as_deref() == Some(name))
        .map(|c| c.id_str().to_string())
}

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

/// 规划流错误写进日志的最大字符数。要能装下一句 provider 错误 **加上**
/// 传输层附带的子进程输出尾部(`op_util::cli_output::TAIL_MAX_CHARS`),
/// 否则唯一的现场证据会被日志自己截掉。
const STREAM_ERROR_LOG_CHARS: usize = 800;

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
    // TWO attempts before the heuristic fallback: a truncated stream or a
    // transient provider blip fails the parse once and usually succeeds
    // immediately after (measured on the desktop: a rich plan cut mid-JSON →
    // "planning parse failure" → a skeleton fallback design, while the very
    // same prompt parsed fine on retry). The fallback plan stays as the
    // final safety net, not the first response to a hiccup.
    for attempt in 1..=2u8 {
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
                    // A pinned guide outranks whatever the model chose — and
                    // whatever it forgot to choose. Applied here rather than in
                    // the backfill above because the backfill only ever has a
                    // value on the Compact planning path.
                    crate::style_guide_context::enforce_pinned_style_guide(&mut plan, request);
                    let norm = normalize(&mut plan, request);
                    return Ok((plan, norm));
                }
                let preview = raw.trim().chars().take(150).collect::<String>();
                tracing::warn!(
                    attempt,
                    preview = %preview,
                    "planning parse failure"
                );
            }
            // abort 在流中发生 → 立即返回
            Err(error) if error.aborted => return Err(OrchestratorError::Aborted),
            Err(error) => {
                // 带上原因:此前只记 attempt,用户贴来的日志里
                // "planning stream error" 无从区分 429 限流 / 网络中断 /
                // provider 报错,只能靠猜。
                //
                // 上限从 200 提到 STREAM_ERROR_LOG_CHARS:CLI 传输层现在
                // 会把子进程自己的输出尾部(已脱敏、已限长)接在错误后面,
                // 200 字正好把这段新证据全部截掉。
                let reason = error
                    .message
                    .trim()
                    .chars()
                    .take(STREAM_ERROR_LOG_CHARS)
                    .collect::<String>();
                tracing::warn!(attempt, error = %reason, "planning stream error");
            }
        }
        if abort.is_set() {
            return Err(OrchestratorError::Aborted);
        }
    }
    tracing::warn!("planning failed twice; using fallback plan");

    // 规划失败 → fallback plan(规划不可出错)
    let mut fallback = build_fallback_plan(request);
    // The fallback is heuristic, not modelled, so it names no guide at all —
    // without this a pin was lost precisely when planning had already failed
    // twice and the design needed every bit of direction it could get.
    crate::style_guide_context::enforce_pinned_style_guide(&mut fallback, request);
    let norm = normalize(&mut fallback, request);
    Ok((fallback, norm))
}

/// 消费一次 LLM 调用的流 —— 拼接所有 `Text` chunk,丢弃 `Thinking`。
/// 错误原样透出(`aborted` 区分中止 / 真实错误),调用方据此决定是
/// 立即返回还是重试,并把 `message` 写进日志。
async fn collect_text(
    mut stream: futures::stream::BoxStream<'static, Result<LlmChunk, crate::types::LlmError>>,
) -> Result<String, crate::types::LlmError> {
    let mut text = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(LlmChunk::Text(t)) => text.push_str(&t),
            Ok(LlmChunk::Thinking(_)) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(text)
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "run_tests_pinned_style.rs"]
mod tests_pinned_style;

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

// multiscreen-fanout-break fix (item A) — screen-group scaffold tests.
#[cfg(test)]
#[path = "run_tests_screen_groups.rs"]
mod tests_screen_groups;
