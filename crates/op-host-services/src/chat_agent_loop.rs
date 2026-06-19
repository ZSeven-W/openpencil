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

use crate::chat_builtin_http::{ensure_success, map_anthropic_stop_reason, map_openai_stop_reason};

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
        let resp = reqwest::Client::new()
            .post(&cfg.url)
            .header("x-api-key", &cfg.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("anthropic POST {}: {e}", cfg.url))?;
        let resp = ensure_success(resp, "anthropic").await?;
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
            results.push(json!({
                "type": "tool_result",
                "tool_use_id": call.id,
                "content": result.content,
                "is_error": result.is_error,
            }));
        }
        messages.push(json!({ "role": "user", "content": results }));
    }

    // Turn cap reached with the model still calling tools — stop the
    // loop the way the TS engine reports error_max_turns.
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
        let body = json!({
            "model": cfg.model,
            "stream": true,
            "max_tokens": cfg.max_output_tokens,
            "messages": messages,
            "tools": tools_json,
        });
        let resp = reqwest::Client::new()
            .post(&cfg.url)
            .bearer_auth(&cfg.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("openai-compatible POST {}: {e}", cfg.url))?;
        let resp = ensure_success(resp, "openai-compatible").await?;
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
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call.id,
                "content": result.content,
            }));
        }
    }

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
