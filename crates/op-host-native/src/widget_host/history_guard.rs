//! Snapshot-before-mutate helper for host dispatch paths.
//!
//! Mirrors TS `mutateWithHistory`
//! (`stores/document-store-node-actions.ts:43-51`) and the explicit
//! `pushState` calls in `stores/document-store-pages.ts:19-121`: the
//! pre-mutation document is pushed onto the undo stack only when the
//! mutator reports an actual change, so guard-rejected ops (last-page
//! delete, missing node, …) never pollute history.

use super::WidgetHostNative;

impl WidgetHostNative {
    /// Snapshot, run `f`, push the snapshot when `f` reports a
    /// change. Returns `f`'s result.
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
