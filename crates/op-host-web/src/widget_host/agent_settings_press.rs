//! Agent settings modal press dispatcher for the web host.

use super::WidgetHost;
use op_editor_core::agent_settings::{
    AcpAgentField, AcpConnectionType, AgentProvider, BuiltinAgentField, ImageGenField,
    ImageSearchField, McpCli, SettingsFocus,
};
use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use op_editor_ui::Point2D;

impl WidgetHost {
    /// Returns true once the modal swallowed the press.
    pub(in crate::widget_host) fn dispatch_agent_settings_press(
        &mut self,
        x: f32,
        y: f32,
        vw: f32,
        vh: f32,
    ) -> bool {
        self.refresh_layout_scene();
        let panel = AgentSettingsPanel::for_editor(&self.editor_state);
        let panel_rect = panel.rect(vw, vh);
        match panel.hit_test(panel_rect, Point2D::new(x, y)) {
            AgentSettingsHit::Close | AgentSettingsHit::Outside => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings_open = false;
            }
            AgentSettingsHit::SelectTab(tab) => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings.tab = tab;
                self.editor_state.editor_ui.agent_settings.scroll_y = 0.0;
            }
            AgentSettingsHit::Connect(provider) => {
                let idx = AgentProvider::ALL
                    .iter()
                    .position(|candidate| *candidate == provider)
                    .unwrap_or(0);
                self.editor_state.editor_ui.agent_settings.connected[idx] ^= true;
                self.editor_state.rebuild_chat_models();
            }
            AgentSettingsHit::ToggleMcpServer => {
                self.commit_settings_focus();
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .running ^= true;
            }
            AgentSettingsHit::ToggleMcpCli(cli) => {
                let idx = McpCli::ALL
                    .iter()
                    .position(|candidate| *candidate == cli)
                    .unwrap_or(0);
                self.editor_state.editor_ui.agent_settings.mcp_cli_enabled[idx] ^= true;
                if self.editor_state.editor_ui.agent_settings.mcp_cli_enabled[idx] {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .mcp_server
                        .running = true;
                }
            }
            AgentSettingsHit::CopyMcpClientConfig => {
                self.commit_settings_focus();
                let config = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .client_config_text();
                self.editor_state.chat.queue_copy_text(config);
            }
            AgentSettingsHit::ToggleImagesAdvanced => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .images_advanced_open ^= true;
            }
            AgentSettingsHit::FocusSearchField(field) => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.settings_input_draft = match field {
                    ImageSearchField::ClientId => self
                        .editor_state
                        .editor_ui
                        .agent_settings
                        .openverse_client_id
                        .clone(),
                    ImageSearchField::ClientSecret => self
                        .editor_state
                        .editor_ui
                        .agent_settings
                        .openverse_client_secret
                        .clone(),
                };
                self.editor_state.editor_ui.agent_settings.focus =
                    Some(SettingsFocus::ImageSearch(field));
                self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
            }
            AgentSettingsHit::TestImageSearch => {
                self.commit_settings_focus();
                let settings = &mut self.editor_state.editor_ui.agent_settings;
                settings.images_search_ready = !settings.openverse_client_id.trim().is_empty()
                    && !settings.openverse_client_secret.trim().is_empty();
            }
            AgentSettingsHit::SetActiveGenConfig(index) => {
                self.commit_settings_focus();
                if let Some(id) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get(index)
                    .map(|profile| profile.id.clone())
                {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .set_active_image_gen_profile(&id);
                }
            }
            AgentSettingsHit::RemoveGenConfig(index) => {
                self.commit_settings_focus();
                if let Some(id) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get(index)
                    .map(|profile| profile.id.clone())
                {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .remove_image_gen_profile(&id);
                }
            }
            AgentSettingsHit::TestGenConfig(_) => {
                self.commit_settings_focus();
            }
            AgentSettingsHit::ToggleGenConfigEditor(index) => {
                let was_editing = matches!(
                    self.editor_state.editor_ui.agent_settings.focus,
                    Some(SettingsFocus::ImageGenProfile {
                        index: focused,
                        ..
                    }) if focused == index
                );
                self.commit_settings_focus();
                if !was_editing {
                    self.focus_image_gen_profile(index, ImageGenField::Name);
                }
            }
            AgentSettingsHit::AddGenConfig => {
                self.commit_settings_focus();
                let id = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .add_image_gen_profile();
                let index = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .iter()
                    .position(|profile| profile.id == id)
                    .unwrap_or(0);
                if let Some(profile) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get(index)
                {
                    self.editor_state.editor_ui.settings_input_draft = profile.name.clone();
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::ImageGenProfile {
                            index,
                            field: ImageGenField::Name,
                        });
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::CycleGenProvider(index) => {
                self.commit_settings_focus();
                if let Some(profile) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get_mut(index)
                {
                    profile.provider = profile.provider.next();
                    profile.model.clear();
                }
            }
            AgentSettingsHit::FocusGenConfig { index, field } => {
                self.commit_settings_focus();
                self.focus_image_gen_profile(index, field);
            }
            AgentSettingsHit::ToggleAutoUpdate => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .auto_update_enabled ^= true;
            }
            AgentSettingsHit::FocusMcpPort => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
                self.editor_state.editor_ui.settings_input_draft = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .port
                    .to_string();
                self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
            }
            AgentSettingsHit::FocusBuiltinAgent { index, field } => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get(index)
                {
                    self.editor_state.editor_ui.settings_input_draft = match field {
                        BuiltinAgentField::DisplayName => agent.display_name.clone(),
                        BuiltinAgentField::ApiKey => agent.api_key.clone(),
                        BuiltinAgentField::Model => agent.model.clone(),
                        BuiltinAgentField::BaseUrl => agent.base_url.clone(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::BuiltinAgent { index, field });
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::FocusBuiltinAgentDraft(field) => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agent_draft
                    .as_ref()
                {
                    self.editor_state.editor_ui.settings_input_draft = match field {
                        BuiltinAgentField::DisplayName => agent.display_name.clone(),
                        BuiltinAgentField::ApiKey => agent.api_key.clone(),
                        BuiltinAgentField::Model => agent.model.clone(),
                        BuiltinAgentField::BaseUrl => agent.base_url.clone(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::BuiltinAgentDraft(field));
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::ToggleBuiltinAgentKind(index) => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get_mut(index)
                {
                    agent.toggle_kind_for_preset();
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::ToggleBuiltinAgentDraftKind => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agent_draft
                    .as_mut()
                {
                    agent.toggle_kind_for_preset();
                }
            }
            AgentSettingsHit::ToggleBuiltinAgentPresetMenu(index) => {
                self.commit_settings_focus();
                let target = match index {
                    Some(index) => {
                        op_editor_core::agent_settings::BuiltinAgentPresetMenuTarget::Agent(index)
                    }
                    None => op_editor_core::agent_settings::BuiltinAgentPresetMenuTarget::Draft,
                };
                let settings = &mut self.editor_state.editor_ui.agent_settings;
                settings.builtin_preset_menu_open =
                    (settings.builtin_preset_menu_open != Some(target)).then_some(target);
                settings.builtin_preset_menu_scroll = 0.0;
                settings.builtin_preset_menu_hover = None;
            }
            AgentSettingsHit::SelectBuiltinAgentPreset { index, preset } => {
                self.commit_settings_focus();
                match index {
                    Some(index) => {
                        self.editor_state
                            .editor_ui
                            .agent_settings
                            .set_builtin_agent_preset(index, preset);
                        self.editor_state.rebuild_chat_models();
                    }
                    None => self
                        .editor_state
                        .editor_ui
                        .agent_settings
                        .set_builtin_agent_draft_preset(preset),
                }
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_preset_menu_open = None;
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_preset_menu_scroll = 0.0;
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_preset_menu_hover = None;
            }
            AgentSettingsHit::ToggleBuiltinAgentEnabled(index) => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get_mut(index)
                {
                    agent.enabled = !agent.enabled;
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::EditBuiltinAgent(index) => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get(index)
                {
                    self.editor_state.editor_ui.settings_input_draft = agent.display_name.clone();
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::BuiltinAgent {
                            index,
                            field: BuiltinAgentField::DisplayName,
                        });
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::RemoveBuiltinAgent(index) => {
                self.commit_settings_focus();
                let agents = &mut self.editor_state.editor_ui.agent_settings.builtin_agents;
                if index < agents.len() {
                    agents.remove(index);
                    self.editor_state.editor_ui.agent_settings.focus = None;
                    self.editor_state.editor_ui.settings_input_draft.clear();
                    self.clear_settings_caret();
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::AddProvider => {
                self.commit_settings_focus();
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .begin_builtin_agent_draft();
                self.editor_state.editor_ui.agent_settings.focus =
                    Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
                self.editor_state.editor_ui.settings_input_draft = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agent_draft
                    .as_ref()
                    .map(|agent| agent.api_key.clone())
                    .unwrap_or_default();
                self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
            }
            AgentSettingsHit::SaveBuiltinAgentDraft => {
                self.commit_settings_focus();
                if self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .save_builtin_agent_draft()
                    .is_some()
                {
                    self.editor_state.editor_ui.agent_settings.focus = None;
                    self.editor_state.editor_ui.settings_input_draft.clear();
                    self.clear_settings_caret();
                    self.editor_state.rebuild_chat_models();
                } else {
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
                    self.editor_state.editor_ui.settings_input_draft.clear();
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::CancelBuiltinAgentDraft => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .cancel_builtin_agent_draft();
                self.editor_state.editor_ui.agent_settings.focus = None;
                self.editor_state.editor_ui.settings_input_draft.clear();
                self.clear_settings_caret();
            }
            AgentSettingsHit::FocusAcpAgent { index, field } => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agents
                    .get(index)
                {
                    self.editor_state.editor_ui.settings_input_draft = match field {
                        AcpAgentField::DisplayName => agent.display_name.clone(),
                        AcpAgentField::Command => agent.command.clone(),
                        AcpAgentField::Args => agent.args_text(),
                        AcpAgentField::Env => agent.env_text(),
                        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgent { index, field });
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::FocusAcpAgentDraft(field) => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agent_draft
                    .as_ref()
                {
                    self.editor_state.editor_ui.settings_input_draft = match field {
                        AcpAgentField::DisplayName => agent.display_name.clone(),
                        AcpAgentField::Command => agent.command.clone(),
                        AcpAgentField::Args => agent.args_text(),
                        AcpAgentField::Env => agent.env_text(),
                        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgentDraft(field));
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::ToggleAcpConnectionType(index) => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agents
                    .get_mut(index)
                {
                    agent.connection_type = match agent.connection_type {
                        AcpConnectionType::Local => AcpConnectionType::Remote,
                        AcpConnectionType::Remote => AcpConnectionType::Local,
                    };
                    agent.connected = false;
                    let field = match agent.connection_type {
                        AcpConnectionType::Local => AcpAgentField::Command,
                        AcpConnectionType::Remote => AcpAgentField::Url,
                    };
                    self.editor_state.editor_ui.settings_input_draft = match field {
                        AcpAgentField::Command => agent.command.clone(),
                        AcpAgentField::Args => agent.args_text(),
                        AcpAgentField::Env => agent.env_text(),
                        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                        AcpAgentField::DisplayName => agent.display_name.clone(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgent { index, field });
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::ToggleAcpDraftConnectionType => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agent_draft
                    .as_mut()
                {
                    agent.connection_type = match agent.connection_type {
                        AcpConnectionType::Local => AcpConnectionType::Remote,
                        AcpConnectionType::Remote => AcpConnectionType::Local,
                    };
                    agent.connected = false;
                    let field = match agent.connection_type {
                        AcpConnectionType::Local => AcpAgentField::Command,
                        AcpConnectionType::Remote => AcpAgentField::Url,
                    };
                    self.editor_state.editor_ui.settings_input_draft = match field {
                        AcpAgentField::Command => agent.command.clone(),
                        AcpAgentField::Args => agent.args_text(),
                        AcpAgentField::Env => agent.env_text(),
                        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                        AcpAgentField::DisplayName => agent.display_name.clone(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgentDraft(field));
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::EditAcpAgent(index) => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agents
                    .get(index)
                {
                    self.editor_state.editor_ui.settings_input_draft = agent.display_name.clone();
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgent {
                            index,
                            field: AcpAgentField::DisplayName,
                        });
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::RemoveAcpAgent(index) => {
                self.commit_settings_focus();
                let agents = &mut self.editor_state.editor_ui.agent_settings.acp_agents;
                if index < agents.len() {
                    agents.remove(index);
                    self.editor_state.editor_ui.agent_settings.focus = None;
                    self.editor_state.editor_ui.settings_input_draft.clear();
                    self.clear_settings_caret();
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::ToggleAcpConnected(index) => {
                self.commit_settings_focus();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agents
                    .get_mut(index)
                {
                    if agent.connected {
                        agent.connected = false;
                    } else if agent.ready() {
                        agent.connected = true;
                    } else {
                        let field = match agent.connection_type {
                            AcpConnectionType::Local => AcpAgentField::Command,
                            AcpConnectionType::Remote => AcpAgentField::Url,
                        };
                        self.editor_state.editor_ui.settings_input_draft = match field {
                            AcpAgentField::Command => agent.command.clone(),
                            AcpAgentField::Args => agent.args_text(),
                            AcpAgentField::Env => agent.env_text(),
                            AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                            AcpAgentField::DisplayName => agent.display_name.clone(),
                        };
                        self.editor_state.editor_ui.agent_settings.focus =
                            Some(SettingsFocus::AcpAgent { index, field });
                        self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                    }
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::AddAcpAgent => {
                self.commit_settings_focus();
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .begin_acp_agent_draft();
                self.editor_state.editor_ui.agent_settings.focus =
                    Some(SettingsFocus::AcpAgentDraft(AcpAgentField::Command));
                self.editor_state.editor_ui.settings_input_draft = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agent_draft
                    .as_ref()
                    .map(|agent| agent.command.clone())
                    .unwrap_or_default();
                self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
            }
            AgentSettingsHit::SaveAcpAgentDraft => {
                self.commit_settings_focus();
                if self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .save_acp_agent_draft()
                    .is_some()
                {
                    self.editor_state.editor_ui.agent_settings.focus = None;
                    self.editor_state.editor_ui.settings_input_draft.clear();
                    self.clear_settings_caret();
                } else {
                    let field = self
                        .editor_state
                        .editor_ui
                        .agent_settings
                        .acp_agent_draft
                        .as_ref()
                        .map(|agent| match agent.connection_type {
                            AcpConnectionType::Local => AcpAgentField::Command,
                            AcpConnectionType::Remote => AcpAgentField::Url,
                        })
                        .unwrap_or(AcpAgentField::Command);
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgentDraft(field));
                    self.editor_state.editor_ui.settings_input_draft.clear();
                    self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
                }
            }
            AgentSettingsHit::CancelAcpAgentDraft => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .cancel_acp_agent_draft();
                self.editor_state.editor_ui.agent_settings.focus = None;
                self.editor_state.editor_ui.settings_input_draft.clear();
                self.clear_settings_caret();
            }
            AgentSettingsHit::Inside => {}
        }
        self.mark_dirty();
        true
    }
}

impl WidgetHost {
    fn focus_image_gen_profile(&mut self, index: usize, field: ImageGenField) {
        if let Some(profile) = self
            .editor_state
            .editor_ui
            .agent_settings
            .image_gen_profiles
            .get(index)
        {
            self.editor_state.editor_ui.settings_input_draft = match field {
                ImageGenField::Name => profile.name.clone(),
                ImageGenField::ApiKey => profile.api_key.clone(),
                ImageGenField::Model => profile.model.clone(),
                ImageGenField::BaseUrl => profile.base_url.clone().unwrap_or_default(),
            };
            self.editor_state.editor_ui.agent_settings.focus =
                Some(SettingsFocus::ImageGenProfile { index, field });
            self.editor_state.editor_ui.settings_input_caret_anchor_ms = self.now_ms;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::agent_settings::AgentSettingsTab;

    #[test]
    fn toggling_builtin_kind_commits_focused_api_key_draft() {
        let mut host = WidgetHost::new();
        host.editor_state
            .editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults("MINIMAX", "", "MiniMax-M2.7");
        host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        });
        host.editor_state.editor_ui.settings_input_draft = "sk-web".into();

        let panel = AgentSettingsPanel::for_editor(&host.editor_state);
        let rect = panel.rect(1200.0, 800.0);
        let content_x = rect.origin.x + 200.0 + 24.0;
        let content_y = rect.origin.y + 24.0;
        let content_w = rect.size.x - 200.0 - 48.0;
        let first_card_y = content_y + 12.0 + 28.0 + 28.0;
        let kind_x = content_x + content_w - 172.0 + 120.0;
        let kind_y = first_card_y + 22.0;

        assert!(host.dispatch_agent_settings_press(kind_x, kind_y, 1200.0, 800.0));

        let agent = &host.editor_state.editor_ui.agent_settings.builtin_agents[0];
        assert_eq!(agent.api_key, "sk-web");
        assert_eq!(
            agent.kind,
            op_editor_core::agent_settings::BuiltinAgentKind::OpenAiCompat
        );
        assert!(host.editor_state.editor_ui.agent_settings.focus.is_none());
    }

    #[test]
    fn add_provider_opens_unsaved_builtin_agent_draft() {
        let mut host = WidgetHost::new();
        host.set_now_ms(1234);

        let panel = AgentSettingsPanel::for_editor(&host.editor_state);
        let rect = panel.rect(1200.0, 800.0);
        let content_x = rect.origin.x + 200.0 + 24.0;
        let content_y = rect.origin.y + 24.0;
        let content_w = rect.size.x - 200.0 - 48.0;
        let add_x = content_x + content_w - 48.0;
        let add_y = content_y + 24.0;

        assert!(host.dispatch_agent_settings_press(add_x, add_y, 1200.0, 800.0));

        let settings = &host.editor_state.editor_ui.agent_settings;
        assert!(settings.builtin_agents.is_empty());
        assert!(settings.builtin_agent_draft.is_some());
        assert_eq!(
            settings.focus,
            Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey))
        );
        assert_eq!(host.editor_state.editor_ui.settings_input_draft, "");
        assert_eq!(
            host.editor_state.editor_ui.settings_input_caret_anchor_ms,
            1234
        );
    }

    #[test]
    fn builtin_provider_menu_selects_ts_preset_for_draft() {
        let mut host = WidgetHost::new();
        let panel = AgentSettingsPanel::for_editor(&host.editor_state);
        let rect = panel.rect(1200.0, 800.0);
        let content_x = rect.origin.x + 200.0 + 24.0;
        let content_y = rect.origin.y + 24.0;
        let content_w = rect.size.x - 200.0 - 48.0;
        let add_x = content_x + content_w - 48.0;
        let add_y = content_y + 24.0;

        assert!(host.dispatch_agent_settings_press(add_x, add_y, 1200.0, 800.0));
        let card_y = content_y + 12.0 + 28.0 + 28.0;
        let provider_x = content_x + 68.0 + 24.0;
        let provider_y = card_y + 60.0;
        assert!(host.dispatch_agent_settings_press(provider_x, provider_y, 1200.0, 800.0));
        let minimax_y = card_y + 76.0 + 4.0 + 5.0 * 24.0 + 12.0;
        assert!(host.dispatch_agent_settings_press(provider_x, minimax_y, 1200.0, 800.0));

        let draft = host
            .editor_state
            .editor_ui
            .agent_settings
            .builtin_agent_draft
            .as_ref()
            .expect("draft remains open");
        assert_eq!(draft.preset, op_editor_core::BuiltinAgentPresetKey::MiniMax);
        assert_eq!(draft.display_name, "MiniMax");
        assert_eq!(draft.model, "MiniMax-M2.7");
        assert_eq!(draft.base_url, "https://api.minimaxi.com/anthropic");
    }

    #[test]
    fn save_builtin_agent_draft_persists_provider() {
        let mut host = WidgetHost::new();
        let panel = AgentSettingsPanel::for_editor(&host.editor_state);
        let rect = panel.rect(1200.0, 800.0);
        let content_x = rect.origin.x + 200.0 + 24.0;
        let content_y = rect.origin.y + 24.0;
        let content_w = rect.size.x - 200.0 - 48.0;
        let add_x = content_x + content_w - 48.0;
        let add_y = content_y + 24.0;

        assert!(host.dispatch_agent_settings_press(add_x, add_y, 1200.0, 800.0));
        for c in "sk-web".chars() {
            assert!(host.apply_text(c));
        }
        let card_y = content_y + 12.0 + 28.0 + 28.0;
        let save_x = content_x + content_w - 12.0 - 34.0;
        let save_y = card_y + 196.0 + 18.0;
        assert!(host.dispatch_agent_settings_press(save_x, save_y, 1200.0, 800.0));

        let settings = &host.editor_state.editor_ui.agent_settings;
        assert_eq!(settings.builtin_agents.len(), 1);
        assert!(settings.builtin_agent_draft.is_none());
        assert_eq!(settings.builtin_agents[0].api_key, "sk-web");
        assert!(settings.focus.is_none());
    }

    #[test]
    fn image_generation_profile_header_click_toggles_editor_closed() {
        let mut host = WidgetHost::new();
        host.editor_state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
        host.editor_state
            .editor_ui
            .agent_settings
            .add_image_gen_profile();
        host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
            index: 0,
            field: ImageGenField::Name,
        });
        host.editor_state.editor_ui.settings_input_draft = "Config 1".into();

        let panel = AgentSettingsPanel::for_editor(&host.editor_state);
        let rect = panel.rect(1200.0, 800.0);
        let content_x = rect.origin.x + 200.0 + 24.0;
        let content_y = rect.origin.y + 24.0;
        let gen_top = content_y + 36.0 + 24.0 + 28.0;
        let row_y = gen_top + 36.0;

        assert!(host.dispatch_agent_settings_press(content_x + 72.0, row_y + 16.0, 1200.0, 800.0));

        assert_eq!(host.editor_state.editor_ui.agent_settings.focus, None);
        assert!(host.editor_state.editor_ui.settings_input_draft.is_empty());
    }
}
