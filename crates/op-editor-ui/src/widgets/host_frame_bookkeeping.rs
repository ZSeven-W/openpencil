//! Per-frame bookkeeping both host spines do around paint: rotating the
//! panel caches' owner tokens, constructing canvas layout transitions,
//! and folding the animation wake-up deadlines.
//!
//! All three were byte-identical in `op-host-native/src/widget_host.rs`
//! and `op-host-web/src/widget_host.rs`. They are pure over
//! `EditorState` + the widget-layer scene / transition types, so they
//! live here and each host keeps only the fields and the platform
//! clauses that genuinely differ (the native host adds gesture-degrade,
//! pan-cache-restore, live-preview and git-clone wake-ups; the web host
//! has no deadline pump of its own).

use std::collections::HashMap;

use op_editor_core::{EditorState, NodeId};

use crate::layout_scene::LayoutScene;
use crate::widgets::{AIChatPlaceholder, CanvasLayoutTransition};
use crate::Rect;

/// Rotate `chat_panel_owner` when the active chat session (tab) changed
/// since `last_chat_session_index` was recorded.
///
/// A fresh owner means the new tab's display-frame cursor hint reads
/// `None` (the thread-local slot still belongs to the old owner) until
/// this tab's next paint re-resolves and re-stamps it — the documented
/// one-frame isolation. Hosts call this at the top of the paint / probe
/// entry points so the very next resolve stores under the rotated owner.
pub fn rotate_chat_owner_if_session_changed(
    state: &EditorState,
    chat_panel_owner: &mut u64,
    last_chat_session_index: &mut usize,
) {
    let active = state.chat.active_index();
    if active != *last_chat_session_index {
        *last_chat_session_index = active;
        *chat_panel_owner = AIChatPlaceholder::next_owner();
    }
}

/// Rotate `chat_panel_owner` NOW, unconditionally — even when
/// `chat.active_index()` is unchanged.
///
/// Hosts call this synchronously at each session-mutation site (tab
/// switch / new tab / close tab). A tab switch changes the active
/// session but a pointer move can arrive before the next paint and run
/// the event-time cursor-shape hint, which would otherwise pair the
/// previous session's cached geometry with the new session's live
/// messages. Rotating unconditionally also covers same-index session
/// replacement — closing active tab 0 installs the next session at index
/// 0, closing the sole tab replaces it in place — which the index-only
/// poll in [`rotate_chat_owner_if_session_changed`] misses.
pub fn force_rotate_chat_owner(
    state: &EditorState,
    chat_panel_owner: &mut u64,
    last_chat_session_index: &mut usize,
) {
    *chat_panel_owner = AIChatPlaceholder::next_owner();
    *last_chat_session_index = state.chat.active_index();
}

/// Transition interpolating every node from `before` to `after`.
pub fn transition_between(
    before: &LayoutScene,
    after: &LayoutScene,
    now_ms: u64,
) -> Option<CanvasLayoutTransition> {
    CanvasLayoutTransition::between(before, after, now_ms)
}

/// [`transition_between`] with one node held out — the node the user is
/// actively dragging must not be animated toward its own drop position.
pub fn transition_between_excluding(
    before: &LayoutScene,
    after: &LayoutScene,
    now_ms: u64,
    excluded_id: &NodeId,
) -> Option<CanvasLayoutTransition> {
    CanvasLayoutTransition::between_excluding(before, after, now_ms, Some(excluded_id.as_str()))
}

/// Transition that animates exactly one node out of `bounds` — used
/// where the pre-mutation scene is unavailable but the moved node's
/// starting rect is known.
pub fn transition_from_single_bounds(
    node_id: &NodeId,
    bounds: Rect,
    now_ms: u64,
) -> Option<CanvasLayoutTransition> {
    let mut starts = HashMap::new();
    starts.insert(node_id.as_str().to_string(), bounds);
    CanvasLayoutTransition::from_start_bounds(starts, now_ms)
}

/// Fold `deadline` into `current`, keeping whichever wake-up comes
/// first. The scheduler wants the EARLIEST pending deadline, so every
/// contributing clause reduces through this.
pub fn earliest(current: Option<u64>, deadline: u64) -> Option<u64> {
    Some(current.map_or(deadline, |current| current.min(deadline)))
}

/// Fold a preview session deadline into the host's shared deadline chain.
///
/// Preview deadlines are absolute host-clock milliseconds. A stale deadline
/// can otherwise re-arm a native timer or browser pump at the current instant
/// forever, so a reported deadline is always kept at least one millisecond in
/// the future. `None` leaves the existing host clauses unchanged.
pub fn fold_preview_deadline(
    current: Option<u64>,
    preview_deadline: Option<u64>,
    now_ms: u64,
) -> Option<u64> {
    let Some(deadline) = preview_deadline else {
        return current;
    };
    earliest(current, deadline.max(now_ms.saturating_add(1)))
}

/// The platform-independent part of the next animation wake-up: agent
/// reveal + generation-scan indicators, the canvas layout transition,
/// the focused text input's caret blink, and the two hover dwells (the
/// top-bar tooltip's and the chat's pinned-style card's). `None` =
/// nothing animating from these sources.
///
/// Hosts fold their own platform clauses on top through [`earliest`].
pub fn base_animation_deadline_ms(
    state: &EditorState,
    transition: Option<&CanvasLayoutTransition>,
    now_ms: u64,
) -> Option<u64> {
    let mut next = op_editor_core::agent_indicators::next_reveal_deadline_ms(now_ms);
    if let Some(deadline) =
        op_editor_core::agent_indicators::next_generation_scan_deadline_ms(now_ms)
    {
        next = earliest(next, deadline);
    }
    if let Some(deadline) = transition.and_then(|t| t.next_deadline_ms(now_ms)) {
        next = earliest(next, deadline);
    }
    if let Some(input) = state.active_text_input() {
        next = earliest(next, input.next_blink_flip_ms(now_ms));
    }
    // A dwelling top-bar tooltip becomes due without any further input,
    // so its instant has to reach the scheduler or the tooltip would
    // only ever appear on the user's next mouse jiggle.
    if let Some(deadline) =
        crate::widgets::top_bar_tooltip::next_deadline_ms(&state.editor_ui, now_ms)
    {
        next = earliest(next, deadline);
    }
    // A toast expires at a wall instant with no input event behind it, so its
    // expiry has to reach the scheduler or the banner would sit there until
    // the user's next mouse move. Folded in here rather than in either host,
    // which is what keeps native and web identical.
    if let Some(deadline) =
        crate::widgets::editor_toast_flow::next_deadline_ms(&state.editor_ui, now_ms)
    {
        next = earliest(next, deadline);
    }
    // Same reason for the chat's pinned-style card: its dwell expires without
    // any further input, and a card that only appeared on the next mouse
    // jiggle would read as the hover not working.
    if let Some(deadline) =
        crate::widgets::ai_chat_style_card::next_deadline_ms(&state.editor_ui, now_ms)
    {
        next = earliest(next, deadline);
    }
    next
}

#[cfg(test)]
mod tests {
    use super::fold_preview_deadline;

    #[test]
    fn preview_deadline_is_folded_and_clamped_only_when_present() {
        assert_eq!(fold_preview_deadline(None, None, 100), None);
        assert_eq!(fold_preview_deadline(Some(150), None, 100), Some(150));
        assert_eq!(fold_preview_deadline(None, Some(116), 100), Some(116));
        assert_eq!(fold_preview_deadline(None, Some(100), 100), Some(101));
        assert_eq!(fold_preview_deadline(Some(300), Some(200), 100), Some(200));
    }
}
