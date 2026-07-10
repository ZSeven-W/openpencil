use super::WidgetHostNative;

impl WidgetHostNative {
    /// Store the chat design-block hover resolved for the current cursor event.
    /// `new_hover` comes from the combined `AIChatPlaceholder::cursor_probe`
    /// (the same probe that drives the header hover), so the design hover no
    /// longer re-fingerprints the transcript on its own. Returns `true` when the
    /// stored hover changed (the caller repaints).
    pub(in crate::widget_host) fn apply_chat_design_hover(
        &mut self,
        new_hover: Option<(usize, usize)>,
    ) -> bool {
        if new_hover == self.editor_state.editor_ui.chat_design_block_hover {
            return false;
        }
        self.editor_state.editor_ui.chat_design_block_hover = new_hover;
        self.mark_dirty();
        true
    }
}
