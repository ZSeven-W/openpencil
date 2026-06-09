use super::WidgetHost;
use op_editor_core::agent_settings::{AcpAgentField, SettingsFocus};

#[test]
fn settings_input_arrows_move_caret_for_insert_and_backspace() {
    let mut host = WidgetHost::new();
    {
        let ui = &mut host.editor_state.editor_ui;
        ui.agent_settings.focus = Some(SettingsFocus::AcpAgentDraft(AcpAgentField::Command));
        ui.settings_input_draft = "abcd".into();
    }

    assert!(host.apply_settings_caret(false));
    assert!(host.apply_settings_caret(false));
    assert_eq!(host.editor_state.editor_ui.settings_input_caret, Some(2));

    assert!(host.apply_text('X'));
    assert_eq!(host.editor_state.editor_ui.settings_input_draft, "abXcd");
    assert_eq!(host.editor_state.editor_ui.settings_input_caret, Some(3));

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state.editor_ui.settings_input_draft, "abcd");
    assert_eq!(host.editor_state.editor_ui.settings_input_caret, Some(2));
}

#[test]
fn select_all_in_settings_input_replaces_next_typed_text() {
    let mut host = WidgetHost::new();
    {
        let ui = &mut host.editor_state.editor_ui;
        ui.agent_settings.focus = Some(SettingsFocus::AcpAgentDraft(AcpAgentField::Command));
        ui.settings_input_draft = "node server.js".into();
    }

    assert!(host.apply_select_all());
    assert!(host.apply_text('x'));
    assert_eq!(host.editor_state.editor_ui.settings_input_draft, "x");
    assert_eq!(host.editor_state.editor_ui.settings_input_caret, Some(1));
}

#[test]
fn select_all_in_chat_input_replaces_next_typed_text() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    host.editor_state.chat.input = "abcdef".into();

    assert!(host.apply_select_all());
    assert!(host.apply_text('X'));
    assert_eq!(host.editor_state.chat.input, "X");
}

#[test]
fn select_all_in_chat_model_picker_replaces_next_typed_text() {
    let mut host = WidgetHost::new();
    {
        let ui = &mut host.editor_state.editor_ui;
        ui.chat_model_picker_open = true;
        ui.chat_model_picker_search = "gpt".into();
        ui.chat_model_picker_caret = Some(3);
    }

    assert!(host.apply_select_all());
    assert!(host.apply_text('x'));
    assert_eq!(host.editor_state.editor_ui.chat_model_picker_search, "x");
    assert_eq!(host.editor_state.editor_ui.chat_model_picker_caret, Some(1));
}
