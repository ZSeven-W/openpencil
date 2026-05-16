//! AI chat sub-state for `EditorState`.
//!
//! Faithful copy of `openpencil-shell-core::document::chat::ChatState`
//! and its supporting types, adapted for the wasm-clean
//! `op-editor-core` crate. These are plain data types — message list,
//! input draft, panel anchor, model catalog — with no widget or
//! transport coupling. The actual `ChatProvider` plumbing stays in the
//! desktop host; this layer only carries state.

/// Which CLI agent backs a model / chat turn. Ported verbatim from
/// shell-core's `agent_settings_state::AgentProvider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProvider {
    ClaudeCode,
    CodexCli,
    OpenCode,
    GithubCopilot,
    GeminiCli,
}

impl AgentProvider {
    pub const ALL: [AgentProvider; 5] = [
        AgentProvider::ClaudeCode,
        AgentProvider::CodexCli,
        AgentProvider::OpenCode,
        AgentProvider::GithubCopilot,
        AgentProvider::GeminiCli,
    ];

    pub fn name(self) -> &'static str {
        match self {
            AgentProvider::ClaudeCode => "Claude Code",
            AgentProvider::CodexCli => "Codex CLI",
            AgentProvider::OpenCode => "OpenCode",
            AgentProvider::GithubCopilot => "GitHub Copilot",
            AgentProvider::GeminiCli => "Gemini CLI",
        }
    }

    /// i18n key for the provider's subtitle.
    pub fn subtitle_key(self) -> &'static str {
        match self {
            AgentProvider::ClaudeCode => "settings.provider.claudeCode",
            AgentProvider::CodexCli => "settings.provider.codexCli",
            AgentProvider::OpenCode => "settings.provider.openCode",
            AgentProvider::GithubCopilot => "settings.provider.githubCopilot",
            AgentProvider::GeminiCli => "settings.provider.geminiCli",
        }
    }
}

/// One selectable model in the chat model picker. Ported from
/// shell-core's `chat_models::ModelEntry`.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelEntry {
    /// Which CLI agent backs this model — also picks the chat
    /// transport.
    pub provider: AgentProvider,
    /// Wire id passed to the CLI (e.g. `gpt-5.5`, `claude-sonnet-4-6`).
    pub value: String,
    /// Human label shown in the picker (e.g. `GPT-5.5`).
    pub display_name: String,
}

impl ModelEntry {
    pub fn new(
        provider: AgentProvider,
        value: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            value: value.into(),
            display_name: display_name.into(),
        }
    }
}

/// Author of a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

/// One message in the chat transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Which corner of the canvas region the floating AI chat panel sits
/// in. Ported verbatim from shell-core's `ChatAnchor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ChatAnchor {
    /// Pick the nearest corner to the given panel-center point inside
    /// the canvas rect. `(canvas_x0, canvas_y0)` is the canvas
    /// top-left, `(canvas_w, canvas_h)` its size.
    pub fn nearest(
        center: crate::render_backend::Point2D,
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

/// Floating AI chat panel state — mirrors shell-core's `ChatState`
/// (messages, input draft, focused flag, panel anchor, model catalog).
#[derive(Debug, Clone)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub focused: bool,
    /// Which canvas corner the floating chat panel snaps to.
    pub anchor: ChatAnchor,
    /// Collapsed state — when true the panel paints only its header.
    pub collapsed: bool,
    /// Last user-action timestamp (focus / keystroke) in ms — drives
    /// the caret blink phase. Reset on focus and on every key event.
    pub caret_anchor_ms: u64,
    /// Set by `begin_send` to the just-sent user text; the desktop
    /// event loop drains this each frame. `None` = idle.
    pub pending_send: Option<String>,
    /// Models the user can pick in the chat panel's model dropdown.
    /// Empty until the desktop host discovers them from connected CLIs.
    pub available_models: Vec<ModelEntry>,
    /// Index into `available_models` of the active model.
    pub selected_model: usize,
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
            available_models: Vec::new(),
            selected_model: 0,
        }
    }
}

impl ChatState {
    /// The currently selected model, or `None` when the catalog is
    /// empty.
    pub fn selected_model_entry(&self) -> Option<&ModelEntry> {
        self.available_models.get(self.selected_model)
    }

    /// Append the focused input as a new user message + a stub
    /// assistant echo, then clear the buffer. Offline fallback used by
    /// hosts with no real `ChatProvider` wired.
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
    /// assistant message, clears the input, and raises `pending_send`
    /// so the desktop event loop launches a real provider turn.
    /// Returns true when a send was queued (non-empty input).
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_send_pushes_user_plus_empty_assistant_and_raises_flag() {
        let mut chat = ChatState::default();
        chat.input = "  design a login page  ".into();
        assert!(chat.begin_send());
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

    #[test]
    fn send_echo_appends_user_and_assistant() {
        let mut chat = ChatState::default();
        chat.input = "hi".into();
        chat.send();
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[1].role, ChatRole::Assistant);
        assert!(chat.input.is_empty());
    }

    #[test]
    fn nearest_anchor_picks_corner() {
        let p = crate::render_backend::Point2D::new(10.0, 10.0);
        assert_eq!(
            ChatAnchor::nearest(p, 0.0, 0.0, 100.0, 100.0),
            ChatAnchor::TopLeft
        );
        let p2 = crate::render_backend::Point2D::new(90.0, 90.0);
        assert_eq!(
            ChatAnchor::nearest(p2, 0.0, 0.0, 100.0, 100.0),
            ChatAnchor::BottomRight
        );
    }
}
