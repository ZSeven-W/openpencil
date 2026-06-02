//! Host glue for the commit-detail card's semantic diff — kept out of the
//! already-large `git_host.rs`. Reads the expanded commit + its tracked file,
//! computes the node diff against the parent (`commit_diff_semantic`), and
//! lands the result on the panel.

use op_editor_core::CommitDiffView;

use crate::DesktopApp;

impl DesktopApp {
    /// Compute `recent_commits[index]`'s semantic diff against its parent
    /// (TS `computeDiff`) and store it in the panel for the inline card.
    pub(crate) fn load_expanded_commit_diff(&mut self, index: usize) {
        let commit = self
            .host
            .editor_state()
            .editor_ui
            .git_panel
            .recent_commits
            .get(index)
            .cloned();
        let relpath = self.git_session.tracked_relpath();
        let view = match (self.git_session.repo(), relpath, commit) {
            (Some(repo), Some(relpath), Some(commit)) => {
                crate::commit_diff_semantic::load_commit_diff(repo, &relpath, &commit)
            }
            _ => CommitDiffView::Error("no tracked file in this repository".to_string()),
        };
        // Only land the result if that row is still expanded — a fast
        // collapse / re-expand could have moved on.
        let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
        if panel.expanded_commit == Some(index) {
            panel.expanded_commit_diff = Some(view);
        }
    }
}
