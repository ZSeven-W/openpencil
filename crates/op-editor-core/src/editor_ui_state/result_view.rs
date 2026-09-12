//! The finished-product 成品视图 shown after a Home-launched generation.
//!
//! The result view is a full-surface takeover over the same `EditorState`
//! as the canvas, exactly like Home. It owns only transient chrome state:
//! the boards it is presenting (captured from the active page when the
//! generation finished) and the pointer feedback for its hit targets.

use super::home::HomeFamily;

/// Interactive target ids used by result-view hover and pressed feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultHit {
    BackHome,
    Professional,
    Screen(usize),
    EditThisScreen,
    PlayPrototype,
    ComponentsVariables,
    Restyle,
    Export,
    FullEdit,
}

/// Entrance motion budget for the boards row: fade + rise per board,
/// staggered. Shared by the widget's paint pass and the host's frame
/// scheduler so both agree on when the motion is over.
pub const RESULT_ENTER_BOARD_MS: u64 = 320;
/// Delay between consecutive boards starting their entrance.
pub const RESULT_ENTER_STAGGER_MS: u64 = 70;
/// The right panel slides in over its own (shorter) window.
pub const RESULT_ENTER_PANEL_MS: u64 = 260;

/// Transient state for the post-generation result view.
///
/// `awaiting_generation` is the "reopen me when this turn ends" intent:
/// Home's send arms it, the desktop's idle edge fulfils it. Everything
/// else describes the view once it is on screen.
#[derive(Debug, Clone, Default)]
pub struct ResultViewState {
    pub visible: bool,
    pub family: Option<HomeFamily>,
    pub brief: String,
    pub root_ids: Vec<String>,
    pub selected: usize,
    pub hover: Option<ResultHit>,
    pub pressed: Option<ResultHit>,
    pub shown_at_ms: u64,
    pub awaiting_generation: bool,
}

impl ResultViewState {
    /// Arm the reopen intent as a Home-launched generation starts. The
    /// family and brief are copied from Home so the finished view can
    /// repeat them without Home still being visible.
    pub fn arm_for_generation(&mut self, family: HomeFamily, brief: impl Into<String>) {
        self.family = Some(family);
        self.brief = brief.into();
        self.awaiting_generation = true;
    }

    /// Take the stage with `root_ids` in page order. Returns whether the
    /// view actually opened; an empty board list must never take over the
    /// canvas (there is nothing to show), so it only cancels the intent.
    pub fn open(&mut self, root_ids: Vec<String>, now_ms: u64) -> bool {
        self.awaiting_generation = false;
        if root_ids.is_empty() {
            return false;
        }
        self.root_ids = root_ids;
        self.selected = 0;
        self.shown_at_ms = now_ms;
        self.visible = true;
        self.hover = None;
        self.pressed = None;
        true
    }

    /// Drop back to the canvas, keeping the captured boards so the view
    /// could be reopened without re-deriving them.
    pub fn hide(&mut self) {
        self.visible = false;
        self.hover = None;
        self.pressed = None;
    }

    /// Drop a pending reopen intent without changing anything else — used
    /// when the turn finished with no boards.
    pub fn cancel_awaited_generation(&mut self) {
        self.awaiting_generation = false;
    }

    /// Clear every transient flag for a New / Open document swap. A
    /// document replaced while a generation is still running must never
    /// pop a result view for boards that no longer exist.
    pub fn reset_for_new_document(&mut self) {
        self.hide();
        self.cancel_awaited_generation();
        self.root_ids.clear();
        self.family = None;
        self.brief.clear();
        self.shown_at_ms = 0;
    }

    /// The next frame instant the entrance motion still needs, or `None`
    /// when the view is hidden or the motion has settled.
    pub fn entrance_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        if !self.visible {
            return None;
        }
        let boards_end = RESULT_ENTER_BOARD_MS
            + RESULT_ENTER_STAGGER_MS * self.root_ids.len().saturating_sub(1) as u64;
        let end = self
            .shown_at_ms
            .saturating_add(boards_end.max(RESULT_ENTER_PANEL_MS));
        (now_ms < end).then_some(end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_view_is_hidden_with_no_pending_generation() {
        let view = ResultViewState::default();
        assert!(!view.visible);
        assert!(!view.awaiting_generation);
        assert!(view.root_ids.is_empty());
        assert_eq!(view.family, None);
    }

    #[test]
    fn arming_copies_the_home_brief_and_waits_for_the_turn() {
        let mut view = ResultViewState::default();
        view.arm_for_generation(HomeFamily::AppUi, "取餐预约，3 个页面");
        assert!(view.awaiting_generation);
        assert!(!view.visible, "arming must not take over mid-generation");
        assert_eq!(view.family, Some(HomeFamily::AppUi));
        assert_eq!(view.brief, "取餐预约，3 个页面");
    }

    #[test]
    fn opening_takes_the_stage_with_fresh_pointer_state() {
        let mut view = ResultViewState::default();
        view.arm_for_generation(HomeFamily::AppUi, "取餐预约");
        view.hover = Some(ResultHit::Export);
        view.pressed = Some(ResultHit::Export);
        assert!(view.open(vec!["a".into(), "b".into()], 5_000));
        assert!(view.visible);
        assert!(!view.awaiting_generation);
        assert_eq!(view.root_ids, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(view.selected, 0);
        assert_eq!(view.shown_at_ms, 5_000);
        assert_eq!(view.hover, None);
        assert_eq!(view.pressed, None);
    }

    #[test]
    fn opening_with_no_boards_only_cancels_the_intent() {
        let mut view = ResultViewState::default();
        view.arm_for_generation(HomeFamily::AppUi, "取餐预约");
        assert!(!view.open(Vec::new(), 5_000));
        assert!(!view.visible, "an empty generation stays on the canvas");
        assert!(!view.awaiting_generation);
    }

    #[test]
    fn hiding_returns_to_the_canvas_but_keeps_the_boards() {
        let mut view = ResultViewState::default();
        view.open(vec!["a".into()], 1_000);
        view.selected = 0;
        view.hide();
        assert!(!view.visible);
        assert_eq!(view.root_ids, vec!["a".to_string()]);
    }

    #[test]
    fn a_document_swap_clears_every_transient_flag() {
        let mut view = ResultViewState::default();
        view.arm_for_generation(HomeFamily::EventPoster, "周末音乐节");
        view.open(vec!["a".into()], 1_000);
        view.reset_for_new_document();
        assert!(!view.visible);
        assert!(!view.awaiting_generation);
        assert!(view.root_ids.is_empty());
        assert_eq!(view.family, None);
        assert!(view.brief.is_empty());
    }

    #[test]
    fn entrance_deadline_spans_the_stagger_and_settles() {
        let mut view = ResultViewState::default();
        assert_eq!(view.entrance_deadline_ms(0), None);
        view.open(vec!["a".into(), "b".into(), "c".into()], 1_000);
        // 320 ms of motion + 2 × 70 ms of stagger after the shown instant.
        assert_eq!(view.entrance_deadline_ms(1_000), Some(1_000 + 320 + 140));
        // A single board still waits out the full board window.
        let mut one = ResultViewState::default();
        one.open(vec!["a".into()], 1_000);
        assert_eq!(one.entrance_deadline_ms(1_000), Some(1_320));
        // Past the end there is nothing left to animate.
        assert_eq!(view.entrance_deadline_ms(1_000 + 320 + 140), None);
    }
}
