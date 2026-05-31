use super::WidgetHost;

impl WidgetHost {
    /// Run a path boolean op on the active selection. The Skia path math
    /// lives in the host layer; the document mutation is committed through
    /// `EditorState` so history and selection stay canonical.
    pub fn apply_boolean_op(&mut self, op: op_editor_core::BooleanOp) -> bool {
        self.refresh_layout_scene();
        let selected: Vec<String> = self
            .editor_state
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let Some(result) =
            crate::boolean_ops::compute_boolean_op(&self.layout_scene, &selected, op)
        else {
            return false;
        };
        let source_ids: Vec<op_editor_core::NodeId> = result
            .source_ids
            .iter()
            .map(op_editor_core::NodeId::new)
            .collect();
        let pre = self.editor_state.snapshot_for_history();
        let new_id = self.editor_state.replace_paths_with_polyline(
            &source_ids,
            &result.points,
            &mut self.next_node_id,
        );
        match new_id {
            Some(id) => {
                self.editor_state.history_push_past(pre);
                self.editor_state.set_single_selection(id);
                self.mark_dirty();
                true
            }
            None => false,
        }
    }
}
