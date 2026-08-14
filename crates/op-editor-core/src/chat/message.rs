//! Chat transcript types: roles, tool calls, images and the
//! `ChatMessage` record itself.

use super::*;

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
    /// Byte offset into the owning message's `content` at the moment this
    /// call landed — the transcript uses it to interleave narration prose
    /// with per-call verb chips in chronological order (Pencil's reading
    /// flow). `None` on plain chat turns keeps the aggregated panel.
    pub content_offset: Option<u32>,
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
/// answer text; `thinking`, `tool_calls`, and `activities` carry the
/// assistant's private reasoning and user-visible work state; `images` are
/// pictures the user attached. `streaming` is true on the trailing assistant
/// bubble while its turn is still in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// Display name of the agent that produced this assistant message.
    pub agent_name: Option<String>,
    /// Optional `#RRGGBB` identity colour assigned by an orchestrated agent.
    pub agent_color: Option<String>,
    /// Accumulated reasoning text (`ChatDelta::Thinking`). Empty for
    /// user messages and for turns that emitted no thinking.
    pub thinking: String,
    /// Tool invocations the assistant made this turn.
    pub tool_calls: Vec<ChatToolCall>,
    /// Provider-neutral design activity. CLI orchestrator progress and
    /// built-in tool events can both target this presentation model.
    pub activities: Vec<ChatActivity>,
    /// Structured terminal metadata for provider history and diagnostics. The
    /// transcript must not recover these values by parsing visible prose.
    pub completion: Option<ChatCompletion>,
    /// Images the user attached to this message.
    pub images: Vec<ChatImage>,
    /// Collapsed state of the thinking block (default collapsed).
    pub thinking_collapsed: bool,
    /// Collapsed state of the tool-calls panel (default collapsed).
    pub tools_collapsed: bool,
    /// Per-tool-card expanded-state overrides. Missing / `None`
    /// entries fall back to the UI's auth-level default.
    pub tool_call_expanded_overrides: Vec<Option<bool>>,
    /// Per-design-JSON-block expanded-state overrides. Missing /
    /// `None` entries fall back to the transcript default: streaming
    /// design blocks open so the incoming .op preview is visible,
    /// completed blocks stay collapsed.
    pub design_block_expanded_overrides: Vec<Option<bool>>,
    /// Per-action-step (subtask card) expanded-state overrides. Missing
    /// / `None` entries fall back to the transcript default (expanded
    /// while the step is active or failed so diagnostics stay visible).
    pub action_step_expanded_overrides: Vec<Option<bool>>,
    /// True while this (assistant) message's turn streams in.
    pub streaming: bool,
    /// Screen-group id for a classic-orchestrator worker bubble. `None` marks
    /// the primary turn message. Worker bubbles are presentation-only; their
    /// screen/activity context is folded into the primary provider-history
    /// entry so one design turn never becomes consecutive assistant messages.
    pub design_worker_group: Option<u32>,
    /// Human-readable screen label paired with [`Self::design_worker_group`].
    pub design_worker_screen: Option<String>,
    /// The turn's original `op_orchestrator::types::DesignRequest`,
    /// `serde_json`-encoded, captured once at launch — the manual retry
    /// entry point needs it to re-run a failed subtask with the same
    /// prompt/model/append-context the turn originally used. `None` for
    /// non-design turns (plain chat) and user messages.
    pub design_request_json_for_retry: Option<String>,
    /// One entry per zero-node subtask failure this message's turn
    /// produced, keyed by the matching `ChatActivity.id` — the progress
    /// panel's per-row "Retry" button resolves through this list. Empty
    /// until a design-turn summary reports a failure.
    pub failed_subtasks: Vec<PendingSubtaskRetry>,
}

impl ChatMessage {
    /// A plain user message — no thinking / tools, not streaming.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            agent_name: None,
            agent_color: None,
            thinking: String::new(),
            tool_calls: Vec::new(),
            activities: Vec::new(),
            completion: None,
            images: Vec::new(),
            thinking_collapsed: true,
            tools_collapsed: true,
            tool_call_expanded_overrides: Vec::new(),
            design_block_expanded_overrides: Vec::new(),
            action_step_expanded_overrides: Vec::new(),
            streaming: false,
            design_worker_group: None,
            design_worker_screen: None,
            design_request_json_for_retry: None,
            failed_subtasks: Vec::new(),
        }
    }

    /// An assistant message. Pass `streaming = true` via
    /// [`ChatMessage::assistant_streaming`] for an in-flight turn.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            agent_name: None,
            agent_color: None,
            thinking: String::new(),
            tool_calls: Vec::new(),
            activities: Vec::new(),
            completion: None,
            images: Vec::new(),
            thinking_collapsed: true,
            tools_collapsed: true,
            tool_call_expanded_overrides: Vec::new(),
            design_block_expanded_overrides: Vec::new(),
            action_step_expanded_overrides: Vec::new(),
            streaming: false,
            design_worker_group: None,
            design_worker_screen: None,
            design_request_json_for_retry: None,
            failed_subtasks: Vec::new(),
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
