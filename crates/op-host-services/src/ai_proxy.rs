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

use crate::ai_proxy_error::ProxyProviderError;
use crate::chat_builtin_http::ConfiguredBuiltinProvider;

/// Parsed `POST /api/ai/stream` body. The web bundle sends skill NAMES
/// (not the corpus) plus the per-turn knobs; the proxy expands the
/// names server-side.
pub struct AiStreamRequest {
    /// Explicit provider identity from the structured model catalog. Older
    /// clients omit it and use the ambiguity-safe legacy resolver.
    pub provider: Option<AgentProvider>,
    /// Exact daemon-owned built-in identity from the structured model
    /// catalog. Kept separate from `model` because both ids may contain `:`.
    pub builtin_provider_id: Option<String>,
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
    let builtin_provider_id = match obj.get("builtinProviderId") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        }
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
        builtin_provider_id,
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
    cors_origin: Option<&str>,
) -> std::io::Result<()> {
    write_sse_headers(out, cors_origin)?;
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
    if model.is_empty() || model == "default" || model.starts_with("builtin:") {
        None
    } else {
        Some(model.to_string())
    }
}

/// Write the SSE header block — same shape as
/// `web_canvas_server::serve_sse` (200 OK + `text/event-stream` +
/// no-cache + keep-alive + blank line). `cors_origin` is the precomputed
/// `Access-Control-Allow-Origin` value (see
/// `web_canvas_server::cors_origin_for`): `Some(origin)` echoes it,
/// `None` omits the header (managed mode, origin not on the allowlist).
pub fn write_sse_headers<W: Write>(out: &mut W, cors_origin: Option<&str>) -> std::io::Result<()> {
    let cors_line = cors_origin
        .map(|origin| format!("Access-Control-Allow-Origin: {origin}\r\n"))
        .unwrap_or_default();
    let headers = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: keep-alive\r\n\
         {cors_line}\r\n"
    );
    out.write_all(headers.as_bytes())?;
    out.flush()
}

/// Write a single SSE error event (with headers) and close — used by
/// the route when no provider could be built. `flush` after so the
/// client sees the error before the socket drops.
pub fn write_sse_error<W: Write>(
    out: &mut W,
    message: &str,
    cors_origin: Option<&str>,
) -> std::io::Result<()> {
    write_sse_headers(out, cors_origin)?;
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
) -> Result<Option<Box<dyn ChatProvider>>, ProxyProviderError> {
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
) -> Result<Option<Box<dyn ChatProvider>>, ProxyProviderError> {
    if let Some(agent) = request.transient_builtin.as_ref() {
        if !agent.has_model(request.model.trim()) {
            return Err(ProxyProviderError::TransientModelMismatch);
        }
        if request
            .provider
            .is_some_and(|expected| agent.kind.model_provider() != expected)
        {
            return Err(ProxyProviderError::TransientProviderMismatch);
        }
        crate::web_credentials::validate_web_provider_base_url(&agent.base_url)?;
        if !crate::web_credentials::public_demo_transient_endpoint_allowed(agent) {
            return Err(ProxyProviderError::EndpointNotPermittedByDeployment);
        }
        return Ok(
            ConfiguredBuiltinProvider::from_builtin_agent_for_web_with_model(agent, &request.model)
                .map(|provider| Box::new(provider) as Box<dyn ChatProvider>),
        );
    }

    Ok(proxy_builtin_for_identity(
        editor,
        request.provider,
        request.builtin_provider_id.as_deref(),
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
    proxy_builtin_for_identity(editor, None, None, model)
}

fn browser_owned_endpoint_is_allowed(agent: &BuiltinAgentConfig) -> bool {
    !crate::web_credentials::browser_owns_builtin_agent(agent)
        || crate::web_credentials::public_demo_transient_endpoint_allowed(agent)
}

/// JSON body for `GET /api/ai/models`. Each row carries its exact built-in
/// provider identity so equal model ids on different credentials remain
/// independently selectable. Host CLI discovery remains private to native.
pub fn models_json(editor: &EditorState) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut models = Vec::new();
    for agent in editor
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .filter(|agent| {
            agent.ready()
                && !crate::web_credentials::browser_owns_builtin_agent(agent)
                && browser_owned_endpoint_is_allowed(agent)
        })
    {
        for model in agent.models.iter().map(|model| model.trim()) {
            if model.is_empty() || !seen.insert((agent.id.as_str(), model)) {
                continue;
            }
            models.push(json!({
                "provider": agent.kind.model_provider().wire_id(),
                "value": format!("builtin:{}:{model}", agent.id),
                "displayName": model,
                "providerDisplayName": agent.display_name,
                "builtinProviderId": agent.id,
            }));
        }
    }
    serde_json::to_string(&models).unwrap_or_else(|_| "[]".to_string())
}

/// Web-request provider constructor: browser-owned agents dial with
/// connect-time DNS screening; operator-owned daemon agents stay trusted.
fn web_provider_for_agent(
    agent: &BuiltinAgentConfig,
    selected_model: &str,
) -> Option<Box<dyn ChatProvider>> {
    let provider = if crate::web_credentials::browser_owns_builtin_agent(agent) {
        ConfiguredBuiltinProvider::from_builtin_agent_for_web_with_model(agent, selected_model)
    } else {
        ConfiguredBuiltinProvider::from_builtin_agent_with_model(agent, selected_model)
    };
    provider.map(|configured| Box::new(configured) as Box<dyn ChatProvider>)
}

fn proxy_builtin_for_identity(
    editor: &EditorState,
    provider: Option<AgentProvider>,
    builtin_provider_id: Option<&str>,
    model: &str,
) -> Option<Box<dyn ChatProvider>> {
    let model = model.trim();
    if let Some(builtin_id) = builtin_provider_id {
        let chosen = editor
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .find(|agent| agent.id == builtin_id)?;
        let builtin_model = match model.strip_prefix("builtin:") {
            Some(structured) => structured
                .strip_prefix(builtin_id)?
                .strip_prefix(':')?
                .trim(),
            None => model,
        };
        if !chosen.ready()
            || !browser_owned_endpoint_is_allowed(chosen)
            || !chosen.has_model(builtin_model)
            || provider.is_some_and(|expected| chosen.kind.model_provider() != expected)
        {
            return None;
        }
        return web_provider_for_agent(chosen, builtin_model);
    }
    // Rolling-upgrade compatibility for a web tab loaded before
    // `builtinProviderId` was added. Never split the structured value: compare
    // the complete value generated from each saved `(provider id, model)` and
    // accept only one exact candidate. Non-injective colon joins remain
    // rejected instead of guessing a credential boundary.
    if model.starts_with("builtin:") {
        let mut candidate = None;
        for agent in &editor.editor_ui.agent_settings.builtin_agents {
            if provider.is_some_and(|expected| agent.kind.model_provider() != expected) {
                continue;
            }
            for saved_model in &agent.models {
                if format!("builtin:{}:{saved_model}", agent.id) != model {
                    continue;
                }
                if candidate.is_some() {
                    return None;
                }
                candidate = Some((agent, saved_model.as_str()));
            }
        }
        let (chosen, selected_model) = candidate?;
        if !chosen.ready() || !browser_owned_endpoint_is_allowed(chosen) {
            return None;
        }
        return web_provider_for_agent(chosen, selected_model);
    }

    let requested = model;
    let is_default = requested.is_empty() || requested == "default";
    let agents = &editor.editor_ui.agent_settings.builtin_agents;
    let exact = || {
        agents.iter().filter(|agent| {
            agent.ready()
                && agent.has_model(requested)
                && provider.is_none_or(|expected| agent.kind.model_provider() == expected)
        })
    };
    let mut eligible = exact().filter(|agent| browser_owned_endpoint_is_allowed(agent));
    if let Some(chosen) = eligible.next() {
        if eligible.next().is_some() {
            return None;
        }
        return web_provider_for_agent(chosen, requested);
    }
    if exact().next().is_some() || !is_default {
        return None;
    }

    let chosen = agents.iter().find(|agent| {
        agent.ready()
            && browser_owned_endpoint_is_allowed(agent)
            && provider.is_none_or(|expected| agent.kind.model_provider() == expected)
    })?;
    let model = chosen.first_model()?;
    web_provider_for_agent(chosen, model)
}

#[cfg(test)]
#[path = "ai_proxy_parse_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "ai_proxy_tests.rs"]
mod ai_proxy_tests;
