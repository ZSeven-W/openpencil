//! Public ACP types — port of `pen-acp/src/types.ts`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Transport an [`AcpAgentConfig`] uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionType {
    /// A local child process spoken to over stdio.
    Local,
    /// A remote agent reached over a WebSocket URL.
    Remote,
}

/// Persisted config for one user-configured ACP agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpAgentConfig {
    pub id: String,
    pub display_name: String,
    pub connection_type: ConnectionType,
    /// Local: the binary to spawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Local: CLI arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Local: extra environment variables. A JSON object on the wire
    /// (`Record<string, string>`), matching the TS `pen-acp` config.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Remote: the WebSocket endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Whether the agent shows in the UI.
    pub enabled: bool,
}

/// Agent identity returned by the `initialize` handshake.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpAgentInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Result of a connection attempt (TS `AcpConnectResult`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpConnectResult {
    pub connected: bool,
    pub agent_info: Option<AcpAgentInfo>,
    pub error: Option<String>,
}

/// Errors raised by the ACP client.
#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    /// A local agent config had no `command`, or a remote one no `url`.
    #[error("ACP config error: {0}")]
    Config(String),
    /// Failed to spawn the local agent process.
    #[error("failed to spawn ACP agent: {0}")]
    Spawn(String),
    /// Transport-level IO failure (stdio / socket).
    #[error("ACP transport error: {0}")]
    Transport(String),
    /// The agent returned a JSON-RPC error response.
    #[error("ACP agent error {code}: {message}")]
    Rpc { code: i64, message: String },
    /// A response / notification could not be (de)serialized.
    #[error("ACP protocol error: {0}")]
    Protocol(String),
    /// The connection closed before the request completed.
    #[error("ACP connection closed")]
    Closed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_config_round_trips_through_json() {
        let mut env = BTreeMap::new();
        env.insert("KEY".to_string(), "value".to_string());
        let cfg = AcpAgentConfig {
            id: "a1".into(),
            display_name: "My Agent".into(),
            connection_type: ConnectionType::Local,
            command: Some("my-agent".into()),
            args: vec!["--acp".into()],
            env,
            url: None,
            enabled: true,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        // camelCase on the wire.
        assert!(json.contains("\"connectionType\":\"local\""));
        assert!(json.contains("\"displayName\""));
        // `env` is a JSON object, not an array of pairs (TS parity).
        assert!(json.contains("\"env\":{\"KEY\":\"value\"}"));
        let back: AcpAgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn remote_config_omits_local_only_fields() {
        let cfg = AcpAgentConfig {
            id: "r1".into(),
            display_name: "Remote".into(),
            connection_type: ConnectionType::Remote,
            command: None,
            args: vec![],
            env: BTreeMap::new(),
            url: Some("ws://localhost:9000".into()),
            enabled: false,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(!json.contains("command"));
        assert!(json.contains("\"url\""));
        let back: AcpAgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cfg);
    }
}
