//! SSH key management for git remote authentication.
//!
//! The Rust counterpart of the TS Electron app's `ssh-keys.ts`:
//! generate / list / import / delete the ed25519 key pairs used to
//! authenticate `git fetch` / `pull` / `push` over SSH.
//!
//! Keys live in an SSH directory — the user's `~/.ssh` in
//! production, or any directory ([`SshKeyStore::at`]) so the
//! operations are hermetically testable.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::GitError;

/// An SSH key pair in an [`SshKeyStore`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshKey {
    /// The key's base name — the private-key file name.
    pub name: String,
    /// Absolute path to the private-key file.
    pub private_path: PathBuf,
    /// The public-key text (the `.pub` file contents, trimmed).
    pub public_key: String,
}

/// A directory of SSH key pairs — normally the user's `~/.ssh`.
#[derive(Debug, Clone)]
pub struct SshKeyStore {
    dir: PathBuf,
}

impl SshKeyStore {
    /// The current user's `~/.ssh` store.
    pub fn user() -> Result<SshKeyStore, GitError> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or_else(|| GitError::Io("no home directory in the environment".into()))?;
        Ok(SshKeyStore {
            dir: PathBuf::from(home).join(".ssh"),
        })
    }

    /// A store rooted at an explicit directory.
    pub fn at(dir: impl Into<PathBuf>) -> SshKeyStore {
        SshKeyStore { dir: dir.into() }
    }

    /// The store's directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Generate a new ed25519 key pair named `name`, embedding
    /// `comment`. Fails if a key of that name already exists, so an
    /// existing key is never silently overwritten.
    pub fn generate(&self, name: &str, comment: &str) -> Result<SshKey, GitError> {
        validate_key_name(name)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| GitError::Io(e.to_string()))?;
        let private = self.dir.join(name);
        if private.exists() {
            return Err(GitError::Io(format!(
                "an SSH key named `{name}` already exists"
            )));
        }
        let private_str = private
            .to_str()
            .ok_or_else(|| GitError::Io("non-UTF-8 key path".into()))?;
        let output = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-f", private_str, "-N", "", "-C", comment])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    GitError::Io("`ssh-keygen` was not found — install OpenSSH".into())
                } else {
                    GitError::Io(e.to_string())
                }
            })?;
        if !output.status.success() {
            return Err(GitError::Command {
                operation: "ssh-keygen".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        self.load(name)
    }

    /// Every key pair in the store — one entry per `.pub` file that
    /// has a matching private-key file beside it.
    pub fn list(&self) -> Result<Vec<SshKey>, GitError> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            // A missing `~/.ssh` is simply "no keys", not an error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(GitError::Io(e.to_string())),
        };
        let mut keys = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".pub"))
            else {
                continue;
            };
            // A `.pub` without its private key is an incomplete pair.
            if self.dir.join(name).is_file() {
                if let Ok(key) = self.load(name) {
                    keys.push(key);
                }
            }
        }
        keys.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(keys)
    }

    /// Load a single key pair by name.
    pub fn load(&self, name: &str) -> Result<SshKey, GitError> {
        validate_key_name(name)?;
        let private_path = self.dir.join(name);
        if !private_path.is_file() {
            return Err(GitError::Io(format!("no SSH key named `{name}`")));
        }
        let public_key = std::fs::read_to_string(self.dir.join(format!("{name}.pub")))
            .map_err(|e| GitError::Io(format!("reading {name}.pub: {e}")))?
            .trim()
            .to_string();
        Ok(SshKey {
            name: name.to_string(),
            private_path,
            public_key,
        })
    }

    /// Import an existing private-key file into the store under
    /// `name`, copying the matching `.pub` alongside it. The private
    /// key is given owner-only (`0600`) permissions on Unix.
    pub fn import(&self, source_private_key: &Path, name: &str) -> Result<SshKey, GitError> {
        validate_key_name(name)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| GitError::Io(e.to_string()))?;
        let dest_private = self.dir.join(name);
        if dest_private.exists() {
            return Err(GitError::Io(format!(
                "an SSH key named `{name}` already exists"
            )));
        }
        // Read the source key and write it through `write_private_file`
        // so the destination is created `0600` *before* the private
        // key bytes land in it — `fs::copy` + `chmod` would leave a
        // window where the key is world-readable.
        let key_bytes = std::fs::read(source_private_key)
            .map_err(|e| GitError::Io(format!("reading source private key: {e}")))?;
        crate::write_private_file(&dest_private, &key_bytes)?;

        // The public key sits next to the private one as `<src>.pub`.
        let source_pub = append_pub(source_private_key);
        if source_pub.is_file() {
            std::fs::copy(&source_pub, self.dir.join(format!("{name}.pub")))
                .map_err(|e| GitError::Io(format!("copying public key: {e}")))?;
        }
        self.load(name)
    }

    /// Delete the key pair named `name` — both the private key and
    /// its `.pub`. Missing files are tolerated (idempotent).
    pub fn delete(&self, name: &str) -> Result<(), GitError> {
        validate_key_name(name)?;
        for path in [self.dir.join(name), self.dir.join(format!("{name}.pub"))] {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(GitError::Io(e.to_string())),
            }
        }
        Ok(())
    }
}

/// Reject a key `name` that is not a single, safe path component.
///
/// Every store method joins `name` onto the store directory; an
/// unchecked `name` such as `../../id_ed25519`, `sub/key`, or an
/// absolute path would let `generate` / `import` / `delete` write or
/// remove files *outside* the store — e.g. clobber the user's real
/// `~/.ssh/id_ed25519`. A valid name has no path separators and is
/// not a directory-traversal token.
fn validate_key_name(name: &str) -> Result<(), GitError> {
    let unsafe_name = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0');
    if unsafe_name {
        return Err(GitError::Io(format!(
            "invalid SSH key name `{name}` — must be a plain file name \
             with no path separators"
        )));
    }
    Ok(())
}

/// `<path>` → `<path>.pub`.
fn append_pub(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".pub");
    PathBuf::from(s)
}

