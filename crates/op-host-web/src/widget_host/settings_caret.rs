use super::WidgetHost;
use op_editor_core::agent_settings::SettingsFocus;

impl WidgetHost {
    pub(in crate::widget_host) fn apply_settings_text(&mut self, c: char) -> bool {
        let Some(focus) = self.editor_state.editor_ui.agent_settings.focus else {
            return false;
        };
        let ui = &mut self.editor_state.editor_ui;
        let replacing_all = ui.settings_input_select_all;
        let accepts_draft = if replacing_all {
            ""
        } else {
            ui.settings_input_draft.as_str()
        };
        if !settings_accepts(focus, accepts_draft, c) {
            return false;
        }
        let mut caret = settings_caret_position(
            &ui.settings_input_draft,
            ui.settings_input_caret,
            ui.settings_input_caret_focus,
            focus,
        );
        if replacing_all {
            ui.settings_input_draft.clear();
            ui.settings_input_select_all = false;
            caret = 0;
        }
        ui.settings_input_draft.insert(caret, c);
        ui.settings_input_caret = Some(caret + c.len_utf8());
        ui.settings_input_caret_focus = Some(focus);
        ui.settings_input_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn apply_settings_backspace(&mut self) -> bool {
        let Some(focus) = self.editor_state.editor_ui.agent_settings.focus else {
            return false;
        };
        let ui = &mut self.editor_state.editor_ui;
        if ui.settings_input_select_all {
            ui.settings_input_draft.clear();
            ui.settings_input_caret = Some(0);
            ui.settings_input_caret_focus = Some(focus);
            ui.settings_input_select_all = false;
            ui.settings_input_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        let caret = settings_caret_position(
            &ui.settings_input_draft,
            ui.settings_input_caret,
            ui.settings_input_caret_focus,
            focus,
        );
        if caret > 0 {
            let start = previous_boundary(&ui.settings_input_draft, caret);
            ui.settings_input_draft.replace_range(start..caret, "");
            ui.settings_input_caret = Some(start);
        } else {
            ui.settings_input_caret = Some(0);
        }
        ui.settings_input_caret_focus = Some(focus);
        ui.settings_input_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    pub fn apply_settings_caret(&mut self, forward: bool) -> bool {
        let Some(focus) = self.editor_state.editor_ui.agent_settings.focus else {
            return false;
        };
        let ui = &mut self.editor_state.editor_ui;
        ui.settings_input_select_all = false;
        let caret = settings_caret_position(
            &ui.settings_input_draft,
            ui.settings_input_caret,
            ui.settings_input_caret_focus,
            focus,
        );
        ui.settings_input_caret = Some(if forward {
            next_boundary(&ui.settings_input_draft, caret)
        } else {
            previous_boundary(&ui.settings_input_draft, caret)
        });
        ui.settings_input_caret_focus = Some(focus);
        ui.settings_input_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn clear_settings_caret(&mut self) {
        self.editor_state.editor_ui.settings_input_caret = None;
        self.editor_state.editor_ui.settings_input_caret_focus = None;
        self.editor_state.editor_ui.settings_input_select_all = false;
    }
}

fn settings_accepts(focus: SettingsFocus, draft: &str, c: char) -> bool {
    match focus {
        SettingsFocus::McpPort => c.is_ascii_digit() && draft.len() < 5,
        SettingsFocus::ImageSearch(_)
        | SettingsFocus::BuiltinAgent { .. }
        | SettingsFocus::BuiltinAgentDraft(_)
        | SettingsFocus::ImageGenProfile { .. }
        | SettingsFocus::AcpAgent { .. }
        | SettingsFocus::AcpAgentDraft(_) => !c.is_control() && draft.len() < 512,
    }
}

fn settings_caret_position(
    draft: &str,
    caret: Option<usize>,
    owner: Option<SettingsFocus>,
    focus: SettingsFocus,
) -> usize {
    let pos = if owner == Some(focus) {
        caret.unwrap_or(draft.len())
    } else {
        draft.len()
    };
    clamp_boundary(draft, pos)
}

fn clamp_boundary(draft: &str, pos: usize) -> usize {
    let mut pos = pos.min(draft.len());
    while pos > 0 && !draft.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

fn previous_boundary(draft: &str, pos: usize) -> usize {
    let pos = clamp_boundary(draft, pos);
    draft
        .char_indices()
        .map(|(idx, _)| idx)
        .take_while(|idx| *idx < pos)
        .last()
        .unwrap_or(0)
}

fn next_boundary(draft: &str, pos: usize) -> usize {
    let pos = clamp_boundary(draft, pos);
    if pos >= draft.len() {
        return draft.len();
    }
    draft
        .char_indices()
        .map(|(idx, _)| idx)
        .find(|idx| *idx > pos)
        .unwrap_or(draft.len())
}
