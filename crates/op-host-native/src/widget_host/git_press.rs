//! Git-panel press dispatch — extracted from `press.rs` to keep
//! that file under the repo's 800-line cap.
//!
//! The floating Git panel sits above the right-rail panels in paint
//! order; `apply_press` calls [`WidgetHostNative::dispatch_git_panel_press`]
//! after the rail blocks (which already skip a click that lands
//! inside the Git-panel rect) and before the canvas overlays.

use op_editor_core::{
    CloneField, CommitDiffView, GitBranchPickerMode, GitDiffTarget, GitOverflowView,
    GitPanelAction, GitPanelState,
};
use op_editor_ui::widgets::{GitPanel, GitPanelHit};
use op_editor_ui::Point2D;

use super::helpers::rect_contains;
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
        let diff_max_h_scroll = GitPanel::for_editor(&self.editor_state)
            .map(|p| p.diff_max_h_scroll())
            .unwrap_or(0);
        // The caret bridge above the body is painted as part of the
        // popover; a click there must be swallowed (not fall through to
        // the canvas), even though `hit_test` against the body returns
        // `None` for it.
        let on_caret = self
            .git_panel_outer_rect(viewport_width, viewport_height)
            .is_some_and(|r| rect_contains(r, Point2D::new(x, y)));
        let now = self.now_ms;
        let panel = &mut self.editor_state.editor_ui.git_panel;
        match hit {
            Some(GitPanelHit::CommitInput) => {
                panel.commit_focused = true;
                panel.commit_caret_anchor_ms = now;
                panel.remote_focused = false;
                panel.https_focused = false;
                // Re-engaging the input dismisses the stale "no changes" hint.
                panel.commit_no_changes = false;
            }
            Some(GitPanelHit::RemoteInput) => {
                panel.remote_focused = true;
                panel.commit_focused = false;
                panel.https_focused = false;
                // Seed the shared caret anchor so it starts solid, then blinks.
                panel.commit_caret_anchor_ms = now;
            }
            Some(GitPanelHit::HttpsInput) => {
                panel.https_focused = true;
                panel.commit_focused = false;
                panel.remote_focused = false;
                panel.commit_caret_anchor_ms = now;
            }
            Some(GitPanelHit::SetRemote) => {
                if !panel.remote_draft.trim().is_empty() {
                    panel.pending_action =
                        Some(GitPanelAction::SetRemote(panel.remote_draft.clone()));
                }
            }
            Some(GitPanelHit::SetupSshAuth) => {
                panel.pending_action = Some(GitPanelAction::SetupSshAuth);
            }
            Some(GitPanelHit::SetHttpsAuth) => {
                if !panel.https_draft.trim().is_empty() {
                    panel.pending_action =
                        Some(GitPanelAction::SetHttpsAuth(panel.https_draft.clone()));
                }
            }
            Some(GitPanelHit::Commit) => {
                // Commit the staged set — requires a message and at
                // least one staged file.
                if !panel.commit_message.trim().is_empty()
                    && panel.changed_files.iter().any(|f| f.staged)
                {
                    panel.pending_action = Some(GitPanelAction::Commit);
                }
            }
            Some(GitPanelHit::CommitMilestone) => {
                // Ready-view Save milestone — saves the live design to
                // the tracked .op + stages + commits in one step, so it
                // only needs a non-empty message (no pre-staged file).
                if !panel.commit_message.trim().is_empty() {
                    panel.pending_action = Some(GitPanelAction::CommitMilestone);
                }
            }
            Some(GitPanelHit::AuthorNameInput) => {
                panel.author_name_focused = true;
                panel.author_email_focused = false;
                panel.commit_focused = false;
                panel.remote_focused = false;
                panel.https_focused = false;
                panel.commit_caret_anchor_ms = now;
            }
            Some(GitPanelHit::AuthorEmailInput) => {
                panel.author_email_focused = true;
                panel.author_name_focused = false;
                panel.commit_focused = false;
                panel.remote_focused = false;
                panel.https_focused = false;
                panel.commit_caret_anchor_ms = now;
            }
            Some(GitPanelHit::AuthorSave) => {
                panel.pending_action = Some(GitPanelAction::SaveAuthor);
            }
            Some(GitPanelHit::AuthorCancel) => {
                panel.author_prompt = false;
                panel.author_name_focused = false;
                panel.author_email_focused = false;
            }
            Some(GitPanelHit::EmptyInit) => {
                panel.pending_action = Some(GitPanelAction::InitRepo);
            }
            Some(GitPanelHit::EmptyOpen) => {
                panel.pending_action = Some(GitPanelAction::OpenRepo);
            }
            Some(GitPanelHit::EmptyClone) => {
                // Opens the inline clone wizard (host-side, so it can seed
                // the URL focus + caret from the live clock).
                panel.pending_action = Some(GitPanelAction::CloneRepo);
            }
            Some(GitPanelHit::CloneUrlInput) => {
                if let Some(form) = panel.clone_form.as_mut() {
                    form.focus = Some(CloneField::Url);
                    form.caret_anchor_ms = now;
                    form.error = None;
                }
            }
            Some(GitPanelHit::CloneDestInput) => {
                if let Some(form) = panel.clone_form.as_mut() {
                    form.focus = Some(CloneField::Dest);
                    form.caret_anchor_ms = now;
                    form.error = None;
                }
            }
            Some(GitPanelHit::CloneDestPick) => {
                panel.pending_action = Some(GitPanelAction::PickCloneDest);
            }
            Some(GitPanelHit::CloneSubmit) => {
                // The host validates (URL + dest non-empty) + runs the
                // clone; a no-op when the form is already cloning.
                panel.pending_action = Some(GitPanelAction::SubmitClone);
            }
            Some(GitPanelHit::CloneCancel) => {
                // Pure UI — drop the wizard back to the empty state.
                panel.clone_form = None;
            }
            Some(GitPanelHit::Refresh) => {
                panel.pending_action = Some(GitPanelAction::Refresh);
            }
            Some(GitPanelHit::Pull) => {
                panel.pending_action = Some(GitPanelAction::Pull);
            }
            Some(GitPanelHit::Push) => {
                panel.pending_action = Some(GitPanelAction::Push);
            }
            Some(GitPanelHit::BranchPicker) => {
                // Toggle the branch-picker dropdown; close the overflow
                // menu so only one ready-state popover is open at a time.
                panel.branch_picker_open = !panel.branch_picker_open;
                panel.overflow_open = false;
                panel.overflow_view = GitOverflowView::Menu;
                // Always (re)open on the branch list — a prior session's
                // create / merge sub-mode should never leak back in.
                panel.branch_picker_mode = GitBranchPickerMode::List;
                panel.branch_create_draft.clear();
                panel.branch_create_focused = false;
            }
            Some(GitPanelHit::Overflow) => {
                // Always (re)open on the top-level menu so a prior
                // session's subview never leaks back in.
                panel.overflow_open = !panel.overflow_open;
                panel.overflow_view = GitOverflowView::Menu;
                panel.branch_picker_open = false;
            }
            Some(GitPanelHit::OverflowRemoteSettings) => {
                panel.overflow_view = GitOverflowView::RemoteSettings;
            }
            Some(GitPanelHit::OverflowSshKeys) => {
                // Open the SSH-keys subview (host enumerates the stored keys).
                panel.pending_action = Some(GitPanelAction::EnterSshKeys);
            }
            Some(GitPanelHit::SshGenerateKey) => {
                panel.pending_action = Some(GitPanelAction::SetupSshAuth);
                panel.overflow_open = false;
                panel.overflow_view = GitOverflowView::Menu;
            }
            Some(GitPanelHit::SshImportKey) => {
                panel.pending_action = Some(GitPanelAction::ImportSshKey);
            }
            Some(GitPanelHit::FetchRemote) => {
                panel.pending_action = Some(GitPanelAction::FetchRemote);
            }
            Some(GitPanelHit::OverflowBack) => {
                panel.overflow_view = GitOverflowView::Menu;
            }
            Some(GitPanelHit::OverflowSwitchTracked) => {
                // Host enumerates the repo's `.op` candidates, then flips the
                // subview to the tracked-file picker.
                panel.pending_action = Some(GitPanelAction::EnterTrackedPicker);
            }
            Some(GitPanelHit::OverflowClearAuthor) => {
                panel.pending_action = Some(GitPanelAction::ClearAuthor);
                panel.overflow_open = false;
                panel.overflow_view = GitOverflowView::Menu;
            }
            Some(GitPanelHit::OverflowCloseRepo) => {
                panel.pending_action = Some(GitPanelAction::CloseRepo);
                panel.overflow_open = false;
                panel.overflow_view = GitOverflowView::Menu;
            }
            Some(GitPanelHit::TrackedPickerRow(index)) => {
                // Pure UI — single-select a candidate.
                if index < panel.candidate_files.len() {
                    panel.tracked_picker_selected = Some(index);
                }
            }
            Some(GitPanelHit::TrackedPickerBind) => {
                if let Some(path) = picker_selected_path(panel) {
                    panel.pending_action = Some(GitPanelAction::BindTrackedFile(path, false));
                }
            }
            Some(GitPanelHit::TrackedPickerBindOpen) => {
                if let Some(path) = picker_selected_path(panel) {
                    panel.pending_action = Some(GitPanelAction::BindTrackedFile(path, true));
                }
            }
            Some(GitPanelHit::TrackedPickerBack) => {
                // Close the picker subview back to the overflow menu.
                panel.overflow_view = GitOverflowView::Menu;
                panel.tracked_picker_selected = None;
            }
            Some(GitPanelHit::DismissPopover) => {
                // Click outside an open popover — close it + swallow.
                panel.branch_picker_open = false;
                panel.branch_picker_mode = GitBranchPickerMode::List;
                panel.branch_create_draft.clear();
                panel.branch_create_focused = false;
                panel.overflow_open = false;
                panel.overflow_view = GitOverflowView::Menu;
                panel.commit_focused = false;
                panel.remote_focused = false;
                panel.https_focused = false;
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
                // Close the branch-picker dropdown after a pick.
                panel.branch_picker_open = false;
            }
            Some(GitPanelHit::MergeBranch(index)) => {
                if let Some(name) = panel.branches.get(index).cloned() {
                    panel.pending_action = Some(GitPanelAction::MergeBranch(name));
                }
                panel.branch_picker_open = false;
            }
            Some(GitPanelHit::BranchCreateMode) => {
                panel.branch_picker_mode = GitBranchPickerMode::Create;
                panel.branch_create_draft.clear();
                panel.branch_create_focused = true;
                panel.commit_focused = false;
                panel.remote_focused = false;
                panel.https_focused = false;
                // Seed the shared caret anchor so it starts solid, then blinks.
                panel.commit_caret_anchor_ms = now;
            }
            Some(GitPanelHit::BranchMergeMode) => {
                panel.branch_picker_mode = GitBranchPickerMode::Merge;
                panel.branch_create_focused = false;
            }
            Some(GitPanelHit::BranchCreateInput) => {
                panel.branch_create_focused = true;
                panel.commit_focused = false;
                panel.remote_focused = false;
                panel.https_focused = false;
                panel.commit_caret_anchor_ms = now;
            }
            Some(GitPanelHit::BranchCreateSubmit) => {
                let name = panel.branch_create_draft.trim().to_string();
                if !name.is_empty() {
                    panel.pending_action = Some(GitPanelAction::CreateBranch(name));
                    panel.branch_picker_mode = GitBranchPickerMode::List;
                    panel.branch_create_draft.clear();
                    panel.branch_create_focused = false;
                    panel.branch_picker_open = false;
                }
            }
            Some(GitPanelHit::BranchPickerCancel) => {
                // Step a create / merge sub-mode back to the branch list.
                panel.branch_picker_mode = GitBranchPickerMode::List;
                panel.branch_create_draft.clear();
                panel.branch_create_focused = false;
            }
            Some(GitPanelHit::ShowWorkingDiff) => {
                panel.pending_action = Some(GitPanelAction::ShowDiff(GitDiffTarget::WorkingTree));
            }
            Some(GitPanelHit::ShowCommitDiff(index)) => {
                // Toggle the inline detail card under that row (TS
                // `HistoryMilestoneRow` expand) — clicking the open row
                // collapses it, a different row moves the card.
                let next = if panel.expanded_commit == Some(index) {
                    None
                } else if index < panel.recent_commits.len() {
                    Some(index)
                } else {
                    panel.expanded_commit
                };
                panel.expanded_commit = next;
                match next {
                    // Newly expanded → show the loading state and ask the
                    // host to compute the semantic diff (TS `computeDiff`).
                    Some(i) => {
                        panel.expanded_commit_diff = Some(CommitDiffView::Loading);
                        panel.pending_action = Some(GitPanelAction::LoadCommitDiff(i));
                    }
                    // Collapsed → drop any loaded diff.
                    None => panel.expanded_commit_diff = None,
                }
            }
            Some(GitPanelHit::RestoreCommit(index)) => {
                if let Some(rev) = panel
                    .recent_commits
                    .get(index)
                    .map(|c| c.short_hash.clone())
                {
                    panel.pending_action = Some(GitPanelAction::RestoreCommit(rev));
                }
            }
            Some(GitPanelHit::CopyCommitHash(index)) => {
                if let Some(rev) = panel
                    .recent_commits
                    .get(index)
                    .map(|c| c.short_hash.clone())
                {
                    panel.pending_action = Some(GitPanelAction::CopyHash(rev));
                }
            }
            Some(GitPanelHit::ShowFileDiff(index)) => {
                if let Some(path) = panel.conflicted_files.get(index).cloned() {
                    panel.pending_action =
                        Some(GitPanelAction::ShowDiff(GitDiffTarget::Path(path)));
                }
            }
            Some(GitPanelHit::ToggleStageFile(index)) => {
                if let Some(path) = panel.changed_files.get(index).map(|f| f.path.clone()) {
                    panel.pending_action = Some(GitPanelAction::ToggleStageFile(path));
                }
            }
            Some(GitPanelHit::ShowChangedFileDiff(index)) => {
                if let Some(path) = panel.changed_files.get(index).map(|f| f.path.clone()) {
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
            Some(GitPanelHit::DiffScrollLeft) => {
                let step = GitPanel::diff_h_step();
                if let Some(diff) = &mut panel.diff {
                    diff.h_scroll = diff.h_scroll.saturating_sub(step);
                }
            }
            Some(GitPanelHit::DiffScrollRight) => {
                let step = GitPanel::diff_h_step();
                if let Some(diff) = &mut panel.diff {
                    diff.h_scroll = (diff.h_scroll + step).min(diff_max_h_scroll);
                }
            }
            Some(GitPanelHit::StageHunk(hunk)) => {
                if let Some(path) = panel.diff.as_ref().and_then(|d| d.stage_path.clone()) {
                    panel.pending_action = Some(GitPanelAction::StageHunk(path, hunk));
                }
            }
            Some(GitPanelHit::MergeChoiceOurs(index)) => {
                if let Some(merge) = &mut panel.merge_resolve {
                    merge.set_choice(index, false);
                }
            }
            Some(GitPanelHit::MergeChoiceTheirs(index)) => {
                if let Some(merge) = &mut panel.merge_resolve {
                    merge.set_choice(index, true);
                }
            }
            Some(GitPanelHit::ApplyMergeResolution) => {
                panel.pending_action = Some(GitPanelAction::ApplyMergeResolution);
            }
            Some(GitPanelHit::CancelMergeResolution) => {
                // Pure UI state — drop the resolution view, no merge.
                panel.merge_resolve = None;
            }
            Some(GitPanelHit::Inside) => {
                // Panel chrome — swallow the click + defocus inputs.
                panel.commit_focused = false;
                panel.remote_focused = false;
                panel.https_focused = false;
            }
            None => {
                // Release the input focus first (clicking away commits
                // intent to defocus regardless).
                if panel.commit_focused || panel.remote_focused || panel.https_focused {
                    panel.commit_focused = false;
                    panel.remote_focused = false;
                    panel.https_focused = false;
                    self.mark_dirty();
                }
                // A caret-bridge click belongs to the popover — swallow
                // it. A truly-outside click falls through.
                if !on_caret {
                    return false;
                }
            }
        }
        self.mark_dirty();
        true
    }

    /// Close the floating Git panel and reset its transient sub-state —
    /// input focus, header popovers, and the diff / merge-resolve / clone
    /// views. The desktop host's poll loops abandon any in-flight `git_*`
    /// job once `open` is false (they gate on `panel.open`). Shared by the
    /// click-outside (canvas) dismiss and any other programmatic close.
    pub(in crate::widget_host) fn close_git_panel(&mut self) {
        let panel = &mut self.editor_state.editor_ui.git_panel;
        panel.open = false;
        panel.commit_focused = false;
        panel.remote_focused = false;
        panel.https_focused = false;
        panel.branch_picker_open = false;
        panel.overflow_open = false;
        panel.overflow_view = op_editor_core::GitOverflowView::Menu;
        panel.diff = None;
        panel.merge_resolve = None;
        panel.clone_form = None;
    }
}

/// The absolute path of the tracked-file picker's selected candidate, if any.
fn picker_selected_path(panel: &GitPanelState) -> Option<String> {
    panel
        .tracked_picker_selected
        .and_then(|i| panel.candidate_files.get(i))
        .map(|c| c.path.clone())
}
