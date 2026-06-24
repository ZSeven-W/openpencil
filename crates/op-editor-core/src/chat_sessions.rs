//! `ChatSessions` — a collection of chat tabs wrapping `ChatState`.
//!
//! Each editor window can hold multiple parallel AI conversations; this
//! struct owns the `Vec<ChatState>` and tracks the active index. The key
//! design principle: `Deref<Target = ChatState>` and `DerefMut` forward to
//! the **active** tab, so the ~100+ call sites that do `state.chat.*` keep
//! compiling unchanged — they transparently operate on the active tab.
//!
//! ## Invariants
//! - `tabs` is **never empty** — at least one `ChatState` always exists.
//! - `active` is always a valid index into `tabs`.
//! - `Deref` and `DerefMut` therefore never panic.

use std::ops::{Deref, DerefMut};

use crate::chat::ChatState;

/// A multi-tab chat container that exposes the active tab via `Deref`.
///
/// The initial state contains a single default `ChatState` at index 0.
/// All mutating tab operations maintain the two structural invariants:
/// `!tabs.is_empty()` and `active < tabs.len()`.
#[derive(Debug, Clone)]
pub struct ChatSessions {
    tabs: Vec<ChatState>,
    active: usize,
}

impl Default for ChatSessions {
    fn default() -> Self {
        Self {
            tabs: vec![ChatState::default()],
            active: 0,
        }
    }
}

// --------------------------------------------------------------------------
// Deref / DerefMut — the whole point of this wrapper.
// All `state.chat.field` / `state.chat.method()` call sites get here for free.
// --------------------------------------------------------------------------

impl Deref for ChatSessions {
    type Target = ChatState;

    fn deref(&self) -> &ChatState {
        // SAFETY: invariants guarantee tabs is non-empty and active is valid.
        &self.tabs[self.active]
    }
}

impl DerefMut for ChatSessions {
    fn deref_mut(&mut self) -> &mut ChatState {
        // SAFETY: invariants guarantee tabs is non-empty and active is valid.
        &mut self.tabs[self.active]
    }
}

// --------------------------------------------------------------------------
// Tab management API
// --------------------------------------------------------------------------

impl ChatSessions {
    /// Index of the currently active tab.
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Total number of open tabs. Always ≥ 1.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Read-only slice of all tabs — for the tab-row UI to read titles,
    /// message counts, etc. without touching the active-tab Deref.
    pub fn tabs(&self) -> &[ChatState] {
        &self.tabs
    }

    /// Push a fresh `ChatState`, set it as active, and return its index.
    ///
    /// The new tab starts blank (default), independent from all others.
    pub fn new_tab(&mut self) -> usize {
        self.tabs.push(ChatState::default());
        self.active = self.tabs.len() - 1;
        self.active
    }

    /// Switch to tab `i`. Out-of-range indices are silently ignored (no
    /// panic) — the active tab is unchanged in that case.
    pub fn switch_to(&mut self, i: usize) {
        if i < self.tabs.len() {
            self.active = i;
        }
    }

    /// Remove tab at index `i` and fix up `active` so it remains valid.
    ///
    /// Closing the **last** surviving tab replaces it with one fresh
    /// `ChatState::default()` so the collection is never empty. Out-of-range
    /// `i` is a no-op.
    pub fn close_tab(&mut self, i: usize) {
        if i >= self.tabs.len() {
            return; // out of range — no-op
        }
        if self.tabs.len() == 1 {
            // Closing the only tab: reset in place rather than removing.
            self.tabs[0] = ChatState::default();
            self.active = 0;
            return;
        }
        self.tabs.remove(i);
        // Fix up `active` so it remains a valid index:
        //   - If we closed a tab before active, shift active left by one.
        //   - If we closed the active tab itself, clamp to the new last tab.
        //   - If we closed a tab after active, active index is unchanged.
        if i <= self.active && self.active > 0 {
            self.active -= 1;
        }
        // Guard: always stay within bounds (should not be needed after the
        // above, but is a cheap invariant check).
        self.active = self.active.min(self.tabs.len() - 1);
    }

    /// Immutable reference to the active tab (same as `Deref`, but explicit).
    pub fn active(&self) -> &ChatState {
        &self.tabs[self.active]
    }

    /// Mutable reference to the active tab (same as `DerefMut`, but explicit).
    pub fn active_mut(&mut self) -> &mut ChatState {
        &mut self.tabs[self.active]
    }

    /// Mutable reference to tab `idx` by index — `None` when out of range.
    ///
    /// This is the write accessor a host uses to target a tab OTHER than the
    /// active one: a streaming AI run is bound to the tab it started on, so
    /// switching tabs mid-run must keep writing the transcript to the bound
    /// tab rather than the (now different) active tab.
    pub fn tab_mut(&mut self, idx: usize) -> Option<&mut ChatState> {
        self.tabs.get_mut(idx)
    }

    /// Resolve the tab a running turn should write to.
    ///
    /// `running_tab` is the index the run was bound to when it launched
    /// (`active_index()` at launch). It returns that bound tab when the index
    /// is still in range; otherwise it falls back to the active tab. So a run
    /// keeps filling its own tab while the user browses other tabs, and a
    /// stale binding (e.g. the bound tab was closed) degrades to the active
    /// tab instead of panicking.
    pub fn run_tab_mut(&mut self, running_tab: Option<usize>) -> &mut ChatState {
        match running_tab {
            Some(idx) if idx < self.tabs.len() => &mut self.tabs[idx],
            _ => &mut self.tabs[self.active],
        }
    }
}

/// Adjust a run's bound tab index for the removal of tab `closed` from a
/// `ChatSessions`, mirroring `close_tab`'s index fix-up.
///
/// The host keeps `running_tab` OUTSIDE `ChatSessions` (it tracks an in-flight
/// worker, not editor state), so it cannot ride `close_tab`'s own `active`
/// adjustment. The rules match `Vec::remove`:
/// - `closed == running` → the bound tab itself was removed; the caller is
///   expected to abort the run first, so this returns `None` to clear the
///   binding.
/// - `closed < running` → everything after the hole shifts left by one, so
///   the run now lives at `running - 1`.
/// - `closed > running` → indices before the hole are untouched.
pub fn adjust_running_tab_after_close(running: usize, closed: usize) -> Option<usize> {
    use std::cmp::Ordering;
    match closed.cmp(&running) {
        Ordering::Equal => None,
        Ordering::Less => Some(running.saturating_sub(1)),
        Ordering::Greater => Some(running),
    }
}

// --------------------------------------------------------------------------
// Unit tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_title::DEFAULT_CHAT_TITLE;

    #[test]
    fn default_has_one_tab_at_index_zero() {
        let s = ChatSessions::default();
        assert_eq!(s.tab_count(), 1);
        assert_eq!(s.active_index(), 0);
    }

    #[test]
    fn new_tab_adds_tab_and_activates_it() {
        let mut s = ChatSessions::default();
        let idx = s.new_tab();
        assert_eq!(s.tab_count(), 2);
        assert_eq!(idx, 1);
        assert_eq!(s.active_index(), 1);
    }

    #[test]
    fn switch_to_changes_active() {
        let mut s = ChatSessions::default();
        s.new_tab();
        assert_eq!(s.active_index(), 1);
        s.switch_to(0);
        assert_eq!(s.active_index(), 0);
    }

    #[test]
    fn switch_to_out_of_range_is_noop() {
        let mut s = ChatSessions::default();
        s.switch_to(99);
        assert_eq!(s.active_index(), 0); // unchanged
    }

    #[test]
    fn deref_reads_active_tab_title() {
        let mut s = ChatSessions::default();
        // Active is tab 0 with default title.
        assert_eq!(s.title, DEFAULT_CHAT_TITLE);

        // Add a second tab and give it a custom title via DerefMut.
        s.new_tab();
        s.title = "custom".to_string();

        // Switch back to tab 0 — Deref should show the default title again.
        s.switch_to(0);
        assert_eq!(s.title, DEFAULT_CHAT_TITLE);
    }

    #[test]
    fn deref_mut_writes_to_active_tab() {
        let mut s = ChatSessions::default();
        s.input.set_text("hello");

        s.new_tab();
        // New tab starts with empty input.
        assert_eq!(s.input.text(), "");

        // Switch back and verify tab 0 still has the old text.
        s.switch_to(0);
        assert_eq!(s.input.text(), "hello");
    }

    #[test]
    fn close_tab_removes_and_fixes_active_forward() {
        let mut s = ChatSessions::default();
        s.new_tab(); // tab 1
        s.new_tab(); // tab 2 (active)
        // Close tab 1 (before active).
        s.close_tab(1);
        assert_eq!(s.tab_count(), 2);
        // active was 2, now tab 2 is at index 1.
        assert_eq!(s.active_index(), 1);
    }

    #[test]
    fn close_active_tab_stays_in_bounds() {
        let mut s = ChatSessions::default();
        s.new_tab(); // tab 1 (now active)
        s.close_tab(1);
        assert_eq!(s.tab_count(), 1);
        assert_eq!(s.active_index(), 0); // clamped
    }

    #[test]
    fn close_tab_after_active_leaves_active_unchanged() {
        let mut s = ChatSessions::default();
        s.new_tab(); // tab 1
        s.switch_to(0); // active = 0
        s.close_tab(1); // remove tab 1 (after active)
        assert_eq!(s.tab_count(), 1);
        assert_eq!(s.active_index(), 0); // unchanged
    }

    #[test]
    fn close_only_tab_resets_to_fresh_default() {
        let mut s = ChatSessions::default();
        s.title = "dirty".to_string();
        s.messages.push(crate::chat::ChatMessage::user("hi"));
        s.close_tab(0);
        assert_eq!(s.tab_count(), 1);
        assert_eq!(s.active_index(), 0);
        // The sole tab is a fresh default, not the dirty one.
        assert_eq!(s.title, DEFAULT_CHAT_TITLE);
        assert!(s.messages.is_empty());
    }

    #[test]
    fn close_tab_out_of_range_is_noop() {
        let mut s = ChatSessions::default();
        s.close_tab(99); // no-op
        assert_eq!(s.tab_count(), 1);
        assert_eq!(s.active_index(), 0);
    }

    #[test]
    fn tab_mut_returns_the_indexed_tab() {
        let mut s = ChatSessions::default();
        s.new_tab(); // tab 1 (active)
        // Write into tab 0 by index even though tab 1 is active.
        s.tab_mut(0).unwrap().title = "zero".to_string();
        s.tab_mut(1).unwrap().title = "one".to_string();
        // Read back via switch_to to prove the writes landed per-index.
        s.switch_to(0);
        assert_eq!(s.title, "zero");
        s.switch_to(1);
        assert_eq!(s.title, "one");
    }

    #[test]
    fn tab_mut_out_of_range_is_none() {
        let mut s = ChatSessions::default();
        assert!(s.tab_mut(1).is_none());
        assert!(s.tab_mut(99).is_none());
    }

    #[test]
    fn run_tab_mut_targets_bound_tab_not_active() {
        let mut s = ChatSessions::default();
        s.new_tab(); // tab 1 (active)
        // Run bound to tab 0; active is tab 1. The pump must write to tab 0.
        s.run_tab_mut(Some(0)).messages.push(crate::chat::ChatMessage::user("run"));
        // Active tab (1) stayed empty.
        assert!(s.active().messages.is_empty());
        // Tab 0 got the message.
        s.switch_to(0);
        assert_eq!(s.messages.len(), 1);
    }

    #[test]
    fn run_tab_mut_falls_back_to_active_when_none_or_stale() {
        let mut s = ChatSessions::default();
        s.new_tab(); // tab 1 (active)
        // None binding → active tab.
        s.run_tab_mut(None).messages.push(crate::chat::ChatMessage::user("a"));
        // Out-of-range binding → active tab.
        s.run_tab_mut(Some(99)).messages.push(crate::chat::ChatMessage::user("b"));
        assert_eq!(s.active().messages.len(), 2);
        // Tab 0 untouched.
        s.switch_to(0);
        assert!(s.messages.is_empty());
    }

    #[test]
    fn new_tab_preserves_old_tab_messages_and_starts_blank() {
        // Mirrors the "+" / NewChat handler path: a fresh tab must NOT reset
        // the previous tab (the MT.1-review regression was that `new_chat()`
        // wiped the active tab in place before a new tab was pushed).
        let mut s = ChatSessions::default();
        s.title = "Working chat".to_string();
        s.messages.push(crate::chat::ChatMessage::user("keep me"));
        s.messages
            .push(crate::chat::ChatMessage::assistant_streaming());

        let idx = s.new_tab();
        // Exactly one new tab — not two.
        assert_eq!(idx, 1);
        assert_eq!(s.tab_count(), 2);
        // The new tab is active and blank.
        assert_eq!(s.active_index(), 1);
        assert!(s.messages.is_empty());
        assert_eq!(s.title, DEFAULT_CHAT_TITLE);
        // The old tab kept its transcript + title intact.
        s.switch_to(0);
        assert_eq!(s.messages.len(), 2);
        assert_eq!(s.title, "Working chat");
    }

    #[test]
    fn host_close_running_tab_pattern_aborts_and_clears() {
        // Models the host's `close_chat_tab` decision (the run abort lives on
        // the host; here we exercise just the index bookkeeping the host
        // delegates to `ChatSessions` + `adjust_running_tab_after_close`).
        let mut s = ChatSessions::default();
        s.new_tab(); // tab 1
        s.new_tab(); // tab 2 (active)
        let mut running_tab = Some(1usize);

        // Close the running tab → host clears the binding, then removes it.
        let closed = 1;
        if running_tab == Some(closed) {
            running_tab = None;
        } else if let Some(r) = running_tab {
            running_tab = adjust_running_tab_after_close(r, closed);
        }
        s.close_tab(closed);
        assert_eq!(running_tab, None);
        assert_eq!(s.tab_count(), 2);
    }

    #[test]
    fn host_close_earlier_tab_shifts_running_binding() {
        let mut s = ChatSessions::default();
        s.new_tab(); // tab 1
        s.new_tab(); // tab 2 (running)
        let mut running_tab = Some(2usize);

        // Close tab 0 (BEFORE the running tab) → binding shifts to 1.
        let closed = 0;
        if running_tab == Some(closed) {
            running_tab = None;
        } else if let Some(r) = running_tab {
            running_tab = adjust_running_tab_after_close(r, closed);
        }
        s.close_tab(closed);
        assert_eq!(running_tab, Some(1));
        assert_eq!(s.tab_count(), 2);
        // The run's transcript still resolves to the right (shifted) tab.
        s.run_tab_mut(running_tab)
            .messages
            .push(crate::chat::ChatMessage::user("still mine"));
        s.switch_to(1);
        assert_eq!(s.messages.len(), 1);
    }

    #[test]
    fn adjust_running_tab_after_close_shifts_and_clears() {
        // Closing the bound tab itself clears the binding.
        assert_eq!(adjust_running_tab_after_close(2, 2), None);
        // Closing a tab BEFORE the bound tab shifts it left.
        assert_eq!(adjust_running_tab_after_close(2, 0), Some(1));
        assert_eq!(adjust_running_tab_after_close(2, 1), Some(1));
        // Closing a tab AFTER the bound tab leaves it unchanged.
        assert_eq!(adjust_running_tab_after_close(2, 3), Some(2));
    }
}
