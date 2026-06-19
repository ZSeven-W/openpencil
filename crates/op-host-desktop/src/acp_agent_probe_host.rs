//! Host pump for connect-time ACP agent probes.
//!
//! The job struct + probe fns live in
//! [`op_web_daemon::acp_agent_probe_host`] (codex Issue 5 — the job is
//! a `DesktopApp` field); this residual keeps only the `impl DesktopApp`
//! pump, which drives the job through its public API.

use op_editor_core::agent_settings::AcpAgentConnectOutcome;

use crate::DesktopApp;
use op_web_daemon::acp_agent_probe_host::AcpAgentProbeOutcome;
// Re-export so `crate::acp_agent_probe_host::AcpAgentConnectJob` (the
// `DesktopApp` field type in `main.rs`) still resolves with zero churn.
pub use op_web_daemon::acp_agent_probe_host::AcpAgentConnectJob;

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
        let id = job.id().to_string();
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
