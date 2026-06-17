use super::WidgetHost;
use op_editor_ui::widgets::AIChatPlaceholder;
use op_editor_ui::Point2D;

impl WidgetHost {
    pub(in crate::widget_host) fn update_chat_model_picker_hover(
        &mut self,
        x: f32,
        y: f32,
        over_topmost: bool,
    ) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker.open || over_topmost {
            if self
                .editor_state
                .editor_ui
                .chat_model_picker
                .hover
                .take()
                .is_some()
            {
                self.mark_dirty();
                return true;
            }
            return false;
        }

        use op_editor_ui::widgets::ai_chat_model_picker::{model_picker_hit, SelectHit};

        let Some(picker) = self
            .ai_chat_rect(self.last_viewport_w, self.last_viewport_h)
            .and_then(|chat_rect| {
                AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                    .model_picker_bounds(chat_rect)
            })
        else {
            return false;
        };

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

    pub(in crate::widget_host) fn update_chat_design_hover(
        &mut self,
        x: f32,
        y: f32,
        over_topmost: bool,
    ) -> bool {
        let new_hover = if !over_topmost {
            self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h)
                .and_then(|chat_rect| {
                    AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                        .design_block_hover_at(chat_rect, Point2D::new(x, y))
                })
        } else {
            None
        };
        if new_hover == self.editor_state.editor_ui.chat_design_block_hover {
            return false;
        }
        self.editor_state.editor_ui.chat_design_block_hover = new_hover;
        self.mark_dirty();
        true
    }
}
