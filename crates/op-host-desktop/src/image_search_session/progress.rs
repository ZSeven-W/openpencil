//! Diagnostic accounting only: a drained batch is not a successful design.

use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

/// Session identities are process-local; stderr also includes the process ID.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProgressSnapshot {
    pub(crate) session_id: u64,
    pub(crate) batch_id: u64,
    pub(crate) enqueued: usize,
    pub(crate) pending: usize,
    pub(crate) applied: usize,
    /// A missing-image fallback was actually written, not a generation retry.
    pub(crate) fallback: usize,
    pub(crate) discarded: usize,
    pub(crate) disconnected: usize,
    pub(crate) cancelled: usize,
}

impl ProgressSnapshot {
    fn terminal_count(self) -> usize {
        self.applied + self.fallback + self.discarded + self.disconnected + self.cancelled
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminalOutcome {
    Applied,
    Fallback,
    Discarded,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProgressEventKind {
    Enqueued,
    Drained,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProgressEvent {
    pub(super) kind: ProgressEventKind,
    pub(super) snapshot: ProgressSnapshot,
}

impl fmt::Display for ProgressEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let event = match self.kind {
            ProgressEventKind::Enqueued => "batch_enqueued",
            ProgressEventKind::Drained => "batch_drained",
            ProgressEventKind::Cancelled => "batch_cancelled",
        };
        let s = self.snapshot;
        write!(
            f,
            "{event} scope=logical_target_batch session_id={} batch_id={} enqueued={} pending={} applied={} fallback={} discarded={} disconnected={} cancelled={}",
            s.session_id, s.batch_id, s.enqueued, s.pending, s.applied,
            s.fallback, s.discarded, s.disconnected, s.cancelled,
        )
    }
}

/// One admission per logical slot, even when several slots share one fetch.
/// Generation-to-stock handoff keeps the original admission pending. Only the
/// UI-thread sink records a terminal outcome, after applying or discarding it.
pub(super) struct ImageBatchProgress {
    snapshot: ProgressSnapshot,
    pending: HashSet<String>,
}

impl Default for ImageBatchProgress {
    fn default() -> Self {
        Self {
            snapshot: ProgressSnapshot {
                session_id: NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed),
                ..Default::default()
            },
            pending: HashSet::new(),
        }
    }
}

impl ImageBatchProgress {
    pub(super) fn snapshot(&self) -> ProgressSnapshot {
        self.snapshot
    }

    pub(super) fn enqueue(&mut self, node_id: &str) {
        if self.pending.is_empty() {
            self.snapshot = ProgressSnapshot {
                session_id: self.snapshot.session_id,
                batch_id: self.snapshot.batch_id + 1,
                ..Default::default()
            };
        }
        if self.pending.insert(node_id.to_owned()) {
            self.snapshot.enqueued += 1;
            self.snapshot.pending += 1;
        }
    }

    pub(super) fn enqueued_event(&self) -> ProgressEvent {
        ProgressEvent {
            kind: ProgressEventKind::Enqueued,
            snapshot: self.snapshot(),
        }
    }

    pub(super) fn finish(
        &mut self,
        node_id: &str,
        outcome: TerminalOutcome,
    ) -> Option<ProgressEvent> {
        // Unknown or repeated completions never fabricate an ended batch.
        if !self.pending.remove(node_id) {
            return None;
        }
        self.snapshot.pending -= 1;
        match outcome {
            TerminalOutcome::Applied => self.snapshot.applied += 1,
            TerminalOutcome::Fallback => self.snapshot.fallback += 1,
            TerminalOutcome::Discarded => self.snapshot.discarded += 1,
            TerminalOutcome::Disconnected => self.snapshot.disconnected += 1,
        }
        (self.snapshot.pending == 0
            && self.snapshot.enqueued > 0
            && self.snapshot.terminal_count() == self.snapshot.enqueued)
            .then_some(ProgressEvent {
                kind: ProgressEventKind::Drained,
                snapshot: self.snapshot(),
            })
    }

    pub(super) fn reset(&mut self) -> Option<ProgressEvent> {
        let event = if self.snapshot.pending > 0 {
            self.snapshot.cancelled += self.snapshot.pending;
            self.snapshot.pending = 0;
            Some(ProgressEvent {
                kind: ProgressEventKind::Cancelled,
                snapshot: self.snapshot(),
            })
        } else {
            None
        };
        *self = Self::default();
        event
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
