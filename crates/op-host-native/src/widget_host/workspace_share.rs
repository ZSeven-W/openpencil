//! Opening a shared document: a file that carries a share recipe is
//! presented in the Studio workspace, settled, with the
//! Make-one-like-this banner up (see
//! `op_editor_core::EditorUiState::open_workspace_for_shared_recipe`).

use super::WidgetHostNative;
use op_editor_core::Tool;

impl WidgetHostNative {
    /// Present the just-installed document in the shared view when it
    /// carries a recipe. Hosts call this right after an Open lands and
    /// the camera is fitted; a document without a recipe is left exactly
    /// as it was. Returns whether the shared view opened.
    pub fn adopt_shared_recipe_view(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let previous_tool = self.editor_state.tool;
        if !self
            .editor_state
            .editor_ui
            .open_workspace_for_shared_recipe(self.now_ms)
        {
            return false;
        }
        let workspace = &mut self.editor_state.editor_ui.workspace;
        workspace.previous_tool = Some(previous_tool);
        workspace.sync_drawer_mode(viewport_w);
        // Pan-only viewing while the workspace owns the canvas, exactly as
        // for a run of the user's own; 专业编辑 restores the tool.
        self.editor_state.tool = Tool::Hand;
        self.apply_workspace_fit(viewport_w, viewport_h);
        self.mark_dirty();
        true
    }
}

#[cfg(test)]
#[path = "workspace_share_tests.rs"]
mod tests;
