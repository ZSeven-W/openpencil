//! Rebase only app-owned iOS document references after a container upgrade.

use crate::error::FfiResult;
use crate::lifecycle::Session;
use std::path::{Component, Path, PathBuf};

pub(crate) fn initialize_documents(session: &mut Session) -> FfiResult<usize> {
    let migration = super::migrate_legacy_documents(&session.document_save);
    if let (Some(root), Some(host)) = (&session.document_save.root, session.editor.as_mut()) {
        let mut changed = false;
        for recent in &mut host.editor_state_mut().editor_ui.recent_files {
            if let Some(path) = relocated_owned_file(Path::new(&recent.path), root) {
                recent.path = path.to_string_lossy().into_owned();
                changed = true;
            }
        }
        if changed {
            host.mark_editor_state_dirty();
            if crate::lifecycle::settings_persistence_active() {
                op_editor_host_core::settings_io::save(host.editor_state());
            }
        }
    }
    migration
}

fn relocated_owned_file(old: &Path, documents_root: &Path) -> Option<PathBuf> {
    // Both paths must name iOS Data/Application/<UUID>/Documents roots in
    // the same container tree. A coincidentally named external Saved folder
    // is never interpreted as an app-owned destination.
    if !documents_root.is_absolute() || documents_root.file_name()? != "Documents" {
        return None;
    }
    let container = documents_root.parent()?;
    if !is_uuid(container.file_name()?.to_str()?) {
        return None;
    }
    let application = container.parent()?;
    if application.file_name()? != "Application"
        || application.parent()?.file_name()? != "Data"
        || application.parent()?.parent()?.file_name()? != "Containers"
    {
        return None;
    }
    let parts: Vec<_> = old.strip_prefix(application).ok()?.components().collect();
    let [Component::Normal(old_id), Component::Normal(documents), Component::Normal(kind), Component::Normal(name)] =
        parts.as_slice()
    else {
        return None;
    };
    if !is_uuid(old_id.to_str()?)
        || *documents != "Documents"
        || !matches!(kind.to_str()?, "Saved" | "Autosave")
        || !Path::new(name).extension()?.eq_ignore_ascii_case("op")
    {
        return None;
    }
    let current = documents_root.join(kind).join(name);
    (current != old && current.is_file()).then_some(current)
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_existing_app_owned_files_follow_a_relocated_container() {
        let base = std::env::temp_dir().join(format!("op-relocate-{}", std::process::id()));
        let apps = base.join("Containers/Data/Application");
        let old = apps.join("11111111-1111-1111-1111-111111111111/Documents");
        let current = apps.join("22222222-2222-2222-2222-222222222222/Documents");
        for kind in ["Saved", "Autosave"] {
            std::fs::create_dir_all(current.join(kind)).unwrap();
            let relative = Path::new(kind).join("coffee.op");
            std::fs::write(current.join(&relative), b"saved work").unwrap();
            assert_eq!(
                relocated_owned_file(&old.join(&relative), &current),
                Some(current.join(relative))
            );
        }
        for path in [
            old.join("Saved/missing.op"),
            old.join("coffee.op"),
            base.join("external/Documents/Saved/coffee.op"),
            old.join("Saved/../coffee.op"),
            current.join("Saved/coffee.op"),
        ] {
            assert_eq!(
                relocated_owned_file(&path, &current),
                None,
                "{}",
                path.display()
            );
        }
        std::fs::remove_dir_all(base).unwrap();
    }
}
