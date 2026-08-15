//! Settings-modal draft commit shared by the native and web widget
//! hosts.
//!
//! Both twins carried this ~190-line `SettingsFocus` walk
//! (`widget_host/settings_dispatch.rs::commit_settings_focus_if_any` on
//! native, `widget_host/keyboard_settings_commit.rs::
//! commit_settings_focus` on the web) as near-identical copies. The only
//! genuine difference is credential OWNERSHIP — see
//! [`SettingsCommitScope`] — so that is the one parameter; everything
//! else is one body.

use crate::agent_settings::{
    AcpAgentField, BuiltinAgentField, ImageGenField, ImageSearchField, SettingsFocus,
};
use crate::editor_ui_state::EditorUiState;
use crate::state::EditorState;

/// Who owns the credential entry a settings draft is about to write to.
///
/// A browser-pushed credential snapshot identifies itself through a
/// scoped id (`web-credential:builtin:…` / `web-credential:image:…`) and
/// through `openverse_credential_owner`. When the DESKTOP operator edits
/// such an entry, the edit is an ownership transfer: the entry is re-idded
/// as local (`AgentSettings::take_over_browser_*`) and the Openverse
/// owner tag is dropped, so the daemon's `web_credentials` merge stops
/// treating it as browser-managed. The browser must NOT do any of that —
/// there it is already the owner, and re-idding its own snapshot would
/// orphan the entry on the next sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCommitScope {
    /// Desktop / daemon operator — commits transfer ownership.
    Operator,
    /// In-browser host — commits leave the credential scoping alone.
    Browser,
}

impl SettingsCommitScope {
    fn takes_over_browser_entries(self) -> bool {
        matches!(self, Self::Operator)
    }
}

/// Commit the focused settings-modal input and drop the focus + caret.
///
/// Returns `true` when a focus was actually taken (so the host marks
/// dirty); `false` is the no-focus fast path both twins used to spell as
/// an early `return`.
pub fn commit_settings_focus(
    state: &mut EditorState,
    scope: SettingsCommitScope,
    now_ms: u64,
) -> bool {
    let Some(focus) = state.editor_ui.agent_settings.focus.take() else {
        return false;
    };
    let draft = state.editor_ui.settings_input.text().to_owned();
    state.editor_ui.settings_input.set_text("");
    let trimmed = draft.trim();
    match focus {
        SettingsFocus::McpPort => {
            if let Ok(port) = trimmed.parse::<u16>() {
                state.editor_ui.agent_settings.mcp_server.port = port.max(1024);
            }
        }
        SettingsFocus::ImageSearch(field) => {
            commit_image_search(&mut state.editor_ui, scope, field, trimmed);
        }
        SettingsFocus::BuiltinAgent { index, field } => {
            let settings = &mut state.editor_ui.agent_settings;
            if scope.takes_over_browser_entries() {
                settings.take_over_browser_builtin_agent(index);
            }
            let target_id = settings
                .builtin_agents
                .get(index)
                .map(|agent| agent.id.clone());
            let previous_credential_field =
                settings
                    .builtin_agents
                    .get(index)
                    .and_then(|agent| match field {
                        BuiltinAgentField::ApiKey => Some(agent.api_key.clone()),
                        BuiltinAgentField::BaseUrl => Some(agent.base_url.clone()),
                        _ => None,
                    });
            if let Some(agent) = settings.builtin_agents.get_mut(index) {
                write_builtin_field(agent, field, &draft);
                let credential_changed =
                    previous_credential_field
                        .as_deref()
                        .is_some_and(|old| match field {
                            BuiltinAgentField::ApiKey => old != agent.api_key,
                            BuiltinAgentField::BaseUrl => old != agent.base_url,
                            _ => false,
                        });
                if credential_changed {
                    if let Some(id) = target_id.as_deref() {
                        settings.invalidate_builtin_model_catalog_for_agent(id);
                        let _ = settings.begin_builtin_model_catalog_refresh(
                            crate::agent_settings_builtin_models::BuiltinModelCatalogTarget::Agent(
                                id.to_string(),
                            ),
                            now_ms,
                        );
                    }
                }
                // The chat picker lists models per ready agent — a
                // re-keyed / re-modelled agent changes that set.
                state.rebuild_chat_models();
            }
        }
        SettingsFocus::BuiltinAgentDraft(field) => {
            if let Some(agent) = state.editor_ui.agent_settings.builtin_agent_draft.as_mut() {
                write_builtin_field(agent, field, &draft);
            }
            if matches!(
                field,
                BuiltinAgentField::ApiKey | BuiltinAgentField::BaseUrl
            ) {
                let settings = &mut state.editor_ui.agent_settings;
                settings.invalidate_builtin_model_catalog(
                    &crate::agent_settings_builtin_models::BuiltinModelCatalogTarget::Draft,
                );
                // Credentials landing in the add-provider form start the
                // model fetch immediately, so the Model dropdown is
                // already warm when the user reaches it.
                let _ = settings.begin_builtin_model_catalog_refresh_if_due(
                    crate::agent_settings_builtin_models::BuiltinModelCatalogTarget::Draft,
                    now_ms,
                );
            }
        }
        SettingsFocus::ImageGenProfile { index, field } => {
            let settings = &mut state.editor_ui.agent_settings;
            if scope.takes_over_browser_entries() {
                settings.take_over_browser_image_profile(index);
            }
            if let Some(profile) = settings.image_gen_profiles.get_mut(index) {
                match field {
                    ImageGenField::Name => profile.name = trimmed.to_string(),
                    ImageGenField::ApiKey => profile.api_key = trimmed.to_string(),
                    ImageGenField::Model => profile.model = trimmed.to_string(),
                    ImageGenField::BaseUrl => {
                        profile.base_url = (!trimmed.is_empty()).then(|| trimmed.to_string());
                    }
                }
            }
        }
        SettingsFocus::AcpAgent { index, field } => {
            let changed_id = state
                .editor_ui
                .agent_settings
                .acp_agents
                .get_mut(index)
                .and_then(|agent| {
                    let id = agent.id.clone();
                    write_acp_field(agent, field, &draft).then_some(id)
                });
            if let Some(id) = changed_id {
                state
                    .editor_ui
                    .agent_settings
                    .invalidate_acp_agent_connection(&id);
                state.rebuild_chat_models();
            }
        }
        SettingsFocus::AcpAgentDraft(field) => {
            if let Some(agent) = state.editor_ui.agent_settings.acp_agent_draft.as_mut() {
                let _ = write_acp_field(agent, field, &draft);
            }
        }
    }
    true
}

fn commit_image_search(
    ui: &mut EditorUiState,
    scope: SettingsCommitScope,
    field: ImageSearchField,
    trimmed: &str,
) {
    match field {
        ImageSearchField::ClientId => {
            ui.agent_settings.openverse_client_id = trimmed.to_string();
        }
        ImageSearchField::ClientSecret => {
            ui.agent_settings.openverse_client_secret = trimmed.to_string();
        }
    }
    if scope.takes_over_browser_entries() {
        ui.agent_settings.openverse_credential_owner = None;
    }
}

/// Write one built-in-agent field from a committed draft.
///
/// `DisplayName` keeps the previous value on an empty draft (a nameless
/// card would be unclickable); `BaseUrl` falls back to the preset's
/// default and is refused outright on presets that pin it. The widget
/// hit-test already skips the BaseUrl row for those presets
/// (`agent_settings_builtin.rs`), so the guard is belt-and-braces — but
/// only the native twin carried it, and defensive is the right side to
/// unify on.
fn write_builtin_field(
    agent: &mut crate::agent_settings::BuiltinAgentConfig,
    field: BuiltinAgentField,
    draft: &str,
) {
    let trimmed = draft.trim();
    match field {
        BuiltinAgentField::DisplayName => {
            if !trimmed.is_empty() {
                agent.display_name = trimmed.to_string();
            }
        }
        BuiltinAgentField::ApiKey => agent.api_key = trimmed.to_string(),
        BuiltinAgentField::Model => agent.set_models(draft.lines()),
        BuiltinAgentField::BaseUrl => {
            if agent.base_url_editable() {
                agent.base_url = if trimmed.is_empty() {
                    let preset =
                        crate::agent_settings_builtin_presets::builtin_agent_preset(agent.preset);
                    preset
                        .base_url_for_kind(agent.kind)
                        .filter(|base_url| !base_url.is_empty())
                        .or_else(|| {
                            (agent.preset
                                != crate::agent_settings_builtin_presets::BuiltinAgentPresetKey::Custom)
                                .then_some(preset.base_url)
                        })
                        .filter(|base_url| !base_url.is_empty())
                        .unwrap_or_else(|| agent.kind.default_base_url())
                        .to_string()
                } else {
                    trimmed.to_string()
                };
            }
        }
    }
}

/// Write one ACP-agent field from a committed draft and report whether
/// the persisted configuration changed. The caller invalidates all runtime
/// connection state for a changed row. `Args` / `Env` parse the RAW draft
/// (their own splitters handle whitespace), everything else the trimmed form.
fn write_acp_field(
    agent: &mut crate::agent_settings::AcpAgentConfig,
    field: AcpAgentField,
    draft: &str,
) -> bool {
    let trimmed = draft.trim();
    match field {
        AcpAgentField::DisplayName => {
            if !trimmed.is_empty() && agent.display_name != trimmed {
                agent.display_name = trimmed.to_string();
                true
            } else {
                false
            }
        }
        AcpAgentField::Command => {
            let next = trimmed.to_string();
            let changed = agent.command != next;
            agent.command = next;
            changed
        }
        AcpAgentField::Args => {
            let previous = agent.args.clone();
            agent.set_args_text(draft);
            agent.args != previous
        }
        AcpAgentField::Env => {
            let previous = agent.env.clone();
            agent.set_env_text(draft);
            agent.env != previous
        }
        AcpAgentField::Url => {
            let next = (!trimmed.is_empty()).then(|| trimmed.to_string());
            let changed = agent.url != next;
            agent.url = next;
            changed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_settings::{AcpAgentConnectOutcome, AcpAgentConnectPhase, AcpConnectionType};
    use std::collections::BTreeMap;

    #[test]
    fn committing_a_new_builtin_api_key_queues_immediate_model_discovery() {
        let mut state = EditorState::new();
        let id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "Provider",
            "",
            "fallback-model",
            crate::BuiltinAgentKind::OpenAiCompat,
            "https://example.test/v1",
        );
        state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        });
        state.editor_ui.settings_input.set_text("sk-new");

        assert!(commit_settings_focus(
            &mut state,
            SettingsCommitScope::Operator,
            123,
        ));

        let request = state
            .editor_ui
            .agent_settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("key commit should queue discovery without opening the picker");
        assert_eq!(request.target, crate::BuiltinModelCatalogTarget::Agent(id));
        assert_eq!(
            state.editor_ui.agent_settings.builtin_agents[0].api_key,
            "sk-new"
        );
    }

    #[test]
    fn recommitting_the_same_builtin_key_does_not_probe_again() {
        let mut state = EditorState::new();
        state.editor_ui.agent_settings.add_builtin_agent_config(
            "Provider",
            "sk-same",
            "fallback-model",
            crate::BuiltinAgentKind::OpenAiCompat,
            "https://example.test/v1",
        );
        state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        });
        state.editor_ui.settings_input.set_text("sk-same");

        assert!(commit_settings_focus(
            &mut state,
            SettingsCommitScope::Operator,
            123,
        ));
        assert!(state
            .editor_ui
            .agent_settings
            .take_pending_builtin_model_catalog_refresh()
            .is_none());
    }

    #[test]
    fn committing_model_lines_trims_blanks_and_deduplicates_in_order() {
        let mut state = EditorState::new();
        state.editor_ui.agent_settings.add_builtin_agent_config(
            "Provider",
            "sk-test",
            "old-model",
            crate::BuiltinAgentKind::OpenAiCompat,
            "https://example.test/v1",
        );
        state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::Model,
        });
        state
            .editor_ui
            .settings_input
            .set_text(" model-b \n\nmodel-a\nmodel-b\n  ");

        assert!(commit_settings_focus(
            &mut state,
            SettingsCommitScope::Operator,
            123,
        ));

        assert_eq!(
            state.editor_ui.agent_settings.builtin_agents[0].models,
            ["model-b", "model-a"]
        );
    }

    #[test]
    fn clearing_builtin_base_url_restores_the_selected_provider_endpoint() {
        let mut state = EditorState::new();
        state.editor_ui.agent_settings.add_builtin_agent_config(
            "MiniMax",
            "sk-minimax",
            "MiniMax-M3",
            crate::BuiltinAgentKind::OpenAiCompat,
            "https://api.minimaxi.com/v1",
        );
        state.editor_ui.agent_settings.builtin_agents[0].base_url =
            "https://proxy.example/v1".into();
        state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::BaseUrl,
        });
        state.editor_ui.settings_input.set_text("");

        assert!(commit_settings_focus(
            &mut state,
            SettingsCommitScope::Operator,
            123,
        ));

        assert_eq!(
            state.editor_ui.agent_settings.builtin_agents[0].base_url,
            "https://api.minimaxi.com/v1"
        );
        let request = state
            .editor_ui
            .agent_settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("restoring the provider endpoint queues discovery");
        let config = state
            .editor_ui
            .agent_settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("request resolves the restored endpoint");
        assert_eq!(config.base_url, "https://api.minimaxi.com/v1");
    }

    #[test]
    fn committed_acp_edit_clears_verified_connection_and_detail() {
        let mut state = EditorState::new();
        let id = state.editor_ui.agent_settings.add_acp_agent_config(
            "Local Agent",
            AcpConnectionType::Local,
            "old-agent",
            Vec::new(),
            BTreeMap::new(),
            None,
            true,
        );
        state.editor_ui.agent_settings.begin_acp_agent_connect(0);
        state
            .editor_ui
            .agent_settings
            .apply_acp_agent_connect_outcome(
                &id,
                AcpAgentConnectOutcome {
                    connected: true,
                    info: Some("Local Agent 1.0".into()),
                    ..AcpAgentConnectOutcome::default()
                },
            );
        assert!(state
            .editor_ui
            .agent_settings
            .acp_agent_verified_connected(&id));

        state.editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
            index: 0,
            field: AcpAgentField::Command,
        });
        state.editor_ui.settings_input.set_text("new-agent");

        assert!(commit_settings_focus(
            &mut state,
            SettingsCommitScope::Operator,
            1,
        ));

        let settings = &state.editor_ui.agent_settings;
        assert_eq!(settings.acp_agents[0].command, "new-agent");
        assert!(!settings.acp_agents[0].connected);
        assert!(!settings.acp_agent_verified_connected(&id));
        assert_eq!(settings.pending_acp_agent_connect, None);
        assert_eq!(
            settings.acp_agent_connection_for(&id).phase,
            AcpAgentConnectPhase::Idle
        );
        assert_eq!(settings.acp_agent_connection_for(&id).info, None);
    }

    #[test]
    fn committing_draft_credentials_queues_draft_model_discovery() {
        let mut state = EditorState::new();
        state.editor_ui.agent_settings.begin_builtin_agent_draft();
        state.editor_ui.agent_settings.focus =
            Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
        state.editor_ui.settings_input.set_text("sk-new");

        assert!(commit_settings_focus(
            &mut state,
            SettingsCommitScope::Operator,
            7,
        ));

        let request = state
            .editor_ui
            .agent_settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("a credentialed draft fetches models without opening the menu");
        assert_eq!(request.target, crate::BuiltinModelCatalogTarget::Draft);
    }

    #[test]
    fn clearing_draft_credentials_does_not_probe() {
        let mut state = EditorState::new();
        state.editor_ui.agent_settings.begin_builtin_agent_draft();
        state.editor_ui.agent_settings.focus =
            Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
        state.editor_ui.settings_input.set_text("");

        assert!(commit_settings_focus(
            &mut state,
            SettingsCommitScope::Operator,
            8,
        ));

        assert_eq!(
            state
                .editor_ui
                .agent_settings
                .take_pending_builtin_model_catalog_refresh(),
            None,
            "discovery needs a credential; an empty key falls back to typing"
        );
    }
}
