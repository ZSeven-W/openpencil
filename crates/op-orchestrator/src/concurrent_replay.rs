//! Per-subtask hand-off for the concurrent screen-group executor.
//!
//! Workers never touch the real document. Each subtask writes into a private
//! `BufferDocSink`; successful buffers cross `WorkerSignal`, and the executor's
//! one writer commits each buffer as a single `EditorCommand::Batch`.

use super::{run_subtask_retry_ladder, BufferDocSink};
use crate::agent_identity::AgentIdentity;
use crate::model_profile::ModelTier;
use crate::plan::OrchestratorPlan;
use crate::screen_groups::ScreenGroup;
use crate::subagent::{apply_command_with_reveal, reveal_now_millis};
use crate::types::{
    AbortFlag, DesignRequest, DocSink, GeometryEchoBudget, LlmClient, Progress, SubtaskOutcome,
};
use op_editor_core::{EditorCommand, EditorState};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Semaphore};

pub(super) struct WorkerResult {
    pub(super) aborted: bool,
}

/// One atomic hand-off to the executor's single real-document writer.
pub(super) struct SubtaskReplay {
    group_idx: usize,
    plan_idx: usize,
    outcome: SubtaskOutcome,
    commands: Option<Vec<EditorCommand>>,
    ack: oneshot::Sender<bool>,
}

pub(super) enum WorkerSignal {
    Progress { group_idx: usize, event: Progress },
    SubtaskSettled(Box<SubtaskReplay>),
}

/// A zero-node result drops its entire private buffer unopened.
fn into_replayable_commands(
    buffer: BufferDocSink,
    outcome: &SubtaskOutcome,
) -> Option<Vec<EditorCommand>> {
    (outcome.node_count > 0).then_some(buffer.commands)
}

/// Run one group's subtasks sequentially while sibling groups overlap.
///
/// `snapshot` is deliberately group-local. After a successful real commit the
/// worker applies the same batch to this mirror, so the next same-group subtask
/// sees its predecessor's content without absorbing race-dependent sibling
/// group state.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_screen_group_worker(
    group_idx: usize,
    group: &ScreenGroup,
    plan: &OrchestratorPlan,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    tier: ModelTier,
    mut snapshot: EditorState,
    semaphore: Arc<Semaphore>,
    geometry_echo_budget: &GeometryEchoBudget,
    event_tx: mpsc::UnboundedSender<WorkerSignal>,
) -> WorkerResult {
    let mut aborted = false;
    for &idx in &group.indices {
        if abort.is_set() {
            aborted = true;
            break;
        }
        let subtask = &plan.subtasks[idx];
        let permit = semaphore
            .acquire()
            .await
            .expect("semaphore should not be closed");

        let ptx = event_tx.clone();
        let mut emit = move |progress: Progress| {
            let _ = ptx.send(WorkerSignal::Progress {
                group_idx,
                event: progress,
            });
        };
        let mut buffer = BufferDocSink::new(snapshot.clone());
        let mut outcome = run_subtask_retry_ladder(
            subtask,
            plan,
            request,
            llm,
            &mut buffer,
            abort,
            tier,
            None,
            geometry_echo_budget,
            &mut emit,
        )
        .await;
        // Real-sink apply may synchronously wait for the UI ack. It must not
        // occupy one of the model-call permits while doing so.
        drop(permit);

        let commands = into_replayable_commands(buffer, &outcome);
        let local_commands = commands.clone();
        // The staging snapshot allocates local ids; the real replay may
        // remap them because sibling workers commit in completion order.
        outcome.inserted_root_ids.clear();
        let (ack_tx, ack_rx) = oneshot::channel();
        if event_tx
            .send(WorkerSignal::SubtaskSettled(Box::new(SubtaskReplay {
                group_idx,
                plan_idx: idx,
                outcome,
                commands,
                ack: ack_tx,
            })))
            .is_err()
        {
            aborted = true;
            break;
        }

        let committed = match ack_rx.await {
            Ok(committed) => committed,
            Err(_) => {
                aborted = true;
                break;
            }
        };
        if committed {
            if let Some(commands) = local_commands {
                // Keep only this group's committed history in the worker view;
                // sibling completion order must not change later prompts.
                if !snapshot.apply(EditorCommand::Batch { commands }) {
                    tracing::warn!(
                        subtask = %subtask.id,
                        "committed subtask batch could not update group-local snapshot"
                    );
                }
            }
        }
    }
    WorkerResult { aborted }
}

/// Apply one worker event. This is called only from the executor's select loop,
/// keeping the real `DocSink` single-writer even while model calls overlap.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_worker_event(
    event: WorkerSignal,
    groups: &[ScreenGroup],
    identities: &[AgentIdentity],
    plan: &OrchestratorPlan,
    sink: &mut dyn DocSink,
    agent_indicator_epoch: Option<u64>,
    per_subtask: &mut [Option<(SubtaskOutcome, bool)>],
    on_progress: &mut dyn FnMut(Progress),
) {
    let emit = |group_idx: usize, event: Progress, on_progress: &mut dyn FnMut(Progress)| {
        let group = groups
            .get(group_idx)
            .expect("worker progress group must exist");
        let identity = identities
            .get(group_idx)
            .expect("worker progress identity must exist");
        on_progress(Progress::worker_scoped(
            group_idx,
            group.screen.clone(),
            identity.clone(),
            event,
        ));
    };
    match event {
        WorkerSignal::Progress { group_idx, event } => emit(group_idx, event, on_progress),
        WorkerSignal::SubtaskSettled(replay) => {
            let SubtaskReplay {
                group_idx,
                plan_idx,
                mut outcome,
                commands,
                ack,
            } = *replay;
            let was_zero = outcome.node_count == 0;
            debug_assert_eq!(commands.is_some(), !was_zero);
            let committed = commands.is_some_and(|commands| {
                apply_command_with_reveal(
                    sink,
                    EditorCommand::Batch { commands },
                    agent_indicator_epoch,
                    reveal_now_millis(),
                )
            });
            if !was_zero && !committed {
                outcome.node_count = 0;
                outcome.error = Some("atomic replay rejected".into());
                outcome.inserted_root_ids.clear();
                outcome.subtask = Some(plan.subtasks[plan_idx].clone());
            }

            let is_zero = outcome.node_count == 0;
            let is_incomplete = crate::subtask_completeness::is_incomplete_outcome(&outcome);
            let subtask = &plan.subtasks[plan_idx];
            let terminal = if is_zero {
                Progress::SubtaskFailed {
                    id: subtask.id.clone(),
                    error: outcome.error.clone().unwrap_or_default(),
                }
            } else {
                Progress::SubtaskDone {
                    id: subtask.id.clone(),
                    node_count: outcome.node_count,
                }
            };
            per_subtask[plan_idx] = Some((outcome, is_zero));
            // Done is observable only after the atomic real-sink commit/ack.
            // An incomplete result already emitted its dedicated terminal
            // progress event inside the worker; do not overwrite it with Done.
            if !is_incomplete {
                emit(group_idx, terminal, on_progress);
            }
            let _ = ack.send(committed);
        }
    }
}

#[cfg(test)]
#[path = "concurrent_replay_tests.rs"]
mod tests;
