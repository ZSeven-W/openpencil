//! Agent settings modal press dispatcher for the web host.

use super::agent_settings_mcp_server::{mcp_server_request, request_mcp_server_update};
use super::WidgetHost;
use op_editor_core::agent_settings::{
    AcpAgentField, AcpConnectionType, BuiltinAgentField, ImageGenField, ImageSearchField,
    ImageTestStatus, McpCli, SettingsFocus,
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
        let before_mcp = {
            let mcp = self.editor_state.editor_ui.agent_settings.mcp_server;
            (mcp.running, mcp.port)
        };
        let panel = AgentSettingsPanel::for_web_editor(&self.editor_state);
        let panel_rect = panel.rect(vw, vh);
        let hit = panel.hit_test(panel_rect, Point2D::new(x, y));
        self.editor_state.editor_ui.pressed_button =
            op_editor_ui::widgets::editor_state_ext::agent_settings_button(hit)
                .map(op_editor_core::ButtonPressTarget::AgentSettings);
        match hit {
            AgentSettingsHit::Close | AgentSettingsHit::Outside => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings_open = false;
            }
            AgentSettingsHit::SelectTab(tab) => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings.tab = tab;
                self.editor_state.editor_ui.agent_settings.scroll_y.offset = 0.0;
            }
            AgentSettingsHit::Connect(provider) => {
                let settings = &mut self.editor_state.editor_ui.agent_settings;
                if settings.provider_verified_connected(provider) {
                    settings.disconnect_provider(provider);
                } else if !settings.provider_probe_in_flight(provider) {
                    settings.begin_provider_connect(provider);
                }
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
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_client_config_copied_at_ms = Some(self.now_ms);
                let config = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .client_config_clipboard_text();
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
                let text = match field {
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
                self.set_settings_input_text(text);
            }
            AgentSettingsHit::OpenImageRegisterLink => {
                self.commit_settings_focus();
                if let Some(w) = web_sys::window() {
                    // The raw `auth_tokens/register/` endpoint only accepts POST,
                    // so a browser GET lands on a 405 page. Point at the API
                    // reference's auth section (documents how to register).
                    let _ = w.open_with_url_and_target(
                        "https://api.openverse.org/v1/#tag/auth",
                        "_blank",
                    );
                }
            }
            AgentSettingsHit::TestImageSearch => {
                self.commit_settings_focus();
                let settings = &mut self.editor_state.editor_ui.agent_settings;
                let has_client_id = !settings.openverse_client_id.trim().is_empty();
                let has_client_secret = !settings.openverse_client_secret.trim().is_empty();
                settings.images_search_ready = true;
                settings.images_search_test_status = if has_client_id && has_client_secret {
                    ImageTestStatus::Testing
                } else {
                    ImageTestStatus::Invalid
                };
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
            AgentSettingsHit::TestGenConfig(index) => {
                self.commit_settings_focus();
                if let Some(profile) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get_mut(index)
                {
                    profile.test_status = if profile.api_key.trim().is_empty() {
                        ImageTestStatus::Invalid
                    } else {
                        ImageTestStatus::Testing
                    };
                }
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
                    let text = profile.name.clone();
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::ImageGenProfile {
                            index,
                            field: ImageGenField::Name,
                        });
                    self.set_settings_input_text(text);
                }
            }
            AgentSettingsHit::ToggleGenProviderMenu(index) => {
                self.commit_settings_focus();
                {
                    let settings = &mut self.editor_state.editor_ui.agent_settings;
                    settings.image_gen_provider_menu_open =
                        (settings.image_gen_provider_menu_open != Some(index)).then_some(index);
                }
                self.focus_image_gen_profile(index, ImageGenField::Name);
            }
            AgentSettingsHit::SelectGenProvider { index, provider: _ } => {
                self.commit_settings_focus();
                self.focus_image_gen_profile(index, ImageGenField::Name);
            }
            AgentSettingsHit::FocusGenConfig { index, field } => {
                self.commit_settings_focus();
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_provider_menu_open = None;
                self.focus_image_gen_profile(index, field);
            }
            AgentSettingsHit::ToggleAutoUpdate => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .auto_update_enabled ^= true;
            }
            AgentSettingsHit::SelectPencilCursor(style) => {
                self.editor_state.editor_ui.pencil_cursor_style = style;
            }
            AgentSettingsHit::ToggleExperimental => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .experimental_features_enabled ^= true;
                if !self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .experimental_features_enabled
                {
                    // Gate off: web preview is just the core flag (no
                    // host-owned runtime), so exit through it. Also drop
                    // stale Widget property focus so a hidden field can't
                    // commit through dispatch.
                    if self.editor_state.editor_ui.preview_mode {
                        self.editor_state.editor_ui.exit_preview();
                    }
                    self.editor_state.ui.property_focus = None;
                }
            }
            AgentSettingsHit::FocusMcpPort => {
                self.commit_settings_focus();
                self.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
                let text = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .port
                    .to_string();
                self.set_settings_input_text(text);
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
                    let text = match field {
                        BuiltinAgentField::DisplayName => agent.display_name.clone(),
                        BuiltinAgentField::ApiKey => agent.api_key.clone(),
                        BuiltinAgentField::Model => agent.model.clone(),
                        BuiltinAgentField::BaseUrl => agent.base_url.clone(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::BuiltinAgent { index, field });
                    self.set_settings_input_text(text);
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
                    let text = match field {
                        BuiltinAgentField::DisplayName => agent.display_name.clone(),
                        BuiltinAgentField::ApiKey => agent.api_key.clone(),
                        BuiltinAgentField::Model => agent.model.clone(),
                        BuiltinAgentField::BaseUrl => agent.base_url.clone(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::BuiltinAgentDraft(field));
                    self.set_settings_input_text(text);
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
                settings.builtin_preset_menu_scroll.offset = 0.0;
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
                    .builtin_preset_menu_scroll
                    .offset = 0.0;
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
                    let text = agent.display_name.clone();
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::BuiltinAgent {
                            index,
                            field: BuiltinAgentField::DisplayName,
                        });
                    self.set_settings_input_text(text);
                }
            }
            AgentSettingsHit::RemoveBuiltinAgent(index) => {
                self.commit_settings_focus();
                let agents = &mut self.editor_state.editor_ui.agent_settings.builtin_agents;
                if index < agents.len() {
                    agents.remove(index);
                    self.editor_state.editor_ui.agent_settings.focus = None;
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
                let text = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agent_draft
                    .as_ref()
                    .map(|agent| agent.api_key.clone())
                    .unwrap_or_default();
                self.set_settings_input_text(text);
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
                    self.clear_settings_caret();
                    self.editor_state.rebuild_chat_models();
                } else {
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
                    self.set_settings_input_text("");
                }
            }
            AgentSettingsHit::CancelBuiltinAgentDraft => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .cancel_builtin_agent_draft();
                self.editor_state.editor_ui.agent_settings.focus = None;
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
                    let text = match field {
                        AcpAgentField::DisplayName => agent.display_name.clone(),
                        AcpAgentField::Command => agent.command.clone(),
                        AcpAgentField::Args => agent.args_text(),
                        AcpAgentField::Env => agent.env_text(),
                        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgent { index, field });
                    self.set_settings_input_text(text);
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
                    let text = match field {
                        AcpAgentField::DisplayName => agent.display_name.clone(),
                        AcpAgentField::Command => agent.command.clone(),
                        AcpAgentField::Args => agent.args_text(),
                        AcpAgentField::Env => agent.env_text(),
                        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgentDraft(field));
                    self.set_settings_input_text(text);
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
                    let text = match field {
                        AcpAgentField::Command => agent.command.clone(),
                        AcpAgentField::Args => agent.args_text(),
                        AcpAgentField::Env => agent.env_text(),
                        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                        AcpAgentField::DisplayName => agent.display_name.clone(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgent { index, field });
                    self.set_settings_input_text(text);
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
                    let text = match field {
                        AcpAgentField::Command => agent.command.clone(),
                        AcpAgentField::Args => agent.args_text(),
                        AcpAgentField::Env => agent.env_text(),
                        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                        AcpAgentField::DisplayName => agent.display_name.clone(),
                    };
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgentDraft(field));
                    self.set_settings_input_text(text);
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
                    let text = agent.display_name.clone();
                    self.editor_state.editor_ui.agent_settings.focus =
                        Some(SettingsFocus::AcpAgent {
                            index,
                            field: AcpAgentField::DisplayName,
                        });
                    self.set_settings_input_text(text);
                }
            }
            AgentSettingsHit::RemoveAcpAgent(index) => {
                self.commit_settings_focus();
                let agents = &mut self.editor_state.editor_ui.agent_settings.acp_agents;
                if index < agents.len() {
                    agents.remove(index);
                    self.editor_state.editor_ui.agent_settings.focus = None;
                    self.clear_settings_caret();
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::ToggleAcpConnected(index) => {
                self.commit_settings_focus();
                let settings = &self.editor_state.editor_ui.agent_settings;
                let needs_config_focus = settings.acp_agents.get(index).is_some_and(|agent| {
                    !settings.acp_agent_verified_connected(&agent.id) && !agent.ready()
                });
                if needs_config_focus {
                    if let Some(agent) = self
                        .editor_state
                        .editor_ui
                        .agent_settings
                        .acp_agents
                        .get(index)
                    {
                        let field = match agent.connection_type {
                            AcpConnectionType::Local => AcpAgentField::Command,
                            AcpConnectionType::Remote => AcpAgentField::Url,
                        };
                        let text = match field {
                            AcpAgentField::Command => agent.command.clone(),
                            AcpAgentField::Args => agent.args_text(),
                            AcpAgentField::Env => agent.env_text(),
                            AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
                            AcpAgentField::DisplayName => agent.display_name.clone(),
                        };
                        self.editor_state.editor_ui.agent_settings.focus =
                            Some(SettingsFocus::AcpAgent { index, field });
                        self.set_settings_input_text(text);
                    }
                } else if self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agent_verified_connected_at(index)
                {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .disconnect_acp_agent(index);
                    self.editor_state.rebuild_chat_models();
                } else {
                    let started = self
                        .editor_state
                        .editor_ui
                        .agent_settings
                        .begin_acp_agent_connect(index)
                        .is_some();
                    if started {
                        self.editor_state.rebuild_chat_models();
                    }
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
                let text = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agent_draft
                    .as_ref()
                    .map(|agent| agent.command.clone())
                    .unwrap_or_default();
                self.set_settings_input_text(text);
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
                    self.set_settings_input_text("");
                }
            }
            AgentSettingsHit::CancelAcpAgentDraft => {
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .cancel_acp_agent_draft();
                self.editor_state.editor_ui.agent_settings.focus = None;
                self.clear_settings_caret();
            }
            AgentSettingsHit::Inside => {
                // Modal chrome that hit no control — blank press;
                // commits the focused settings input (and blurs the
                // rest of the chrome inputs under the modal).
                self.blur_text_inputs_on_blank_press();
            }
        }
        let after_mcp = {
            let mcp = self.editor_state.editor_ui.agent_settings.mcp_server;
            (mcp.running, mcp.port)
        };
        if let Some(request) =
            mcp_server_request(before_mcp.0, before_mcp.1, after_mcp.0, after_mcp.1)
        {
            request_mcp_server_update(request);
        }
        self.mark_dirty();
        true
    }
}

impl WidgetHost {
    pub(in crate::widget_host) fn focus_image_gen_profile(
        &mut self,
        index: usize,
        field: ImageGenField,
    ) {
        if let Some(profile) = self
            .editor_state
            .editor_ui
            .agent_settings
            .image_gen_profiles
            .get(index)
        {
            let text = match field {
                ImageGenField::Name => profile.name.clone(),
                ImageGenField::ApiKey => profile.api_key.clone(),
                ImageGenField::Model => profile.model.clone(),
                ImageGenField::BaseUrl => profile.base_url.clone().unwrap_or_default(),
            };
            self.editor_state.editor_ui.agent_settings.focus =
                Some(SettingsFocus::ImageGenProfile { index, field });
            self.set_settings_input_text(text);
        }
    }
}
