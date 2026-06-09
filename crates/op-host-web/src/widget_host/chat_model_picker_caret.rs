use super::WidgetHost;

impl WidgetHost {
    pub(in crate::widget_host) fn apply_chat_model_picker_text(&mut self, c: char) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker_open || c.is_control() {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        let replacing_all = ui.chat_model_picker_select_all;
        let caret = if replacing_all {
            ui.chat_model_picker_search.clear();
            ui.chat_model_picker_select_all = false;
            0
        } else {
            caret_position(&ui.chat_model_picker_search, ui.chat_model_picker_caret)
        };
        ui.chat_model_picker_search.insert(caret, c);
        ui.chat_model_picker_caret = Some(caret + c.len_utf8());
        ui.chat_model_picker_caret_anchor_ms = self.now_ms;
        ui.chat_model_picker_scroll = 0.0;
        ui.chat_model_picker_hover = None;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn apply_chat_model_picker_backspace(&mut self) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker_open {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        if ui.chat_model_picker_select_all {
            ui.chat_model_picker_search.clear();
            ui.chat_model_picker_caret = Some(0);
            ui.chat_model_picker_select_all = false;
            ui.chat_model_picker_scroll = 0.0;
            ui.chat_model_picker_hover = None;
            ui.chat_model_picker_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        let caret = caret_position(&ui.chat_model_picker_search, ui.chat_model_picker_caret);
        if caret > 0 {
            let start = previous_boundary(&ui.chat_model_picker_search, caret);
            ui.chat_model_picker_search.replace_range(start..caret, "");
            ui.chat_model_picker_caret = Some(start);
            ui.chat_model_picker_scroll = 0.0;
            ui.chat_model_picker_hover = None;
        } else {
            ui.chat_model_picker_caret = Some(0);
        }
        ui.chat_model_picker_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    pub fn apply_chat_model_picker_caret(&mut self, forward: bool) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker_open {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        ui.chat_model_picker_select_all = false;
        let caret = caret_position(&ui.chat_model_picker_search, ui.chat_model_picker_caret);
        ui.chat_model_picker_caret = Some(if forward {
            next_boundary(&ui.chat_model_picker_search, caret)
        } else {
            previous_boundary(&ui.chat_model_picker_search, caret)
        });
        ui.chat_model_picker_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }
}

fn caret_position(value: &str, caret: Option<usize>) -> usize {
    clamp_boundary(value, caret.unwrap_or(value.len()))
}

fn clamp_boundary(value: &str, pos: usize) -> usize {
    let mut pos = pos.min(value.len());
    while pos > 0 && !value.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

fn previous_boundary(value: &str, pos: usize) -> usize {
    let pos = clamp_boundary(value, pos);
    value
        .char_indices()
        .map(|(idx, _)| idx)
        .take_while(|idx| *idx < pos)
        .last()
        .unwrap_or(0)
}

fn next_boundary(value: &str, pos: usize) -> usize {
    let pos = clamp_boundary(value, pos);
    if pos >= value.len() {
        return value.len();
    }
    value
        .char_indices()
        .map(|(idx, _)| idx)
        .find(|idx| *idx > pos)
        .unwrap_or(value.len())
}
