//! Floating AI chat panel state.

/// Floating AI chat panel state — mirrors the TS app's
/// `useAIStore` (messages, input draft, focused flag).
#[derive(Debug, Clone)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub focused: bool,
    /// Which canvas corner the floating chat panel snaps to.
    /// User can drag the panel by its header; on release the
    /// host computes the nearest corner and updates this field.
    pub anchor: ChatAnchor,
    /// Collapsed state — when true the panel paints only the
    /// 36 px header strip (clicking the chevron toggles).
    pub collapsed: bool,
    /// Last user-action timestamp (focus / keystroke) in
    /// milliseconds — drives the caret blink phase via
    /// [`jian_core::anim::blink_visible`]. Reset on focus and on
    /// every key event so the caret reappears immediately when
    /// the user types instead of mid-blink.
    pub caret_anchor_ms: u64,
    /// Set by [`ChatState::begin_send`] to the just-sent user text.
    /// The desktop event loop drains this each frame: `Some(text)`
    /// means "start a real provider turn for this message". The
    /// host clears it once the turn is launched. `None` = idle.
    /// shell-core stays transport-free — it only raises the flag;
    /// `openpencil-desktop` owns the actual `ChatProvider` plumbing.
    pub pending_send: Option<String>,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            focused: false,
            anchor: ChatAnchor::BottomLeft,
            collapsed: false,
            caret_anchor_ms: 0,
            pending_send: None,
        }
    }
}

/// Which corner of the canvas region the AI chat panel sits in.
/// Step 5 P2: 4-corner edge snap on drag release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ChatAnchor {
    /// Pick the nearest corner to the given panel-center point
    /// inside the canvas rect. `(canvas_x0, canvas_y0)` is the
    /// canvas top-left, `(canvas_w, canvas_h)` its size.
    pub fn nearest(
        center: crate::Point2D,
        canvas_x0: f32,
        canvas_y0: f32,
        canvas_w: f32,
        canvas_h: f32,
    ) -> Self {
        let mid_x = canvas_x0 + canvas_w / 2.0;
        let mid_y = canvas_y0 + canvas_h / 2.0;
        let left = center.x < mid_x;
        let top = center.y < mid_y;
        match (top, left) {
            (true, true) => ChatAnchor::TopLeft,
            (true, false) => ChatAnchor::TopRight,
            (false, true) => ChatAnchor::BottomLeft,
            (false, false) => ChatAnchor::BottomRight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatState {
    /// Append the focused input as a new user message + a stub
    /// assistant echo, then clear the buffer. Offline fallback
    /// used by hosts with no real `ChatProvider` wired (the web
    /// shell today); the native desktop uses [`begin_send`].
    pub fn send(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }
        let user_msg = ChatMessage {
            role: ChatRole::User,
            content: trimmed.to_string(),
        };
        let echo = ChatMessage {
            role: ChatRole::Assistant,
            content: format!("(stub) Got it — \"{}\"", trimmed),
        };
        self.messages.push(user_msg);
        self.messages.push(echo);
        self.input.clear();
    }

    /// Real-send entry point. Pushes the user message + an empty
    /// assistant message (the host streams provider deltas into
    /// that last message), clears the input, and raises
    /// `pending_send` so the desktop event loop launches a real
    /// `ChatProvider` turn. Returns true when a send was queued
    /// (non-empty input), false on an empty buffer.
    pub fn begin_send(&mut self) -> bool {
        let trimmed = self.input.trim().to_string();
        if trimmed.is_empty() {
            return false;
        }
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content: trimmed.clone(),
        });
        // Empty assistant bubble — provider deltas append here.
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: String::new(),
        });
        self.input.clear();
        self.pending_send = Some(trimmed);
        true
    }

    /// Push the focused input as a user message and stream the reply
    /// through `provider`. Accumulates `TextDelta` + `Thinking`
    /// fragments into a single assistant message; bails on `Error`
    /// with the error text as the assistant body. Returns the count
    /// of fragments consumed so callers can report streaming
    /// progress / errors.
    ///
    /// Synchronous wrapper — the provider's iterator yields deltas
    /// until `Done`; this drains them. A future async-capable widget
    /// can replace the loop with one delta per render frame.
    pub fn send_via_provider(
        &mut self,
        provider: &dyn crate::chat_provider::ChatProvider,
        system_prompt: impl Into<String>,
        max_output_tokens: u32,
    ) -> usize {
        use crate::chat_provider::{ChatDelta, ChatRequest};
        let trimmed = self.input.trim().to_string();
        if trimmed.is_empty() {
            return 0;
        }
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content: trimmed.clone(),
        });
        self.input.clear();
        let req = ChatRequest {
            system_prompt: system_prompt.into(),
            user_message: trimmed,
            max_output_tokens,
        };
        let mut buffer = String::new();
        let mut count = 0usize;
        for delta in provider.send(req) {
            count += 1;
            match delta {
                ChatDelta::TextDelta(s) | ChatDelta::Thinking(s) => buffer.push_str(&s),
                ChatDelta::Error(msg) => {
                    buffer = format!("error: {msg}");
                    break;
                }
                ChatDelta::Done { .. } => break,
                ChatDelta::ToolUse { .. } => {} // tool dispatch is the agent runtime's job
            }
        }
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: buffer,
        });
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_provider::{ChatDelta, EchoProvider, StopReason};

    #[test]
    fn send_via_provider_streams_text_into_assistant_message() {
        let mut chat = ChatState::default();
        chat.input = "hi".into();
        let p = EchoProvider {
            script: vec![
                ChatDelta::TextDelta("Hello".into()),
                ChatDelta::TextDelta(", world!".into()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        };
        let n = chat.send_via_provider(&p, "be helpful", 1024);
        assert_eq!(n, 3, "consumed 3 deltas");
        // User message + assistant message pushed.
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[1].role, ChatRole::Assistant);
        assert_eq!(chat.messages[1].content, "Hello, world!");
        assert!(chat.input.is_empty());
    }

    #[test]
    fn send_via_provider_surfaces_provider_error_in_assistant_body() {
        let mut chat = ChatState::default();
        chat.input = "x".into();
        let p = EchoProvider {
            script: vec![ChatDelta::Error("rate limited".into())],
        };
        chat.send_via_provider(&p, "", 0);
        assert!(chat.messages[1].content.contains("rate limited"));
    }

    #[test]
    fn send_via_provider_empty_input_no_ops() {
        let mut chat = ChatState::default();
        let p = EchoProvider { script: Vec::new() };
        let n = chat.send_via_provider(&p, "", 0);
        assert_eq!(n, 0);
        assert!(chat.messages.is_empty());
    }

    #[test]
    fn begin_send_pushes_user_plus_empty_assistant_and_raises_flag() {
        let mut chat = ChatState::default();
        chat.input = "  design a login page  ".into();
        assert!(chat.begin_send());
        // User message + empty assistant bubble for streaming.
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[0].role, ChatRole::User);
        assert_eq!(chat.messages[0].content, "design a login page");
        assert_eq!(chat.messages[1].role, ChatRole::Assistant);
        assert!(chat.messages[1].content.is_empty());
        assert!(chat.input.is_empty());
        assert_eq!(chat.pending_send.as_deref(), Some("design a login page"));
    }

    #[test]
    fn begin_send_empty_input_no_ops() {
        let mut chat = ChatState::default();
        chat.input = "   ".into();
        assert!(!chat.begin_send());
        assert!(chat.messages.is_empty());
        assert!(chat.pending_send.is_none());
    }
}
