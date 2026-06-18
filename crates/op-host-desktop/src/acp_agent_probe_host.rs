//! Host pump for connect-time ACP agent probes.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_editor_core::agent_settings::{
    AcpAgentConfig as CoreAcpAgentConfig, AcpAgentConnectOutcome, AcpConnectionType,
};

use crate::DesktopApp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpAgentProbeOutcome {
    pub connected: bool,
    pub info: Option<String>,
    pub error: Option<String>,
}

impl AcpAgentProbeOutcome {
    fn connected(info: String) -> Self {
        Self {
            connected: true,
            info: Some(info),
            error: None,
        }
    }

    fn failed(error: impl Into<String>) -> Self {
        Self {
            connected: false,
            info: None,
            error: Some(error.into()),
        }
    }
}

pub struct AcpAgentConnectJob {
    id: String,
    rx: Option<Receiver<AcpAgentProbeOutcome>>,
}

impl AcpAgentConnectJob {
    pub fn spawn(agent: CoreAcpAgentConfig) -> Self {
        let id = agent.id.clone();
        let config = acp_config_for_probe(&agent);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let outcome = probe_acp_agent_config(config);
            let _ = tx.send(outcome);
        });
        Self { id, rx: Some(rx) }
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    #[cfg(test)]
    pub(crate) fn pending_for_test(
        id: impl Into<String>,
    ) -> (Self, mpsc::Sender<AcpAgentProbeOutcome>) {
        let (tx, rx) = mpsc::channel();
        (
            Self {
                id: id.into(),
                rx: Some(rx),
            },
            tx,
        )
    }

    fn poll(&mut self) -> Option<AcpAgentProbeOutcome> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(outcome) => {
                self.rx = None;
                Some(outcome)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                Some(AcpAgentProbeOutcome::failed(
                    "ACP probe worker disconnected",
                ))
            }
        }
    }
}

pub(crate) fn probe_acp_agent_config(config: op_acp::AcpAgentConfig) -> AcpAgentProbeOutcome {
    crate::chat_runtime::shared_runtime().block_on(async move {
        match op_acp::connect_acp_agent(&config).await {
            Ok(conn) => {
                let info = format_acp_agent_info(conn.agent_info(), &config.display_name);
                AcpAgentProbeOutcome::connected(info)
            }
            Err(err) => AcpAgentProbeOutcome::failed(err.to_string()),
        }
    })
}

pub(crate) fn acp_config_for_probe(agent: &CoreAcpAgentConfig) -> op_acp::AcpAgentConfig {
    op_acp::AcpAgentConfig {
        id: agent.id.clone(),
        display_name: agent.display_name.clone(),
        connection_type: match agent.connection_type {
            AcpConnectionType::Local => op_acp::ConnectionType::Local,
            AcpConnectionType::Remote => op_acp::ConnectionType::Remote,
        },
        command: match agent.connection_type {
            AcpConnectionType::Local => Some(agent.command.clone()),
            AcpConnectionType::Remote => None,
        },
        args: agent.args.clone(),
        env: agent.env.clone(),
        url: agent.url.clone(),
        enabled: agent.enabled,
    }
}

pub(crate) fn format_acp_agent_info(info: &op_acp::AcpAgentInfo, fallback: &str) -> String {
    let name = if info.name.trim().is_empty() {
        fallback.trim()
    } else {
        info.name.trim()
    };
    match info
        .version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(version) => format!("{name} {version}"),
        None => name.to_string(),
    }
}

impl DesktopApp {
    pub(crate) fn drain_acp_agent_connect(&mut self) -> bool {
        let mut changed = self.poll_acp_agent_connect_job();
        if self
            .acp_agent_connect_job
            .as_ref()
            .is_some_and(AcpAgentConnectJob::is_pending)
        {
            return changed;
        }
        let pending_id = self
            .host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .pending_acp_agent_connect
            .take();
        if let Some(id) = pending_id {
            let agent = self
                .host
                .editor_state()
                .editor_ui
                .agent_settings
                .acp_agents
                .iter()
                .find(|agent| agent.id == id && agent.ready())
                .cloned();
            if let Some(agent) = agent {
                self.acp_agent_connect_job = Some(AcpAgentConnectJob::spawn(agent));
            }
            changed = true;
        }
        changed
    }

    fn poll_acp_agent_connect_job(&mut self) -> bool {
        let Some(job) = self.acp_agent_connect_job.as_mut() else {
            return false;
        };
        let id = job.id.clone();
        let Some(outcome) = job.poll() else {
            return false;
        };
        self.acp_agent_connect_job = None;

        let es = self.host.editor_state_mut();
        if !es.editor_ui.agent_settings.acp_agent_probe_in_flight(&id) {
            return false;
        }
        es.editor_ui.agent_settings.apply_acp_agent_connect_outcome(
            &id,
            AcpAgentConnectOutcome {
                connected: outcome.connected,
                info: outcome.info,
                error: outcome.error,
            },
        );
        es.rebuild_chat_models();
        self.host.mark_editor_state_dirty();
        true
    }

    pub(crate) fn acp_agent_connect_pending(&self) -> bool {
        self.acp_agent_connect_job
            .as_ref()
            .is_some_and(AcpAgentConnectJob::is_pending)
            || self
                .host
                .editor_state()
                .editor_ui
                .agent_settings
                .pending_acp_agent_connect
                .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::agent_settings::{AcpAgentConnectPhase, AcpConnectionType};
    use std::collections::BTreeMap;

    #[test]
    fn landed_acp_probe_failure_keeps_agent_disconnected() {
        let mut app = DesktopApp::new(None);
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.add_acp_agent_config(
            "Claude Code",
            AcpConnectionType::Local,
            "claude",
            Vec::new(),
            BTreeMap::new(),
            None,
            true,
        );
        settings.begin_acp_agent_connect(0);
        settings.pending_acp_agent_connect = None;
        let (job, tx) = AcpAgentConnectJob::pending_for_test("acp-1");
        app.acp_agent_connect_job = Some(job);

        tx.send(AcpAgentProbeOutcome::failed("initialize failed"))
            .unwrap();

        assert!(app.drain_acp_agent_connect());
        let settings = &app.host.editor_state().editor_ui.agent_settings;
        assert!(!settings.acp_agents[0].connected);
        let conn = settings.acp_agent_connection_for("acp-1");
        assert_eq!(conn.phase, AcpAgentConnectPhase::Error);
        assert_eq!(conn.error.as_deref(), Some("initialize failed"));
    }

    #[test]
    fn landed_acp_probe_success_marks_agent_connected() {
        let mut app = DesktopApp::new(None);
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.add_acp_agent_config(
            "Claude Code",
            AcpConnectionType::Local,
            "claude",
            Vec::new(),
            BTreeMap::new(),
            None,
            true,
        );
        settings.begin_acp_agent_connect(0);
        settings.pending_acp_agent_connect = None;
        let (job, tx) = AcpAgentConnectJob::pending_for_test("acp-1");
        app.acp_agent_connect_job = Some(job);

        tx.send(AcpAgentProbeOutcome::connected("Claude Code 1.0".into()))
            .unwrap();

        assert!(app.drain_acp_agent_connect());
        let settings = &app.host.editor_state().editor_ui.agent_settings;
        assert!(settings.acp_agents[0].connected);
        let conn = settings.acp_agent_connection_for("acp-1");
        assert_eq!(conn.phase, AcpAgentConnectPhase::Connected);
        assert_eq!(conn.info.as_deref(), Some("Claude Code 1.0"));
    }
}
