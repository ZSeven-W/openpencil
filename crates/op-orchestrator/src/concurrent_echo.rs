//! The `geometry_echo` in-loop self-correction tail of
//! `concurrent::run_subtask_retry_ladder`, extracted verbatim to keep the
//! ladder module under the 800-line spine budget. See
//! [`maybe_geometry_echo_with_outcomes`] for the full contract.

use super::*;

#[path = "concurrent_echo_copy.rs"]
mod copy;

#[path = "concurrent_echo_interaction.rs"]
mod interaction;

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
    let Some(original_copy) = copy::CopySnapshot::capture(sink.state(), &outcome.inserted_root_ids)
    else {
        return outcome;
    };
    let Some(original_interaction) =
        interaction::InteractionSnapshot::capture(sink.state(), &outcome.inserted_root_ids)
    else {
        return outcome;
    };
    let copy_feedback = original_copy.feedback();
    let interaction_feedback = original_interaction.feedback();
    // A giant source-copy payload would consume the repair's output budget.
    // Keep the original and let deterministic layout cleanup handle it.
    if copy_feedback.len() + interaction_feedback.len() > 16_384 {
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
        retry_feedback: Some(crate::plan::RetryFeedback::Geometry(format!(
            "{}\n\n{copy_feedback}\n\n{interaction_feedback}",
            issues.join("\n")
        ))),
        ..subtask.clone()
    };
    // Rejecting a rewrite must discard ALL its mutations, including app state.
    // Never expose a speculative replacement to the live document.
    let before = sink.state().clone();
    let mut candidate = BufferDocSink::new(before.clone());
    let mut retried = run_subtask_with_reveal_at_and_outcomes(
        &echo_subtask,
        plan,
        request,
        llm,
        &mut candidate,
        abort,
        reduced_complexity,
        minimal_skills,
        None,
        reveal_now_millis(),
        None,
        prior_outcomes,
        IntentCheckMode::Reject,
    )
    .await;

    if retried.node_count == 0 || retried.inserted_root_ids.is_empty() || abort.is_set() {
        // The echo retry itself failed (LLM error, parse failure, or a
        // fresh self-check rejection) — keep the ORIGINAL, still-real
        // content rather than lose it; the deterministic net in
        // `cleanup.rs` picks up whatever geometry issues remain.
        return outcome;
    }

    // Layout improvement cannot trade away already-delivered items or language.
    // The normal ladder inspected the original before this echo; inspect the
    // candidate again before deleting that original subtree.
    let (completeness, language) =
        crate::output_language::inspect_insert_gates(&candidate, request, subtask, &retried);
    let copy_preserved = original_copy.matches_after_removal(
        &candidate,
        &retried.inserted_root_ids,
        &outcome.inserted_root_ids,
    );
    let mut commands = candidate.commands;
    commands.extend(
        outcome
            .inserted_root_ids
            .iter()
            .map(|id| EditorCommand::DeleteNode {
                node_id: op_editor_core::NodeId::new(id),
                page_id: None,
            }),
    );
    let batch = EditorCommand::Batch { commands };
    // BufferDocSink reports buffering, so preflight the actual atomic replay.
    let mut future = before.clone();
    let replayable = future.apply(batch.clone());
    let interaction_preserved = replayable
        && original_interaction.matches(&future, &retried.inserted_root_ids)
        && interaction::outside_unchanged(
            &before,
            &future,
            &outcome.inserted_root_ids,
            &retried.inserted_root_ids,
        );
    if completeness.is_some() || language.is_some() || !copy_preserved || !interaction_preserved {
        tracing::warn!(
            subtask = %subtask.id,
            completeness = ?completeness,
            language = ?language,
            copy_preserved,
            interaction_preserved,
            "geometry echo candidate failed delivery gates; retaining original content"
        );
        return outcome;
    }

    if abort.is_set()
        || sink.state().document_generation() != before.document_generation()
        || sink.state().document_revision() != before.document_revision()
        || sink.state().ui.active_page_index != before.ui.active_page_index
        || sink.state().doc != before.doc
    {
        return outcome;
    }
    let parent = subtask
        .parent_frame_id
        .as_ref()
        .map(op_editor_core::NodeId::new)
        .unwrap_or(op_editor_core::NodeId::NONE);
    let roots_before = crate::types::child_ids_under(sink.state(), &parent);
    if !crate::subagent::apply_command_with_reveal(
        sink,
        batch,
        agent_indicator_epoch,
        reveal_now_millis(),
    ) {
        return outcome;
    }
    retried.inserted_root_ids = crate::types::child_ids_under(sink.state(), &parent)
        .into_iter()
        .filter(|id| !roots_before.contains(id))
        .collect();
    retried
}

#[cfg(test)]
#[path = "concurrent_echo_contract_tests.rs"]
mod contract_tests;
