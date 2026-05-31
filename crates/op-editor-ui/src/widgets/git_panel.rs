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
use op_editor_core::{EditorState, GitPanelState};

/// Panel width in logical px.
pub const GIT_PANEL_WIDTH: f32 = 320.0;
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
    /// A recent-commit row — open that commit's patch.
    ShowCommitDiff(usize),
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
            locale: state.editor_ui.locale,
            now_ms,
        })
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

    /// What the panel's list slot shows — conflicts during a merge,
    /// the working-tree changes when the tree is dirty, otherwise the
    /// recent-commit history.
    pub(super) fn list_mode(&self) -> ListMode {
        if self.state.merging {
            ListMode::Merge
        } else if !self.state.changed_files.is_empty() {
            ListMode::Changes
        } else {
            ListMode::Commits
        }
    }

    /// Row count of the list slot. At least one (the placeholder
    /// row), capped at `MAX_COMMITS`.
    fn list_rows(&self) -> usize {
        let len = match self.list_mode() {
            ListMode::Merge => self.state.conflicted_files.len(),
            ListMode::Changes => self.state.changed_files.len(),
            ListMode::Commits => self.state.recent_commits.len(),
        };
        len.clamp(1, MAX_COMMITS)
    }

    /// The panel's total height for the current content.
    pub fn height(&self) -> f32 {
        // The inline clone wizard takes over the whole panel.
        if self.state.clone_form.is_some() {
            return self.clone_view_height();
        }
        // The merge-resolution view sizes to its conflict count.
        if self.state.merge_resolve.is_some() {
            return self.resolve_view_height();
        }
        // Diff mode is a fixed-height scrollable view.
        if self.state.diff.is_some() {
            return DIFF_VIEW_HEIGHT;
        }
        if self.is_empty_state() {
            // Centred onboarding: clock + heading + cards + note.
            return EMPTY_STATE_HEIGHT;
        }
        if self.state.loading || !self.state.in_repo {
            // Header + one status line ("Loading…" / "not a repo")
            // + footer — no branch / action area / commit list.
            return HEADER_BASELINE + 24.0 + FOOTER_H + PAD;
        }
        // Clean bound-repo → the TS ready layout sizes to its history.
        if self.is_ready_state() {
            return self.ready_height();
        }
        // List + Branches + Remotes sections, then the footer.
        self.remotes_section_top() + self.remotes_block_height() + FOOTER_H + PAD
    }

    /// Panel-relative `y` where the Remotes section begins — below
    /// the list slot and the (optional) Branches section.
    pub(super) fn remotes_section_top(&self) -> f32 {
        let commit_rows = self.list_rows();
        let branch_count = self.state.branches.len().min(MAX_BRANCHES);
        // The Branches section is omitted entirely when empty.
        let branches_h = if branch_count == 0 {
            0.0
        } else {
            SECTION_GAP + BRANCH_LABEL_GAP + branch_count as f32 * BRANCH_ROW_H
        };
        COMMITS_FIRST_BASELINE + commit_rows as f32 * COMMIT_ROW_H + branches_h
    }

    /// The Branches section layout — the "Branches" label baseline
    /// and one clickable rect per listed branch. The list is empty
    /// when the repository has no branches yet. Shared by
    /// [`GitPanel::paint`] + [`GitPanel::hit_test`].
    pub(super) fn branch_layout(&self, panel: Rect) -> (f32, Vec<Rect>) {
        let commit_rows = self.list_rows();
        let label_baseline = panel.origin.y
            + COMMITS_FIRST_BASELINE
            + commit_rows as f32 * COMMIT_ROW_H
            + SECTION_GAP;
        let left = panel.origin.x + PAD;
        let inner_w = panel.size.x - PAD * 2.0;
        let first_row_top = label_baseline + BRANCH_LABEL_GAP;
        let rects = (0..self.state.branches.len().min(MAX_BRANCHES))
            .map(|i| Rect {
                origin: Point2D::new(left, first_row_top + i as f32 * BRANCH_ROW_H),
                size: Point2D::new(inner_w, BRANCH_ROW_H),
            })
            .collect();
        (label_baseline, rects)
    }

    /// The interactive action-area sub-rects, derived from the panel
    /// rect. The button row holds 4 equal buttons normally and 3 in
    /// merge mode. Shared by [`GitPanel::paint`] + [`GitPanel::hit_test`].
    pub(super) fn action_rects(panel: Rect, merging: bool) -> ActionRects {
        let left = panel.origin.x + PAD;
        let inner_w = panel.size.x - PAD * 2.0;
        let input = Rect {
            origin: Point2D::new(left, panel.origin.y + INPUT_TOP),
            size: Point2D::new(inner_w, INPUT_H),
        };
        let n = if merging { 3 } else { 4 };
        let nf = n as f32;
        let button_w = (inner_w - (nf - 1.0) * BUTTON_GAP) / nf;
        let button_top = panel.origin.y + BUTTON_TOP;
        let buttons = (0..n)
            .map(|i| Rect {
                origin: Point2D::new(left + i as f32 * (button_w + BUTTON_GAP), button_top),
                size: Point2D::new(button_w, BUTTON_H),
            })
            .collect();
        ActionRects { input, buttons }
    }

    /// One clickable rect per displayed list row — changed files,
    /// recent commits, or conflicts depending on [`GitPanel::list_mode`].
    /// Mirrors the [`GitPanel::paint`] list walk so paint + hit-test
    /// agree.
    pub(super) fn list_row_rects(&self, panel: Rect) -> Vec<Rect> {
        let count = match self.list_mode() {
            ListMode::Merge => self.state.conflicted_files.len(),
            ListMode::Changes => self.state.changed_files.len(),
            ListMode::Commits => self.state.recent_commits.len(),
        }
        .min(MAX_COMMITS);
        let left = panel.origin.x + PAD;
        let inner_w = panel.size.x - PAD * 2.0;
        let first = panel.origin.y + COMMITS_FIRST_BASELINE;
        (0..count)
            .map(|i| Rect {
                origin: Point2D::new(left, first + i as f32 * COMMIT_ROW_H - 15.0),
                size: Point2D::new(inner_w, COMMIT_ROW_H),
            })
            .collect()
    }

    /// The working-tree status-line rect — a diff trigger when the
    /// tree has changes.
    pub(super) fn status_rect(&self, panel: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                panel.origin.x + PAD,
                panel.origin.y + STATUS_BASELINE - 14.0,
            ),
            size: Point2D::new(panel.size.x - PAD * 2.0, 18.0),
        }
    }

    /// The "merge into current" button at the right edge of a
    /// (non-current) branch row.
    pub(super) fn branch_merge_button(row: Rect) -> Rect {
        let size = BRANCH_ROW_H - 4.0;
        Rect {
            origin: Point2D::new(row.origin.x + row.size.x - size - 4.0, row.origin.y + 2.0),
            size: Point2D::new(size, size),
        }
    }

    /// Paint the panel into `rect`.
    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 10.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(rect, 10.0, self.theme.border, 1.0);

        // The inline clone wizard replaces the whole body.
        if self.state.clone_form.is_some() {
            self.paint_clone(cx, rect);
            return;
        }
        // The merge-resolution view replaces the whole body.
        if self.state.merge_resolve.is_some() {
            self.paint_resolve(cx, rect);
            return;
        }
        // Diff mode replaces the whole body with the scrollable view.
        if let Some(view) = &self.state.diff {
            self.paint_diff(cx, rect, view);
            return;
        }
        // No-repo onboarding empty state — a centred clock + heading +
        // Init/Open/Clone cards + note (no "Git" title bar), mirroring
        // the TS `git-panel-empty-state`.
        if self.is_empty_state() {
            self.paint_empty_state(cx, rect);
            return;
        }

        let left = rect.origin.x + PAD;
        let top = rect.origin.y;

        // Clean bound-repo → the TS `GitPanelReady` layout: a compact
        // header (branch button + pull/push + overflow), a commit
        // textarea, and the recent-commit history. Painted BEFORE the
        // classic "Git" title so that title never bleeds through the
        // ready header. Dirty trees + merges keep the classic body.
        if self.is_ready_state() {
            self.paint_ready(cx, rect);
            // Header popovers paint on top of the ready body.
            if self.state.branch_picker_open {
                self.paint_branch_picker(cx, rect);
            } else if self.state.overflow_open {
                self.paint_overflow(cx, rect);
            }
            return;
        }

        self.text(
            cx,
            self.t("git.panel.title"),
            left,
            top + HEADER_BASELINE,
            15.0,
            self.theme.foreground,
        );

        // Loading: the prior data is for a since-switched repository,
        // so show a neutral "Loading…" rather than stale branch /
        // commits until the new snapshot lands.
        if self.state.loading {
            self.text(
                cx,
                self.t("git.panel.loading"),
                left,
                top + HEADER_BASELINE + 24.0,
                12.0,
                self.theme.muted_foreground,
            );
            self.footer(cx, left, top + self.height() - PAD);
            return;
        }

        if !self.state.in_repo {
            self.text(
                cx,
                self.t("git.panel.notARepo"),
                left,
                top + HEADER_BASELINE + 24.0,
                12.0,
                self.theme.muted_foreground,
            );
            self.footer(cx, left, top + self.height() - PAD);
            return;
        }

        // Branch + working-tree status.
        let branch = self
            .state
            .branch
            .clone()
            .unwrap_or_else(|| self.t("git.panel.detachedHead").to_string());
        self.text(
            cx,
            &self.t("git.panel.branch").replace("{{name}}", &branch),
            left,
            top + BRANCH_BASELINE,
            13.0,
            self.theme.foreground,
        );
        let (status_text, status_color) = self.status_line();
        self.text(
            cx,
            &status_text,
            left,
            top + STATUS_BASELINE,
            12.0,
            status_color,
        );

        self.divider(cx, left, top + DIVIDER_1_Y, rect.size.x);

        // Action area. Normal mode: commit input + Commit / Refresh
        // / Pull / Push. Merge mode: a warning banner + Abort /
        // Refresh / Complete (Complete only once conflicts resolve).
        let rects = Self::action_rects(rect, self.state.merging);
        if self.state.merging {
            self.paint_merge_banner(cx, rects.input);
            self.paint_button(
                cx,
                rects.buttons[0],
                self.t("git.panel.abortMerge"),
                true,
                false,
            );
            self.paint_button(
                cx,
                rects.buttons[1],
                self.t("git.panel.refresh"),
                true,
                false,
            );
            let can_complete = self.state.conflicted_files.is_empty();
            self.paint_button(
                cx,
                rects.buttons[2],
                self.t("git.panel.complete"),
                can_complete,
                true,
            );
        } else {
            self.paint_input(cx, rects.input);
            // Commit needs a message *and* a staged file — it commits
            // exactly the staged set, so nothing staged is a no-op.
            let commit_enabled = !self.state.commit_message.trim().is_empty()
                && self.state.changed_files.iter().any(|f| f.staged);
            self.paint_button(
                cx,
                rects.buttons[0],
                self.t("git.panel.commit"),
                commit_enabled,
                true,
            );
            self.paint_button(
                cx,
                rects.buttons[1],
                self.t("git.panel.refresh"),
                true,
                false,
            );
            // Pull / Push are disabled while their op already runs.
            self.paint_button(
                cx,
                rects.buttons[2],
                self.t("git.panel.pull"),
                !self.state.pulling,
                false,
            );
            self.paint_button(
                cx,
                rects.buttons[3],
                self.t("git.panel.push"),
                !self.state.pushing,
                false,
            );
        }

        self.divider(cx, left, top + DIVIDER_2_Y, rect.size.x);

        // List section — conflicts (merge), the per-file staging
        // list (dirty tree), or recent commits (clean tree).
        let conflict_red = Color {
            r: 0.94,
            g: 0.27,
            b: 0.27,
            a: 1.0,
        };
        let label_y = top + COMMITS_LABEL_BASELINE;
        let mut y = top + COMMITS_FIRST_BASELINE;
        match self.list_mode() {
            ListMode::Merge => {
                self.text(
                    cx,
                    self.t("git.panel.conflicts"),
                    left,
                    label_y,
                    12.0,
                    self.theme.muted_foreground,
                );
                if self.state.conflicted_files.is_empty() {
                    self.text(
                        cx,
                        self.t("git.panel.noConflicts"),
                        left,
                        y,
                        12.0,
                        self.theme.muted_foreground,
                    );
                }
                for path in self.state.conflicted_files.iter().take(MAX_COMMITS) {
                    self.text(
                        cx,
                        &format!("⚠ {}", truncate(path, SUMMARY_MAX)),
                        left,
                        y,
                        12.0,
                        conflict_red,
                    );
                    y += COMMIT_ROW_H;
                }
            }
            ListMode::Changes => {
                let staged = self.state.changed_files.iter().filter(|f| f.staged).count();
                self.text(
                    cx,
                    &self
                        .t("git.panel.changes")
                        .replace("{{staged}}", &staged.to_string())
                        .replace("{{total}}", &self.state.changed_files.len().to_string()),
                    left,
                    label_y,
                    12.0,
                    self.theme.muted_foreground,
                );
                for file in self.state.changed_files.iter().take(MAX_COMMITS) {
                    // `[✓] M  path` — a click on the row toggles
                    // whether the file is staged.
                    let mark = if file.staged { "☑" } else { "☐" };
                    let color = if file.staged {
                        self.theme.foreground
                    } else {
                        self.theme.muted_foreground
                    };
                    self.text(
                        cx,
                        &format!(
                            "{mark} {}  {}",
                            file.status,
                            truncate(&file.path, SUMMARY_MAX)
                        ),
                        left,
                        y,
                        12.0,
                        color,
                    );
                    y += COMMIT_ROW_H;
                }
            }
            ListMode::Commits => {
                self.text(
                    cx,
                    self.t("git.panel.recentCommits"),
                    left,
                    label_y,
                    12.0,
                    self.theme.muted_foreground,
                );
                if self.state.recent_commits.is_empty() {
                    self.text(
                        cx,
                        self.t("git.panel.noCommits"),
                        left,
                        y,
                        12.0,
                        self.theme.muted_foreground,
                    );
                }
                for commit in self.state.recent_commits.iter().take(MAX_COMMITS) {
                    let summary = truncate(&commit.summary, SUMMARY_MAX);
                    self.text(
                        cx,
                        &format!("{}  {}", commit.short_hash, summary),
                        left,
                        y,
                        12.0,
                        self.theme.foreground,
                    );
                    y += COMMIT_ROW_H;
                }
            }
        }

        // Branches section — one row per local branch, the current
        // one marked + faintly highlighted, the rest click-to-switch.
        if !self.state.branches.is_empty() {
            let (label_baseline, branch_rects) = self.branch_layout(rect);
            self.text(
                cx,
                self.t("git.panel.branches"),
                left,
                label_baseline,
                12.0,
                self.theme.muted_foreground,
            );
            for (i, row) in branch_rects.iter().enumerate() {
                let name = &self.state.branches[i];
                let is_current = Some(name) == self.state.branch.as_ref();
                if is_current {
                    cx.backend.fill_round_rect(*row, 4.0, self.theme.muted);
                }
                let (marker, color) = if is_current {
                    ("● ", self.theme.primary)
                } else {
                    ("  ", self.theme.foreground)
                };
                let baseline = row.origin.y + BRANCH_ROW_H - 7.0;
                self.text(
                    cx,
                    &format!("{marker}{name}"),
                    left + 4.0,
                    baseline,
                    12.0,
                    color,
                );
                // Non-current branches carry a "merge into current"
                // button at the row's right edge.
                if !is_current {
                    self.paint_glyph_button(cx, Self::branch_merge_button(*row), "⤵");
                }
            }
        }

        // Remotes section — remote summary + a URL input that adds /
        // re-points `origin` (see `git_panel_remotes.rs`).
        self.paint_remotes(cx, rect);

        // Footer — always pinned a fixed inset above the panel foot.
        self.footer(cx, left, top + self.height() - PAD);
    }

    /// Paint one action button. `enabled` dims a disabled button;
    /// `primary` paints the accent (Commit) style.
    pub(super) fn paint_button(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        label: &str,
        enabled: bool,
        primary: bool,
    ) {
        let (fill, text_color) = match (enabled, primary) {
            (true, true) => (self.theme.primary, self.theme.primary_foreground),
            (true, false) => (self.theme.muted, self.theme.foreground),
            (false, _) => (self.theme.muted, self.theme.muted_foreground),
        };
        cx.backend.fill_round_rect(rect, 6.0, fill);
        if !primary {
            cx.backend
                .stroke_round_rect(rect, 6.0, self.theme.border, 1.0);
        }
        // Centre the label using the real measured width so CJK labels
        // (e.g. "保存为里程碑", ~2× a Latin glyph) centre correctly
        // instead of overflowing the right edge. A long localized label
        // can still exceed the fixed button — clip the draw to the rect
        // so it never bleeds into a neighbour.
        let label_w = cx.backend.measure_text(label, 12.0);
        let text_x = rect.origin.x + (rect.size.x - label_w).max(6.0) / 2.0;
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        cx.backend.save();
        cx.backend.clip_rect(rect);
        self.text(cx, label, text_x, baseline, 12.0, text_color);
        cx.backend.restore();
    }

    /// Paint a 1-px divider line.
    pub(super) fn divider(&self, cx: &mut PaintCx<'_>, left: f32, y: f32, panel_width: f32) {
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(left, y),
                size: Point2D::new(panel_width - PAD * 2.0, 1.0),
            },
            self.theme.border,
        );
    }
}

/// Whether `point` is inside `rect`.
pub(super) fn contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

/// Char truncation with an ellipsis.
pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}
