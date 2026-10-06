//! The Studio generation workspace: a docked-chat + real-canvas chrome
//! over the SAME `EditorState` as the professional editor.
//!
//! Home's 开始设计 opens the workspace instead of hiding the editor:
//! the chat panel pins into the LEFT PANEL — the very rail the
//! professional editor shows, open at `layer_panel_width` while
//! `sidebar_open` — while the real canvas renders the boards the
//! generation produces. The workspace owns only transient chrome state —
//! phase, view mode, pointer feedback, and the run-epoch fence that
//! keeps a stopped run's late events from repainting it. The dock's
//! metrics deliberately live on the shared panel state, not here: the
//! dock IS the rail, and a second width is how the two columns drifted
//! apart in the first place.

use super::home::{HomeFamily, TaskDraft};
use super::{EditorUiState, LeftPanelTab};
use crate::tool::Tool;

/// Workspace header height (back button, doc tile, title, actions).
pub const WORKSPACE_HEADER_H: f32 = 64.0;
/// Canvas toolbar height (view segments, zoom, deck actions).
pub const WORKSPACE_TOOLBAR_H: f32 = 44.0;
/// Bottom deck-strip height for the presentation family.
pub const WORKSPACE_DECK_STRIP_H: f32 = 110.0;

/// Entrance motion window for the chrome (fade + 8 px rise), shared by
/// paint and the host frame scheduler.
pub const WORKSPACE_ENTER_MS: u64 = 320;

/// Where the workspace's generation stands, derived from real chat /
/// orchestrator state — never a timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspacePhase {
    /// A turn is streaming or agents are running.
    #[default]
    Generating,
    /// The run's idle edge arrived with at least one board.
    Done,
    /// The user pressed Stop (`pending_stop_chat` drained).
    Stopped,
    /// The run ended with an error / zero boards.
    Failed,
}

/// How the right-hand canvas presents the boards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceView {
    /// Fit every board (`zoom_to_fit`).
    AllBoards,
    /// Fit one board (`zoom_to_fit_node`); `index` is the board slot.
    Single { index: usize },
    /// Fit the board WIDTH into the canvas width; wheel pans vertically.
    LongPage,
    /// Deck overview: fit all boards (presentation family).
    Overview,
}

impl WorkspaceView {
    /// The default view a family opens with.
    pub fn default_for(family: HomeFamily) -> Self {
        match family {
            HomeFamily::Presentation => Self::Single { index: 0 },
            HomeFamily::Web | HomeFamily::Infographic => Self::LongPage,
            _ => Self::AllBoards,
        }
    }
}

/// Interactive target ids used by workspace hover and pressed feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceHit {
    /// The header's ← back-to-home circle.
    Back,
    /// The header's 导出 outline button.
    Export,
    /// The header's 专业编辑 outline button.
    Professional,
    /// The toolbar's collapse/expand-chat toggle.
    ToggleDock,
    /// The dock's drag handle (starts a width drag, not a click).
    DockResize,
    /// One segment of the view-mode control.
    View(WorkspaceView),
    /// Previous board (Single / deck).
    Prev,
    /// Next board (Single / deck).
    Next,
    ZoomOut,
    ZoomFit,
    ZoomIn,
    /// One thumbnail in the deck strip.
    Thumb(usize),
    /// The strip's 总览 toggle.
    Overview,
    /// The strip's 放映 button.
    Play,
    /// The failed-phase banner's 重试 button.
    Retry,
    /// The failed-phase banner's 返回修改 button.
    ReturnEdit,
}

/// Transient state for the generation workspace. Never persisted.
#[derive(Debug, Clone)]
pub struct WorkspaceState {
    /// A workspace exists for this document/run. Stays true while the
    /// user visits Home so the 回到工作区 link can return to it.
    pub active: bool,
    /// The workspace chrome is painted and the canvas is docked.
    pub visible: bool,
    pub family: HomeFamily,
    /// The brief that launched this run (retried verbatim, shown as the
    /// title fallback).
    pub brief: String,
    /// Copy of the task options the run captured.
    pub options: TaskDraft,
    pub view: WorkspaceView,
    /// The deck strip's selected board slot.
    pub selected: usize,
    pub phase: WorkspacePhase,
    /// The `agent_indicators` epoch this workspace's run captured at
    /// launch. `0` = not stamped yet (pre-launch); finish/stop/error
    /// edges with a different epoch are ignored.
    pub run_epoch: u64,
    /// The tool to restore when the user enters the professional canvas.
    pub previous_tool: Option<Tool>,
    pub hover: Option<WorkspaceHit>,
    pub pressed: Option<WorkspaceHit>,
    /// Wall-clock instant the entrance motion phases against; `0` means
    /// the host paint has not stamped it yet.
    pub shown_at_ms: u64,
    /// Board count the last AllBoards refit ran against. During
    /// generation a refit fires once per NEW count so the user watches
    /// boards land without the camera fighting manual pans afterwards.
    pub fitted_board_count: usize,
    /// Content bounds the generation camera last fitted, as
    /// `(x, y, w, h)`. The board COUNT alone is not enough to know the
    /// camera is still right: the orchestrator re-flows the boards it
    /// already created into rows and resizes them, which moves the
    /// content without adding a board, and the stale fit then left the
    /// deck off-centre and clipped (measured 2026-09-13).
    pub fitted_bounds: Option<(f32, f32, f32, f32)>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            active: false,
            visible: false,
            family: HomeFamily::AppUi,
            brief: String::new(),
            options: TaskDraft::default(),
            view: WorkspaceView::default_for(HomeFamily::AppUi),
            selected: 0,
            phase: WorkspacePhase::Generating,
            run_epoch: 0,
            previous_tool: None,
            hover: None,
            pressed: None,
            shown_at_ms: 0,
            fitted_board_count: 0,
            fitted_bounds: None,
        }
    }
}

impl WorkspaceState {
    /// Take the stage for a Home-launched generation. `run_epoch` is `0`
    /// until the desktop launch stamps the live `agent_indicators`
    /// epoch; the fence accepts any epoch while unstamped.
    pub fn open_for_generation(
        &mut self,
        family: HomeFamily,
        brief: impl Into<String>,
        options: TaskDraft,
        run_epoch: u64,
        now_ms: u64,
        previous_tool: Option<Tool>,
    ) {
        self.active = true;
        self.visible = true;
        self.family = family;
        self.brief = brief.into();
        self.options = options;
        self.view = WorkspaceView::default_for(family);
        self.selected = 0;
        self.phase = WorkspacePhase::Generating;
        self.run_epoch = run_epoch;
        self.previous_tool = previous_tool;
        self.hover = None;
        self.pressed = None;
        self.shown_at_ms = now_ms.max(1);
        self.fitted_board_count = 0;
        self.fitted_bounds = None;
    }

    /// 专业编辑: drop the chrome (rails come back, the previous tool is
    /// restored host-side) but keep every workspace fact so returning
    /// re-enters the same view on the same document.
    pub fn enter_professional(&mut self) {
        self.visible = false;
        self.hover = None;
        self.pressed = None;
    }

    /// 回到工作区: take the chrome back after Home or the professional
    /// canvas. Restamps the entrance so the return reads as a transition.
    pub fn reenter(&mut self, now_ms: u64) {
        if !self.active || self.visible {
            return;
        }
        self.visible = true;
        self.hover = None;
        self.pressed = None;
        self.shown_at_ms = now_ms.max(1);
    }

    /// Whether a terminal-edge epoch belongs to this workspace's run.
    /// An unstamped epoch (`0`) accepts any edge — the launch has not
    /// identified the run yet, so there is nothing to discriminate with.
    fn owns_epoch(&self, epoch: u64) -> bool {
        self.run_epoch == 0 || self.run_epoch == epoch
    }

    /// The idle edge with ≥1 board finished the run. A stale epoch or a
    /// non-generating phase ignores the edge (Stopped/Failed are sticky;
    /// a retry re-opens Generating first).
    pub fn mark_done(&mut self, epoch: u64) -> bool {
        if self.phase != WorkspacePhase::Generating || !self.owns_epoch(epoch) {
            return false;
        }
        self.phase = WorkspacePhase::Done;
        true
    }

    /// The user pressed Stop. Marked from `drain_stop_request`; the idle
    /// edge that follows must not flip a stopped workspace to Done.
    pub fn mark_stopped(&mut self, epoch: u64) -> bool {
        if !self.owns_epoch(epoch) {
            return false;
        }
        self.phase = WorkspacePhase::Stopped;
        true
    }

    /// The run ended with an error or zero boards.
    pub fn mark_failed(&mut self, epoch: u64) -> bool {
        if self.phase != WorkspacePhase::Generating || !self.owns_epoch(epoch) {
            return false;
        }
        self.phase = WorkspacePhase::Failed;
        true
    }

    /// A follow-up / retry turn is live again for this workspace.
    pub fn resume_generating(&mut self, run_epoch: u64) {
        if !self.active {
            return;
        }
        self.phase = WorkspacePhase::Generating;
        self.run_epoch = run_epoch;
    }

    /// Step the deck selection, clamped into `board_count`. Returns
    /// whether the selection moved.
    pub fn step_selected(&mut self, delta: i32, board_count: usize) -> bool {
        if board_count == 0 {
            return false;
        }
        let target = (self.selected as i32 + delta).clamp(0, board_count as i32 - 1) as usize;
        let moved = target != self.selected;
        self.selected = target;
        if moved {
            self.view = match self.view {
                WorkspaceView::Single { .. } => WorkspaceView::Single { index: target },
                other => other,
            };
        }
        moved
    }

    /// Select a board slot (deck strip / view switch). Clamps into
    /// `board_count`.
    pub fn select_board(&mut self, index: usize, board_count: usize) {
        if board_count == 0 {
            return;
        }
        self.selected = index.min(board_count - 1);
        self.view = match self.view {
            WorkspaceView::Single { .. } => WorkspaceView::Single {
                index: self.selected,
            },
            other => other,
        };
    }

    /// Clear everything for a New / Open document swap. A workspace
    /// opened for the replaced document must never repaint over a
    /// document it does not describe.
    pub fn reset_for_new_document(&mut self) {
        self.active = false;
        self.visible = false;
        self.brief.clear();
        self.phase = WorkspacePhase::Generating;
        self.run_epoch = 0;
        self.previous_tool = None;
        self.hover = None;
        self.pressed = None;
        self.shown_at_ms = 0;
        self.fitted_board_count = 0;
        self.fitted_bounds = None;
        self.selected = 0;
    }

    /// The next frame instant the entrance motion still needs, or
    /// `None` when hidden or settled.
    pub fn entrance_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        if !self.visible || self.shown_at_ms == 0 {
            return None;
        }
        let end = self.shown_at_ms.saturating_add(WORKSPACE_ENTER_MS);
        (now_ms < end).then_some(end)
    }
}

impl EditorUiState {
    /// Whether the AI chat panel is pinned into a left column right
    /// now — the workspace's dock while the workspace is up, else the
    /// professional editor's Chat tab. THE one predicate answering "is
    /// the chat pinned into a column?": the press tiers that inert the
    /// panel's floating controls and the shared click flow both read
    /// this and nothing else. Touch chrome never pins — it hosts the
    /// chat as its own bottom sheet.
    pub fn chat_pinned(&self) -> bool {
        if self.touch_chrome() {
            return false;
        }
        if self.workspace.visible {
            return self.sidebar_open;
        }
        self.sidebar_open && self.slides_panel.tab == LeftPanelTab::Chat
    }

    /// Whether the chat shows nothing but its composer, docked at the
    /// canvas floor.
    ///
    /// True on desktop chrome whenever the conversation's home — the
    /// rail's Agent tab — is not the tab on show. The full transcript
    /// then has exactly one place to live, and what sits over the canvas
    /// is a launcher: type into it, or open it, and the rail switches to
    /// the Agent tab where the conversation actually happens.
    pub fn chat_composer_only(&self) -> bool {
        if self.touch_chrome() || self.workspace.visible || self.preview.mode {
            return false;
        }
        !self.chat_pinned()
    }

    /// Open the generation workspace over this chrome: the workspace
    /// state takes the stage, the left panel comes OPEN (it is the
    /// dock), and the Chat tab is showing so a run always lands with
    /// its conversation visible and 专业编辑 keeps it that way.
    pub fn open_workspace_for_generation(
        &mut self,
        family: HomeFamily,
        brief: impl Into<String>,
        options: TaskDraft,
        run_epoch: u64,
        now_ms: u64,
        previous_tool: Option<Tool>,
    ) {
        self.workspace.open_for_generation(
            family,
            brief,
            options,
            run_epoch,
            now_ms,
            previous_tool,
        );
        self.sidebar_open = true;
        self.enter_chat_tab();
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
