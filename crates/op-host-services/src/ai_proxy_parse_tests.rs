//! Body-parsing / SSE-framing / provider-selection tests for
//! `ai_proxy.rs`, carved off into a sibling file to keep both under the
//! 800-line cap. Wired in as `ai_proxy::tests` via `#[path]` so
//! `use super::*` still resolves against `ai_proxy` itself.

use super::*;
use op_ai::chat_provider::{EchoProvider, StopReason};
use std::sync::{Arc, Mutex};

struct CaptureModelProvider {
    seen_model: Arc<Mutex<Option<Option<String>>>>,
}

impl ChatProvider for CaptureModelProvider {
    fn provider_label(&self) -> &str {
        "capture"
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        *self.seen_model.lock().expect("seen model lock") = Some(request.model);
        Box::new(
            vec![ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            }]
            .into_iter(),
        )
    }
}

#[test]
fn parse_ai_stream_body_maps_full_body() {
    let body = r#"{"provider":"grok-build","builtinProviderId":"account:7","model":"m","skills":["codegen-planning"],"user":"hi","max_output_tokens":2000,"thinking":"adaptive","effort":"low"}"#;
    let req = parse_ai_stream_body(body).expect("body parses");
    assert_eq!(req.provider, Some(AgentProvider::GrokBuild));
    assert_eq!(req.builtin_provider_id.as_deref(), Some("account:7"));
    assert_eq!(req.model, "m");
    assert_eq!(req.skills, vec!["codegen-planning".to_string()]);
    assert_eq!(req.user, "hi");
    assert_eq!(req.max_output_tokens, 2000);
    assert_eq!(req.thinking, ThinkingMode::Adaptive);
    assert_eq!(req.effort, EffortLevel::Low);
}

#[test]
fn parse_ai_stream_body_defaults_missing_and_unknown_knobs() {
    // No thinking/effort → Adaptive/Low; unknown tokens also fall
    // back to the defaults.
    let req = parse_ai_stream_body(r#"{"user":"hi"}"#).expect("body parses");
    assert_eq!(req.thinking, ThinkingMode::Adaptive);
    assert_eq!(req.effort, EffortLevel::Low);
    assert_eq!(req.max_output_tokens, 4096);
    assert!(req.skills.is_empty());

    let req2 =
        parse_ai_stream_body(r#"{"thinking":"weird","effort":"ludicrous"}"#).expect("parses");
    assert_eq!(req2.thinking, ThinkingMode::Adaptive);
    assert_eq!(req2.effort, EffortLevel::Low);
}

#[test]
fn parse_ai_stream_body_reads_thinking_and_effort_variants() {
    let req = parse_ai_stream_body(r#"{"thinking":"disabled","effort":"high"}"#).expect("parses");
    assert_eq!(req.thinking, ThinkingMode::Disabled);
    assert_eq!(req.effort, EffortLevel::High);
    let req2 = parse_ai_stream_body(r#"{"thinking":"enabled","effort":"max"}"#).expect("parses");
    assert_eq!(req2.thinking, ThinkingMode::Enabled);
    assert_eq!(req2.effort, EffortLevel::Max);
}

#[test]
fn parse_ai_stream_body_rejects_non_object() {
    assert!(parse_ai_stream_body("[]").is_none());
    assert!(parse_ai_stream_body("not json").is_none());
    assert!(parse_ai_stream_body(r#"{"provider":"unknown"}"#).is_none());
    assert!(parse_ai_stream_body(r#"{"builtinProviderId":7}"#).is_none());
}

#[test]
fn request_scoped_builtin_credential_builds_a_provider_without_mutating_settings() {
    let body = serde_json::json!({
        "model": "private-model",
        "user": "generate",
        "credential": {
            "id": "builtin-web-1",
            "preset": "openai",
            "display_name": "Private",
            "kind": "openai-compat",
            "api_key": "sk-transient",
            "model": "private-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }
    })
    .to_string();
    let request = parse_ai_stream_body(&body).expect("request parses");
    let state = EditorState::new();

    let provider = proxy_provider_for_request(
        &state,
        &request,
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
    )
    .expect("public built-in endpoint is allowed");

    assert!(provider.is_some());
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
}

#[test]
fn request_scoped_custom_endpoint_is_rejected_by_browser_only_policy() {
    let body = serde_json::json!({
        "model": "private-model",
        "user": "generate",
        "credential": {
            "id": "builtin-web-1",
            "preset": "custom",
            "display_name": "Private",
            "kind": "openai-compat",
            "api_key": "sk-transient",
            "model": "private-model",
            "base_url": "http://127.0.0.1:8080/v1",
            "enabled": true
        }
    })
    .to_string();
    let request = parse_ai_stream_body(&body).expect("request parses");

    assert!(proxy_provider_for_request(
        &EditorState::new(),
        &request,
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
    )
    .is_err());
}

#[test]
fn server_persistence_does_not_allow_a_loopback_request_endpoint() {
    let body = serde_json::json!({
        "model": "private-model",
        "user": "generate",
        "credential": {
            "id": "builtin-web-1",
            "preset": "custom",
            "display_name": "Private",
            "kind": "openai-compat",
            "api_key": "sk-transient",
            "model": "private-model",
            "base_url": "http://127.0.0.1:8080/v1",
            "enabled": true
        }
    })
    .to_string();
    let request = parse_ai_stream_body(&body).expect("request parses");

    assert!(proxy_provider_for_request(
        &EditorState::new(),
        &request,
        crate::web_credential_policy::WebCredentialPersistence::Server,
    )
    .is_err());
}

#[test]
fn delta_to_sse_frames_text_delta() {
    assert_eq!(
        delta_to_sse(&ChatDelta::TextDelta("hi".into())),
        "data: {\"delta\":\"hi\"}\n\n"
    );
}

#[test]
fn delta_to_sse_escapes_quotes_into_valid_json() {
    // A delta carrying a quote must produce valid JSON, not a raw
    // `"` that breaks the event payload.
    let framed = delta_to_sse(&ChatDelta::TextDelta("a\"b".into()));
    let data = framed
        .strip_prefix("data: ")
        .and_then(|s| s.strip_suffix("\n\n"))
        .expect("frame shape");
    let value: Value = serde_json::from_str(data).expect("payload is valid JSON");
    assert_eq!(value["delta"], "a\"b");
}

#[test]
fn delta_to_sse_frames_done() {
    assert_eq!(
        delta_to_sse(&ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        }),
        "data: {\"done\":true}\n\n"
    );
}

#[test]
fn delta_to_sse_frames_error() {
    assert_eq!(
        delta_to_sse(&ChatDelta::Error("x".into())),
        "data: {\"error\":\"x\"}\n\n"
    );
}

#[test]
fn stream_ai_response_writes_headers_and_deltas() {
    let req = AiStreamRequest {
        provider: None,
        builtin_provider_id: None,
        model: "m".into(),
        // Real corpus loads server-side; the EchoProvider ignores
        // the system prompt but the expansion must not error.
        skills: vec!["codegen-planning".into()],
        user: "hi".into(),
        max_output_tokens: 2000,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
        transient_builtin: None,
    };
    let provider = EchoProvider {
        script: vec![
            ChatDelta::TextDelta("Hello".into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ],
    };
    let mut out = Vec::<u8>::new();
    stream_ai_response(&mut out, req, &provider, Some("*")).expect("stream");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains("Content-Type: text/event-stream"), "{text}");
    assert!(text.contains(r#"data: {"delta":"Hello"}"#), "{text}");
    assert!(text.contains(r#"data: {"done":true}"#), "{text}");
}

#[test]
fn stream_ai_response_forwards_requested_model_to_provider() {
    let req = AiStreamRequest {
        provider: None,
        builtin_provider_id: None,
        model: "claude-sonnet-4-6".into(),
        skills: vec![],
        user: "hi".into(),
        max_output_tokens: 128,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
        transient_builtin: None,
    };
    let seen_model = Arc::new(Mutex::new(None));
    let provider = CaptureModelProvider {
        seen_model: seen_model.clone(),
    };
    let mut out = Vec::<u8>::new();

    stream_ai_response(&mut out, req, &provider, Some("*")).expect("stream");

    assert_eq!(
        seen_model.lock().expect("seen model lock").as_ref(),
        Some(&Some("claude-sonnet-4-6".to_string()))
    );
}

#[test]
fn stream_ai_response_stops_after_error() {
    // An Error delta is terminal — nothing written after it even if
    // the script has trailing deltas.
    let req = AiStreamRequest {
        provider: None,
        builtin_provider_id: None,
        model: "m".into(),
        skills: vec![],
        user: "x".into(),
        max_output_tokens: 64,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
        transient_builtin: None,
    };
    let provider = EchoProvider {
        script: vec![
            ChatDelta::Error("boom".into()),
            ChatDelta::TextDelta("should not appear".into()),
        ],
    };
    let mut out = Vec::<u8>::new();
    stream_ai_response(&mut out, req, &provider, Some("*")).expect("stream");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains(r#"data: {"error":"boom"}"#), "{text}");
    assert!(!text.contains("should not appear"), "{text}");
}

#[test]
fn write_sse_error_emits_headers_and_error() {
    let mut out = Vec::<u8>::new();
    write_sse_error(&mut out, "no model configured", Some("*")).expect("write");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains("Content-Type: text/event-stream"), "{text}");
    assert!(
        text.contains(r#"data: {"error":"no model configured"}"#),
        "{text}"
    );
}

#[test]
fn proxy_provider_none_without_configured_agent() {
    let editor = EditorState::new();
    assert!(proxy_provider(&editor, "anything").is_none());
}

#[test]
fn proxy_provider_builds_from_ready_builtin_agent() {
    let mut editor = EditorState::new();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Built-in Claude", "sk-test", "claude-sonnet-4-5");
    let provider = proxy_provider(&editor, "claude-sonnet-4-5").expect("provider builds");
    assert_eq!(provider.provider_label(), "Built-in Claude");
}

#[test]
fn proxy_provider_rejects_an_unmatched_legacy_model() {
    let mut editor = EditorState::new();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Built-in Claude", "sk-test", "claude-sonnet-4-5");
    assert!(proxy_provider(&editor, "no-such-model").is_none());
}

#[test]
fn models_json_lists_ready_agent_models() {
    let mut editor = EditorState::new();
    assert_eq!(models_json(&editor), "[]");
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Built-in Claude", "sk-test", "claude-sonnet-4-5");
    let json = models_json(&editor);
    let parsed: Vec<Value> = serde_json::from_str(&json).expect("valid model array");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["displayName"], "claude-sonnet-4-5");
    assert_eq!(parsed[0]["providerDisplayName"], "Built-in Claude");
    assert!(parsed[0]["value"]
        .as_str()
        .is_some_and(|value| value.ends_with(":claude-sonnet-4-5")));
}

#[test]
fn models_json_excludes_verified_cli_models() {
    let mut editor = EditorState::new();
    editor.chat.discovered_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "claude-sonnet-4-6",
        "Claude Sonnet 4.6",
    )];
    editor
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            op_editor_core::AgentProvider::ClaudeCode,
            op_editor_core::ProviderConnectOutcome {
                connected: true,
                info: Some("Connected via Claude Code".into()),
                ..Default::default()
            },
        );
    editor.rebuild_chat_models();

    let json = models_json(&editor);
    let parsed: Value = serde_json::from_str(&json).expect("valid json array");
    let arr = parsed.as_array().expect("array");
    assert!(arr.is_empty(), "{json}");
}

#[test]
fn models_json_excludes_unverified_cli_models() {
    let mut editor = EditorState::new();
    editor.chat.discovered_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "claude-sonnet-4-6",
        "Claude Sonnet 4.6",
    )];
    editor.editor_ui.agent_settings.connected[0] = true;
    editor.rebuild_chat_models();

    assert_eq!(models_json(&editor), "[]");
    assert!(proxy_provider(&editor, "claude-sonnet-4-6").is_none());
}

#[test]
fn proxy_provider_rejects_verified_cli_model() {
    let mut editor = EditorState::new();
    editor.chat.discovered_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "claude-sonnet-4-6",
        "Claude Sonnet 4.6",
    )];
    editor
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            op_editor_core::AgentProvider::ClaudeCode,
            op_editor_core::ProviderConnectOutcome {
                connected: true,
                info: Some("Connected via Claude Code".into()),
                ..Default::default()
            },
        );
    editor.rebuild_chat_models();

    assert!(proxy_provider(&editor, "claude-sonnet-4-6").is_none());
}

#[test]
fn explicit_cli_provider_remains_unavailable_to_web_proxy() {
    let mut editor = EditorState::new();
    editor.chat.discovered_models = vec![
        op_editor_core::ModelEntry::new(AgentProvider::Antigravity, "default", "Default"),
        op_editor_core::ModelEntry::new(AgentProvider::GrokBuild, "default", "Default"),
    ];
    for provider in [AgentProvider::Antigravity, AgentProvider::GrokBuild] {
        editor
            .editor_ui
            .agent_settings
            .apply_provider_connect_outcome(
                provider,
                op_editor_core::ProviderConnectOutcome {
                    connected: true,
                    info: Some("Connected".into()),
                    ..Default::default()
                },
            );
    }
    editor.rebuild_chat_models();

    let request =
        parse_ai_stream_body(r#"{"provider":"grok-build","model":"default","user":"hi"}"#)
            .expect("request parses");
    let provider = proxy_provider_for_request_with_chat_session(
        &editor,
        &request,
        true,
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
    )
    .expect("request is valid");
    assert!(provider.is_none());
    assert!(proxy_provider(&editor, "default").is_none());
}
