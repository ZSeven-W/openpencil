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
    /// `Some(id)` when this model belongs to a built-in API-key
    /// provider rather than an external CLI.
    pub builtin_provider_id: Option<String>,
    /// Display label for the built-in provider group (for example
    /// `MiniMax`). Kept separate from `display_name`, which is the
    /// model row label.
    pub builtin_provider_display_name: Option<String>,
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
            builtin_provider_id: None,
            builtin_provider_display_name: None,
        }
    }

    pub fn builtin(
        provider: AgentProvider,
        builtin_provider_id: impl Into<String>,
        value: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            value: value.into(),
            display_name: display_name.into(),
            builtin_provider_id: Some(builtin_provider_id.into()),
            builtin_provider_display_name: None,
        }
    }

    pub fn builtin_with_display_name(
        provider: AgentProvider,
        builtin_provider_id: impl Into<String>,
        builtin_provider_display_name: impl Into<String>,
        value: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            value: value.into(),
            display_name: display_name.into(),
            builtin_provider_id: Some(builtin_provider_id.into()),
            builtin_provider_display_name: Some(builtin_provider_display_name.into()),
        }
    }

    pub fn acp(acp_agent_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        let id = acp_agent_id.into();
        Self {
            provider: AgentProvider::CodexCli,
            value: format!("acp:{id}"),
            display_name: display_name.into(),
            builtin_provider_id: None,
            builtin_provider_display_name: None,
        }
    }

    pub fn acp_agent_id(&self) -> Option<&str> {
        self.value.strip_prefix("acp:")
    }
}

/// Author of a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

/// One tool invocation surfaced inside an assistant message. The chat
/// panel renders these in a collapsible "tool calls" panel — the
/// transcript view, not the agent runtime (dispatch stays the
/// runtime's job). `args` is the raw JSON the model passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatToolCall {
    pub name: String,
    pub args: String,
}

/// One image carried inside a chat message — a copy of an image
/// [`ChatAttachment`] the user sent, kept so the transcript can show
/// it after the input strip is cleared. `id` is a process-unique
/// handle the render backend keys its decode cache on (decoding the
/// raw bytes every frame would be far too slow).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatImage {
    /// Process-unique id — stable across frames for the backend cache.
    pub id: u64,
    pub name: String,
    pub media_type: String,
    /// Raw encoded image bytes (PNG / JPEG / …), not base64.
    pub data: Vec<u8>,
}

/// One message in the chat transcript. `content` is the visible
/// answer text; `thinking` and `tool_calls` are the assistant's
/// reasoning + tool activity, each shown in a collapsible block;
/// `images` are pictures the user attached. `streaming` is true on
/// the trailing assistant bubble while its turn is still in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// Accumulated reasoning text (`ChatDelta::Thinking`). Empty for
    /// user messages and for turns that emitted no thinking.
    pub thinking: String,
    /// Tool invocations the assistant made this turn.
    pub tool_calls: Vec<ChatToolCall>,
    /// Images the user attached to this message.
    pub images: Vec<ChatImage>,
    /// Collapsed state of the thinking block (default collapsed).
    pub thinking_collapsed: bool,
    /// Collapsed state of the tool-calls panel (default collapsed).
    pub tools_collapsed: bool,
    /// True while this (assistant) message's turn streams in.
    pub streaming: bool,
}

impl ChatMessage {
    /// A plain user message — no thinking / tools, not streaming.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            images: Vec::new(),
            thinking_collapsed: true,
            tools_collapsed: true,
            streaming: false,
        }
    }

    /// An assistant message. Pass `streaming = true` via
    /// [`ChatMessage::assistant_streaming`] for an in-flight turn.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            images: Vec::new(),
            thinking_collapsed: true,
            tools_collapsed: true,
            streaming: false,
        }
    }

    /// An empty assistant bubble for a turn that is about to stream —
    /// provider deltas append into it and `streaming` clears on `Done`.
    pub fn assistant_streaming() -> Self {
        Self {
            streaming: true,
            ..Self::assistant("")
        }
    }
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
    /// Maximized state — when true the host lays the panel out across
    /// the canvas region with a small inset, mirroring the TS app's
    /// expanded panel.
    pub maximized: bool,
    /// Collapsed state for the fixed "Pencil it out" design checklist
    /// pinned above the input, mirroring the TS checklist header.
    pub checklist_collapsed: bool,
    /// Last user-action timestamp (focus / keystroke) in ms — drives
    /// the caret blink phase. Reset on focus and on every key event.
    pub caret_anchor_ms: u64,
    /// Set by `begin_send` to the just-sent user text; the desktop
    /// event loop drains this each frame. `None` = idle.
    pub pending_send: Option<String>,
    /// Raised when the user clicks the panel's New Chat affordance.
    /// The desktop event loop drains this to drop any in-flight chat
    /// or design worker that could otherwise keep appending into the
    /// fresh empty transcript.
    pub pending_new_chat: bool,
    /// Raised when the user clicks the streaming turn's Stop
    /// affordance. Unlike New Chat, the transcript stays visible; the
    /// desktop event loop only drops the in-flight worker.
    pub pending_stop_chat: bool,
    /// Full model catalog discovered from every *installed* CLI,
    /// before the connected-providers filter. The desktop host fills
    /// this from `model_discovery`; [`rebuild_available_models`] then
    /// derives [`available_models`] from it.
    ///
    /// [`rebuild_available_models`]: ChatState::rebuild_available_models
    /// [`available_models`]: ChatState::available_models
    pub discovered_models: Vec<ModelEntry>,
    /// Models the user can pick in the chat panel's model dropdown —
    /// `discovered_models` filtered to the providers the user has
    /// *connected* in Settings → Agents. Empty until the host runs
    /// discovery and the user connects at least one agent.
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

/// Process-global allocator for [`ChatImage::id`]. A *global* counter
/// (not a per-`ChatState` field) is required: the backend image
/// decode cache is keyed on this id, so a fresh `ChatState` — e.g.
/// after "New Chat" — must never restart the sequence and collide
/// with a still-cached decode.
static NEXT_IMAGE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Hand out the next process-unique [`ChatImage::id`].
fn alloc_image_id() -> u64 {
    NEXT_IMAGE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            focused: false,
            anchor: ChatAnchor::BottomLeft,
            collapsed: false,
            maximized: false,
            checklist_collapsed: false,
            caret_anchor_ms: 0,
            pending_send: None,
            pending_new_chat: false,
            pending_stop_chat: false,
            discovered_models: Vec::new(),
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

    /// Recompute [`available_models`] = [`discovered_models`] filtered
    /// to the providers the user has connected (`connected` is indexed
    /// by [`AgentProvider::ALL`]). The previously-selected model is
    /// preserved by identity when it survives the filter, otherwise
    /// `selected_model` falls back to `0`.
    ///
    /// Called by the host after model discovery completes and after
    /// every connect / disconnect toggle, so the picker only ever
    /// lists models the user can actually reach.
    ///
    /// [`available_models`]: ChatState::available_models
    /// [`discovered_models`]: ChatState::discovered_models
    pub fn rebuild_available_models(&mut self, connected: &[bool; 5]) {
        let prev = self.available_models.get(self.selected_model).cloned();
        self.available_models = self
            .discovered_models
            .iter()
            .filter(|m| {
                AgentProvider::ALL
                    .iter()
                    .position(|p| *p == m.provider)
                    .is_some_and(|i| connected[i])
            })
            .cloned()
            .collect();
        self.selected_model = prev
            .and_then(|p| {
                self.available_models.iter().position(|m| {
                    m.provider == p.provider
                        && m.value == p.value
                        && m.builtin_provider_id == p.builtin_provider_id
                })
            })
            .unwrap_or(0);
    }

    /// Append the focused input as a new user message + a stub
    /// assistant echo, then clear the buffer. Offline fallback used by
    /// hosts with no real `ChatProvider` wired.
    pub fn send(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }
        let echo = format!("(stub) Got it — \"{}\"", trimmed);
        self.messages.push(ChatMessage::user(trimmed));
        self.messages.push(ChatMessage::assistant(echo));
        self.input.clear();
    }

    /// Real-send entry point. Pushes the user message + an empty
    /// streaming assistant message, clears the input, and raises
    /// `pending_send` so the desktop event loop launches a real
    /// provider turn. Returns true when a send was queued — a turn
    /// may be queued with text, with staged attachments, or both
    /// (TS parity: an attachment-only message is sendable).
    pub fn begin_send(&mut self) -> bool {
        let trimmed = self.input.trim().to_string();
        if trimmed.is_empty() && self.pending_attachments.is_empty() {
            return false;
        }
        // A turn still in flight is interrupted by this new send — its
        // assistant bubble will never reach `Done`. Clear every
        // `streaming` flag so a stale bubble doesn't animate forever;
        // only the bubble pushed below should stream.
        for msg in &mut self.messages {
            msg.streaming = false;
        }
        // Copy the staged *image* attachments into the user message so
        // the transcript keeps showing them after the input strip is
        // cleared. Each gets a fresh decode-cache id. Non-image
        // attachments are dropped here (the backend can't draw them);
        // the host still drains `pending_attachments` for the request.
        let mut user_msg = ChatMessage::user(trimmed.clone());
        for att in &self.pending_attachments {
            if att.is_image() {
                let id = alloc_image_id();
                user_msg.images.push(ChatImage {
                    id,
                    name: att.name.clone(),
                    media_type: att.media_type.clone(),
                    data: att.data.clone(),
                });
            }
        }
        self.messages.push(user_msg);
        // Empty streaming assistant bubble — provider deltas append here.
        self.messages.push(ChatMessage::assistant_streaming());
        self.input.clear();
        self.pending_send = Some(trimmed);
        true
    }

    /// Stop the currently streaming turn while keeping the visible
    /// transcript. Returns true when either a queued send or streaming
    /// bubble was actually cancelled.
    pub fn stop_streaming(&mut self) -> bool {
        let had_pending = self.pending_send.take().is_some();
        let mut had_streaming = false;
        for msg in &mut self.messages {
            if msg.streaming {
                had_streaming = true;
                msg.streaming = false;
            }
        }
        if had_pending || had_streaming {
            self.pending_stop_chat = true;
            true
        } else {
            false
        }
    }

    /// Start a fresh chat transcript and ask the host to abort any
    /// in-flight worker tied to the previous conversation.
    pub fn new_chat(&mut self) {
        self.messages.clear();
        self.input.clear();
        self.pending_send = None;
        self.pending_stop_chat = false;
        self.pending_attachments.clear();
        self.pending_attachment_pick = false;
        self.pending_new_chat = true;
    }

    /// Flip the collapsed state of message `idx`'s thinking block.
    /// Out-of-range index is a no-op.
    pub fn toggle_message_thinking(&mut self, idx: usize) {
        if let Some(msg) = self.messages.get_mut(idx) {
            msg.thinking_collapsed = !msg.thinking_collapsed;
        }
    }

    /// Flip the collapsed state of message `idx`'s tool-calls panel.
    /// Out-of-range index is a no-op.
    pub fn toggle_message_tool_calls(&mut self, idx: usize) {
        if let Some(msg) = self.messages.get_mut(idx) {
            msg.tools_collapsed = !msg.tools_collapsed;
        }
    }

    /// Flip the fixed design-checklist panel state.
    pub fn toggle_checklist_collapsed(&mut self) {
        self.checklist_collapsed = !self.checklist_collapsed;
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
    fn begin_send_clears_a_prior_interrupted_streaming_bubble() {
        let mut chat = ChatState {
            input: "first question".into(),
            ..Default::default()
        };
        assert!(chat.begin_send());
        // messages[1] is the in-flight assistant bubble.
        assert!(chat.messages[1].streaming);
        // The user sends again before the first turn finished — the
        // first turn is now interrupted and will never reach `Done`.
        chat.input = "second question".into();
        assert!(chat.begin_send());
        assert!(
            !chat.messages[1].streaming,
            "the interrupted turn's bubble must stop streaming"
        );
        assert!(
            chat.messages[3].streaming,
            "only the newest assistant bubble streams"
        );
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
    fn chat_message_user_constructor_has_empty_structured_fields() {
        let m = ChatMessage::user("hello");
        assert_eq!(m.role, ChatRole::User);
        assert_eq!(m.content, "hello");
        assert!(m.thinking.is_empty());
        assert!(m.tool_calls.is_empty());
        assert!(m.images.is_empty());
        assert!(!m.streaming);
    }

    #[test]
    fn begin_send_marks_only_the_assistant_message_streaming() {
        let mut chat = ChatState {
            input: "design something".into(),
            ..Default::default()
        };
        assert!(chat.begin_send());
        assert!(!chat.messages[0].streaming, "user message is not streaming");
        assert!(
            chat.messages[1].streaming,
            "the empty assistant bubble is streaming until the turn ends"
        );
    }

    #[test]
    fn begin_send_copies_image_attachments_into_user_message_with_unique_ids() {
        let mut chat = ChatState {
            input: "look at these".into(),
            ..Default::default()
        };
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
        assert!(chat.begin_send());
        let user = &chat.messages[0];
        assert_eq!(user.images.len(), 2, "both images shown in the bubble");
        assert_eq!(user.images[0].name, "a.png");
        assert_eq!(user.images[0].data, vec![1]);
        assert_ne!(
            user.images[0].id, user.images[1].id,
            "each image gets a distinct decode-cache id"
        );
        // The host still drains pending_attachments into the request.
        assert_eq!(chat.pending_attachments.len(), 2);
    }

    #[test]
    fn image_ids_never_collide_across_fresh_chat_states() {
        // A "New Chat" makes a fresh ChatState — its image ids must
        // not restart at 0 and collide with a still-cached decode.
        let mut a = ChatState {
            input: "x".into(),
            ..Default::default()
        };
        a.add_attachment(ChatAttachment {
            name: "a.png".into(),
            media_type: "image/png".into(),
            data: vec![1],
        });
        a.begin_send();
        let first_id = a.messages[0].images[0].id;

        let mut b = ChatState {
            input: "y".into(),
            ..Default::default()
        };
        b.add_attachment(ChatAttachment {
            name: "b.png".into(),
            media_type: "image/png".into(),
            data: vec![2],
        });
        b.begin_send();
        assert_ne!(
            first_id, b.messages[0].images[0].id,
            "a fresh ChatState must not reuse image ids"
        );
    }

    #[test]
    fn begin_send_skips_non_image_attachments_for_the_bubble() {
        let mut chat = ChatState {
            input: "and a doc".into(),
            ..Default::default()
        };
        chat.add_attachment(ChatAttachment {
            name: "notes.txt".into(),
            media_type: "text/plain".into(),
            data: vec![7],
        });
        assert!(chat.begin_send());
        // A non-image attachment can't be drawn — keep it out of the
        // bubble's image strip (the host still sends it).
        assert!(chat.messages[0].images.is_empty());
    }

    #[test]
    fn toggle_message_thinking_flips_collapsed_flag() {
        let mut chat = ChatState::default();
        chat.messages.push(ChatMessage::assistant("hi"));
        let before = chat.messages[0].thinking_collapsed;
        chat.toggle_message_thinking(0);
        assert_eq!(chat.messages[0].thinking_collapsed, !before);
        // Out-of-range index is a no-op (must not panic).
        chat.toggle_message_thinking(99);
    }

    #[test]
    fn toggle_message_tool_calls_flips_collapsed_flag() {
        let mut chat = ChatState::default();
        chat.messages.push(ChatMessage::assistant("hi"));
        let before = chat.messages[0].tools_collapsed;
        chat.toggle_message_tool_calls(0);
        assert_eq!(chat.messages[0].tools_collapsed, !before);
        chat.toggle_message_tool_calls(99);
    }

    #[test]
    fn toggle_checklist_collapsed_flips_panel_checklist_flag() {
        let mut chat = ChatState::default();
        assert!(!chat.checklist_collapsed);

        chat.toggle_checklist_collapsed();
        assert!(chat.checklist_collapsed);

        chat.toggle_checklist_collapsed();
        assert!(!chat.checklist_collapsed);
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

    #[test]
    fn rebuild_available_models_keeps_only_connected_providers() {
        let mut chat = ChatState {
            discovered_models: vec![
                ModelEntry::new(AgentProvider::ClaudeCode, "opus", "Opus"),
                ModelEntry::new(AgentProvider::ClaudeCode, "sonnet", "Sonnet"),
                ModelEntry::new(AgentProvider::CodexCli, "gpt-5.5", "GPT-5.5"),
                ModelEntry::new(AgentProvider::OpenCode, "oc/x", "oc/x"),
            ],
            ..Default::default()
        };
        // Only Claude Code (index 0 of AgentProvider::ALL) connected.
        let mut connected = [false; 5];
        connected[0] = true;
        chat.rebuild_available_models(&connected);
        assert_eq!(chat.available_models.len(), 2);
        assert!(chat
            .available_models
            .iter()
            .all(|m| m.provider == AgentProvider::ClaudeCode));
    }

    #[test]
    fn rebuild_available_models_preserves_selection_by_identity() {
        let mut chat = ChatState {
            discovered_models: vec![
                ModelEntry::new(AgentProvider::ClaudeCode, "opus", "Opus"),
                ModelEntry::new(AgentProvider::CodexCli, "gpt-5.5", "GPT-5.5"),
            ],
            ..Default::default()
        };
        let mut connected = [false; 5];
        connected[0] = true; // Claude
        connected[1] = true; // Codex
        chat.rebuild_available_models(&connected);
        // Select Codex's GPT-5.5 (index 1).
        chat.selected_model = 1;
        // Disconnecting Claude drops index 0 — the selection must
        // follow GPT-5.5 to its new index rather than dangle.
        connected[0] = false;
        chat.rebuild_available_models(&connected);
        assert_eq!(chat.available_models.len(), 1);
        assert_eq!(chat.selected_model, 0);
        assert_eq!(chat.available_models[0].value, "gpt-5.5");
        // Disconnecting the last provider empties the list and the
        // selection clamps back to 0.
        connected[1] = false;
        chat.rebuild_available_models(&connected);
        assert!(chat.available_models.is_empty());
        assert_eq!(chat.selected_model, 0);
    }
}
