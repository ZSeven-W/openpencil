//! Git remote credential storage.
//!
//! The Rust counterpart of the TS Electron app's `auth-store.ts`:
//! a host-keyed table of the credentials used to authenticate
//! `git fetch` / `pull` / `push` against remotes.
//!
//! Credentials persist as a JSON file — `AuthStore::user()` puts it
//! under the per-user config directory; `AuthStore::at()` takes an
//! explicit path so the store is hermetically testable. On Unix the
//! file is created with owner-only (`0600`) permissions.
//!
//! NOTE: tokens are stored in plain text inside that `0600` file.
//! Binding the store to an OS keychain (macOS Keychain / libsecret /
//! Windows Credential Manager) is a deliberate later hardening step;
//! the file store is the portable foundation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::GitError;

/// A credential for authenticating to a git remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Credential {
    /// HTTPS — a username plus a personal-access-token / password.
    Https {
        /// Remote account user name.
        username: String,
        /// Personal access token or password.
        token: String,
    },
    /// SSH — the name of an [`crate::SshKeyStore`] key to use.
    Ssh {
        /// Key name within the SSH key store.
        key_name: String,
    },
}

/// A host-keyed credential store, persisted as a JSON file.
#[derive(Debug, Clone)]
pub struct AuthStore {
    path: PathBuf,
}

/// On-disk shape of the store.
#[derive(Debug, Default, Serialize, Deserialize)]
struct AuthFile {
    /// Credentials keyed by host (or full remote URL — the caller
    /// decides the key granularity).
    #[serde(default)]
    credentials: BTreeMap<String, Credential>,
}

impl AuthStore {
    /// The per-user credential store
    /// (`~/.openpencil/git-auth.json`).
    pub fn user() -> Result<AuthStore, GitError> {
        let base = op_config_store::openpencil_dir().map_err(io_error)?;
        Ok(AuthStore {
            path: base.join(op_config_store::well_known::GIT_AUTH),
        })
    }

    /// A store backed by an explicit JSON file path.
    pub fn at(path: impl Into<PathBuf>) -> AuthStore {
        AuthStore { path: path.into() }
    }

    /// The store's backing file path.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// The credential stored for `host`, if any.
    pub fn get(&self, host: &str) -> Result<Option<Credential>, GitError> {
        Ok(self.load()?.credentials.remove(host))
    }

    /// Store (or replace) the credential for `host`.
    pub fn set(&self, host: &str, credential: Credential) -> Result<(), GitError> {
        let mut file = self.load()?;
        file.credentials.insert(host.to_string(), credential);
        self.save(&file)
    }

    /// Remove `host`'s credential. Removing an absent host is a
    /// tolerated no-op.
    pub fn remove(&self, host: &str) -> Result<(), GitError> {
        let mut file = self.load()?;
        if file.credentials.remove(host).is_some() {
            self.save(&file)?;
        }
        Ok(())
    }

    /// Every host with a stored credential, sorted.
    pub fn hosts(&self) -> Result<Vec<String>, GitError> {
        Ok(self.load()?.credentials.into_keys().collect())
    }

    /// Read the store file — a missing file is an empty store.
    fn load(&self) -> Result<AuthFile, GitError> {
        op_config_store::read_json_path(&self.path)
            .map(|value| value.unwrap_or_default())
            .map_err(io_error)
    }

    /// Write the store file atomically: the secrets land in a temp
    /// file that is created with owner-only (`0600`) permissions
    /// *before* any content is written, then it is renamed into
    /// place (rename preserves the mode).
    fn save(&self, file: &AuthFile) -> Result<(), GitError> {
        op_config_store::write_json_path(&self.path, file).map_err(io_error)
    }
}

fn io_error(e: std::io::Error) -> GitError {
    GitError::Io(e.to_string())
}
