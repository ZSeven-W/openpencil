//! API-key backed built-in chat providers.
//!
//! These mirror the TS app's built-in provider route without enabling
//! agent-rs concrete-provider features in this Rust build. The
//! implementation posts directly to Anthropic or OpenAI-compatible
//! streaming endpoints and converts SSE payloads into `ChatDelta`s.

use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures::StreamExt;
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
}

impl ConfiguredBuiltinProvider {
    pub fn from_builtin_agent(config: &BuiltinAgentConfig) -> Option<Self> {
        let base_url = if config.base_url.trim().is_empty() {
            config.kind.default_base_url()
        } else {
            config.base_url.trim()
        };
        let label = if config.display_name.trim().is_empty() {
            config.model.trim()
        } else {
            config.display_name.trim()
        };
        config.ready().then(|| Self {
            kind: config.kind,
            api_key: config.api_key.trim().to_string(),
            model: config.model.trim().to_string(),
            base_url: base_url.to_string(),
            label: label.to_string(),
            tools: Vec::new(),
            executor: None,
            finalize_on_exit: false,
        })
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
                    let _ = tx.send(ChatDelta::Error(e)).await;
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

/// MiniMax M 系("MiniMax-M*"、旧 "abab*")是推理模型,其思考由 MiniMax 专属的
/// `thinking` body 字段控制。据模型名判定,以便只对它发关思考字段。
pub(crate) fn is_minimax_model(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    m.starts_with("minimax") || m.starts_with("abab")
}

/// GLM-4.5+/GLM-5.x（智谱 / 方舟 coding plan）同为推理模型：reasoning 走
/// `reasoning_content`，不关思考会烧光输出预算把 `content` 留空（实测 glm-5.2
/// 一个设计子任务 thinking_len≈3 万、text_len=0，整段 parse 失败）。它接受和
/// MiniMax 同样的 `thinking:{type:"disabled"}`（curl 对 ark glm-5.2 验证：关思考后
/// reasoning_tokens=0、content 为干净 JSON）。按名判定，只对 GLM 下发。
pub(crate) fn is_glm_model(model: &str) -> bool {
    model.to_ascii_lowercase().contains("glm")
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

async fn throttle_builtin_http_request() {
    let adaptive = Duration::from_millis(
        BUILTIN_HTTP_ADAPTIVE_GAP_MS.load(std::sync::atomic::Ordering::Relaxed),
    );
    let min_gap = builtin_http_min_gap() + adaptive;
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

async fn send_with_backoff(
    label: &str,
    url: &str,
    build: impl Fn() -> reqwest::RequestBuilder,
) -> Result<reqwest::Response, String> {
    for attempt in 0..=BUILTIN_HTTP_MAX_RETRIES {
        throttle_builtin_http_request().await;
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
                if is_retryable_status(status) && attempt < BUILTIN_HTTP_MAX_RETRIES {
                    let delay =
                        parse_retry_after(resp.headers()).unwrap_or_else(|| backoff_delay(attempt));
                    drop(resp);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                let body = resp.text().await.unwrap_or_default();
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    // Raw provider JSON reads as gibberish in the chat
                    // transcript — say what happened and what to do.
                    return Err(format!(
                        "The model provider is rate-limiting this account (429) and the run                          could not ride it out after {BUILTIN_HTTP_MAX_RETRIES} retries. Wait                          a moment, then send the prompt again to continue. ({label}: {})",
                        body.trim()
                    ));
                }
                return Err(format!("{label} http {status}: {}", body.trim()));
            }
            Err(e) => {
                if attempt < BUILTIN_HTTP_MAX_RETRIES {
                    tokio::time::sleep(backoff_delay(attempt)).await;
                    continue;
                }
                return Err(format!("{label} POST {url}: {e}"));
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
) -> Result<bool, String> {
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
    // MiniMax M 系和 GLM-5.x 都是推理模型:不关思考会把 reasoning 烧到占满输出
    // 预算,JSON content 被截断甚至留空(glm-5.2 实测一个设计子任务 thinking≈3 万
    // 字符、content 0,整段 parse 失败、重试也撞同一堵墙)。当调用方明确要求关思考
    // (`disable_thinking`,如编排器的设计子任务),在线级下发 `thinking:{type:"disabled"}`
    // (实测 MiniMax 与 ark glm-5.2 都接受、返回干净 content)。普通对话走 `Adaptive`
    // 不进这里,保留推理。只对这两类加此字段,不碰别的 openai-compat(DeepSeek / Qwen)。
    if disable_thinking && (is_minimax_model(&provider.model) || is_glm_model(&provider.model)) {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("thinking".into(), json!({ "type": "disabled" }));
        }
    }
    let resp = send_with_backoff("openai-compatible", &url, || {
        builtin_http_client()
            .post(&url)
            .bearer_auth(&provider.api_key)
            .json(&body)
    })
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
fn builtin_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

async fn run_anthropic_chat(
    provider: ConfiguredBuiltinProvider,
    system_prompt: String,
    history: Vec<(ChatHistoryRole, String)>,
    prompt: String,
    max_output_tokens: u32,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, String> {
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
    let resp = send_with_backoff("anthropic", &url, || {
        builtin_http_client()
            .post(&url)
            .header("x-api-key", &provider.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
    })
    .await?;
    pump_sse_response(resp, tx, parse_anthropic_sse_data).await
}

pub async fn ensure_success(
    resp: reqwest::Response,
    provider_label: &str,
) -> Result<reqwest::Response, String> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(format!("{provider_label} http {status}: {}", body.trim()))
}

async fn pump_sse_response(
    resp: reqwest::Response,
    tx: &mpsc::Sender<ChatDelta>,
    parse: fn(&str) -> Option<ChatDelta>,
) -> Result<bool, String> {
    let mut stream = resp.bytes_stream();
    let mut buf = Vec::<u8>::new();
    let mut event_data = String::new();
    let mut emitted_done = false;

    while let Some(chunk) = stream.next().await {
        if tx.is_closed() {
            return Ok(true);
        }
        let bytes = chunk.map_err(|e| format!("sse stream: {e}"))?;
        buf.extend_from_slice(&bytes);
        while let Some(nl_pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl_pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                if emit_sse_event(&mut event_data, tx, parse).await {
                    emitted_done = true;
                    break;
                }
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                if !event_data.is_empty() {
                    event_data.push('\n');
                }
                event_data.push_str(data.trim_start());
            }
        }
        if emitted_done {
            break;
        }
    }

    if !emitted_done && !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(data) = line.strip_prefix("data:") {
            if !event_data.is_empty() {
                event_data.push('\n');
            }
            event_data.push_str(data.trim_start());
        }
    }
    if !emitted_done && emit_sse_event(&mut event_data, tx, parse).await {
        emitted_done = true;
    }

    Ok(emitted_done)
}

async fn emit_sse_event(
    event_data: &mut String,
    tx: &mpsc::Sender<ChatDelta>,
    parse: fn(&str) -> Option<ChatDelta>,
) -> bool {
    let data = event_data.trim();
    if data.is_empty() {
        event_data.clear();
        return false;
    }
    let Some(delta) = parse(data) else {
        event_data.clear();
        return false;
    };
    let emitted_done = matches!(delta, ChatDelta::Done { .. });
    let emitted_error = matches!(delta, ChatDelta::Error(_));
    if tx.send(delta).await.is_err() {
        event_data.clear();
        return true;
    }
    if emitted_error {
        let _ = tx
            .send(ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            })
            .await;
        event_data.clear();
        return true;
    }
    event_data.clear();
    emitted_done
}

fn provider_endpoint(base_url: &str, path: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with(path) {
        return base.to_string();
    }
    if path == "/v1/messages" && base.ends_with("/v1") {
        return format!("{base}/messages");
    }
    format!("{base}{path}")
}

fn parse_openai_sse_data(data: &str) -> Option<ChatDelta> {
    let data = data.trim();
    if data == "[DONE]" {
        return Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        });
    }
    let value: Value = serde_json::from_str(data).ok()?;
    if let Some(message) = value.pointer("/error/message").and_then(Value::as_str) {
        return Some(ChatDelta::Error(message.to_string()));
    }
    let choice = value.get("choices")?.as_array()?.first()?;
    if let Some(delta) = choice.get("delta") {
        if let Some(reasoning) = delta
            .get("reasoning_content")
            .or_else(|| delta.get("reasoning"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            return Some(ChatDelta::Thinking(reasoning.to_string()));
        }
        if let Some(content) = delta
            .get("content")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            return Some(ChatDelta::TextDelta(content.to_string()));
        }
    }
    choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(|reason| ChatDelta::Done {
            stop_reason: map_openai_stop_reason(reason),
        })
}

fn parse_anthropic_sse_data(data: &str) -> Option<ChatDelta> {
    let value: Value = serde_json::from_str(data.trim()).ok()?;
    match value.get("type").and_then(Value::as_str).unwrap_or("") {
        "content_block_delta" => {
            let delta = value.get("delta")?;
            match delta.get("type").and_then(Value::as_str).unwrap_or("") {
                "text_delta" => delta
                    .get("text")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| ChatDelta::TextDelta(s.to_string())),
                "thinking_delta" => delta
                    .get("thinking")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| ChatDelta::Thinking(s.to_string())),
                _ => None,
            }
        }
        "message_delta" => value
            .pointer("/delta/stop_reason")
            .and_then(Value::as_str)
            .map(|reason| ChatDelta::Done {
                stop_reason: map_anthropic_stop_reason(reason),
            }),
        "message_stop" => Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        }),
        "error" => value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .map(|message| ChatDelta::Error(message.to_string())),
        _ => None,
    }
}

pub fn map_anthropic_stop_reason(reason: &str) -> StopReason {
    match reason {
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "aborted" | "user_abort" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

pub fn map_openai_stop_reason(reason: &str) -> StopReason {
    match reason {
        "length" => StopReason::MaxTokens,
        "tool_calls" | "function_call" => StopReason::ToolUse,
        "content_filter" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc as std_mpsc;
    use std::time::Duration;

    fn read_http_request(stream: &mut TcpStream) -> String {
        let mut buf = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let n = stream.read(&mut chunk).expect("read request");
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            let Some(header_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&buf[..header_end]);
            let content_len = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if buf.len() >= header_end + 4 + content_len {
                break;
            }
        }
        String::from_utf8_lossy(&buf).to_string()
    }

    #[test]
    fn parse_openai_sse_data_extracts_text_delta() {
        let data = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
        assert_eq!(
            parse_openai_sse_data(data),
            Some(ChatDelta::TextDelta("hello".into()))
        );
    }

    #[test]
    fn parse_anthropic_sse_data_extracts_text_delta() {
        let data = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hello"}}"#;
        assert_eq!(
            parse_anthropic_sse_data(data),
            Some(ChatDelta::TextDelta("hello".into()))
        );
    }

    #[test]
    fn is_minimax_model_gates_thinking_field() {
        // 仅 MiniMax 模型加 `thinking:{type:disabled}`;别的 provider 不加。
        assert!(is_minimax_model("MiniMax-M3"));
        assert!(is_minimax_model("MiniMax-M2.7"));
        assert!(is_minimax_model("abab6.5s-chat"));
        assert!(!is_minimax_model("deepseek-v4-pro"));
        assert!(!is_minimax_model("qwen3-coder-plus"));
        assert!(!is_minimax_model("ark-code-latest"));
    }

    #[test]
    fn is_retryable_status_flags_provider_rate_limit_and_overload() {
        assert!(is_retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(
            reqwest::StatusCode::SERVICE_UNAVAILABLE
        ));
        assert!(is_retryable_status(
            reqwest::StatusCode::from_u16(529).expect("status 529")
        ));

        assert!(!is_retryable_status(reqwest::StatusCode::OK));
        assert!(!is_retryable_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!is_retryable_status(reqwest::StatusCode::UNAUTHORIZED));
        assert!(!is_retryable_status(reqwest::StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(
            reqwest::StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS
        ));
    }

    #[test]
    fn parse_retry_after_accepts_integer_seconds_and_caps_large_values() {
        let mut headers = reqwest::header::HeaderMap::new();
        assert_eq!(parse_retry_after(&headers), None);

        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("3"),
        );
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(3)));

        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("0"),
        );
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(0)));

        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("abc"),
        );
        assert_eq!(parse_retry_after(&headers), None);

        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("3600"),
        );
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(30)));
    }

    #[test]
    fn backoff_delay_exponentially_increases_and_caps() {
        assert_eq!(backoff_delay(0), Duration::from_secs(1));
        assert_eq!(backoff_delay(1), Duration::from_secs(2));
        assert_eq!(backoff_delay(2), Duration::from_secs(4));
        assert_eq!(backoff_delay(3), Duration::from_secs(8));
        assert_eq!(backoff_delay(99), Duration::from_secs(8));
    }

    #[test]
    fn throttle_wait_respects_reserved_last_request_slot() {
        let now = std::time::Instant::now();
        let min_gap = Duration::from_millis(350);

        assert_eq!(throttle_wait(None, now, min_gap), Duration::ZERO);
        assert_eq!(throttle_wait(Some(now), now, min_gap), min_gap);
        assert_eq!(
            throttle_wait(Some(now - Duration::from_millis(400)), now, min_gap),
            Duration::ZERO
        );
    }

    #[test]
    fn openai_sse_error_finishes_aborted() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local SSE server");
        let addr = listener.local_addr().expect("local addr");
        let (req_tx, req_rx) = std_mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("set read timeout");
            let request = read_http_request(&mut stream);
            req_tx.send(request).expect("send request capture");

            let body = "data: {\"error\":{\"message\":\"bad key\"}}\n\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write SSE response");
        });
        let provider = ConfiguredBuiltinProvider::from_builtin_agent(&BuiltinAgentConfig {
            id: "builtin-1".into(),
            preset: op_editor_core::BuiltinAgentPresetKey::Custom,
            display_name: "Mock OpenAI".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "sk-test".into(),
            model: "gpt-test".into(),
            base_url: format!("http://{addr}"),
            enabled: true,
        })
        .expect("ready provider");

        let deltas: Vec<ChatDelta> = provider
            .send(ChatRequest {
                user_message: "hello".into(),
                max_output_tokens: 64,
                ..Default::default()
            })
            .collect();
        server.join().expect("server thread exits");
        let request = req_rx
            .recv()
            .expect("captured request")
            .to_ascii_lowercase();

        assert!(request.starts_with("post /chat/completions "));
        assert!(request.contains("authorization: bearer sk-test"));
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d, ChatDelta::Error(message) if message == "bad key")),
            "expected upstream error delta, got {deltas:?}"
        );
        assert!(
            matches!(
                deltas.last(),
                Some(ChatDelta::Done {
                    stop_reason: StopReason::Aborted
                })
            ),
            "SSE errors must terminate as aborted, got {deltas:?}"
        );
    }
}
