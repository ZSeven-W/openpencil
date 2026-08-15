//! `ChatProvider` construction + selection for the chat launch path —
//! agent-index and selected-model resolution, the built-in provider's
//! canvas-tool wiring, and ACP config translation. Carved out of the
//! `chat_session_launch.rs` spine to keep it under the 800-line cap;
//! pure code motion.

use std::sync::mpsc::Receiver;
use std::sync::Arc;

use op_ai::chat_provider::{ChatProvider, CliName};
use op_editor_core::{BuiltinAgentConfig, EditorState, ModelEntry};
use op_host_native::WidgetHostNative;

use crate::chat_acp::AcpProvider;
use op_host_services::chat_builtin_http::ConfiguredBuiltinProvider;
use op_host_services::chat_canvas_tools::{
    chat_tool_channel, chat_tool_defs_for_write_scope, ChatToolRequest,
};
use op_host_services::chat_claude::ClaudeCodeProvider;
use op_host_services::chat_copilot::CopilotProvider;
use op_host_services::chat_http_server::OpenCodeProvider;
use op_host_services::chat_subprocess::SubprocessProvider;

/// Build the `ChatProvider` for an agent index (into
/// `AgentProvider::ALL`: 0 ClaudeCode, 1 CodexCli, 2 OpenCode,
/// 3 GithubCopilot, 4 Antigravity, 5 GrokBuild, 6 DeepSeekHarness). Claude Code uses its
/// dedicated SDK adapter; Codex uses the subprocess transport; Copilot
/// rides the official SDK; OpenCode chats over its local HTTP server
/// (`chat_http_server.rs`); DeepSeek Harness is a one-shot subprocess
/// bridge (`chat_subprocess_dsh.rs`).
///
/// `chat_session` opts the Claude Code and Copilot adapters into
/// their process-wide chat resume slots (multi-turn context via
/// `--resume` / `session.resume`). The design / codegen paths pass
/// `false` so their sub-requests never join — or pollute — the
/// user's chat conversation. OpenCode opens a fresh server session
/// per turn (TS parity: `streamViaOpenCode` never resumes), so it
/// has no chat/non-chat split.
fn provider_for_agent(agent_idx: usize, chat_session: bool) -> Option<Box<dyn ChatProvider>> {
    match agent_idx {
        0 => Some(Box::new(if chat_session {
            ClaudeCodeProvider::for_chat()
        } else {
            ClaudeCodeProvider::new()
        })),
        1 => SubprocessProvider::for_cli(CliName::Codex)
            .map(|p| Box::new(p) as Box<dyn ChatProvider>),
        2 => Some(Box::new(OpenCodeProvider::new())),
        3 => Some(Box::new(if chat_session {
            CopilotProvider::for_chat()
        } else {
            CopilotProvider::new()
        })),
        4 => subprocess_provider(CliName::Antigravity, chat_session),
        5 => subprocess_provider(CliName::GrokBuild, chat_session),
        6 => subprocess_provider(CliName::Dsh, chat_session),
        _ => None,
    }
}

fn subprocess_provider(cli: CliName, chat: bool) -> Option<Box<dyn ChatProvider>> {
    let provider = if chat {
        SubprocessProvider::for_cli(cli)
    } else {
        SubprocessProvider::for_cli_generation(cli)
    };
    provider.map(|p| Box::new(p) as Box<dyn ChatProvider>)
}

/// Provider for non-chat consumers (design orchestrator LLM, codegen)
/// — session-untracked so their requests stay out of the user's chat
/// conversation.
pub(crate) fn provider_for_selected_model(
    host: &WidgetHostNative,
) -> Option<Box<dyn ChatProvider>> {
    provider_for_selected_model_impl(host, false)
}

/// Provider for the chat panel's own turns — Claude Code / Copilot
/// resume their process-wide chat sessions across sends.
pub(super) fn chat_provider_for_selected_model(
    host: &WidgetHostNative,
) -> Option<Box<dyn ChatProvider>> {
    provider_for_selected_model_impl(host, true)
}

fn provider_for_selected_model_impl(
    host: &WidgetHostNative,
    chat_session: bool,
) -> Option<Box<dyn ChatProvider>> {
    if let Some(entry) = host.editor_state().chat.selected_model_entry() {
        if entry.builtin_provider_id.is_some() {
            return provider_for_builtin(host.editor_state(), entry);
        }
        if let Some(id) = entry.acp_agent_id() {
            return provider_for_acp(host.editor_state(), id);
        }
    }
    provider_for_agent(
        host.editor_state().editor_ui.chat_selected_agent,
        chat_session,
    )
}

/// When the selected chat model is a ready builtin (API-key) entry,
/// build its provider with the canvas tool set + a fresh UI tool
/// channel — the GAP #32 tool-executing path. `None` falls through to
/// the plain provider routing.
pub(crate) fn builtin_provider_with_tools(
    host: &WidgetHostNative,
) -> Option<(Box<dyn ChatProvider>, Receiver<ChatToolRequest>)> {
    let state = host.editor_state();
    let entry = state.chat.selected_model_entry()?;
    let config = selected_builtin_agent_config(state, entry)?;
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&config)?;
    let (executor, tool_rx) = chat_tool_channel();
    let has_frame_scope = op_host_services::chat_intent::has_selected_frame_target(state);
    let provider = provider.with_canvas_tools(
        chat_tool_defs_for_write_scope(has_frame_scope),
        Arc::new(executor),
    );
    Some((Box::new(provider), tool_rx))
}

/// Model id to forward to the routed CLI transport, from the chat
/// panel's selected model entry (`chat.available_models[selected_model]`).
///
/// Only CLI-backed entries qualify:
/// - built-in entries (`builtin_provider_id`) carry their model in the
///   selected entry (`entry.value` is the composite
///   `builtin:<id>:<model>`, not a CLI wire id);
/// - ACP entries (`acp:<id>`) address an agent, not a model;
/// - an entry whose provider differs from the agent actually routed by
///   [`provider_for_agent`] must not leak its id to a different CLI
///   (selection sync normally keeps them aligned, but a stale index
///   after a rebuild could diverge).
///
/// Blank ids collapse to `None` so transports never emit an empty
/// model flag.
pub(crate) fn selected_cli_model_id(host: &WidgetHostNative) -> Option<String> {
    let state = host.editor_state();
    let entry = state.chat.selected_model_entry()?;
    if entry.builtin_provider_id.is_some() || entry.acp_agent_id().is_some() {
        return None;
    }
    let routed = op_editor_core::AgentProvider::ALL.get(state.editor_ui.chat_selected_agent)?;
    if *routed != entry.provider {
        return None;
    }
    let value = entry.value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

pub(super) fn selected_builtin_agent_config(
    state: &EditorState,
    entry: &ModelEntry,
) -> Option<BuiltinAgentConfig> {
    let id = entry.builtin_provider_id.as_deref()?;
    let selected_model = entry.builtin_model_id()?;
    let mut config = state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .find(|agent| agent.id == id && agent.ready())?
        .clone();
    if !config.has_model(selected_model) {
        return None;
    }
    // Downstream provider construction remains single-model. Narrow the
    // cloned request config without mutating the persisted provider.
    config.models = vec![selected_model.to_string()];
    config.ready().then_some(config)
}

fn provider_for_builtin(state: &EditorState, entry: &ModelEntry) -> Option<Box<dyn ChatProvider>> {
    let config = selected_builtin_agent_config(state, entry)?;
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&config)?;
    Some(Box::new(provider))
}

fn provider_for_acp(state: &EditorState, id: &str) -> Option<Box<dyn ChatProvider>> {
    let settings = &state.editor_ui.agent_settings;
    let config = settings.acp_agents.iter().find(|agent| {
        agent.id == id && agent.ready() && settings.acp_agent_verified_connected(&agent.id)
    })?;
    // Canvas tool surface for the agent (TS parity, agent.ts:503-521):
    // the live MCP server's HTTP endpoint when it is running. `None`
    // makes the provider refuse the turn with the TS error message.
    let mcp = state.editor_ui.agent_settings.mcp_server;
    let live_mcp_port = mcp.running.then_some(mcp.port);
    Some(Box::new(AcpProvider::new(
        acp_config_for_provider(config),
        live_mcp_port,
    )))
}

fn acp_config_for_provider(agent: &op_editor_core::AcpAgentConfig) -> op_acp::AcpAgentConfig {
    op_acp::AcpAgentConfig {
        id: agent.id.clone(),
        display_name: agent.display_name.clone(),
        connection_type: match agent.connection_type {
            op_editor_core::AcpConnectionType::Local => op_acp::ConnectionType::Local,
            op_editor_core::AcpConnectionType::Remote => op_acp::ConnectionType::Remote,
        },
        command: match agent.connection_type {
            op_editor_core::AcpConnectionType::Local => Some(agent.command.clone()),
            op_editor_core::AcpConnectionType::Remote => None,
        },
        args: agent.args.clone(),
        env: agent.env.clone(),
        url: agent.url.clone(),
        enabled: agent.enabled,
    }
}

pub(super) fn selected_provider_label(host: &WidgetHostNative) -> String {
    if let Some(entry) = host.editor_state().chat.selected_model_entry() {
        if let Some(id) = entry.builtin_provider_id.as_deref() {
            if let Some(agent) = host
                .editor_state()
                .editor_ui
                .agent_settings
                .builtin_agents
                .iter()
                .find(|agent| agent.id == id)
            {
                return agent.display_name.clone();
            }
        }
        if let Some(id) = entry.acp_agent_id() {
            if let Some(agent) = host
                .editor_state()
                .editor_ui
                .agent_settings
                .acp_agents
                .iter()
                .find(|agent| agent.id == id)
            {
                return agent.display_name.clone();
            }
        }
    }
    let agent_idx = host.editor_state().editor_ui.chat_selected_agent;
    op_editor_core::AgentProvider::ALL
        .get(agent_idx)
        .map(|a| a.name().to_string())
        .unwrap_or_else(|| "This agent".into())
}

#[cfg(test)]
mod routing_tests {
    use super::*;

    #[test]
    fn deepseek_harness_routes_to_the_dsh_subprocess_provider() {
        // ALL index 6 is the append-only DeepSeek Harness tail slot
        // (its persisted `connected` flag lives there too).
        assert_eq!(
            op_editor_core::AgentProvider::ALL[6],
            op_editor_core::AgentProvider::DeepSeekHarness
        );
        let design = provider_for_agent(6, false).expect("DSH routes to a provider");
        assert_eq!(design.provider_label(), "DeepSeek Harness");
        // The chat-session path routes the same CLI (single-shot
        // subprocess — there is no session-resume slot to join).
        let chat = provider_for_agent(6, true).expect("chat DSH routes too");
        assert_eq!(chat.provider_label(), "DeepSeek Harness");
        // Out-of-range agent indices stay unrouted (fail-closed).
        assert!(provider_for_agent(7, false).is_none());
    }

    #[test]
    fn every_all_index_has_a_provider_route() {
        // The ALL array is append-only, so routing must stay keyed to
        // it: a new tail entry with no route arm would silently fall
        // through to `None` and strand the user's agent picker.
        for index in 0..op_editor_core::AgentProvider::ALL.len() {
            let provider = provider_for_agent(index, false);
            assert!(
                provider.is_some(),
                "AgentProvider::ALL index {index} has no provider route"
            );
        }
    }
}
