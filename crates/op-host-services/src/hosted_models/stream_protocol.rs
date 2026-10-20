use op_ai::chat_provider::{ChatDelta, StopReason};
use op_editor_core::HostedProtocol;
use serde_json::Value;

use super::HostedRuntimeError;

/// Completion is the protocol sentinel after final usage, not finish_reason.
pub(super) struct ProtocolState {
    protocol: HostedProtocol,
    reason: Option<StopReason>,
    input_usage: bool,
    output_usage: bool,
}

impl ProtocolState {
    pub fn new(protocol: HostedProtocol) -> Self {
        Self {
            protocol,
            reason: None,
            input_usage: false,
            output_usage: false,
        }
    }

    pub fn consume(&mut self, data: &str) -> Result<Vec<ChatDelta>, HostedRuntimeError> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        if self.protocol == HostedProtocol::OpenAiCompatible && data == "[DONE]" {
            return self.complete();
        }
        let value: Value =
            serde_json::from_str(data).map_err(|_| HostedRuntimeError::GatewayRejected)?;
        if value.get("error").is_some() || value["type"] == "error" {
            return Err(HostedRuntimeError::GatewayRejected);
        }
        match self.protocol {
            HostedProtocol::OpenAiCompatible => self.openai(&value),
            HostedProtocol::AnthropicMessages => self.anthropic(&value, data),
            HostedProtocol::OpenAiResponses => Err(HostedRuntimeError::UnsupportedProtocol),
        }
    }

    fn complete(&self) -> Result<Vec<ChatDelta>, HostedRuntimeError> {
        if !self.input_usage || !self.output_usage {
            return Err(HostedRuntimeError::GatewayRejected);
        }
        let reason = self.reason.ok_or(HostedRuntimeError::GatewayRejected)?;
        Ok(vec![ChatDelta::Done {
            stop_reason: reason,
        }])
    }

    fn openai(&mut self, value: &Value) -> Result<Vec<ChatDelta>, HostedRuntimeError> {
        if let Some(usage) = value.get("usage").filter(|usage| !usage.is_null()) {
            self.input_usage |= usage
                .get("prompt_tokens")
                .is_some_and(|count| count.as_u64().is_some());
            self.output_usage |= usage
                .get("completion_tokens")
                .is_some_and(|count| count.as_u64().is_some());
        }
        let choices = value["choices"]
            .as_array()
            .ok_or(HostedRuntimeError::GatewayRejected)?;
        if choices.len() > 1 {
            return Err(HostedRuntimeError::GatewayRejected);
        }
        let Some(choice) = choices.first() else {
            return Ok(Vec::new());
        };
        let mut output = Vec::new();
        let delta = &choice["delta"];
        if let Some(content) = delta
            .get("content")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
        {
            output.push(ChatDelta::TextDelta(content.to_string()));
        }
        if let Some(thinking) = delta
            .get("reasoning_content")
            .or_else(|| delta.get("reasoning"))
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
        {
            output.push(ChatDelta::Thinking(thinking.to_string()));
        }
        if let Some(reason) = choice["finish_reason"].as_str() {
            self.reason = Some(op_ai::chat_sse::map_openai_stop_reason(reason));
        }
        Ok(output)
    }

    fn anthropic(
        &mut self,
        value: &Value,
        data: &str,
    ) -> Result<Vec<ChatDelta>, HostedRuntimeError> {
        match value["type"].as_str() {
            Some("message_start") => {
                self.input_usage = value
                    .pointer("/message/usage/input_tokens")
                    .is_some_and(|count| count.as_u64().is_some());
            }
            Some("message_delta") => {
                if let Some(reason) = value.pointer("/delta/stop_reason").and_then(Value::as_str) {
                    self.reason = Some(op_ai::chat_sse::map_anthropic_stop_reason(reason));
                }
                self.output_usage |= value
                    .pointer("/usage/output_tokens")
                    .is_some_and(|count| count.as_u64().is_some());
            }
            Some("message_stop") => return self.complete(),
            _ => {}
        }
        Ok(match op_ai::chat_sse::parse_anthropic_sse_data(data) {
            Some(delta @ (ChatDelta::TextDelta(_) | ChatDelta::Thinking(_))) => vec![delta],
            _ => Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_waits_for_usage_and_sentinel_preserving_same_event_content_and_reason() {
        let mut state = ProtocolState::new(HostedProtocol::OpenAiCompatible);
        let output = state
            .consume(r#"{"choices":[{"delta":{"content":"last"},"finish_reason":"length"}]}"#)
            .unwrap();
        assert!(matches!(&output[..], [ChatDelta::TextDelta(text)] if text == "last"));
        assert!(state
            .consume(r#"{"choices":[],"usage":{"prompt_tokens":7,"completion_tokens":9}}"#)
            .unwrap()
            .is_empty());
        let output = state.consume("[DONE]").unwrap();
        assert!(matches!(
            &output[..],
            [ChatDelta::Done {
                stop_reason: StopReason::MaxTokens
            }]
        ));
    }

    #[test]
    fn openai_finish_without_usage_cannot_succeed_and_late_error_fails() {
        let mut state = ProtocolState::new(HostedProtocol::OpenAiCompatible);
        assert!(state
            .consume(r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#)
            .unwrap()
            .is_empty());
        assert_eq!(
            state.consume("[DONE]").unwrap_err(),
            HostedRuntimeError::GatewayRejected
        );
        assert_eq!(
            state
                .consume(r#"{"error":{"message":"secret"}}"#)
                .unwrap_err(),
            HostedRuntimeError::GatewayRejected
        );
    }

    #[test]
    fn anthropic_waits_for_message_stop_after_final_usage() {
        let mut state = ProtocolState::new(HostedProtocol::AnthropicMessages);
        assert!(state.consume(r#"{"type":"message_start","message":{"usage":{"input_tokens":10,"output_tokens":0}}}"#).unwrap().is_empty());
        assert!(state.consume(r#"{"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":20}}"#).unwrap().is_empty());
        let output = state.consume(r#"{"type":"message_stop"}"#).unwrap();
        assert!(matches!(
            &output[..],
            [ChatDelta::Done {
                stop_reason: StopReason::MaxTokens
            }]
        ));
    }

    #[test]
    fn anthropic_missing_usage_or_error_after_finish_never_succeeds() {
        let mut state = ProtocolState::new(HostedProtocol::AnthropicMessages);
        state.consume(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":20}}"#).unwrap();
        assert_eq!(
            state.consume(r#"{"type":"message_stop"}"#).unwrap_err(),
            HostedRuntimeError::GatewayRejected
        );
        assert_eq!(
            state
                .consume(r#"{"type":"error","error":{"message":"secret"}}"#)
                .unwrap_err(),
            HostedRuntimeError::GatewayRejected
        );
    }
}
