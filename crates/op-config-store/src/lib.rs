//! Per-user file-backed config primitives.
//!
//! Desktop defaults to `~/.openpencil`. Embedded hosts must call
//! [`configure_user_root`] with their private app-sandbox directory before
//! any process-level config access.

use serde::{de::DeserializeOwned, Serialize};
use std::ffi::OsString;
use std::io::{Error, ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Process-wide config root. First resolution wins so a late mobile
/// reconfiguration cannot split state between HOME and the app sandbox.
static USER_ROOT_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();
static USER_ROOT_INIT_LOCK: Mutex<()> = Mutex::new(());
/// Set only by [`configure_user_root`] — never by the lazy `~/.openpencil`
/// default — so callers can distinguish "an embedded shell selected its
/// app sandbox" from "desktop resolved the home-directory default".
static EXPLICIT_USER_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Name of the per-user config directory under the home directory
/// (`~/.openpencil`). Exposed for callers that must resolve the
/// directory under an explicit home (e.g. subprocess-safety checks
/// against a caller-supplied `$HOME`); everything else should go
/// through [`openpencil_dir`].
pub const OPENPENCIL_DIR_NAME: &str = ".openpencil";

/// Environment variables shared across OpenPencil processes.
pub mod env_vars {
    /// Per-instance MCP identity/shutdown token the spawning CLI passes
    /// to the host process (echoed in the server's `ping` reply).
    pub const MCP_TOKEN: &str = "OPENPENCIL_MCP_TOKEN";
}

pub mod well_known {
    /// Existing Rust git credential store file.
    pub const GIT_AUTH: &str = "git-auth.json";
    /// Plan-level generic credentials name; currently aliases Rust git auth.
    pub const CREDENTIALS: &str = GIT_AUTH;
    /// Reserved for a future file-backed SSH key registry.
    pub const SSH_KEYS: &str = "ssh-keys.json";
    /// Live editor discovery file written under `~/.openpencil`.
    pub const LIVE_MCP_PORT: &str = ".op-mcp-port";
    /// Default CLI session document.
    pub const CLI_SESSION: &str = "cli-session.op";
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    root: PathBuf,
}

impl ConfigStore {
    pub fn user() -> std::io::Result<Self> {
        Ok(Self {
            root: default_openpencil_dir()?,
        })
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, file: &str) -> std::io::Result<PathBuf> {
        validate_relative_file(file)?;
        Ok(self.root.join(file))
    }

    pub fn read_json<T: DeserializeOwned>(&self, file: &str) -> std::io::Result<Option<T>> {
        read_json_path(&self.path(file)?)
    }

    pub fn write_json<T: Serialize>(&self, file: &str, value: &T) -> std::io::Result<()> {
        write_json_path(&self.path(file)?, value)
    }
}

pub fn openpencil_dir() -> std::io::Result<PathBuf> {
    let dir = default_openpencil_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Selects the private application-sandbox directory used by every
/// process-level config helper.
///
/// This must run before [`ConfigStore::user`], [`openpencil_dir`],
/// [`read_json`], or [`write_json`]. Repeating the same canonical directory is
/// idempotent; selecting a different directory after initialization is
/// rejected. The root itself must be an absolute, real directory rather than
/// a symlink. On Unix its permissions are restricted to the current user.
pub fn configure_user_root(root: impl Into<PathBuf>) -> std::io::Result<&'static Path> {
    let _guard = USER_ROOT_INIT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let configured = configure_user_root_in(&USER_ROOT_OVERRIDE, root.into())?;
    let _ = EXPLICIT_USER_ROOT.set(configured.to_path_buf());
    Ok(configured)
}

/// The private app-sandbox root an embedded shell explicitly selected via
/// [`configure_user_root`], or `None` when the process runs on the desktop
/// default. Persistence layers that must not touch a desktop config
/// location from an embedded process (mobile `settings.json`) key off this.
pub fn configured_user_root() -> Option<PathBuf> {
    EXPLICIT_USER_ROOT.get().cloned()
}

fn configure_user_root_in(slot: &OnceLock<PathBuf>, root: PathBuf) -> std::io::Result<&Path> {
    if let Some(current) = slot.get() {
        return match existing_explicit_user_root(&root) {
            Ok(requested) if &requested == current => Ok(current.as_path()),
            Ok(_) | Err(_) => Err(Error::new(
                ErrorKind::AlreadyExists,
                "config root is already initialized",
            )),
        };
    }
    let root = prepare_explicit_user_root(root)?;
    install_user_root(slot, root)
}

pub fn read_json<T: DeserializeOwned>(file: &str) -> std::io::Result<Option<T>> {
    ConfigStore::user()?.read_json(file)
}

pub fn write_json<T: Serialize>(file: &str, value: &T) -> std::io::Result<()> {
    ConfigStore::user()?.write_json(file, value)
}

pub fn read_json_path<T: DeserializeOwned>(path: &Path) -> std::io::Result<Option<T>> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn write_json_path<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let json =
        serde_json::to_vec_pretty(value).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(path)?;
    write_private_file(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Redirect [`ConfigStore::user`] — and everything built on it
/// (`read_json` / `write_json` / [`openpencil_dir`]) — at `root` for the
/// remainder of THIS PROCESS.
///
/// Exists so a test binary can point the whole crate graph at a scratch
/// directory instead of the developer's real `~/.openpencil`. Without it
/// a test that exercises a persistence path rewrites live user config:
/// `cargo test -p op-host-desktop` was measured clobbering
/// `agents.json`, the file that decides which agent providers
/// auto-reconnect at launch, with whichever value the last parallel test
/// happened to write.
///
/// Deliberately NOT an environment variable. The harness runs cases in
/// parallel threads of a single process, where `set_var` races every
/// concurrent reader — and is `unsafe` on current Rust besides.
///
/// First call wins and later calls are no-ops, so every test can install
/// the same root unconditionally with no ordering rules between them.
/// Returns the root actually in force.
#[doc(hidden)]
pub fn redirect_user_root_for_tests(root: impl Into<PathBuf>) -> &'static Path {
    USER_ROOT_OVERRIDE.get_or_init(|| root.into()).as_path()
}

fn default_openpencil_dir() -> std::io::Result<PathBuf> {
    let _guard = USER_ROOT_INIT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(root) = USER_ROOT_OVERRIDE.get() {
        return Ok(root.clone());
    }
    let default = home_dir()?.join(OPENPENCIL_DIR_NAME);
    Ok(install_user_root(&USER_ROOT_OVERRIDE, default)?.to_path_buf())
}

fn existing_explicit_user_root(root: &Path) -> std::io::Result<PathBuf> {
    if !root.is_absolute() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "config root must be absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "config root must be a real directory",
        ));
    }
    root.canonicalize()
}

fn prepare_explicit_user_root(root: PathBuf) -> std::io::Result<PathBuf> {
    if !root.is_absolute() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "config root must be absolute",
        ));
    }
    std::fs::create_dir_all(&root)?;
    let metadata = std::fs::symlink_metadata(&root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "config root must be a real directory",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
    }
    root.canonicalize()
}

fn install_user_root(slot: &OnceLock<PathBuf>, root: PathBuf) -> std::io::Result<&Path> {
    if let Some(current) = slot.get() {
        return if current == &root {
            Ok(current.as_path())
        } else {
            Err(Error::new(
                ErrorKind::AlreadyExists,
                "config root is already initialized",
            ))
        };
    }
    match slot.set(root) {
        Ok(()) => Ok(slot
            .get()
            .expect("config root was just installed")
            .as_path()),
        Err(root) => {
            let current = slot.get().expect("another thread installed config root");
            if current == &root {
                Ok(current.as_path())
            } else {
                Err(Error::new(
                    ErrorKind::AlreadyExists,
                    "config root is already initialized",
                ))
            }
        }
    }
}

pub fn home_dir() -> std::io::Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "home directory not available"))
}

fn validate_relative_file(file: &str) -> std::io::Result<()> {
    let path = Path::new(file);
    if file.is_empty() || path.is_absolute() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "invalid config file path",
        ));
    }
    let unsafe_component = path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    });
    if unsafe_component {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "invalid config file path",
        ));
    }
    Ok(())
}

fn temp_path(path: &Path) -> std::io::Result<PathBuf> {
    let file_name = path
        .file_name()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "invalid config file path"))?;
    let mut tmp_name = OsString::from(file_name);
    tmp_name.push(".tmp");
    Ok(path.with_file_name(tmp_name))
}

fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "op-config-store-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    #[test]
    fn store_at_round_trips_json_and_missing_is_none() {
        let root = temp_root("roundtrip");
        let store = ConfigStore::at(&root);

        let missing: Option<serde_json::Value> = store.read_json("settings.json").unwrap();
        assert_eq!(missing, None);

        store
            .write_json("settings.json", &json!({ "theme": "dark", "zoom": 1.25 }))
            .unwrap();
        let loaded: Option<serde_json::Value> = store.read_json("settings.json").unwrap();

        assert_eq!(loaded, Some(json!({ "theme": "dark", "zoom": 1.25 })));
        assert!(root.join("settings.json").is_file());
        assert!(!root.join("settings.json.tmp").exists());
    }

    #[test]
    fn write_json_serializes_before_replacing_existing_file() {
        #[derive(Debug)]
        struct FailingSerialize;

        impl Serialize for FailingSerialize {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(serde::ser::Error::custom("intentional failure"))
            }
        }

        let root = temp_root("atomic");
        let store = ConfigStore::at(&root);
        store
            .write_json("state.json", &json!({ "version": 1 }))
            .unwrap();

        let err = store
            .write_json("state.json", &FailingSerialize)
            .unwrap_err();
        let after: Option<serde_json::Value> = store.read_json("state.json").unwrap();

        assert_eq!(after, Some(json!({ "version": 1 })));
        assert!(!root.join("state.json.tmp").exists());
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn store_rejects_absolute_or_parent_paths() {
        let root = temp_root("paths");
        let store = ConfigStore::at(&root);

        assert!(store.read_json::<serde_json::Value>("../bad.json").is_err());
        assert!(store.write_json("/tmp/bad.json", &json!({})).is_err());
    }

    #[test]
    fn user_root_redirect_covers_the_process_level_helpers_and_wins_once() {
        let root = temp_root("redirect");
        assert_eq!(redirect_user_root_for_tests(&root), root);
        assert_eq!(ConfigStore::user().unwrap().root(), root);
        assert_eq!(openpencil_dir().unwrap(), root);

        // The free helpers are what callers actually reach for, and they
        // must land in the redirected root, not the real home directory.
        write_json("probe.json", &json!({ "ok": true })).unwrap();
        assert!(root.join("probe.json").is_file());
        let back: Option<serde_json::Value> = read_json("probe.json").unwrap();
        assert_eq!(back, Some(json!({ "ok": true })));
        assert_ne!(root, home_dir().unwrap().join(".openpencil"));

        // Idempotent: a second install does not move the root, so tests
        // may all call it in any order.
        assert_eq!(
            redirect_user_root_for_tests(temp_root("redirect-again")),
            root
        );
    }

    #[test]
    fn well_known_files_preserve_existing_names() {
        assert_eq!(well_known::GIT_AUTH, "git-auth.json");
        assert_eq!(well_known::LIVE_MCP_PORT, ".op-mcp-port");
        assert_eq!(well_known::CLI_SESSION, "cli-session.op");
    }

    #[test]
    fn sandbox_root_configuration_is_idempotent_and_rejects_identity_drift() {
        let slot = OnceLock::new();
        let first = temp_root("sandbox-first");
        let second = temp_root("sandbox-second");

        assert_eq!(
            configure_user_root_in(&slot, first.clone()).unwrap(),
            first.canonicalize().unwrap()
        );
        assert_eq!(
            configure_user_root_in(&slot, first.clone()).unwrap(),
            first.canonicalize().unwrap()
        );
        let error = configure_user_root_in(&slot, second).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::AlreadyExists);
        assert_eq!(slot.get(), Some(&first.canonicalize().unwrap()));
    }

    #[test]
    fn explicit_sandbox_root_requires_an_absolute_real_directory() {
        let relative = prepare_explicit_user_root(PathBuf::from("relative/root")).unwrap_err();
        assert_eq!(relative.kind(), ErrorKind::InvalidInput);

        let root = temp_root("sandbox-private");
        let prepared = prepare_explicit_user_root(root.clone()).unwrap();
        assert_eq!(prepared, root.canonicalize().unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn rejected_late_root_is_not_created_or_permission_mutated() {
        let slot = OnceLock::new();
        let first = temp_root("late-first");
        configure_user_root_in(&slot, first).unwrap();
        let absent = std::env::temp_dir().join(format!(
            "op-config-store-late-absent-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&absent);
        assert!(!absent.exists());

        let result = configure_user_root_in(&slot, absent.clone());
        assert_eq!(result.unwrap_err().kind(), ErrorKind::AlreadyExists);
        assert!(!absent.exists());
    }

    #[cfg(unix)]
    #[test]
    fn explicit_sandbox_root_rejects_a_symlink() {
        use std::os::unix::fs::symlink;

        let parent = temp_root("sandbox-symlink");
        let real = parent.join("real");
        let link = parent.join("link");
        std::fs::create_dir(&real).unwrap();
        symlink(&real, &link).unwrap();

        let error = prepare_explicit_user_root(link).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
}
