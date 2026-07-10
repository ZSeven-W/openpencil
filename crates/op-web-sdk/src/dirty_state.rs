//! Pure dirty/schedule state machine backing the CanvasKit rAF pump.
//!
//! Extracted out of `render.rs` (which is gated behind the `canvaskit`
//! feature and pulls in wasm-only types: `wasm_bindgen::closure::Closure`,
//! `web_sys`, `op-host-web`) so the actual arm/consume transition logic is
//! unit-testable in the crate's default (non-wasm, non-canvaskit) test
//! build — see `dirty_state_tests.rs`. `render.rs` wraps this in the wasm
//! plumbing (`RenderShared`, the rAF `Closure`) but never re-implements the
//! transition rules itself.
//!
//! Design notes (each maps to a review finding on the first version):
//!
//! - **Generation counter, not a bool.** `content_gen` is bumped by every
//!   `mark_dirty()` via `wrapping_add(1)`; `painted_gen` records the
//!   generation of the last *successful* paint (always a copy of some past
//!   `content_gen` value — see `finish_paint_success`). "Dirty" means
//!   `content_gen != painted_gen`, so a mark that arrives while a paint is
//!   in flight is never lost: the paint only consumes the generation it
//!   snapshotted via `begin_paint()`, and `finish_paint_success()` reports
//!   whether newer content still needs a frame.
//!   **Wrap-safety:** every needs-paint comparison uses `!=`, never `>` —
//!   `content_gen` wraps (u64::MAX -> 0) after enough marks, and once it
//!   does, a `>` comparison can alias (a wrapped, chronologically-newer
//!   generation can compare numerically *less than* `painted_gen`, reading
//!   as clean when it is not). `!=` has no such failure mode because
//!   `painted_gen` only ever receives an exact copy of a `content_gen`
//!   value that was observed to differ from it.
//! - **`scheduled` commits only after the rAF request succeeds.** A caller
//!   first calls `mark_dirty()` (which does NOT set `scheduled`), attempts
//!   the `requestAnimationFrame` call, and calls `commit_arm()` only on
//!   success. If the request fails the machine stays dirty-but-unscheduled,
//!   so the next `mark_dirty()` reports "arm needed" again instead of
//!   freezing forever on `scheduled == true` with no frame pending.
//! - **Bounded failure breaker, shared by paint AND arm failures.** If
//!   making progress on the *same* content generation fails
//!   `MAX_CONSECUTIVE_FAILURES` times in a row — whether the failures are
//!   failed paints, failed `requestAnimationFrame` calls, or a mix of the
//!   two — the machine trips `stopped` and the pump deliberately goes quiet
//!   (log once, no rAF spin). Any subsequent `mark_dirty()` — new content —
//!   resets the breaker and retries. The bound is therefore per content
//!   generation, not global: a persistent fault costs at most N attempts
//!   per user-visible update instead of an infinite retry loop or a silent
//!   freeze with no external mark to rescue it.
//! - **Breaker ownership is scoped to whichever generation is current when
//!   it trips, not to whichever `begin_paint`/`finish_paint_success` pair
//!   happens to complete afterwards.** A paint snapshots its generation in
//!   `begin_paint()` and may still be running when a re-entrant mark bumps
//!   `content_gen` again; if THAT newer generation's own arm attempts then
//!   trip the breaker, the in-flight (older) paint can still succeed
//!   afterwards. `finish_paint_success()` therefore never writes
//!   `consecutive_failures` or `stopped` — an old generation's success is
//!   not evidence that a newer generation's failures are resolved — and it
//!   never asks the caller to re-arm while `stopped` is set. `stopped` /
//!   `consecutive_failures` have exactly two writers in the whole type:
//!   `mark_dirty()` (always clears both — a genuinely new mark is the only
//!   thing allowed to reopen the breaker) and `record_failure()` (which
//!   only ever sets `stopped`, never clears it). No other method touches
//!   either field.
//!
//! The invariant the whole machine upholds: *dirty (content newer than the
//! last successful paint) implies an armed frame, OR a failed-arm state that
//! the very next mark retries, OR a deliberate bounded stop after repeated
//! paint failures.*
//!
//! Its only production consumer is the `canvaskit`-feature-gated
//! `render.rs`, so the default (no-`canvaskit`) `lib` build sees it as dead
//! code; `dirty_state_tests.rs` is what actually exercises it in a default
//! `cargo test` run. Mirrors the existing `#![allow(dead_code)]` on
//! `viewer_host.rs` for the same reason (sole consumer is feature-gated).
#![allow(dead_code)]

use std::cell::Cell;

/// How many consecutive failures — paint failures, `requestAnimationFrame`
/// rejections, or a mix — of the same content generation the pump
/// tolerates before it stops re-arming (and logs once). Kept small: these
/// failures are not expected to be transient, so retrying is mostly about
/// surviving a one-off glitch, not polling until recovery.
pub(crate) const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// What the caller must do after a paint or arm attempt failed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FailureAction {
    /// Retry (failure budget left) — request another animation frame.
    Rearm,
    /// Failure budget exhausted — stop retrying and log once. The next
    /// `mark_dirty()` (new content) resets the breaker and retries.
    Stop,
}

/// Tracks what content needs painting (`content_gen` vs `painted_gen`),
/// whether a `requestAnimationFrame` callback is currently pending
/// (`scheduled`), and the consecutive-failure breaker (paint failures and
/// arm/`requestAnimationFrame` failures share it), for one render pump.
///
/// This is the ONLY type allowed to make the clean-to-dirty transition —
/// every dirty-setting call site in `render.rs` (`mark_dirty`, `push_scene`,
/// `push_viewport_to_render`) routes through `mark_dirty()` here so setting
/// the flag and arming the pump stay paired up. A missed re-arm means
/// content newer than the last paint with no frame scheduled — a permanently
/// frozen viewer.
#[derive(Debug)]
pub(crate) struct DirtyState {
    /// Monotonic content generation; bumped by every `mark_dirty()`.
    content_gen: Cell<u64>,
    /// Generation of the last successfully painted frame.
    painted_gen: Cell<u64>,
    /// Whether a rAF callback is pending. Set only by `commit_arm()`, i.e.
    /// only after the `requestAnimationFrame` call actually succeeded.
    scheduled: Cell<bool>,
    /// Consecutive failed paint or arm attempts since the last
    /// `mark_dirty()`. Conceptually owned by whichever content generation
    /// is currently unpainted, not by whichever `begin_paint`/
    /// `finish_paint_success` pair happens to run — a paint success does
    /// NOT reset this (see `finish_paint_success`'s doc comment); only a
    /// fresh `mark_dirty()` (new content) does.
    consecutive_failures: Cell<u32>,
    /// Breaker: set when `consecutive_failures` hits the budget; cleared
    /// ONLY by the next `mark_dirty()`, never by a paint success. The
    /// tripped state belongs to whichever generation caused the trip
    /// (typically the newest one seen so far); an older, already
    /// in-flight generation's paint finishing afterwards must not reopen
    /// it or request another arm attempt.
    stopped: Cell<bool>,
}

impl DirtyState {
    /// Construct a state that is dirty (one unpainted content generation)
    /// with no frame scheduled yet. Matches `attach_canvas`: the initial
    /// frame is armed through the same request-then-`commit_arm` path as
    /// every later one, so a failed initial request leaves the machine
    /// re-armable instead of frozen.
    pub(crate) fn new_dirty() -> Self {
        Self {
            content_gen: Cell::new(1),
            painted_gen: Cell::new(0),
            scheduled: Cell::new(false),
            consecutive_failures: Cell::new(0),
            stopped: Cell::new(false),
        }
    }

    /// Test-only constructor that seeds both generation counters directly,
    /// so tests can start right at (or past) the `u64` wrap boundary
    /// instead of driving `mark_dirty()` `u64::MAX` times to get there.
    #[cfg(test)]
    pub(crate) fn new_dirty_with_gens(content_gen: u64, painted_gen: u64) -> Self {
        Self {
            content_gen: Cell::new(content_gen),
            painted_gen: Cell::new(painted_gen),
            scheduled: Cell::new(false),
            consecutive_failures: Cell::new(0),
            stopped: Cell::new(false),
        }
    }

    /// Record that the content changed. Returns `true` when no frame is
    /// currently pending and the caller MUST attempt to arm one (request a
    /// frame, then call `commit_arm()` only if the request succeeded);
    /// returns `false` when a frame is already in flight and will pick the
    /// new generation up on its own.
    ///
    /// Also resets the failure breaker: the bound is "the same content
    /// failed N times", and this is new content.
    ///
    /// Uses `wrapping_add` deliberately: after `u64::MAX` marks this wraps
    /// back to a small value. That's safe precisely because every
    /// needs-paint comparison in this type uses `!=`, not `>` — see the
    /// module doc's wrap-safety note.
    pub(crate) fn mark_dirty(&self) -> bool {
        self.content_gen.set(self.content_gen.get().wrapping_add(1));
        self.consecutive_failures.set(0);
        self.stopped.set(false);
        !self.scheduled.get()
    }

    /// Commit that a `requestAnimationFrame` call succeeded. Never call
    /// this when the request failed — leaving `scheduled == false` on
    /// failure is exactly what lets a later `mark_dirty()` retry.
    pub(crate) fn commit_arm(&self) {
        self.scheduled.set(true);
    }

    /// Called once at the top of every pump tick. Consumes the pending
    /// schedule (so a mark arriving during the paint below knows it must
    /// arm a fresh frame) and returns the content generation to paint, or
    /// `None` when there is nothing to do (clean, or breaker tripped).
    ///
    /// Note this does NOT clear the dirty condition — only a subsequent
    /// `finish_paint_success()` with the returned generation does, so a
    /// paint that dies midway leaves the content still marked unpainted.
    pub(crate) fn begin_paint(&self) -> Option<u64> {
        self.scheduled.set(false);
        if self.stopped.get() {
            return None;
        }
        let gen = self.content_gen.get();
        // `!=`, not `>`: `content_gen` wraps, so a newer generation can be
        // numerically smaller than `painted_gen` after the wrap. Equality
        // is the only wrap-safe test for "still the same as last painted".
        (gen != self.painted_gen.get()).then_some(gen)
    }

    /// Record that painting generation `gen` (returned by `begin_paint`)
    /// succeeded. Consumes exactly that generation — never a newer one —
    /// and returns `true` when the caller must re-arm because newer content
    /// arrived during the paint and no frame is pending for it (its own
    /// arm attempt may have failed, or it could not run one).
    ///
    /// Deliberately does NOT touch the failure breaker
    /// (`consecutive_failures` / `stopped`) and deliberately suppresses its
    /// own `true` (re-arm) result while `stopped` is set. Reasoning: `gen`
    /// was snapshotted by `begin_paint()` before this paint ran, and a
    /// re-entrant `mark_dirty()` may have bumped `content_gen` again (and
    /// reset the breaker) while this paint was still in flight. If THAT
    /// newer generation's own arm attempts then tripped the breaker, this
    /// (older) paint's later success is not evidence the newer
    /// generation's failures are resolved — resetting the breaker here, or
    /// asking the caller to re-arm, would hand the newer, already-tripped
    /// generation a fresh failure budget and risk a second breaker-trip
    /// log. Only a genuinely new `mark_dirty()` may reopen the breaker.
    pub(crate) fn finish_paint_success(&self, gen: u64) -> bool {
        // Unconditional set, not `if gen > painted_gen`: `gen` is exactly
        // the `content_gen` snapshot `begin_paint` already proved differs
        // from `painted_gen`, so it is by construction the generation that
        // was just painted — a `>` guard here would misfire across the
        // wrap (a chronologically-newer `gen` can compare numerically
        // smaller) and wrongly refuse to record the paint as done.
        self.painted_gen.set(gen);
        self.content_gen.get() != self.painted_gen.get()
            && !self.scheduled.get()
            && !self.stopped.get()
    }

    /// Record that a paint OR arm (`requestAnimationFrame`) attempt
    /// failed. The failed generation stays unpainted (dirty), and the
    /// caller is told whether to retry (`Rearm`) or give up until new
    /// content arrives (`Stop`, returned exactly once per breaker trip so
    /// the caller can log once). Paint failures and arm failures share this
    /// one counter/bound — see the module doc.
    pub(crate) fn record_failure(&self) -> FailureAction {
        let n = self.consecutive_failures.get() + 1;
        self.consecutive_failures.set(n);
        if n >= MAX_CONSECUTIVE_FAILURES {
            self.stopped.set(true);
            FailureAction::Stop
        } else {
            FailureAction::Rearm
        }
    }

    /// Detach-time reset: clear the pending-schedule flag (the caller has
    /// cancelled the underlying rAF callback, so no frame will fire). Keeps
    /// the machine consistent even if something still holds a reference to
    /// it after `detach`.
    pub(crate) fn cancel_schedule(&self) {
        self.scheduled.set(false);
    }

    /// Whether content newer than the last successful paint exists.
    /// `!=`, not `>` — see the module doc's wrap-safety note.
    pub(crate) fn is_dirty(&self) -> bool {
        self.content_gen.get() != self.painted_gen.get()
    }

    #[cfg(test)]
    pub(crate) fn is_scheduled(&self) -> bool {
        self.scheduled.get()
    }

    #[cfg(test)]
    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped.get()
    }
}
