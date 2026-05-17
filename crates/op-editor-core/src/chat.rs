//! AI chat sub-state for `EditorState`.
//!
//! Faithful copy of `openpencil-shell-core::document::chat::ChatState`
//! and its supporting types, adapted for the wasm-clean
//! `op-editor-core` crate. These are plain data types — message list,
//! input draft, panel anchor, model catalog — with no widget or
//! transport coupling. The actual `ChatProvider` plumbing stays in the
//! desktop host; this layer only carries state.

/// Re-export of the chat-request knobs from `op-ai` so callers of
/// `op-editor-core` get one import path. `ThinkingMode` / `EffortLevel`
/// drive the chat panel's per-turn selectors; `ChatAttachment` is one
/// pending image / file the user staged for the next turn.
pub use op_ai::chat_provider::{ChatAttachment, EffortLevel, ThinkingMode};

/// Maximum number of files that can be staged for one chat turn
/// (TS parity — the web chat input caps at four attachments).
pub const MAX_ATTACHMENTS: usize = 4;

/// Maximum size of a single staged attachment, in bytes (TS parity —
/// the web chat input rejects files over 5 MiB).
pub const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;

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
    /// Per-turn thinking-mode selector — the host copies this into the
    /// `ChatRequest` it builds for the provider.
    pub thinking_mode: ThinkingMode,
    /// Per-turn reasoning-effort selector.
    pub effort_level: EffortLevel,
    /// Files staged for the next turn (images the user pasted / picked).
    /// Drained by the host into `ChatRequest::attachments`, then cleared.
    pub pending_attachments: Vec<ChatAttachment>,
    /// Raised when the user clicks the attach button — the desktop
    /// host drains this each frame, opens a native file picker, and
    /// stages the chosen file via `add_attachment`. Mirrors the
    /// `pending_send` host-drain pattern.
    pub pending_attachment_pick: bool,
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
            thinking_mode: ThinkingMode::Adaptive,
            effort_level: EffortLevel::Low,
            pending_attachments: Vec::new(),
            pending_attachment_pick: false,
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
    /// Returns true when a send was queued — a turn may be queued with
    /// text, with staged attachments, or both (TS parity: an
    /// attachment-only message is sendable).
    pub fn begin_send(&mut self) -> bool {
        let trimmed = self.input.trim().to_string();
        if trimmed.is_empty() && self.pending_attachments.is_empty() {
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

    /// Advance the thinking-mode selector one step:
    /// Adaptive → Disabled → Enabled → Adaptive.
    pub fn cycle_thinking_mode(&mut self) {
        self.thinking_mode = match self.thinking_mode {
            ThinkingMode::Adaptive => ThinkingMode::Disabled,
            ThinkingMode::Disabled => ThinkingMode::Enabled,
            ThinkingMode::Enabled => ThinkingMode::Adaptive,
        };
    }

    /// Advance the effort selector one step:
    /// Low → Medium → High → Max → Low.
    pub fn cycle_effort_level(&mut self) {
        self.effort_level = match self.effort_level {
            EffortLevel::Low => EffortLevel::Medium,
            EffortLevel::Medium => EffortLevel::High,
            EffortLevel::High => EffortLevel::Max,
            EffortLevel::Max => EffortLevel::Low,
        };
    }

    /// Stage a file for the next turn. Rejected (returns `false`) when
    /// the per-turn attachment cap is already reached or the file
    /// exceeds [`MAX_ATTACHMENT_BYTES`].
    pub fn add_attachment(&mut self, attachment: ChatAttachment) -> bool {
        if self.pending_attachments.len() >= MAX_ATTACHMENTS {
            return false;
        }
        if attachment.data.len() > MAX_ATTACHMENT_BYTES {
            return false;
        }
        self.pending_attachments.push(attachment);
        true
    }

    /// Drop the staged attachment at `index`; out-of-range is a no-op.
    pub fn remove_attachment(&mut self, index: usize) {
        if index < self.pending_attachments.len() {
            self.pending_attachments.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_send_pushes_user_plus_empty_assistant_and_raises_flag() {
        let mut chat = ChatState {
            input: "  design a login page  ".into(),
            ..Default::default()
        };
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
        let mut chat = ChatState {
            input: "   ".into(),
            ..Default::default()
        };
        assert!(!chat.begin_send());
        assert!(chat.messages.is_empty());
        assert!(chat.pending_send.is_none());
    }

    #[test]
    fn send_echo_appends_user_and_assistant() {
        let mut chat = ChatState {
            input: "hi".into(),
            ..Default::default()
        };
        chat.send();
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[1].role, ChatRole::Assistant);
        assert!(chat.input.is_empty());
    }

    #[test]
    fn cycle_thinking_mode_wraps() {
        let mut chat = ChatState::default();
        assert_eq!(chat.thinking_mode, ThinkingMode::Adaptive);
        chat.cycle_thinking_mode();
        assert_eq!(chat.thinking_mode, ThinkingMode::Disabled);
        chat.cycle_thinking_mode();
        assert_eq!(chat.thinking_mode, ThinkingMode::Enabled);
        chat.cycle_thinking_mode();
        assert_eq!(chat.thinking_mode, ThinkingMode::Adaptive);
    }

    #[test]
    fn cycle_effort_level_wraps() {
        let mut chat = ChatState::default();
        assert_eq!(chat.effort_level, EffortLevel::Low);
        chat.cycle_effort_level();
        assert_eq!(chat.effort_level, EffortLevel::Medium);
        chat.cycle_effort_level();
        assert_eq!(chat.effort_level, EffortLevel::High);
        chat.cycle_effort_level();
        assert_eq!(chat.effort_level, EffortLevel::Max);
        chat.cycle_effort_level();
        assert_eq!(chat.effort_level, EffortLevel::Low);
    }

    #[test]
    fn add_and_remove_attachment() {
        let mut chat = ChatState::default();
        assert!(chat.pending_attachments.is_empty());
        chat.add_attachment(ChatAttachment {
            name: "a.png".into(),
            media_type: "image/png".into(),
            data: vec![1],
        });
        chat.add_attachment(ChatAttachment {
            name: "b.png".into(),
            media_type: "image/png".into(),
            data: vec![2],
        });
        assert_eq!(chat.pending_attachments.len(), 2);
        chat.remove_attachment(0);
        assert_eq!(chat.pending_attachments.len(), 1);
        assert_eq!(chat.pending_attachments[0].name, "b.png");
        // Out-of-range remove is a no-op.
        chat.remove_attachment(9);
        assert_eq!(chat.pending_attachments.len(), 1);
    }

    #[test]
    fn begin_send_leaves_pending_attachments_for_host_to_drain() {
        let mut chat = ChatState {
            input: "design with this".into(),
            ..Default::default()
        };
        chat.add_attachment(ChatAttachment {
            name: "ref.png".into(),
            media_type: "image/png".into(),
            data: vec![9],
        });
        assert!(chat.begin_send());
        // begin_send clears the input but NOT the attachments — the
        // host copies them into the ChatRequest, then clears.
        assert_eq!(chat.pending_attachments.len(), 1);
    }

    #[test]
    fn add_attachment_enforces_count_cap() {
        let mut chat = ChatState::default();
        for i in 0..MAX_ATTACHMENTS {
            assert!(chat.add_attachment(ChatAttachment {
                name: format!("{i}.png"),
                media_type: "image/png".into(),
                data: vec![1],
            }));
        }
        // The cap is reached — a further attachment is rejected.
        assert!(!chat.add_attachment(ChatAttachment {
            name: "extra.png".into(),
            media_type: "image/png".into(),
            data: vec![1],
        }));
        assert_eq!(chat.pending_attachments.len(), MAX_ATTACHMENTS);
    }

    #[test]
    fn add_attachment_rejects_oversized_file() {
        let mut chat = ChatState::default();
        let huge = ChatAttachment {
            name: "big.png".into(),
            media_type: "image/png".into(),
            data: vec![0u8; MAX_ATTACHMENT_BYTES + 1],
        };
        assert!(!chat.add_attachment(huge));
        assert!(chat.pending_attachments.is_empty());
    }

    #[test]
    fn begin_send_allows_attachment_only_message() {
        let mut chat = ChatState::default();
        chat.add_attachment(ChatAttachment {
            name: "ref.png".into(),
            media_type: "image/png".into(),
            data: vec![9],
        });
        // Empty text but a staged attachment — still sendable.
        assert!(chat.begin_send());
        assert_eq!(chat.pending_attachments.len(), 1);
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
