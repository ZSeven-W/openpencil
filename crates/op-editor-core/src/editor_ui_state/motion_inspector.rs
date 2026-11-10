//! Which Motion-section row has its fields open. Keyed by node id, so a
//! different selection starts collapsed.

use super::EditorUiState;

impl EditorUiState {
    /// Open row `index` of `node_id`, or close it when it is already open.
    pub fn toggle_motion_row(&mut self, node_id: &str, index: usize) {
        let row = (node_id.to_owned(), index);
        self.motion_expanded = if self.motion_expanded.as_ref() == Some(&row) {
            None
        } else {
            Some(row)
        };
    }

    /// The open row for `node_id`, if any.
    pub fn motion_row_expanded(&self, node_id: &str) -> Option<usize> {
        self.motion_expanded
            .as_ref()
            .filter(|(id, _)| id == node_id)
            .map(|(_, index)| *index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_open_close_and_belong_to_one_node() {
        let mut ui = EditorUiState::default();
        ui.toggle_motion_row("a", 1);
        assert_eq!(ui.motion_row_expanded("a"), Some(1));
        assert_eq!(ui.motion_row_expanded("b"), None);
        ui.toggle_motion_row("a", 0);
        assert_eq!(ui.motion_row_expanded("a"), Some(0));
        ui.toggle_motion_row("a", 0);
        assert_eq!(ui.motion_row_expanded("a"), None);
    }
}
