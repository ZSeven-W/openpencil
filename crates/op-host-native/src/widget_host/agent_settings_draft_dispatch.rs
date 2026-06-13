//! Draft Add/Save/Cancel helpers for the agent-settings modal.

use super::WidgetHostNative;
use op_editor_core::agent_settings::{
    AcpAgentField, AcpConnectionType, BuiltinAgentField, SettingsFocus,
};

impl WidgetHostNative {
    pub(in crate::widget_host) fn focus_builtin_agent_draft(&mut self, field: BuiltinAgentField) {
        self.commit_settings_focus_if_any();
        if let Some(agent) = self
            .editor_state
            .editor_ui
            .agent_settings
            .builtin_agent_draft
            .as_ref()
        {
            if field == BuiltinAgentField::BaseUrl && !agent.base_url_editable() {
                return;
            }
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

    pub(in crate::widget_host) fn toggle_builtin_agent_draft_kind(&mut self) {
        self.commit_settings_focus_if_any();
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

    pub(in crate::widget_host) fn begin_builtin_agent_draft(&mut self) {
        self.commit_settings_focus_if_any();
        self.editor_state
            .editor_ui
            .agent_settings
            .begin_builtin_agent_draft();
        self.focus_builtin_agent_draft(BuiltinAgentField::ApiKey);
    }

    pub(in crate::widget_host) fn save_builtin_agent_draft(&mut self) {
        self.commit_settings_focus_if_any();
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
            self.focus_builtin_agent_draft(BuiltinAgentField::ApiKey);
        }
    }

    pub(in crate::widget_host) fn cancel_builtin_agent_draft(&mut self) {
        self.editor_state
            .editor_ui
            .agent_settings
            .cancel_builtin_agent_draft();
        self.editor_state.editor_ui.agent_settings.focus = None;
        self.clear_settings_caret();
    }

    pub(in crate::widget_host) fn focus_acp_agent_draft(&mut self, field: AcpAgentField) {
        self.commit_settings_focus_if_any();
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

    pub(in crate::widget_host) fn toggle_acp_agent_draft_connection_type(&mut self) {
        self.commit_settings_focus_if_any();
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

    pub(in crate::widget_host) fn begin_acp_agent_draft(&mut self) {
        self.commit_settings_focus_if_any();
        self.editor_state
            .editor_ui
            .agent_settings
            .begin_acp_agent_draft();
        self.focus_acp_agent_draft(AcpAgentField::Command);
    }

    pub(in crate::widget_host) fn save_acp_agent_draft(&mut self) {
        self.commit_settings_focus_if_any();
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
            self.focus_acp_agent_draft(field);
        }
    }

    pub(in crate::widget_host) fn cancel_acp_agent_draft(&mut self) {
        self.editor_state
            .editor_ui
            .agent_settings
            .cancel_acp_agent_draft();
        self.editor_state.editor_ui.agent_settings.focus = None;
        self.clear_settings_caret();
    }
}
