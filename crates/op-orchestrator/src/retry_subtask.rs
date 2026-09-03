//! Manual per-subtask retry — the manual layer of the failed-subtask
//! remediation feature (phase 2 of the m1 investigation report). A user
//! clicks "Retry" on a failed row in the progress panel; this module
//! re-runs EXACTLY that one persisted [`Subtask`] against the LIVE
//! document, ONCE, at full complexity.
//!
//! Deliberately NOT a 3-attempt ladder and NOT wired into `run.rs`'s
//! automatic salvage pass: the user is in the loop here — they clicked,
//! they'll see the single result, and THEY decide whether to click again,
//! switch provider, or fall back to the chat modify flow. Stacking the
//! automatic retry ladder underneath a manual click would silently
//! multiply LLM calls the user never asked for.
//!
//! Reuses [`crate::subagent::run_subtask_with_reveal_at`] — the SAME
//! generation unit every subtask (orchestrator-planned or
//! `spawn_agents`-spawned) runs through — so a retried subtask's Class-A
//! passes (theme detection, canvas-width role resolution, self-check)
//! behave identically to its original attempt.

use crate::plan::{OrchestratorPlan, PlanFill, RootFrameSpec, Subtask};
use crate::subagent::{reveal_now_millis, run_subtask_with_reveal_at};
use crate::types::{AbortFlag, DesignRequest, DocSink, LlmClient, Progress, SubtaskOutcome};
use op_editor_core::PenNodeExt;

/// Build a minimal [`OrchestratorPlan`] context from the LIVE document's
/// current root frame — the SAME technique
/// `crate::spawn_concurrent::plan_from_state` uses for the model's own
/// `spawn_agents` tool, duplicated here rather than shared: that helper
/// flattens a `Subtask` down through a `SpawnAgentSpec` (dropping
/// `region`/`elements`/`screen`), which is exactly the fidelity a faithful
/// retry must NOT lose. Re-deriving `root_frame` from the CURRENT document
/// (rather than trusting whatever the ORIGINAL plan's root frame said)
/// correctly reflects any reshaping `finalize_design`'s cleanup passes did
/// after the original run.
fn plan_for_retry(sink: &dyn DocSink, subtask: &Subtask) -> OrchestratorPlan {
    let (width, height, fill) = sink
        .state()
        .active_children()
        .first()
        .map(|n| {
            (
                n.width_px().unwrap_or(1200.0),
                n.height_px().unwrap_or(800.0),
                op_editor_core::fills::first_solid_fill_hex(n).map(|hex| {
                    vec![PlanFill {
                        kind: "solid".into(),
                        color: hex.to_string(),
                    }]
                }),
            )
        })
        .unwrap_or((1200.0, 800.0, None));

    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "retry-root".into(),
            name: "Page".into(),
            width,
            height,
            layout: None,
            gap: None,
            padding: None,
            fill,
        },
        // Exactly the ONE persisted subtask being retried, byte-for-byte —
        // no re-flattening through a spec type. Its region/elements/screen/
        // id_prefix ride through unchanged.
        subtasks: vec![subtask.clone()],
        style_guide_name: None,
    }
}

/// Re-run exactly one persisted, previously-failed [`Subtask`] against the
/// live document, ONCE, at full complexity. Returns a [`SubtaskOutcome`] —
/// the SAME shape every other subtask attempt returns — so callers (the
/// host's progress-panel folding logic) need no new result type.
///
/// When `subtask.parent_frame_id` no longer resolves in the live document —
/// most likely because `finalize_design`'s cleanup passes replaced the root
/// subtree after the original run (`ReplaceSubtree` allocates a FRESH root
/// id; an ordinary insert can't target the old one) — this fails FAST with
/// a `node_count: 0` outcome naming the stale id, instead of guessing an
/// insertion point. The approved v1 scope is "tell the user the truth"; see
/// the TODO below for the deferred structural fix.
///
/// A stale `parent_frame_id` is re-resolved through the subtask's own
/// `screen` marker before giving up (see [`reparent_by_screen`]): a screen
/// rebuilt between the original run and the retry keeps its `screen` path
/// while every id under it changes, and failing on the old id made the user
/// re-describe a location that had simply been renumbered.
pub async fn retry_subtask(
    subtask: &Subtask,
    request: &DesignRequest,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    abort: &AbortFlag,
    indicator_epoch: Option<u64>,
    on_progress: Option<&mut dyn FnMut(Progress)>,
) -> SubtaskOutcome {
    // Owned so a re-resolved parent can replace the stale one without
    // touching the caller's persisted subtask.
    let mut subtask = subtask.clone();
    if let Some(parent_id) = subtask.parent_frame_id.clone() {
        let resolves = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &op_editor_core::NodeId::new(parent_id.clone()),
        )
        .is_some();
        if !resolves {
            match reparent_by_screen(sink, &subtask) {
                Some(recovered) => subtask.parent_frame_id = Some(recovered),
                None => {
                    return SubtaskOutcome {
                        id: subtask.id.clone(),
                        node_count: 0,
                        error: Some(format!(
                            "this section's original location (frame \"{parent_id}\") no longer \
                             exists in the document — describe where to add it instead"
                        )),
                        inserted_root_ids: Vec::new(),
                        headline: None,
                        subtask: None,
                    };
                }
            }
        }
    }
    let subtask = &subtask;
    let plan = plan_for_retry(sink, subtask);
    run_subtask_with_reveal_at(
        &plan.subtasks[0],
        &plan,
        request,
        llm,
        sink,
        abort,
        false,
        false,
        indicator_epoch,
        reveal_now_millis(),
        on_progress,
    )
    .await
}

/// The CURRENT top-level frame that carries this subtask's `screen` path, when
/// the subtask's recorded parent id has gone stale.
///
/// A screen regenerated between the original run and the retry keeps its
/// `screen` marker (that is the routing contract every top-level frame is
/// stamped with) while every id beneath it is renumbered — measured on
/// `0827-gk-1`, where the saved page rebuilt from `n117` to `n380` and two
/// sections then failed pointing at a frame that no longer existed.
///
/// Deliberately conservative: no `screen` on the subtask, or no unique
/// top-level frame carrying it, resolves to `None` and the caller still fails
/// with its own message. Guessing a parent would put a section on the wrong
/// screen, which is worse than asking.
fn reparent_by_screen(sink: &dyn DocSink, subtask: &Subtask) -> Option<String> {
    let screen = subtask.screen.as_deref()?;
    let mut matches = sink
        .state()
        .active_children()
        .iter()
        .filter_map(|node| match node {
            jian_ops_schema::node::PenNode::Frame(frame) => {
                (frame.screen.as_deref() == Some(screen)).then(|| frame.base.id.clone())
            }
            _ => None,
        });
    let first = matches.next()?;
    // Two frames claiming one screen path is a document-level defect; picking
    // either would be a coin flip, so decline and let the caller report.
    matches.next().is_none().then_some(first)
}

#[cfg(test)]
#[path = "retry_subtask_tests.rs"]
mod tests;
