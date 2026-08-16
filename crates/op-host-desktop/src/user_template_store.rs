//! `~/.openpencil/templates/` — the desktop's home for saved scene templates.
//!
//! The runtime registry (`op_editor_core::user_scene_templates`) is pure
//! memory. Native directory scanning is shared with headless MCP/web hosts in
//! `op-host-services`; this desktop residual owns the GUI save and delete
//! flows that write the three files.
//!
//! One directory per template — `<slug>/document.op`, `preview.jpg`,
//! `meta.json`. A template is a two-file asset, and the directory shape is
//! what localizes the half-written case: a directory missing `document.op`
//! or `meta.json` is skipped as an incomplete save, while a flat listing
//! would have no consistent answer for a template that has a document but no
//! preview.
//!
//! The directory name is the id. `user:studio-ochre` lives in
//! `studio-ochre/`, which is what lets a boot scan re-derive the same ids a
//! previous session saved against, and what lets a delete find the directory
//! from the id alone.

use std::path::PathBuf;

use op_editor_core::editor_toast::EditorToastLevel;
#[cfg(test)]
use op_editor_core::user_scene_templates::UserSceneTemplate;
use op_editor_core::user_scene_templates::{user_scene_templates, USER_TEMPLATE_ID_PREFIX};
use op_host_native::widget_host::WidgetHostNative;

/// Resolve the templates directory, creating it if this is the first save.
fn templates_dir() -> std::io::Result<PathBuf> {
    crate::test_config_root::guard_user_config();
    op_host_services::user_scene_template_store::user_scene_templates_dir()
}

/// Load the templates directory into the runtime registry, once per process.
///
/// The registry is process-global, so scanning it is boot work, not
/// per-document work — and `app_state` is built more than once in a process.
/// Rescanning on each build would also discard any template saved since boot
/// by replacing the whole registry with what happens to be on disk.
pub(crate) fn load_user_scene_templates_once() {
    crate::test_config_root::guard_user_config();
    op_host_services::user_scene_template_store::initialize_user_scene_templates_once();
}

/// Load every template directory into the runtime registry.
///
/// A directory without a readable `meta.json` or `document.op` is skipped
/// with a note rather than failing the scan: the directory is user-editable,
/// and one incomplete save must not cost the rest of the collection. A
/// missing `preview.jpg` is tolerated — the card then paints a plain block.
pub(crate) fn load_user_scene_templates() {
    crate::test_config_root::guard_user_config();
    if let Err(error) = op_host_services::user_scene_template_store::reload_user_scene_templates() {
        eprintln!("[templates] could not scan saved templates: {error}");
    }
}

/// Write a saved template's three files, by id.
///
/// The template is already in the runtime registry — the save flow put it
/// there so the card appears the instant the user confirms — so this only
/// has to make it survive a restart. The document bytes are the registry's
/// canonical string, so disk and memory can never disagree about what a
/// click will rebuild. Atomicity of the individual writes is deliberately
/// not enforced: a half-written directory is skipped by the next boot scan,
/// which is the same guarantee a rename dance would buy.
pub(crate) fn persist_user_scene_template(id: &str) -> std::io::Result<PathBuf> {
    let template = user_scene_templates()
        .into_iter()
        .find(|template| template.id == id)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no saved scene template with id {id}"),
            )
        })?;
    let dir = templates_dir()?.join(template.slug());
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("document.op"), template.document.as_bytes())?;
    std::fs::write(dir.join("preview.jpg"), &template.preview_jpeg)?;
    std::fs::write(
        dir.join("meta.json"),
        op_host_services::user_scene_template_store::encode_user_scene_template_meta(&template)?,
    )?;
    Ok(dir)
}

/// Delete a saved template's directory. A missing directory is success — the
/// user asked for it gone, and it is.
pub(crate) fn delete_user_scene_template(id: &str) -> std::io::Result<()> {
    let slug = id.strip_prefix(USER_TEMPLATE_ID_PREFIX).unwrap_or(id);
    // The id is derived from a directory name we wrote, but it arrives here
    // through editor state, so it is re-validated rather than trusted: a slug
    // that survives `slugify` unchanged cannot contain a separator or `..`,
    // which is what keeps a delete inside the templates directory.
    if slug.is_empty() || op_ai_skills::style_guide::slugify(slug) != slug {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("refusing to delete an unsafe template id: {id}"),
        ));
    }
    let path = templates_dir()?.join(slug);
    match std::fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Remove the directories of templates the Asset Center already forgot.
///
/// The panel removes from memory first. If the disk delete fails, reload the
/// registry from disk so the still-present template reappears and surface the
/// failure. The user can explicitly retry from the restored card.
pub(crate) fn drain_pending_template_delete(host: &mut WidgetHostNative, now_ms: u64) -> bool {
    drain_pending_template_delete_with(
        host,
        now_ms,
        delete_user_scene_template,
        load_user_scene_templates,
    )
}

fn drain_pending_template_delete_with(
    host: &mut WidgetHostNative,
    now_ms: u64,
    mut delete: impl FnMut(&str) -> std::io::Result<()>,
    mut reload: impl FnMut(),
) -> bool {
    let ids = host
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .take_pending_template_delete();
    if ids.is_empty() {
        return false;
    }
    let mut delete_failed = false;
    let mut registry_changed = false;
    for id in ids {
        match delete(&id) {
            Ok(()) => {
                // The first attempt already removed this entry in the shared
                // press flow. A retry may be deleting an entry restored after
                // an earlier disk failure, so make memory match disk again.
                registry_changed |=
                    op_editor_core::user_scene_templates::remove_user_scene_template(&id).is_some();
            }
            Err(error) => {
                eprintln!("[templates] {id}: could not be deleted: {error}");
                delete_failed = true;
            }
        }
    }
    if delete_failed {
        // The directory still exists, so restore the card. Do not immediately
        // requeue: redraws also drain this function, and a persistent
        // permission error must not become an infinite retry/present loop.
        reload();
        host.editor_state_mut().editor_ui.show_toast(
            "sceneTemplate.deleteFailed",
            Vec::new(),
            EditorToastLevel::Warn,
            now_ms,
        );
        registry_changed = true;
    }
    if registry_changed {
        host.mark_editor_state_dirty();
    }
    registry_changed
}

#[cfg(test)]
#[path = "user_template_store_tests.rs"]
mod user_template_store_tests;
