//! API-key backed built-in chat providers.
//!
//! These mirror the TS app's built-in provider route without enabling
//! agent-rs concrete-provider features in this Rust build. The
//! implementation posts directly to Anthropic or OpenAI-compatible
//! streaming endpoints and converts SSE payloads into `ChatDelta`s.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use op_ai::chat_provider::{
    ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, ChatToolDef, ChatToolExecutor,
    EffortLevel, StopReason, ThinkingMode,
};
use op_editor_core::{BuiltinAgentConfig, BuiltinAgentKind};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::chat_agent_loop::{run_anthropic_agent_loop, run_openai_agent_loop, AgentLoopConfig};
use crate::chat_canvas_tools::MAX_TOOL_TURNS;
use crate::chat_runtime::{resolved_skill_preamble, shared_runtime, BlockingRecvIter};

// The error enum, retry ladder, throttle, and provider-client builders moved
// to `op-chat-agent` (pure code motion) so the mobile FFI hosts share them;
// re-exported here so every existing path stays valid. Test-only helpers
// (`is_retryable_status`, `throttle_wait`) ride along for the sibling tests.
#[cfg(test)]
use op_chat_agent::backoff::{
    backoff_delay, is_retryable_status, parse_retry_after, throttle_wait,
};
pub(crate) use op_chat_agent::backoff::{builtin_http_client, send_with_backoff};
use op_chat_agent::backoff::{builtin_http_min_gap, BUILTIN_HTTP_MAX_RETRIES};
pub use op_chat_agent::backoff::{DESIGN_LOOP_MAX_OUTPUT_TOKENS, DESIGN_LOOP_MAX_TURNS};
pub use op_chat_agent::chat_builtin_http::BuiltinHttpError;

pub(crate) use crate::chat_builtin_http_wire::{
    normalize_provider_base_url, parse_anthropic_sse_data, parse_openai_sse_data,
    provider_endpoint, pump_sse_response,
};
// `apply_reasoning_wire_control` is public so the headless benchmark harness
// (op-smoke) builds its request body through the SAME entry point the live
// chat path uses. The harness used to keep its own copy of both the capability
// table and the JSON shape, which drifted twice.
pub use crate::chat_builtin_http_wire::{
    apply_reasoning_wire_control, map_anthropic_stop_reason, map_openai_stop_reason,
};

#[derive(Clone)]
pub struct ConfiguredBuiltinProvider {
    kind: BuiltinAgentKind,
    api_key: String,
    model: String,
    base_url: String,
    label: String,
    /// Canvas tool defs + executor for the tool-executing agent loop.
    /// Empty / `None` keeps the plain streaming path (no tools on the
    /// wire). Wired by the chat path only — the design orchestrator
    /// uses this provider as a plain LLM and must never see tools.
    tools: Vec<ChatToolDef>,
    executor: Option<Arc<dyn ChatToolExecutor>>,
    /// Run the loop-end structural backstop (`apply_loop_finalize`) for
    /// this provider's turns. Set ONLY by the gated design-generation
    /// provider; regular chat leaves it false so an ordinary tool-using
    /// chat turn never mutates an existing design (Track-1 Step 4 scope).
    finalize_on_exit: bool,
    construction_error: Option<BuiltinHttpError>,
    http_client: Option<reqwest::Client>,
    max_retries: u32,
    min_gap: Duration,
    /// How this provider's endpoint may be dialed. Browser-originated
    /// credentials get `PublicOnly` (connect-time DNS screening + pinning);
    /// operator-owned daemon settings stay `Trusted`.
    dial_policy: crate::provider_dial::EndpointDialPolicy,
    /// Optional agent-loop turn-cap override. `None` keeps the standard
    /// caps (`DESIGN_LOOP_MAX_TURNS` when `finalize_on_exit`, else
    /// `MAX_TOOL_TURNS`); the headless `run_design_agent` tool sets it.
    max_turns_override: Option<usize>,
}

impl ConfiguredBuiltinProvider {
    /// Build a provider from operator-owned (trusted) configuration.
    pub fn from_builtin_agent(config: &BuiltinAgentConfig) -> Option<Self> {
        let model = config.first_model()?;
        Self::from_builtin_agent_with_model(config, model)
    }

    /// Build a provider for one explicitly selected saved model.
    ///
    /// A built-in configuration may expose several models in the picker, but
    /// every provider request still carries exactly one model id. Keeping the
    /// membership check at construction prevents a stale or forged picker row
    /// from borrowing this provider's credential for an unsaved model.
    pub fn from_builtin_agent_with_model(
        config: &BuiltinAgentConfig,
        selected_model: &str,
    ) -> Option<Self> {
        let selected_model = selected_model.trim();
        if !config.ready() || !config.has_model(selected_model) {
            return None;
        }
        let configured_base = if config.base_url.trim().is_empty() {
            config.kind.default_base_url()
        } else {
            config.base_url.trim()
        };
        let (base_url, mut construction_error) = match normalize_provider_base_url(configured_base)
        {
            Ok(base_url) => (base_url, None),
            Err(error) => (
                configured_base.trim_end_matches('/').to_string(),
                Some(error),
            ),
        };
        let http_client = match builtin_http_client() {
            Ok(client) => Some(client),
            Err(error) => {
                // The dial guard's `ClientBuild` folds into `Dial` — same
                // sentence, so the reported construction error is unchanged.
                construction_error.get_or_insert(error.into());
                None
            }
        };
        let label = if config.display_name.trim().is_empty() {
            selected_model
        } else {
            config.display_name.trim()
        };
        Some(Self {
            kind: config.kind,
            api_key: config.api_key.trim().to_string(),
            model: selected_model.to_string(),
            base_url,
            label: label.to_string(),
            tools: Vec::new(),
            executor: None,
            finalize_on_exit: false,
            construction_error,
            http_client,
            max_retries: BUILTIN_HTTP_MAX_RETRIES,
            min_gap: builtin_http_min_gap(),
            dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
            max_turns_override: None,
        })
    }

    /// Build a provider from a browser-supplied credential. The endpoint is
    /// dialed `PublicOnly` (connect-time DNS screening + address pinning)
    /// unless the operator explicitly allowlisted it.
    pub fn from_builtin_agent_for_web(config: &BuiltinAgentConfig) -> Option<Self> {
        let model = config.first_model()?;
        Self::from_builtin_agent_for_web_with_model(config, model)
    }

    /// Browser-dial-policy variant of [`Self::from_builtin_agent_with_model`].
    pub fn from_builtin_agent_for_web_with_model(
        config: &BuiltinAgentConfig,
        selected_model: &str,
    ) -> Option<Self> {
        let mut provider = Self::from_builtin_agent_with_model(config, selected_model)?;
        let allowlist = std::env::var(crate::web_credentials::WEB_AI_ENDPOINT_ALLOWLIST_ENV).ok();
        provider.dial_policy =
            crate::provider_dial::web_dial_policy_for(&provider.base_url, allowlist.as_deref());
        Some(provider)
    }

    /// Per-request client honoring this provider's dial policy. `Trusted`
    /// reuses the eagerly-built client; `PublicOnly` resolves + pins.
    async fn dial_client(&self, url: &str) -> Result<reqwest::Client, BuiltinHttpError> {
        match self.dial_policy {
            crate::provider_dial::EndpointDialPolicy::Trusted => self
                .http_client
                .clone()
                .ok_or(BuiltinHttpError::ClientUnavailable),
            crate::provider_dial::EndpointDialPolicy::PublicOnly => {
                Ok(crate::provider_dial::client_for(self.dial_policy, url).await?)
            }
        }
    }

    /// Enable the tool-executing agent loop for this provider's turns.
    /// `tools` are advertised on the wire; `executor` runs each call
    /// (production: the UI-thread channel bridge in
    /// `chat_canvas_tools`). Chat path only — see the field docs.
    pub fn with_canvas_tools(
        mut self,
        tools: Vec<ChatToolDef>,
        executor: Arc<dyn ChatToolExecutor>,
    ) -> Self {
        self.tools = tools;
        self.executor = Some(executor);
        self
    }

    /// Opt this provider's agent-loop turns into the loop-end structural
    /// backstop (Track-1 Step 4). The gated design-generation provider calls
    /// this; regular chat does not, so a plain chat turn never re-finalizes
    /// (and mutates) the live document. No-op without `with_canvas_tools`
    /// (the plain streaming path never reaches `run_loop_finalize`).
    pub fn with_loop_finalize(mut self) -> Self {
        self.finalize_on_exit = true;
        self
    }

    /// Override the agent-loop turn cap for this provider's turns. Used by
    /// the headless `run_design_agent` MCP tool; no-op for plain streaming.
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns_override = Some(max_turns.max(1));
        self
    }

    fn endpoint(&self, path: &str) -> String {
        provider_endpoint(&self.base_url, path)
    }
}

impl fmt::Debug for ConfiguredBuiltinProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfiguredBuiltinProvider")
            .field("kind", &self.kind)
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("label", &self.label)
            .finish()
    }
}

impl ChatProvider for ConfiguredBuiltinProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn supports_evidence_only_send(&self) -> bool {
        self.tools.is_empty() && self.executor.is_none()
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, None)
    }

    fn send_cancellable(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, Some(cancel))
    }
}

impl ConfiguredBuiltinProvider {
    fn send_inner(
        &self,
        request: ChatRequest,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        if cancel
            .as_ref()
            .is_some_and(|cancel| cancel.load(Ordering::Acquire))
        {
            return Box::new(
                [ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                }]
                .into_iter(),
            );
        }
        if let Some(error) = &self.construction_error {
            return Box::new(
                [
                    // `ChatDelta::Error` is `op-ai`'s transcript sink and
                    // takes a `String`; render at this boundary only.
                    ChatDelta::Error(error.to_string()),
                    ChatDelta::Done {
                        stop_reason: StopReason::Aborted,
                    },
                ]
                .into_iter(),
            );
        }
        let (mut prompt, guard) = match crate::chat_attachment::prompt_with_attachments(
            &request.user_message,
            &request.attachments,
        ) {
            Ok(pair) => pair,
            Err(e) => return crate::chat_attachment::attachment_error_turn(e),
        };
        let mut directive = String::new();
        if let Some(d) = crate::chat_attachment::thinking_directive(request.thinking) {
            directive.push_str(d);
        }
        if request.effort != EffortLevel::Low {
            if !directive.is_empty() {
                directive.push(' ');
            }
            directive.push_str(&format!(
                "Apply {} reasoning effort.",
                request.effort.as_str()
            ));
        }
        if !directive.is_empty() {
            prompt = format!("{directive}\n\n{prompt}");
        }
        // The per-turn system prompt (chat_system_prompt.rs) already
        // resolves the skill corpus; only fall back to the in-prompt
        // preamble when the caller sent no system prompt — otherwise
        // the skills would ride the wire twice.
        if request.system_prompt.trim().is_empty() {
            let preamble = resolved_skill_preamble(&request.user_message);
            if !preamble.is_empty() {
                prompt = format!("{preamble}\n\n---\n\n{prompt}");
            }
        }

        let provider = self.clone();
        let system_prompt = request.system_prompt;
        let history = request.history;
        let max_output_tokens = request.max_output_tokens.max(1);
        // Only force MiniMax thinking off when the CALLER asked for it
        // (the orchestrator sets `Disabled`; normal chat defaults to
        // `Adaptive` and must keep M3's reasoning). Codex review caught
        // an earlier version disabling thinking unconditionally for all
        // MiniMax chat.
        let disable_thinking = request.thinking == ThinkingMode::Disabled;
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        let task = shared_runtime().spawn(async move {
            let _guard = guard;
            // Tool-capable turns route through the agent loop: tool
            // defs ride the request, `tool_use` streams back, the
            // executor runs each call, and `tool_result` rides a
            // follow-up request — looping until the model stops
            // calling tools (GAP #32).
            let emitted_done = if let Some(executor) = provider.executor.clone() {
                let cfg = AgentLoopConfig {
                    url: match provider.kind {
                        BuiltinAgentKind::Anthropic => provider.endpoint("/v1/messages"),
                        BuiltinAgentKind::OpenAiCompat => provider.endpoint("/chat/completions"),
                    },
                    api_key: provider.api_key.clone(),
                    model: provider.model.clone(),
                    system_prompt,
                    history,
                    user_prompt: prompt,
                    max_output_tokens,
                    tools: provider.tools.clone(),
                    executor,
                    max_turns: provider.max_turns_override.unwrap_or(
                        if provider.finalize_on_exit {
                            DESIGN_LOOP_MAX_TURNS
                        } else {
                            MAX_TOOL_TURNS
                        },
                    ),
                    finalize_on_exit: provider.finalize_on_exit,
                    disable_thinking,
                    dial_policy: provider.dial_policy,
                };
                match provider.kind {
                    BuiltinAgentKind::Anthropic => run_anthropic_agent_loop(cfg, &tx).await,
                    BuiltinAgentKind::OpenAiCompat => run_openai_agent_loop(cfg, &tx).await,
                }
            } else {
                match provider.kind {
                    BuiltinAgentKind::Anthropic => {
                        run_anthropic_chat(
                            provider,
                            system_prompt,
                            history,
                            prompt,
                            max_output_tokens,
                            &tx,
                        )
                        .await
                    }
                    BuiltinAgentKind::OpenAiCompat => {
                        run_openai_chat(
                            provider,
                            system_prompt,
                            history,
                            prompt,
                            max_output_tokens,
                            disable_thinking,
                            &tx,
                        )
                        .await
                    }
                }
            };
            match emitted_done {
                Ok(true) => {}
                Ok(false) => {
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::EndTurn,
                        })
                        .await;
                }
                Err(e) => {
                    // Same `op-ai`-owned sink as above: render the typed
                    // failure into the transcript's `String` here.
                    let _ = tx.send(ChatDelta::Error(e.to_string())).await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                }
            }
        });
        match cancel {
            Some(cancel) => Box::new(BlockingRecvIter::cancellable(rx, cancel, task)),
            None => Box::new(BlockingRecvIter::new(rx)),
        }
    }
}

async fn run_openai_chat(
    provider: ConfiguredBuiltinProvider,
    system_prompt: String,
    history: Vec<(ChatHistoryRole, String)>,
    prompt: String,
    max_output_tokens: u32,
    disable_thinking: bool,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let url = provider.endpoint("/chat/completions");
    let mut messages = Vec::new();
    if !system_prompt.trim().is_empty() {
        messages.push(json!({
            "role": "system",
            "content": system_prompt,
        }));
    }
    // Prior turns ride as full wire messages (TS parity: the builtin
    // route seeds the engine with `messages.slice(0, -1)`).
    for (role, text) in &history {
        messages.push(json!({ "role": role.as_str(), "content": text }));
    }
    messages.push(json!({
        "role": "user",
        "content": prompt,
    }));
    let mut body = json!({
        "model": provider.model,
        "stream": true,
        "max_tokens": max_output_tokens,
        "messages": messages,
    });
    // Optional temperature override: read OPENPENCIL_LLM_TEMPERATURE
    // (f32 in range 0.0..=2.0). If set and valid, add to body; else
    // omit (preserve provider default).
    if let Ok(temp_str) = std::env::var("OPENPENCIL_LLM_TEMPERATURE") {
        if let Ok(temp) = temp_str.parse::<f32>() {
            if (0.0..=2.0).contains(&temp) {
                body["temperature"] = json!(temp);
            }
        }
    }
    // 推理模型不关思考会把 reasoning 烧到占满输出预算,JSON content 被截断甚至
    // 留空(glm-5.2 实测一个设计子任务 thinking≈3 万字符、content 0,整段 parse
    // 失败、重试也撞同一堵墙)。当调用方明确要求关思考(`disable_thinking`,如编排器
    // 的设计子任务),且该模型家族能在线级表达这条意图时,下发
    // provider-specific wire control. K2.5/K2.6, GLM, DeepSeek, and MiniMax
    // use `thinking:{type:"disabled"}`; Kimi K3 rejects that field and uses
    // top-level `reasoning_effort:"low"`. Ordinary chat stays Adaptive.
    //
    // The policy lives in `op_orchestrator::reasoning_wire_control`; the JSON
    // mutation is shared with the agent loop so the two paths stay identical.
    apply_reasoning_wire_control(&mut body, &provider.model, disable_thinking);
    let client = provider.dial_client(&url).await?;
    let resp = send_with_backoff(
        "openai-compatible",
        &url,
        provider.max_retries,
        provider.min_gap,
        || client.post(&url).bearer_auth(&provider.api_key).json(&body),
    )
    .await?;
    pump_sse_response(resp, tx, parse_openai_sse_data).await
}

async fn run_anthropic_chat(
    provider: ConfiguredBuiltinProvider,
    system_prompt: String,
    history: Vec<(ChatHistoryRole, String)>,
    prompt: String,
    max_output_tokens: u32,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let url = provider.endpoint("/v1/messages");
    // Prior turns ride as full wire messages ahead of the current
    // user prompt (TS parity: builtin multi-turn context seeding).
    let mut messages: Vec<Value> = history
        .iter()
        .map(|(role, text)| json!({ "role": role.as_str(), "content": text }))
        .collect();
    messages.push(json!({ "role": "user", "content": prompt }));
    let mut body = json!({
        "model": provider.model,
        "max_tokens": max_output_tokens,
        "stream": true,
        "messages": messages,
    });
    if !system_prompt.trim().is_empty() {
        body.as_object_mut()
            .expect("anthropic request body is object")
            .insert("system".into(), json!(system_prompt));
    }
    let client = provider.dial_client(&url).await?;
    let resp = send_with_backoff(
        "anthropic",
        &url,
        provider.max_retries,
        provider.min_gap,
        || {
            client
                .post(&url)
                .header("x-api-key", &provider.api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&body)
        },
    )
    .await?;
    pump_sse_response(resp, tx, parse_anthropic_sse_data).await
}

#[cfg(test)]
#[path = "chat_builtin_http_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chat_builtin_http_cancellation_tests.rs"]
mod cancellation_tests;
