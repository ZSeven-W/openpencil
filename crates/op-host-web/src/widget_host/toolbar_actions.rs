//! Shared toolbar action dispatch for web press/click paths.

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
                }
                acted
            }
            ToolbarAction::Redo => {
                let acted = self.editor_state.redo();
                if acted {
                    self.mark_dirty();
                }
                acted
            }
            ToolbarAction::ToggleVariablesPanel => {
                let ui = &mut self.editor_state.editor_ui;
                ui.variables_panel_open = !ui.variables_panel_open;
                if !ui.variables_panel_open {
                    ui.variables_panel_hover = None;
                    ui.axis_dropdown_open = None;
                }
                ui.design_md_panel_open = false;
                ui.design_md_hover = None;
                self.mark_dirty();
                true
            }
            ToolbarAction::ToggleDesignPanel => {
                let ui = &mut self.editor_state.editor_ui;
                ui.design_md_panel_open = !ui.design_md_panel_open;
                if !ui.design_md_panel_open {
                    ui.design_md_hover = None;
                }
                self.mark_dirty();
                true
            }
        }
    }
}
