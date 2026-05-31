use crate::agent_settings::AgentSettings;

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
