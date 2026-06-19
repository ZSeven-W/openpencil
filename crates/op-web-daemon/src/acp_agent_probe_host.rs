//! Connect-time ACP agent probe job + outcome — the host-free half
//! carved out of `op-host-desktop`'s `acp_agent_probe_host.rs` (codex
//! Issue 5: the job struct is a `DesktopApp` field, so it lives here
//! for both crates to name it). The `impl DesktopApp` pump stays
//! desktop-side and drives this job through its public API.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_editor_core::agent_settings::{AcpAgentConfig as CoreAcpAgentConfig, AcpConnectionType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpAgentProbeOutcome {
    pub connected: bool,
    pub info: Option<String>,
    pub error: Option<String>,
}

impl AcpAgentProbeOutcome {
    /// Public (was private) so the desktop residual's pump tests can
    /// build a success outcome across the crate boundary.
    pub fn connected(info: String) -> Self {
        Self {
            connected: true,
            info: Some(info),
            error: None,
        }
    }

    /// Public (was private) so the desktop residual's pump tests can
    /// build a failure outcome across the crate boundary.
    pub fn failed(error: impl Into<String>) -> Self {
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

    /// The agent id this job is probing. Public accessor for the
    /// desktop-residual pump (private field is unreachable cross-crate).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Test seam: construct a pending job + the sender that feeds it a
    /// fake outcome. Public (not `#[cfg(test)]`) so the desktop residual's
    /// `impl DesktopApp` tests can build one across the crate boundary.
    #[doc(hidden)]
    pub fn pending_for_test(
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

    pub fn poll(&mut self) -> Option<AcpAgentProbeOutcome> {
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

pub fn probe_acp_agent_config(config: op_acp::AcpAgentConfig) -> AcpAgentProbeOutcome {
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

pub fn acp_config_for_probe(agent: &CoreAcpAgentConfig) -> op_acp::AcpAgentConfig {
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

pub fn format_acp_agent_info(info: &op_acp::AcpAgentInfo, fallback: &str) -> String {
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
