//! Remote operations — clone, fetch, pull, push, remote config.
//!
//! The network operations (`clone` / `fetch` / `pull` / `push`) run
//! entirely in-process through libgit2 (`git2`); there is no system
//! `git` subprocess. Authentication is supplied through a
//! [`git2::RemoteCallbacks`] credential closure built from the
//! handle's stored auth carrier (see [`GitRepo::auth_env`] /
//! [`GitRepo::with_auth_env`]); when the carrier is empty the closure
//! falls back to the ambient credential helpers / ssh-agent, exactly
//! the way the subprocess backend deferred to the user's git setup.
//! The network ops are not unit-tested here (no network in tests);
//! the remote-config readers / writers are.
//!
//! ## Auth carrier (libgit2 migration note)
//!
//! The subprocess backend carried auth as *git environment* pairs —
//! `GIT_SSH_COMMAND`, `GIT_CONFIG_*`, `OP_GIT_HTTPS_*`. libgit2 takes
//! credentials through a callback, not the environment, so the
//! `auth_env` `Vec<(String, String)>` is repurposed as a small,
//! structured credential carrier interpreted by [`build_callbacks`]:
//!
//! - `("token", "<username>:<token>")` → an HTTPS user/password
//!   credential ([`git2::Cred::userpass_plaintext`]).
//! - `("ssh_key_path", "<private-key-path>")` → an SSH key credential
//!   ([`git2::Cred::ssh_key`]) honoring the `username_from_url`.
//!
//! The public surface — [`GitRepo::auth_env`] returning a
//! `Vec<(String, String)>` and [`GitRepo::with_auth_env`] storing it —
//! is unchanged; only the *meaning* of the pairs changed, and the
//! `git_session` host wiring (`with_auth_env(repo.auth_env(..))`) keeps
//! compiling and working without edits.

use std::path::{Path, PathBuf};

use git2::{
    build::RepoBuilder, AutotagOption, Cred, CredentialType, FetchOptions, PushOptions,
    RemoteCallbacks,
};

use crate::{GitError, GitRepo, MergeOutcome};

/// A configured git remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// Remote name (`origin`, …).
    pub name: String,
    /// Fetch URL.
    pub url: String,
}

impl GitRepo {
    /// Clone `url` into `dir` and return a handle to the clone.
    /// `dir` must not already exist (libgit2 creates it).
    ///
    /// Authentication uses the ambient credential helpers / ssh-agent
    /// — a fresh clone has no stored credential carrier yet (the
    /// returned handle starts with an empty `auth_env`, matching the
    /// subprocess backend, where the spawned `git clone` likewise
    /// inherited only the ambient git environment).
    pub fn clone(url: &str, dir: &Path) -> Result<GitRepo, GitError> {
        // The parent must exist for the clone target to be created
        // inside it; libgit2 (like `git clone`) creates only `dir`,
        // not its ancestors.
        let parent = dir.parent().unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| GitError::Io(e.to_string()))?;
        }

        // No stored credential yet — the clone authenticates through
        // the ambient credential helpers / ssh-agent (the closure
        // falls back to `Cred::credential_helper` / the ssh-agent when
        // the carrier is empty, exactly as the spawned `git clone`
        // inherited only the ambient git environment).
        let auth: Vec<(String, String)> = Vec::new();
        let callbacks = build_callbacks(&auth);
        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let repo = RepoBuilder::new()
            .fetch_options(fetch_opts)
            .clone(url, dir)
            .map_err(|e| GitError::Command {
                operation: "clone".to_string(),
                stderr: e.message().to_string(),
            })?;

        // Cloning an EMPTY remote leaves the local HEAD on libgit2's own
        // default branch name, which can differ from the remote's
        // configured initial branch (e.g. `main`). Match `git clone` by
        // pointing the still-unborn HEAD at the remote's advertised
        // default branch — falling back to `refs/heads/main` (the
        // initial-branch this crate's `init` uses) when the empty remote
        // advertises none — so the first local commit lands on a
        // predictable branch.
        if repo.is_empty().unwrap_or(false) {
            let target = repo
                .find_remote("origin")
                .ok()
                .and_then(|mut origin| {
                    origin.connect(git2::Direction::Fetch).ok()?;
                    let branch = origin
                        .default_branch()
                        .ok()
                        .and_then(|b| b.as_str().map(str::to_string));
                    let _ = origin.disconnect();
                    branch
                })
                .unwrap_or_else(|| "refs/heads/main".to_string());
            let _ = repo.set_head(&target);
        }

        GitRepo::discover(dir)?.ok_or_else(|| GitError::NotARepo(PathBuf::from(dir)))
    }

    /// Fetch every remote — update remote-tracking refs without
    /// touching the working tree. Prunes deleted upstream branches,
    /// matching the former `git fetch --all --prune`.
    pub fn fetch(&self) -> Result<(), GitError> {
        let repo = self.open()?;
        // `git fetch --all` walks every configured remote, not just
        // `origin`; replicate that so a multi-remote repo behaves the
        // same as the subprocess backend.
        let remote_names = repo.remotes()?;
        for name in remote_names.iter().flatten() {
            let mut remote = repo.find_remote(name)?;
            // A fresh callback set per remote — the carrier is shared by
            // reference into each, so credentials are presented to every
            // remote uniformly. (The subprocess backend scoped its
            // credential helper per host; this carrier-based form
            // authenticates each remote with the same stored credential —
            // see the "Auth carrier" note for the behavioural delta.)
            let callbacks = build_callbacks(&self.auth_env);
            let mut fetch_opts = FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);
            fetch_opts.prune(git2::FetchPrune::On);
            fetch_opts.download_tags(AutotagOption::All);
            // Empty refspecs → use the remote's configured fetch
            // refspecs, exactly like a bare `git fetch <remote>`.
            let empty: &[&str] = &[];
            remote
                .fetch(empty, Some(&mut fetch_opts), None)
                .map_err(|e| GitError::Command {
                    operation: "fetch".to_string(),
                    stderr: e.message().to_string(),
                })?;
        }
        Ok(())
    }

    /// Fetch and integrate the current branch's upstream, returning
    /// the [`MergeOutcome`] — exactly the TS `enginePull` model: a
    /// pull is a fetch followed by a merge of the remote-tracking
    /// ref. The fast-forward / merge / up-to-date decision is the
    /// shared, ancestry-based [`GitRepo::integrate`] classifier, which
    /// also surfaces [`GitError::WorkingTreeDirty`] / refuses while a
    /// merge is in progress.
    pub fn pull(&self) -> Result<MergeOutcome, GitError> {
        // Stop *before* any network work: a pull during an unresolved
        // merge is refused by `integrate` anyway, and fetching first
        // would be wasted effort that can also hang or fail. `integrate`
        // keeps its own identical guard (it also backs `merge`).
        if self.is_merging() {
            return Err(GitError::MergeInProgress);
        }
        // Resolve the pre-pull HEAD commit.
        let before = {
            let repo = self.open()?;
            let head = repo.head().map_err(|e| GitError::Command {
                operation: "rev-parse".to_string(),
                stderr: e.message().to_string(),
            })?;
            let oid = head.target().ok_or_else(|| GitError::Command {
                operation: "rev-parse".to_string(),
                stderr: "HEAD does not point at a commit".to_string(),
            })?;
            oid.to_string()
        };
        self.fetch()?;
        // The configured upstream tracking ref; pulling without one
        // configured is a genuine error (mirrors `git rev-parse @{u}`
        // failing).
        let upstream = self.upstream_oid()?;
        self.integrate(&before, &upstream)
    }

    /// Publish the current branch to its upstream.
    ///
    /// When the branch already tracks an upstream this is a plain
    /// `push` of `HEAD` to that upstream branch. When it does not, the
    /// push targets `origin` (or the sole configured remote) and also
    /// *sets* the upstream — the user never has to configure tracking
    /// by hand (the subprocess `push -u <remote> HEAD` behaviour).
    pub fn push(&self) -> Result<(), GitError> {
        let repo = self.open()?;

        // The current branch's short name and full ref — a push needs
        // an explicit `refs/heads/<branch>` refspec.
        let head = repo.head().map_err(|e| GitError::Command {
            operation: "push".to_string(),
            stderr: e.message().to_string(),
        })?;
        if !head.is_branch() {
            return Err(GitError::Command {
                operation: "push".to_string(),
                stderr: "cannot push a detached HEAD — switch to a branch first".to_string(),
            });
        }
        let head_ref = head.name().ok_or_else(|| GitError::Command {
            operation: "push".to_string(),
            stderr: "HEAD has no symbolic name".to_string(),
        })?;
        let branch_short = head.shorthand().unwrap_or("HEAD").to_string();

        // Does this branch already track an upstream? `git push` with
        // a configured upstream pushes there; otherwise we set it up.
        let tracked = repo.branch_upstream_name(head_ref).ok();

        // Choose the target remote: the upstream's remote when one is
        // configured, else `origin`, else the sole remote.
        let remote_name = match repo.branch_upstream_remote(head_ref).ok() {
            Some(buf) => buf
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "origin".to_string()),
            None => {
                let remotes = self.remotes()?;
                remotes
                    .iter()
                    .find(|r| r.name == "origin")
                    .or_else(|| remotes.first())
                    .map(|r| r.name.clone())
                    .ok_or_else(|| GitError::Command {
                        operation: "push".to_string(),
                        stderr: "no remote configured — add one first".to_string(),
                    })?
            }
        };

        let mut remote = repo.find_remote(&remote_name)?;

        // `HEAD:refs/heads/<branch>` publishes the current branch under
        // the same name on the remote — the conventional push refspec.
        let refspec = format!("{head_ref}:refs/heads/{branch_short}");

        let callbacks = build_callbacks(&self.auth_env);
        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        remote
            .push(&[refspec.as_str()], Some(&mut push_opts))
            .map_err(|e| GitError::Command {
                operation: "push".to_string(),
                stderr: e.message().to_string(),
            })?;

        // First push of an untracked branch also sets up tracking, so
        // a later `push` / `pull` finds the upstream — the `-u` half
        // of the subprocess `push -u <remote> HEAD`.
        if tracked.is_none() {
            if let Ok(mut branch) = repo.find_branch(&branch_short, git2::BranchType::Local) {
                // `<remote>/<branch>` is the remote-tracking ref name.
                let upstream = format!("{remote_name}/{branch_short}");
                let _ = branch.set_upstream(Some(&upstream));
            }
        }
        Ok(())
    }

    /// Every configured remote with its fetch URL.
    pub fn remotes(&self) -> Result<Vec<Remote>, GitError> {
        let repo = self.open()?;
        let names = repo.remotes()?;
        let mut remotes = Vec::new();
        for name in names.iter().flatten() {
            let Ok(remote) = repo.find_remote(name) else {
                continue;
            };
            // A remote with no fetch URL is degenerate; skip it rather
            // than emit an empty URL (`git remote -v` would not list a
            // `(fetch)` line for it either).
            let Some(url) = remote.url() else {
                continue;
            };
            remotes.push(Remote {
                name: name.to_string(),
                url: url.to_string(),
            });
        }
        Ok(remotes)
    }

    /// The fetch URL of remote `name`, if it exists.
    pub fn remote_url(&self, name: &str) -> Result<Option<String>, GitError> {
        let repo = self.open()?;
        // Bind the lookup to a local so the borrowed `Remote` (whose
        // `url()` borrows `repo`) is consumed before the block ends —
        // otherwise the temporary would outlive `repo`'s drop.
        let found = repo.find_remote(name);
        match found {
            Ok(remote) => Ok(remote.url().map(str::to_string)),
            // An unknown remote is "no such remote", not a hard error —
            // the same tolerance the subprocess backend gave a non-zero
            // `git remote get-url`.
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Point remote `name` at `url`, adding the remote when it does
    /// not exist yet.
    pub fn set_remote(&self, name: &str, url: &str) -> Result<(), GitError> {
        let repo = self.open()?;
        if self.remote_url(name)?.is_some() {
            repo.remote_set_url(name, url)?;
        } else {
            repo.remote(name, url)?;
        }
        Ok(())
    }
}

impl GitRepo {
    /// Resolve the credential carrier for an authenticated network op
    /// against the `origin` remote, using the credential + SSH-key
    /// stores. Returns carrier pairs to apply via
    /// [`GitRepo::with_auth_env`] — an empty `Vec` when no stored
    /// credential matches the remote's host, in which case the network
    /// ops fall back to the ambient credential helpers / ssh-agent.
    ///
    /// The returned pairs are interpreted by [`build_callbacks`]:
    /// `("ssh_key_path", <path>)` for SSH, `("token", "<user>:<tok>")`
    /// for HTTPS. They are no longer git environment variables — see
    /// the module-level "Auth carrier" note.
    pub fn auth_env(
        &self,
        auth: &crate::AuthStore,
        ssh: &crate::SshKeyStore,
    ) -> Vec<(String, String)> {
        let Ok(Some(url)) = self.remote_url("origin") else {
            return Vec::new();
        };
        let host = remote_host(&url);
        if host.is_empty() {
            return Vec::new();
        }
        let Ok(Some(credential)) = auth.get(&host) else {
            return Vec::new();
        };
        match credential {
            crate::Credential::Ssh { key_name } => match ssh.load(&key_name) {
                // Carry the private-key path; `build_callbacks` turns it
                // into a `Cred::ssh_key` honoring `username_from_url`.
                Ok(key) => vec![(
                    "ssh_key_path".to_string(),
                    key.private_path.display().to_string(),
                )],
                Err(_) => Vec::new(),
            },
            crate::Credential::Https { username, token } => {
                // A control character in the credential cannot be
                // carried safely (it would corrupt a downstream
                // credential protocol if the carrier is ever spilled
                // back to git) — reject it, as the subprocess backend
                // did, rather than present a malformed credential.
                if has_control_char(&username) || has_control_char(&token) {
                    return Vec::new();
                }
                // `<username>:<token>` — `build_callbacks` splits on the
                // first `:` into a `Cred::userpass_plaintext`. The `host`
                // pair scopes the token: `build_callbacks` only releases
                // it to a URL whose host matches, so a redirect to (or a
                // tampered origin pointing at) a different host can never
                // exfiltrate the PAT — preserving the host-scoping the
                // subprocess credential helper had.
                vec![
                    ("token".to_string(), format!("{username}:{token}")),
                    ("host".to_string(), host),
                ]
            }
        }
    }
}

impl GitRepo {
    /// The host of the `origin` remote — `Some("github.com")` etc.
    /// `None` when there is no `origin` or its URL has no host.
    pub fn origin_host(&self) -> Option<String> {
        let url = self.remote_url("origin").ok()??;
        let host = remote_host(&url);
        (!host.is_empty()).then_some(host)
    }

    /// The commit Oid (hex) of the current branch's configured
    /// upstream tracking ref. Errors — mapping to a
    /// [`GitError::Command`] — when no upstream is configured, mirroring
    /// the subprocess `git rev-parse @{u}` failure.
    fn upstream_oid(&self) -> Result<String, GitError> {
        let repo = self.open()?;
        let head = repo.head().map_err(|e| GitError::Command {
            operation: "rev-parse".to_string(),
            stderr: e.message().to_string(),
        })?;
        let head_ref = head.name().ok_or_else(|| GitError::Command {
            operation: "rev-parse".to_string(),
            stderr: "HEAD has no symbolic name".to_string(),
        })?;
        // `branch_upstream_name` yields the remote-tracking ref name
        // (`refs/remotes/origin/main`) for the branch's `@{u}`.
        let upstream_buf = repo
            .branch_upstream_name(head_ref)
            .map_err(|e| GitError::Command {
                operation: "rev-parse".to_string(),
                stderr: e.message().to_string(),
            })?;
        let upstream_ref = upstream_buf.as_str().ok_or_else(|| GitError::Command {
            operation: "rev-parse".to_string(),
            stderr: "upstream ref name is not valid UTF-8".to_string(),
        })?;
        let reference = repo
            .find_reference(upstream_ref)
            .map_err(|e| GitError::Command {
                operation: "rev-parse".to_string(),
                stderr: e.message().to_string(),
            })?;
        // Peel to the commit it points at — the upstream tip's Oid.
        let oid = reference
            .peel_to_commit()
            .map_err(|e| GitError::Command {
                operation: "rev-parse".to_string(),
                stderr: e.message().to_string(),
            })?
            .id();
        Ok(oid.to_string())
    }
}

/// Build the [`RemoteCallbacks`] that authenticate a network op from
/// the handle's stored credential carrier (`auth` — the repurposed
/// `auth_env` `Vec`). The `.credentials` closure interprets the
/// carrier keys:
///
/// - `("ssh_key_path", <path>)` → [`Cred::ssh_key`] for the
///   `username_from_url` (defaulting to `git`).
/// - `("token", "<user>:<tok>")` → [`Cred::userpass_plaintext`].
///
/// When the carrier holds nothing usable for the credential type
/// libgit2 asks for, the closure falls back to
/// [`Cred::credential_helper`] (the user's configured helpers) for
/// HTTPS, [`Cred::ssh_key_from_agent`] for SSH, and finally
/// [`Cred::default`] — so an un-authenticated handle behaves exactly
/// like the subprocess backend deferring to the ambient git setup.
fn build_callbacks<'a>(auth: &'a [(String, String)]) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, allowed| {
        // SSH key authentication — preferred when libgit2 asks for it
        // and we carry a key path.
        if allowed.contains(CredentialType::SSH_KEY) {
            let user = username_from_url.unwrap_or("git");
            if let Some(path) = carrier_value(auth, "ssh_key_path") {
                return Cred::ssh_key(user, None, Path::new(path), None);
            }
            // No stored key — let the agent answer (ssh-agent / Pageant),
            // matching the subprocess backend's ambient ssh-agent use.
            return Cred::ssh_key_from_agent(user);
        }

        // HTTPS username/password (personal access token).
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            // Only release the stored token to the host it was scoped to.
            // libgit2 invokes this closure with the URL it is actually
            // authenticating against, which a redirect (or a tampered
            // remote) can change — without this check the PAT would be
            // sent to an attacker-controlled host.
            let host_ok = match carrier_value(auth, "host") {
                Some(expected) => &remote_host(url) == expected,
                // No host scoping recorded — be conservative and do not
                // release the token blindly; fall through to the helpers.
                None => false,
            };
            if host_ok {
                if let Some(pair) = carrier_value(auth, "token") {
                    // `<username>:<token>` — split on the FIRST `:` so a
                    // token containing `:` survives intact.
                    let (user, token) = match pair.split_once(':') {
                        Some((u, t)) => (u, t),
                        None => ("", pair.as_str()),
                    };
                    return Cred::userpass_plaintext(user, token);
                }
            }
            // Fall back to the user's configured credential helpers
            // (osxkeychain / manager / store) for ambient HTTPS auth.
            let config = git2::Config::open_default()?;
            return Cred::credential_helper(&config, url, username_from_url);
        }

        // libgit2 sometimes asks only for the SSH *username* before the
        // key exchange (e.g. an scp-like URL without `user@`).
        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username_from_url.unwrap_or("git"));
        }

        // Nothing matched — defer to the default credential (e.g. an
        // anonymous / already-authenticated transport).
        Cred::default()
    });
    callbacks
}

/// The value of carrier key `key`, if present.
fn carrier_value<'a>(auth: &'a [(String, String)], key: &str) -> Option<&'a String> {
    auth.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Whether `s` holds an ASCII control character — a credential with
/// one cannot be carried safely.
fn has_control_char(s: &str) -> bool {
    s.chars().any(|c| c.is_control())
}

/// Extract the host from a git remote URL — `scheme://[user@]host/…`
/// or the scp-like `[user@]host:path`. Handles a bracketed IPv6
/// literal and rejects a Windows drive path (`C:\…`). Empty when the
/// URL has no recognizable host.
fn remote_host(url: &str) -> String {
    let url = url.trim();
    let authority = if let Some(rest) = url.split("://").nth(1) {
        // scheme://[user@]host[:port]/path — the authority ends at
        // the first `/`.
        let after_user = rest.rsplit('@').next().unwrap_or(rest);
        after_user.split('/').next().unwrap_or("")
    } else {
        // scp-like `[user@]host:path`. Strip `user@` first so a `:`
        // inside a bracketed IPv6 host is not mistaken for the
        // host/path separator.
        let without_user = match url.split_once('@') {
            Some((_, rest)) => rest,
            None => url,
        };
        if let Some(inner) = without_user.strip_prefix('[') {
            return inner.split(']').next().unwrap_or("").to_string();
        }
        let Some((host_part, _)) = without_user.split_once(':') else {
            return String::new();
        };
        // A single-letter "host" is a Windows drive (`C:\repo`), and
        // a `/` in the host part means the `:` came from a local
        // path (`/tmp/foo:bar.git`) — neither is an scp host.
        let drive_letter = host_part.len() == 1
            && host_part
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic());
        if drive_letter || host_part.contains('/') {
            return String::new();
        }
        host_part
    };
    // A bracketed IPv6 literal — `[2001:db8::1]:2222` — keeps the
    // address inside the brackets; otherwise strip a `:port` suffix.
    if let Some(inner) = authority.strip_prefix('[') {
        return inner.split(']').next().unwrap_or("").to_string();
    }
    authority.split(':').next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_host_parses_every_url_shape() {
        assert_eq!(remote_host("https://github.com/org/repo.git"), "github.com");
        assert_eq!(
            remote_host("https://user@github.com/org/repo"),
            "github.com"
        );
        assert_eq!(
            remote_host("https://gitlab.example.com:8443/x.git"),
            "gitlab.example.com"
        );
        assert_eq!(
            remote_host("ssh://git@github.com/org/repo.git"),
            "github.com"
        );
        // scp-like.
        assert_eq!(remote_host("git@github.com:org/repo.git"), "github.com");
        assert_eq!(remote_host("github.com:org/repo"), "github.com");
        // Bracketed IPv6 literal — scheme + scp-like.
        assert_eq!(
            remote_host("ssh://git@[2001:db8::1]:2222/org/repo"),
            "2001:db8::1"
        );
        assert_eq!(remote_host("git@[2001:db8::1]:repo.git"), "2001:db8::1");
        // No host — local path, Windows drive, path with a colon.
        assert_eq!(remote_host("/local/path"), "");
        assert_eq!(remote_host("C:\\repo.git"), "");
        assert_eq!(remote_host("/tmp/foo:bar.git"), "");
        assert_eq!(remote_host("./foo:bar.git"), "");
    }

    #[test]
    fn has_control_char_flags_bad_credentials() {
        assert!(!has_control_char("alice"));
        assert!(!has_control_char("ghp_AbCdEf123456"));
        assert!(has_control_char("bad\ntoken"));
        assert!(has_control_char("bad\0token"));
    }

    #[test]
    fn carrier_value_finds_the_key() {
        let carrier = vec![
            ("token".to_string(), "alice:secret".to_string()),
            (
                "ssh_key_path".to_string(),
                "/home/alice/.ssh/id".to_string(),
            ),
        ];
        assert_eq!(carrier_value(&carrier, "token").unwrap(), "alice:secret");
        assert_eq!(
            carrier_value(&carrier, "ssh_key_path").unwrap(),
            "/home/alice/.ssh/id"
        );
        assert!(carrier_value(&carrier, "missing").is_none());
    }
}
