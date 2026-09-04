//! Provider-neutral activity state for the chat transcript.
//!
//! Built-in design turns receive tool events while CLI-backed design turns
//! receive orchestrator progress events. Both are execution details; the
//! transcript consumes this small shared model instead of reparsing display
//! strings emitted by either backend.

/// Lifecycle state of one user-visible design activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatActivityStatus {
    Pending,
    Running,
    Done,
    Error,
}

/// One stable activity row in an assistant message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChatActivity {
    /// Backend-stable id used to update the row in place.
    pub id: String,
    /// Short user-facing label, such as "Recently Played".
    pub title: String,
    /// Optional concise outcome. Internal prompt, skill, and token details do
    /// not belong here.
    pub detail: Option<String>,
    pub status: ChatActivityStatus,
    /// Byte offset in the visible narration where this activity entered the
    /// timeline. `None` keeps compatibility with legacy grouped checklists.
    pub content_offset: Option<u32>,
}

/// Structured terminal metadata for provider history and diagnostics. The
/// visible transcript uses ordinary localized narration instead of a special
/// terminal card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChatCompletion {
    pub succeeded: u32,
    pub failed: u32,
    pub nodes: u32,
}

/// A failed orchestrator subtask's spec, persisted so the progress panel's
/// per-row "Retry" button can re-run EXACTLY that subtask later instead of a
/// re-derived approximation (see the failed-subtask-remediation plan).
///
/// Stored as an opaque `serde_json`-encoded string rather than the concrete
/// `op_orchestrator::plan::Subtask` type: `op-editor-core` cannot depend on
/// `op-orchestrator` (the dependency runs the other way — orchestrator
/// depends on editor-core — so a concrete-type field here would be
/// circular). The host layer (`op-host-desktop`, which already depends on
/// `op-orchestrator`) serializes this at capture time and deserializes it
/// back at retry-click time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSubtaskRetry {
    /// Matches the originating `ChatActivity.id` — the same key a retry
    /// click resolves through `activities[source_index].id`.
    pub subtask_id: String,
    /// `serde_json`-encoded `op_orchestrator::plan::Subtask`.
    pub subtask_json: String,
    /// The document sibling this section should follow when it is retried.
    /// `None` means the first child of its parent.
    pub insert_after_sibling_id: Option<String>,
}
