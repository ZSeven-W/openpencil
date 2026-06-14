//! Host-side bridge for the hidden accessibility DOM layer
//! (`crate::a11y`).
//!
//! `WidgetHost::editor_state` and the dirty flag are scoped
//! `pub(in crate::widget_host)`, so the a11y module reaches editor
//! state exclusively through these accessors. The mutators mirror the
//! corresponding painted-control press arms verbatim so activating a
//! hidden a11y control behaves exactly like clicking the canvas
//! chrome.

use super::WidgetHost;

impl WidgetHost {
    /// Read-only editor state for the a11y mirror's diff/sync pass.
    /// Unlike [`WidgetHost::editor_state`] (gated behind the
    /// `codegen` / `live-sync` features for its callers), this is
    /// unconditional within the skia host build — the a11y layer
    /// ships with every real bundle.
    pub(crate) fn a11y_editor_state(&self) -> &op_editor_core::EditorState {
        &self.editor_state
    }

    /// Activate a tool from the hidden a11y toolbar — mirrors the
    /// painted toolbar's `ToolbarHit::Tool` arm in
    /// `widget_host/press.rs` (tool write + shape-picker close).
    pub(crate) fn a11y_set_tool(&mut self, tool: op_editor_core::Tool) {
        self.editor_state.tool = tool;
        self.editor_state.editor_ui.shape_picker.open = false;
        self.editor_state.editor_ui.shape_picker.hover = None;
        self.editor_state.editor_ui.shape_picker.pressed = None;
        self.mark_dirty();
    }

    /// Focus the chat input from the hidden a11y button — mirrors
    /// `widget_host/click.rs` `AIChatHit::FocusInput` (focus + clear
    /// stale selections), plus the caret-blink anchor reset so the
    /// painted caret restarts its phase like a real click. Callers
    /// should `set_now_ms` first so the anchor is current.
    pub(crate) fn a11y_focus_chat_input(&mut self) {
        self.editor_state.chat.focus_input_at_end(self.now_ms);
        self.editor_state.chat.transcript_selection = None;
        self.mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetHost;

    #[test]
    fn a11y_set_tool_switches_tool_and_closes_shape_picker() {
        let mut host = WidgetHost::new();
        host.editor_state.editor_ui.shape_picker.open = true;
        host.a11y_set_tool(op_editor_core::Tool::Frame);
        assert_eq!(host.editor_state.tool, op_editor_core::Tool::Frame);
        assert!(!host.editor_state.editor_ui.shape_picker.open);
        assert!(host.editor_state_dirty);
    }

    #[test]
    fn a11y_focus_chat_input_focuses_and_clears_selections() {
        let mut host = WidgetHost::new();
        host.set_now_ms(1234);
        host.editor_state.chat.set_input_text("hello");
        host.editor_state.chat.select_all_input(0);
        host.a11y_focus_chat_input();
        let chat = &host.editor_state.chat;
        assert!(chat.focused);
        assert!(chat.input.highlight_range().is_none());
        assert!(chat.transcript_selection.is_none());
        assert_eq!(chat.input.next_blink_flip_ms(1234), 1734);
    }
}
