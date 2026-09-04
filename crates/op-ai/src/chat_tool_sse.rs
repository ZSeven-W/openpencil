//! Transport-free tool-call SSE accumulation for the agent tool loops.
//!
//! Streamed tool calls span many SSE events (OpenAI `tool_calls` argument
//! fragments, Anthropic `input_json_delta` blocks), so unlike the pure
//! one-payload parsers in [`crate::chat_sse`] these collectors are stateful:
//! feed every `data:` payload of one response through [`handle`]
//! (forwarding the returned deltas to the transcript channel), then read the
//! accumulated calls / text / stop state once the stream ends.
//!
//! Moved here from `op-host-services/src/chat_agent_loop/{openai,anthropic}.rs`
//! (which re-export them unchanged) so the desktop / daemon agent loop and the
//! mobile FFI design loop (`op-engine-ffi`) accumulate the wire protocol
//! through ONE implementation instead of drifting copies — the same migration
//! `chat_sse`'s plain parsers already made. Byte-level transports (reqwest
//! pumps, retry ladders) stay host-side.
//!
//! [`handle`]: OpenAiToolCollector::handle

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::chat_provider::ChatDelta;

/// One fully-accumulated model tool call.
#[derive(Debug, Clone)]
pub struct PendingToolCall {
    pub id: String,
    pub name: String,
    pub args_json: String,
}

/// Build the transcript tool-card payload for a [`ChatDelta::ToolUse`].
/// The chat panel's tool card (`ai_chat_transcript_tools.rs`) parses
/// this envelope: `level` picks the expand default, `args` renders,
/// `status: "running"` animates until the host attaches the result.
pub fn tool_card_envelope(level: &str, args_json: &str) -> String {
    let args = serde_json::from_str::<Value>(args_json)
        .unwrap_or_else(|_| Value::String(args_json.to_string()));
    json!({ "level": level, "args": args, "status": "running" }).to_string()
}

/// Empty / whitespace accumulated tool arguments normalize to `{}`.
pub fn normalized_args(args_json: &str) -> String {
    let trimmed = args_json.trim();
    if trimmed.is_empty() {
        "{}".to_string()
    } else {
        trimmed.to_string()
    }
}

// ---------------------------------------------------------------------------
// OpenAI-compatible wire
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct OpenAiToolCallAcc {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Accumulates one OpenAI-compatible streaming response: text deltas are
/// forwarded (and kept for the follow-up assistant message), tool-call
/// fragments are stitched per index, `finish_reason` / in-stream `error`
/// are recorded for the caller.
#[derive(Default)]
pub struct OpenAiToolCollector {
    pub text: String,
    pub tool_calls: BTreeMap<u64, OpenAiToolCallAcc>,
    pub finish_reason: Option<String>,
    pub error: Option<String>,
}

impl OpenAiToolCollector {
    /// Digest one SSE `data:` payload; returns the deltas to forward.
    pub fn handle(&mut self, data: &str) -> Vec<ChatDelta> {
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

    /// The accumulated tool calls in stream-index order. Calls that never
    /// received a name (pure argument fragments) are dropped; missing ids
    /// synthesize `call_<index>`.
    pub fn pending_calls(&self) -> Vec<(u64, PendingToolCall)> {
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

// ---------------------------------------------------------------------------
// Anthropic wire
// ---------------------------------------------------------------------------

/// Per-index content-block accumulation state for one Anthropic turn.
pub enum AnthropicBlock {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        json: String,
    },
    Other,
}

/// Accumulates one Anthropic `/v1/messages` streaming response, block by
/// block: text deltas forward, `input_json_delta` fragments stitch onto
/// their `tool_use` block, `message_delta` records the stop reason.
#[derive(Default)]
pub struct AnthropicToolCollector {
    pub blocks: BTreeMap<u64, AnthropicBlock>,
    pub stop_reason: Option<String>,
    pub error: Option<String>,
}

impl AnthropicToolCollector {
    /// Digest one SSE `data:` payload; returns the deltas to forward.
    pub fn handle(&mut self, data: &str) -> Vec<ChatDelta> {
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

    /// The accumulated tool calls in block order. Blocks that never
    /// received a name (a malformed / truncated `content_block_start`)
    /// are dropped — same discipline as
    /// [`OpenAiToolCollector::pending_calls`], so a nameless call can
    /// never reach the executor as `""` and burn a turn on a guaranteed
    /// "tool not available" error.
    pub fn tool_calls(&self) -> Vec<PendingToolCall> {
        self.blocks
            .values()
            .filter_map(|b| match b {
                AnthropicBlock::ToolUse { id, name, json } if !name.is_empty() => {
                    Some(PendingToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        args_json: normalized_args(json),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Assistant `content[]` for the follow-up request — text + the
    /// tool_use blocks in stream order. (Thinking blocks are not
    /// replayed: the builtin chat request never enables thinking, so
    /// none arrive with signatures worth preserving.)
    pub fn assistant_content(&self) -> Vec<Value> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_collector_drops_nameless_tool_blocks() {
        // A malformed / truncated `content_block_start` can open a tool_use
        // block with no name; it must never reach the executor as "" (same
        // discipline as the OpenAI collector's pending_calls filter).
        let mut collector = AnthropicToolCollector::default();
        collector.handle(
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1"}}"#,
        );
        collector.handle(
            r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_2","name":"update_node"}}"#,
        );
        let calls = collector.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "update_node");
    }

    #[test]
    fn openai_collector_drops_nameless_tool_calls() {
        let mut collector = OpenAiToolCollector::default();
        collector.handle(
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c0","function":{"arguments":"{}"}}]}}]}"#,
        );
        collector.handle(
            r#"{"choices":[{"delta":{"tool_calls":[{"index":1,"id":"c1","function":{"name":"update_node","arguments":"{}"}}]}}]}"#,
        );
        let calls = collector.pending_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1.name, "update_node");
    }
}
