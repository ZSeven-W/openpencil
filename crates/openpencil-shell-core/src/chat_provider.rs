//! AI chat provider abstraction. Real LLM endpoints (Anthropic,
//! OpenAI-compatible, OpenCode, Gemini, etc.) implement this trait;
//! the chat widget calls `send` to push a user message and receives
//! `ChatDelta`s via the iterator until `Done`. Mirrors the streaming
//! shape `apps/web/src/services/ai/ai-service.ts` uses.
//!
//! v1 scope: trait + an `EchoProvider` test double. Real HTTP
//! transport + per-provider serialisation arrive with the agent
//! runtime port (`packages/agent-native` → Rust subprocess /
//! NAPI / direct).

/// Streaming delta from a provider — text fragments, tool calls,
/// status events. Mirrors `streaming/events.zig::Event` in
/// agent-native.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatDelta {
    /// Text fragment appended to the assistant's reply.
    TextDelta(String),
    /// Thinking trace (Claude's "thinking" content blocks).
    Thinking(String),
    /// Tool invocation — name + JSON-stringified args.
    ToolUse { name: String, args: String },
    /// Final assistant message, stop-reason known.
    Done { stop_reason: StopReason },
    /// Provider-side error — abort + surface to user.
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Model finished naturally.
    EndTurn,
    /// User-side `AbortController` interrupted the stream.
    Aborted,
    /// `maxOutputTokens` budget reached.
    MaxTokens,
    /// Tool call sequence — caller should run the tool + resume.
    ToolUse,
}

/// One pending request. Providers hold this in their stream state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest {
    pub system_prompt: String,
    pub user_message: String,
    pub max_output_tokens: u32,
}

/// Provider abstraction. `send` initiates the stream + returns an
/// iterator of deltas. Errors surface as `ChatDelta::Error` rather
/// than `Result` so partial output is preserved.
pub trait ChatProvider: Send + Sync {
    fn provider_label(&self) -> &str;
    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send>;
}

/// Test double that replays a fixed script. Useful for chat-widget
/// unit tests that need a deterministic stream without an actual
/// LLM round-trip.
pub struct EchoProvider {
    /// Sequence of deltas to yield in order. Last entry should be
    /// `Done { stop_reason: EndTurn }` for typical fixtures.
    pub script: Vec<ChatDelta>,
}

impl ChatProvider for EchoProvider {
    fn provider_label(&self) -> &str {
        "echo"
    }
    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(self.script.clone().into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_provider_replays_script() {
        let p = EchoProvider {
            script: vec![
                ChatDelta::TextDelta("Hello".into()),
                ChatDelta::TextDelta(", world!".into()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        };
        let req = ChatRequest {
            system_prompt: "be helpful".into(),
            user_message: "hi".into(),
            max_output_tokens: 1024,
        };
        let mut iter = p.send(req);
        match iter.next() {
            Some(ChatDelta::TextDelta(s)) => assert_eq!(s, "Hello"),
            _ => panic!(),
        }
        match iter.next() {
            Some(ChatDelta::TextDelta(s)) => assert_eq!(s, ", world!"),
            _ => panic!(),
        }
        match iter.next() {
            Some(ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            }) => {}
            _ => panic!(),
        }
        assert!(iter.next().is_none());
    }

    #[test]
    fn echo_provider_label_is_echo() {
        let p = EchoProvider { script: Vec::new() };
        assert_eq!(p.provider_label(), "echo");
    }

    #[test]
    fn error_delta_carries_message() {
        let p = EchoProvider {
            script: vec![ChatDelta::Error("rate limited".into())],
        };
        let req = ChatRequest {
            system_prompt: String::new(),
            user_message: String::new(),
            max_output_tokens: 0,
        };
        let mut iter = p.send(req);
        match iter.next() {
            Some(ChatDelta::Error(m)) => assert_eq!(m, "rate limited"),
            _ => panic!(),
        }
    }
}
