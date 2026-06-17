//! Snapshot-before-mutate helper for web widget dispatch paths.
//!
//! Keep web parity with the native host: only push an undo snapshot
//! when the guarded mutator reports a real document change.

use super::WidgetHost;

impl WidgetHost {
    pub(in crate::widget_host) fn with_doc_history(
        &mut self,
        f: impl FnOnce(&mut op_editor_core::EditorState) -> bool,
    ) -> bool {
        let snap = self.editor_state.snapshot_for_history();
        let changed = f(&mut self.editor_state);
        if changed {
            self.editor_state.history_push_past(snap);
        }
        changed
    }
}
