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
