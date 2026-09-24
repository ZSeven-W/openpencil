//! The design turn's quality report on the desktop host: fold the run's
//! quality progress into `WorkspaceState::quality` while it streams, then
//! audit the final document when the run succeeds and say what was found in
//! the transcript.
//!
//! Every number comes from a real event or a real detector pass (see
//! `op_editor_core::quality_report`). A run that never reported a quality
//! check gets no report and no transcript line.

use op_editor_core::{ChatMessage, ChatRole, EditorState, Locale, PenNodeExt, QualityReport};
use op_orchestrator::{Progress, RunSummary};

/// Fold every quality-bearing event of one progress batch into the
/// report. Returns whether the report changed.
pub(super) fn fold_quality_progress(state: &mut EditorState, progress: &[Progress]) -> bool {
    let mut changed = false;
    for event in progress {
        changed |= fold_event(state, event);
    }
    changed
}

fn fold_event(state: &mut EditorState, event: &Progress) -> bool {
    match event {
        Progress::WorkerScoped(worker) => fold_event(state, worker.event.as_ref()),
        Progress::QualityChecked {
            checks,
            notes,
            items,
            ..
        } => {
            report_mut(state).ingest_repairs(checks, items, notes);
            true
        }
        Progress::ValidationPreCheckDone { by_category, .. } => {
            report_mut(state).ingest_lint_fixes(by_category);
            true
        }
        Progress::ValidationRoundDone { applied, .. } if *applied > 0 => {
            report_mut(state).ingest_visual_review(*applied);
            true
        }
        _ => false,
    }
}

/// The live run's report. A report that was already audited belongs to a
/// finished run, so a new run's first event starts a fresh one instead of
/// adding to it.
fn report_mut(state: &mut EditorState) -> &mut QualityReport {
    let slot = &mut state.editor_ui.workspace.quality;
    if slot.as_ref().is_some_and(|report| report.audited) {
        *slot = None;
    }
    slot.get_or_insert_with(QualityReport::default)
}

/// The run succeeded: audit the boards it produced, attribute every item
/// to its board, and append the summary line to the run's primary
/// assistant message. Returns whether anything changed.
pub(super) fn finish_quality_report(
    state: &mut EditorState,
    summary: &RunSummary,
    running_tab: Option<usize>,
    locale: Locale,
) -> bool {
    let Some(line) = finalize_quality_report(state, summary, locale) else {
        return false;
    };
    let chat = state.chat.run_tab_mut(running_tab);
    let Some(message) = primary_message(&mut chat.messages) else {
        return true;
    };
    if !message.content.contains(&line) {
        if !message.content.trim().is_empty() {
            message.content.push_str("\n\n");
        }
        message.content.push_str(&line);
    }
    true
}

/// Audit + board attribution. `None` when the run never reported a check
/// (nothing to vouch for) or the report was already finalized.
pub(super) fn finalize_quality_report(
    state: &mut EditorState,
    summary: &RunSummary,
    locale: Locale,
) -> Option<String> {
    if state
        .editor_ui
        .workspace
        .quality
        .as_ref()
        .is_none_or(|report| report.audited)
    {
        return None;
    }
    let boards = run_boards(state, summary);
    let audit = op_orchestrator::quality_audit::audit_final_quality(state, &boards);
    let mut report = state.editor_ui.workspace.quality.take()?;
    report.ingest_audit(&audit.audited_topics, audit.remaining);
    report.attribute_boards(state, &boards);
    let line = (!report.is_empty()).then(|| report.transcript_line(locale));
    state.editor_ui.workspace.quality = Some(report);
    line
}

/// The active page's top-level boards this run wrote into, in page order:
/// every board that is, or contains, a root the summary names. Falls back
/// to every board on the page when the summary carries no usable ids (a
/// buffered sink reports none) — the run drew on this page either way.
pub(super) fn run_boards(state: &EditorState, summary: &RunSummary) -> Vec<String> {
    let mut ids: Vec<op_editor_core::NodeId> = summary
        .subtasks
        .iter()
        .flat_map(|outcome| outcome.inserted_root_ids.iter())
        .chain(std::iter::once(&summary.root_frame_id))
        .filter(|id| !id.is_empty())
        .map(|id| op_editor_core::NodeId::new(id.clone()))
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

/// The current turn's primary (non-worker) assistant message.
fn primary_message(messages: &mut [ChatMessage]) -> Option<&mut ChatMessage> {
    let turn_start = messages
        .iter()
        .rposition(|message| message.role == ChatRole::User)
        .map_or(0, |index| index + 1);
    messages.iter_mut().skip(turn_start).find(|message| {
        message.role == ChatRole::Assistant && message.design_worker_group.is_none()
    })
}

#[cfg(test)]
#[path = "design_session_quality_report_tests.rs"]
mod tests;
