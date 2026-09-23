//! The `geometry_echo` in-loop self-correction tail of
//! `concurrent::run_subtask_retry_ladder`, extracted verbatim to keep the
//! ladder module under the 800-line spine budget. See
//! [`maybe_geometry_echo_with_outcomes`] for the full contract.

use super::*;

/// One in-loop self-correction round for real resolved-layout violations.
///
/// A no-op (zero extra LLM calls) whenever:
/// - `outcome.node_count == 0` — nothing landed, nothing to check;
/// - `outcome.inserted_root_ids` is empty — a sink that cannot surface
///   post-insert ids has nothing live to lay out or address for a replace.
/// - the diagnostics come back empty — the common case, zero cost;
/// - the run-wide [`GeometryEchoBudget`] is exhausted.
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(super) async fn maybe_geometry_echo(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    agent_indicator_epoch: Option<u64>,
    budget: &GeometryEchoBudget,
    on_progress: &mut dyn FnMut(Progress),
    outcome: SubtaskOutcome,
) -> SubtaskOutcome {
    maybe_geometry_echo_with_outcomes(
        subtask,
        plan,
        request,
        llm,
        sink,
        abort,
        reduced_complexity,
        minimal_skills,
        agent_indicator_epoch,
        budget,
        on_progress,
        &[],
        outcome,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn maybe_geometry_echo_with_outcomes(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    agent_indicator_epoch: Option<u64>,
    budget: &GeometryEchoBudget,
    on_progress: &mut dyn FnMut(Progress),
    prior_outcomes: &[SubtaskOutcome],
    outcome: SubtaskOutcome,
) -> SubtaskOutcome {
    if outcome.node_count == 0
        || outcome.inserted_root_ids.is_empty()
        || abort.is_set()
        || sink.is_buffered()
    {
        return outcome;
    }
    let issues = crate::geometry_validation::geometry_diagnostics_for_roots(
        sink.state(),
        &outcome.inserted_root_ids,
    );
    if issues.is_empty() {
        return outcome;
    }
    if !budget.try_consume() {
        return outcome;
    }

    on_progress(Progress::GeometryEcho {
        id: subtask.id.clone(),
        issue_count: issues.len(),
    });
    tracing::info!(
        subtask = %subtask.id,
        issue_count = issues.len(),
        "geometry echo: resolved-layout violation(s) found, retrying in-loop"
    );

    let echo_subtask = Subtask {
        retry_feedback: Some(crate::plan::RetryFeedback::Geometry(issues.join("\n"))),
        ..subtask.clone()
    };
    let retried = run_subtask_with_reveal_at_and_outcomes(
        &echo_subtask,
        plan,
        request,
        llm,
        sink,
        abort,
        reduced_complexity,
        minimal_skills,
        agent_indicator_epoch,
        reveal_now_millis(),
        None,
        prior_outcomes,
    )
    .await;

    if retried.node_count == 0 {
        // The echo retry itself failed (LLM error, parse failure, or a
        // fresh self-check rejection) — keep the ORIGINAL, still-real
        // content rather than lose it; the deterministic net in
        // `cleanup.rs` picks up whatever geometry issues remain.
        return outcome;
    }

    // Adopt the corrected content: drop the original insert now that a
    // real replacement has landed. Delete-then-keep-the-new-insert rather
    // than a literal `EditorCommand::ReplaceSubtree` (which is 1-old-root-
    // to-1-new-node only) because a subtask can produce N top-level roots
    // on either side — this generalizes to N-old/M-new without assuming a
    // 1:1 shape.
    crate::subtask_completeness::rollback_inserted_roots(sink, &outcome.inserted_root_ids);
    retried
}
