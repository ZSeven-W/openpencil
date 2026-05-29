//! `GitPanel::hit_test` — maps a click to a [`GitPanelHit`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. Hit-test order mirrors paint Z-order: the
//! full-takeover modes (merge-resolution, diff) first, then the
//! normal status / action / list / branches / remotes regions.

use crate::widgets::git_panel::{contains, GitPanel, GitPanelHit, ListMode};
use crate::{Point2D, Rect};

impl GitPanel<'_> {
    /// Map a click at `point` onto a [`GitPanelHit`]. `None` when the
    /// click is outside `panel_rect` entirely.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<GitPanelHit> {
        if !contains(panel_rect, point) {
            return None;
        }
        // Merge-resolution mode — Ours/Theirs choices + Apply/Cancel.
        if self.state.merge_resolve.is_some() {
            let layout = self.resolve_layout(panel_rect);
            for (i, (ours, theirs)) in layout.rows.iter().enumerate() {
                if contains(*ours, point) {
                    return Some(GitPanelHit::MergeChoiceOurs(i));
                }
                if contains(*theirs, point) {
                    return Some(GitPanelHit::MergeChoiceTheirs(i));
                }
            }
            return Some(if contains(layout.apply, point) {
                GitPanelHit::ApplyMergeResolution
            } else if contains(layout.cancel, point) {
                GitPanelHit::CancelMergeResolution
            } else {
                GitPanelHit::Inside
            });
        }
        // Diff mode — the header's ◀ / ▶ / ▲ / ▼ / ✕ and the per-hunk
        // "Stage" buttons; the diff body itself swallows clicks.
        if self.state.diff.is_some() {
            for (hunk, button) in self.diff_hunk_buttons(panel_rect) {
                if contains(button, point) {
                    return Some(GitPanelHit::StageHunk(hunk));
                }
            }
            let [left, right, up, down, close] = Self::diff_header_buttons(panel_rect);
            return Some(if contains(left, point) {
                GitPanelHit::DiffScrollLeft
            } else if contains(right, point) {
                GitPanelHit::DiffScrollRight
            } else if contains(up, point) {
                GitPanelHit::DiffScrollUp
            } else if contains(down, point) {
                GitPanelHit::DiffScrollDown
            } else if contains(close, point) {
                GitPanelHit::CloseDiff
            } else {
                GitPanelHit::Inside
            });
        }
        // No-repo onboarding empty state — the three cards are the
        // only targets (Init is gated on a saved doc).
        if self.is_empty_state() {
            let cards = self.empty_state_rects(panel_rect);
            if contains(cards[0], point) {
                return Some(if self.state.has_saved_file {
                    GitPanelHit::EmptyInit
                } else {
                    GitPanelHit::Inside
                });
            }
            if contains(cards[1], point) {
                return Some(GitPanelHit::EmptyOpen);
            }
            if contains(cards[2], point) {
                return Some(GitPanelHit::EmptyClone);
            }
            return Some(GitPanelHit::Inside);
        }
        // While loading / outside a repo there are no action targets;
        // an in-bounds click is just swallowed.
        if self.state.loading || !self.state.in_repo {
            return Some(GitPanelHit::Inside);
        }
        // The button row. Normal: Commit / Refresh / Pull / Push.
        // Merge mode: Abort / Refresh / Complete; the commit input
        // is inert.
        let merging = self.state.merging;
        let rects = Self::action_rects(panel_rect, merging);
        if contains(rects.input, point) {
            return Some(if merging {
                GitPanelHit::Inside
            } else {
                GitPanelHit::CommitInput
            });
        }
        for (i, button) in rects.buttons.iter().enumerate() {
            if !contains(*button, point) {
                continue;
            }
            return Some(match (merging, i) {
                (true, 0) => GitPanelHit::AbortMerge,
                (true, 1) => GitPanelHit::Refresh,
                // Slot 2 = Complete — inert until every conflict is
                // resolved, mirroring the button's disabled paint
                // state so a click dispatches nothing.
                (true, _) => {
                    if self.state.conflicted_files.is_empty() {
                        GitPanelHit::CompleteMerge
                    } else {
                        GitPanelHit::Inside
                    }
                }
                (false, 0) => GitPanelHit::Commit,
                (false, 1) => GitPanelHit::Refresh,
                (false, 2) => GitPanelHit::Pull,
                (false, _) => GitPanelHit::Push,
            });
        }
        // The status line opens the whole-tree diff — but only when
        // the working tree actually has something to diff.
        if contains(self.status_rect(panel_rect), point)
            && (self.state.dirty_count > 0 || self.state.conflicted_count > 0)
        {
            return Some(GitPanelHit::ShowWorkingDiff);
        }
        // List rows. Changes mode: the left-edge checkbox toggles
        // staging, the row body opens the file's diff. Merge mode: a
        // conflicted file's diff. History: a commit's patch.
        for (i, row) in self.list_row_rects(panel_rect).iter().enumerate() {
            if contains(*row, point) {
                return Some(match self.list_mode() {
                    ListMode::Merge => GitPanelHit::ShowFileDiff(i),
                    ListMode::Changes => {
                        let checkbox = Rect {
                            origin: row.origin,
                            size: Point2D::new(24.0, row.size.y),
                        };
                        if contains(checkbox, point) {
                            GitPanelHit::ToggleStageFile(i)
                        } else {
                            GitPanelHit::ShowChangedFileDiff(i)
                        }
                    }
                    ListMode::Commits => GitPanelHit::ShowCommitDiff(i),
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
        // Remotes section — the URL input + "Set origin" button.
        let remotes = self.remotes_layout(panel_rect);
        if contains(remotes.input, point) {
            return Some(GitPanelHit::RemoteInput);
        }
        if contains(remotes.set_button, point) {
            return Some(GitPanelHit::SetRemote);
        }
        if contains(remotes.ssh_button, point) {
            return Some(GitPanelHit::SetupSshAuth);
        }
        if contains(remotes.https_input, point) {
            return Some(GitPanelHit::HttpsInput);
        }
        if contains(remotes.login_button, point) {
            return Some(GitPanelHit::SetHttpsAuth);
        }
        Some(GitPanelHit::Inside)
    }
}
