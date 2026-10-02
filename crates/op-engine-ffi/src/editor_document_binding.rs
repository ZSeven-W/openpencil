//! Restore picker-owned destinations from the app's private save-target registry.

use super::{
    documents_dir, sibling_temp_path, DocumentBinding, DocumentSaveShellState, ShellBinding,
};
use crate::error::{FfiError, FfiResult};
use crate::OpStatus;
use std::path::{Path, PathBuf};

fn metadata_path(save: &DocumentSaveShellState, path: &Path) -> FfiResult<PathBuf> {
    let root = op_config_store::configured_user_root()
        .map(|root| root.join("save-targets"))
        .unwrap_or(documents_dir(save)?.join(".save-targets"));
    let kind = path
        .parent()
        .and_then(Path::file_name)
        .ok_or_else(|| FfiError::invalid("saved document has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| FfiError::invalid("saved document has no name"))?;
    Ok(root.join(kind).join(name).with_extension("op.json"))
}

pub(super) fn remember(
    save: &DocumentSaveShellState,
    path: &Path,
    handle: &str,
    display_name: &str,
) -> FfiResult<()> {
    let target = metadata_path(save, path)?;
    std::fs::create_dir_all(target.parent().expect("metadata has a parent")).map_err(|e| {
        FfiError::new(
            OpStatus::NotReady,
            format!("could not retain the save destination: {e}"),
        )
    })?;
    let temp = sibling_temp_path(&target);
    let bytes = serde_json::to_vec(&serde_json::json!({
        "version": 1, "handle": handle, "display_name": display_name,
    }))
    .expect("save target metadata serializes");
    let result = std::fs::write(&temp, bytes).and_then(|_| std::fs::rename(&temp, &target));
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(FfiError::new(
            OpStatus::NotReady,
            format!("could not retain the save destination: {error}"),
        ));
    }
    Ok(())
}

pub(super) fn restore(save: &DocumentSaveShellState, path: &Path) -> DocumentBinding {
    let owned = documents_dir(save).ok().is_some_and(|root| {
        path.parent() == Some(root.join("Saved").as_path())
            || path.parent() == Some(root.join("Autosave").as_path())
    });
    if !save.picker || !owned {
        return DocumentBinding::Path(path.to_path_buf());
    }
    let value = metadata_path(save, path)
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    if let Some(value) = value {
        if value["version"].as_u64() == Some(1) {
            if let (Some(handle), Some(name)) =
                (value["handle"].as_str(), value["display_name"].as_str())
            {
                if !handle.trim().is_empty()
                    && !name.trim().is_empty()
                    && !name.contains(['/', '\\'])
                {
                    return DocumentBinding::Shell(ShellBinding {
                        handle: handle.into(),
                        display_name: name.into(),
                        recent_copy: Some(path.to_path_buf()),
                    });
                }
            }
        }
    }
    // Legacy or damaged metadata cannot turn a private recovery file into
    // the user's original destination. Ask the picker instead.
    DocumentBinding::None
}
