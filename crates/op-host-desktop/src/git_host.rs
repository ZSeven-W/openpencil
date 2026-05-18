//! In-app Git host logic on `DesktopApp` — Git-panel snapshot
//! refresh, background-job draining, and panel action dispatch.
//!
//! Split out of `main.rs` to keep that file under the repo's
//! 800-line-per-file cap; `main.rs` keeps the `DesktopApp` struct,
//! its general `impl`, and `fn main`.

use crate::{git_jobs, persistence, DesktopApp};

impl DesktopApp {
    /// Request a refresh of the Git-panel snapshot.
    ///
    /// The git queries (`status` / `log` / branch) run on a worker
    /// thread — `poll_git_status_job` applies the result on a later
    /// frame — so even a large repository never freezes the UI. When
    /// no repository is bound there is nothing to query, so the panel
    /// is cleared inline. The panel's `open` / input / `pulling`
    /// fields are left untouched — the caller owns those.
    pub(crate) fn refresh_git_panel(&mut self) {
        match self.git_session.repo().cloned() {
            Some(repo) => {
                // A fresh request supersedes any in-flight query.
                self.git_status_job = Some(git_jobs::GitStatusJob::spawn(repo));
            }
            None => {
                // Discard any in-flight status query first — its
                // result is for the previous repository and must not
                // land back onto the now-unbound panel.
                self.git_status_job = None;
                let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
                panel.in_repo = false;
                panel.branch = None;
                panel.branches.clear();
                panel.dirty_count = 0;
                panel.conflicted_count = 0;
                panel.merging = false;
                panel.conflicted_files.clear();
                panel.recent_commits.clear();
                // Cleared synchronously — there is nothing to wait for.
                panel.loading = false;
            }
        }
    }

    /// Drain a finished background Git status query into the panel.
    /// Returns `true` when the snapshot actually changed, so a
    /// periodic refresh only repaints on a real change.
    pub(crate) fn poll_git_status_job(&mut self) -> bool {
        let Some(job) = self.git_status_job.as_mut() else {
            return false;
        };
        let Some(snap) = job.poll() else {
            return false;
        };
        self.git_status_job = None;
        // Guard against a stale job: if the document is no longer in
        // a repository, the snapshot is for a since-unbound repo —
        // drop it rather than repopulating the panel.
        if !self.git_session.is_bound() {
            return false;
        }
        let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
        let changed = panel.in_repo != snap.in_repo
            || panel.branch != snap.branch
            || panel.branches != snap.branches
            || panel.dirty_count != snap.dirty_count
            || panel.conflicted_count != snap.conflicted_count
            || panel.merging != snap.merging
            || panel.conflicted_files != snap.conflicted_files
            || panel.recent_commits != snap.recent_commits;
        panel.in_repo = snap.in_repo;
        panel.branch = snap.branch;
        panel.branches = snap.branches;
        panel.dirty_count = snap.dirty_count;
        panel.conflicted_count = snap.conflicted_count;
        panel.merging = snap.merging;
        panel.conflicted_files = snap.conflicted_files;
        panel.recent_commits = snap.recent_commits;
        // The fresh snapshot has landed — leave the loading state.
        let was_loading = panel.loading;
        panel.loading = false;
        changed || was_loading
    }

    /// Drain a finished background Git diff into the panel's diff
    /// view. Returns `true` when the view actually changed, so a
    /// periodic refresh only repaints on a real change.
    pub(crate) fn poll_git_diff_job(&mut self) -> bool {
        let Some(job) = self.git_diff_job.as_mut() else {
            return false;
        };
        let Some(result) = job.poll() else {
            return false;
        };
        self.git_diff_job = None;
        let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
        // The user may have closed the panel / diff view (or it was
        // cleared by a repo switch) while the worker ran — drop a
        // stale result rather than re-opening the view.
        if !panel.open || panel.diff.is_none() {
            return false;
        }
        panel.diff = Some(op_editor_core::GitDiffView {
            title: result.title,
            lines: result.lines,
            scroll: 0,
        });
        true
    }

    /// Run a queued Git-panel action — `pending_action`, set by a
    /// panel click or by Enter in the commit input — then refresh
    /// the panel snapshot. A no-op when nothing is queued.
    pub(crate) fn drain_git_action(&mut self) {
        use op_editor_core::GitPanelAction;
        let Some(action) = self
            .host
            .editor_state_mut()
            .editor_ui
            .git_panel
            .pending_action
            .take()
        else {
            return;
        };
        match action {
            // Refresh is handled by the shared snapshot below.
            GitPanelAction::Refresh => {}
            GitPanelAction::Pull => {
                // Run the network-bound pull on a worker thread so the
                // UI never freezes. A pull rewrites the tracked
                // document on disk and the editor is reloaded when it
                // lands (`poll_git_pull_job`) — confirm first so
                // unsaved in-memory edits are not silently lost. A
                // second Pull while one is in flight is ignored.
                if self.git_pull_job.is_none() && self.confirm_document_reload() {
                    if let Some(repo) = self.git_session.repo().cloned() {
                        self.git_pull_job = Some(git_jobs::GitPullJob::spawn(repo));
                        // Snapshot the document so the post-pull reload
                        // can detect edits made *during* the async
                        // pull — those the confirm above did not cover.
                        self.git_pull_doc_baseline = Some(persistence::document_fingerprint(
                            self.host.editor_state(),
                        ));
                        self.host.editor_state_mut().editor_ui.git_panel.pulling = true;
                    }
                }
            }
            GitPanelAction::Commit => {
                let message = self
                    .host
                    .editor_state()
                    .editor_ui
                    .git_panel
                    .commit_message
                    .trim()
                    .to_string();
                // A commit must capture what the editor shows, not the
                // stale last-saved file — persist unsaved edits first,
                // and skip the commit entirely if that write fails.
                if !message.is_empty() && self.save_tracked_document() {
                    match self.git_session.commit_tracked(&message) {
                        Ok(()) => {
                            let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
                            panel.commit_message.clear();
                            panel.commit_focused = false;
                        }
                        Err(err) => {
                            eprintln!("openpencil-desktop: git commit failed: {err}");
                        }
                    }
                }
            }
            GitPanelAction::SwitchBranch(name) => {
                self.run_reloading_git_op("switch", |repo| repo.switch_branch(&name));
            }
            GitPanelAction::MergeBranch(name) => {
                self.run_branch_merge(&name);
            }
            GitPanelAction::AbortMerge => {
                self.run_reloading_git_op("merge --abort", |repo| repo.abort_merge());
            }
            GitPanelAction::CompleteMerge => {
                self.run_reloading_git_op("complete merge", |repo| repo.complete_merge());
            }
            GitPanelAction::ShowDiff(target) => {
                // Run `git diff` / `git show` on a worker thread — a
                // diff can be large. The panel enters diff mode at
                // once showing a placeholder; `poll_git_diff_job`
                // swaps in the real lines on a later frame.
                if let Some(repo) = self.git_session.repo().cloned() {
                    self.host.editor_state_mut().editor_ui.git_panel.diff =
                        Some(op_editor_core::GitDiffView {
                            title: "loading…".to_string(),
                            lines: vec!["Computing diff…".to_string()],
                            scroll: 0,
                        });
                    self.git_diff_job = Some(git_jobs::GitDiffJob::spawn(repo, target));
                }
            }
        }
        // Every action ends with a fresh snapshot + a repaint.
        self.refresh_git_panel();
        self.host.mark_editor_state_dirty();
        self.request_redraw(true);
    }

    /// Run a working-tree-rewriting git op (branch switch, merge
    /// abort / complete) then reload the document from disk so the
    /// editor reflects the new state. Confirms first — the reload
    /// would otherwise silently discard unsaved in-memory edits.
    fn run_reloading_git_op(
        &mut self,
        label: &str,
        op: impl FnOnce(&op_git::GitRepo) -> Result<(), op_git::GitError>,
    ) {
        if !self.confirm_document_reload() {
            return;
        }
        let ok = match self.git_session.repo() {
            Some(repo) => match op(repo) {
                Ok(()) => true,
                Err(err) => {
                    eprintln!("openpencil-desktop: git {label} failed: {err}");
                    false
                }
            },
            None => false,
        };
        if ok {
            self.reload_tracked_document();
        }
    }

    /// Persist the editor's in-memory document to its tracked path
    /// so a following git op (a commit) acts on current content
    /// rather than a stale last-saved file.
    ///
    /// Returns `true` when the tree is ready to commit — no unsaved
    /// edits, or the write succeeded — and `false` when there is no
    /// path to save to or the write failed (the caller then skips
    /// the git op).
    fn save_tracked_document(&mut self) -> bool {
        // Flush any pending inline-input edit into the document so
        // the dirty check + save capture it.
        self.host.commit_variable_row_focus_if_any_pub();
        if !self.document_is_dirty() {
            return true;
        }
        let Some(path) = self.current_path.clone() else {
            return false;
        };
        match persistence::save_to_path(self.host.editor_state(), &path) {
            Ok(()) => {
                self.mark_document_saved();
                true
            }
            Err(err) => {
                eprintln!("openpencil-desktop: save before commit failed: {err}");
                false
            }
        }
    }

    /// Reload the tracked document from disk after a git op rewrote
    /// it, marking the in-memory state as saved.
    pub(crate) fn reload_tracked_document(&mut self) {
        if let Some(path) = self.current_path.clone() {
            if persistence::open_path(
                &mut self.host,
                path,
                &mut self.current_path,
                self.window.as_ref(),
            ) {
                self.mark_document_saved();
            }
        }
    }

    /// Merge `other` into the current branch through the isolated
    /// worktree orchestrator (`op_git::merge_branch_isolated`).
    ///
    /// A clean merge advances the live branch — the document is
    /// reloaded from disk. A conflicting merge is quarantined to the
    /// throwaway worktree: the live tree is left pristine and the
    /// conflicted files are reported in a dialog (structured in-app
    /// conflict resolution is a later increment).
    fn run_branch_merge(&mut self, other: &str) {
        // A clean merge reloads the document — confirm first so the
        // reload cannot silently discard unsaved in-memory edits.
        if !self.confirm_document_reload() {
            return;
        }
        let Some(result) = self.git_session.merge_branch(other) else {
            return;
        };
        match result {
            Ok(report) if report.outcome == op_git::MergeOutcome::Conflict => {
                self.show_merge_conflict_dialog(other, &report.conflicts);
            }
            Ok(_) => {
                // Clean merge / fast-forward / already-up-to-date —
                // the live branch advanced on disk; reload it.
                self.reload_tracked_document();
            }
            Err(err) => {
                eprintln!("openpencil-desktop: git merge of {other} failed: {err}");
            }
        }
        self.refresh_git_panel();
        self.host.mark_editor_state_dirty();
        self.request_redraw(true);
    }

    /// Report a quarantined merge conflict — the live tree is
    /// untouched, so this is informational, not an error.
    fn show_merge_conflict_dialog(&self, other: &str, conflicts: &op_git::ConflictBag) {
        use op_editor_core::Locale;
        let zh = matches!(
            self.host.editor_state().editor_ui.locale,
            Locale::ZhCn | Locale::ZhTw
        );
        let files: Vec<&str> = conflicts.files.iter().map(|f| f.path.as_str()).collect();
        let (title, body) = if zh {
            (
                "合并存在冲突".to_string(),
                format!(
                    "分支 {other} 与当前分支有 {} 个文件冲突,合并未应用,\
                     当前工作区保持原样。请先在该分支上解决冲突:\n\n{}",
                    files.len(),
                    files.join("\n")
                ),
            )
        } else {
            (
                "Merge has conflicts".to_string(),
                format!(
                    "Merging {other} conflicts in {} file(s); the merge was \
                     not applied and your working tree is unchanged. Resolve \
                     the conflicts on that branch first:\n\n{}",
                    files.len(),
                    files.join("\n")
                ),
            )
        };
        rfd::MessageDialog::new()
            .set_title(&title)
            .set_description(&body)
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}
