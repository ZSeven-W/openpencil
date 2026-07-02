//! Concurrency primitives shared by the agent-team fan-out (`spawn_concurrent`).
//!
//! The orchestrator's former multi-screen concurrent path (screen grouping +
//! N-root scaffold + semaphore/`join_all` worker fan-in) was collapsed into the
//! sequential path: it produced byte-identical per-subtask output (same
//! `run_subtask` unit, same deterministic post-passes) and only bought
//! wall-clock parallelism, which the default + benchmarked path never used
//! (`concurrency = 1`). What remains here are the two reusable primitives the
//! `spawn_agents` MCP fan-out (`spawn_concurrent.rs`) still builds on:
//!
//! - [`clamp_concurrency`] — the defensive `[1, 6]` permit cap.
//! - [`BufferDocSink`] — a per-worker buffering [`DocSink`] that collects applied
//!   [`EditorCommand`]s without touching the real document.

use crate::types::DocSink;
use op_editor_core::{EditorCommand, EditorState};

/// Clamps a raw concurrency value to the valid range `[1, 6]`.
///
/// This mirrors the store-side clamp in TS (store clamps to [1,6] before
/// writing to `request.concurrency`). The Rust crate clamps defensively on
/// the way in so callers need not worry about out-of-range values.
pub(crate) fn clamp_concurrency(v: u32) -> u32 {
    v.clamp(1, 6)
}

// ── BufferDocSink ─────────────────────────────────────────────────────────────

/// A per-worker buffering [`DocSink`] that collects every applied
/// [`EditorCommand`] into an in-memory `Vec` without touching the real document.
///
/// **Design choice (spec §5 implementer note):** the buffer is still a plain
/// command collector; it does not re-execute worker commands locally. The
/// pre-spawn snapshot is read-only context for generation post-processing
/// such as design-variable colour binding.
///
/// After all workers finish the caller replays `commands` into the real
/// `DocSink` in a deterministic order (serialized replay).
pub(crate) struct BufferDocSink {
    /// Snapshot of `EditorState` taken before the concurrent phase.
    /// Returned by `state()` unchanged for read-only generation context.
    snapshot: EditorState,
    /// All `EditorCommand`s collected via `apply()` calls.
    pub commands: Vec<EditorCommand>,
    /// Tracks undo-batch nesting depth (for parity with `DocSink` contract).
    pub batch_depth: i32,
}

impl BufferDocSink {
    /// Create a new buffer sink from a pre-concurrent state snapshot.
    pub(crate) fn new(snapshot: EditorState) -> Self {
        Self {
            snapshot,
            commands: Vec::new(),
            batch_depth: 0,
        }
    }
}

impl DocSink for BufferDocSink {
    fn state(&self) -> &EditorState {
        &self.snapshot
    }

    /// Buffer the command for later replay; always returns `true`.
    ///
    /// We return `true` unconditionally — the real document is not touched
    /// here; the per-worker result is validated after replay.
    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.commands.push(cmd);
        true
    }

    fn begin_undo_batch(&mut self) {
        self.batch_depth += 1;
    }

    fn end_undo_batch(&mut self) {
        self.batch_depth -= 1;
    }
}

#[cfg(test)]
#[path = "concurrent_tests.rs"]
mod tests;
