//! Tool-executing agent loop for the builtin (API-key) chat providers.
//!
//! Mirrors the TS agent pipeline (`apps/web/server/api/ai/agent.ts` +
//! the Zig engine behind it): the request carries canvas tool
//! definitions, `tool_use` comes back in the SSE stream, the host
//! executes the call against the live document, and the result rides
//! a follow-up request as `tool_result` — looping until the model
//! stops calling tools or the turn cap is reached (TS `maxTurns: 20`).
//!
//! Both builtin wires are covered:
//! - **Anthropic** `/v1/messages` — `tools[]` + `tool_use` content
//!   blocks (`input_json_delta` accumulation) + `tool_result` blocks.
//! - **OpenAI-compatible** `/chat/completions` — `tools[]` function
//!   defs + streamed `delta.tool_calls` fragments + `role:"tool"`
//!   result messages.
//!
//! Execution itself goes through the [`ChatToolExecutor`] the caller
//! injected (production: `chat_canvas_tools::UiChatToolExecutor`,
//! which forwards to the UI thread). The loop is transport-pure and
//! testable against a loopback mock server with a scripted executor.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use op_ai::chat_provider::{
    ChatDelta, ChatHistoryRole, ChatToolDef, ChatToolExecutor, ChatToolResult, StopReason,
};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::chat_builtin_http::{map_anthropic_stop_reason, map_openai_stop_reason};

/// Everything one agent-loop run needs. `max_turns` is the TS
/// `maxTurns` cap (20 in production; tests shrink it).
pub struct AgentLoopConfig {
    pub url: String,
    pub api_key: String,
    pub model: String,
    pub system_prompt: String,
    pub history: Vec<(ChatHistoryRole, String)>,
    pub user_prompt: String,
    pub max_output_tokens: u32,
    pub tools: Vec<ChatToolDef>,
    pub executor: Arc<dyn ChatToolExecutor>,
    pub max_turns: usize,
    /// When true, run the deterministic structural-quality backstop
    /// ([`run_loop_finalize`] → `op_orchestrator::apply_loop_finalize`)
    /// once at loop end. Set ONLY for the gated design-generation loop;
    /// the regular tool-using chat loop leaves this `false` so an ordinary
    /// chat turn never re-runs the Class-A passes over — and mutates — an
    /// existing design (Track-1 Step 4 scoping fix). The agent loop is the
    /// shared execution path for BOTH regular builtin chat and the design
    /// loop, so the finalize side-effect MUST be opt-in per provider.
    pub finalize_on_exit: bool,
    /// When true AND the model is a MiniMax / GLM reasoning model, send
    /// `thinking:{type:"disabled"}` in the OpenAI-compat body. Without it a
    /// reasoning model (glm-5.2) streams tens of thousands of hidden reasoning
    /// tokens per turn — measured 94k thinking chars vs 4k of actual
    /// `batch_design` DSL — which burns the per-turn `max_output_tokens` and
    /// truncates the design mid-build. The single-shot builtin chat path
    /// already gates this on the same flag; the loop body was missing it.
    pub disable_thinking: bool,
}

impl AgentLoopConfig {
    fn level_for(&self, tool: &str) -> String {
        self.tools
            .iter()
            .find(|t| t.name == tool)
            .map(|t| t.level.clone())
            .unwrap_or_else(|| "read".to_string())
    }
}

/// One fully-accumulated model tool call.
#[derive(Debug, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    args_json: String,
}

/// Build the transcript tool-card payload for a `ChatDelta::ToolUse`.
/// The chat panel's tool card (`ai_chat_transcript_tools.rs`) parses
/// this envelope: `level` picks the expand default, `args` renders,
/// `status: "running"` animates until the host attaches the result.
pub fn tool_card_envelope(level: &str, args_json: &str) -> String {
    let args = serde_json::from_str::<Value>(args_json)
        .unwrap_or_else(|_| Value::String(args_json.to_string()));
    json!({ "level": level, "args": args, "status": "running" }).to_string()
}

/// Empty / whitespace accumulated tool arguments normalize to `{}`.
fn normalized_args(args_json: &str) -> String {
    let trimmed = args_json.trim();
    if trimmed.is_empty() {
        "{}".to_string()
    } else {
        trimmed.to_string()
    }
}

/// If a tool result is a `get_screenshot` payload, pull out the base64 PNG
/// so it can be replayed to the model as a real image content block.
///
/// `get_screenshot` (see `mcp_serve::screenshot_tool.rs`) returns its result
/// as a JSON object `{"image_base64": <b64>, "format": "png"}`. Without this
/// the base64 rides the follow-up request as an opaque text string and the
/// model literally cannot see the render — defeating the self-correction
/// loop. We detect the screenshot result by tool name AND the structured
/// marker (`format == "png"` + a non-empty `image_base64`) so a tool that
/// happened to be renamed, or an error envelope, falls back to plain text.
///
/// Returns `Some(base64)` only for a successful, non-error screenshot result.
fn screenshot_image_base64(tool_name: &str, result: &ChatToolResult) -> Option<String> {
    if tool_name != "get_screenshot" || result.is_error {
        return None;
    }
    let v = serde_json::from_str::<Value>(&result.content).ok()?;
    if v.get("format").and_then(Value::as_str) != Some("png") {
        return None;
    }
    let b64 = v.get("image_base64").and_then(Value::as_str)?;
    if b64.is_empty() {
        return None;
    }
    Some(b64.to_string())
}

/// Run one tool call through the executor on a blocking thread (the
/// executor blocks on the UI ack; never block the runtime directly).
async fn execute_tool(
    executor: &Arc<dyn ChatToolExecutor>,
    name: &str,
    args_json: &str,
) -> ChatToolResult {
    let executor = executor.clone();
    let name = name.to_string();
    let args = normalized_args(args_json);
    tokio::task::spawn_blocking(move || executor.execute(&name, &args))
        .await
        .unwrap_or_else(|e| ChatToolResult {
            content: json!({ "success": false, "error": format!("tool executor panicked: {e}") })
                .to_string(),
            is_error: true,
        })
}

/// Run the deterministic structural-quality backstop once, at loop end
/// (Track-1 Step 4). Forwards to the executor's `finalize` (which the desktop
/// host bridges to `op_orchestrator::apply_loop_finalize` against the live
/// `EditorState`). Runs on a blocking thread — the host round-trip blocks until
/// the UI thread acks — mirroring [`execute_tool`]. The default executor
/// `finalize` is a no-op, so this is inert for scripted / read-only executors.
///
/// `enabled` is `cfg.finalize_on_exit`: a no-op early-return when this loop run
/// is a regular chat turn (the shared agent loop also serves plain builtin
/// chat), so only the gated design-generation loop mutates the document here.
async fn run_loop_finalize(executor: &Arc<dyn ChatToolExecutor>, enabled: bool) {
    if !enabled {
        return;
    }
    let executor = executor.clone();
    let _ = tokio::task::spawn_blocking(move || executor.finalize()).await;
}

// ---------------------------------------------------------------------------
// Shared SSE pump
// ---------------------------------------------------------------------------

/// Stateful per-event handler. `handle` returns the deltas to forward
/// to the chat channel for one SSE `data:` payload.
trait SseCollector {
    fn handle(&mut self, data: &str) -> Vec<ChatDelta>;
}

/// Drain `resp`'s SSE body through `collector`, forwarding returned
/// deltas to `tx`. Line/event framing mirrors
/// `chat_builtin_http::pump_sse_response` (multi-line `data:`
/// accumulation, trailing-buffer flush) but hands events to a
/// stateful collector instead of a pure parse fn — tool-call
/// accumulation spans many events.
async fn pump_sse<C: SseCollector>(
    resp: reqwest::Response,
    tx: &mpsc::Sender<ChatDelta>,
    collector: &mut C,
) -> Result<(), String> {
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut event_data = String::new();

    while let Some(chunk) = stream.next().await {
        if tx.is_closed() {
            return Ok(());
        }
        let bytes = chunk.map_err(|e| format!("sse stream: {e}"))?;
        buf.extend_from_slice(&bytes);
        while let Some(nl_pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl_pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                dispatch_event(&mut event_data, tx, collector).await;
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                if !event_data.is_empty() {
                    event_data.push('\n');
                }
                event_data.push_str(data.trim_start());
            }
        }
    }

    if !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(data) = line.strip_prefix("data:") {
            if !event_data.is_empty() {
                event_data.push('\n');
            }
            event_data.push_str(data.trim_start());
        }
    }
    dispatch_event(&mut event_data, tx, collector).await;
    Ok(())
}

async fn dispatch_event<C: SseCollector>(
    event_data: &mut String,
    tx: &mpsc::Sender<ChatDelta>,
    collector: &mut C,
) {
    let data = event_data.trim();
    if data.is_empty() {
        event_data.clear();
        return;
    }
    let deltas = collector.handle(data);
    event_data.clear();
    for delta in deltas {
        if tx.send(delta).await.is_err() {
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Anthropic wire
// ---------------------------------------------------------------------------

/// Per-index content-block accumulation state for one Anthropic turn.
enum AnthropicBlock {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        json: String,
    },
    Other,
}

#[derive(Default)]
struct AnthropicCollector {
    blocks: BTreeMap<u64, AnthropicBlock>,
    stop_reason: Option<String>,
    error: Option<String>,
}

impl AnthropicCollector {
    fn tool_calls(&self) -> Vec<PendingToolCall> {
        self.blocks
            .values()
            .filter_map(|b| match b {
                AnthropicBlock::ToolUse { id, name, json } => Some(PendingToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    args_json: normalized_args(json),
                }),
                _ => None,
            })
            .collect()
    }

    /// Assistant `content[]` for the follow-up request — text + the
    /// tool_use blocks in stream order. (Thinking blocks are not
    /// replayed: the builtin chat request never enables thinking, so
    /// none arrive with signatures worth preserving.)
    fn assistant_content(&self) -> Vec<Value> {
        self.blocks
            .values()
            .filter_map(|b| match b {
                AnthropicBlock::Text(text) if !text.is_empty() => {
                    Some(json!({ "type": "text", "text": text }))
                }
                AnthropicBlock::ToolUse {
                    id,
                    name,
                    json: args,
                } => {
                    let input = serde_json::from_str::<Value>(&normalized_args(args))
                        .unwrap_or_else(|_| json!({}));
                    Some(json!({ "type": "tool_use", "id": id, "name": name, "input": input }))
                }
                _ => None,
            })
            .collect()
    }
}

impl SseCollector for AnthropicCollector {
    fn handle(&mut self, data: &str) -> Vec<ChatDelta> {
        let Ok(value) = serde_json::from_str::<Value>(data) else {
            return Vec::new();
        };
        match value.get("type").and_then(Value::as_str).unwrap_or("") {
            "content_block_start" => {
                let index = value.get("index").and_then(Value::as_u64).unwrap_or(0);
                let block = value.get("content_block");
                let kind = block
                    .and_then(|b| b.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let entry = match kind {
                    "text" => AnthropicBlock::Text(String::new()),
                    "tool_use" => AnthropicBlock::ToolUse {
                        id: block
                            .and_then(|b| b.get("id"))
                            .and_then(Value::as_str)
                            .unwrap_or("toolu_unknown")
                            .to_string(),
                        name: block
                            .and_then(|b| b.get("name"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        json: String::new(),
                    },
                    _ => AnthropicBlock::Other,
                };
                self.blocks.insert(index, entry);
                Vec::new()
            }
            "content_block_delta" => {
                let index = value.get("index").and_then(Value::as_u64).unwrap_or(0);
                let Some(delta) = value.get("delta") else {
                    return Vec::new();
                };
                match delta.get("type").and_then(Value::as_str).unwrap_or("") {
                    "text_delta" => {
                        let Some(text) = delta.get("text").and_then(Value::as_str) else {
                            return Vec::new();
                        };
                        if let Some(AnthropicBlock::Text(acc)) = self.blocks.get_mut(&index) {
                            acc.push_str(text);
                        } else {
                            self.blocks
                                .insert(index, AnthropicBlock::Text(text.to_string()));
                        }
                        if text.is_empty() {
                            Vec::new()
                        } else {
                            vec![ChatDelta::TextDelta(text.to_string())]
                        }
                    }
                    "thinking_delta" => delta
                        .get("thinking")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(|s| vec![ChatDelta::Thinking(s.to_string())])
                        .unwrap_or_default(),
                    "input_json_delta" => {
                        if let Some(partial) = delta.get("partial_json").and_then(Value::as_str) {
                            if let Some(AnthropicBlock::ToolUse { json, .. }) =
                                self.blocks.get_mut(&index)
                            {
                                json.push_str(partial);
                            }
                        }
                        Vec::new()
                    }
                    _ => Vec::new(),
                }
            }
            "message_delta" => {
                if let Some(reason) = value.pointer("/delta/stop_reason").and_then(Value::as_str) {
                    self.stop_reason = Some(reason.to_string());
                }
                Vec::new()
            }
            "error" => {
                let message = value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .unwrap_or("anthropic stream error")
                    .to_string();
                self.error = Some(message);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }
}

/// Run the Anthropic agent loop to completion. Returns `Ok(true)` when
/// a terminal `Done` was emitted; `Err` for transport / in-stream
/// errors (caller surfaces them as `Error + Done{Aborted}`).
pub async fn run_anthropic_agent_loop(
    cfg: AgentLoopConfig,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, String> {
    let tools_json: Vec<Value> = cfg
        .tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": serde_json::from_str::<Value>(&t.input_schema_json)
                    .unwrap_or_else(|_| json!({ "type": "object" })),
            })
        })
        .collect();
    let mut messages: Vec<Value> = Vec::new();
    for (role, text) in &cfg.history {
        messages.push(json!({ "role": role.as_str(), "content": text }));
    }
    messages.push(json!({ "role": "user", "content": cfg.user_prompt }));

    for _turn in 0..cfg.max_turns.max(1) {
        let mut body = json!({
            "model": cfg.model,
            "max_tokens": cfg.max_output_tokens,
            "stream": true,
            "messages": messages,
            "tools": tools_json,
        });
        if !cfg.system_prompt.trim().is_empty() {
            body.as_object_mut()
                .expect("anthropic request body is object")
                .insert("system".into(), json!(cfg.system_prompt));
        }
        let client = crate::chat_builtin_http::builtin_http_client()?;
        let (max_retries, min_gap) = crate::chat_builtin_http::default_backoff_knobs();
        let resp = crate::chat_builtin_http::send_with_backoff(
            "anthropic",
            &cfg.url,
            max_retries,
            min_gap,
            || {
                client
                    .post(&cfg.url)
                    .header("x-api-key", &cfg.api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body)
            },
        )
        .await?;
        let mut collector = AnthropicCollector::default();
        pump_sse(resp, tx, &mut collector).await?;
        if tx.is_closed() {
            return Ok(true);
        }
        if let Some(err) = collector.error {
            return Err(err);
        }
        let calls = collector.tool_calls();
        if calls.is_empty() {
            let reason = collector
                .stop_reason
                .as_deref()
                .map(map_anthropic_stop_reason)
                .unwrap_or(StopReason::EndTurn);
            // Normal model-stop exit: run the Step-4 structural backstop ONCE
            // over the assembled doc BEFORE the Done delta, so the finalized
            // document is what the UI persists/displays for this turn.
            run_loop_finalize(&cfg.executor, cfg.finalize_on_exit).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: reason,
                })
                .await;
            return Ok(true);
        }

        messages.push(json!({ "role": "assistant", "content": collector.assistant_content() }));
        let mut results: Vec<Value> = Vec::new();
        for call in &calls {
            let level = cfg.level_for(&call.name);
            let _ = tx
                .send(ChatDelta::ToolUse {
                    name: call.name.clone(),
                    args: tool_card_envelope(&level, &call.args_json),
                })
                .await;
            let result = execute_tool(&cfg.executor, &call.name, &call.args_json).await;
            // When the result is a `get_screenshot` PNG, feed it back as a
            // real Anthropic image content block instead of an opaque base64
            // text string — otherwise the model can never see the render.
            let content: Value = match screenshot_image_base64(&call.name, &result) {
                Some(b64) => json!([
                    { "type": "text", "text": "Rendered screenshot of the current design:" },
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": b64,
                        },
                    },
                ]),
                None => Value::String(result.content),
            };
            results.push(json!({
                "type": "tool_result",
                "tool_use_id": call.id,
                "content": content,
                "is_error": result.is_error,
            }));
        }
        messages.push(json!({ "role": "user", "content": results }));
    }

    // Turn cap reached with the model still calling tools — stop the
    // loop the way the TS engine reports error_max_turns. Still run the Step-4
    // structural backstop ONCE over whatever the truncated turn assembled.
    run_loop_finalize(&cfg.executor, cfg.finalize_on_exit).await;
    let _ = tx
        .send(ChatDelta::Done {
            stop_reason: StopReason::MaxTokens,
        })
        .await;
    Ok(true)
}

// ---------------------------------------------------------------------------
// OpenAI-compatible wire
// ---------------------------------------------------------------------------

#[derive(Default)]
struct OpenAiToolCallAcc {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Default)]
struct OpenAiCollector {
    text: String,
    tool_calls: BTreeMap<u64, OpenAiToolCallAcc>,
    finish_reason: Option<String>,
    error: Option<String>,
}

impl SseCollector for OpenAiCollector {
    fn handle(&mut self, data: &str) -> Vec<ChatDelta> {
        if data == "[DONE]" {
            return Vec::new();
        }
        let Ok(value) = serde_json::from_str::<Value>(data) else {
            return Vec::new();
        };
        if let Some(message) = value.pointer("/error/message").and_then(Value::as_str) {
            self.error = Some(message.to_string());
            return Vec::new();
        }
        let Some(choice) = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|c| c.first())
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Some(delta) = choice.get("delta") {
            if let Some(reasoning) = delta
                .get("reasoning_content")
                .or_else(|| delta.get("reasoning"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                out.push(ChatDelta::Thinking(reasoning.to_string()));
            }
            if let Some(content) = delta
                .get("content")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                self.text.push_str(content);
                out.push(ChatDelta::TextDelta(content.to_string()));
            }
            if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    let index = call.get("index").and_then(Value::as_u64).unwrap_or(0);
                    let acc = self.tool_calls.entry(index).or_default();
                    if let Some(id) = call.get("id").and_then(Value::as_str) {
                        if !id.is_empty() {
                            acc.id = id.to_string();
                        }
                    }
                    if let Some(name) = call.pointer("/function/name").and_then(Value::as_str) {
                        if !name.is_empty() {
                            acc.name = name.to_string();
                        }
                    }
                    if let Some(args) = call.pointer("/function/arguments").and_then(Value::as_str)
                    {
                        acc.arguments.push_str(args);
                    }
                }
            }
        }
        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            self.finish_reason = Some(reason.to_string());
        }
        out
    }
}

impl OpenAiCollector {
    fn pending_calls(&self) -> Vec<(u64, PendingToolCall)> {
        self.tool_calls
            .iter()
            .filter(|(_, acc)| !acc.name.is_empty())
            .map(|(index, acc)| {
                let id = if acc.id.is_empty() {
                    format!("call_{index}")
                } else {
                    acc.id.clone()
                };
                (
                    *index,
                    PendingToolCall {
                        id,
                        name: acc.name.clone(),
                        args_json: normalized_args(&acc.arguments),
                    },
                )
            })
            .collect()
    }
}

/// Run the OpenAI-compatible agent loop to completion. Same contract
/// as [`run_anthropic_agent_loop`].
pub async fn run_openai_agent_loop(
    cfg: AgentLoopConfig,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, String> {
    let tools_json: Vec<Value> = cfg
        .tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema_json)
                        .unwrap_or_else(|_| json!({ "type": "object" })),
                },
            })
        })
        .collect();
    let mut messages: Vec<Value> = Vec::new();
    if !cfg.system_prompt.trim().is_empty() {
        messages.push(json!({ "role": "system", "content": cfg.system_prompt }));
    }
    for (role, text) in &cfg.history {
        messages.push(json!({ "role": role.as_str(), "content": text }));
    }
    messages.push(json!({ "role": "user", "content": cfg.user_prompt }));

    for _turn in 0..cfg.max_turns.max(1) {
        let mut body = json!({
            "model": cfg.model,
            "stream": true,
            "max_tokens": cfg.max_output_tokens,
            "messages": messages,
            "tools": tools_json,
        });
        // Turn OFF hidden reasoning for MiniMax / GLM. Without this a glm-5.2
        // design turn spends its whole `max_tokens` on `reasoning_content` and
        // truncates the `batch_design` mid-JSON — the single-shot builtin body
        // gates the same field on the same flag (`chat_builtin_http`), but the
        // loop body was missing it, so every loop turn leaked thinking.
        if cfg.disable_thinking
            && (crate::chat_builtin_http::is_minimax_model(&cfg.model)
                || crate::chat_builtin_http::is_glm_model(&cfg.model))
        {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("thinking".into(), json!({ "type": "disabled" }));
            }
        }
        // Through the shared throttle/backoff: this tool-loop path used
        // to post raw, so a provider rate limit killed the design run with
        // no retries and a raw JSON error (measured: glm-5.2, 429
        // AccountRateLimitExceeded, 2026-07-12).
        let client = crate::chat_builtin_http::builtin_http_client()?;
        let (max_retries, min_gap) = crate::chat_builtin_http::default_backoff_knobs();
        let resp = crate::chat_builtin_http::send_with_backoff(
            "openai-compatible",
            &cfg.url,
            max_retries,
            min_gap,
            || client.post(&cfg.url).bearer_auth(&cfg.api_key).json(&body),
        )
        .await?;
        let mut collector = OpenAiCollector::default();
        pump_sse(resp, tx, &mut collector).await?;
        if tx.is_closed() {
            return Ok(true);
        }
        if let Some(err) = collector.error {
            return Err(err);
        }
        let calls = collector.pending_calls();
        if calls.is_empty() {
            let reason = collector
                .finish_reason
                .as_deref()
                .map(map_openai_stop_reason)
                .unwrap_or(StopReason::EndTurn);
            // Normal model-stop exit: run the Step-4 structural backstop ONCE
            // over the assembled doc BEFORE the Done delta.
            run_loop_finalize(&cfg.executor, cfg.finalize_on_exit).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: reason,
                })
                .await;
            return Ok(true);
        }

        let tool_calls_json: Vec<Value> = calls
            .iter()
            .map(|(_, call)| {
                json!({
                    "id": call.id,
                    "type": "function",
                    "function": { "name": call.name, "arguments": call.args_json },
                })
            })
            .collect();
        let content = if collector.text.is_empty() {
            Value::Null
        } else {
            Value::String(collector.text.clone())
        };
        messages.push(json!({
            "role": "assistant",
            "content": content,
            "tool_calls": tool_calls_json,
        }));
        for (_, call) in &calls {
            let level = cfg.level_for(&call.name);
            let _ = tx
                .send(ChatDelta::ToolUse {
                    name: call.name.clone(),
                    args: tool_card_envelope(&level, &call.args_json),
                })
                .await;
            let result = execute_tool(&cfg.executor, &call.name, &call.args_json).await;
            // OpenAI `role:"tool"` messages only accept a string `content`,
            // so an image can't ride the tool result directly. When the
            // result is a `get_screenshot` PNG, send a short text ack as the
            // tool result and follow it with a `role:"user"` message carrying
            // the render as an `image_url` data URL — the only OpenAI-wire
            // way to make the model actually see the screenshot.
            match screenshot_image_base64(&call.name, &result) {
                Some(b64) => {
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call.id,
                        "content": "Rendered screenshot attached as an image in the following message.",
                    }));
                    messages.push(json!({
                        "role": "user",
                        "content": [
                            { "type": "text", "text": "Rendered screenshot of the current design:" },
                            {
                                "type": "image_url",
                                "image_url": { "url": format!("data:image/png;base64,{b64}") },
                            },
                        ],
                    }));
                }
                None => {
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call.id,
                        "content": result.content,
                    }));
                }
            }
        }
    }

    // Turn cap reached — run the Step-4 structural backstop ONCE over whatever
    // the truncated turn assembled.
    run_loop_finalize(&cfg.executor, cfg.finalize_on_exit).await;
    let _ = tx
        .send(ChatDelta::Done {
            stop_reason: StopReason::MaxTokens,
        })
        .await;
    Ok(true)
}

#[cfg(test)]
#[path = "chat_agent_loop_tests.rs"]
mod tests;
