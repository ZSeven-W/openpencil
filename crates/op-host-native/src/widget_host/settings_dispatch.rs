//! Agent/settings modal input commits.

use super::WidgetHostNative;

impl WidgetHostNative {
    /// Commit any focused settings-modal input.
    pub(in crate::widget_host) fn commit_settings_focus_if_any(&mut self) {
        use op_editor_core::agent_settings::{BuiltinAgentField, ImageGenField, SettingsFocus};
        let Some(focus) = self.editor_state.editor_ui.agent_settings.focus.take() else {
            return;
        };
        let draft = std::mem::take(&mut self.editor_state.editor_ui.settings_input_draft);
        match focus {
            SettingsFocus::McpPort => {
                if let Ok(port) = draft.trim().parse::<u16>() {
                    self.editor_state.editor_ui.agent_settings.mcp_server.port = port.max(1024);
                }
            }
            SettingsFocus::ImageSearch(field) => match field {
                op_editor_core::agent_settings::ImageSearchField::ClientId => {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .openverse_client_id = draft.trim().to_string();
                }
                op_editor_core::agent_settings::ImageSearchField::ClientSecret => {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .openverse_client_secret = draft.trim().to_string();
                }
            },
            SettingsFocus::BuiltinAgent { index, field } => {
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get_mut(index)
                {
                    match field {
                        BuiltinAgentField::DisplayName => {
                            if !draft.trim().is_empty() {
                                agent.display_name = draft.trim().to_string();
                            }
                        }
                        BuiltinAgentField::ApiKey => {
                            agent.api_key = draft.trim().to_string();
                        }
                        BuiltinAgentField::Model => {
                            agent.model = draft.trim().to_string();
                        }
                        BuiltinAgentField::BaseUrl => {
                            agent.base_url = if draft.trim().is_empty() {
                                agent.kind.default_base_url().to_string()
                            } else {
                                draft.trim().to_string()
                            };
                        }
                    }
                    self.editor_state.rebuild_chat_models();
                }
            }
            SettingsFocus::ImageGenProfile { index, field } => {
                if let Some(profile) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get_mut(index)
                {
                    match field {
                        ImageGenField::Name => {
                            profile.name = draft.trim().to_string();
                        }
                        ImageGenField::ApiKey => {
                            profile.api_key = draft.trim().to_string();
                        }
                        ImageGenField::Model => {
                            profile.model = draft.trim().to_string();
                        }
                        ImageGenField::BaseUrl => {
                            profile.base_url = if draft.trim().is_empty() {
                                None
                            } else {
                                Some(draft.trim().to_string())
                            };
                        }
                    }
                }
            }
        }
        self.mark_dirty();
    }
}
