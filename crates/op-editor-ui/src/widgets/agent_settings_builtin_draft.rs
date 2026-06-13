//! Built-in provider draft helpers.

use op_editor_core::agent_settings::{AgentSettings, BuiltinAgentField, SettingsFocus};
use op_editor_core::editor_ui_state::EditorUiState;

pub fn ready(settings: &AgentSettings, ui: &EditorUiState) -> bool {
    let Some(draft) = settings.builtin_agent_draft.as_ref() else {
        return false;
    };
    let api_key = field_value(settings, ui, BuiltinAgentField::ApiKey, &draft.api_key);
    let model = field_value(settings, ui, BuiltinAgentField::Model, &draft.model);
    let name = field_value(
        settings,
        ui,
        BuiltinAgentField::DisplayName,
        &draft.display_name,
    );
    !api_key.trim().is_empty() && !model.trim().is_empty() && !name.trim().is_empty()
}

fn field_value<'a>(
    settings: &AgentSettings,
    ui: &'a EditorUiState,
    field: BuiltinAgentField,
    fallback: &'a str,
) -> &'a str {
    if settings.focus == Some(SettingsFocus::BuiltinAgentDraft(field)) {
        ui.settings_input.text()
    } else {
        fallback
    }
}
