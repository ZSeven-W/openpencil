//! Focus seeding for the agent-settings modal's text inputs — resolve
//! the current value of the picked field and hand it to the shared
//! settings input.
//!
//! Split out of `agent_settings_press_flow.rs` to keep every file under
//! the repo's 800-line cap.

use op_editor_core::agent_settings::{
    AcpAgentField, AcpConnectionType, BuiltinAgentField, ImageGenField, SettingsFocus,
};
use op_editor_core::host_ui_transitions::set_settings_input_text;
use op_editor_core::EditorState;

pub(crate) fn focus_with_text(
    state: &mut EditorState,
    focus: SettingsFocus,
    text: String,
    now_ms: u64,
) {
    state.editor_ui.agent_settings.focus = Some(focus);
    set_settings_input_text(&mut state.editor_ui, text, now_ms);
}

pub(crate) fn clear_focus(state: &mut EditorState) {
    state.editor_ui.agent_settings.focus = None;
    state.editor_ui.settings_input.set_text("");
}

pub(crate) fn image_gen_profile_id(state: &EditorState, index: usize) -> Option<String> {
    state
        .editor_ui
        .agent_settings
        .image_gen_profiles
        .get(index)
        .map(|profile| profile.id.clone())
}

/// Focus one field of the image-generation profile at `index`, seeding
/// the shared settings input from its current value.
pub fn focus_image_gen_profile(
    state: &mut EditorState,
    index: usize,
    field: ImageGenField,
    now_ms: u64,
) {
    let Some(profile) = state.editor_ui.agent_settings.image_gen_profiles.get(index) else {
        return;
    };
    let text = match field {
        ImageGenField::Name => profile.name.clone(),
        ImageGenField::ApiKey => profile.api_key.clone(),
        ImageGenField::Model => profile.model.clone(),
        ImageGenField::BaseUrl => profile.base_url.clone().unwrap_or_default(),
    };
    focus_with_text(
        state,
        SettingsFocus::ImageGenProfile { index, field },
        text,
        now_ms,
    );
}

pub(crate) fn builtin_field_text(
    agent: &op_editor_core::agent_settings::BuiltinAgentConfig,
    field: BuiltinAgentField,
) -> String {
    match field {
        BuiltinAgentField::DisplayName => agent.display_name.clone(),
        BuiltinAgentField::ApiKey => agent.api_key.clone(),
        BuiltinAgentField::Model => agent.models_text(),
        BuiltinAgentField::BaseUrl => agent.base_url.clone(),
    }
}

/// Focus one field of the built-in agent at `index`. Refuses `BaseUrl`
/// on presets that pin it — the widget hit-test already skips that row
/// (`agent_settings_builtin.rs`), so this is belt-and-braces.
pub(crate) fn focus_builtin_agent(
    state: &mut EditorState,
    index: usize,
    field: BuiltinAgentField,
    now_ms: u64,
) {
    let Some(agent) = state.editor_ui.agent_settings.builtin_agents.get(index) else {
        return;
    };
    if field == BuiltinAgentField::BaseUrl && !agent.base_url_editable() {
        return;
    }
    let text = builtin_field_text(agent, field);
    focus_with_text(
        state,
        SettingsFocus::BuiltinAgent { index, field },
        text,
        now_ms,
    );
}

pub(crate) fn focus_builtin_agent_draft(
    state: &mut EditorState,
    field: BuiltinAgentField,
    now_ms: u64,
) {
    let Some(agent) = state.editor_ui.agent_settings.builtin_agent_draft.as_ref() else {
        return;
    };
    if field == BuiltinAgentField::BaseUrl && !agent.base_url_editable() {
        return;
    }
    let text = builtin_field_text(agent, field);
    focus_with_text(state, SettingsFocus::BuiltinAgentDraft(field), text, now_ms);
}

pub(crate) fn acp_field_text(
    agent: &op_editor_core::agent_settings::AcpAgentConfig,
    field: AcpAgentField,
) -> String {
    match field {
        AcpAgentField::DisplayName => agent.display_name.clone(),
        AcpAgentField::Command => agent.command.clone(),
        AcpAgentField::Args => agent.args_text(),
        AcpAgentField::Env => agent.env_text(),
        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
    }
}

/// The field that carries the persisted transport for `connection_type` —
/// the one an unconfigured ACP card focuses. New M1 drafts are Local-only;
/// Remote remains here for legacy persisted rows.
pub(crate) fn transport_field(connection_type: AcpConnectionType) -> AcpAgentField {
    match connection_type {
        AcpConnectionType::Local => AcpAgentField::Command,
        AcpConnectionType::Remote => AcpAgentField::Url,
    }
}

pub(crate) fn focus_acp_agent(
    state: &mut EditorState,
    index: usize,
    field: AcpAgentField,
    now_ms: u64,
) {
    let Some(agent) = state.editor_ui.agent_settings.acp_agents.get(index) else {
        return;
    };
    let text = acp_field_text(agent, field);
    focus_with_text(
        state,
        SettingsFocus::AcpAgent { index, field },
        text,
        now_ms,
    );
}

pub(crate) fn focus_acp_agent_draft(state: &mut EditorState, field: AcpAgentField, now_ms: u64) {
    let Some(agent) = state.editor_ui.agent_settings.acp_agent_draft.as_ref() else {
        return;
    };
    let text = acp_field_text(agent, field);
    focus_with_text(state, SettingsFocus::AcpAgentDraft(field), text, now_ms);
}

/// Open the usable built-in connection path directly from the novice entry.
/// Existing providers are edited in place; a fresh setup begins by choosing
/// the service so its protocol, model and URL can be populated together.
pub fn open_builtin_setup(state: &mut EditorState, now_ms: u64) {
    use op_editor_core::agent_settings::BuiltinAgentPresetMenuTarget;
    let ui = &mut state.editor_ui;
    ui.home.connect_card_open = false;
    ui.agent_settings_open = true;
    ui.agent_settings.tab = op_editor_core::AgentSettingsTab::Agents;
    ui.agent_settings.scroll_y.offset = 0.0;
    if !ui.agent_settings.builtin_agents.is_empty() {
        let index = ui
            .agent_settings
            .builtin_agents
            .iter()
            .position(|a| a.enabled)
            .unwrap_or(0);
        focus_builtin_agent(state, index, BuiltinAgentField::DisplayName, now_ms);
    } else {
        ui.agent_settings.begin_builtin_agent_draft();
        focus_builtin_agent_draft(state, BuiltinAgentField::DisplayName, now_ms);
        state.editor_ui.agent_settings.builtin_preset_menu_open =
            Some(BuiltinAgentPresetMenuTarget::Draft);
        state
            .editor_ui
            .agent_settings
            .builtin_preset_menu_scroll
            .offset = 0.0;
        state.editor_ui.agent_settings.focus = None;
        state.editor_ui.settings_input.set_text("");
    }
}

#[cfg(test)]
mod novice_setup_tests {
    use super::*;
    #[test]
    fn a_new_connection_begins_at_service_choice_and_never_saves_an_empty_key() {
        let mut state = EditorState::new();
        open_builtin_setup(&mut state, 1);
        assert!(state.editor_ui.agent_settings_open);
        assert!(state
            .editor_ui
            .agent_settings
            .builtin_preset_menu_open
            .is_some());
        assert!(
            state.editor_ui.agent_settings.focus.is_none(),
            "no hidden input owns the software keyboard"
        );
        assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
        assert!(state
            .editor_ui
            .agent_settings
            .builtin_agent_draft
            .as_ref()
            .unwrap()
            .api_key
            .is_empty());
    }
    #[test]
    fn an_existing_connection_is_opened_in_place_without_duplicate_or_credential_changes() {
        let mut state = EditorState::new();
        let id = state
            .editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults("GLM", "synthetic-test-key", "glm-5.3-flash");
        open_builtin_setup(&mut state, 1);
        let settings = &state.editor_ui.agent_settings;
        assert_eq!(settings.builtin_agents.len(), 1);
        assert_eq!(settings.builtin_agents[0].id, id);
        assert_eq!(settings.builtin_agents[0].api_key, "synthetic-test-key");
        assert!(settings.builtin_agent_draft.is_none());
    }
}
