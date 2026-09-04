//! Hermetic contracts for the native mobile codegen provider.

use super::*;
use std::time::Instant;

use op_ai::chat_provider::EffortLevel;
use op_editor_core::{AgentProvider, ModelEntry};
use serde_json::json;

fn state_with_builtin(base_url: &str) -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Mobile Test",
        "sk-super-secret-mobile",
        "test-code-model",
        BuiltinAgentKind::OpenAiCompat,
        base_url,
    );
    state.rebuild_chat_models();
    state.chat.selected_model = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.is_some())
        .expect("ready built-in model row");
    state
}

#[test]
fn selected_ready_builtin_is_bound_without_exposing_its_key() {
    let state = state_with_builtin("https://example.com/v1");
    let provider = MobileBuiltinProvider::from_selected_model(&state).expect("provider");

    assert_eq!(provider.model, "test-code-model");
    assert_eq!(provider.label, "Mobile Test");
    assert!(provider.supports_cancellable_send());
    assert!(provider.supports_evidence_only_send());
    let debug = format!("{provider:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("sk-super-secret-mobile"));
}

#[test]
fn external_model_never_falls_back_to_a_mobile_builtin() {
    let mut state = state_with_builtin("https://example.com/v1");
    state.chat.available_models = vec![ModelEntry::new(
        AgentProvider::ClaudeCode,
        "claude-cli",
        "Claude CLI",
    )];
    state.chat.selected_model = 0;

    assert!(matches!(
        MobileBuiltinProvider::from_selected_model(&state),
        Err(MobileBuiltinProviderError::ExternalModel { .. })
    ));
}

#[test]
fn stale_catalog_row_cannot_borrow_a_disabled_or_different_model_config() {
    let mut state = state_with_builtin("https://example.com/v1");
    state.editor_ui.agent_settings.builtin_agents[0].enabled = false;
    assert!(matches!(
        MobileBuiltinProvider::from_selected_model(&state),
        Err(MobileBuiltinProviderError::ProviderNotReady { .. })
    ));

    state.editor_ui.agent_settings.builtin_agents[0].enabled = true;
    state.editor_ui.agent_settings.builtin_agents[0].models = vec!["another-model".into()];
    assert!(matches!(
        MobileBuiltinProvider::from_selected_model(&state),
        Err(MobileBuiltinProviderError::ModelNotSaved { .. })
    ));
}

#[test]
fn endpoint_rejects_embedded_credentials_query_and_non_http_schemes() {
    assert!(matches!(
        normalize_provider_base_url("https://user:pass@example.com/v1"),
        Err(MobileBuiltinProviderError::EndpointHasUserInfo)
    ));
    assert!(matches!(
        normalize_provider_base_url("https://example.com/v1?token=secret"),
        Err(MobileBuiltinProviderError::EndpointHasQueryOrFragment)
    ));
    assert!(matches!(
        normalize_provider_base_url("file:///tmp/provider"),
        Err(MobileBuiltinProviderError::UnsupportedEndpointScheme { .. })
    ));
}

#[test]
fn default_low_effort_does_not_override_reasoning_or_anthropic_system_shape() {
    let request = ChatRequest {
        system_prompt: "codegen-system".into(),
        user_message: "generate".into(),
        history: vec![(ChatHistoryRole::Assistant, "prior".into())],
        effort: EffortLevel::Low,
        thinking: ThinkingMode::Adaptive,
        ..ChatRequest::default()
    };
    assert!(!reduce_reasoning(&request));

    let openai = openai_request_body(&request, "deepseek-v4");
    assert_eq!(openai.pointer("/messages/0/role"), Some(&json!("system")));
    assert!(openai.get("thinking").is_none());
    assert!(openai.get("reasoning_effort").is_none());

    let anthropic = anthropic_request_body(&request, "deepseek-v4");
    assert_eq!(anthropic.get("system"), Some(&json!("codegen-system")));
    assert!(anthropic.get("thinking").is_none());
    assert!(anthropic
        .get("messages")
        .and_then(Value::as_array)
        .expect("anthropic messages")
        .iter()
        .all(|message| message.get("role") != Some(&json!("system"))));
}

#[test]
fn explicit_disabled_thinking_uses_each_supported_wire_control() {
    let request = ChatRequest {
        thinking: ThinkingMode::Disabled,
        ..ChatRequest::default()
    };
    assert!(reduce_reasoning(&request));
    assert_eq!(
        openai_request_body(&request, "deepseek-v4").pointer("/thinking/type"),
        Some(&json!("disabled"))
    );
    assert_eq!(
        anthropic_request_body(&request, "deepseek-v4").pointer("/thinking/type"),
        Some(&json!("disabled"))
    );
}

#[test]
fn cancel_unblocks_a_silent_async_transport_and_aborts_it() {
    let (tx, rx) = mpsc::channel::<ChatDelta>();
    let task = op_chat_agent::runtime::shared_runtime().spawn(async move {
        let _keep_sender_alive = tx;
        std::future::pending::<()>().await;
    });
    let abort_probe = task.abort_handle();
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancel);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        signal.store(true, Ordering::Release);
    });

    let started = Instant::now();
    let mut iter = CancellableRecvIter::new(rx, cancel, abort_probe.clone());
    assert!(iter.next().is_none());
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "cancellation waited for the silent provider"
    );
    let deadline = Instant::now() + Duration::from_millis(500);
    while !abort_probe.is_finished() && Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(abort_probe.is_finished(), "provider task was not aborted");
}

#[test]
fn attachment_request_fails_locally_without_starting_paid_work() {
    let state = state_with_builtin("https://example.com/v1");
    let provider = MobileBuiltinProvider::from_selected_model(&state).expect("provider");
    let request = ChatRequest {
        attachments: vec![op_ai::chat_provider::ChatAttachment {
            name: "shot.png".into(),
            media_type: "image/png".into(),
            data: vec![1, 2, 3],
        }],
        ..ChatRequest::default()
    };
    let deltas: Vec<_> = provider.send(request).collect();
    assert!(matches!(deltas.first(), Some(ChatDelta::Error(_))));
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::Aborted
        })
    ));
}
