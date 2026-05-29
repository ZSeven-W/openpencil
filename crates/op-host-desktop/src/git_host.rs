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
            || panel.changed_files != snap.changed_files
            || panel.remotes != snap.remotes
            || panel.recent_commits != snap.recent_commits;
        panel.in_repo = snap.in_repo;
        panel.branch = snap.branch;
        panel.branches = snap.branches;
        panel.dirty_count = snap.dirty_count;
        panel.conflicted_count = snap.conflicted_count;
        panel.merging = snap.merging;
        panel.conflicted_files = snap.conflicted_files;
        panel.changed_files = snap.changed_files;
        panel.remotes = snap.remotes;
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
                self.clone_repo_prompt();
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
                if self.git_pull_job.is_none() && self.confirm_document_reload() {
                    if let Some(repo) = self.git_session.authed_repo() {
                        self.git_pull_job = Some(git_jobs::GitPullJob::spawn(repo));
                        // Snapshot the document so the post-pull reload
                        // can detect edits made *during* the async
                        // pull — those the confirm above did not cover.
                        self.git_pull_doc_baseline =
                            Some(persistence::document_fingerprint(self.host.editor_state()));
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
                    .commit_message
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
                            panel.commit_message.clear();
                            panel.commit_focused = false;
                        }
                        Err(err) => {
                            self.show_git_op_error_dialog("commit", &err);
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
                    panel.remote_draft.clear();
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

    /// Stage one hunk of the open file diff — `git apply --cached`
    /// of a patch built from the diff view's lines (the file header
    /// plus the chosen hunk). The file's diff is then re-fetched so
    /// the staged hunk drops out of the (unstaged) view.
    fn stage_diff_hunk(&mut self, path: &str, hunk_index: usize) {
        let lines = match &self.host.editor_state().editor_ui.git_panel.diff {
            Some(view) => view.lines.clone(),
            None => return,
        };
        let Some(patch) = build_hunk_patch(&lines, hunk_index) else {
            return;
        };
        match self.git_session.repo() {
            Some(repo) => {
                if let Err(err) = repo.apply_cached(&patch) {
                    eprintln!("openpencil-desktop: stage hunk failed: {err}");
                    return;
                }
            }
            None => return,
        }
        // Re-fetch the file diff so the now-staged hunk drops out.
        if let Some(repo) = self.git_session.repo().cloned() {
            let locale = self.host.editor_state().editor_ui.locale;
            self.git_diff_job = Some(git_jobs::GitDiffJob::spawn(
                repo,
                op_editor_core::GitDiffTarget::Path(path.to_string()),
                locale,
            ));
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
    /// reloaded from disk. A conflict whose files are all structured
    /// `.op` documents opens the interactive resolution view; any
    /// other conflict is reported in a dialog.
    fn run_branch_merge(&mut self, other: &str) {
        // A clean merge reloads the document — confirm first so the
        // reload cannot silently discard unsaved in-memory edits.
        if !self.confirm_document_reload() {
            return;
        }
        let Some(result) = self.git_session.merge_branch(other) else {
            return;
        };
        let locale = self.host.editor_state().editor_ui.locale;
        match result {
            Ok(report) if report.outcome == op_git::MergeOutcome::Conflict => {
                match build_merge_resolve(other, &report.conflicts, locale) {
                    // Every conflicted file is a structured `.op` —
                    // open the per-node resolution view.
                    Some(state) => {
                        self.host
                            .editor_state_mut()
                            .editor_ui
                            .git_panel
                            .merge_resolve = Some(state);
                    }
                    None => self.show_merge_conflict_dialog(other, &report.conflicts),
                }
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

    /// Re-run the branch merge with the per-node ours/theirs choices
    /// the user picked in the resolution view (drained from
    /// `git_panel.merge_resolve`). A clean re-run completes the merge
    /// and reloads the document.
    fn apply_merge_resolution(&mut self) {
        let Some(state) = self
            .host
            .editor_state_mut()
            .editor_ui
            .git_panel
            .merge_resolve
            .take()
        else {
            return;
        };
        // The completed merge reloads the document — confirm first.
        if !self.confirm_document_reload() {
            // Declined — restore the resolution view untouched.
            self.host
                .editor_state_mut()
                .editor_ui
                .git_panel
                .merge_resolve = Some(state);
            return;
        }
        match self.git_session.merge_branch_resolved(&state) {
            Some(Ok(report)) if report.outcome == op_git::MergeOutcome::Conflict => {
                // A conflict still slipped through (e.g. the branch
                // tips moved) — fall back to the dialog.
                self.show_merge_conflict_dialog(&state.branch, &report.conflicts);
            }
            Some(Ok(_)) => self.reload_tracked_document(),
            Some(Err(err)) => {
                eprintln!("openpencil-desktop: applying merge resolution failed: {err}");
            }
            None => {}
        }
        self.refresh_git_panel();
        self.host.mark_editor_state_dirty();
        self.request_redraw(true);
    }

    /// Set up SSH authentication for the `origin` remote: generate
    /// (or reuse) an SSH key named after the host, bind it as that
    /// host's stored credential, and show the public key so the user
    /// can add it to their git host. From then on pull / push run
    /// through `authed_repo` with that key.
    fn setup_ssh_auth(&mut self) {
        use op_git::Credential;
        let outcome: Result<(String, String), String> = (|| {
            let (auth, ssh) = self
                .git_session
                .auth_stores()
                .ok_or_else(|| "credential stores are unavailable".to_string())?;
            let repo = self
                .git_session
                .repo()
                .ok_or_else(|| "no repository is bound".to_string())?;
            let host = repo
                .origin_host()
                .ok_or_else(|| "no `origin` remote — set one first".to_string())?;
            let key_name = format!("op-{}", host.replace(['/', '\\', ':'], "-"));
            // Reuse an existing key of that name, else generate one.
            let key = match ssh.load(&key_name) {
                Ok(key) => key,
                Err(_) => ssh
                    .generate(&key_name, &format!("openpencil@{host}"))
                    .map_err(|e| e.to_string())?,
            };
            auth.set(&host, Credential::Ssh { key_name })
                .map_err(|e| e.to_string())?;
            Ok((host, key.public_key))
        })();
        match outcome {
            Ok((host, public_key)) => self.show_ssh_setup_dialog(&host, &public_key),
            Err(err) => eprintln!("openpencil-desktop: ssh auth setup failed: {err}"),
        }
    }

    /// Store an HTTPS credential for the `origin` host. The input is
    /// the Remotes section's `username:token` text; pull / push then
    /// authenticate with it via `authed_repo`. The draft is cleared
    /// only on success, so a failure leaves it for a retry.
    fn store_https_credential(&mut self, credential: &str) {
        use op_git::Credential;
        let result: Result<(), String> = (|| {
            let (username, token) = credential
                .split_once(':')
                .ok_or_else(|| "credential must be `username:token`".to_string())?;
            let username = username.trim().to_string();
            let token = token.trim().to_string();
            if username.is_empty() || token.is_empty() {
                return Err("both a username and a token are required".to_string());
            }
            let (auth, _ssh) = self
                .git_session
                .auth_stores()
                .ok_or_else(|| "credential store is unavailable".to_string())?;
            let repo = self
                .git_session
                .repo()
                .ok_or_else(|| "no repository is bound".to_string())?;
            // An HTTPS credential only belongs on an HTTPS remote —
            // storing one for an SSH `origin` would shadow (and
            // break) its SSH credential, since the store is
            // host-keyed.
            let url = repo
                .remote_url("origin")
                .ok()
                .flatten()
                .ok_or_else(|| "no `origin` remote — set one first".to_string())?;
            if !url.starts_with("https://") && !url.starts_with("http://") {
                return Err(
                    "the origin remote is not HTTPS — use the SSH button instead".to_string(),
                );
            }
            let host = repo
                .origin_host()
                .ok_or_else(|| "the origin URL has no host".to_string())?;
            auth.set(&host, Credential::Https { username, token })
                .map_err(|e| e.to_string())
        })();
        match result {
            Ok(()) => {
                let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
                panel.https_draft.clear();
                panel.https_focused = false;
            }
            Err(err) => {
                eprintln!("openpencil-desktop: store HTTPS credential failed: {err}");
            }
        }
    }

    /// Empty-state "Init" card — create a local repo at the saved
    /// document's directory, then re-discover so the doc is tracked.
    fn init_repo_for_doc(&mut self) {
        let Some(dir) = self
            .current_path
            .clone()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        else {
            return;
        };
        match op_git::GitRepo::init(&dir) {
            Ok(_) => self.rebind_git_session_for_current_path(),
            Err(err) => self.show_git_empty_error(&format!("git init: {err}")),
        }
    }

    /// Empty-state "Open" card — pick an existing repo folder and bind
    /// it. The open doc is tracked only when it lives inside the repo.
    fn open_existing_repo(&mut self) {
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        match op_git::GitRepo::discover(&folder) {
            Ok(Some(repo)) => {
                self.git_session
                    .bind_repo(repo, self.current_path.as_deref());
                self.host.editor_state_mut().editor_ui.git_panel.loading = true;
                self.host.mark_editor_state_dirty();
                self.refresh_git_panel();
            }
            Ok(None) => {
                let locale = self.host.editor_state().editor_ui.locale;
                self.show_git_empty_error(op_i18n::translate(locale, "git.empty.openNotARepo"));
            }
            Err(err) => self.show_git_empty_error(&format!("git open: {err}")),
        }
    }

    /// Empty-state "Clone" card — cloning needs a remote URL, which
    /// requires the in-panel clone form (a follow-up); for now point
    /// the user at it.
    fn clone_repo_prompt(&mut self) {
        let locale = self.host.editor_state().editor_ui.locale;
        rfd::MessageDialog::new()
            .set_title(op_i18n::translate(locale, "git.empty.cloneCard"))
            .set_description(op_i18n::translate(locale, "git.empty.cloneComingSoon"))
            .set_level(rfd::MessageLevel::Info)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }

    /// Info/error dialog for the empty-state cards.
    fn show_git_empty_error(&self, msg: &str) {
        rfd::MessageDialog::new()
            .set_title("Git")
            .set_description(msg)
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }

    /// Show the generated SSH public key so the user can register it
    /// with their git host.
    fn show_ssh_setup_dialog(&self, host: &str, public_key: &str) {
        let locale = self.host.editor_state().editor_ui.locale;
        let body = op_i18n::translate(locale, "git.ssh.readyBody").replace("{{host}}", host);
        rfd::MessageDialog::new()
            .set_title(op_i18n::translate(locale, "git.ssh.readyTitle"))
            .set_description(format!("{body}\n\n{public_key}"))
            .set_level(rfd::MessageLevel::Info)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }

    /// Report a quarantined merge conflict — the live tree is
    /// untouched, so this is informational, not an error. For `.op`
    /// files the three index stages are run through the structured
    /// node-level merge so the report names the conflicting
    /// PenNodes, not just the file.
    fn show_merge_conflict_dialog(&self, other: &str, conflicts: &op_git::ConflictBag) {
        let locale = self.host.editor_state().editor_ui.locale;
        let detail = merge_conflict_detail(conflicts, locale);
        let body = op_i18n::translate(locale, "git.merge.conflictBody")
            .replace("{{branch}}", other)
            .replace("{{count}}", &conflicts.files.len().to_string());
        rfd::MessageDialog::new()
            .set_title(op_i18n::translate(locale, "git.merge.conflictTitle"))
            .set_description(format!("{body}\n\n{detail}"))
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

/// Build the interactive [`MergeResolveState`] from a conflict bag
/// — `None` when any conflicted file is not a structured `.op`
/// document (those cannot be resolved per node, so the caller falls
/// back to the conflict dialog).
fn build_merge_resolve(
    branch: &str,
    conflicts: &op_git::ConflictBag,
    locale: op_editor_core::Locale,
) -> Option<op_editor_core::MergeResolveState> {
    use op_editor_core::{MergeConflictRow, MergeResolveFile, MergeResolveState};
    let mut files = Vec::new();
    for file in &conflicts.files {
        // A non-`.op` file carries no merge stages — bail to the
        // dialog rather than offer a partial resolution.
        let stages = file.stages.as_ref()?;
        // A whole-file add / delete conflict (a missing ours or
        // theirs stage) is not a per-node content conflict — the
        // node-level view cannot resolve it, so bail to the dialog.
        let (Some(base), Some(ours), Some(theirs)) = (&stages.base, &stages.ours, &stages.theirs)
        else {
            return None;
        };
        let result = op_opmerge::merge_op_documents(base, ours, theirs).ok()?;
        if result.conflicts.is_empty() {
            // Structurally clean already (the auto-apply resolver
            // would have handled it) — nothing to resolve here.
            continue;
        }
        let rows = result
            .conflicts
            .iter()
            .map(|c| MergeConflictRow {
                id: c.id.clone(),
                label: c.label.clone(),
                kind: op_i18n::translate(locale, c.kind.i18n_key()).to_string(),
                theirs_allowed: c.theirs_applicable,
                take_theirs: false,
            })
            .collect();
        let (base, ours, theirs) = (base.clone(), ours.clone(), theirs.clone());
        files.push(MergeResolveFile {
            path: file.path.clone(),
            base,
            ours,
            theirs,
            conflicts: rows,
        });
    }
    if files.is_empty() {
        return None;
    }
    Some(MergeResolveState {
        branch: branch.to_string(),
        files,
    })
}

/// Build a self-contained patch for hunk `hunk_index` of a unified
/// diff — the file header (everything before the first `@@`) plus
/// just that hunk's lines. `None` when the diff has no such hunk.
fn build_hunk_patch(lines: &[String], hunk_index: usize) -> Option<String> {
    let first = lines.iter().position(|l| l.starts_with("@@"))?;
    let hunk_starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("@@"))
        .map(|(i, _)| i)
        .collect();
    let &start = hunk_starts.get(hunk_index)?;
    let end = hunk_starts
        .get(hunk_index + 1)
        .copied()
        .unwrap_or(lines.len());
    let mut patch = String::new();
    for line in &lines[..first] {
        patch.push_str(line);
        patch.push('\n');
    }
    for line in &lines[start..end] {
        patch.push_str(line);
        patch.push('\n');
    }
    Some(patch)
}

/// Build the per-file conflict breakdown for the merge dialog. A
/// `.op` file's three index stages are run through the structured
/// node-level merge ([`op_opmerge`]) so the report names the
/// conflicting PenNodes — or flags the file as structurally
/// auto-mergeable when the node merge is clean despite git's
/// line-level conflict. Non-`.op` files list as a bare path.
fn merge_conflict_detail(
    conflicts: &op_git::ConflictBag,
    locale: op_editor_core::Locale,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    for file in &conflicts.files {
        let Some(stages) = &file.stages else {
            lines.push(file.path.clone());
            continue;
        };
        // A missing stage (e.g. add/add has no base) reads as an
        // empty document for the structured merge.
        let base = stages.base.as_deref().unwrap_or("{}");
        let ours = stages.ours.as_deref().unwrap_or("{}");
        let theirs = stages.theirs.as_deref().unwrap_or("{}");
        match op_opmerge::merge_op_documents(base, ours, theirs) {
            Ok(result) if result.is_clean() => {
                lines.push(
                    op_i18n::translate(locale, "git.merge.autoMergeable")
                        .replace("{{path}}", &file.path),
                );
            }
            Ok(result) => {
                lines.push(format!("{}:", file.path));
                for node in result.conflicts.iter().take(12) {
                    let kind = op_i18n::translate(locale, node.kind.i18n_key());
                    lines.push(format!("    • {} — {}", node.label, kind));
                }
                if result.conflicts.len() > 12 {
                    lines.push(
                        op_i18n::translate(locale, "git.merge.andMore")
                            .replace("{{count}}", &(result.conflicts.len() - 12).to_string()),
                    );
                }
            }
            Err(_) => lines.push(file.path.clone()),
        }
    }
    lines.join("\n")
}
