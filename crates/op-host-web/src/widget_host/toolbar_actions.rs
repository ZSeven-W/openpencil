//! Toolbar action dispatch for web press/click paths — panel toggles
//! delegate to the shared `EditorUiState` transitions.

use super::WidgetHost;

impl WidgetHost {
    pub(in crate::widget_host) fn dispatch_toolbar_action(
        &mut self,
        action: op_editor_ui::widgets::ToolbarAction,
    ) -> bool {
        use op_editor_ui::widgets::ToolbarAction;
        match action {
            ToolbarAction::Undo => {
                let acted = self.editor_state.undo();
                if acted {
                    self.mark_dirty();
                    self.refresh_missing_fonts_after_history_change();
                }
                acted
            }
            ToolbarAction::Redo => {
                let acted = self.editor_state.redo();
                if acted {
                    self.mark_dirty();
                    self.refresh_missing_fonts_after_history_change();
                }
                acted
            }
            ToolbarAction::ToggleVariablesPanel => {
                self.editor_state.editor_ui.toggle_variables_panel();
                self.mark_dirty();
                true
            }
            ToolbarAction::ToggleDesignPanel => {
                self.editor_state.editor_ui.toggle_design_md_panel();
                self.mark_dirty();
                true
            }
            ToolbarAction::ToggleComments => {
                op_editor_core::comment_ui::toggle_comment_mode(&mut self.editor_state);
                self.mark_dirty();
                true
            }
        }
    }
}
