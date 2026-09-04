//! `GitPanel` — the floating in-app Git panel.
//!
//! Shows the open document's git repository — current branch,
//! working-tree change counts, recent commits — and offers the
//! interactive actions: a commit-message input + Commit / Refresh /
//! Pull buttons. Clicking the status line or a commit / conflict row
//! opens an in-panel scrollable unified-diff view.
//!
//! The panel is platform-free: it is filled by the desktop host
//! from its `GitSession` and never calls git itself. A click is
//! mapped to a [`GitPanelHit`] by [`GitPanel::hit_test`]; the host
//! turns that into focus changes / a `GitPanelState::pending_action`.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
pub use jian_widgets::components::menu::MenuHit;
pub use jian_widgets::components::select::SelectHit;
use op_editor_core::{EditorState, GitButton, GitPanelState};

/// Panel width in logical px (TS ready/history popover is `w-[420px]`).
pub const GIT_PANEL_WIDTH: f32 = 420.0;
/// Panel width while a diff is open — wide enough for ~95-column diffs.
pub const GIT_DIFF_PANEL_WIDTH: f32 = 620.0;
/// Inset from the canvas corner the panel floats at.
pub const GIT_PANEL_INSET: f32 = 16.0;

pub(super) const PAD: f32 = 16.0;

// Element positions as fixed offsets from the panel's top edge.
// Keeping them constant (the action area never shifts with the
// commit count) lets paint + hit-test share the exact same maths.
pub(super) const HEADER_BASELINE: f32 = 30.0;
const BRANCH_BASELINE: f32 = 56.0;
const STATUS_BASELINE: f32 = 78.0;
const DIVIDER_1_Y: f32 = 90.0;
const INPUT_TOP: f32 = 100.0;
pub(super) const INPUT_H: f32 = 28.0;
const BUTTON_TOP: f32 = 138.0;
pub(super) const BUTTON_H: f32 = 28.0;
const DIVIDER_2_Y: f32 = 180.0;
const COMMITS_LABEL_BASELINE: f32 = 200.0;
/// Baseline of the first commit row.
const COMMITS_FIRST_BASELINE: f32 = 222.0;
const COMMIT_ROW_H: f32 = 22.0;
const BRANCH_ROW_H: f32 = 22.0;
/// Gap from the "Branches" label baseline to the first branch row.
const BRANCH_LABEL_GAP: f32 = 10.0;
pub(super) const FOOTER_H: f32 = 22.0;
const BUTTON_GAP: f32 = 8.0;
/// Gap between the commit list and the Branches section.
pub(super) const SECTION_GAP: f32 = 16.0;

/// Most commits the panel shows.
const MAX_COMMITS: usize = 8;
/// Most branches the panel lists.
const MAX_BRANCHES: usize = 8;
/// Commit-summary truncation length (chars).
const SUMMARY_MAX: usize = 38;

/// Fixed panel height while a diff view is open. The remaining
/// diff-view metrics + rendering live in the `git_panel_diff`
/// sibling module (split out for the 800-line file cap).
pub(super) const DIFF_VIEW_HEIGHT: f32 = 484.0;

/// Empty-state (no-repo) panel size — wider + taller than the status
/// view to fit the centred onboarding UI (clock + heading + three
/// Init/Open/Clone cards + note). The card-row metrics + paint live
/// in the `git_panel_empty` sibling module (split for the 800 cap).
pub(super) const EMPTY_STATE_WIDTH: f32 = 380.0;
pub(super) const EMPTY_STATE_HEIGHT: f32 = 300.0;

/// What a click landed on inside the Git panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitPanelHit {
    /// The commit-message input box — focus it.
    CommitInput,
    /// The Commit button.
    Commit,
    /// The ready-view "Save milestone" button — save the live design to
    /// the tracked `.op` + stage + commit (TS `commitMilestone`).
    CommitMilestone,
    /// The Refresh button.
    Refresh,
    /// The Pull button.
    Pull,
    /// The Push button.
    Push,
    /// The ready-state header's `⎇ <branch> ▾` button — toggle the
    /// branch-picker dropdown.
    BranchPicker,
    /// The ready-state header's `…` button — toggle the overflow menu.
    Overflow,
    /// The overflow menu's "Remote settings" entry — open that subview.
    OverflowRemoteSettings,
    /// The overflow menu's "SSH keys" entry — set up SSH auth.
    OverflowSshKeys,
    /// The overflow menu's "Switch tracked file" entry — open the
    /// tracked-file picker subview.
    OverflowSwitchTracked,
    /// The overflow menu's "Clear commit author" entry.
    OverflowClearAuthor,
    /// The overflow menu's "Close repository" entry — unbind the repo.
    OverflowCloseRepo,
    /// A tracked-file-picker candidate row — select `candidate_files[index]`.
    TrackedPickerRow(usize),
    /// The tracked-file picker's "Track this file" button.
    TrackedPickerBind,
    /// The tracked-file picker's "Track and open" button.
    TrackedPickerBindOpen,
    /// The tracked-file picker's Back / Cancel button.
    TrackedPickerBack,
    /// Remote-settings "获取" — run `git fetch` on origin.
    FetchRemote,
    /// Commit-signature form — focus the 姓名 (name) input.
    AuthorNameInput,
    /// Commit-signature form — focus the 邮箱 (email) input.
    AuthorEmailInput,
    /// Commit-signature form "保存" — save identity + re-fire the commit.
    AuthorSave,
    /// Commit-signature form "取消" — dismiss without committing.
    AuthorCancel,
    /// SSH subview "生成新密钥" — generate a key for the origin host.
    SshGenerateKey,
    /// SSH subview "导入现有密钥" — import an existing private key.
    SshImportKey,
    /// A subview's `‹ Back` row — return to the overflow menu.
    OverflowBack,
    /// A click outside an open header popover (but inside the panel) —
    /// the host closes the popover and swallows the click.
    DismissPopover,
    /// The "Abort Merge" button (shown while a merge is in progress).
    AbortMerge,
    /// The "Complete Merge" button (shown while a merge is in
    /// progress; only actionable once conflicts are resolved).
    CompleteMerge,
    /// A branch row — switch to `branches[index]`.
    SwitchBranch(usize),
    /// A branch row's merge button — merge `branches[index]` into
    /// the current branch.
    MergeBranch(usize),
    /// Footer "新建分支" — enter the inline create-branch form.
    BranchCreateMode,
    /// Footer "合并分支" — enter merge mode (pick a branch to merge into HEAD).
    BranchMergeMode,
    /// The inline create-branch name input — focus it.
    BranchCreateInput,
    /// The inline create-branch submit button — create + switch.
    BranchCreateSubmit,
    /// Cancel a create / merge sub-mode — return to the branch list.
    BranchPickerCancel,
    /// The Remotes-section URL input box — focus it.
    RemoteInput,
    /// The Remotes-section "Set origin" button.
    SetRemote,
    /// The Remotes-section "SSH" button — set up SSH auth for the
    /// origin host.
    SetupSshAuth,
    /// The Remotes-section HTTPS-credential input box — focus it.
    HttpsInput,
    /// The Remotes-section "Login" button — store the HTTPS credential.
    SetHttpsAuth,
    /// The working-tree status line — open the whole-repo diff.
    ShowWorkingDiff,
    /// A recent-commit row — toggle its inline detail card
    /// (里程碑详情 — restore + copy-hash). TS `HistoryMilestoneRow`.
    ShowCommitDiff(usize),
    /// The expanded commit card's "恢复" button — roll the tracked
    /// document back to `recent_commits[index]` (TS `restoreCommit`).
    RestoreCommit(usize),
    /// The expanded commit card's "复制哈希" button — copy
    /// `recent_commits[index]`'s hash to the OS clipboard.
    CopyCommitHash(usize),
    /// A conflicted-file row (merge mode) — open that file's diff.
    ShowFileDiff(usize),
    /// A changed-file row's checkbox — toggle whether it is staged.
    ToggleStageFile(usize),
    /// A changed-file row's body — open that file's diff (where its
    /// hunks can be staged individually).
    ShowChangedFileDiff(usize),
    /// The diff view's ✕ — close the diff, returning to status mode.
    CloseDiff,
    /// The diff view's ▲ — page the diff up.
    DiffScrollUp,
    /// The diff view's ▼ — page the diff down.
    DiffScrollDown,
    /// The diff view's ◀ — scroll the diff left.
    DiffScrollLeft,
    /// The diff view's ▶ — scroll the diff right.
    DiffScrollRight,
    /// A diff-view hunk's "Stage" button — stage that hunk (index).
    StageHunk(usize),
    /// A merge-resolution row's "Ours" choice — flat conflict index.
    MergeChoiceOurs(usize),
    /// A merge-resolution row's "Theirs" choice — flat conflict index.
    MergeChoiceTheirs(usize),
    /// The merge-resolution "Apply" button.
    ApplyMergeResolution,
    /// The merge-resolution "Cancel" button.
    CancelMergeResolution,
    /// Empty-state "Init" card — create a local repo for the doc.
    EmptyInit,
    /// Empty-state "Open" card — bind an existing repo folder.
    EmptyOpen,
    /// Empty-state "Clone" card — clone from a remote.
    EmptyClone,
    /// Clone view — focus the URL field.
    CloneUrlInput,
    /// Clone view — focus the destination field.
    CloneDestInput,
    /// Clone view — open a native folder picker for the destination.
    CloneDestPick,
    /// Clone view — submit `git clone`.
    CloneSubmit,
    /// Clone view — cancel back to the empty state.
    CloneCancel,
    /// Inside the panel but not on an interactive target — the
    /// click is swallowed (and the commit input defocused).
    Inside,
}

/// What the panel's list slot is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListMode {
    /// Unresolved merge conflicts (a merge is in progress).
    Merge,
    /// Working-tree changed files — the per-file staging list.
    Changes,
    /// Recent-commit history (the working tree is clean).
    Commits,
}

/// Sub-rectangles of the panel's interactive action area — the
/// commit-message input plus the button row: 4 buttons normally
/// (Commit / Refresh / Pull / Push), 3 in merge mode (Abort /
/// Refresh / Complete).
pub(super) struct ActionRects {
    pub(super) input: Rect,
    pub(super) buttons: Vec<Rect>,
}

/// The floating Git panel, built from a [`GitPanelState`] snapshot.
pub struct GitPanel<'a> {
    pub(super) state: &'a GitPanelState,
    pub(super) theme: Theme,
    /// UI locale — every painted string goes through [`GitPanel::t`].
    pub(super) locale: op_editor_core::Locale,
    /// Wall-clock ms, for caret-blink animation. `0` (hit-test / tests)
    /// just yields a steady un-blinked caret.
    pub(super) now_ms: u64,
    /// Which Git action button is currently pressed by the primary pointer.
    pub(super) pressed: Option<GitButton>,
}

impl<'a> GitPanel<'a> {
    /// Build the panel for the editor, or `None` when it is closed.
    /// Hit-test / tests use this (no blink); paint uses
    /// [`GitPanel::for_editor_at`] to drive the caret blink.
    pub fn for_editor(state: &'a EditorState) -> Option<GitPanel<'a>> {
        Self::for_editor_at(state, 0)
    }

    /// Like [`GitPanel::for_editor`] but threads the wall-clock ms so
    /// the commit-input caret can blink.
    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<GitPanel<'a>> {
        let panel = &state.editor_ui.git_panel;
        if !panel.open {
            return None;
        }
        Some(GitPanel {
            state: panel,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.effective_locale(),
            now_ms,
            pressed: match state.editor_ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::Git(button)) => Some(button),
                _ => None,
            },
        })
    }

    /// Whether the cursor is over the button `hit` represents — drives
    /// the per-button hover wash. Compares the canonical `button_hover`
    /// (set by the host from the same hit-test) against `hit`'s mirror.
    /// `false` for non-button hits (inputs / dismiss / branch trigger).
    pub(super) fn is_hovered(&self, hit: GitPanelHit) -> bool {
        let mapped = crate::widgets::editor_state_ext::git_button_hover(hit);
        mapped.is_some() && self.state.button_hover == mapped
    }

    pub(super) fn is_pressed(&self, hit: GitPanelHit) -> bool {
        crate::widgets::editor_state_ext::git_button_hover(hit)
            .is_some_and(|b| self.pressed == Some(b))
    }

    /// Paint shared button feedback over `rect` when the cursor is over
    /// the button `hit` represents.
    pub(super) fn wash_if_hovered(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        r: f32,
        hit: GitPanelHit,
    ) {
        let hovered = self.is_hovered(hit);
        let pressed = self.is_pressed(hit);
        if (r - 6.0).abs() < 0.01 {
            crate::widgets::button::paint_ghost_button_feedback(
                cx.backend,
                &self.theme,
                rect,
                hovered,
                pressed,
            );
        } else {
            crate::widgets::button::paint_button_feedback_wash(
                cx.backend,
                &self.theme,
                rect,
                r,
                hovered,
                pressed,
            );
        }
    }

    /// Translate `key` through the locale tables — the panel paints
    /// no hardcoded UI strings.
    pub(super) fn t(&self, key: &'static str) -> &'static str {
        op_i18n::translate(self.locale, key)
    }

    /// Panel width for the current mode — wider while a diff or the
    /// merge-resolution view is open, and for the onboarding empty
    /// state (which lays out three cards in a row).
    pub fn panel_width(&self) -> f32 {
        if self.state.diff.is_some() || self.state.merge_resolve.is_some() {
            GIT_DIFF_PANEL_WIDTH
        } else if self.is_empty_state() {
            EMPTY_STATE_WIDTH
        } else {
            GIT_PANEL_WIDTH
        }
    }
}

mod geometry;
mod paint;

/// Char truncation with an ellipsis.
pub(super) use crate::util::truncate_ellipsis as truncate;
