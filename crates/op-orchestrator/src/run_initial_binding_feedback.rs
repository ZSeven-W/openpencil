//! Bounded initial-state repair after every section has committed, before cleanup
//! remaps root ids. Rejected candidates never touch the live document or state.

use crate::concurrent::BufferDocSink;
use crate::initial_binding_check::{missing_initial_bindings, InitialBindingFinding};
use crate::plan::{OrchestratorPlan, RetryFeedback, Subtask};
use crate::subagent::{
    reveal_now_millis, run_subtask_with_reveal_at_and_outcomes, IntentCheckMode,
};
use crate::types::{AbortFlag, DesignRequest, DocSink, LlmClient, Progress, SubtaskOutcome};
use op_editor_core::{EditorCommand, EditorState, NodeId};

#[path = "run_initial_binding_feedback_candidate.rs"]
mod candidate;
use candidate::{CandidateContract, RepairFailure};

const UNRESOLVED: &str =
    "initial-binding-unresolved: generated content retained; initial UI state still needs repair";

/// Returns true when any owned, generated section still has an unresolved
/// initial binding. Existing nonzero content and unrelated errors are retained.
#[allow(clippy::too_many_arguments)]
pub(super) async fn repair_initial_bindings(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    outcomes: &mut [SubtaskOutcome],
    on_progress: &mut dyn FnMut(Progress),
) -> bool {
    // Preserve first-view contracts before any owner supplies shared state.
    // Later owners may stop looking "missing" while receiving the wrong value.
    let initial_findings = outcomes
        .iter()
        .map(|outcome| {
            if outcome.node_count == 0 {
                Vec::new()
            } else {
                missing_initial_bindings(sink.state(), &outcome.inserted_root_ids)
            }
        })
        .collect::<Vec<_>>();
    let original_errors = outcomes
        .iter()
        .map(|outcome| outcome.error.clone())
        .collect::<Vec<_>>();
    let mut pending = Vec::new();
    for subtask in &plan.subtasks {
        let Some(index) = outcomes.iter().position(|o| o.id == subtask.id) else {
            continue;
        };
        let original = outcomes[index].clone();
        if original.node_count == 0 || original.inserted_root_ids.is_empty() {
            continue;
        }
        // Resolve every owner's reads against the latest complete document. An
        // earlier repair may already have supplied a shared missing default.
        let findings = missing_initial_bindings(sink.state(), &original.inserted_root_ids);
        if findings.is_empty() {
            continue;
        }
        let result = if abort.is_set() || sink.is_buffered() {
            Err(RepairFailure::Unavailable)
        } else {
            on_progress(Progress::SubtaskRetry {
                id: subtask.id.clone(),
                attempt: 1,
                reason: "repair initial bindings after full document assembly".into(),
            });
            repair_owner(
                sink,
                plan,
                request,
                llm,
                abort,
                subtask,
                &original,
                outcomes,
                &findings,
                on_progress,
            )
            .await
        };
        match result {
            Ok(mut repaired) => {
                // This pass must not erase an earlier, unrelated failure.
                repaired.error = original.error;
                let event = match &repaired.error {
                    Some(error) => Progress::SubtaskFailed {
                        id: subtask.id.clone(),
                        error: error.clone(),
                    },
                    None => Progress::SubtaskDone {
                        id: subtask.id.clone(),
                        node_count: repaired.node_count,
                    },
                };
                outcomes[index] = repaired;
                on_progress(event);
            }
            Err(reason) => {
                let keys = findings
                    .iter()
                    .map(|finding| finding.state_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ");
                let detail = format!("{UNRESOLVED} ({keys}); {reason}");
                outcomes[index].error = Some(match &original.error {
                    Some(previous) => format!("{previous}\n{detail}"),
                    None => detail,
                });
                pending.push(index);
                // Nonzero retained content must not advertise the insert-only
                // manual Retry route, which would duplicate the original.
                outcomes[index].subtask = None;
                on_progress(Progress::SubtaskFailed {
                    id: subtask.id.clone(),
                    error: outcomes[index].error.clone().unwrap_or_default(),
                });
            }
        }
    }
    let mut unresolved = false;
    for (index, findings) in initial_findings.iter().enumerate() {
        if findings.is_empty() {
            continue;
        }
        let outcome = &mut outcomes[index];
        let values =
            candidate::validate_initial_values(sink.state(), &outcome.inserted_root_ids, findings);
        if missing_initial_bindings(sink.state(), &outcome.inserted_root_ids).is_empty()
            && values.is_ok()
        {
            if pending.contains(&index) {
                // Another owner may repair the same shared value, but only clear
                // this error if the original section's first-view contract holds.
                outcome.error = original_errors[index].clone();
                on_progress(match &outcome.error {
                    Some(error) => Progress::SubtaskFailed {
                        id: outcome.id.clone(),
                        error: error.clone(),
                    },
                    None => Progress::SubtaskDone {
                        id: outcome.id.clone(),
                        node_count: outcome.node_count,
                    },
                });
            }
        } else {
            unresolved = true;
            if !outcome
                .error
                .as_ref()
                .is_some_and(|error| error.contains(UNRESOLVED))
            {
                let detail =
                    format!("{UNRESOLVED}; shared default does not satisfy the original section");
                outcome.error = Some(match &outcome.error {
                    Some(previous) => format!("{previous}\n{detail}"),
                    None => detail,
                });
                outcome.subtask = None;
                on_progress(Progress::SubtaskFailed {
                    id: outcome.id.clone(),
                    error: outcome.error.clone().unwrap_or_default(),
                });
            }
        }
    }
    unresolved
}

#[allow(clippy::too_many_arguments)]
async fn repair_owner(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    subtask: &Subtask,
    original: &SubtaskOutcome,
    outcomes: &[SubtaskOutcome],
    findings: &[InitialBindingFinding],
    on_progress: &mut dyn FnMut(Progress),
) -> Result<SubtaskOutcome, RepairFailure> {
    let before = sink.state().clone();
    let contract = CandidateContract::capture(&before, original, findings)?;
    let mut corrected = subtask.clone();
    corrected.retry_feedback = Some(RetryFeedback::InitialBinding(
        contract.feedback(&before, original, findings)?,
    ));
    let mut buffer = BufferDocSink::new(before.clone());
    let prior = outcomes
        .iter()
        .take_while(|outcome| outcome.id != subtask.id)
        .cloned()
        .collect::<Vec<_>>();
    // Exactly one same-model, full-skills attempt. Keep the original in staging
    // until insertion finishes so fresh ids cannot reuse its deleted ids.
    let repaired = run_subtask_with_reveal_at_and_outcomes(
        &corrected,
        plan,
        request,
        llm,
        &mut buffer,
        abort,
        false,
        false,
        None,
        reveal_now_millis(),
        Some(on_progress),
        &prior,
        IntentCheckMode::Reject,
    )
    .await;
    if abort.is_set() {
        return Err(RepairFailure::Unavailable);
    }
    if repaired.node_count == 0 || repaired.error.is_some() {
        return Err(RepairFailure::Candidate(
            repaired.error.unwrap_or_else(|| "empty correction".into()),
        ));
    }
    // The generated section is only a proposal source. Never publish its nodes:
    // changing ids/layout would risk unrelated cross-section references.
    contract.validate_proposal(&buffer, request, subtask, &repaired)?;
    let batch = contract.state_batch(&buffer)?;
    // A user edit/open/page switch during the model call invalidates the
    // candidate's ownership and state snapshot. Keep the user's live document.
    if sink.state().document_generation() != before.document_generation()
        || sink.state().document_revision() != before.document_revision()
        || sink.state().ui.active_page_index != before.ui.active_page_index
        || sink.state().doc != before.doc
    {
        return Err(RepairFailure::DocumentChanged);
    }
    // BufferDocSink::apply reports buffering, not successful application. The
    // real editor Batch must be preflighted, including state-owner rollback.
    let mut trial_state = sink.state().clone();
    if !trial_state.apply(batch.clone()) {
        return Err(RepairFailure::ReplayRejected);
    }
    let trial = BufferDocSink::new(trial_state);
    contract.validate_original(&trial, request, subtask, original)?;
    if abort.is_set() {
        return Err(RepairFailure::Unavailable);
    }
    if !sink.apply(batch) {
        return Err(RepairFailure::ReplayRejected);
    }
    Ok(original.clone())
}

#[cfg(test)]
#[path = "run_initial_binding_feedback_tests.rs"]
mod tests;
