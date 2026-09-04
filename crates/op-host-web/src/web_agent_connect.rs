//! Browser-side bridge for desktop-daemon provider probes.
//!
//! The WASM shell cannot spawn local CLIs. It raises the same
//! `pending_provider_connect` seam as native, then this module POSTs that
//! request to the `openpencil-desktop --serve-web` daemon. The daemon runs the
//! real probe and returns the concrete result; this module applies it to the
//! browser's local editor state.

use std::cell::RefCell;
use std::rc::Rc;

use op_editor_core::agent_settings::ProviderConnectOutcome;
use op_editor_core::{AgentProvider, EditorState, ModelEntry};
use wasm_bindgen::JsValue;

use crate::repaint_ctx::RepaintContext;

type InnerRc<C> = Rc<RefCell<C>>;

fn console_warn(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(msg));
}

pub(crate) fn drain_pending_provider_connect<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let pending = inner
        .borrow_mut()
        .host_mut()
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .pending_provider_connect
        .take();
    let Some(provider) = pending else {
        return;
    };

    let body = serde_json::json!({ "provider": provider_key(provider) }).to_string();
    let base = crate::daemon_base::daemon_base();
    let inner_for_response = inner.clone();
    let on_response: Rc<dyn Fn(String)> = Rc::new(move |response: String| {
        let mut b = inner_for_response.borrow_mut();
        if apply_provider_connect_response(b.host_mut().editor_state_mut(), provider, &response) {
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
    });
    if !crate::live_sync::post_json(
        &format!("{base}/api/agents/connect"),
        &body,
        Some(on_response),
    ) {
        let mut b = inner.borrow_mut();
        apply_provider_connect_start_failure(b.host_mut().editor_state_mut(), provider);
        b.host_mut().mark_editor_state_dirty();
        let _ = b.repaint();
        console_warn("provider connect request could not start");
    }
}

pub(crate) fn apply_provider_connect_response(
    state: &mut EditorState,
    provider: AgentProvider,
    body: &str,
) -> bool {
    if !state
        .editor_ui
        .agent_settings
        .provider_probe_in_flight(provider)
    {
        return false;
    }
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            apply_provider_connect_error(
                state,
                provider,
                "Agent CLI connection failed: invalid daemon response",
            );
            return true;
        }
    };
    let mut connected = parsed
        .get("connected")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let models = parse_models(provider, parsed.get("models"));
    let mut error = str_field(&parsed, "error");
    let warning = str_field(&parsed, "warning");
    let not_installed = parsed
        .get("notInstalled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let install_command = str_field(&parsed, "installCommand");
    let mut info = str_field(&parsed, "connectionInfo");
    let hint_path = str_field(&parsed, "hintPath");
    let version = str_field(&parsed, "version");
    if connected && models.is_empty() {
        connected = false;
        info = None;
        error = Some(op_editor_core::missing_models_connect_error(
            state.editor_ui.effective_locale(),
            provider,
        ));
    }

    state
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            provider,
            ProviderConnectOutcome {
                connected,
                info,
                warning,
                error,
                not_installed,
                install_command,
                hint_path,
                version,
            },
        );
    if connected && !models.is_empty() {
        state
            .chat
            .discovered_models
            .retain(|m| m.provider != provider);
        state.chat.discovered_models.extend(models);
        sort_discovered_models(state);
    }
    state.rebuild_chat_models();
    true
}

fn apply_provider_connect_start_failure(state: &mut EditorState, provider: AgentProvider) {
    apply_provider_connect_error(
        state,
        provider,
        "Agent CLI connection request could not start. Is the web daemon running?",
    );
}

fn apply_provider_connect_error(state: &mut EditorState, provider: AgentProvider, error: &str) {
    state
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            provider,
            ProviderConnectOutcome {
                connected: false,
                error: Some(error.to_string()),
                ..Default::default()
            },
        );
    state.rebuild_chat_models();
}

fn parse_models(provider: AgentProvider, value: Option<&serde_json::Value>) -> Vec<ModelEntry> {
    value
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let value = item.get("value").and_then(|v| v.as_str())?;
            let display_name = item
                .get("displayName")
                .or_else(|| item.get("display_name"))
                .and_then(|v| v.as_str())
                .unwrap_or(value);
            Some(ModelEntry::new(provider, value, display_name))
        })
        .collect()
}

fn str_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn sort_discovered_models(state: &mut EditorState) {
    state.chat.discovered_models.sort_by_key(|m| {
        AgentProvider::ALL
            .iter()
            .position(|p| *p == m.provider)
            .unwrap_or(usize::MAX)
    });
}

fn provider_key(provider: AgentProvider) -> &'static str {
    match provider {
        AgentProvider::ClaudeCode => "claude",
        AgentProvider::CodexCli => "codex",
        AgentProvider::OpenCode => "opencode",
        AgentProvider::GithubCopilot => "github-copilot",
        AgentProvider::Antigravity => "antigravity",
        AgentProvider::GrokBuild => "grok-build",
    }
}
