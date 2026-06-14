//! `~/.openpencil` file-backed config primitives.
//!
//! The crate centralizes the home-directory resolver plus JSON
//! read/write helpers used by OpenPencil native tools.

use serde::{de::DeserializeOwned, Serialize};
use std::ffi::OsString;
use std::io::{Error, ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

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

fn default_openpencil_dir() -> std::io::Result<PathBuf> {
    home_dir().map(|home| home.join(".openpencil"))
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
    fn well_known_files_preserve_existing_names() {
        assert_eq!(well_known::GIT_AUTH, "git-auth.json");
        assert_eq!(well_known::LIVE_MCP_PORT, ".op-mcp-port");
        assert_eq!(well_known::CLI_SESSION, "cli-session.op");
    }
}
