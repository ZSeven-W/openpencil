//! Browser-side bridge for desktop-daemon ACP agent probes.

use std::cell::RefCell;
use std::rc::Rc;

use op_editor_core::agent_settings::AcpAgentConnectOutcome;
use op_editor_core::EditorState;
use wasm_bindgen::JsValue;

use crate::repaint_ctx::RepaintContext;

type InnerRc<C> = Rc<RefCell<C>>;

fn console_warn(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(msg));
}

pub(crate) fn drain_pending_acp_agent_connect<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let pending = inner
        .borrow_mut()
        .host_mut()
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .pending_acp_agent_connect
        .take();
    let Some(id) = pending else {
        return;
    };

    let body = serde_json::json!({ "id": id }).to_string();
    let base = crate::daemon_base::daemon_base();
    let id_for_response = id.clone();
    let inner_for_response = inner.clone();
    let on_response: Rc<dyn Fn(String)> = Rc::new(move |response: String| {
        let mut b = inner_for_response.borrow_mut();
        if apply_acp_agent_connect_response(
            b.host_mut().editor_state_mut(),
            &id_for_response,
            &response,
        ) {
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
    });
    if !crate::live_sync::post_json(&format!("{base}/api/acp/connect"), &body, Some(on_response)) {
        let mut b = inner.borrow_mut();
        apply_acp_agent_connect_error(
            b.host_mut().editor_state_mut(),
            &id,
            "ACP agent connection request could not start. Is the web daemon running?",
        );
        b.host_mut().mark_editor_state_dirty();
        let _ = b.repaint();
        console_warn("ACP agent connect request could not start");
    }
}

pub(crate) fn apply_acp_agent_connect_response(
    state: &mut EditorState,
    id: &str,
    body: &str,
) -> bool {
    if !state.editor_ui.agent_settings.acp_agent_probe_in_flight(id) {
        return false;
    }
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            apply_acp_agent_connect_error(
                state,
                id,
                "ACP agent connection failed: invalid daemon response",
            );
            return true;
        }
    };
    let connected = parsed
        .get("connected")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let info = str_field(&parsed, "connectionInfo").or_else(|| str_field(&parsed, "info"));
    let error = str_field(&parsed, "error");
    state
        .editor_ui
        .agent_settings
        .apply_acp_agent_connect_outcome(
            id,
            AcpAgentConnectOutcome {
                connected,
                info,
                error,
            },
        );
    state.rebuild_chat_models();
    true
}

fn apply_acp_agent_connect_error(state: &mut EditorState, id: &str, error: &str) {
    state
        .editor_ui
        .agent_settings
        .apply_acp_agent_connect_outcome(
            id,
            AcpAgentConnectOutcome {
                connected: false,
                error: Some(error.to_string()),
                ..Default::default()
            },
        );
    state.rebuild_chat_models();
}

fn str_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::agent_settings::{AcpAgentConnectPhase, AcpConnectionType};
    use std::collections::BTreeMap;

    fn state_with_pending_acp() -> EditorState {
        let mut state = EditorState::new();
        state.editor_ui.agent_settings.add_acp_agent_config(
            "Claude Code",
            AcpConnectionType::Local,
            "claude",
            Vec::new(),
            BTreeMap::new(),
            None,
            true,
        );
        state.editor_ui.agent_settings.begin_acp_agent_connect(0);
        state
    }

    #[test]
    fn web_acp_connect_response_marks_agent_connected() {
        let mut state = state_with_pending_acp();
        let body = serde_json::json!({
            "connected": true,
            "connectionInfo": "Claude Code 1.0"
        })
        .to_string();

        assert!(apply_acp_agent_connect_response(&mut state, "acp-1", &body));

        let settings = &state.editor_ui.agent_settings;
        assert!(settings.acp_agents[0].connected);
        let conn = settings.acp_agent_connection_for("acp-1");
        assert_eq!(conn.phase, AcpAgentConnectPhase::Connected);
        assert_eq!(conn.info.as_deref(), Some("Claude Code 1.0"));
    }

    #[test]
    fn web_acp_connect_response_failure_keeps_agent_disconnected() {
        let mut state = state_with_pending_acp();
        let body = serde_json::json!({
            "connected": false,
            "error": "failed to spawn ACP agent"
        })
        .to_string();

        assert!(apply_acp_agent_connect_response(&mut state, "acp-1", &body));

        let settings = &state.editor_ui.agent_settings;
        assert!(!settings.acp_agents[0].connected);
        let conn = settings.acp_agent_connection_for("acp-1");
        assert_eq!(conn.phase, AcpAgentConnectPhase::Error);
        assert_eq!(conn.error.as_deref(), Some("failed to spawn ACP agent"));
    }
}
