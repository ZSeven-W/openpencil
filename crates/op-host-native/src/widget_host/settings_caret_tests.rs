use super::WidgetHostNative;
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};

#[test]
fn settings_input_uses_text_input_state_for_editing() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.agent_settings.focus =
            Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::BaseUrl));
        ui.settings_input.set_text("abcd");
    }

    assert!(host.apply_settings_caret(false));
    assert!(host.apply_settings_caret(false));
    assert_eq!(host.editor_state().editor_ui.settings_input.caret(), 2);

    assert!(host.apply_text('X'));
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "abXcd");
    assert_eq!(host.editor_state().editor_ui.settings_input.caret(), 3);

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "abcd");
    assert_eq!(host.editor_state().editor_ui.settings_input.caret(), 2);

    assert!(host.apply_select_all());
    assert!(host.apply_text('Z'));
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "Z");
    assert_eq!(host.editor_state().editor_ui.settings_input.caret(), 1);
}
