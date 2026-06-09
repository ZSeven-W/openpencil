use crate::agent_settings::{
    AcpConnectionType, AgentSettings, AgentSettingsTab, BuiltinAgentConfig, BuiltinAgentKind,
    ImageGenProvider, ImageTestStatus, McpCli,
};
use crate::agent_settings_builtin_presets::BuiltinAgentPresetKey;

#[test]
fn default_settings_are_quiescent() {
    let s = AgentSettings::default();
    assert_eq!(s.tab, AgentSettingsTab::Agents);
    assert_eq!(s.connected, [false; 5]);
    assert!(s.builtin_agents.is_empty());
    assert!(s.builtin_agent_draft.is_none());
    assert!(s.acp_agent_draft.is_none());
    assert!(s.image_gen_profiles.is_empty());
    assert!(s.active_image_gen_profile_id.is_none());
    assert!(s.image_gen_provider_menu_open.is_none());
    assert!(s.hover_image_gen_provider_option.is_none());
    assert!(!s.images_advanced_open);
    assert!(s.openverse_client_id.is_empty());
    assert!(s.openverse_client_secret.is_empty());
    assert_eq!(s.images_search_test_status, ImageTestStatus::Idle);
    assert_eq!(s.mcp_server.port, 3100);
    assert!(s.auto_update_enabled);
    assert!(s.focus.is_none());
    assert_eq!(s.hover_provider, usize::MAX);
    assert_eq!(s.hover_builtin_agent, usize::MAX);
    assert_eq!(s.hover_acp_agent, usize::MAX);
}

#[test]
fn tab_and_cli_arrays_cover_all_variants() {
    assert_eq!(AgentSettingsTab::ALL.len(), 4);
    assert_eq!(McpCli::ALL.len(), 6);
}

#[test]
fn settings_tab_fallback_labels_match_ts_order() {
    let labels: Vec<_> = AgentSettingsTab::ALL
        .iter()
        .map(|tab| tab.label())
        .collect();

    assert_eq!(labels, vec!["Agents", "MCP", "Images", "System"]);
}

#[test]
fn image_generation_profiles_follow_ts_lifecycle() {
    let mut s = AgentSettings::default();

    let first = s.add_image_gen_profile();
    assert_eq!(s.image_gen_profiles.len(), 1);
    assert_eq!(
        s.active_image_gen_profile_id.as_deref(),
        Some(first.as_str())
    );
    assert_eq!(s.image_gen_profiles[0].name, "Config 1");
    assert_eq!(s.image_gen_profiles[0].provider, ImageGenProvider::OpenAi);
    assert_eq!(s.image_gen_profiles[0].test_status, ImageTestStatus::Idle);

    let second = s.add_image_gen_profile();
    assert_eq!(s.image_gen_profiles.len(), 2);
    assert_eq!(
        s.active_image_gen_profile_id.as_deref(),
        Some(first.as_str())
    );

    assert!(s.set_active_image_gen_profile(&second));
    assert_eq!(
        s.active_image_gen_profile_id.as_deref(),
        Some(second.as_str())
    );

    assert!(s.remove_image_gen_profile(&second));
    assert_eq!(
        s.active_image_gen_profile_id.as_deref(),
        Some(first.as_str())
    );

    assert!(s.remove_image_gen_profile(&first));
    assert!(s.image_gen_profiles.is_empty());
    assert!(s.active_image_gen_profile_id.is_none());
}

#[test]
fn add_builtin_agent_prefills_ts_provider_presets_first() {
    let mut s = AgentSettings::default();

    for _ in 0..4 {
        s.add_builtin_agent();
    }

    let summary: Vec<_> = s
        .builtin_agents
        .iter()
        .map(|agent| {
            (
                agent.display_name.as_str(),
                agent.kind,
                agent.model.as_str(),
                agent.base_url.as_str(),
                agent.api_key.as_str(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "Anthropic",
                BuiltinAgentKind::Anthropic,
                "claude-sonnet-4-6-20250916",
                "https://api.anthropic.com",
                "",
            ),
            (
                "OpenAI",
                BuiltinAgentKind::OpenAiCompat,
                "gpt-5.4",
                "https://api.openai.com/v1",
                "",
            ),
            (
                "OpenRouter",
                BuiltinAgentKind::OpenAiCompat,
                "anthropic/claude-sonnet-4.6",
                "https://openrouter.ai/api/v1",
                "",
            ),
            (
                "DeepSeek",
                BuiltinAgentKind::OpenAiCompat,
                "deepseek-v4-pro",
                "https://api.deepseek.com/v1",
                "",
            ),
        ]
    );
}

#[test]
fn ark_coding_builtin_agent_infers_ark_coding_preset() {
    let mut s = AgentSettings::default();

    let id = s.add_builtin_agent_config(
        "方舟CP",
        "sk-test",
        "ark-code-latest",
        BuiltinAgentKind::Anthropic,
        "https://ark.cn-beijing.volces.com/api/coding",
    );

    let agent = s
        .builtin_agents
        .iter()
        .find(|agent| agent.id == id)
        .expect("agent added");
    assert_eq!(agent.preset, BuiltinAgentPresetKey::ArkCoding);
}

#[test]
fn pure_builtin_presets_do_not_toggle_api_format() {
    let mut anthropic = BuiltinAgentConfig {
        id: "anthropic".into(),
        preset: BuiltinAgentPresetKey::Anthropic,
        display_name: "Anthropic".into(),
        kind: BuiltinAgentKind::Anthropic,
        api_key: String::new(),
        model: "claude-sonnet-4-6-20250916".into(),
        base_url: "https://api.anthropic.com".into(),
        enabled: true,
    };
    anthropic.toggle_kind_for_preset();
    assert_eq!(anthropic.kind, BuiltinAgentKind::Anthropic);
    assert_eq!(anthropic.base_url, "https://api.anthropic.com");

    let mut openai = BuiltinAgentConfig {
        id: "openai".into(),
        preset: BuiltinAgentPresetKey::OpenAi,
        display_name: "OpenAI".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key: String::new(),
        model: "gpt-5.4".into(),
        base_url: "https://api.openai.com/v1".into(),
        enabled: true,
    };
    openai.toggle_kind_for_preset();
    assert_eq!(openai.kind, BuiltinAgentKind::OpenAiCompat);
    assert_eq!(openai.base_url, "https://api.openai.com/v1");
}

#[test]
fn pure_builtin_presets_keep_base_url_read_only_like_ts() {
    let mut s = AgentSettings::default();

    for _ in 0..3 {
        s.add_builtin_agent();
    }

    assert!(!s.builtin_agents[0].base_url_editable());
    assert!(!s.builtin_agents[1].base_url_editable());
    assert!(s.builtin_agents[2].base_url_editable());
}

#[test]
fn add_acp_agent_assigns_id_and_defaults_to_local_config() {
    let mut s = AgentSettings::default();

    let first = s.add_acp_agent();
    let second = s.add_acp_agent();

    assert_eq!(first, "acp-1");
    assert_eq!(second, "acp-2");
    assert_eq!(s.acp_agents.len(), 2);
    assert_eq!(s.next_acp_agent_id, 3);
    assert_eq!(s.acp_agents[0].display_name, "ACP Agent 1");
    assert_eq!(s.acp_agents[0].connection_type, AcpConnectionType::Local);
    assert!(s.acp_agents[0].command.is_empty());
    assert!(s.acp_agents[0].args.is_empty());
    assert!(s.acp_agents[0].env.is_empty());
    assert!(s.acp_agents[0].url.is_none());
    assert!(s.acp_agents[0].enabled);
    assert!(!s.acp_agents[0].connected);
}
