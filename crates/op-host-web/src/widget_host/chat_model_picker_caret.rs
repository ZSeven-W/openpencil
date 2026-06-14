use super::WidgetHost;

impl WidgetHost {
    pub(in crate::widget_host) fn apply_chat_model_picker_text(&mut self, c: char) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker.open || c.is_control() {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        let mut s = [0u8; 4];
        ui.chat_model_picker_input
            .insert_str(c.encode_utf8(&mut s), self.now_ms);
        ui.chat_model_picker.scroll.offset = 0.0;
        ui.chat_model_picker.hover = None;
        ui.chat_model_picker.pressed = None;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn apply_chat_model_picker_backspace(&mut self) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker.open {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        ui.chat_model_picker_input.backspace(self.now_ms);
        ui.chat_model_picker_input.touch(self.now_ms);
        ui.chat_model_picker.scroll.offset = 0.0;
        ui.chat_model_picker.hover = None;
        ui.chat_model_picker.pressed = None;
        self.mark_dirty();
        true
    }

    pub fn apply_chat_model_picker_caret(&mut self, forward: bool) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker.open {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        if forward {
            ui.chat_model_picker_input.move_right(false, self.now_ms);
        } else {
            ui.chat_model_picker_input.move_left(false, self.now_ms);
        }
        self.mark_dirty();
        true
    }
}
