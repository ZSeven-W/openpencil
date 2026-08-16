//! Chat model selection and catalog rebuilds on [`EditorState`].

use crate::EditorState;

impl EditorState {
    /// Select a chat model and close the picker. A chat selection is tab-local:
    /// choosing a row never mutates the provider's saved model list.
    pub fn select_chat_model(&mut self, idx: usize) {
        let update = self.chat.available_models.get(idx).map(|entry| {
            (
                entry.provider,
                entry.builtin_provider_id.is_none() && entry.acp_agent_id().is_none(),
            )
        });
        if let Some((provider, use_native_agent)) = update {
            self.chat.selected_model = idx;
            if use_native_agent {
                if let Some(pidx) = crate::AgentProvider::ALL
                    .iter()
                    .position(|candidate| *candidate == provider)
                {
                    self.editor_ui.chat_selected_agent = pidx;
                }
            }
        }
        self.editor_ui.close_chat_model_picker();
    }

    /// Recompute the active chat tab's selectable catalog. Every ready
    /// built-in contributes only its explicitly saved models. Runtime provider
    /// discovery is deliberately confined to provider settings.
    pub fn rebuild_chat_models(&mut self) {
        let previous = self
            .chat
            .available_models
            .get(self.chat.selected_model)
            .cloned();
        let connected = self.editor_ui.agent_settings.verified_connected_mask();
        self.chat.rebuild_available_models(&connected);

        let builtin_entries = self
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .filter(|agent| agent.ready())
            .flat_map(|agent| {
                agent.models.iter().map(|model| {
                    crate::ModelEntry::builtin_with_display_name(
                        agent.kind.model_provider(),
                        agent.id.clone(),
                        agent.display_name.clone(),
                        format!("builtin:{}:{model}", agent.id),
                        model.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();
        self.chat.available_models.extend(builtin_entries);

        let settings = &self.editor_ui.agent_settings;
        self.chat.available_models.extend(
            settings
                .acp_agents
                .iter()
                .filter(|agent| agent.ready() && settings.acp_agent_verified_connected(&agent.id))
                .map(|agent| crate::ModelEntry::acp(agent.id.clone(), agent.display_name.clone())),
        );
        if let Some(previous) = previous {
            if let Some(index) = self.chat.available_models.iter().position(|entry| {
                entry.provider == previous.provider
                    && entry.value == previous.value
                    && entry.builtin_provider_id == previous.builtin_provider_id
            }) {
                self.chat.selected_model = index;
            }
        }
        if let Some(entry) = self.chat.selected_model_entry() {
            if entry.builtin_provider_id.is_none() && entry.acp_agent_id().is_none() {
                if let Some(index) = crate::AgentProvider::ALL
                    .iter()
                    .position(|candidate| *candidate == entry.provider)
                {
                    self.editor_ui.chat_selected_agent = index;
                }
            }
        }
    }
}
