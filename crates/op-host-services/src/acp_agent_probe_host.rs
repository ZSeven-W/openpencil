//! Connect-time ACP agent probe job + outcome — the host-free half
//! carved out of `op-host-desktop`'s `acp_agent_probe_host.rs` (codex
//! Issue 5: the job struct is a `DesktopApp` field, so it lives here
//! for both crates to name it). The `impl DesktopApp` pump stays
//! desktop-side and drives this job through its public API.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_editor_core::agent_settings::{
    AcpAgentConfig as CoreAcpAgentConfig, AcpAgentConnectRequest, AcpConnectionType,
};

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
    request: AcpAgentConnectRequest,
    config: CoreAcpAgentConfig,
    rx: Option<Receiver<AcpAgentProbeOutcome>>,
}

impl AcpAgentConnectJob {
    pub fn spawn(request: AcpAgentConnectRequest, agent: CoreAcpAgentConfig) -> Self {
        debug_assert_eq!(request.id, agent.id);
        let config = acp_config_for_probe(&agent);
        let (tx, rx) = mpsc::channel();
        // Detached one-shot: local/remote connection and handshake stages are
        // bounded inside op-acp, and the probe explicitly shuts down/reaps a
        // local process tree before this worker exits.
        std::thread::spawn(move || {
            let outcome = probe_acp_agent_config(config);
            let _ = tx.send(outcome);
        });
        Self {
            request,
            config: agent,
            rx: Some(rx),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// The agent id this job is probing. Public accessor for the
    /// desktop-residual pump (private field is unreachable cross-crate).
    pub fn id(&self) -> &str {
        &self.request.id
    }

    pub fn request(&self) -> &AcpAgentConnectRequest {
        &self.request
    }

    /// Exact configuration snapshot that launched this probe.
    pub fn config(&self) -> &CoreAcpAgentConfig {
        &self.config
    }

    /// Test seam: construct a pending job + the sender that feeds it a
    /// fake outcome. Public (not `#[cfg(test)]`) so the desktop residual's
    /// `impl DesktopApp` tests can build one across the crate boundary.
    #[doc(hidden)]
    pub fn pending_for_test(
        request: AcpAgentConnectRequest,
        config: CoreAcpAgentConfig,
    ) -> (Self, mpsc::Sender<AcpAgentProbeOutcome>) {
        debug_assert_eq!(request.id, config.id);
        let (tx, rx) = mpsc::channel();
        (
            Self {
                request,
                config,
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
    // Reached from the probe worker thread today and from tokio workers via
    // the web-canvas server, so bridge through the runtime-aware helper.
    crate::chat_runtime::block_on_anywhere(async move {
        match op_acp::connect_acp_agent(&config).await {
            Ok(mut conn) => {
                let info = format_acp_agent_info(conn.agent_info(), &config.display_name);
                let outcome = validate_probe_session(&conn, info).await;
                conn.shutdown().await;
                outcome
            }
            Err(err) => AcpAgentProbeOutcome::failed(err.to_string()),
        }
    })
}

/// Validate that an initialized agent can create a session, then clean up the
/// deliberately ephemeral session before its transport is shut down. Close
/// releases active resources; delete then removes persisted `session/list`
/// state. Either advertised method remains a valid fallback on its own.
async fn validate_probe_session(
    conn: &op_acp::AcpConnection,
    info: String,
) -> AcpAgentProbeOutcome {
    // Initialize alone only proves protocol negotiation. Creating one empty
    // session also verifies auth state and catches agents that cannot serve.
    let session = match conn.new_session().await {
        Ok(session) => session,
        Err(err) => {
            return AcpAgentProbeOutcome::failed(format!("ACP session validation failed: {err}"));
        }
    };
    let mut failures = Vec::new();
    if conn.supports_session_close() {
        if let Err(err) = conn.close_session_if_supported(&session.session_id).await {
            failures.push(format!("close: {err}"));
        }
    }
    // Attempt delete even when close failed: the active-resource failure must
    // be reported, but it must not also leave avoidable persisted probe state.
    if conn.supports_session_delete() {
        if let Err(err) = conn.delete_session_if_supported(&session.session_id).await {
            failures.push(format!("delete: {err}"));
        }
    }
    if failures.is_empty() {
        AcpAgentProbeOutcome::connected(info)
    } else {
        AcpAgentProbeOutcome::failed(format!(
            "ACP session cleanup failed: {}",
            failures.join("; ")
        ))
    }
}

pub fn acp_config_for_probe(agent: &CoreAcpAgentConfig) -> op_acp::AcpAgentConfig {
    let mut env = agent.env.clone();
    if agent.connection_type == AcpConnectionType::Local
        && !env.keys().any(|key| key.eq_ignore_ascii_case("PATH"))
    {
        let path = crate::chat_spawn::effective_path_env();
        if !path.is_empty() {
            env.insert("PATH".into(), path);
        }
    }
    op_acp::AcpAgentConfig {
        id: agent.id.clone(),
        display_name: agent.display_name.clone(),
        connection_type: match agent.connection_type {
            AcpConnectionType::Local => op_acp::ConnectionType::Local,
            AcpConnectionType::Remote => op_acp::ConnectionType::Remote,
        },
        command: match agent.connection_type {
            AcpConnectionType::Local => Some(crate::chat_spawn::find_binary(&agent.command)),
            AcpConnectionType::Remote => None,
        },
        args: agent.args.clone(),
        env,
        url: agent.url.clone(),
        enabled: agent.enabled,
    }
}

/// Whether each quick-add preset's command resolves to a real file on
/// this machine, keyed by preset id.
///
/// Advisory only. It resolves against the same merged login-shell PATH
/// the spawn path uses (so a GUI launch sees the user's nvm/homebrew
/// shims), but a `false` here never blocks adding the preset — PATH is a
/// snapshot, and the ACP handshake is the authority on whether the agent
/// actually runs.
pub fn probe_acp_preset_availability() -> std::collections::BTreeMap<String, bool> {
    op_editor_core::acp_agent_presets::ACP_AGENT_PRESETS
        .iter()
        .map(|preset| {
            let resolved = crate::chat_spawn::find_binary(preset.command);
            // `find_binary` echoes the bare name back when it finds
            // nothing, so "resolved to an existing file" is the test.
            let found = std::path::Path::new(&resolved).is_file();
            (preset.id.to_string(), found)
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tokio::io::BufReader;

    fn core_config(connection_type: AcpConnectionType) -> CoreAcpAgentConfig {
        CoreAcpAgentConfig {
            id: "test".into(),
            display_name: "Test".into(),
            connection_type,
            command: std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            args: vec!["--acp".into()],
            env: BTreeMap::from([("Path".into(), "configured-path".into())]),
            url: Some("ws://example.invalid/acp".into()),
            enabled: true,
            connected: false,
        }
    }

    #[test]
    fn local_probe_resolves_command_and_preserves_explicit_path() {
        let source = core_config(AcpConnectionType::Local);
        let config = acp_config_for_probe(&source);
        assert_eq!(config.command.as_deref(), Some(source.command.as_str()));
        assert_eq!(
            config.env.get("Path").map(String::as_str),
            Some("configured-path")
        );
        assert!(!config.env.contains_key("PATH"));
    }

    #[test]
    fn remote_probe_does_not_carry_a_local_command() {
        let config = acp_config_for_probe(&core_config(AcpConnectionType::Remote));
        assert_eq!(config.connection_type, op_acp::ConnectionType::Remote);
        assert!(config.command.is_none());
    }

    fn probe_lifecycle(
        session_capabilities: serde_json::Value,
        fail_close: bool,
    ) -> (Vec<String>, AcpAgentProbeOutcome) {
        crate::chat_runtime::block_on_anywhere(async move {
            let lifecycle_count = usize::from(session_capabilities.get("close").is_some())
                + usize::from(session_capabilities.get("delete").is_some());
            let (client_write, agent_read) = tokio::io::duplex(4096);
            let (agent_write, client_read) = tokio::io::duplex(4096);
            let agent = tokio::spawn(async move {
                let mut read = BufReader::new(agent_read);
                let mut write = agent_write;
                let mut methods = Vec::new();
                while let Some(frame) = op_acp::transport::read_frame(&mut read).await.unwrap() {
                    let method = frame["method"].as_str().unwrap().to_string();
                    methods.push(method.clone());
                    let response = match method.as_str() {
                        "session/close" if fail_close => {
                            assert_eq!(frame["params"]["sessionId"], "probe-session");
                            serde_json::json!({
                                "jsonrpc": "2.0", "id": frame["id"],
                                "error": { "code": -32_001, "message": "close failed" }
                            })
                        }
                        method => {
                            let result = match method {
                                "initialize" => serde_json::json!({
                                    "protocolVersion": 1,
                                    "agentCapabilities": {
                                        "sessionCapabilities": session_capabilities.clone()
                                    }
                                }),
                                "session/new" => {
                                    serde_json::json!({ "sessionId": "probe-session" })
                                }
                                "session/delete" | "session/close" => {
                                    assert_eq!(frame["params"]["sessionId"], "probe-session");
                                    serde_json::json!({})
                                }
                                other => panic!("unexpected probe method: {other}"),
                            };
                            serde_json::json!({
                                "jsonrpc": "2.0", "id": frame["id"], "result": result
                            })
                        }
                    };
                    op_acp::transport::write_frame(&mut write, &response)
                        .await
                        .unwrap();
                    if methods.len() == 2 + lifecycle_count {
                        break;
                    }
                }
                methods
            });

            let mut conn = op_acp::AcpConnection::new(client_read, client_write, None);
            conn.initialize("Probe").await.unwrap();
            let outcome = validate_probe_session(&conn, "Probe 1.0".into()).await;
            conn.shutdown().await;
            (agent.await.unwrap(), outcome)
        })
    }

    fn probe_lifecycle_methods(session_capabilities: serde_json::Value) -> Vec<String> {
        let (methods, outcome) = probe_lifecycle(session_capabilities, false);
        assert_eq!(outcome, AcpAgentProbeOutcome::connected("Probe 1.0".into()));
        methods
    }

    #[test]
    fn probe_closes_then_deletes_when_both_are_advertised() {
        let methods = probe_lifecycle_methods(serde_json::json!({
            "delete": {},
            "close": {}
        }));
        assert_eq!(
            methods,
            [
                "initialize",
                "session/new",
                "session/close",
                "session/delete"
            ]
        );
    }

    #[test]
    fn probe_falls_back_to_close_when_delete_is_not_advertised() {
        let methods = probe_lifecycle_methods(serde_json::json!({ "close": {} }));
        assert_eq!(methods, ["initialize", "session/new", "session/close"]);
    }

    #[test]
    fn probe_uses_delete_when_close_is_not_advertised() {
        let methods = probe_lifecycle_methods(serde_json::json!({ "delete": {} }));
        assert_eq!(methods, ["initialize", "session/new", "session/delete"]);
    }

    #[test]
    fn probe_still_deletes_and_reports_when_close_fails() {
        let (methods, outcome) =
            probe_lifecycle(serde_json::json!({ "close": {}, "delete": {} }), true);
        assert_eq!(
            methods,
            [
                "initialize",
                "session/new",
                "session/close",
                "session/delete"
            ]
        );
        let error = outcome.error.expect("cleanup failure");
        assert!(error.contains("close: ACP agent error -32001: close failed"));
        assert!(!outcome.connected);
    }
}
