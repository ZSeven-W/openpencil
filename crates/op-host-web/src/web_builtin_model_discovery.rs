//! Browser bridge for request-scoped built-in provider model discovery.

use std::cell::RefCell;
use std::rc::Rc;

use op_editor_core::{
    BuiltinAgentConfig, BuiltinModelCatalogRefreshOutcome, BuiltinModelCatalogRefreshRequest,
    EditorState,
};
use wasm_bindgen::JsValue;

use crate::repaint_ctx::RepaintContext;

type InnerRc<C> = Rc<RefCell<C>>;

fn console_warn(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
}

fn identity_is_current(issued_epoch: u64) -> bool {
    issued_epoch == crate::identity_epoch::epoch()
}

/// Drain every picker-triggered refresh. Each callback carries both the
/// account epoch and the core request generation/config snapshot, so a result
/// from an old account or an edited credential can never populate the picker.
pub(crate) fn drain_pending_builtin_model_discovery<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
) {
    loop {
        let request = {
            let mut context = inner.borrow_mut();
            context
                .host_mut()
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .take_pending_builtin_model_catalog_refresh()
        };
        let Some(request) = request else {
            break;
        };
        let expected = inner
            .borrow()
            .host()
            .editor_state()
            .editor_ui
            .agent_settings
            .builtin_model_catalog_config_for_request(&request);
        let Some(expected) = expected else {
            inner
                .borrow_mut()
                .host_mut()
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .invalidate_builtin_model_catalog(&request.target);
            continue;
        };
        start_request(inner, request, expected);
    }
}

fn start_request<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
    request: BuiltinModelCatalogRefreshRequest,
    expected: BuiltinAgentConfig,
) {
    let body = discovery_request_json(&request, &expected);
    let issued_epoch = crate::identity_epoch::epoch();
    let request_for_response = request.clone();
    let expected_for_response = expected.clone();
    let inner_for_response = inner.clone();
    let callback: Rc<dyn Fn(u16, String)> = Rc::new(move |status, body| {
        if !identity_is_current(issued_epoch) {
            return;
        }
        let mut context = inner_for_response.borrow_mut();
        if apply_response(
            context.host_mut().editor_state_mut(),
            &expected_for_response,
            &request_for_response,
            status,
            &body,
        ) {
            context.host_mut().mark_editor_state_dirty();
            let _ = context.repaint();
        }
    });
    let url = format!(
        "{}/api/ai/models/discover",
        crate::daemon_base::daemon_base()
    );
    if !crate::live_sync::post_json_with_status(&url, &body, callback) {
        let mut context = inner.borrow_mut();
        if apply_error(
            context.host_mut().editor_state_mut(),
            &expected,
            &request,
            "Model discovery request could not start. Is the web daemon running?",
        ) {
            context.host_mut().mark_editor_state_dirty();
            let _ = context.repaint();
        }
        console_warn("built-in model discovery request could not start");
    }
}

fn discovery_request_json(
    request: &BuiltinModelCatalogRefreshRequest,
    expected: &BuiltinAgentConfig,
) -> String {
    serde_json::json!({
        "id": &expected.id,
        "generation": request.generation,
        "credential": crate::web_ai_credentials::builtin_credential(expected),
    })
    .to_string()
}

pub(crate) fn apply_response(
    state: &mut EditorState,
    expected: &BuiltinAgentConfig,
    request: &BuiltinModelCatalogRefreshRequest,
    status: u16,
    body: &str,
) -> bool {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            return apply_error(
                state,
                expected,
                request,
                "Model discovery failed: invalid daemon response",
            );
        }
    };
    if parsed.get("id").and_then(|value| value.as_str()) != Some(expected.id.as_str())
        || parsed.get("generation").and_then(|value| value.as_u64()) != Some(request.generation)
    {
        // Origin/content-type/auth gates return generic JSON without the
        // discovery echo fields. The callback is already request-scoped and
        // core still validates generation + credential fingerprint, so land a
        // sanitized error instead of leaving this catalog stuck in Loading.
        return apply_error(
            state,
            expected,
            request,
            "Model discovery failed: invalid daemon response",
        );
    }
    if !(200..300).contains(&status)
        || parsed.get("ok").and_then(|value| value.as_bool()) != Some(true)
    {
        let error = parsed
            .get("error")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("Model discovery failed");
        let outcome = if parsed
            .get("unsupported")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            BuiltinModelCatalogRefreshOutcome::Unsupported {
                error: Some(error.to_string()),
            }
        } else {
            BuiltinModelCatalogRefreshOutcome::Error {
                error: error.to_string(),
            }
        };
        return apply_outcome(state, expected, request, outcome);
    }
    let models = parsed
        .get("models")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let id = model.get("id")?.as_str()?.to_string();
            let display_name = model
                .get("displayName")
                .or_else(|| model.get("display_name"))
                .and_then(|value| value.as_str())
                .unwrap_or(&id)
                .to_string();
            Some(op_editor_core::BuiltinModelOption { id, display_name })
        })
        .collect();
    apply_outcome(
        state,
        expected,
        request,
        BuiltinModelCatalogRefreshOutcome::Success { models },
    )
}

fn apply_error(
    state: &mut EditorState,
    expected: &BuiltinAgentConfig,
    request: &BuiltinModelCatalogRefreshRequest,
    error: &str,
) -> bool {
    apply_outcome(
        state,
        expected,
        request,
        BuiltinModelCatalogRefreshOutcome::Error {
            error: error.to_string(),
        },
    )
}

fn apply_outcome(
    state: &mut EditorState,
    expected: &BuiltinAgentConfig,
    request: &BuiltinModelCatalogRefreshRequest,
    outcome: BuiltinModelCatalogRefreshOutcome,
) -> bool {
    state
        .editor_ui
        .agent_settings
        .apply_builtin_model_catalog_refresh_outcome_if_current(expected, request, outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_state() -> (
        EditorState,
        BuiltinAgentConfig,
        BuiltinModelCatalogRefreshRequest,
    ) {
        let mut state = EditorState::new();
        state.editor_ui.agent_settings.add_builtin_agent_config(
            "Provider",
            "sk-selected",
            "fallback-model",
            op_editor_core::BuiltinAgentKind::OpenAiCompat,
            "https://api.openai.com/v1",
        );
        let settings = &mut state.editor_ui.agent_settings;
        settings.request_ready_builtin_model_catalog_refreshes(1);
        let request = settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("catalog request");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("provider snapshot");
        (state, expected, request)
    }

    #[test]
    fn request_body_contains_only_the_target_credential() {
        let (mut state, expected, request) = pending_state();
        state.editor_ui.agent_settings.add_builtin_agent_config(
            "Other",
            "sk-must-not-leak",
            "other-model",
            op_editor_core::BuiltinAgentKind::OpenAiCompat,
            "https://other.example/v1",
        );

        let body = discovery_request_json(&request, &expected);
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("request JSON");

        assert_eq!(parsed["credential"]["api_key"], "sk-selected");
        assert!(!body.contains("sk-must-not-leak"));
        assert!(parsed.get("credentials").is_none());
    }

    #[test]
    fn successful_response_lands_runtime_options_and_display_names() {
        let (mut state, expected, request) = pending_state();
        state.rebuild_chat_models();
        let chat_before = state.chat.available_models.clone();
        let body = serde_json::json!({
            "ok": true,
            "id": &expected.id,
            "generation": request.generation,
            "models": [
                {"id": "remote-a", "displayName": "Remote A"},
                {"id": "remote-b", "display_name": "Remote B"},
            ],
        })
        .to_string();

        assert!(apply_response(&mut state, &expected, &request, 200, &body,));
        assert_eq!(
            state
                .editor_ui
                .agent_settings
                .builtin_model_catalog_options(&expected.id),
            &[
                op_editor_core::BuiltinModelOption::new("remote-a", "Remote A"),
                op_editor_core::BuiltinModelOption::new("remote-b", "Remote B"),
            ]
        );
        assert_eq!(state.chat.available_models, chat_before);
        assert!(!state
            .chat
            .available_models
            .iter()
            .any(|entry| entry.builtin_model_id() == Some("remote-a")));
    }

    #[test]
    fn response_for_changed_credentials_is_ignored() {
        let (mut state, expected, request) = pending_state();
        state.editor_ui.agent_settings.builtin_agents[0].api_key = "sk-new".into();
        let body = serde_json::json!({
            "ok": true,
            "id": &expected.id,
            "generation": request.generation,
            "models": [{"id": "remote", "displayName": "Remote"}],
        })
        .to_string();

        assert!(!apply_response(&mut state, &expected, &request, 200, &body,));
        assert!(state
            .editor_ui
            .agent_settings
            .builtin_model_catalog_options(&expected.id)
            .is_empty());
    }

    #[test]
    fn response_with_another_generation_exits_loading_as_error() {
        let (mut state, expected, request) = pending_state();
        let body = serde_json::json!({
            "ok": true,
            "id": &expected.id,
            "generation": request.generation + 1,
            "models": [{"id": "remote"}],
        })
        .to_string();

        assert!(apply_response(&mut state, &expected, &request, 200, &body,));
        assert_eq!(
            state
                .editor_ui
                .agent_settings
                .builtin_model_catalogs
                .get(&request.target)
                .map(|catalog| catalog.phase),
            Some(op_editor_core::BuiltinModelCatalogPhase::Error)
        );
    }

    #[test]
    fn gateway_rejection_without_echo_exits_loading() {
        let (mut state, expected, request) = pending_state();

        assert!(apply_response(
            &mut state,
            &expected,
            &request,
            403,
            r#"{"ok":false,"error":"forbidden"}"#,
        ));
        assert_eq!(
            state
                .editor_ui
                .agent_settings
                .builtin_model_catalogs
                .get(&request.target)
                .map(|catalog| catalog.phase),
            Some(op_editor_core::BuiltinModelCatalogPhase::Error)
        );
    }

    #[test]
    fn identity_epoch_rejects_a_previous_accounts_callback() {
        crate::identity_epoch::reset_for_test();
        crate::identity_epoch::observe_subject(Some("alice"));
        let issued_epoch = crate::identity_epoch::epoch();
        assert!(identity_is_current(issued_epoch));

        crate::identity_epoch::observe_subject(Some("bob"));

        assert!(!identity_is_current(issued_epoch));
    }
}
