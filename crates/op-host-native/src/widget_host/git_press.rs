//! Git-panel press dispatch — extracted from `press.rs` to keep
//! that file under the repo's 800-line cap.
//!
//! The floating Git panel sits above the right-rail panels in paint
//! order; `apply_press` calls [`WidgetHostNative::dispatch_git_panel_press`]
//! after the rail blocks (which already skip a click that lands
//! inside the Git-panel rect) and before the canvas overlays.

use op_editor_core::{GitDiffTarget, GitPanelAction};
use op_editor_ui::widgets::{GitPanel, GitPanelHit};
use op_editor_ui::Point2D;

use super::WidgetHostNative;

impl WidgetHostNative {
    /// Dispatch a press inside the floating Git panel.
    ///
    /// Returns `true` when the click was consumed by the panel — a
    /// focus change or a queued `GitPanelState::pending_action`.
    /// Returns `false` when the click is outside the panel (the
    /// commit input is defocused as a side effect) so the caller
    /// keeps hit-testing.
    pub(in crate::widget_host) fn dispatch_git_panel_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.git_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        // Compute everything that needs an immutable `GitPanel`
        // borrow up front — the hit, and the diff scroll bound —
        // then drop the borrow before mutating `editor_state`.
        let hit = GitPanel::for_editor(&self.editor_state)
            .and_then(|p| p.hit_test(panel_rect, Point2D::new(x, y)));
        let diff_max_scroll = GitPanel::for_editor(&self.editor_state)
            .map(|p| p.diff_max_scroll())
            .unwrap_or(0);
        let panel = &mut self.editor_state.editor_ui.git_panel;
        match hit {
            Some(GitPanelHit::CommitInput) => {
                panel.commit_focused = true;
            }
            Some(GitPanelHit::Commit) => {
                if !panel.commit_message.trim().is_empty() {
                    panel.pending_action = Some(GitPanelAction::Commit);
                }
            }
            Some(GitPanelHit::Refresh) => {
                panel.pending_action = Some(GitPanelAction::Refresh);
            }
            Some(GitPanelHit::Pull) => {
                panel.pending_action = Some(GitPanelAction::Pull);
            }
            Some(GitPanelHit::AbortMerge) => {
                panel.pending_action = Some(GitPanelAction::AbortMerge);
            }
            Some(GitPanelHit::CompleteMerge) => {
                panel.pending_action = Some(GitPanelAction::CompleteMerge);
            }
            Some(GitPanelHit::SwitchBranch(index)) => {
                if let Some(name) = panel.branches.get(index).cloned() {
                    panel.pending_action = Some(GitPanelAction::SwitchBranch(name));
                }
            }
            Some(GitPanelHit::MergeBranch(index)) => {
                if let Some(name) = panel.branches.get(index).cloned() {
                    panel.pending_action = Some(GitPanelAction::MergeBranch(name));
                }
            }
            Some(GitPanelHit::ShowWorkingDiff) => {
                panel.pending_action =
                    Some(GitPanelAction::ShowDiff(GitDiffTarget::WorkingTree));
            }
            Some(GitPanelHit::ShowCommitDiff(index)) => {
                if let Some(rev) = panel.recent_commits.get(index).map(|c| c.short_hash.clone()) {
                    panel.pending_action =
                        Some(GitPanelAction::ShowDiff(GitDiffTarget::Commit(rev)));
                }
            }
            Some(GitPanelHit::ShowFileDiff(index)) => {
                if let Some(path) = panel.conflicted_files.get(index).cloned() {
                    panel.pending_action =
                        Some(GitPanelAction::ShowDiff(GitDiffTarget::Path(path)));
                }
            }
            Some(GitPanelHit::CloseDiff) => {
                // Pure UI state — no git op needed.
                panel.diff = None;
            }
            Some(GitPanelHit::DiffScrollUp) => {
                let step = GitPanel::diff_page_step();
                if let Some(diff) = &mut panel.diff {
                    diff.scroll = diff.scroll.saturating_sub(step);
                }
            }
            Some(GitPanelHit::DiffScrollDown) => {
                let step = GitPanel::diff_page_step();
                if let Some(diff) = &mut panel.diff {
                    diff.scroll = (diff.scroll + step).min(diff_max_scroll);
                }
            }
            Some(GitPanelHit::Inside) => {
                // Panel chrome — swallow the click + defocus.
                panel.commit_focused = false;
            }
            None => {
                // Outside the panel — release the commit input's
                // focus, then let the click fall through.
                if panel.commit_focused {
                    panel.commit_focused = false;
                    self.mark_dirty();
                }
                return false;
            }
        }
        self.mark_dirty();
        true
    }
}
