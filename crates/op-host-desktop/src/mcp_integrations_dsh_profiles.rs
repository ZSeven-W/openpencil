//! Profile-aware DSH integration and migration of the legacy home block.
//!
//! Home patches remain supported by DSH. Existing profiles are targeted
//! explicitly so the desktop profile receives its configuration directly;
//! before any profile exists, the supported home layer remains the fallback.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::mcp_config_error::McpConfigError;
use crate::mcp_config_io::{atomic_write, FileSnapshot};
use crate::mcp_integrations_dsh::{dsh_config_has_openpencil, read_dsh_patch, render_dsh_patch};

const PATCH_FILE: &str = "cordis.patch.yml";

pub(crate) fn dsh_root(home: &Path, override_root: Option<&OsStr>) -> PathBuf {
    override_root
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".dsh"))
}

/// DSH accepts any object-shaped package.json; dsh.profile is optional.
/// Shared node_modules and symlinks are never profile rewrite targets.
fn profile_patch_paths(root: &Path) -> Result<Vec<PathBuf>, McpConfigError> {
    discover_profile_patches(root, false)
}

fn discover_profile_patches(
    root: &Path,
    best_effort: bool,
) -> Result<Vec<PathBuf>, McpConfigError> {
    let profiles = root.join("profiles");
    match fs::symlink_metadata(&profiles) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(read_error(&profiles, error)),
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(McpConfigError::Read {
                path: profiles,
                message: "refusing to traverse a symbolic-link DSH profiles directory".into(),
            });
        }
        Ok(_) => {}
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(&profiles).map_err(|error| read_error(&profiles, error))? {
        match entry
            .map_err(|error| read_error(&profiles, error))
            .and_then(profile_patch_path)
        {
            Ok(Some(path)) => paths.push(path),
            Ok(None) => {}
            Err(_) if best_effort => {}
            Err(error) => return Err(error),
        }
    }
    paths.sort();
    Ok(paths)
}

fn profile_patch_path(entry: fs::DirEntry) -> Result<Option<PathBuf>, McpConfigError> {
    if entry.file_name() == "node_modules"
        || !entry
            .file_type()
            .map_err(|error| read_error(&entry.path(), error))?
            .is_dir()
    {
        return Ok(None);
    }
    let manifest = entry.path().join("package.json");
    match fs::symlink_metadata(&manifest) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(read_error(&manifest, error)),
        Ok(metadata) if !metadata.file_type().is_file() => return Ok(None),
        Ok(_) => {}
    }
    let source = fs::read_to_string(&manifest).map_err(|error| read_error(&manifest, error))?;
    Ok(serde_json::from_str::<serde_json::Value>(&source)
        .is_ok_and(|value| value.is_object())
        .then(|| entry.path().join(PATCH_FILE)))
}

fn read_error(path: &Path, error: io::Error) -> McpConfigError {
    McpConfigError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

pub(crate) fn dsh_root_has_openpencil(root: &Path) -> bool {
    dsh_config_has_openpencil(&root.join(PATCH_FILE))
        // An unrelated unreadable profile must not turn off a known integration.
        // Mutations still use strict discovery and validate every target.
        || discover_profile_patches(root, true)
            .unwrap_or_default()
            .iter()
            .any(|path| dsh_config_has_openpencil(path))
}

struct PlannedWrite {
    path: PathBuf,
    text: String,
    snapshot: FileSnapshot,
}

fn plan_write(
    path: PathBuf,
    enabled: bool,
    url: &str,
) -> Result<Option<PlannedWrite>, McpConfigError> {
    let existing = read_dsh_patch(&path)?;
    if !enabled && !path.exists() {
        return Ok(None);
    }
    let text = render_dsh_patch(&path, &existing, enabled, url)?;
    if text == existing {
        return Ok(None);
    }
    let snapshot = FileSnapshot::capture(&path)?;
    Ok(Some(PlannedWrite {
        path,
        text,
        snapshot,
    }))
}

fn apply_plan(
    plan: &[PlannedWrite],
    mut write: impl FnMut(&Path, &[u8]) -> Result<(), McpConfigError>,
) -> Result<(), McpConfigError> {
    for (index, item) in plan.iter().enumerate() {
        if let Err(cause) = write(&item.path, item.text.as_bytes()) {
            let failures: Vec<_> = plan[..=index]
                .iter()
                .rev()
                .filter_map(|item| item.snapshot.restore(&item.path).err())
                .collect();
            return Err(if failures.is_empty() {
                cause
            } else {
                McpConfigError::Rollback {
                    cause: Box::new(cause),
                    failures,
                }
            });
        }
    }
    Ok(())
}

pub(crate) fn set_dsh_enabled(
    root: &Path,
    enabled: bool,
    url: &str,
) -> Result<PathBuf, McpConfigError> {
    let home_patch = root.join(PATCH_FILE);
    let profiles = profile_patch_paths(root)?;
    let representative = profiles.first().unwrap_or(&home_patch).clone();
    let mut plan = Vec::new();
    // Remove our home insert first, but only after all changes validate:
    // DSH does not deduplicate inserts and HMR can observe each write.
    if let Some(change) = plan_write(home_patch, enabled && profiles.is_empty(), url)? {
        plan.push(change);
    }
    for path in profiles {
        if let Some(change) = plan_write(path, enabled, url)? {
            plan.push(change);
        }
    }
    apply_plan(&plan, atomic_write)?;
    Ok(representative)
}

#[cfg(test)]
#[path = "mcp_integrations_dsh_profile_tests.rs"]
mod tests;
