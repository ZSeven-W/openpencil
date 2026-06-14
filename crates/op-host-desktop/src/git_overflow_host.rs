//! Host glue for the git overflow menu's three new actions — kept out of the
//! already-large `git_host.rs`. Ports the TS git-store methods
//! `enterTrackedFilePicker` / `bindTrackedFile` / `clearAuthorIdentity` /
//! `closeRepo`.

use std::cmp::Ordering;
use std::path::PathBuf;

use op_editor_core::{GitCandidateFile, GitOverflowView, GitPanelState};

use crate::DesktopApp;
use crate::{git_jobs, persistence};

impl DesktopApp {
    /// Overflow "切换跟踪文件" — enumerate the repo's `.op` candidates (sorted
    /// newest-commit first, then by path) into the panel and open the picker.
    pub(crate) fn enter_tracked_picker(&mut self) {
        let mut candidates = self
            .git_session
            .repo()
            .and_then(|r| r.candidate_op_files().ok())
            .unwrap_or_default();
        // TS sort: lastCommitAt DESC (None last), tiebreak relativePath ASC.
        candidates.sort_by(|a, b| match (a.last_commit_secs, b.last_commit_secs) {
            (Some(x), Some(y)) => y
                .cmp(&x)
                .then_with(|| a.relative_path.cmp(&b.relative_path)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.relative_path.cmp(&b.relative_path),
        });
        let now_secs = now_unix_secs();
        let files: Vec<GitCandidateFile> = candidates
            .into_iter()
            .map(|c| GitCandidateFile {
                path: c.path,
                relative_path: c.relative_path,
                milestone_count: c.milestone_count,
                last_commit_time: c
                    .last_commit_secs
                    .map(|s| git_jobs::format_compact_time(s, now_secs))
                    .unwrap_or_default(),
                last_commit_message: c.last_commit_message,
            })
            .collect();

        let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
        panel.candidate_files = files;
        panel.open_tracked_picker();
        panel.overflow_view = GitOverflowView::TrackedPicker;
        panel.overflow_open = true;
    }

    /// Bind the panel to `path` as the tracked file (TS `bindTrackedFile`).
    /// `open` also loads that `.op` into the editor (TS "track and open").
    pub(crate) fn bind_tracked_file(&mut self, path: String, open: bool) {
        let path = PathBuf::from(path);
        {
            let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
            panel.overflow_open = false;
            panel.overflow_view = GitOverflowView::Menu;
            panel.close_tracked_picker();
            panel.candidate_files.clear();
        }
        if open {
            // "Track and open" loads the file as the editor document, then
            // rebinds the Git session to whatever is now `current_path` — so
            // the session and the open document can't diverge (`open_path`
            // updates `current_path` on success, leaves it on failure; the
            // canonical rebind follows it either way).
            if self.confirm_document_reload()
                && persistence::open_path(
                    &mut self.host,
                    path,
                    &mut self.current_path,
                    self.window.as_ref(),
                )
            {
                self.mark_document_saved();
            }
            self.rebind_git_session_for_current_path();
        } else {
            // "Track only" — bind the session to `path` without touching the
            // editor document (TS `bindTrackedFile` with no open follow-up).
            self.git_session.rebind(Some(&path));
        }
    }

    /// Overflow "清除提交作者" — clear the repo's local commit-author identity.
    pub(crate) fn clear_commit_author(&mut self) {
        if let Some(repo) = self.git_session.repo() {
            if let Err(e) = repo.unset_local_author() {
                eprintln!("openpencil-desktop: git clear-author failed: {e}");
            }
        }
    }

    /// Commit-signature form "保存" — validate + write the name/email drafts
    /// into the repo identity, then re-fire the deferred milestone commit
    /// (TS `setAuthorIdentity` + the pending-commit re-run).
    pub(crate) fn save_author_identity(&mut self) {
        let (name, email) = {
            let panel = &self.host.editor_state().editor_ui.git_panel;
            (
                panel.author_name_input.text().trim().to_string(),
                panel.author_email_input.text().trim().to_string(),
            )
        };
        // Basic validation (TS validationName / validationEmail). Leave the
        // form open on failure so the user can correct it.
        if name.is_empty() || !email.contains('@') {
            return;
        }
        if let Some(repo) = self.git_session.repo() {
            if let Err(e) = repo.set_local_author(&name, &email) {
                eprintln!("openpencil-desktop: set commit author failed: {e}");
                return;
            }
        }
        let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
        panel.author_prompt = false;
        panel.author_name_focused = false;
        panel.author_email_focused = false;
        // Re-fire the deferred commit — the message is still in
        // `commit_input` and the identity now resolves.
        panel.pending_action = Some(op_editor_core::GitPanelAction::CommitMilestone);
    }

    /// Overflow "关闭仓库" — unbind the repository and reset the panel to its
    /// empty state (TS `closeRepo`).
    pub(crate) fn close_repo(&mut self) {
        self.git_session.rebind(None);
        let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
        *panel = GitPanelState::default();
    }
}

/// Current wall-clock Unix seconds (0 if the clock is before the epoch).
fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
