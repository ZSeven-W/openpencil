//! The pure decisions behind the generation workspace's phase pump.
//!
//! Every host that drives a workspace run observes the "everything is
//! done" edge — the desktop runner each frame, the mobile engine's chat
//! pump when its design turn finishes — and both need the same verdicts:
//! how many real boards the run produced, whether the last assistant
//! message reports a failure, and whether a queued send has yet to
//! launch. They live here, host-agnostic, so the two pumps cannot
//! disagree about what 已完成 and 失败 mean.

use crate::chat::ChatMessage;
use crate::{ChatRole, EditorState};

/// Whether the last assistant message reports a failed turn: a failed
/// orchestrator subtask, a completion with failures, or an errored
/// activity row. The workspace's Failed banner keys off this.
pub fn last_assistant_failed(state: &EditorState) -> bool {
    let Some(message) = last_assistant_message(state) else {
        return false;
    };
    if !message.failed_subtasks.is_empty() {
        return true;
    }
    if message
        .completion
        .is_some_and(|completion| completion.failed > 0)
    {
        return true;
    }
    message
        .activities
        .iter()
        .any(|activity| activity.status == crate::ChatActivityStatus::Error)
}

/// The active tab's last assistant message, if the transcript has one.
fn last_assistant_message(state: &EditorState) -> Option<&ChatMessage> {
    state
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ChatRole::Assistant)
}

/// How many REAL boards the run produced.
///
/// `active_page_boards` counts the blank starter frame like any other
/// board, so a run that drew nothing at all still reported one board and
/// settled as Done (measured 2026-09-13: a 演示文稿 brief that never
/// reached the design pipeline showed 已完成 over an empty Frame). A page
/// that is still the untouched starter has produced nothing.
pub fn produced_board_count(state: &EditorState) -> usize {
    if crate::blank_starter::active_page_is_blank_starter(state) {
        return 0;
    }
    crate::preview_slideshow::active_page_boards(state).len()
}

/// Whether a queued send has yet to reach the launcher.
///
/// Between `begin_send` and the next `launch_if_pending` drain there is
/// no chat or design session, so every "is everything idle?" test says
/// yes even though the run has not started. The workspace must keep
/// showing 生成中 across that gap instead of settling a verdict on a run
/// that has not begun.
pub fn awaiting_launch(state: &EditorState) -> bool {
    state.chat.pending_send.is_some()
}

/// Whether any assistant message is still streaming in the active tab
/// — the "generating" half of the phase that `agents_running` alone
/// misses (single-agent turns never touch the running counters).
pub fn assistant_streaming(state: &EditorState) -> bool {
    state
        .chat
        .messages
        .iter()
        .any(|message| message.role == ChatRole::Assistant && message.streaming)
}

#[cfg(test)]
#[path = "workspace_run_tests.rs"]
mod tests;
