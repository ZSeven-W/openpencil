use super::WidgetHost;

impl WidgetHost {
    pub fn apply_group(&mut self) -> bool {
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self
            .editor_state
            .group_selected(&mut self.next_node_id)
            .is_some()
        {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }
}
