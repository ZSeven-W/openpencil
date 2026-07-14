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
