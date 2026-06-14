use super::WidgetHost;
use op_editor_core::agent_settings::SettingsFocus;

impl WidgetHost {
    pub(in crate::widget_host) fn set_settings_input_text(&mut self, text: impl Into<String>) {
        let input = &mut self.editor_state.editor_ui.settings_input;
        input.set_text(text);
        input.touch(self.now_ms);
    }

    pub(in crate::widget_host) fn apply_settings_text(&mut self, c: char) -> bool {
        let Some(focus) = self.editor_state.editor_ui.agent_settings.focus else {
            return false;
        };
        let ui = &mut self.editor_state.editor_ui;
        if !settings_accepts(focus, &ui.settings_input, c) {
            return false;
        }
        let mut buf = [0; 4];
        ui.settings_input
            .insert_str(c.encode_utf8(&mut buf), self.now_ms);
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn apply_settings_backspace(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_none() {
            return false;
        };
        let ui = &mut self.editor_state.editor_ui;
        ui.settings_input.backspace(self.now_ms);
        self.mark_dirty();
        true
    }

    pub fn apply_settings_caret(&mut self, forward: bool) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_none() {
            return false;
        };
        let ui = &mut self.editor_state.editor_ui;
        if forward {
            ui.settings_input.move_right(false, self.now_ms);
        } else {
            ui.settings_input.move_left(false, self.now_ms);
        }
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn clear_settings_caret(&mut self) {
        self.editor_state.editor_ui.settings_input.set_text("");
    }
}

fn settings_accepts(
    focus: SettingsFocus,
    input: &jian_core::text_input::TextInputState,
    c: char,
) -> bool {
    match focus {
        SettingsFocus::McpPort => {
            c.is_ascii_digit() && (input.is_select_all() || input.text().len() < 5)
        }
        SettingsFocus::ImageSearch(_)
        | SettingsFocus::BuiltinAgent { .. }
        | SettingsFocus::BuiltinAgentDraft(_)
        | SettingsFocus::ImageGenProfile { .. }
        | SettingsFocus::AcpAgent { .. }
        | SettingsFocus::AcpAgentDraft(_) => {
            !c.is_control() && (input.is_select_all() || input.text().len() < 512)
        }
    }
}
