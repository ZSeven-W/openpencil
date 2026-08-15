use super::WidgetHost;
use op_editor_ui::widgets::AIChatPlaceholder;
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    pub(in crate::widget_host) fn chat_model_picker_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        if !self.editor_state.editor_ui.chat_model_picker.open {
            return None;
        }
        let chat_rect = self.ai_chat_rect(viewport_w, viewport_h)?;
        AIChatPlaceholder::from_editor(&self.editor_state).model_picker_bounds(chat_rect)
    }

    pub(in crate::widget_host) fn update_chat_model_picker_hover(
        &mut self,
        x: f32,
        y: f32,
        picker: Rect,
    ) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker.open {
            return false;
        }

        use op_editor_ui::widgets::ai_chat_model_picker::{model_picker_hit, SelectHit};

        let new_hover = match model_picker_hit(
            &self.editor_state.editor_ui.chat_model_picker,
            picker,
            Point2D::new(x, y),
            &self.editor_state.chat.available_models,
            self.editor_state.editor_ui.chat_model_picker_input.text(),
        ) {
            SelectHit::Row(index) => Some(index),
            SelectHit::Inside | SelectHit::Outside => None,
        };
        if new_hover != self.editor_state.editor_ui.chat_model_picker.hover {
            self.editor_state.editor_ui.chat_model_picker.hover = new_hover;
            self.mark_dirty();
            return true;
        }
        false
    }

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
