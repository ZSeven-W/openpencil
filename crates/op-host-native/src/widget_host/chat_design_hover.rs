use super::WidgetHostNative;
use op_editor_ui::widgets::AIChatPlaceholder;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    pub(in crate::widget_host) fn update_chat_design_hover(
        &mut self,
        x: f32,
        y: f32,
        over_topmost: bool,
    ) -> bool {
        let new_hover = if !over_topmost {
            self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h)
                .and_then(|chat_rect| {
                    let panel = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms);
                    panel.design_block_hover_at(chat_rect, Point2D::new(x, y))
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
