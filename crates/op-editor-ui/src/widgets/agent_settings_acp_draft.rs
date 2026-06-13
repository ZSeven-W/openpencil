//! ACP agent draft helpers.

use op_editor_core::agent_settings::{
    AcpAgentField, AcpConnectionType, AgentSettings, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;

pub fn ready(settings: &AgentSettings, ui: &EditorUiState) -> bool {
    let Some(draft) = settings.acp_agent_draft.as_ref() else {
        return false;
    };
    let name = field_value(
        settings,
        ui,
        AcpAgentField::DisplayName,
        &draft.display_name,
    );
    let endpoint = match draft.connection_type {
        AcpConnectionType::Local => {
            field_value(settings, ui, AcpAgentField::Command, &draft.command)
        }
        AcpConnectionType::Remote => field_value(
            settings,
            ui,
            AcpAgentField::Url,
            draft.url.as_deref().unwrap_or(""),
        ),
    };
    !name.trim().is_empty() && !endpoint.trim().is_empty()
}

fn field_value<'a>(
    settings: &AgentSettings,
    ui: &'a EditorUiState,
    field: AcpAgentField,
    fallback: &'a str,
) -> &'a str {
    if settings.focus == Some(SettingsFocus::AcpAgentDraft(field)) {
        ui.settings_input.text()
    } else {
        fallback
    }
}
