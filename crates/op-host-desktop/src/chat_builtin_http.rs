//! API-key backed built-in chat providers.
//!
//! These mirror the TS app's built-in provider route without enabling
//! agent-rs concrete-provider features in this Rust build. The
//! implementation posts directly to Anthropic or OpenAI-compatible
//! streaming endpoints and converts SSE payloads into `ChatDelta`s.

use std::fmt;
use std::sync::Arc;

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
                    max_turns: MAX_TOOL_TURNS,
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
fn is_minimax_model(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    m.starts_with("minimax") || m.starts_with("abab")
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
    // MiniMax M 系是推理模型,默认把 `<think>…</think>` 注进 `content`,既烧光
    // 输出预算(JSON 被截断)又逼解析层去剥 think。当调用方明确要求关思考时
    // (`disable_thinking`,如编排器的设计子任务),在线级关掉(实测确认 MiniMax
    // 接受 `thinking:{type:"disabled"}` 返回干净 content)。普通对话用 `Adaptive`、
    // 不会进这里,保留 M3 推理。仅对 MiniMax 加此字段,不碰别的 openai-compat
    // provider(DeepSeek / 方舟 / Qwen)。
    if disable_thinking && is_minimax_model(&provider.model) {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("thinking".into(), json!({ "type": "disabled" }));
        }
    }
    let resp = reqwest::Client::new()
        .post(&url)
        .bearer_auth(&provider.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("openai-compatible POST {url}: {e}"))?;
    let resp = ensure_success(resp, "openai-compatible").await?;
    pump_sse_response(resp, tx, parse_openai_sse_data).await
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
    let resp = reqwest::Client::new()
        .post(&url)
        .header("x-api-key", &provider.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("anthropic POST {url}: {e}"))?;
    let resp = ensure_success(resp, "anthropic").await?;
    pump_sse_response(resp, tx, parse_anthropic_sse_data).await
}

pub(crate) async fn ensure_success(
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

pub(crate) fn map_anthropic_stop_reason(reason: &str) -> StopReason {
    match reason {
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "aborted" | "user_abort" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

pub(crate) fn map_openai_stop_reason(reason: &str) -> StopReason {
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
