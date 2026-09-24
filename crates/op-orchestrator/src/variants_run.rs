//! Concurrent driver for side-by-side design directions ("variants").
//!
//! Each direction is a complete, ordinary [`Orchestrator::run`] — planning,
//! scaffold, sub-agents, cleanup, validation — so every model tier goes
//! through the same pipeline a single design does (weak models included:
//! there is no separate raw path for variants). What this module adds is
//! the fan-out around it:
//!
//! - **Isolation.** Every direction runs against its own private
//!   [`EditorState`] (a [`LocalVariantSink`]), never the live document:
//!   N pipelines writing scaffolds, variables and repairs into one page at
//!   once would trample each other.
//! - **Genuine concurrency.** The direction futures are driven together
//!   by one [`FuturesUnordered`] on the current task — the same no-`Send`,
//!   no-`tokio::spawn` discipline [`crate::spawn_concurrent`] uses — so
//!   their model calls overlap. Each direction's own worker budget is a
//!   share of the request's (see [`crate::variants::variant_request`]), so
//!   the provider sees roughly the load of one run, and the builtin
//!   transports' 429 backoff and the image-generation semaphore still sit
//!   underneath every call.
//! - **Failure containment.** A direction that errors (or produces nothing)
//!   reports [`Progress::VariantFailed`] and the rest keep going; the run
//!   only fails when every direction did.
//! - **Progressive landing.** A direction lands on the live page the
//!   moment it finishes — self-contained (instances expanded, variables
//!   resolved against its own palette), its boards prefixed with its
//!   label, placed to the right of whatever is already there. Once all
//!   directions settled they are re-flowed into slot order (A, B, C…).

use std::future::Future;

use futures::channel::mpsc;
use futures::future::{select, Either};
use futures::stream::{FuturesUnordered, StreamExt};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt, WorkspaceVariant};

use crate::agent_identity::assign_agent_identities;
use crate::run::Orchestrator;
use crate::types::{
    AbortFlag, DesignRequest, DocSink, LlmClient, OrchestratorError, Progress, RunSummary,
    ValidationProviders,
};
use crate::variants::{
    plan_variant_relayout, prefix_root_names, roots_bounds, scope_variant_progress,
    translate_roots, variant_request, variant_roots_for_merge, VariantPlan, VARIANT_GAP,
};

/// One direction, ready to run: its plan, its request and the private
/// document it builds into.
#[derive(Debug, Clone)]
pub struct VariantJob {
    pub plan: VariantPlan,
    pub request: DesignRequest,
    pub state: EditorState,
}

/// What a direction's run hands back: its private document plus the
/// ordinary run summary.
pub type VariantResult = Result<(EditorState, RunSummary), OrchestratorError>;

/// A direction's progress channel. Cheap to clone; events are forwarded
/// to the host (scoped to the direction) while the directions run.
#[derive(Debug, Clone)]
pub struct VariantTap {
    index: usize,
    tx: mpsc::UnboundedSender<(usize, Progress)>,
}

impl VariantTap {
    /// Slot of the direction this tap reports for.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Report one progress event.
    pub fn emit(&self, event: Progress) {
        // A closed channel means the driver already returned; nothing
        // is listening for this direction any more.
        let _ = self.tx.unbounded_send((self.index, event));
    }
}

/// The private document a direction builds into — an immediate-apply
/// sink over its own [`EditorState`].
pub struct LocalVariantSink {
    pub state: EditorState,
}

impl DocSink for LocalVariantSink {
    fn state(&self) -> &EditorState {
        &self.state
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.state.apply(cmd)
    }

    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        self.state
            .insert_subtree_returning_root_ids(nodes, parent_id)
    }

    fn begin_undo_batch(&mut self) {}

    fn end_undo_batch(&mut self) {}
}

/// Build one job per plan. Every direction starts from `base_state` with
/// the active page emptied: it is a whole new design, and whatever the
/// live page holds is not part of it.
pub fn variant_jobs(
    base: &DesignRequest,
    plans: &[VariantPlan],
    base_state: &EditorState,
) -> Vec<VariantJob> {
    let mut blank = base_state.clone();
    blank.active_children_mut().clear();
    blank.clear_selection();
    plans
        .iter()
        .map(|plan| VariantJob {
            plan: plan.clone(),
            request: variant_request(base, plan, plans.len()),
            state: blank.clone(),
        })
        .collect()
}

/// Run one direction through the ordinary orchestrator pipeline against
/// its private document.
pub async fn run_variant_job(
    job: VariantJob,
    tap: VariantTap,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    providers: &ValidationProviders<'_>,
    indicator_epoch: Option<u64>,
) -> VariantResult {
    let mut sink = LocalVariantSink { state: job.state };
    let mut on_progress = |event: Progress| tap.emit(event);
    let mut orchestrator = Orchestrator::new();
    if let Some(epoch) = indicator_epoch {
        // Adopt the host's epoch: a direction minting its own would move
        // the global indicator epoch the host fences its run edges with.
        orchestrator = orchestrator.with_indicator_epoch(epoch);
    }
    let summary = orchestrator
        .run(
            job.request,
            &mut sink,
            llm,
            &mut on_progress,
            abort,
            providers,
        )
        .await?;
    Ok((sink.state, summary))
}

/// Plan, run and land every direction concurrently — the production
/// entry point the hosts call.
#[allow(clippy::too_many_arguments)]
pub async fn run_design_variants(
    request: &DesignRequest,
    plans: &[VariantPlan],
    base_state: &EditorState,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    on_progress: &mut dyn FnMut(Progress),
    abort: &AbortFlag,
    providers: &ValidationProviders<'_>,
    indicator_epoch: Option<u64>,
) -> Result<RunSummary, OrchestratorError> {
    let jobs = variant_jobs(request, plans, base_state);
    run_variants_with(jobs, sink, on_progress, |job, tap| {
        run_variant_job(job, tap, llm, abort, providers, indicator_epoch)
    })
    .await
}

/// The driver, generic over how one direction runs (tests substitute a
/// scripted runner for the orchestrator).
pub async fn run_variants_with<F, Fut>(
    jobs: Vec<VariantJob>,
    sink: &mut dyn DocSink,
    on_progress: &mut dyn FnMut(Progress),
    run_one: F,
) -> Result<RunSummary, OrchestratorError>
where
    F: Fn(VariantJob, VariantTap) -> Fut,
    Fut: Future<Output = VariantResult>,
{
    if jobs.is_empty() {
        return Err(OrchestratorError::NoContent);
    }
    let plans: Vec<VariantPlan> = jobs.iter().map(|job| job.plan.clone()).collect();
    let identities = assign_agent_identities(plans.len());
    let forward = |on_progress: &mut dyn FnMut(Progress), (index, event): (usize, Progress)| {
        let slot = plans
            .iter()
            .position(|plan| plan.index == index)
            .unwrap_or(0);
        let label = plans.get(slot).map(VariantPlan::label).unwrap_or_default();
        on_progress(scope_variant_progress(
            index,
            &label,
            &identities[slot % identities.len()],
            event,
        ));
    };

    let (tx, mut rx) = mpsc::unbounded::<(usize, Progress)>();
    let mut running = FuturesUnordered::new();
    for job in jobs {
        let index = job.plan.index;
        let tap = VariantTap {
            index,
            tx: tx.clone(),
        };
        let run = run_one(job, tap);
        running.push(async move { (index, run.await) });
    }
    // Only the taps keep the channel open now.
    drop(tx);

    let mut landed: Vec<WorkspaceVariant> = Vec::new();
    let mut summaries: Vec<(usize, RunSummary)> = Vec::new();
    let mut errors: Vec<OrchestratorError> = Vec::new();

    while !running.is_empty() {
        let finished = match select(running.next(), rx.next()).await {
            Either::Left((finished, _)) => finished,
            Either::Right((Some(item), _)) => {
                forward(on_progress, item);
                continue;
            }
            // Every tap is gone; only completions remain.
            Either::Right((None, _)) => running.next().await,
        };
        let Some((index, result)) = finished else {
            break;
        };
        // A direction's own last events come before its landing report.
        while let Ok(item) = rx.try_recv() {
            forward(on_progress, item);
        }
        let Some(plan) = plans.iter().find(|plan| plan.index == index) else {
            continue;
        };
        let outcome = result.and_then(|(state, summary)| {
            merge_variant(sink, plan, &state)
                .map(|variant| (variant, summary))
                .ok_or(OrchestratorError::NoContent)
        });
        match outcome {
            Ok((variant, summary)) => {
                on_progress(Progress::VariantReady(variant.clone()));
                landed.push(variant);
                summaries.push((index, summary));
            }
            Err(error) => {
                on_progress(Progress::VariantFailed {
                    index,
                    name: plan.name.clone(),
                    error: error.to_string(),
                });
                errors.push(error);
            }
        }
    }
    while let Ok(item) = rx.try_recv() {
        forward(on_progress, item);
    }

    if landed.is_empty() {
        return Err(all_failed(errors));
    }
    landed.sort_by_key(|variant| variant.index);
    relayout_landed(sink, &landed);
    adopt_primary_palette(sink, &landed[0]);
    Ok(merge_summaries(&landed, summaries))
}

/// Land one finished direction on the live page. `None` when it has no
/// boards or the live document refused them.
pub fn merge_variant(
    sink: &mut dyn DocSink,
    plan: &VariantPlan,
    state: &EditorState,
) -> Option<WorkspaceVariant> {
    let mut roots = variant_roots_for_merge(state);
    if roots.is_empty() {
        return None;
    }
    let prefix = plan.name_prefix();
    prefix_root_names(&mut roots, &prefix);
    let (min_x, min_y, _, _) = roots_bounds(&roots)?;
    let (target_x, target_y) = match roots_bounds(sink.state().active_children()) {
        Some((_, live_min_y, live_max_x, _)) => (live_max_x + VARIANT_GAP, live_min_y),
        None => (0.0, 0.0),
    };
    translate_roots(&mut roots, target_x - min_x, target_y - min_y);
    let root_ids = sink.insert_subtree_returning_root_ids(roots, &NodeId::NONE)?;
    if root_ids.is_empty() {
        return None;
    }
    Some(WorkspaceVariant {
        index: plan.index,
        name: plan.name.clone(),
        style_guide: plan.style_guide.clone(),
        style_label: plan.style_label.clone(),
        name_prefix: prefix,
        root_ids,
        variables: state.doc.variables.clone(),
        themes: state.doc.themes.clone(),
    })
}

/// Re-flow the landed directions into slot order once all settled —
/// completion order is whoever finished first, reading order is A, B, C.
fn relayout_landed(sink: &mut dyn DocSink, landed: &[WorkspaceVariant]) {
    let moves = {
        let live = sink.state().active_children();
        let groups: Vec<(usize, Vec<&PenNode>)> = landed
            .iter()
            .map(|variant| {
                let roots = variant
                    .root_ids
                    .iter()
                    .filter_map(|id| live.iter().find(|node| node.id_str() == id))
                    .collect();
                (variant.index, roots)
            })
            .collect();
        let all: Vec<PenNode> = groups
            .iter()
            .flat_map(|(_, roots)| roots.iter().map(|root| (*root).clone()))
            .collect();
        let Some((origin_x, origin_y, _, _)) = roots_bounds(&all) else {
            return;
        };
        plan_variant_relayout(&groups, origin_x, origin_y, VARIANT_GAP)
    };
    for (id, x, y) in moves {
        sink.apply(EditorCommand::UpdateNode {
            node_id: NodeId::new(id),
            x: Some(x.round() as i32),
            y: Some(y.round() as i32),
            width: None,
            height: None,
            name: None,
            fill_hex: None,
            page_id: None,
        });
    }
}

/// The document's own palette follows direction A (the first to land in
/// slot order): its boards no longer need it, but follow-up edits and the
/// style panels read the document's variables.
fn adopt_primary_palette(sink: &mut dyn DocSink, primary: &WorkspaceVariant) {
    if let Some(variables) = &primary.variables {
        if sink.state().doc.variables.as_ref() != Some(variables) {
            sink.apply(EditorCommand::SetVariables {
                variables: variables.clone(),
                replace: true,
            });
        }
    }
    if let Some(themes) = &primary.themes {
        if sink.state().doc.themes.as_ref() != Some(themes) {
            sink.apply(EditorCommand::SetThemes {
                themes: themes.clone(),
                replace: true,
            });
        }
    }
}

fn all_failed(errors: Vec<OrchestratorError>) -> OrchestratorError {
    if !errors.is_empty()
        && errors
            .iter()
            .all(|error| matches!(error, OrchestratorError::Aborted))
    {
        return OrchestratorError::Aborted;
    }
    let first = errors
        .iter()
        .find(|error| !matches!(error, OrchestratorError::Aborted))
        .map(ToString::to_string)
        .unwrap_or_else(|| "every direction produced nothing".into());
    OrchestratorError::AllFailed(first)
}

fn merge_summaries(
    landed: &[WorkspaceVariant],
    mut summaries: Vec<(usize, RunSummary)>,
) -> RunSummary {
    summaries.sort_by_key(|(index, _)| *index);
    let root_frame_id = landed
        .first()
        .and_then(|variant| variant.root_ids.first())
        .cloned()
        .unwrap_or_default();
    let mut merged = RunSummary {
        root_frame_id,
        subtasks: Vec::new(),
        total_nodes: 0,
        unfilled_screens: Vec::new(),
        incomplete_subtask_failure: false,
    };
    for (index, summary) in summaries {
        // Outcomes are re-addressed the way their transcript rows were
        // (`A-hero`), and lose what only made sense in the direction's
        // private document: its root ids, and the section spec a per-row
        // retry would replay against the shared page without its pinned
        // guide. A direction is retried as a whole, from the workspace.
        let letter = op_editor_core::variant_letter(index);
        merged
            .subtasks
            .extend(summary.subtasks.into_iter().map(|mut outcome| {
                outcome.id = format!("{letter}-{}", outcome.id);
                outcome.inserted_root_ids.clear();
                outcome.subtask = None;
                outcome
            }));
        merged.total_nodes += summary.total_nodes;
        merged.unfilled_screens.extend(summary.unfilled_screens);
        merged.incomplete_subtask_failure |= summary.incomplete_subtask_failure;
    }
    merged
}

#[cfg(test)]
#[path = "variants_run_tests.rs"]
mod tests;
