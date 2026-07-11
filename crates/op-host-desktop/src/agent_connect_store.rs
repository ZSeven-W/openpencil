//! Cross-launch persistence for CLI provider connections.
//!
//! Before this file existed NOTHING about agent settings survived a
//! restart — `connected[5]` and the probed model catalog are runtime
//! state, so every launch greeted the user with five "Connect" buttons
//! (measured / user-reported on both dev and packaged builds). The store
//! keeps only the CONNECTED flags (no keys, no models — CLI providers
//! have no secrets and the catalog must be re-probed anyway); on startup
//! the host replays a silent connect probe for each remembered provider,
//! so the status and the model picker come back by themselves and an
//! uninstalled CLI honestly degrades to disconnected.

use op_editor_core::AgentProvider;

const FILE: &str = "agents.json";

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct PersistedAgentConnections {
    #[serde(default)]
    connected: Vec<String>,
}

/// Persist the currently-connected provider ids. Best-effort: a failed
/// write must never break the probe flow.
pub(crate) fn save(connected: &[bool; 5]) {
    let value = PersistedAgentConnections {
        connected: AgentProvider::ALL
            .iter()
            .enumerate()
            .filter(|(index, _)| connected[*index])
            .map(|(_, provider)| provider.name().to_string())
            .collect(),
    };
    if let Err(err) = op_config_store::write_json(FILE, &value) {
        eprintln!("[agents] connection store write failed: {err}");
    }
}

/// Providers remembered as connected from the previous session, in
/// `AgentProvider::ALL` order.
pub(crate) fn load() -> Vec<AgentProvider> {
    let Ok(Some(value)) = op_config_store::read_json::<PersistedAgentConnections>(FILE) else {
        return Vec::new();
    };
    AgentProvider::ALL
        .iter()
        .copied()
        .filter(|provider| value.connected.iter().any(|name| name == provider.name()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_connected_providers_through_json() {
        let value = PersistedAgentConnections {
            connected: vec!["Claude Code".into(), "Gemini CLI".into()],
        };
        let json = serde_json::to_string(&value).expect("serialize");
        let back: PersistedAgentConnections = serde_json::from_str(&json).expect("parse");
        assert_eq!(back.connected, vec!["Claude Code", "Gemini CLI"]);
    }
}
