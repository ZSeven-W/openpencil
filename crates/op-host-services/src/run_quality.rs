//! A design run's quality report, shared by every host that runs the
//! orchestrator: fold the run's quality-bearing progress into a report while
//! it streams, then audit the boards it produced when it succeeds.
//!
//! The desktop design session keeps the report on `WorkspaceState::quality`;
//! the web daemon folds it into a turn-local slot and ships the finished
//! report to the browser as one SSE event. Both use these helpers so the two
//! hosts can never report different numbers for the same run.
//!
//! Every number comes from a real event or a real detector pass (see
//! `op_editor_core::quality_report`).

use op_editor_core::{EditorState, NodeId, PenNodeExt, QualityReport};
use op_orchestrator::{Progress, RunSummary};

/// Fold one progress event into `slot`. Returns whether the report
/// changed. A report that was already audited belongs to a finished run,
/// so a new run's first quality event starts a fresh one instead of adding
/// to it.
pub fn fold_quality_event(slot: &mut Option<QualityReport>, event: &Progress) -> bool {
    match event {
        Progress::WorkerScoped(worker) => fold_quality_event(slot, worker.event.as_ref()),
        Progress::QualityChecked {
            checks,
            notes,
            items,
            ..
        } => {
            report_mut(slot).ingest_repairs(checks, items, notes);
            true
        }
        Progress::ValidationPreCheckDone { by_category, .. } => {
            report_mut(slot).ingest_lint_fixes(by_category);
            true
        }
        Progress::ValidationRoundDone { applied, .. } if *applied > 0 => {
            report_mut(slot).ingest_visual_review(*applied);
            true
        }
        _ => false,
    }
}

fn report_mut(slot: &mut Option<QualityReport>) -> &mut QualityReport {
    if slot.as_ref().is_some_and(|report| report.audited) {
        *slot = None;
    }
    slot.get_or_insert_with(QualityReport::default)
}

/// Audit the run's boards on the final document and attribute every item
/// to its board. The caller decides whether the run earned an audit (it
/// succeeded and reported at least one check).
pub fn audit_run_report(
    mut report: QualityReport,
    state: &EditorState,
    summary: &RunSummary,
) -> QualityReport {
    let boards = run_boards(state, summary);
    let audit = op_orchestrator::quality_audit::audit_final_quality(state, &boards);
    report.ingest_audit(&audit.audited_topics, audit.remaining);
    report.attribute_boards(state, &boards);
    report
}

/// The active page's top-level boards this run wrote into, in page order:
/// every board that is, or contains, a root the summary names. Falls back
/// to every board on the page when the summary carries no usable ids (a
/// buffered sink reports none) — the run drew on this page either way.
pub fn run_boards(state: &EditorState, summary: &RunSummary) -> Vec<String> {
    let mut ids: Vec<NodeId> = summary
        .subtasks
        .iter()
        .flat_map(|outcome| outcome.inserted_root_ids.iter())
        .chain(std::iter::once(&summary.root_frame_id))
        .filter(|id| !id.is_empty())
        .map(|id| NodeId::new(id.clone()))
        .collect();
    ids.dedup();
    let boards: Vec<_> = state
        .active_children()
        .iter()
        .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
        .collect();
    let touched: Vec<String> = boards
        .iter()
        .filter(|board| {
            ids.iter().any(|id| {
                op_editor_core::walkers::find_node(std::slice::from_ref(**board), id).is_some()
            })
        })
        .map(|board| board.id_str().to_string())
        .collect();
    if !touched.is_empty() {
        return touched;
    }
    boards
        .iter()
        .map(|board| board.id_str().to_string())
        .collect()
}
