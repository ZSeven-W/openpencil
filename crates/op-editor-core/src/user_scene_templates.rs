//! The runtime half of the scene-template catalogue: templates the user saved.
//!
//! The shipped catalogue is baked into the binary and never changes, so it can
//! be a `&'static` slice. A template saved while the editor is running can be
//! deleted while it is running, and on the web it has nowhere to be written
//! down at all — so it lives here instead, behind a process-global lock, the
//! same shape the imported style guides use
//! (`op_ai_skills::style_guide::user_registry`).
//!
//! Two sources, one lookup. Every consumer that used to reach straight for
//! the shipped catalogue now asks the right half for the id it was handed:
//! ids carrying the `user:` prefix resolve here, bare ids resolve to the
//! embedded corpus, and the two can never collide.
//!
//! This module holds no files. Persistence is a host capability (the desktop
//! writes `~/.openpencil/templates/<slug>/`; the browser has no disk), so
//! hosts call [`set_user_scene_templates`] on boot and
//! [`load_user_scene_template`] on save, and this stays a pure in-memory
//! catalogue that compiles for wasm32 unchanged.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Prefix separating saved-template ids from the shipped catalogue's bare ids.
///
/// Shipped templates are addressed by their bare id, so a saved template
/// whose file was called `slide-deck` would otherwise silently shadow (or be
/// shadowed by) the shipped template of that name depending on lookup order.
pub const USER_TEMPLATE_ID_PREFIX: &str = "user:";

/// How many saved templates one user may hold.
///
/// Documents are loaded into memory eagerly (about 100 KB each, so the cap
/// bounds the registry at ~5 MB), and the card-grid hover-token bands index
/// into the list — 50 sits far below the nearest band edge.
pub const USER_TEMPLATE_QUOTA: usize = 50;

/// A scene template the user saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSceneTemplate {
    /// Stable id, `user:<slug>`. This is what the Asset Center requests and
    /// what the host derives the on-disk directory name from.
    pub id: String,
    /// Display name, verbatim from the document's file stem. Deliberately not
    /// translated: it is the user's own word for their template.
    pub name: String,
    /// Top-level node count of the saved document's active page.
    pub frames: u16,
    /// First top-level node's resolved width / height, rounded.
    pub frame_width: u32,
    pub frame_height: u32,
    /// Canonical `.op` JSON — the same bytes the host wrote to disk, so a
    /// click can rebuild the document without touching the filesystem.
    pub document: String,
    /// JPEG preview of the first top-level node. Empty when the document had
    /// no top-level node to render; the card then paints a plain block.
    pub preview_jpeg: Vec<u8>,
}

impl UserSceneTemplate {
    /// The `<slug>` half of [`Self::id`] — the host's directory name.
    pub fn slug(&self) -> &str {
        self.id
            .strip_prefix(USER_TEMPLATE_ID_PREFIX)
            .unwrap_or(&self.id)
    }
}

/// Why a template could not be registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserSceneTemplateError {
    /// The quota is full and the id is new. Reported rather than overwriting:
    /// a save that silently replaced an existing template would be how a
    /// template the user kept disappears.
    QuotaExceeded,
}

type TemplateStore = Vec<Arc<UserSceneTemplate>>;

fn store() -> &'static RwLock<TemplateStore> {
    static STORE: RwLock<TemplateStore> = RwLock::new(Vec::new());
    &STORE
}

fn read_store() -> RwLockReadGuard<'static, TemplateStore> {
    store()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_store() -> RwLockWriteGuard<'static, TemplateStore> {
    store()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Every saved template, in save order.
///
/// A poisoned lock is recovered rather than propagated. The store's entries
/// remain valid after an unrelated writer panic, and every read/write path
/// follows the same recovery rule so a successful save is always queryable.
pub fn user_scene_templates() -> Vec<Arc<UserSceneTemplate>> {
    read_store().clone()
}

/// Replace the whole saved set — the host's boot-time scan.
///
/// Deliberately not quota-gated: a scan restores whatever is on disk, which
/// is the user's own collection, while the quota stays a save-time gate.
pub fn set_user_scene_templates(templates: Vec<UserSceneTemplate>) {
    *write_store() = templates.into_iter().map(Arc::new).collect();
}

/// Forget a saved template. Returns it, so the host knows which directory to
/// delete, or `None` when the id was already gone.
pub fn remove_user_scene_template(id: &str) -> Option<Arc<UserSceneTemplate>> {
    let mut store = write_store();
    let index = store.iter().position(|template| template.id == id)?;
    Some(store.remove(index))
}

/// Register a saved template, replacing an existing entry with the same id.
///
/// Replacing in place is what makes a boot-time re-scan idempotent instead of
/// doubling every card. A NEW id is refused once the quota is full — see
/// [`UserSceneTemplateError::QuotaExceeded`].
pub fn load_user_scene_template(
    template: UserSceneTemplate,
) -> Result<Arc<UserSceneTemplate>, UserSceneTemplateError> {
    let entry = Arc::new(template);
    let mut store = write_store();
    match store.iter().position(|held| held.id == entry.id) {
        Some(index) => store[index] = Arc::clone(&entry),
        None if store.len() < USER_TEMPLATE_QUOTA => store.push(Arc::clone(&entry)),
        None => return Err(UserSceneTemplateError::QuotaExceeded),
    }
    Ok(entry)
}

/// `user:<slug>`, with `-2`, `-3`, … appended until it is free.
///
/// Saving two documents that share a file stem is ordinary — a user keeping
/// variants of one design does it on the first afternoon — so a collision
/// numbers the newcomer rather than overwriting what is already saved.
pub fn allocate_template_id(slug: &str) -> String {
    let taken = |candidate: &str| read_store().iter().any(|template| template.id == candidate);
    let base = format!("{USER_TEMPLATE_ID_PREFIX}{slug}");
    if !taken(&base) {
        return base;
    }
    for suffix in 2..1000 {
        let candidate = format!("{base}-{suffix}");
        if !taken(&candidate) {
            return candidate;
        }
    }
    format!("{base}-{}", user_scene_templates().len() + 1)
}

/// Serialized, emptied access to the process-global saved-template store.
///
/// One lock for the whole crate, deliberately. Every test module that touches
/// the registry shares this store, and giving each its own mutex — which is
/// what the first version of the style registry did — lets two modules run
/// concurrently and empty the registry out from under each other.
#[cfg(test)]
pub(crate) fn exclusive_registry_for_tests() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    set_user_scene_templates(Vec::new());
    guard
}

#[cfg(test)]
#[path = "user_scene_templates_tests.rs"]
mod user_scene_templates_tests;
