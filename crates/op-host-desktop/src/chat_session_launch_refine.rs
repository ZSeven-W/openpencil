//! The pinned [`op_editor_core::LaunchRoute::Refine`] turn: Studio Home's
//! one-click start loaded a template as the instant draft and now asks
//! the model to edit THOSE boards toward the example brief.
//!
//! The turn goes through the real modify route — `build_modify_plan` over
//! the selected frames, `run_modify_turn_cancellable` on a worker, the
//! scoped `__apply_design_modification` apply — for every provider kind.
//! It never reaches the keyword / LLM classifiers: the refine brief quotes
//! the example, and "做一份 5 页 PPT" reads to them as a NEW design, which
//! would draw a second deck beside the draft instead of editing it.
//! Split out of `chat_session_launch.rs` at the 800-line cap.

use op_editor_host_core::chat::ChatSession;
use op_editor_host_core::design::DesignSession;
use op_host_native::WidgetHostNative;

/// Launch the refine of the selected draft boards. Always returns true: a
/// turn that cannot start still ends honestly in its bubble (and re-arms
/// the draft banner) rather than falling through to a route that would
/// generate over the draft.
pub(super) fn launch_draft_refine_turn(
    host: &mut WidgetHostNative,
    user_text: &str,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    if super::launch_direct_modify_turn(host, user_text, current_chat, current_design) {
        // The modify plan has captured its target frames; the selection was
        // only the hand-off, so the draft stops wearing selection outlines.
        host.editor_state_mut().clear_selection();
        host.mark_editor_state_dirty();
        return true;
    }
    // No provider transport or no frame scope (the boards were deleted or
    // deselected before the launch drained). Say so where the turn is.
    super::super::finalize_design_session_if_needed(host, current_chat, "teardown-backstop");
    *current_chat = None;
    *current_design = None;
    let state = host.editor_state_mut();
    let note = op_i18n::translate(state.editor_ui.locale, "workspace.draft.refineUnavailable");
    if let Some(message) = state.chat.messages.last_mut() {
        message.content = note.to_string();
        message.streaming = false;
    }
    state.chat.pending_attachments.clear();
    state.clear_selection();
    // The draft still has not been refined: bring its banner back once the
    // idle edge settles the workspace.
    state.editor_ui.workspace.draft_awaiting_refine = true;
    host.mark_editor_state_dirty();
    true
}

#[cfg(test)]
#[path = "chat_session_launch_refine_tests.rs"]
mod tests;
