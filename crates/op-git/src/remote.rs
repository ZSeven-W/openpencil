//! Remote operations — clone, fetch, pull, push, remote config.
//!
//! The network operations (`clone` / `fetch` / `pull` / `push`)
//! depend on the ambient git credential / SSH setup; dedicated
//! credential + SSH-key handling lands in a later increment. They
//! are not unit-tested here (no network in tests); the remote-config
//! readers / writers are.

use std::path::{Path, PathBuf};

use crate::{git_output, stderr_of, GitError, GitRepo, MergeOutcome};

/// A configured git remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// Remote name (`origin`, …).
    pub name: String,
    /// Fetch URL.
    pub url: String,
}

impl GitRepo {
    /// `git clone <url>` into `dir` and return a handle to the clone.
    /// `dir` must not already exist (git creates it).
    pub fn clone(url: &str, dir: &Path) -> Result<GitRepo, GitError> {
        // Run from `dir`'s parent so a relative target resolves; the
        // parent must exist for git to create `dir` inside it.
        let parent = dir.parent().unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| GitError::Io(e.to_string()))?;
        }
        let dir_str = dir.to_str().ok_or_else(|| GitError::Io("non-UTF-8 path".into()))?;
        let output = git_output(parent, &["clone", url, dir_str])?;
        if !output.status.success() {
            return Err(GitError::Command {
                operation: "clone".to_string(),
                stderr: stderr_of(&output),
            });
        }
        GitRepo::discover(dir)?.ok_or_else(|| GitError::NotARepo(PathBuf::from(dir)))
    }

    /// `git fetch` — update remote-tracking refs without touching the
    /// working tree.
    pub fn fetch(&self) -> Result<(), GitError> {
        self.run(&["fetch", "--all", "--prune"])?;
        Ok(())
    }

    /// Fetch and integrate the current branch's upstream, returning
    /// the [`MergeOutcome`] — exactly the TS `enginePull` model: a
    /// pull is a fetch followed by a merge of the remote-tracking
    /// ref. The fast-forward / merge / up-to-date decision is the
    /// shared, ancestry-based [`GitRepo::integrate`] classifier.
    pub fn pull(&self) -> Result<MergeOutcome, GitError> {
        // Stop *before* any network work: a pull during an unresolved
        // merge is refused by `integrate` anyway, and fetching first
        // would be wasted effort that can also hang or fail. `integrate`
        // keeps its own identical guard (it also backs `merge`).
        if self.is_merging() {
            return Err(GitError::MergeInProgress);
        }
        let before = self.run(&["rev-parse", "HEAD"])?.trim().to_string();
        self.fetch()?;
        // `@{u}` is the configured upstream tracking ref; pulling
        // without one configured is a genuine error.
        let upstream = self.run(&["rev-parse", "@{u}"])?.trim().to_string();
        self.integrate(&before, &upstream)
    }

    /// `git push` — publish the current branch to its upstream.
    ///
    /// When the branch already tracks an upstream this is a plain
    /// `git push`. When it does not, the push targets `origin` (or
    /// the sole configured remote) with `-u`, so the very first push
    /// also *sets* the upstream — the user never has to configure
    /// tracking by hand.
    pub fn push(&self) -> Result<(), GitError> {
        if self.run(&["rev-parse", "--abbrev-ref", "@{u}"]).is_ok() {
            self.run(&["push"])?;
            return Ok(());
        }
        let remotes = self.remotes()?;
        let remote = remotes
            .iter()
            .find(|r| r.name == "origin")
            .or_else(|| remotes.first())
            .ok_or_else(|| GitError::Command {
                operation: "push".to_string(),
                stderr: "no remote configured — add one first".to_string(),
            })?;
        let remote_name = remote.name.clone();
        self.run(&["push", "-u", &remote_name, "HEAD"])?;
        Ok(())
    }

    /// Every configured remote with its fetch URL.
    pub fn remotes(&self) -> Result<Vec<Remote>, GitError> {
        let raw = self.run(&["remote", "-v"])?;
        Ok(parse_remotes(&raw))
    }

    /// The fetch URL of remote `name`, if it exists.
    pub fn remote_url(&self, name: &str) -> Result<Option<String>, GitError> {
        match self.run(&["remote", "get-url", name]) {
            Ok(url) => Ok(Some(url.trim().to_string())),
            // `git remote get-url` exits non-zero for an unknown
            // remote — that is "no such remote", not a hard error.
            Err(GitError::Command { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Point remote `name` at `url`, adding the remote when it does
    /// not exist yet.
    pub fn set_remote(&self, name: &str, url: &str) -> Result<(), GitError> {
        if self.remote_url(name)?.is_some() {
            self.run(&["remote", "set-url", name, url])?;
        } else {
            self.run(&["remote", "add", name, url])?;
        }
        Ok(())
    }
}

impl GitRepo {
    /// Resolve the git environment for an authenticated network op
    /// against the `origin` remote, using the credential + SSH-key
    /// stores. Returns env-var pairs to apply via
    /// [`GitRepo::with_auth_env`] — an empty `Vec` when no stored
    /// credential matches the remote's host, in which case git falls
    /// back to its ambient credential helpers / ssh-agent.
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
                Ok(key) => vec![(
                    "GIT_SSH_COMMAND".to_string(),
                    // The key path is shell-quoted — `GIT_SSH_COMMAND`
                    // is parsed shell-like by git, so an unescaped
                    // path containing a quote / `$()` would otherwise
                    // break out and run shell code.
                    format!(
                        "ssh -i {} -o IdentitiesOnly=yes",
                        shell_single_quote(&key.private_path.display().to_string())
                    ),
                )],
                Err(_) => Vec::new(),
            },
            crate::Credential::Https { username, token } => {
                // A control character in the credential would corrupt
                // the line-based credential protocol — reject it
                // rather than emit a malformed (or unsafe) helper.
                if has_control_char(&username) || has_control_char(&token) {
                    return Vec::new();
                }
                // The credential-helper scope key must carry the
                // remote's port — git's credential URL match treats a
                // portless config URL as the default port, so a
                // non-standard-port HTTPS remote would otherwise miss.
                https_auth_env(&remote_authority(&url), username, token)
            }
        }
    }
}

/// The git environment for an HTTPS-authenticated op against
/// `authority` (a `host` or `host:port`).
///
/// The credential helper is registered **URL-scoped** —
/// `credential.https://<authority>.helper`, not the bare
/// `credential.helper` — so a multi-remote operation (`fetch --all`)
/// can never present this token to a *different* remote. The
/// `authority` keeps any explicit port because git's credential URL
/// match is port-sensitive. The helper itself is a static shell
/// snippet (no interpolated user data, so a crafted credential
/// cannot inject shell code); the username / token reach it through
/// dedicated env vars, emitted verbatim by `printf '%s'`.
fn https_auth_env(authority: &str, username: String, token: String) -> Vec<(String, String)> {
    let helper = "!f() { printf '%s\\n' \
        \"username=$OP_GIT_HTTPS_USER\" \
        \"password=$OP_GIT_HTTPS_PASS\"; }; f";
    vec![
        ("GIT_CONFIG_COUNT".to_string(), "1".to_string()),
        (
            "GIT_CONFIG_KEY_0".to_string(),
            format!("credential.https://{authority}.helper"),
        ),
        ("GIT_CONFIG_VALUE_0".to_string(), helper.to_string()),
        ("OP_GIT_HTTPS_USER".to_string(), username),
        ("OP_GIT_HTTPS_PASS".to_string(), token),
    ]
}

/// Whether `s` holds an ASCII control character — a credential with
/// one cannot be carried safely by git's line-based protocol.
fn has_control_char(s: &str) -> bool {
    s.chars().any(|c| c.is_control())
}

/// POSIX single-quote `s` so it survives shell word-splitting intact
/// — every embedded `'` becomes `'\''`.
fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

impl GitRepo {
    /// The host of the `origin` remote — `Some("github.com")` etc.
    /// `None` when there is no `origin` or its URL has no host.
    pub fn origin_host(&self) -> Option<String> {
        let url = self.remote_url("origin").ok()??;
        let host = remote_host(&url);
        (!host.is_empty()).then_some(host)
    }
}

/// The authority of a git remote URL — `host` or `host:port`
/// (bracketed for IPv6) — verbatim, port preserved. Used to build a
/// port-sensitive `credential.https://<authority>` scope key. An
/// scp-like remote carries no URL port, so its bare host is used.
fn remote_authority(url: &str) -> String {
    let url = url.trim();
    if let Some(rest) = url.split("://").nth(1) {
        let after_user = rest.rsplit('@').next().unwrap_or(rest);
        after_user.split('/').next().unwrap_or("").to_string()
    } else {
        remote_host(url)
    }
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
            && host_part.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
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

/// Parse `git remote -v` output. Each remote prints a `(fetch)` and
/// a `(push)` line; the `(fetch)` URL is kept, deduplicated by name.
fn parse_remotes(raw: &str) -> Vec<Remote> {
    let mut remotes: Vec<Remote> = Vec::new();
    for line in raw.lines() {
        // Format: `<name>\t<url> (fetch|push)`.
        if !line.contains("(fetch)") {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(name), Some(url)) = (parts.next(), parts.next()) else {
            continue;
        };
        if !remotes.iter().any(|r| r.name == name) {
            remotes.push(Remote {
                name: name.to_string(),
                url: url.to_string(),
            });
        }
    }
    remotes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fetch_urls_only_deduped() {
        let raw = "origin\tgit@github.com:ZSeven-W/openpencil.git (fetch)\n\
                   origin\tgit@github.com:ZSeven-W/openpencil.git (push)\n\
                   fork\thttps://example.com/x.git (fetch)\n\
                   fork\thttps://example.com/x.git (push)\n";
        let remotes = parse_remotes(raw);
        assert_eq!(remotes.len(), 2);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(remotes[0].url, "git@github.com:ZSeven-W/openpencil.git");
        assert_eq!(remotes[1].name, "fork");
    }

    #[test]
    fn empty_remote_output_is_empty() {
        assert!(parse_remotes("").is_empty());
    }

    #[test]
    fn remote_host_parses_every_url_shape() {
        assert_eq!(remote_host("https://github.com/org/repo.git"), "github.com");
        assert_eq!(remote_host("https://user@github.com/org/repo"), "github.com");
        assert_eq!(remote_host("https://gitlab.example.com:8443/x.git"), "gitlab.example.com");
        assert_eq!(remote_host("ssh://git@github.com/org/repo.git"), "github.com");
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
    fn shell_single_quote_escapes_embedded_quotes() {
        assert_eq!(shell_single_quote("/plain/key"), "'/plain/key'");
        assert_eq!(shell_single_quote("/a'b/key"), "'/a'\\''b/key'");
    }

    #[test]
    fn https_auth_env_scopes_the_helper_to_the_host() {
        let env = https_auth_env("github.com", "alice".into(), "tok".into());
        let key = &env.iter().find(|(k, _)| k == "GIT_CONFIG_KEY_0").unwrap().1;
        // URL-scoped — never the bare `credential.helper`, so a
        // `fetch --all` cannot leak this token to another remote.
        assert_eq!(key, "credential.https://github.com.helper");
        // The values travel as env vars, not interpolated.
        assert!(env.iter().any(|(k, v)| k == "OP_GIT_HTTPS_USER" && v == "alice"));
        assert!(env.iter().any(|(k, v)| k == "OP_GIT_HTTPS_PASS" && v == "tok"));
    }

    #[test]
    fn https_auth_env_keeps_a_non_standard_port() {
        // git's credential URL match is port-sensitive — a portless
        // scope key would miss a `:8443` remote.
        let env = https_auth_env("gitlab.example.com:8443", "u".into(), "t".into());
        let key = &env.iter().find(|(k, _)| k == "GIT_CONFIG_KEY_0").unwrap().1;
        assert_eq!(key, "credential.https://gitlab.example.com:8443.helper");
    }

    #[test]
    fn remote_authority_keeps_the_port() {
        assert_eq!(remote_authority("https://github.com/org/repo.git"), "github.com");
        assert_eq!(
            remote_authority("https://gitlab.example.com:8443/x.git"),
            "gitlab.example.com:8443"
        );
        assert_eq!(
            remote_authority("https://user@gitlab.example.com:8443/x"),
            "gitlab.example.com:8443"
        );
        // scp-like carries no URL port — bare host.
        assert_eq!(remote_authority("git@github.com:org/repo.git"), "github.com");
    }
}
