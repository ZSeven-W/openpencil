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
                panel.changed_files.clear();
                panel.remotes.clear();
                panel.recent_commits.clear();
                // No repo → no ready view → no header popovers.
                panel.branch_picker_open = false;
                panel.overflow_open = false;
                panel.overflow_view = op_editor_core::GitOverflowView::Menu;
                panel.close_tracked_picker();
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
        // Resolve the stored-credential kind for the remote host before the
        // `panel` borrow (it reads `git_session`'s auth store).
        let stored_auth = match snap.remote_host.as_deref() {
            None => String::new(),
            Some(host) => match self
                .git_session
                .auth_stores()
                .and_then(|(auth, _)| auth.get(host).ok().flatten())
            {
                Some(op_git::Credential::Ssh { .. }) => "ssh".to_string(),
                Some(op_git::Credential::Https { .. }) => "token".to_string(),
                None => "none".to_string(),
            },
        };
        let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
        let changed = panel.in_repo != snap.in_repo
            || panel.branch != snap.branch
            || panel.branches != snap.branches
            || panel.dirty_count != snap.dirty_count
            || panel.ahead != snap.ahead
            || panel.conflicted_count != snap.conflicted_count
            || panel.merging != snap.merging
            || panel.conflicted_files != snap.conflicted_files
            || panel.changed_files != snap.changed_files
            || panel.remotes != snap.remotes
            || panel.recent_commits != snap.recent_commits;
        // Collapse the inline detail card when the log changes — its
        // index would otherwise point at a since-shifted commit.
        let commits_changed = panel.recent_commits != snap.recent_commits;
        if commits_changed {
            panel.expanded_commit = None;
            panel.expanded_commit_diff = None;
        }
        panel.in_repo = snap.in_repo;
        panel.branch = snap.branch;
        panel.branches = snap.branches;
        panel.dirty_count = snap.dirty_count;
        panel.ahead = snap.ahead;
        panel.behind = snap.behind;
        panel.remote_host = snap.remote_host;
        panel.stored_auth = stored_auth;
        panel.conflicted_count = snap.conflicted_count;
        panel.merging = snap.merging;
        panel.conflicted_files = snap.conflicted_files;
        panel.changed_files = snap.changed_files;
        panel.remotes = snap.remotes;
        panel.recent_commits = snap.recent_commits;
        // The header popovers only exist in the ready view; if the
        // refresh left that state (no repo / merging), clear the popover
        // flags so they can't go stale and dead-end input (the press-time
        // modal guard keys off these flags). A dirty working tree still
        // shows the ready view (TS parity), so it must NOT close them —
        // otherwise a periodic status poll snaps an open branch-picker /
        // overflow menu shut.
        if !panel.header_popovers_allowed() {
            panel.branch_picker_open = false;
            panel.overflow_open = false;
            panel.overflow_view = op_editor_core::GitOverflowView::Menu;
            panel.close_tracked_picker();
        }
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
            h_scroll: 0,
            stage_path: result.stage_path,
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
            GitPanelAction::InitRepo => {
                self.init_repo_for_doc();
            }
            GitPanelAction::OpenRepo => {
                self.open_existing_repo();
            }
            GitPanelAction::CloneRepo => {
                self.open_clone_form();
            }
            GitPanelAction::PickCloneDest => {
                self.pick_clone_dest();
            }
            GitPanelAction::SubmitClone => {
                self.submit_clone();
            }
            // Refresh is handled by the shared snapshot below.
            GitPanelAction::Refresh => {}
            GitPanelAction::Pull => {
                // Run the network-bound pull on a worker thread so the
                // UI never freezes. A pull rewrites the tracked
                // document on disk and the editor is reloaded when it
                // lands (`poll_git_pull_job`) — confirm first so
                // unsaved in-memory edits are not silently lost. A
                // second Pull while one is in flight is ignored.
                if self.git_pull_job.is_none()
                    && self.collaboration_allows_git_worktree_rewrite()
                    && self.confirm_document_reload()
                {
                    if let Some(repo) = self.git_session.authed_repo() {
                        self.git_pull_job = Some(git_jobs::GitPullJob::spawn(repo));
                        // Snapshot the document so the post-pull reload
                        // can detect edits made *during* the async
                        // pull — those the confirm above did not cover.
                        let state = self.host.editor_state();
                        self.git_pull_doc_baseline = Some((
                            self.host.document_epoch(),
                            state.document_generation(),
                            state.document_revision(),
                        ));
                        self.host.editor_state_mut().editor_ui.git_panel.pulling = true;
                    }
                }
            }
            GitPanelAction::Push => {
                // Network-bound push on a worker thread; a second
                // Push while one is in flight is ignored. Push does
                // not touch the working tree, so no reload / confirm.
                if self.git_push_job.is_none() {
                    if let Some(repo) = self.git_session.authed_repo() {
                        self.git_push_job = Some(git_jobs::GitPushJob::spawn(repo));
                        self.host.editor_state_mut().editor_ui.git_panel.pushing = true;
                    }
                }
            }
            GitPanelAction::Commit => {
                let message = self
                    .host
                    .editor_state()
                    .editor_ui
                    .git_panel
                    .commit_input
                    .text()
                    .trim()
                    .to_string();
                // Commit exactly the staged index — the set the user
                // assembled with the per-file checkboxes and per-hunk
                // Stage buttons. Nothing is auto-saved or force-staged
                // here, so a partial (hunk-level) staging is never
                // silently widened into a whole-file commit. Unsaved
                // editor edits stay in the editor; the user saves +
                // stages them explicitly before they can be committed.
                if !message.is_empty() {
                    match self.git_session.commit_staged(&message) {
                        Ok(()) => {
                            let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
                            panel.commit_input.set_text("");
                            panel.defocus_commit_input(0);
                        }
                        Err(err) => {
                            self.show_git_op_error_dialog("commit", &err);
                        }
                    }
                }
            }
            GitPanelAction::CommitMilestone => {
                let message = self
                    .host
                    .editor_state()
                    .editor_ui
                    .git_panel
                    .commit_input
                    .text()
                    .trim()
                    .to_string();
                // Ready-view "Save milestone": snapshot the live design
                // as a commit in one step — write the editor's current
                // state to the tracked .op, stage that file, then commit
                // (the TS `commitMilestone` flow). stage_tracked is
                // explicitly designed to refresh the index blob after a
                // save, so a milestone captures exactly what's on screen.
                // No committer identity yet → show the signature form and
                // defer the commit (the message stays put; `save_author_identity`
                // re-fires this action). TS `authorIdentity === null` path.
                let collaboration_allows_milestone = message.is_empty()
                    || self.host.gate_collaboration_action(
                        op_editor_core::CollabGateAction::SaveShared,
                        op_editor_core::CollabEditSource::User,
                    );
                let needs_author = collaboration_allows_milestone
                    && !message.is_empty()
                    && !self
                        .git_session
                        .repo()
                        .map(|r| r.has_committer_identity())
                        .unwrap_or(true);
                if needs_author {
                    let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
                    panel.author_prompt = true;
                    panel.author_name_focused = true;
                    panel.author_email_focused = false;
                    // Hand keyboard focus to the form, off the (now hidden)
                    // commit box, so typing lands in the name/email fields.
                    panel.defocus_commit_input(0);
                    panel.commit_no_changes = false;
                } else if collaboration_allows_milestone
                    && !message.is_empty()
                    && self.finish_background_saves()
                {
                    // A milestone performs its own synchronous save before staging.
                    // Drain any UI-requested save first so an older background
                    // snapshot cannot rename over the milestone afterward.
                    match self.git_session.tracked_file().map(|p| p.to_path_buf()) {
                        Some(path) => {
                            match op_host_services::doc_io::save_to_path(
                                self.host.editor_state(),
                                &path,
                            ) {
                                Ok(()) => {
                                    self.mark_document_saved();
                                    // Stage, then guard against an empty
                                    // milestone: if the saved file matches the
                                    // last commit there is nothing to commit,
                                    // so skip rather than create an empty one.
                                    let committed = match self.git_session.stage_tracked() {
                                        Ok(()) if self.git_session.tracked_has_staged_changes() => {
                                            self.git_session.commit_staged(&message).map(|()| true)
                                        }
                                        Ok(()) => Ok(false),
                                        Err(e) => Err(e),
                                    };
                                    match committed {
                                        Ok(true) => {
                                            let panel = &mut self
                                                .host
                                                .editor_state_mut()
                                                .editor_ui
                                                .git_panel;
                                            panel.commit_input.set_text("");
                                            panel.defocus_commit_input(0);
                                            panel.commit_no_changes = false;
                                        }
                                        // Nothing changed — keep the message and
                                        // flag a "no changes" hint under the box.
                                        Ok(false) => {
                                            self.host
                                                .editor_state_mut()
                                                .editor_ui
                                                .git_panel
                                                .commit_no_changes = true;
                                        }
                                        Err(err) => self.show_git_op_error_dialog("commit", &err),
                                    }
                                }
                                // `show_error_dialog_public` takes an
                                // already-rendered `&str` detail; render
                                // through `Display` so this stays valid
                                // whichever error type `doc_io` reports.
                                Err(detail) => persistence::show_error_dialog_public(
                                    &self.host,
                                    op_host_services::doc_io::ErrorKind::Save,
                                    Some(&path),
                                    &detail.to_string(),
                                ),
                            }
                        }
                        None => persistence::show_error_dialog_public(
                            &self.host,
                            op_host_services::doc_io::ErrorKind::Save,
                            None,
                            "the open document is not tracked under this repository — \
                             save it into the repository folder first",
                        ),
                    }
                }
            }
            GitPanelAction::SwitchBranch(name) => {
                self.run_reloading_git_op("switch", |repo| repo.switch_branch(&name));
            }
            GitPanelAction::CreateBranch(name) => {
                self.run_reloading_git_op("branch", |repo| repo.create_and_switch_branch(&name));
            }
            GitPanelAction::MergeBranch(name) => {
                self.run_branch_merge(&name);
            }
            GitPanelAction::SetupSshAuth => {
                self.setup_ssh_auth();
            }
            GitPanelAction::ApplyMergeResolution => {
                self.apply_merge_resolution();
            }
            GitPanelAction::StageHunk(path, hunk_index) => {
                self.stage_diff_hunk(&path, hunk_index);
            }
            GitPanelAction::SetHttpsAuth(credential) => {
                self.store_https_credential(&credential);
            }
            GitPanelAction::SetRemote(url) => {
                let url = url.trim().to_string();
                if !url.is_empty() {
                    if let Some(repo) = self.git_session.repo() {
                        if let Err(err) = repo.set_remote("origin", &url) {
                            eprintln!("openpencil-desktop: git set-remote failed: {err}");
                        }
                    }
                    let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
                    panel.remote_input.set_text("");
                    panel.remote_focused = false;
                }
            }
            GitPanelAction::AbortMerge => {
                self.run_reloading_git_op("merge --abort", |repo| repo.abort_merge());
            }
            GitPanelAction::CompleteMerge => {
                self.run_reloading_git_op("complete merge", |repo| repo.complete_merge());
            }
            GitPanelAction::ToggleStageFile(path) => {
                // Flip the file's index state — stage it if currently
                // unstaged, unstage it otherwise. The shared snapshot
                // refresh below re-reads the real index.
                let staged = self
                    .host
                    .editor_state()
                    .editor_ui
                    .git_panel
                    .changed_files
                    .iter()
                    .find(|f| f.path == path)
                    .map(|f| f.staged)
                    .unwrap_or(false);
                if let Some(repo) = self.git_session.repo() {
                    let p = std::path::Path::new(&path);
                    let result = if staged {
                        repo.unstage(&[p])
                    } else {
                        repo.stage(&[p])
                    };
                    if let Err(err) = result {
                        eprintln!("openpencil-desktop: git stage toggle failed: {err}");
                    }
                }
            }
            GitPanelAction::ShowDiff(target) => {
                // Run `git diff` / `git show` on a worker thread — a
                // diff can be large. The panel enters diff mode at
                // once showing a placeholder; `poll_git_diff_job`
                // swaps in the real lines on a later frame.
                if let Some(repo) = self.git_session.repo().cloned() {
                    let locale = self.host.editor_state().editor_ui.locale;
                    self.host.editor_state_mut().editor_ui.git_panel.diff =
                        Some(op_editor_core::GitDiffView {
                            title: op_i18n::translate(locale, "git.panel.diffLoading").to_string(),
                            lines: vec![
                                op_i18n::translate(locale, "git.panel.diffComputing").to_string()
                            ],
                            scroll: 0,
                            h_scroll: 0,
                            stage_path: None,
                        });
                    self.git_diff_job = Some(git_jobs::GitDiffJob::spawn(repo, target, locale));
                }
            }
            GitPanelAction::RestoreCommit(rev) => {
                // Roll the tracked document back to that commit, then
                // reload so the editor reflects it (TS `restoreCommit`).
                if let Some(path) = self.git_session.tracked_file().map(|p| p.to_path_buf()) {
                    self.run_reloading_git_op("restore", move |repo| repo.restore(&path, &rev));
                }
            }
            GitPanelAction::CopyHash(rev) => {
                // Pure clipboard write — no git op, no reload.
                crate::clipboard::set_text(&rev);
            }
            GitPanelAction::LoadCommitDiff(index) => self.load_expanded_commit_diff(index),
            GitPanelAction::EnterTrackedPicker => self.enter_tracked_picker(),
            GitPanelAction::BindTrackedFile(path, open) => {
                if !open || self.collaboration_allows_git_worktree_rewrite() {
                    self.bind_tracked_file(path, open);
                }
            }
            GitPanelAction::ClearAuthor => self.clear_commit_author(),
            GitPanelAction::CloseRepo => self.close_repo(),
            GitPanelAction::EnterSshKeys => self.enter_ssh_keys(),
            GitPanelAction::ImportSshKey => self.import_ssh_key(),
            GitPanelAction::SaveAuthor => self.save_author_identity(),
            GitPanelAction::FetchRemote => {
                // `git fetch` on origin (with stored credentials) — the tail
                // refresh re-reads ahead/behind afterward.
                if let Some(repo) = self.git_session.authed_repo() {
                    if let Err(e) = repo.fetch() {
                        eprintln!("openpencil-desktop: git fetch failed: {e}");
                    }
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
        if !self.collaboration_allows_git_worktree_rewrite() || !self.confirm_document_reload() {
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

    /// Git pull is the only current background job that can rewrite the
    /// tracked document. Start/Join/Retry calls this before changing the
    /// collaboration phase: a ready result is drained while the document is
    /// still standalone; an in-flight result returns `false`, keeping the
    /// transition fail-closed until a later UI turn.
    pub(crate) fn settle_git_before_collaboration_transition(&mut self) -> bool {
        if self.git_pull_job.is_none() {
            return true;
        }
        let _ = self.poll_git_pull_job();
        self.git_pull_job.is_none()
    }

    /// A worktree rewrite is a whole-document replacement from the editor's
    /// perspective. Keep this typed seam shared by synchronous Git actions,
    /// the async pull launch, and the final reload sink.
    pub(crate) fn collaboration_allows_git_worktree_rewrite(&mut self) -> bool {
        self.host.gate_collaboration_action(
            op_editor_core::CollabGateAction::ReplaceDocument,
            op_editor_core::CollabEditSource::ExternalSync,
        )
    }
}

mod auth_error;
#[cfg(test)]
mod collab_tests;
mod patch;
mod repo_ops;

use patch::{build_hunk_patch, build_merge_resolve, merge_conflict_detail};
