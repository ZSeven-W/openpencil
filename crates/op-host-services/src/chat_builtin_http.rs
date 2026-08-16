//! API-key backed built-in chat providers.
//!
//! These mirror the TS app's built-in provider route without enabling
//! agent-rs concrete-provider features in this Rust build. The
//! implementation posts directly to Anthropic or OpenAI-compatible
//! streaming endpoints and converts SSE payloads into `ChatDelta`s.

use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

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

#[path = "chat_builtin_http_error.rs"]
mod error;
pub use error::BuiltinHttpError;

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

/// Design turns build one <=25-op section batch per model turn, plus repair
/// turns after layoutIssues feedback. 28 sits in the requested 24-32 window:
/// enough for roughly 8-10 sections with fixes, without letting a bad loop run
/// indefinitely. Plain chat keeps [`MAX_TOOL_TURNS`].
pub const DESIGN_LOOP_MAX_TURNS: usize = 28;

/// A <=25-op batch plus one concise tool call fits comfortably in 4-6k output
/// tokens. 6144 keeps section batches complete without encouraging the model to
/// emit a whole screen in one turn.
pub const DESIGN_LOOP_MAX_OUTPUT_TOKENS: u32 = 6_144;

const BUILTIN_HTTP_DEFAULT_MIN_GAP: Duration = Duration::from_millis(350);
// 5 retries at 1/2/4/8/16s covers ~31s — enough to outlast a per-minute
// account frequency window (GLM "AccountRateLimitExceeded" killed a run
// that 3 retries / 7s could not ride out, measured 2026-07-12).
const BUILTIN_HTTP_MAX_RETRIES: u32 = 5;
/// Extra inter-request gap added after each 429, halved after each
/// success — the throttle adapts to the account's real ceiling instead of
/// hammering at the configured floor.
static BUILTIN_HTTP_ADAPTIVE_GAP_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
const ADAPTIVE_GAP_START_MS: u64 = 1_000;
const ADAPTIVE_GAP_MAX_MS: u64 = 5_000;
const BUILTIN_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const BUILTIN_HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const RETRY_AFTER_MAX: Duration = Duration::from_secs(30);
const BACKOFF_MAX: Duration = Duration::from_secs(8);

static BUILTIN_HTTP_LAST_REQUEST: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

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

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
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
        shared_runtime().spawn(async move {
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
                    max_turns: if provider.finalize_on_exit {
                        DESIGN_LOOP_MAX_TURNS
                    } else {
                        MAX_TOOL_TURNS
                    },
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
        Box::new(BlockingRecvIter::new(rx))
    }
}

fn builtin_http_min_gap() -> Duration {
    std::env::var("OPENPENCIL_BUILTIN_HTTP_MIN_GAP_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(BUILTIN_HTTP_DEFAULT_MIN_GAP)
}

fn throttle_wait(last: Option<Instant>, now: Instant, min_gap: Duration) -> Duration {
    last.and_then(|last| last.checked_add(min_gap))
        .map(|next_allowed| next_allowed.saturating_duration_since(now))
        .unwrap_or(Duration::ZERO)
}

fn widen_adaptive_gap() {
    use std::sync::atomic::Ordering;
    let current = BUILTIN_HTTP_ADAPTIVE_GAP_MS.load(Ordering::Relaxed);
    let next = (current * 2).clamp(ADAPTIVE_GAP_START_MS, ADAPTIVE_GAP_MAX_MS);
    BUILTIN_HTTP_ADAPTIVE_GAP_MS.store(next, Ordering::Relaxed);
}

fn relax_adaptive_gap() {
    use std::sync::atomic::Ordering;
    let current = BUILTIN_HTTP_ADAPTIVE_GAP_MS.load(Ordering::Relaxed);
    if current > 0 {
        BUILTIN_HTTP_ADAPTIVE_GAP_MS.store(current / 2, Ordering::Relaxed);
    }
}

async fn throttle_builtin_http_request(base_min_gap: Duration) {
    let adaptive = Duration::from_millis(
        BUILTIN_HTTP_ADAPTIVE_GAP_MS.load(std::sync::atomic::Ordering::Relaxed),
    );
    let min_gap = base_min_gap + adaptive;
    if min_gap.is_zero() {
        return;
    }

    let wait = {
        let last_request = BUILTIN_HTTP_LAST_REQUEST.get_or_init(|| Mutex::new(None));
        let mut last = last_request
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();
        let wait = throttle_wait(*last, now, min_gap);
        *last = now.checked_add(wait);
        wait
    };

    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
        || status == reqwest::StatusCode::from_u16(529).expect("529 is a valid HTTP status")
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let seconds = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()?;
    Some(Duration::from_secs(seconds).min(RETRY_AFTER_MAX))
}

fn backoff_delay(attempt: u32) -> Duration {
    let delay = 1_u64.checked_shl(attempt).unwrap_or(u64::MAX);
    Duration::from_secs(delay).min(BACKOFF_MAX)
}

/// Default retry/throttle knobs for callers without a per-provider
/// profile (the design tool-loop's own HTTP path).
pub(crate) fn default_backoff_knobs() -> (u32, Duration) {
    (BUILTIN_HTTP_MAX_RETRIES, builtin_http_min_gap())
}

pub(crate) async fn send_with_backoff(
    label: &str,
    url: &str,
    max_retries: u32,
    min_gap: Duration,
    build: impl Fn() -> reqwest::RequestBuilder,
) -> Result<reqwest::Response, BuiltinHttpError> {
    for attempt in 0..=max_retries {
        throttle_builtin_http_request(min_gap).await;
        match build().send().await {
            Ok(resp) if resp.status().is_success() => {
                relax_adaptive_gap();
                return Ok(resp);
            }
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    widen_adaptive_gap();
                }
                if is_retryable_status(status) && attempt < max_retries {
                    let delay =
                        parse_retry_after(resp.headers()).unwrap_or_else(|| backoff_delay(attempt));
                    drop(resp);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    // Must contain the literal substring "http 429" (case-
                    // insensitively) — `op_orchestrator::retry::is_non_retryable`
                    // pattern-matches on it to stop the subtask retry ladder
                    // at attempt 1 instead of burning two more full LLM calls
                    // on a rate limit that will still be in effect seconds
                    // later. The prior wording "(429)" (no "http") silently
                    // missed that classifier — a genuinely exhausted 429 was
                    // misread as retryable and got attempts 2/3 anyway.
                    return Err(BuiltinHttpError::RateLimited {
                        label: label.to_string(),
                        max_retries,
                    });
                }
                // Provider error bodies are untrusted and can echo request
                // headers. Never relay them into chat/SSE where credentials
                // could be reflected back into logs or browser responses.
                return Err(BuiltinHttpError::HttpStatus {
                    label: label.to_string(),
                    status,
                });
            }
            Err(e) => {
                if e.is_timeout() {
                    return Err(BuiltinHttpError::Timeout {
                        label: label.to_string(),
                        url: url.to_string(),
                        message: e.to_string(),
                    });
                }
                if attempt < max_retries {
                    tokio::time::sleep(backoff_delay(attempt)).await;
                    continue;
                }
                return Err(BuiltinHttpError::Transport {
                    label: label.to_string(),
                    url: url.to_string(),
                    message: e.to_string(),
                });
            }
        }
    }
    unreachable!("backoff loop always returns before exhausting range")
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

/// reqwest client with connect + overall timeouts. A bare `Client::new()` has
/// NO timeout, so a hung LLM endpoint (connection opens but the server never
/// streams, or stalls mid-response) makes the blocking provider iterator — and
/// therefore the orchestrator's planning / sub-agent call — hang forever
/// (desktop pinned on "Planning…", with Stop unable to interrupt the already
/// in-flight request). These deadlines surface the stall as an error so the
/// planning loop falls back instead of hanging. 300s is generous enough not to
/// kill a slow-but-live generation.
///
/// Reports [`crate::provider_dial::ProviderDialError::ClientBuild`] rather
/// than a local variant: this IS the `Trusted` half of `provider_dial`'s
/// dial, and the pinned `PublicOnly` half produces the same sentence for the
/// same reqwest failure — one variant keeps them from drifting apart.
pub(crate) fn builtin_http_client(
) -> Result<reqwest::Client, crate::provider_dial::ProviderDialError> {
    builtin_http_client_builder().build().map_err(|error| {
        crate::provider_dial::ProviderDialError::ClientBuild {
            message: error.to_string(),
        }
    })
}

/// Shared builder so pinned (DNS-screened) clients keep the same redirect
/// and timeout posture as the default provider client.
pub(crate) fn builtin_http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(BUILTIN_HTTP_CONNECT_TIMEOUT)
        .timeout(BUILTIN_HTTP_REQUEST_TIMEOUT)
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
