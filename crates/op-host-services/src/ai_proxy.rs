//! Backend AI proxy for the WASM web bundle.
//!
//! The browser shell (`op-host-web`) can't bundle the skill corpus or run
//! native providers, so it POSTs a model request here (with an optional
//! request-scoped browser credential) and the daemon expands skill NAMES via
//! `op_ai_skills::compose_system_prompt`, then streams the
//! provider's `ChatDelta`s back as Server-Sent Events. The framing and
//! skill-expansion live here (pure + tested); `web_canvas_server.rs`
//! only routes `/api/ai/stream` (POST → SSE) and `/api/ai/models`
//! (GET → JSON) to it.

use std::io::Write;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, ThinkingMode};
use op_editor_core::{AgentProvider, BuiltinAgentConfig, EditorState};
use serde_json::{json, Value};

use crate::chat_builtin_http::ConfiguredBuiltinProvider;

/// Parsed `POST /api/ai/stream` body. The web bundle sends skill NAMES
/// (not the corpus) plus the per-turn knobs; the proxy expands the
/// names server-side.
pub struct AiStreamRequest {
    /// Explicit provider identity from the structured model catalog. Older
    /// clients omit it and use the ambiguity-safe legacy resolver.
    pub provider: Option<AgentProvider>,
    pub model: String,
    pub skills: Vec<String>,
    pub user: String,
    pub max_output_tokens: u32,
    pub thinking: ThinkingMode,
    pub effort: EffortLevel,
    pub transient_builtin: Option<BuiltinAgentConfig>,
}

/// Parse a `/api/ai/stream` JSON body into an [`AiStreamRequest`].
/// Returns `None` when the body isn't a JSON object. Missing scalar
/// fields fall back to sensible defaults (empty model/user, no skills,
/// 4096 output tokens); `thinking` / `effort` strings map onto their
/// enums, defaulting to `Adaptive` / `Low` (TS parity) when missing or
/// unknown.
pub fn parse_ai_stream_body(body: &str) -> Option<AiStreamRequest> {
    let value: Value = serde_json::from_str(body).ok()?;
    let obj = value.as_object()?;
    let model = obj
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let provider = match obj.get("provider") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(AgentProvider::from_wire_id(value)?),
        Some(_) => return None,
    };
    let user = obj
        .get("user")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let skills = obj
        .get("skills")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let max_output_tokens = obj
        .get("max_output_tokens")
        .and_then(Value::as_u64)
        .map(|n| n.min(u32::MAX as u64) as u32)
        .unwrap_or(4096);
    let thinking = obj
        .get("thinking")
        .and_then(Value::as_str)
        .map(parse_thinking)
        .unwrap_or_default();
    let effort = obj
        .get("effort")
        .and_then(Value::as_str)
        .map(parse_effort)
        .unwrap_or_default();
    let transient_builtin = match obj.get("credential") {
        None | Some(Value::Null) => None,
        Some(value) => Some(crate::web_credentials::parse_transient_builtin(value)?),
    };
    Some(AiStreamRequest {
        provider,
        model,
        skills,
        user,
        max_output_tokens,
        thinking,
        effort,
        transient_builtin,
    })
}

/// Map a wire `thinking` token onto [`ThinkingMode`]; unknown → default
/// (`Adaptive`).
fn parse_thinking(s: &str) -> ThinkingMode {
    match s {
        "disabled" => ThinkingMode::Disabled,
        "enabled" => ThinkingMode::Enabled,
        "adaptive" => ThinkingMode::Adaptive,
        _ => ThinkingMode::default(),
    }
}

/// Map a wire `effort` token onto [`EffortLevel`]; unknown → default
/// (`Low`).
fn parse_effort(s: &str) -> EffortLevel {
    match s {
        "medium" => EffortLevel::Medium,
        "high" => EffortLevel::High,
        "max" => EffortLevel::Max,
        "low" => EffortLevel::Low,
        _ => EffortLevel::default(),
    }
}

/// Frame one [`ChatDelta`] as a single SSE event line (trailing blank
/// line included). Uses `serde_json` so payloads with quotes / newlines
/// stay valid JSON. `TextDelta` → `data: {"delta":"…"}`; `Done` →
/// `data: {"done":true}`; `Error` → `data: {"error":"…"}`. `Thinking`
/// / `ToolUse` are framed too so a richer web client can render them,
/// but the web bundle only needs delta/done/error today.
pub fn delta_to_sse(delta: &ChatDelta) -> String {
    let payload = match delta {
        ChatDelta::TextDelta(s) => json!({ "delta": s }),
        ChatDelta::Thinking(s) => json!({ "thinking": s }),
        ChatDelta::ToolUse { name, args } => json!({ "tool": name, "args": args }),
        ChatDelta::Done { .. } => json!({ "done": true }),
        ChatDelta::Error(s) => json!({ "error": s }),
    };
    format!("data: {payload}\n\n")
}

/// Expand `req.skills` into a system prompt, build the `ChatRequest`,
/// and stream the provider's deltas to `out` as SSE. Generic over
/// `Write` so it's testable over a `Vec<u8>`. Writes the SSE header
/// block first (copied from `web_canvas_server::serve_sse`), then one
/// `data:` event per delta, flushing after each, stopping after the
/// terminal `Done` / `Error`.
pub fn stream_ai_response<W: Write>(
    out: &mut W,
    req: AiStreamRequest,
    provider: &dyn ChatProvider,
) -> std::io::Result<()> {
    write_sse_headers(out)?;
    // Expand skill NAMES → system prompt server-side (the web bundle
    // never ships the corpus). 0 budget = unlimited; the proxy trusts
    // the caller's skill list.
    let skill_refs: Vec<&str> = req.skills.iter().map(String::as_str).collect();
    let system = op_ai_skills::compose_system_prompt(&skill_refs, 0);
    let chat_req = ChatRequest {
        system_prompt: system,
        user_message: req.user,
        // Proxy requests are self-contained — no chat history wire yet.
        history: Vec::new(),
        max_output_tokens: req.max_output_tokens,
        thinking: req.thinking,
        effort: req.effort,
        attachments: vec![],
        model: request_model_id(&req.model),
    };
    for delta in provider.send(chat_req) {
        out.write_all(delta_to_sse(&delta).as_bytes())?;
        out.flush()?;
        if matches!(delta, ChatDelta::Done { .. } | ChatDelta::Error(_)) {
            break;
        }
    }
    Ok(())
}

fn request_model_id(model: &str) -> Option<String> {
    let model = model.trim();
    if model.is_empty() || model == "default" || parse_builtin_model_key(model).is_some() {
        None
    } else {
        Some(model.to_string())
    }
}

/// Write the SSE header block — same shape as
/// `web_canvas_server::serve_sse` (200 OK + `text/event-stream` +
/// no-cache + keep-alive + permissive CORS + blank line).
pub fn write_sse_headers<W: Write>(out: &mut W) -> std::io::Result<()> {
    let headers = "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: keep-alive\r\n\
         Access-Control-Allow-Origin: *\r\n\r\n";
    out.write_all(headers.as_bytes())?;
    out.flush()
}

/// Write a single SSE error event (with headers) and close — used by
/// the route when no provider could be built. `flush` after so the
/// client sees the error before the socket drops.
pub fn write_sse_error<W: Write>(out: &mut W, message: &str) -> std::io::Result<()> {
    write_sse_headers(out)?;
    out.write_all(delta_to_sse(&ChatDelta::Error(message.to_string())).as_bytes())?;
    out.flush()
}

/// Build a `ChatProvider` for a legacy model-only request. Ambiguous targets
/// return `None`; structured browser requests use the exact-provider request
/// resolver instead. Browser-owned built-ins remain subject to the deployment
/// endpoint policy even when they were persisted by a private deployment.
///
// Host CLI and ACP agents stay unavailable here: a browser request must never
// turn the web daemon into an arbitrary local subprocess launcher.
pub fn proxy_provider(editor: &EditorState, model: &str) -> Option<Box<dyn ChatProvider>> {
    proxy_provider_with_chat_session(editor, model, true)
}

/// Resolve one browser request in chat-session mode while preserving the
/// deployment's credential-persistence context. Persistence controls whether
/// browser credentials may be copied into daemon settings; it never relaxes
/// per-request endpoint validation.
pub fn proxy_provider_for_request(
    editor: &EditorState,
    request: &AiStreamRequest,
    policy: crate::web_credential_policy::WebCredentialPersistence,
) -> Result<Option<Box<dyn ChatProvider>>, String> {
    proxy_provider_for_request_with_chat_session(editor, request, true, policy)
}

/// Resolve one parsed browser request. Structured clients may constrain the
/// built-in provider identity; host CLI and ACP agents remain unavailable. A
/// request-scoped built-in credential always wins over daemon settings but is
/// validated independently of persistence policy.
pub fn proxy_provider_for_request_with_chat_session(
    editor: &EditorState,
    request: &AiStreamRequest,
    _chat_session: bool,
    _policy: crate::web_credential_policy::WebCredentialPersistence,
) -> Result<Option<Box<dyn ChatProvider>>, String> {
    if let Some(agent) = request.transient_builtin.as_ref() {
        if agent.model.trim() != request.model.trim() {
            return Err("transient credential model does not match the request".into());
        }
        if request
            .provider
            .is_some_and(|expected| agent.kind.model_provider() != expected)
        {
            return Err("transient credential provider does not match the request".into());
        }
        crate::web_credentials::validate_web_provider_base_url(&agent.base_url)?;
        if !crate::web_credentials::public_demo_transient_endpoint_allowed(agent) {
            return Err("custom provider endpoint is not explicitly allowed".into());
        }
        return Ok(ConfiguredBuiltinProvider::from_builtin_agent(agent)
            .map(|provider| Box::new(provider) as Box<dyn ChatProvider>));
    }

    Ok(proxy_builtin_for_identity(
        editor,
        request.provider,
        &request.model,
    ))
}

/// Build a legacy built-in proxy provider. The session flag is retained for
/// call-site parity, but web requests never launch a host CLI session.
pub fn proxy_provider_with_chat_session(
    editor: &EditorState,
    model: &str,
    _chat_session: bool,
) -> Option<Box<dyn ChatProvider>> {
    proxy_builtin_for_identity(editor, None, model)
}

fn browser_owned_endpoint_is_allowed(agent: &BuiltinAgentConfig) -> bool {
    !crate::web_credentials::browser_owns_builtin_agent(agent)
        || crate::web_credentials::public_demo_transient_endpoint_allowed(agent)
}

/// JSON body for `GET /api/ai/models`. Only permitted ready built-in model ids
/// are exposed; host CLI discovery remains private to the native desktop.
pub fn models_json(editor: &EditorState) -> String {
    let mut models: Vec<String> = editor
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .filter(|agent| agent.ready() && browser_owned_endpoint_is_allowed(agent))
        .map(|agent| agent.model.trim())
        .filter(|model| !model.is_empty())
        .map(str::to_string)
        .collect();
    let mut seen = std::collections::HashSet::new();
    models.retain(|model| seen.insert(model.clone()));
    serde_json::to_string(&models).unwrap_or_else(|_| "[]".to_string())
}

fn proxy_builtin_for_identity(
    editor: &EditorState,
    provider: Option<AgentProvider>,
    model: &str,
) -> Option<Box<dyn ChatProvider>> {
    if let Some((builtin_id, builtin_model)) = parse_builtin_model_key(model) {
        let chosen = editor
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .find(|agent| {
                agent.ready()
                    && browser_owned_endpoint_is_allowed(agent)
                    && agent.id == builtin_id
                    && agent.model.trim() == builtin_model
                    && provider.is_none_or(|expected| agent.kind.model_provider() == expected)
            })?;
        return ConfiguredBuiltinProvider::from_builtin_agent(chosen)
            .map(|p| Box::new(p) as Box<dyn ChatProvider>);
    }

    let requested = model.trim();
    let is_default = requested.is_empty() || requested == "default";
    let agents = &editor.editor_ui.agent_settings.builtin_agents;
    let exact = || {
        agents.iter().filter(|agent| {
            agent.ready()
                && agent.model.trim() == requested
                && provider.is_none_or(|expected| agent.kind.model_provider() == expected)
        })
    };
    if let Some(chosen) = exact().find(|agent| browser_owned_endpoint_is_allowed(agent)) {
        return ConfiguredBuiltinProvider::from_builtin_agent(chosen)
            .map(|configured| Box::new(configured) as Box<dyn ChatProvider>);
    }
    if exact().next().is_some() || !is_default {
        return None;
    }

    let chosen = agents.iter().find(|agent| {
        agent.ready()
            && browser_owned_endpoint_is_allowed(agent)
            && provider.is_none_or(|expected| agent.kind.model_provider() == expected)
    })?;
    ConfiguredBuiltinProvider::from_builtin_agent(chosen)
        .map(|configured| Box::new(configured) as Box<dyn ChatProvider>)
}

fn parse_builtin_model_key(value: &str) -> Option<(&str, &str)> {
    let mut parts = value.trim().splitn(3, ':');
    if parts.next()? != "builtin" {
        return None;
    }
    let id = parts.next()?.trim();
    let model = parts.next()?.trim();
    (!id.is_empty() && !model.is_empty()).then_some((id, model))
}

#[cfg(test)]
mod tests {
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
        let body = r#"{"provider":"grok-build","model":"m","skills":["codegen-planning"],"user":"hi","max_output_tokens":2000,"thinking":"adaptive","effort":"low"}"#;
        let req = parse_ai_stream_body(body).expect("body parses");
        assert_eq!(req.provider, Some(AgentProvider::GrokBuild));
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
        let req =
            parse_ai_stream_body(r#"{"thinking":"disabled","effort":"high"}"#).expect("parses");
        assert_eq!(req.thinking, ThinkingMode::Disabled);
        assert_eq!(req.effort, EffortLevel::High);
        let req2 =
            parse_ai_stream_body(r#"{"thinking":"enabled","effort":"max"}"#).expect("parses");
        assert_eq!(req2.thinking, ThinkingMode::Enabled);
        assert_eq!(req2.effort, EffortLevel::Max);
    }

    #[test]
    fn parse_ai_stream_body_rejects_non_object() {
        assert!(parse_ai_stream_body("[]").is_none());
        assert!(parse_ai_stream_body("not json").is_none());
        assert!(parse_ai_stream_body(r#"{"provider":"unknown"}"#).is_none());
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
        stream_ai_response(&mut out, req, &provider).expect("stream");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("Content-Type: text/event-stream"), "{text}");
        assert!(text.contains(r#"data: {"delta":"Hello"}"#), "{text}");
        assert!(text.contains(r#"data: {"done":true}"#), "{text}");
    }

    #[test]
    fn stream_ai_response_forwards_requested_model_to_provider() {
        let req = AiStreamRequest {
            provider: None,
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

        stream_ai_response(&mut out, req, &provider).expect("stream");

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
        stream_ai_response(&mut out, req, &provider).expect("stream");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains(r#"data: {"error":"boom"}"#), "{text}");
        assert!(!text.contains("should not appear"), "{text}");
    }

    #[test]
    fn write_sse_error_emits_headers_and_error() {
        let mut out = Vec::<u8>::new();
        write_sse_error(&mut out, "no model configured").expect("write");
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
        let parsed: Vec<String> = serde_json::from_str(&json).expect("valid model array");
        assert_eq!(parsed, vec!["claude-sonnet-4-5"]);
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
}

#[cfg(test)]
#[path = "ai_proxy_tests.rs"]
mod ai_proxy_tests;
