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
use crate::{Color, Point2D, Rect, TextLayout};
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
const INPUT_H: f32 = 28.0;
const BUTTON_TOP: f32 = 138.0;
const BUTTON_H: f32 = 28.0;
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
const SECTION_GAP: f32 = 16.0;

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

/// What a click landed on inside the Git panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitPanelHit {
    /// The commit-message input box — focus it.
    CommitInput,
    /// The Commit button.
    Commit,
    /// The Refresh button.
    Refresh,
    /// The Pull button.
    Pull,
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
    /// The working-tree status line — open the whole-repo diff.
    ShowWorkingDiff,
    /// A recent-commit row — open that commit's patch.
    ShowCommitDiff(usize),
    /// A conflicted-file row (merge mode) — open that file's diff.
    ShowFileDiff(usize),
    /// The diff view's ✕ — close the diff, returning to status mode.
    CloseDiff,
    /// The diff view's ▲ — page the diff up.
    DiffScrollUp,
    /// The diff view's ▼ — page the diff down.
    DiffScrollDown,
    /// Inside the panel but not on an interactive target — the
    /// click is swallowed (and the commit input defocused).
    Inside,
}

/// Sub-rectangles of the panel's interactive action area.
pub(super) struct ActionRects {
    pub(super) input: Rect,
    pub(super) commit: Rect,
    pub(super) refresh: Rect,
    pub(super) pull: Rect,
}

/// The floating Git panel, built from a [`GitPanelState`] snapshot.
pub struct GitPanel<'a> {
    pub(super) state: &'a GitPanelState,
    pub(super) theme: Theme,
}

impl<'a> GitPanel<'a> {
    /// Build the panel for the editor, or `None` when it is closed.
    pub fn for_editor(state: &'a EditorState) -> Option<GitPanel<'a>> {
        let panel = &state.editor_ui.git_panel;
        if !panel.open {
            return None;
        }
        Some(GitPanel {
            state: panel,
            theme: theme_for(&state.editor_ui),
        })
    }

    /// Panel width for the current mode — wider while a diff is open.
    pub fn panel_width(&self) -> f32 {
        if self.state.diff.is_some() {
            GIT_DIFF_PANEL_WIDTH
        } else {
            GIT_PANEL_WIDTH
        }
    }

    /// Row count of the list slot — conflicted files while a merge
    /// is in progress, recent commits otherwise. At least one (the
    /// placeholder row), capped at `MAX_COMMITS`.
    fn list_rows(&self) -> usize {
        let len = if self.state.merging {
            self.state.conflicted_files.len()
        } else {
            self.state.recent_commits.len()
        };
        len.clamp(1, MAX_COMMITS)
    }

    /// The panel's total height for the current content.
    pub fn height(&self) -> f32 {
        // Diff mode is a fixed-height scrollable view.
        if self.state.diff.is_some() {
            return DIFF_VIEW_HEIGHT;
        }
        if self.state.loading || !self.state.in_repo {
            // Header + one status line ("Loading…" / "not a repo")
            // + footer — no branch / action area / commit list.
            return HEADER_BASELINE + 24.0 + FOOTER_H + PAD;
        }
        // At least one commit row — either the commits or the single
        // "No commits yet." placeholder line — capped at `MAX_COMMITS`.
        let commit_rows = self.list_rows();
        let branch_count = self.state.branches.len().min(MAX_BRANCHES);
        // The Branches section is omitted entirely when empty.
        let branches_h = if branch_count == 0 {
            0.0
        } else {
            SECTION_GAP + BRANCH_LABEL_GAP + branch_count as f32 * BRANCH_ROW_H
        };
        COMMITS_FIRST_BASELINE
            + commit_rows as f32 * COMMIT_ROW_H
            + branches_h
            + FOOTER_H
            + PAD
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

    /// The interactive action-area sub-rects, derived from the
    /// panel rect. Shared by [`GitPanel::paint`] + [`GitPanel::hit_test`].
    pub(super) fn action_rects(panel: Rect) -> ActionRects {
        let left = panel.origin.x + PAD;
        let inner_w = panel.size.x - PAD * 2.0;
        let input = Rect {
            origin: Point2D::new(left, panel.origin.y + INPUT_TOP),
            size: Point2D::new(inner_w, INPUT_H),
        };
        let button_w = (inner_w - 2.0 * BUTTON_GAP) / 3.0;
        let button_top = panel.origin.y + BUTTON_TOP;
        let nth = |i: f32| Rect {
            origin: Point2D::new(left + i * (button_w + BUTTON_GAP), button_top),
            size: Point2D::new(button_w, BUTTON_H),
        };
        ActionRects {
            input,
            commit: nth(0.0),
            refresh: nth(1.0),
            pull: nth(2.0),
        }
    }

    /// One clickable rect per displayed list row — recent commits, or
    /// conflicted files in merge mode. Mirrors the [`GitPanel::paint`]
    /// list walk so paint + hit-test agree.
    pub(super) fn list_row_rects(&self, panel: Rect) -> Vec<Rect> {
        let count = if self.state.merging {
            self.state.conflicted_files.len()
        } else {
            self.state.recent_commits.len()
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
            origin: Point2D::new(
                row.origin.x + row.size.x - size - 4.0,
                row.origin.y + 2.0,
            ),
            size: Point2D::new(size, size),
        }
    }

    /// Map a click at `point` onto a [`GitPanelHit`]. `None` when the
    /// click is outside `panel_rect` entirely.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<GitPanelHit> {
        if !contains(panel_rect, point) {
            return None;
        }
        // Diff mode — the only interactive targets are the header's
        // ▲ / ▼ / ✕; the diff body itself swallows clicks.
        if self.state.diff.is_some() {
            let [up, down, close] = Self::diff_header_buttons(panel_rect);
            return Some(if contains(up, point) {
                GitPanelHit::DiffScrollUp
            } else if contains(down, point) {
                GitPanelHit::DiffScrollDown
            } else if contains(close, point) {
                GitPanelHit::CloseDiff
            } else {
                GitPanelHit::Inside
            });
        }
        // While loading / outside a repo there are no action targets;
        // an in-bounds click is just swallowed.
        if self.state.loading || !self.state.in_repo {
            return Some(GitPanelHit::Inside);
        }
        // During a merge the action area becomes the conflict
        // controls — slot 0 = Abort, slot 1 = Refresh, slot 2 =
        // Complete; the commit input is inert.
        let rects = Self::action_rects(panel_rect);
        let merging = self.state.merging;
        if contains(rects.input, point) {
            return Some(if merging {
                GitPanelHit::Inside
            } else {
                GitPanelHit::CommitInput
            });
        }
        if contains(rects.commit, point) {
            return Some(if merging {
                GitPanelHit::AbortMerge
            } else {
                GitPanelHit::Commit
            });
        }
        if contains(rects.refresh, point) {
            return Some(GitPanelHit::Refresh);
        }
        if contains(rects.pull, point) {
            return Some(if merging {
                // Complete is inert until every conflict is resolved
                // — mirror the button's disabled paint state so a
                // click on the disabled button dispatches nothing.
                if self.state.conflicted_files.is_empty() {
                    GitPanelHit::CompleteMerge
                } else {
                    GitPanelHit::Inside
                }
            } else {
                GitPanelHit::Pull
            });
        }
        // The status line opens the whole-tree diff — but only when
        // the working tree actually has something to diff.
        if contains(self.status_rect(panel_rect), point)
            && (self.state.dirty_count > 0 || self.state.conflicted_count > 0)
        {
            return Some(GitPanelHit::ShowWorkingDiff);
        }
        // List rows — a commit's patch (normal) or a conflicted
        // file's diff (merge mode).
        for (i, row) in self.list_row_rects(panel_rect).iter().enumerate() {
            if contains(*row, point) {
                return Some(if merging {
                    GitPanelHit::ShowFileDiff(i)
                } else {
                    GitPanelHit::ShowCommitDiff(i)
                });
            }
        }
        // Branch rows — the row body switches to a non-current
        // branch; its right-edge button merges that branch into the
        // current one. The current branch's own row is a no-op.
        let (_, branch_rects) = self.branch_layout(panel_rect);
        for (index, row) in branch_rects.iter().enumerate() {
            if contains(*row, point) {
                let is_current = self.state.branches.get(index) == self.state.branch.as_ref();
                if is_current {
                    return Some(GitPanelHit::Inside);
                }
                if contains(Self::branch_merge_button(*row), point) {
                    return Some(GitPanelHit::MergeBranch(index));
                }
                return Some(GitPanelHit::SwitchBranch(index));
            }
        }
        Some(GitPanelHit::Inside)
    }

    /// Paint the panel into `rect`.
    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 10.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(rect, 10.0, self.theme.border, 1.0);

        // Diff mode replaces the whole body with the scrollable view.
        if let Some(view) = &self.state.diff {
            self.paint_diff(cx, rect, view);
            return;
        }

        let left = rect.origin.x + PAD;
        let top = rect.origin.y;

        self.text(cx, "Git", left, top + HEADER_BASELINE, 15.0, self.theme.foreground);

        // Loading: the prior data is for a since-switched repository,
        // so show a neutral "Loading…" rather than stale branch /
        // commits until the new snapshot lands.
        if self.state.loading {
            self.text(
                cx,
                "Loading repository…",
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
                "Not a git repository.",
                left,
                top + HEADER_BASELINE + 24.0,
                12.0,
                self.theme.muted_foreground,
            );
            self.footer(cx, left, top + self.height() - PAD);
            return;
        }

        // Branch + working-tree status.
        let branch = self.state.branch.as_deref().unwrap_or("(detached HEAD)");
        self.text(
            cx,
            &format!("Branch: {branch}"),
            left,
            top + BRANCH_BASELINE,
            13.0,
            self.theme.foreground,
        );
        let (status_text, status_color) = self.status_line();
        self.text(cx, &status_text, left, top + STATUS_BASELINE, 12.0, status_color);

        self.divider(cx, left, top + DIVIDER_1_Y, rect.size.x);

        // Action area. Normal mode: commit input + Commit / Refresh
        // / Pull. Merge mode: a warning banner + Abort / Refresh /
        // Complete (Complete only once conflicts are resolved).
        let rects = Self::action_rects(rect);
        if self.state.merging {
            self.paint_merge_banner(cx, rects.input);
            self.paint_button(cx, rects.commit, "Abort Merge", true, false);
            self.paint_button(cx, rects.refresh, "Refresh", true, false);
            let can_complete = self.state.conflicted_files.is_empty();
            self.paint_button(cx, rects.pull, "Complete", can_complete, true);
        } else {
            self.paint_input(cx, rects.input);
            let commit_enabled = !self.state.commit_message.trim().is_empty();
            self.paint_button(cx, rects.commit, "Commit", commit_enabled, true);
            self.paint_button(cx, rects.refresh, "Refresh", true, false);
            // The Pull button is disabled while a pull already runs.
            self.paint_button(cx, rects.pull, "Pull", !self.state.pulling, false);
        }

        self.divider(cx, left, top + DIVIDER_2_Y, rect.size.x);

        // List section — conflicted files during a merge, recent
        // commits otherwise.
        let conflict_red = Color { r: 0.94, g: 0.27, b: 0.27, a: 1.0 };
        let mut y = top + COMMITS_FIRST_BASELINE;
        if self.state.merging {
            self.text(
                cx,
                "Conflicts",
                left,
                top + COMMITS_LABEL_BASELINE,
                12.0,
                self.theme.muted_foreground,
            );
            if self.state.conflicted_files.is_empty() {
                self.text(
                    cx,
                    "No conflicts — ready to complete.",
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
        } else {
            self.text(
                cx,
                "Recent commits",
                left,
                top + COMMITS_LABEL_BASELINE,
                12.0,
                self.theme.muted_foreground,
            );
            if self.state.recent_commits.is_empty() {
                self.text(cx, "No commits yet.", left, y, 12.0, self.theme.muted_foreground);
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

        // Branches section — one row per local branch, the current
        // one marked + faintly highlighted, the rest click-to-switch.
        if !self.state.branches.is_empty() {
            let (label_baseline, branch_rects) = self.branch_layout(rect);
            self.text(
                cx,
                "Branches",
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

        // Footer — always pinned a fixed inset above the panel foot.
        self.footer(cx, left, top + self.height() - PAD);
    }

    /// The working-tree status line text + colour.
    fn status_line(&self) -> (String, Color) {
        if self.state.pulling {
            return (
                "Pulling…".to_string(),
                Color { r: 0.23, g: 0.51, b: 0.96, a: 1.0 },
            );
        }
        if self.state.conflicted_count > 0 {
            (
                format!(
                    "{} changed · {} conflicted",
                    self.state.dirty_count, self.state.conflicted_count
                ),
                Color { r: 0.94, g: 0.27, b: 0.27, a: 1.0 },
            )
        } else if self.state.dirty_count > 0 {
            (
                format!("{} changed", self.state.dirty_count),
                Color { r: 0.96, g: 0.62, b: 0.04, a: 1.0 },
            )
        } else {
            (
                "Working tree clean".to_string(),
                Color { r: 0.22, g: 0.78, b: 0.42, a: 1.0 },
            )
        }
    }

    /// Paint the merge-in-progress banner into the action-input slot
    /// (the commit input is replaced by it during a merge).
    fn paint_merge_banner(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let amber = Color { r: 0.96, g: 0.62, b: 0.04, a: 1.0 };
        cx.backend.fill_round_rect(rect, 6.0, self.theme.muted);
        cx.backend.stroke_round_rect(rect, 6.0, amber, 1.0);
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        self.text(cx, "⚠ Merge in progress", rect.origin.x + 8.0, baseline, 12.0, amber);
    }

    /// Paint the commit-message input box.
    fn paint_input(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 6.0, self.theme.muted);
        let border = if self.state.commit_focused {
            self.theme.primary
        } else {
            self.theme.border
        };
        cx.backend.stroke_round_rect(rect, 6.0, border, 1.0);

        let text_x = rect.origin.x + 8.0;
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        let msg = &self.state.commit_message;
        if msg.is_empty() && !self.state.commit_focused {
            self.text(cx, "Commit message…", text_x, baseline, 12.0, self.theme.muted_foreground);
        } else {
            let shown = if self.state.commit_focused {
                format!("{msg}|")
            } else {
                msg.clone()
            };
            self.text(cx, &shown, text_x, baseline, 12.0, self.theme.foreground);
        }
    }

    /// Paint one action button. `enabled` dims a disabled button;
    /// `primary` paints the accent (Commit) style.
    fn paint_button(&self, cx: &mut PaintCx<'_>, rect: Rect, label: &str, enabled: bool, primary: bool) {
        let (fill, text_color) = match (enabled, primary) {
            (true, true) => (self.theme.primary, self.theme.primary_foreground),
            (true, false) => (self.theme.muted, self.theme.foreground),
            (false, _) => (self.theme.muted, self.theme.muted_foreground),
        };
        cx.backend.fill_round_rect(rect, 6.0, fill);
        if !primary {
            cx.backend.stroke_round_rect(rect, 6.0, self.theme.border, 1.0);
        }
        // Roughly centre the label (no per-glyph measurement here).
        let label_w = label.chars().count() as f32 * 6.5;
        let text_x = rect.origin.x + (rect.size.x - label_w).max(6.0) / 2.0;
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        self.text(cx, label, text_x, baseline, 12.0, text_color);
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

    /// Paint the menu hint at the panel foot.
    fn footer(&self, cx: &mut PaintCx<'_>, left: f32, y: f32) {
        self.text(
            cx,
            "View ▸ Git Panel to close",
            left,
            y,
            10.0,
            self.theme.muted_foreground,
        );
    }

    /// Draw one line of text.
    pub(super) fn text(
        &self,
        cx: &mut PaintCx<'_>,
        s: &str,
        x: f32,
        baseline_y: f32,
        size: f32,
        color: Color,
    ) {
        let layout = TextLayout::single_run(
            s,
            "system-ui",
            size,
            to_jian(color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
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

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
