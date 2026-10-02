//! Ordinary-mode follow-ups must resolve their board before the broad
//! whole-design classifier sees words like “page” or “layout”.

use op_editor_host_core::chat::ChatSession;
use op_editor_host_core::design::DesignSession;
use op_host_native::WidgetHostNative;
use op_host_services::chat_intent::workspace_edit::{self, WorkspaceEditScope};

pub(super) fn launch_workspace_edit(
    host: &mut WidgetHostNative,
    user_text: &str,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    if super::active_page_is_blank_starter_frame(host.editor_state()) {
        return false;
    }
    let missing_target =
        match workspace_edit::resolve_workspace_edit_scope(host.editor_state(), user_text) {
            WorkspaceEditScope::NotApplicable => return false,
            WorkspaceEditScope::Target(_) => {
                if super::launch_direct_modify_turn(
                    host,
                    user_text,
                    op_editor_core::LaunchRoute::Auto,
                    current_chat,
                    current_design,
                ) {
                    host.editor_state_mut()
                        .editor_ui
                        .workspace
                        .begin_page_edit_turn();
                    return true;
                }
                false
            }
            WorkspaceEditScope::NeedsTarget => true,
        };
    // A missing or ambiguous target cannot degrade into whole-work generation.
    super::super::finalize_design_session_if_needed(host, current_chat, "teardown-backstop");
    *current_chat = None;
    *current_design = None;
    let state = host.editor_state_mut();
    let note = if missing_target {
        workspace_edit::scope_unavailable_message(state.editor_ui.locale)
    } else {
        op_i18n::translate(state.editor_ui.locale, "workspace.draft.refineUnavailable")
    };
    if let Some(message) = state.chat.messages.last_mut() {
        message.content = note.to_string();
        message.streaming = false;
    }
    state.chat.pending_attachments.clear();
    state.editor_ui.workspace.clear_staged_page_edit();
    host.mark_editor_state_dirty();
    true
}

#[cfg(test)]
#[path = "chat_session_launch_workspace_tests.rs"]
mod tests;
