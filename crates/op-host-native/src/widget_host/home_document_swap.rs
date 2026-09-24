//! The document a Studio Home brief replaced, handed to the platform shell.
//!
//! A brief sent from Home is a NEW deliverable, so the host swaps in a fresh
//! starter page (`start_fresh_document_for_home`). That swap used to drop the
//! previous work on the floor: its unsaved edits were gone, and the desktop
//! shell kept the previous file path, so the next ⌘S wrote the NEW design over
//! the OLD file. The host cannot open a dialog or touch the file system, so it
//! parks what it replaced here and the shell drains it — forgets the path and
//! keeps unsaved work somewhere the user can reopen it.

use super::WidgetHostNative;

/// What a Home brief replaced.
pub struct ReplacedHomeDocument {
    /// The replaced editor state, narrowed to what a save needs.
    pub state: Box<op_editor_core::EditorState>,
    /// The replaced document had edits that were never saved.
    pub had_unsaved_changes: bool,
    /// The replaced run's conversation title — a name for the saved copy.
    pub title: String,
}

impl WidgetHostNative {
    /// Take the document the last Home brief replaced, if any. A shell that
    /// never drains simply keeps the latest one until the next swap.
    pub fn take_replaced_home_document(&mut self) -> Option<ReplacedHomeDocument> {
        self.replaced_home_document.take()
    }

    /// Park the current document before a Home brief replaces it.
    pub(in crate::widget_host) fn park_document_replaced_by_home(&mut self) {
        let had_unsaved_changes = self.editor_state.is_dirty();
        let title = self.editor_state.chat.title.trim().to_string();
        let state = op_editor_core::request_snapshot::narrowed_snapshot(&mut self.editor_state);
        self.replaced_home_document = Some(ReplacedHomeDocument {
            state: Box::new(state),
            had_unsaved_changes,
            title,
        });
    }
}

#[cfg(test)]
#[path = "home_document_swap_tests.rs"]
mod tests;
