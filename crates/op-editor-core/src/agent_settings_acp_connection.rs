//! Runtime ACP-agent connect lifecycle state.
//!
//! The persisted ACP configuration only says how to reach an agent
//! (command/args/env or URL). Pressing Connect must run a real host
//! probe and only then mark the agent connected. This module carries
//! the wasm-clean request seam and result state; desktop/web hosts do
//! the transport-specific probe work.

use crate::agent_settings::AgentSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AcpAgentConnectPhase {
    #[default]
    Idle,
    Probing,
    Connected,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AcpAgentConnection {
    pub phase: AcpAgentConnectPhase,
    pub info: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AcpAgentConnectOutcome {
    pub connected: bool,
    pub info: Option<String>,
    pub error: Option<String>,
}

impl AgentSettings {
    pub fn acp_agent_connection_for(&self, id: &str) -> AcpAgentConnection {
        self.acp_agent_connection
            .get(id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn acp_agent_probe_in_flight(&self, id: &str) -> bool {
        self.acp_agent_connection_for(id).phase == AcpAgentConnectPhase::Probing
    }

    pub fn acp_agent_verified_connected(&self, id: &str) -> bool {
        self.acp_agents
            .iter()
            .any(|agent| agent.id == id && agent.connected)
            && self.acp_agent_connection_for(id).phase == AcpAgentConnectPhase::Connected
    }

    pub fn acp_agent_verified_connected_at(&self, index: usize) -> bool {
        self.acp_agents
            .get(index)
            .is_some_and(|agent| self.acp_agent_verified_connected(&agent.id))
    }

    pub fn begin_acp_agent_connect(&mut self, index: usize) -> Option<String> {
        let id = self.acp_agents.get(index)?.id.clone();
        if self.acp_agent_probe_in_flight(&id) {
            return None;
        }
        let agent = self.acp_agents.get_mut(index)?;
        if !agent.ready() {
            return None;
        }
        agent.connected = false;
        self.acp_agent_connection.insert(
            id.clone(),
            AcpAgentConnection {
                phase: AcpAgentConnectPhase::Probing,
                ..AcpAgentConnection::default()
            },
        );
        self.pending_acp_agent_connect = Some(id.clone());
        Some(id)
    }

    pub fn disconnect_acp_agent(&mut self, index: usize) -> Option<String> {
        let agent = self.acp_agents.get_mut(index)?;
        let id = agent.id.clone();
        agent.connected = false;
        self.acp_agent_connection.remove(&id);
        if self.pending_acp_agent_connect.as_deref() == Some(id.as_str()) {
            self.pending_acp_agent_connect = None;
        }
        Some(id)
    }

    pub fn apply_acp_agent_connect_outcome(
        &mut self,
        id: &str,
        outcome: AcpAgentConnectOutcome,
    ) -> bool {
        let Some(agent) = self.acp_agents.iter_mut().find(|agent| agent.id == id) else {
            if self.pending_acp_agent_connect.as_deref() == Some(id) {
                self.pending_acp_agent_connect = None;
            }
            return false;
        };
        agent.connected = outcome.connected;
        self.acp_agent_connection.insert(
            id.to_string(),
            AcpAgentConnection {
                phase: if outcome.connected {
                    AcpAgentConnectPhase::Connected
                } else {
                    AcpAgentConnectPhase::Error
                },
                info: outcome.info,
                error: outcome.error,
            },
        );
        if self.pending_acp_agent_connect.as_deref() == Some(id) {
            self.pending_acp_agent_connect = None;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_settings::AcpConnectionType;
    use std::collections::BTreeMap;

    fn configured_settings() -> AgentSettings {
        let mut settings = AgentSettings::default();
        settings.add_acp_agent_config(
            "Claude Code",
            AcpConnectionType::Local,
            "claude",
            Vec::new(),
            BTreeMap::new(),
            None,
            true,
        );
        settings
    }

    #[test]
    fn begin_acp_agent_connect_raises_request_without_marking_connected() {
        let mut settings = configured_settings();

        let id = settings
            .begin_acp_agent_connect(0)
            .expect("configured agent can start connecting");

        assert_eq!(id, "acp-1");
        assert_eq!(settings.pending_acp_agent_connect.as_deref(), Some("acp-1"));
        assert!(!settings.acp_agents[0].connected);
        assert_eq!(
            settings.acp_agent_connection_for("acp-1").phase,
            AcpAgentConnectPhase::Probing
        );
    }

    #[test]
    fn failed_acp_agent_connect_keeps_agent_disconnected() {
        let mut settings = configured_settings();
        settings.begin_acp_agent_connect(0);

        assert!(settings.apply_acp_agent_connect_outcome(
            "acp-1",
            AcpAgentConnectOutcome {
                connected: false,
                error: Some("ACP initialize failed".into()),
                ..AcpAgentConnectOutcome::default()
            },
        ));

        assert!(!settings.acp_agents[0].connected);
        assert_eq!(settings.pending_acp_agent_connect, None);
        let conn = settings.acp_agent_connection_for("acp-1");
        assert_eq!(conn.phase, AcpAgentConnectPhase::Error);
        assert_eq!(conn.error.as_deref(), Some("ACP initialize failed"));
    }

    #[test]
    fn successful_acp_agent_connect_marks_agent_connected() {
        let mut settings = configured_settings();
        settings.begin_acp_agent_connect(0);

        assert!(settings.apply_acp_agent_connect_outcome(
            "acp-1",
            AcpAgentConnectOutcome {
                connected: true,
                info: Some("Claude Code".into()),
                ..AcpAgentConnectOutcome::default()
            },
        ));

        assert!(settings.acp_agents[0].connected);
        assert_eq!(settings.pending_acp_agent_connect, None);
        let conn = settings.acp_agent_connection_for("acp-1");
        assert_eq!(conn.phase, AcpAgentConnectPhase::Connected);
        assert_eq!(conn.info.as_deref(), Some("Claude Code"));
    }

    #[test]
    fn disconnect_acp_agent_clears_runtime_connection_state() {
        let mut settings = configured_settings();
        settings.apply_acp_agent_connect_outcome(
            "acp-1",
            AcpAgentConnectOutcome {
                connected: true,
                info: Some("Claude Code".into()),
                ..AcpAgentConnectOutcome::default()
            },
        );

        assert_eq!(settings.disconnect_acp_agent(0).as_deref(), Some("acp-1"));

        assert!(!settings.acp_agents[0].connected);
        assert_eq!(
            settings.acp_agent_connection_for("acp-1"),
            AcpAgentConnection::default()
        );
    }

    #[test]
    fn verified_connection_requires_successful_probe_state() {
        let mut settings = configured_settings();
        settings.acp_agents[0].connected = true;

        assert!(
            !settings.acp_agent_verified_connected("acp-1"),
            "a stale persisted connected flag is not a real ACP connection"
        );

        settings.acp_agent_connection.insert(
            "acp-1".into(),
            AcpAgentConnection {
                phase: AcpAgentConnectPhase::Connected,
                info: Some("Claude Code".into()),
                error: None,
            },
        );

        assert!(settings.acp_agent_verified_connected("acp-1"));
        assert!(settings.acp_agent_verified_connected_at(0));
    }
}
