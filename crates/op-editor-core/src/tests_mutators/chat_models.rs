//! Chat-model catalog + agent-sync mutator tests.

use crate::test_support::sample;

#[test]
fn select_chat_model_picks_model_and_syncs_agent() {
    let mut s = sample();
    s.chat.available_models = vec![
        crate::ModelEntry::new(crate::AgentProvider::ClaudeCode, "claude", "Claude"),
        crate::ModelEntry::new(
            crate::AgentProvider::Antigravity,
            "antigravity",
            "Antigravity",
        ),
    ];
    s.editor_ui.chat_model_picker.open = true;
    s.select_chat_model(1);
    assert_eq!(s.chat.selected_model, 1);
    // Antigravity is index 4 in AgentProvider::ALL.
    assert_eq!(s.editor_ui.chat_selected_agent, 4);
    // Picker closes on selection.
    assert!(!s.editor_ui.chat_model_picker.open);
}

#[test]
fn select_chat_model_bad_index_still_closes_picker() {
    let mut s = sample();
    s.chat.available_models = vec![crate::ModelEntry::new(
        crate::AgentProvider::ClaudeCode,
        "c",
        "C",
    )];
    s.editor_ui.chat_model_picker.open = true;
    s.select_chat_model(9);
    // Out-of-range index ignored — selected_model unchanged.
    assert_eq!(s.chat.selected_model, 0);
    assert!(!s.editor_ui.chat_model_picker.open);
}

#[test]
fn cycle_agent_team_size_mirrors_into_preferred_agent_team_size() {
    let mut s = sample();
    assert_eq!(s.editor_ui.preferred_agent_team_size, 1);

    s.cycle_agent_team_size();

    assert_eq!(s.chat.agent_team_size, 2);
    assert_eq!(
        s.editor_ui.preferred_agent_team_size, 2,
        "the sticky preference must track the picker's live value"
    );
}

#[test]
fn set_agent_team_size_mirrors_into_preferred_agent_team_size() {
    let mut s = sample();

    s.set_agent_team_size(5);

    assert_eq!(s.chat.agent_team_size, 5);
    assert_eq!(s.editor_ui.preferred_agent_team_size, 5);
}

#[test]
fn rebuild_chat_models_syncs_agent_to_selected_model_provider() {
    let mut s = sample();
    s.chat.discovered_models = vec![crate::ModelEntry::new(
        crate::AgentProvider::CodexCli,
        "gpt-5.5",
        "GPT-5.5",
    )];
    s.editor_ui.agent_settings.connected = [false, true, false, false, false, false, false];
    s.editor_ui.agent_settings.provider_connection[1].phase =
        crate::agent_settings::ProviderConnectPhase::Connected;
    s.editor_ui.chat_selected_agent = 0;

    s.rebuild_chat_models();

    assert_eq!(s.chat.selected_model, 0);
    assert_eq!(s.editor_ui.chat_selected_agent, 1);
}

#[test]
fn rebuild_chat_models_does_not_invent_cli_models_without_discovery() {
    let mut s = sample();
    s.chat.discovered_models.clear();
    s.editor_ui.agent_settings.connected = [false, true, false, false, false, false, false];

    s.rebuild_chat_models();

    assert!(
        s.chat.available_models.is_empty(),
        "CLI providers must only become selectable after a probe returns real models"
    );
}

#[test]
fn rebuild_chat_models_includes_ready_builtin_agents() {
    let mut s = sample();
    let id = s.editor_ui.agent_settings.add_builtin_agent_with_defaults(
        "Built-in Claude",
        "sk-test",
        "claude-sonnet-4-5",
    );

    s.rebuild_chat_models();

    let entry = s
        .chat
        .available_models
        .iter()
        .find(|m| m.builtin_provider_id.as_deref() == Some(id.as_str()))
        .expect("ready built-in agent should appear in model picker");
    assert_eq!(entry.display_name, "claude-sonnet-4-5");
    assert!(entry.value.starts_with("builtin:"));
}

#[test]
fn rebuild_chat_models_retains_builtin_agent_display_name_as_group_label() {
    let mut s = sample();
    let id = s.editor_ui.agent_settings.add_builtin_agent_with_defaults(
        "MiniMax",
        "sk-test",
        "MiniMax-M2.7",
    );

    s.rebuild_chat_models();

    let entry = s
        .chat
        .available_models
        .iter()
        .find(|m| m.builtin_provider_id.as_deref() == Some(id.as_str()))
        .expect("ready built-in agent should appear in model picker");
    assert_eq!(entry.display_name, "MiniMax-M2.7");
    assert_eq!(
        entry.builtin_provider_display_name.as_deref(),
        Some("MiniMax")
    );
}

#[test]
fn rebuild_chat_models_flattens_saved_models_and_ignores_runtime_options() {
    let mut state = sample();
    let id = state.editor_ui.agent_settings.add_builtin_agent_config(
        "Provider",
        "sk-test",
        "configured",
        crate::BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    let target = crate::BuiltinModelCatalogTarget::Agent(id.clone());
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter_mut()
        .find(|agent| agent.id == id)
        .expect("configured agent")
        .set_models(["configured", "saved-b"]);
    let request = state
        .editor_ui
        .agent_settings
        .begin_builtin_model_catalog_refresh(target, 1)
        .expect("request");
    state
        .editor_ui
        .agent_settings
        .take_pending_builtin_model_catalog_refresh();
    let expected = state
        .editor_ui
        .agent_settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("snapshot");
    assert!(state
        .editor_ui
        .agent_settings
        .apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            crate::BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![
                    crate::BuiltinModelOption::new("live-b", "Live B"),
                    crate::BuiltinModelOption::new("configured", "duplicate configured"),
                    crate::BuiltinModelOption::new("live-c", "Live C"),
                ],
            },
        ));

    state.rebuild_chat_models();

    let entries = state
        .chat
        .available_models
        .iter()
        .filter(|entry| entry.builtin_provider_id.as_deref() == Some(id.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        entries
            .iter()
            .filter_map(|entry| entry.builtin_model_id())
            .collect::<Vec<_>>(),
        vec!["configured", "saved-b"]
    );
    assert_eq!(entries[1].display_name, "saved-b");
    assert!(entries
        .iter()
        .all(|entry| { entry.builtin_provider_display_name.as_deref() == Some("Provider") }));
}

#[test]
fn discovered_model_is_not_a_chat_choice_until_it_is_saved() {
    let mut state = sample();
    let id = state.editor_ui.agent_settings.add_builtin_agent_config(
        "Provider",
        "sk-test",
        "",
        crate::BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    let request = state
        .editor_ui
        .agent_settings
        .begin_builtin_model_catalog_refresh(crate::BuiltinModelCatalogTarget::Agent(id.clone()), 1)
        .expect("discovery request");
    state
        .editor_ui
        .agent_settings
        .take_pending_builtin_model_catalog_refresh();
    let expected = state
        .editor_ui
        .agent_settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("snapshot");
    assert!(state
        .editor_ui
        .agent_settings
        .apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            crate::BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![crate::BuiltinModelOption::new("first-live", "First Live")],
            },
        ));

    state.rebuild_chat_models();

    assert!(state
        .chat
        .available_models
        .iter()
        .all(|entry| entry.builtin_provider_id.as_deref() != Some(id.as_str())));
}

#[test]
fn selecting_saved_builtin_is_tab_local_and_does_not_mutate_provider_models() {
    let mut state = sample();
    let first = state.editor_ui.agent_settings.add_builtin_agent_config(
        "First",
        "sk-first",
        "first-default",
        crate::BuiltinAgentKind::OpenAiCompat,
        "https://first.example/v1",
    );
    let second = state.editor_ui.agent_settings.add_builtin_agent_config(
        "Second",
        "sk-second",
        "second-default",
        crate::BuiltinAgentKind::OpenAiCompat,
        "https://second.example/v1",
    );
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter_mut()
        .find(|agent| agent.id == first)
        .expect("first provider")
        .set_models(["first-default", "second-saved"]);
    state.rebuild_chat_models();
    let selected = state
        .chat
        .available_models
        .iter()
        .position(|entry| {
            entry.builtin_provider_id.as_deref() == Some(first.as_str())
                && entry.builtin_model_id() == Some("second-saved")
        })
        .expect("second saved row");

    state.select_chat_model(selected);

    assert_eq!(
        state
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .find(|agent| agent.id == first)
            .map(|agent| agent.models.clone()),
        Some(vec![
            "first-default".to_string(),
            "second-saved".to_string()
        ])
    );
    assert_eq!(
        state
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .find(|agent| agent.id == second)
            .map(|agent| agent.models.clone()),
        Some(vec!["second-default".to_string()])
    );
    state.rebuild_chat_models();
    assert_eq!(
        state
            .chat
            .selected_model_entry()
            .and_then(|entry| entry.builtin_model_id()),
        Some("second-saved")
    );
}

#[test]
fn identical_model_ids_from_distinct_builtins_keep_distinct_identity() {
    let mut state = sample();
    let first = state.editor_ui.agent_settings.add_builtin_agent_config(
        "First",
        "sk-first",
        "shared",
        crate::BuiltinAgentKind::OpenAiCompat,
        "https://first.example/v1",
    );
    let second = state.editor_ui.agent_settings.add_builtin_agent_config(
        "Second",
        "sk-second",
        "shared",
        crate::BuiltinAgentKind::OpenAiCompat,
        "https://second.example/v1",
    );

    state.rebuild_chat_models();

    let shared = state
        .chat
        .available_models
        .iter()
        .filter(|entry| entry.builtin_model_id() == Some("shared"))
        .collect::<Vec<_>>();
    assert_eq!(shared.len(), 2);
    assert_eq!(
        shared
            .iter()
            .filter_map(|entry| entry.builtin_provider_id.as_deref())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([first.as_str(), second.as_str()])
    );
}

#[test]
fn rebuild_chat_models_includes_connected_acp_agents() {
    let mut s = sample();
    let id = s.editor_ui.agent_settings.add_acp_agent_config(
        "Local ACP",
        crate::AcpConnectionType::Local,
        "op-agent",
        Vec::new(),
        std::collections::BTreeMap::new(),
        None,
        true,
    );
    s.editor_ui.agent_settings.apply_acp_agent_connect_outcome(
        &id,
        crate::AcpAgentConnectOutcome {
            connected: true,
            info: Some("Local ACP".into()),
            error: None,
        },
    );

    s.rebuild_chat_models();

    let entry = s
        .chat
        .available_models
        .iter()
        .find(|m| m.value == format!("acp:{id}"))
        .expect("connected ACP agent should appear in model picker");
    assert_eq!(entry.display_name, "Local ACP");
}

#[test]
fn rebuild_chat_models_excludes_stale_acp_connected_flag_without_probe() {
    let mut s = sample();
    let id = s.editor_ui.agent_settings.add_acp_agent_config(
        "Local ACP",
        crate::AcpConnectionType::Local,
        "op-agent",
        Vec::new(),
        std::collections::BTreeMap::new(),
        None,
        true,
    );
    s.editor_ui.agent_settings.acp_agents[0].connected = true;

    s.rebuild_chat_models();

    assert!(
        s.chat
            .available_models
            .iter()
            .all(|m| m.value != format!("acp:{id}")),
        "ACP agent without a successful probe must not appear as a selectable model"
    );
}

#[test]
fn select_chat_model_keeps_agent_sync_unchanged_for_acp_models() {
    let mut s = sample();
    s.chat.available_models = vec![
        crate::ModelEntry::new(crate::AgentProvider::ClaudeCode, "claude", "Claude"),
        crate::ModelEntry::new(crate::AgentProvider::CodexCli, "acp:acp-1", "Local ACP"),
    ];
    s.editor_ui.chat_selected_agent = 0;
    s.editor_ui.chat_model_picker.open = true;

    s.select_chat_model(1);

    assert_eq!(s.chat.selected_model, 1);
    assert_eq!(s.editor_ui.chat_selected_agent, 0);
    assert!(!s.editor_ui.chat_model_picker.open);
}

// --- Layer collapse (Gap 3) -----------------------------------------
