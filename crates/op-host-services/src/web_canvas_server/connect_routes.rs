//! Agent/provider connection + MCP-server settings routes: `POST
//! /api/mcp/server` plus the ACP-agent and provider connect handlers and
//! their probe-outcome folding. Split out of `web_canvas_server.rs` to keep
//! the spine under the 800-line cap.

use super::*;

pub(super) fn update_mcp_server_settings(body: &str, state: &mut WebCanvasState) -> WebReply {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            return WebReply {
                status: "400 Bad Request",
                body: crate::mcp_serve::rest_error_body("Invalid MCP server request body"),
            };
        }
    };
    let action = parsed
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let port = match parsed.get("port").and_then(|v| v.as_u64()) {
        Some(raw) if raw <= u16::MAX as u64 => Some((raw as u16).max(1024)),
        Some(_) => {
            return WebReply {
                status: "400 Bad Request",
                body: crate::mcp_serve::rest_error_body("Invalid MCP server port"),
            };
        }
        None => None,
    };
    let server = &mut state.editor.editor_ui.agent_settings.mcp_server;
    match action {
        "start" => {
            if let Some(port) = port {
                server.port = port;
            }
            server.running = true;
        }
        "stop" => {
            if let Some(port) = port {
                server.port = port;
            }
            server.running = false;
        }
        _ => {
            return WebReply {
                status: "400 Bad Request",
                body: crate::mcp_serve::rest_error_body("Invalid MCP server action"),
            };
        }
    }
    WebReply {
        status: "200 OK",
        body: serde_json::json!({
            "ok": true,
            "running": server.running,
            "port": server.port,
        })
        .to_string(),
    }
}

pub fn handle_acp_agent_connect_request_with_probe<F>(
    body: &str,
    state: &mut WebCanvasState,
    probe: F,
) -> WebReply
where
    F: FnOnce(op_acp::AcpAgentConfig) -> crate::acp_agent_probe_host::AcpAgentProbeOutcome,
{
    let Some(id) = parse_acp_agent_connect_request(body) else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("Missing ACP agent id"),
        };
    };
    let Some(index) = state
        .editor
        .editor_ui
        .agent_settings
        .acp_agents
        .iter()
        .position(|agent| agent.id == id && agent.ready())
    else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("ACP agent is not configured"),
        };
    };
    let agent = state.editor.editor_ui.agent_settings.acp_agents[index].clone();
    state
        .editor
        .editor_ui
        .agent_settings
        .begin_acp_agent_connect(index);
    let outcome = probe(crate::acp_agent_probe_host::acp_config_for_probe(&agent));
    apply_acp_agent_probe_outcome(&id, outcome, state)
}

pub(super) fn parse_acp_agent_connect_request(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    parsed
        .get("id")
        .or_else(|| parsed.get("agentId"))
        .and_then(|v| v.as_str())
        .filter(|id| !id.trim().is_empty())
        .map(str::to_string)
}

pub(super) fn apply_acp_agent_probe_outcome(
    id: &str,
    outcome: crate::acp_agent_probe_host::AcpAgentProbeOutcome,
    state: &mut WebCanvasState,
) -> WebReply {
    state
        .editor
        .editor_ui
        .agent_settings
        .apply_acp_agent_connect_outcome(
            id,
            AcpAgentConnectOutcome {
                connected: outcome.connected,
                info: outcome.info.clone(),
                error: outcome.error.clone(),
            },
        );
    state.editor.rebuild_chat_models();
    WebReply {
        status: "200 OK",
        body: serde_json::json!({
            "ok": true,
            "id": id,
            "connected": outcome.connected,
            "connectionInfo": outcome.info,
            "error": outcome.error,
        })
        .to_string(),
    }
}

pub fn handle_provider_connect_request_with_probe<F>(
    body: &str,
    state: &mut WebCanvasState,
    probe: F,
) -> WebReply
where
    F: FnOnce(op_ai::agent_settings_state::AgentProvider) -> crate::provider_probe::ProbeOutcome,
{
    let Some(provider) = parse_provider_connect_request(body) else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("Missing provider"),
        };
    };
    state
        .editor
        .editor_ui
        .agent_settings
        .begin_provider_connect(provider);
    let outcome = probe(provider_to_probe(provider));
    apply_provider_probe_outcome(provider, outcome, state)
}

pub(super) fn parse_provider_connect_request(body: &str) -> Option<op_editor_core::AgentProvider> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    parsed
        .get("provider")
        .and_then(|v| v.as_str())
        .and_then(parse_agent_provider)
}

pub(super) fn parse_agent_provider(raw: &str) -> Option<op_editor_core::AgentProvider> {
    use op_editor_core::AgentProvider;
    let normalized = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect::<String>();
    match normalized.as_str() {
        "claude" | "claudecode" => Some(AgentProvider::ClaudeCode),
        "codex" | "codexcli" => Some(AgentProvider::CodexCli),
        "opencode" => Some(AgentProvider::OpenCode),
        "githubcopilot" | "copilot" => Some(AgentProvider::GithubCopilot),
        "antigravity" | "agy" => Some(AgentProvider::Antigravity),
        "grok" | "grokbuild" => Some(AgentProvider::GrokBuild),
        "dsh" | "deepseekharness" => Some(AgentProvider::DeepSeekHarness),
        _ => None,
    }
}

pub(super) fn provider_to_probe(
    provider: op_editor_core::AgentProvider,
) -> op_ai::agent_settings_state::AgentProvider {
    use op_ai::agent_settings_state::AgentProvider as ProbeProvider;
    use op_editor_core::AgentProvider;
    match provider {
        AgentProvider::ClaudeCode => ProbeProvider::ClaudeCode,
        AgentProvider::CodexCli => ProbeProvider::CodexCli,
        AgentProvider::OpenCode => ProbeProvider::OpenCode,
        AgentProvider::GithubCopilot => ProbeProvider::GithubCopilot,
        AgentProvider::Antigravity => ProbeProvider::Antigravity,
        AgentProvider::GrokBuild => ProbeProvider::GrokBuild,
        AgentProvider::DeepSeekHarness => ProbeProvider::DeepSeekHarness,
    }
}

pub(super) fn apply_provider_probe_outcome(
    provider: op_editor_core::AgentProvider,
    outcome: crate::provider_probe::ProbeOutcome,
    state: &mut WebCanvasState,
) -> WebReply {
    let outcome = crate::provider_probe_host::normalize_provider_probe_outcome(provider, outcome);
    let crate::provider_probe::ProbeOutcome {
        connected,
        models,
        error,
        warning,
        not_installed,
        install_command,
        connection_info,
        hint_path,
        version,
    } = outcome;
    let response_models: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            serde_json::json!({
                "provider": provider_key(provider),
                "value": m.value,
                "displayName": m.display_name,
            })
        })
        .collect();
    state
        .editor
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            provider,
            ProviderConnectOutcome {
                connected,
                info: connection_info.clone(),
                warning: warning.clone(),
                error: error.clone(),
                not_installed,
                install_command: install_command.clone(),
                hint_path: hint_path.clone(),
                version: version.clone(),
            },
        );
    state
        .editor
        .editor_ui
        .agent_settings
        .pending_provider_connect = None;
    if connected && !models.is_empty() {
        state
            .editor
            .chat
            .discovered_models
            .retain(|m| m.provider != provider);
        state.editor.chat.discovered_models.extend(
            models
                .into_iter()
                .map(crate::model_discovery::model_entry_to_ec),
        );
        sort_discovered_models(&mut state.editor);
    }
    state.editor.rebuild_chat_models();
    WebReply {
        status: "200 OK",
        body: serde_json::json!({
            "ok": true,
            "provider": provider_key(provider),
            "connected": connected,
            "models": response_models,
            "error": error,
            "warning": warning,
            "notInstalled": not_installed,
            "installCommand": install_command,
            "connectionInfo": connection_info,
            "hintPath": hint_path,
            "version": version,
        })
        .to_string(),
    }
}

pub(super) fn sort_discovered_models(editor: &mut EditorState) {
    editor.chat.discovered_models.sort_by_key(|m| {
        op_editor_core::AgentProvider::ALL
            .iter()
            .position(|p| *p == m.provider)
            .unwrap_or(usize::MAX)
    });
}

pub(super) fn provider_key(provider: op_editor_core::AgentProvider) -> &'static str {
    use op_editor_core::AgentProvider;
    match provider {
        AgentProvider::ClaudeCode => "claude",
        AgentProvider::CodexCli => "codex",
        AgentProvider::OpenCode => "opencode",
        AgentProvider::GithubCopilot => "github-copilot",
        AgentProvider::Antigravity => "antigravity",
        AgentProvider::GrokBuild => "grok-build",
        AgentProvider::DeepSeekHarness => "deepseek-harness",
    }
}
