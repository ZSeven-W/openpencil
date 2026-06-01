//! Host glue for the git overflow menu's SSH-keys subview — kept out of the
//! already-large `git_host.rs`. Enumerates the stored keys and imports a key
//! file chosen via a native picker (TS `git-panel-ssh-keys.tsx` list view).

use op_editor_core::GitOverflowView;

use crate::DesktopApp;

impl DesktopApp {
    /// Overflow "SSH 密钥" — list the stored SSH key names into the panel and
    /// open the subview.
    pub(crate) fn enter_ssh_keys(&mut self) {
        let names = self
            .git_session
            .auth_stores()
            .and_then(|(_, ssh)| ssh.list().ok())
            .map(|keys| keys.into_iter().map(|k| k.name).collect())
            .unwrap_or_default();
        let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
        panel.ssh_keys = names;
        panel.overflow_view = GitOverflowView::SshKeys;
        panel.overflow_open = true;
    }

    /// SSH subview "导入现有密钥" — pick a private key file and import it into
    /// the key store, then refresh the list.
    pub(crate) fn import_ssh_key(&mut self) {
        let Some(source) = rfd::FileDialog::new()
            .set_title("Import SSH private key")
            .pick_file()
        else {
            return; // user cancelled
        };
        // Name the imported key after its file stem so the list shows
        // something meaningful (the store keeps its own copy).
        let name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported")
            .to_string();
        // The origin host to bind the key to (if a remote is configured).
        let host = self.git_session.repo().and_then(|r| r.origin_host());
        let imported = self.git_session.auth_stores().map(|(auth, ssh)| {
            // The store IS `~/.ssh`, so a key the user picks from there is
            // already "in the store" — copying it onto itself would fail with
            // "already exists". Only copy when the source lives elsewhere.
            if source.parent() != Some(ssh.dir()) {
                ssh.import(&source, &name)?;
            }
            // Bind the (now-stored) key as the origin host's SSH credential so
            // git operations actually USE it (not just list it) — but ONLY when
            // it won't clobber an existing HTTPS token: bind only if the host
            // has no credential yet or already uses SSH. Without a remote
            // there's no host to bind to; the key is still stored for later.
            if let Some(host) = &host {
                let existing = auth.get(host).ok().flatten();
                let safe_to_bind =
                    matches!(existing, None | Some(op_git::Credential::Ssh { .. }));
                if safe_to_bind {
                    auth.set(host, op_git::Credential::Ssh { key_name: name.clone() })?;
                }
            }
            Ok::<(), op_git::GitError>(())
        });
        match imported {
            Some(Ok(())) => self.enter_ssh_keys(), // re-enumerate
            Some(Err(e)) => eprintln!("openpencil-desktop: ssh import failed: {e}"),
            None => eprintln!("openpencil-desktop: ssh import — no key store available"),
        }
    }
}
