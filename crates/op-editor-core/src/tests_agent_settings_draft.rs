use crate::agent_settings::{AgentSettings, BuiltinAgentKind};
use crate::agent_settings_builtin_presets::BuiltinAgentPresetKey;

#[test]
fn duplicate_builtin_agent_config_reuses_existing_provider() {
    let mut s = AgentSettings::default();

    let first = s.add_builtin_agent_with_defaults("MINIMAX", "sk-test", "MiniMax-M2.7");
    let second = s.add_builtin_agent_with_defaults("MINIMAX", "sk-test", "MiniMax-M2.7");

    assert_eq!(second, first);
    assert_eq!(s.builtin_agents.len(), 1);
    assert_eq!(s.next_builtin_agent_id, 2);
}

#[test]
fn builtin_agent_draft_does_not_persist_until_save() {
    let mut s = AgentSettings::default();

    s.begin_builtin_agent_draft();

    assert!(s.builtin_agent_draft.is_some());
    assert!(s.builtin_agents.is_empty());
    assert!(s.save_builtin_agent_draft().is_none());
    s.builtin_agent_draft.as_mut().unwrap().api_key = "sk-test".into();

    let id = s.save_builtin_agent_draft();

    assert_eq!(id.as_deref(), Some("builtin-1"));
    assert_eq!(s.builtin_agents.len(), 1);
    assert!(s.builtin_agent_draft.is_none());
    assert_eq!(s.builtin_agents[0].api_key, "sk-test");
}

#[test]
fn builtin_agent_draft_starts_from_anthropic_preset() {
    let mut s = AgentSettings::default();

    s.begin_builtin_agent_draft();

    let draft = s.builtin_agent_draft.as_ref().expect("draft exists");
    assert_eq!(draft.display_name, "Anthropic");
    assert_eq!(draft.base_url, "https://api.anthropic.com");
}

#[test]
fn builtin_agent_draft_can_select_ts_builtin_provider_preset() {
    let mut s = AgentSettings::default();

    s.begin_builtin_agent_draft();
    s.builtin_agent_draft.as_mut().unwrap().api_key = "sk-test".into();
    s.set_builtin_agent_draft_preset(BuiltinAgentPresetKey::MiniMax);

    let draft = s.builtin_agent_draft.as_ref().expect("draft exists");
    assert_eq!(draft.preset, BuiltinAgentPresetKey::MiniMax);
    assert_eq!(draft.display_name, "MiniMax");
    assert_eq!(draft.kind, BuiltinAgentKind::Anthropic);
    assert_eq!(draft.base_url, "https://api.minimaxi.com/anthropic");
    assert_eq!(draft.model, "MiniMax-M2.7");
    assert_eq!(draft.api_key, "sk-test");
}

#[test]
fn builtin_agent_format_toggle_uses_selected_provider_alt_base_url() {
    let mut s = AgentSettings::default();

    s.begin_builtin_agent_draft();
    s.set_builtin_agent_draft_preset(BuiltinAgentPresetKey::MiniMax);
    s.builtin_agent_draft
        .as_mut()
        .unwrap()
        .toggle_kind_for_preset();

    let draft = s.builtin_agent_draft.as_ref().expect("draft exists");
    assert_eq!(draft.kind, BuiltinAgentKind::OpenAiCompat);
    assert_eq!(draft.base_url, "https://api.minimaxi.com/v1");
}

#[test]
fn acp_agent_draft_can_cancel_or_save() {
    let mut s = AgentSettings::default();

    s.begin_acp_agent_draft();
    assert!(s.acp_agent_draft.is_some());
    assert!(s.acp_agents.is_empty());

    s.cancel_acp_agent_draft();
    assert!(s.acp_agent_draft.is_none());
    assert!(s.acp_agents.is_empty());

    s.begin_acp_agent_draft();
    s.acp_agent_draft.as_mut().unwrap().command = "op-agent".into();
    let id = s.save_acp_agent_draft();

    assert_eq!(id.as_deref(), Some("acp-1"));
    assert_eq!(s.acp_agents.len(), 1);
    assert!(s.acp_agent_draft.is_none());
    assert_eq!(s.acp_agents[0].command, "op-agent");
}

#[test]
fn acp_agent_args_text_uses_comma_separated_values_like_ts() {
    let mut s = AgentSettings::default();
    let id = s.add_acp_agent_config(
        "Local ACP",
        crate::AcpConnectionType::Local,
        "op-agent",
        vec!["--stdio".into(), "--workspace /tmp".into()],
        Default::default(),
        None,
        true,
    );
    let agent = s
        .acp_agents
        .iter_mut()
        .find(|agent| agent.id == id)
        .expect("agent exists");

    assert_eq!(agent.args_text(), "--stdio, --workspace /tmp");

    agent.set_args_text(" --stdio, --workspace /tmp, , --verbose ");

    assert_eq!(agent.args, vec!["--stdio", "--workspace /tmp", "--verbose"]);
}
