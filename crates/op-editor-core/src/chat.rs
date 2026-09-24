//! AI chat sub-state for `EditorState`.
//!
//! Faithful copy of `openpencil-shell-core::document::chat::ChatState`
//! and its supporting types, adapted for the wasm-clean
//! `op-editor-core` crate. These are plain data types — message list,
//! input draft, panel anchor, model catalog — with no widget or
//! transport coupling. The actual `ChatProvider` plumbing stays in the
//! desktop host; this layer only carries state.
//!
//! ### Module layout
//!
//! This file is the public spine: the panel-level types
//! ([`ChatAnchor`], [`ChatTranscriptSelection`], [`ChatState`] itself)
//! plus the shared re-exports. Everything else lives in sibling
//! submodules (per the 800-line-per-file ceiling) and is re-exported
//! here, so every existing `chat::*` import path still resolves:
//!
//! - [`models`] — [`AgentProvider`] + the [`ModelEntry`] catalog entry
//! - [`message`] — transcript records ([`ChatRole`], [`ChatToolCall`],
//!   [`ChatImage`], [`ChatMessage`])
//! - `session` — `impl ChatState` turn lifecycle + input editing
//! - `messages` — `impl ChatState` per-message card state, per-turn
//!   selectors and attachments

pub mod message;
mod messages;
pub mod models;
mod session;
#[cfg(test)]
mod tests;

pub use message::{ChatImage, ChatMessage, ChatRole, ChatToolCall};
pub use models::{AgentProvider, ModelEntry};

/// Re-export of the chat-request knobs from `op-ai` so callers of
/// `op-editor-core` get one import path. `ThinkingMode` / `EffortLevel`
/// drive the chat panel's per-turn selectors; `ChatAttachment` is one
/// pending image / file the user staged for the next turn.
pub use op_ai::chat_provider::{ChatAttachment, EffortLevel, ThinkingMode};

use crate::chat_activity::{ChatActivity, ChatActivityStatus, ChatCompletion, PendingSubtaskRetry};
use crate::chat_title::{suggest_chat_title, DEFAULT_CHAT_TITLE};
use jian_core::text_input::{prev_char_boundary, Selection, TextInputState};

/// Maximum number of files that can be staged for one chat turn
/// (TS parity — the web chat input caps at four attachments).
pub const MAX_ATTACHMENTS: usize = 4;

/// Maximum size of a single staged attachment, in bytes (TS parity —
/// the web chat input rejects files over 5 MiB).
pub const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;

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

pub const DEFAULT_CHAT_PANEL_WIDTH: f32 = 360.0;
pub const DEFAULT_CHAT_PANEL_HEIGHT: f32 = 520.0;

/// Byte-offset text selection inside one chat transcript message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatTranscriptSelection {
    pub message_index: usize,
    pub anchor: usize,
    pub focus: usize,
}

impl ChatTranscriptSelection {
    pub fn ordered(self) -> (usize, usize) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    pub fn is_collapsed(self) -> bool {
        self.anchor == self.focus
    }
}

/// Which pipeline the next drained send takes.
///
/// `Auto` keeps the usual routing (intent classification, the
/// design-agent-loop gate); `Orchestrator` pins the turn to the
/// orchestrator pipeline — the one `op-smoke` exercises and the only one
/// reasoning-budget models such as GLM 5.3 / GLM Flash reliably finish a
/// whole-design brief on, because the design-agent loop lets them spend
/// the entire output budget thinking before writing a single node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaunchRoute {
    /// Route by the usual intent classification and loop gate.
    #[default]
    Auto,
    /// Skip the design-agent loop; run the orchestrator pipeline.
    Orchestrator,
    /// Edit the SELECTED boards in place through the modify route.
    ///
    /// Studio Home's one-click start pins this after it loads a template
    /// as the instant draft: the refine brief quotes the example, whose
    /// wording ("做一份 5 页 PPT") reads to every keyword classifier as a
    /// NEW design — which would draw a second deck beside the draft instead
    /// of editing it. The route decides the intent, not the wording.
    Refine,
}

impl LaunchRoute {
    /// True when this route must skip the design-agent loop even when
    /// the loop gate would otherwise say yes.
    pub fn bypasses_design_agent_loop(self) -> bool {
        matches!(self, Self::Orchestrator)
    }

    /// True when the route itself establishes that the turn is a design
    /// request, so the keyword intent classifier must not get a vote.
    ///
    /// Studio Home pins this route only after the user has picked a task
    /// card and pressed 开始设计 — the deliverable is already decided, and
    /// re-deriving it from the prompt's wording is strictly worse
    /// information. It is also wrong in practice: the keyword list grew up
    /// around app/web briefs, so the 演示文稿 / 图文卡片 / 截图教程 /
    /// 信息图 / 活动海报 wrappers matched nothing and five of the seven
    /// task families silently answered as plain chat (measured
    /// 2026-09-13: a 5-page PPT brief produced one empty starter frame and
    /// a collapsed thinking block).
    pub fn implies_design_intent(self) -> bool {
        matches!(self, Self::Orchestrator)
    }

    /// True when the turn must edit the selected boards in place and
    /// never fall through to a new-design or chat route.
    pub fn forces_in_place_refine(self) -> bool {
        matches!(self, Self::Refine)
    }
}

/// Floating AI chat panel state — mirrors shell-core's `ChatState`
/// (messages, input draft, focused flag, panel anchor, model catalog).
#[derive(Debug, Clone)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    /// Short label shown in the floating chat panel header.
    pub title: String,
    /// Text input state for the chat textarea draft.
    pub input: TextInputState,
    pub focused: bool,
    /// Text selection inside a visible transcript user message.
    pub transcript_selection: Option<ChatTranscriptSelection>,
    /// Which canvas corner the floating chat panel snaps to.
    pub anchor: ChatAnchor,
    /// Non-maximized panel width. TS persists this as
    /// `panelWidth` in the AI UI store.
    pub panel_width: f32,
    /// Non-maximized panel height. TS persists this as
    /// `panelHeight` in the AI UI store.
    pub panel_height: f32,
    /// Absolute top-left while the user has resized from an edge
    /// that moves the panel origin. `None` falls back to the snapped
    /// corner anchor.
    pub panel_position: Option<(f32, f32)>,
    /// Legacy collapsed flag. The header-only middle state it used to
    /// name is retired — the panel now has exactly three forms
    /// (minimized bar / normal / maximized). Kept as a compatibility
    /// *read*: [`ChatState::is_minimized`] treats `collapsed == true` as
    /// minimized, so state built before the split (and older tests) still
    /// resolves to the compact bar. Nothing writes it any more.
    pub collapsed: bool,
    /// Minimized state — when true the panel paints as a compact input
    /// bar pinned to the anchored corner's bottom edge, and any click on
    /// it expands back to the normal panel.
    pub minimized: bool,
    /// Maximized state — when true the host lays the panel out across
    /// the canvas region with a small inset, mirroring the TS app's
    /// expanded panel.
    pub maximized: bool,
    /// Vertical scroll offset (px from the conversation top) of the
    /// transcript message list. Clamped to `[0, content_height - body]`
    /// by the host on wheel; ignored while [`transcript_pinned`] holds.
    ///
    /// [`transcript_pinned`]: ChatState::transcript_pinned
    pub transcript_scroll: jian_core::scroll::ScrollState,
    /// Whether the transcript auto-follows the latest content (pinned to
    /// the bottom). True until the user scrolls up; re-pins when they
    /// scroll back to the bottom, and is forced true on send / new chat
    /// so a fresh turn always reveals the latest reply.
    pub transcript_pinned: bool,
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
    /// Raised when the user clicks a transcript copy affordance; hosts
    /// drain this into the platform clipboard.
    pub pending_copy_text: Option<String>,
    /// Which pipeline the next drained `pending_send` must take. Home and
    /// the 成品视图's restyle set [`LaunchRoute::Orchestrator`] because
    /// their briefs are whole-design requests that reasoning-tuned models
    /// (GLM 5.x / Flash) can only finish on the orchestrator pipeline;
    /// the desktop launcher consumes it and resets it to `Auto`.
    pub launch_route: LaunchRoute,
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
    /// Number of parallel sub-agents used for the next design turn.
    pub agent_team_size: u32,
    /// Agents currently running / total agents in the active design-loop
    /// turn. `(0, 0)` when idle; `(1, 1)` for a single design-loop turn;
    /// extended to `(N, M)` when parallel sub-agents land in Phase 3.1.
    /// Set host-side on design-loop launch; cleared on turn end.
    pub agents_running: (usize, usize),
    /// Files staged for the next turn (images the user pasted / picked).
    /// Drained by the host into `ChatRequest::attachments`, then cleared.
    pub pending_attachments: Vec<ChatAttachment>,
    /// Raised when the user clicks the attach button — the desktop
    /// host drains this each frame, opens a native file picker, and
    /// stages the chosen file via `add_attachment`. Mirrors the
    /// `pending_send` host-drain pattern.
    pub pending_attachment_pick: bool,
    /// Raised by [`ChatState::begin_subtask_retry`] when the user clicks a
    /// failed row's "Retry" button: `(message index, subtask id)`. The
    /// desktop host drains this each frame, looks up the matching
    /// [`PendingSubtaskRetry`] + [`ChatMessage::design_request_json_for_retry`],
    /// and launches a single-subtask retry worker. Mirrors the
    /// `pending_send` / `codegen.pending_regenerate` host-drain pattern.
    pub pending_subtask_retry: Option<(usize, String)>,
    /// Wheel-driven scroll offset (px from the first wrapped line) of the
    /// draft *input* text area — distinct from [`transcript_scroll`], which
    /// moves the message list above it.
    ///
    /// It is only honoured while [`input_scroll_caret`] still matches the
    /// live caret: any caret motion (typing, arrows, a click, an IME
    /// commit) makes the stored offset stale, and the input snaps back to
    /// the line the caret sits on. That is what keeps "scroll to read the
    /// top of a long prompt" and "always see what I am typing" from
    /// fighting each other without a flag every mutation site has to set.
    ///
    /// [`transcript_scroll`]: ChatState::transcript_scroll
    /// [`input_scroll_caret`]: ChatState::input_scroll_caret
    pub input_scroll: f32,
    /// Caret byte offset captured when [`input_scroll`] was last written.
    ///
    /// [`input_scroll`]: ChatState::input_scroll
    pub input_scroll_caret: usize,
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
            title: DEFAULT_CHAT_TITLE.to_string(),
            input: TextInputState::default(),
            focused: false,
            transcript_selection: None,
            anchor: ChatAnchor::BottomLeft,
            panel_width: DEFAULT_CHAT_PANEL_WIDTH,
            panel_height: DEFAULT_CHAT_PANEL_HEIGHT,
            panel_position: None,
            collapsed: false,
            minimized: false,
            maximized: false,
            transcript_scroll: Default::default(),
            transcript_pinned: true,
            pending_send: None,
            pending_new_chat: false,
            pending_stop_chat: false,
            pending_copy_text: None,
            launch_route: LaunchRoute::Auto,
            discovered_models: Vec::new(),
            available_models: Vec::new(),
            selected_model: 0,
            thinking_mode: ThinkingMode::Adaptive,
            effort_level: EffortLevel::Low,
            agent_team_size: 1,
            agents_running: (0, 0),
            pending_attachments: Vec::new(),
            pending_attachment_pick: false,
            pending_subtask_retry: None,
            input_scroll: 0.0,
            input_scroll_caret: 0,
        }
    }
}
