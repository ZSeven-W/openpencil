//! The editor's four bare-arrow priority chains, carved out of
//! `keyboard_input.rs` so that file stays under the repo's 800-line cap.
//!
//! **The order inside each chain is the behaviour.** Every rung is a
//! surface claiming the key: the first one that owns a focused input
//! consumes it, and only a chain that falls all the way through reaches
//! `apply_nudge` and moves the selected canvas node. Reordering a rung is
//! a behaviour change, not a refactor.

use crate::DesktopApp;

impl DesktopApp {
    /// Bare Up / Down. `step` is the doc-px nudge for this press (10 with
    /// Shift), applied upward for `down == false`.
    ///
    /// The inline canvas text editor moves its caret by visual line first;
    /// then the focused chat draft, whose wrapped rows are the only thing
    /// Up/Down can mean while it owns the keyboard; then a focused numeric
    /// property input steps its value; otherwise the arrow nudges the
    /// selection.
    pub(crate) fn arrow_vertical(&mut self, down: bool, step: f32) -> bool {
        let sign = if down { 1.0 } else { -1.0 };
        self.host.apply_text_edit_vertical(down)
            || self
                .host
                .apply_chat_input_vertical_caret(down, self.shift_modifier)
            || self.host.apply_property_step(-sign * step)
            || self.host.apply_nudge(0.0, sign * step)
    }

    /// Bare Left / Right — focused text inputs move their caret, then
    /// the workspace's deck pager claims the key while the workspace
    /// shows one board / the overview, before the arrow falls through
    /// to selection nudging.
    pub(crate) fn arrow_horizontal(&mut self, forward: bool, step: f32) -> bool {
        let sign = if forward { 1.0 } else { -1.0 };
        self.host.apply_chat_model_picker_caret(forward)
            || self
                .host
                .apply_chat_input_caret(forward, self.shift_modifier)
            || self.host.apply_rename_caret(forward)
            || self.host.apply_text_edit_caret(forward)
            || self.host.apply_property_caret(forward)
            || self.workspace_page_board(forward)
            || self.host.apply_nudge(sign * step, 0.0)
    }

    /// ← / → step the deck while the workspace is up in a paging view.
    fn workspace_page_board(&mut self, forward: bool) -> bool {
        let ui = &self.host.editor_state().editor_ui;
        let paging = ui.workspace.visible
            && matches!(
                ui.workspace.view,
                op_editor_core::WorkspaceView::Single { .. }
                    | op_editor_core::WorkspaceView::Overview
            );
        if !paging {
            return false;
        }
        self.host
            .workspace_step_board(forward, self.viewport_width, self.viewport_height)
    }
}
