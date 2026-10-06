//! Shared display title of a work, independent of its view.

use op_editor_core::EditorState;

/// The work's display title: the file name, else the brief's first 16
/// chars, else the localized untitled fallback. Shared by the desktop
/// header and the phone reader so the two never name one work twice.
pub fn workspace_title(state: &EditorState) -> String {
    state
        .editor_ui
        .file_name_display
        .clone()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| {
            let brief = state.editor_ui.workspace.brief.trim();
            if brief.is_empty() {
                op_i18n::translate(state.editor_ui.locale, "common.untitled").to_string()
            } else {
                // The brief's first line; painters ellipsize it to their
                // own width (a fixed 16-char cut left "为 OpenPencil 做一份"
                // with no ellipsis next to a wide empty header).
                brief.lines().next().unwrap_or(brief).trim().to_string()
            }
        })
}
