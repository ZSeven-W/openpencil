//! `op install --target cline`: expose the embedded skill bundle to Cline.
//!
//! Cline (the CLI and the VS Code extension alike) discovers global skills
//! exactly one level below `<cline-root>/skills`: `skills/<name>/SKILL.md`.
//! The Codex layout — one link to the bundle's whole `skills/` container —
//! would put `SKILL.md` two levels down and stay invisible, so every skill
//! in the bundle gets its own discovery entry pointing into a bundle copy
//! kept beside it at `<cline-root>/openpencil-skill`.
//!
//! Discovery entries share a directory with the user's own skills, so the
//! installer only ever replaces or removes an entry it can prove it owns:
//! a symlink into the bundle copy, or a copied fallback directory carrying
//! [`OWNER_MARKER`].

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::skill_install_cli::{
    link_or_copy_dir, remove_path, write_bundle_to, SkillBundle, SKILL_NAME,
};
use crate::skill_install_error::{FsAction, SkillInstallError};

type Result<T> = std::result::Result<T, SkillInstallError>;

/// Dropped into a discovery entry when symlinking failed and the skill was
/// copied instead, so a later install/uninstall can tell our copy apart from
/// a user-authored skill of the same name.
pub(crate) const OWNER_MARKER: &str = ".openpencil-skill-owned";

pub(crate) fn install(root: &Path, bundle: &SkillBundle) -> Result<()> {
    let names = skill_names(bundle);
    let bundle_dir = root.join(SKILL_NAME);
    let skills_dir = root.join("skills");

    // Refuse before writing anything if a user-owned entry is in the way.
    for name in &names {
        let link_path = skills_dir.join(name);
        if entry_exists(&link_path)? && !is_owned_entry(&link_path, &bundle_dir) {
            return Err(SkillInstallError::NotOwned(link_path));
        }
    }

    // The bundle copy is ours outright: rewrite it so files dropped from a
    // newer bundle do not linger.
    remove_path(&bundle_dir)?;
    write_bundle_to(&bundle_dir, bundle)?;

    fs::create_dir_all(&skills_dir)
        .map_err(|e| SkillInstallError::fs(FsAction::Create, &skills_dir, e))?;
    for name in &names {
        let link_path = skills_dir.join(name);
        let target = bundle_dir.join("skills").join(name);
        remove_path(&link_path)?;
        link_or_copy_dir(&target, &link_path)?;
        if !is_symlink(&link_path) {
            let marker = link_path.join(OWNER_MARKER);
            fs::write(&marker, SKILL_NAME)
                .map_err(|e| SkillInstallError::fs(FsAction::Write, &marker, e))?;
        }
    }
    prune_owned_entries(&skills_dir, &bundle_dir, &names)
}

pub(crate) fn uninstall(root: &Path) -> Result<()> {
    let bundle_dir = root.join(SKILL_NAME);
    let skills_dir = root.join("skills");
    prune_owned_entries(&skills_dir, &bundle_dir, &BTreeSet::new())?;
    remove_path(&bundle_dir)
}

/// Remove every owned discovery entry whose name is not in `keep`. Walks the
/// directory rather than the current bundle's names so entries left by an
/// older bundle with different skills are cleaned up too.
fn prune_owned_entries(
    skills_dir: &Path,
    bundle_dir: &Path,
    keep: &BTreeSet<String>,
) -> Result<()> {
    let entries = match fs::read_dir(skills_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(SkillInstallError::fs(FsAction::Read, skills_dir, e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| SkillInstallError::fs(FsAction::Read, skills_dir, e))?;
        let path = entry.path();
        let kept = entry
            .file_name()
            .to_str()
            .is_some_and(|name| keep.contains(name));
        if !kept && is_owned_entry(&path, bundle_dir) {
            remove_path(&path)?;
        }
    }
    Ok(())
}

/// Skill directory names in the bundle: the `<name>` of `skills/<name>/…`.
fn skill_names(bundle: &SkillBundle) -> BTreeSet<String> {
    bundle
        .files
        .iter()
        .filter_map(|(path, _)| {
            let mut parts = path.split('/');
            match (parts.next(), parts.next(), parts.next()) {
                (Some("skills"), Some(name), Some(_)) if !name.is_empty() => Some(name.to_string()),
                _ => None,
            }
        })
        .collect()
}

fn entry_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(SkillInstallError::fs(FsAction::Inspect, path, e)),
    }
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// True for a symlink resolving (textually — it may dangle) into
/// `bundle_dir`, or a real directory carrying [`OWNER_MARKER`].
fn is_owned_entry(path: &Path, bundle_dir: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() {
        return fs::read_link(path).is_ok_and(|target| {
            let target: PathBuf = if target.is_absolute() {
                target
            } else {
                path.parent().map(|p| p.join(&target)).unwrap_or(target)
            };
            target.starts_with(bundle_dir)
        });
    }
    metadata.is_dir() && path.join(OWNER_MARKER).is_file()
}

#[cfg(test)]
#[path = "skill_install_cline_tests.rs"]
mod tests;
