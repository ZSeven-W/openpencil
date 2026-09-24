//! The phone (compact touch) reading surface over the generation
//! workspace: which boards read as pages vs. one long page, the board a
//! 改这一页 follow-up is bound to, and opening a finished document for
//! reading without a Home brief.
//!
//! The reader is not a second workspace. It is the SAME `WorkspaceState`
//! shown through a phone composition: `visible` means "the normal view
//! of this work is up", and only the presentation differs between a
//! desktop window and a compact touch host.

use super::super::home::HomeFamily;
use super::{EditorUiState, WorkspacePhase, WorkspaceState, WorkspaceView};

/// Interactive targets of the compact works reader. Hit-test and paint
/// share one geometry function (`op_editor_ui::widgets::works_reader`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReaderHit {
    /// The header's ← back to Home (the run keeps going).
    Back,
    /// The 普通 half of the header's mode switch (the reader IS normal
    /// mode, so it only confirms the selected segment).
    ModeNormal,
    /// The 专业 half: the full mobile canvas over the same document.
    ModeProfessional,
    /// Previous board.
    Prev,
    /// Next board.
    Next,
    /// The status line's 停止生成 (while generating).
    Stop,
    /// The status line's 重试 (after a failed or stopped run).
    Retry,
    /// The bottom bar's 继续对话: opens the chat sheet.
    ContinueChat,
    /// The bottom bar's 改这一页: a follow-up bound to the current board.
    EditPage,
    /// The rendered board area (scroll / swipe, never a click).
    Stage,
}

/// The board a 改这一页 follow-up is bound to: its node id (the scope
/// the turn is fenced to) and its slot in the reader (for the copy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageEditTarget {
    pub board_id: String,
    pub index: usize,
}

impl PageEditTarget {
    /// One-based page number for user-facing copy.
    pub fn page_number(&self) -> usize {
        self.index + 1
    }
}

/// Families whose deliverable is one long page (a website, an
/// infographic): the reader fits the board width and scrolls
/// vertically instead of paging.
pub fn reads_as_long_page(family: HomeFamily) -> bool {
    matches!(family, HomeFamily::Web | HomeFamily::Infographic)
}

/// Whether the reader reserves its pager row. Paged families always do
/// (so the stage does not jump when the second board lands); a long
/// page only once the run produced more than one board.
pub fn reader_is_paged(family: HomeFamily, board_count: usize) -> bool {
    !reads_as_long_page(family) || board_count > 1
}

/// Guess what a document opened for reading is, from its boards'
/// `(width, height)`. There is no Home brief to say, and the answer only
/// picks the reading posture (paged vs. long page), never the content.
pub fn infer_reading_family(board_sizes: &[(f64, f64)]) -> HomeFamily {
    if board_sizes.is_empty() {
        return HomeFamily::AppUi;
    }
    let long = board_sizes.iter().any(|&(w, h)| w >= 600.0 && h >= w * 1.8);
    if long {
        return HomeFamily::Web;
    }
    let landscape = board_sizes
        .iter()
        .all(|&(w, h)| w >= 800.0 && h > 0.0 && w / h >= 1.25);
    if landscape {
        return HomeFamily::Presentation;
    }
    HomeFamily::AppUi
}

impl WorkspaceView {
    /// The view the phone reader opens a family with: one board at a
    /// time, or the whole long page fitted to width.
    pub fn default_for_reader(family: HomeFamily) -> Self {
        if reads_as_long_page(family) {
            Self::LongPage
        } else {
            Self::Single { index: 0 }
        }
    }
}

impl WorkspaceState {
    /// Open the normal view over a document that already exists (the
    /// works list, a blank canvas the user drew on): a finished work, no
    /// run, no brief. A later follow-up turn stamps its own epoch.
    pub fn open_for_reading(&mut self, family: HomeFamily, now_ms: u64) {
        self.active = true;
        self.visible = true;
        self.family = family;
        self.brief.clear();
        self.view = WorkspaceView::default_for_reader(family);
        self.selected = 0;
        self.phase = WorkspacePhase::Done;
        self.run_epoch = 0;
        self.hover = None;
        self.pressed = None;
        self.shown_at_ms = now_ms.max(1);
        self.fitted_board_count = 0;
        self.fitted_bounds = None;
        self.page_edit = None;
        self.page_edit_running = None;
        self.reader_pressed = None;
    }

    /// 改这一页: bind the NEXT send to `board_id`.
    pub fn stage_page_edit(&mut self, board_id: impl Into<String>, index: usize) {
        self.page_edit = Some(PageEditTarget {
            board_id: board_id.into(),
            index,
        });
    }

    /// Drop a staged (not yet sent) page edit — 继续对话 is a whole-work
    /// conversation, so it must not inherit an earlier page binding.
    pub fn clear_staged_page_edit(&mut self) {
        self.page_edit = None;
    }

    /// The launcher consumed a send: move the staged binding to the
    /// running run and hand it back so the launcher can scope the turn.
    pub fn begin_page_edit_turn(&mut self) -> Option<PageEditTarget> {
        let target = self.page_edit.take()?;
        self.page_edit_running = Some(target.clone());
        Some(target)
    }
}

impl EditorUiState {
    /// Whether the phone works reader owns the screen: the workspace's
    /// normal view is up on a compact touch host. Tablets and desktop
    /// windows keep their own compositions.
    pub fn works_reader_visible(&self) -> bool {
        self.workspace.visible && self.compact_layout()
    }
}

#[cfg(test)]
#[path = "workspace_reader_tests.rs"]
mod tests;
