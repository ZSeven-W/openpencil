//! Phase 4.4 — the end-of-run salvage pass, extracted verbatim from
//! `run_orchestrator.rs`'s `run()` to keep that driver under the 800-line
//! spine budget.
//!
//! 瞬时故障(供应商网络抖动、偶发空回复)会把一个 subtask 的 3 次紧挨着的
//! 尝试全部烧掉(measured:Ark 连续 3 次 "empty content from provider" →
//! 侧栏子任务整段消失,设计**无侧栏出厂**且无可见信号)。其余 subtask 跑完
//! 后隔了几十秒再给每个失败者最后一次完整尝试 —— 瞬时故障此时多已恢复;
//! 仍失败的维持 SubtaskFailed,不再重试。Reuse attempt 3's minimal skill tier
//! and persisted subtask feedback. This keeps deterministic self-check
//! guidance while still giving transient provider failures one final recovery
//! window.

use super::*;

/// Re-runs every salvagable failed subtask once, in plan order. Mutates
/// `outcomes` in place and flips `*aborted_mid` if the abort flag fires
/// mid-pass. Returns the `(zero_node_failure, incomplete_subtask_failure)`
/// tallies — the PASSED-IN values unchanged when the pass does not run (an
/// empty `salvage` or a preset abort flag must preserve phase 3's tallies,
/// e.g. an incomplete subtask delivers nodes and never enters `salvage`),
/// recomputed over the final outcomes when it does.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_salvage_pass(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    groups: &[crate::screen_groups::ScreenGroup],
    group_identities: &[crate::agent_identity::AgentIdentity],
    agent_indicator_epoch: Option<u64>,
    salvage: Vec<(usize, usize)>,
    outcomes: &mut [SubtaskOutcome],
    aborted_mid: &mut bool,
    mut zero_node_failure: bool,
    mut incomplete_subtask_failure: bool,
    on_progress: &mut dyn FnMut(Progress),
) -> (bool, bool) {
    if salvage.is_empty() || abort.is_set() {
        return (zero_node_failure, incomplete_subtask_failure);
    }
    for (subtask_index, outcome_index) in salvage {
        if abort.is_set() {
            *aborted_mid = true;
            break;
        }
        if !crate::run_salvage_feedback::should_salvage(outcomes.get(outcome_index)) {
            continue;
        }
        let subtask = crate::run_salvage_feedback::subtask_for_salvage(
            outcomes.get(outcome_index),
            &plan.subtasks[subtask_index],
        );
        let prior_outcomes = outcomes.get(..outcome_index).unwrap_or(&[]);
        on_progress(scope_progress_for_subtask(
            groups,
            group_identities,
            subtask_index,
            Progress::SubtaskRetry {
                id: subtask.id.clone(),
                attempt: 4,
                reason: "salvage pass after transient failures".into(),
            },
        ));
        let mut outcome = run_subtask_with_reveal_at_and_outcomes(
            &subtask,
            plan,
            request,
            llm,
            sink,
            abort,
            true,
            true,
            agent_indicator_epoch,
            reveal_now_millis(),
            None,
            prior_outcomes,
        )
        .await;
        if outcome.node_count > 0 {
            // The salvage appended the section after everything
            // generated since it first failed; put it back where
            // the plan placed it.
            crate::run_salvage_feedback::restore_planned_order(
                sink,
                plan,
                outcomes,
                subtask_index,
                &outcome,
            );
            on_progress(scope_progress_for_subtask(
                groups,
                group_identities,
                subtask_index,
                Progress::SubtaskDone {
                    id: subtask.id.clone(),
                    node_count: outcome.node_count,
                },
            ));
            outcomes[outcome_index] = outcome;
        } else {
            let error = crate::run_salvage_feedback::finalize_failed_salvage(&mut outcome);
            on_progress(scope_progress_for_subtask(
                groups,
                group_identities,
                subtask_index,
                Progress::SubtaskFailed {
                    id: subtask.id.clone(),
                    error,
                },
            ));
            outcomes[outcome_index] = outcome;
        }
    }
    zero_node_failure = outcomes.iter().any(|o| o.node_count == 0);
    incomplete_subtask_failure = outcomes
        .iter()
        .any(crate::subtask_completeness::is_incomplete_outcome);
    (zero_node_failure, incomplete_subtask_failure)
}
