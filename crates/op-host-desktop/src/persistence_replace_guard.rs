//! Ask before New / Open / Open Recent throws away unsaved edits.
//!
//! Only the window close and the Git reloads asked; every other path that
//! replaces the document (File ▸ New, Open, Open Recent — from the menu, the
//! file popover, ⌘N / ⌘O, or Home's recent list) dropped unsaved work without
//! a word. The guard lives on the persistence side so every one of those
//! entry points passes through it.

use std::path::PathBuf;

use op_editor_core::EditorState;
use op_host_native::widget_host::WidgetHostNative;

use crate::message_dialog::{ask_yes_no_cancel, Choice};

/// Replacing this document would lose edits that were never saved.
pub(crate) fn replace_needs_confirmation(state: &EditorState) -> bool {
    state.is_dirty()
}

/// `true` = go ahead with the replacement (nothing unsaved, the user saved,
/// or chose to discard); `false` = keep the current document.
pub(crate) fn confirm_replace(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    host.commit_pending_input_pub();
    if !replace_needs_confirmation(host.editor_state()) {
        return true;
    }
    let locale = host.editor_state().editor_ui.locale;
    let name = current_path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| op_i18n::translate(locale, "dialog.untitledDocument").to_string());
    let body = op_i18n::translate(locale, "dialog.replaceBody").replace("{{name}}", &name);
    match ask_yes_no_cancel(
        op_i18n::translate(locale, "dialog.unsavedTitle"),
        &body,
        rfd::MessageLevel::Warning,
    ) {
        // Yes — or no dialog backend (`None`): never drop the edits silently;
        // save first and replace only if the save really happened.
        Some(Choice::Yes) | None => {
            host.commit_variable_row_focus_if_any_pub();
            matches!(
                crate::persistence::handle_save(host, current_path, window),
                crate::persistence::SaveActionOutcome::Saved
            )
        }
        Some(Choice::No) => true,
        Some(Choice::Cancel) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unsaved_edits_need_a_question() {
        let mut state = EditorState::starter();
        assert!(!replace_needs_confirmation(&state), "a pristine document");
        state.revision += 1;
        assert!(replace_needs_confirmation(&state), "an unsaved edit");
        state.mark_saved_revision();
        assert!(!replace_needs_confirmation(&state), "saved again");
    }
}
