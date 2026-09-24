//! The design turn's quality report on the desktop host: fold the run's
//! quality progress into `WorkspaceState::quality` while it streams, then
//! audit the final document when the run succeeds and say what was found in
//! the transcript.
//!
//! Every number comes from a real event or a real detector pass (see
//! `op_editor_core::quality_report`). A run that never reported a quality
//! check gets no report and no transcript line.

use op_editor_core::{ChatMessage, ChatRole, EditorState, Locale};
use op_orchestrator::{Progress, RunSummary};

/// Fold every quality-bearing event of one progress batch into the
/// report. Returns whether the report changed.
pub(super) fn fold_quality_progress(state: &mut EditorState, progress: &[Progress]) -> bool {
    let mut changed = false;
    for event in progress {
        changed |= op_host_services::run_quality::fold_quality_event(
            &mut state.editor_ui.workspace.quality,
            event,
        );
    }
    changed
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
    let report = state.editor_ui.workspace.quality.take()?;
    let report = op_host_services::run_quality::audit_run_report(report, state, summary);
    let line = (!report.is_empty()).then(|| report.transcript_line(locale));
    state.editor_ui.workspace.quality = Some(report);
    line
}

#[cfg(test)]
pub(super) use op_host_services::run_quality::run_boards;

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
