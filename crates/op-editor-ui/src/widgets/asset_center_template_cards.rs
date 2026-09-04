//! The Asset Center's "My templates": saved templates → card view models.
//!
//! Mirror of `asset_center_style_cards`: the shipped catalogue is baked into
//! the binary; the user's saved templates arrive at runtime from
//! `op_editor_core::user_scene_templates`. The panel merges them here, saved
//! first, because a list where your own material sits below sixty shipped
//! entries is a list you scroll past your own work in. Ids keep them apart —
//! a saved template is addressed by its `user:` id and can never collide
//! with a shipped bare id.

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use op_editor_core::user_scene_templates::{user_scene_templates, UserSceneTemplate};

use super::scene_template_previews::USER_PREVIEW_CACHE_ID_BASE;

/// One saved-template card.
pub struct UserTemplateCard {
    /// Stable renderer cache id for this exact registry allocation.
    ///
    /// The registry replaces an entry with a fresh [`Arc`] when its preview
    /// changes. Keeping the id on allocation identity therefore gives the
    /// renderer both properties it needs: reordering/searching preserves a
    /// template's cache slot, while a replacement can never inherit a stale
    /// raster from the previous preview.
    pub image_id: u64,
    /// Share the registry allocation instead of cloning its document strings
    /// and 1024 px JPEG on every hover, layout, and paint pass.
    template: Arc<UserSceneTemplate>,
}

impl Deref for UserTemplateCard {
    type Target = UserSceneTemplate;

    fn deref(&self) -> &Self::Target {
        self.template.as_ref()
    }
}

#[derive(Default)]
struct UserPreviewIdRegistry {
    next: u64,
    /// Allocation address -> weak owner + renderer id. Holding a `Weak` keeps
    /// the allocation address reserved until the entry is pruned, so an
    /// allocator cannot recycle an old address while its raster is cached.
    by_allocation: HashMap<usize, (Weak<UserSceneTemplate>, u64)>,
}

fn user_preview_ids() -> &'static Mutex<UserPreviewIdRegistry> {
    static IDS: OnceLock<Mutex<UserPreviewIdRegistry>> = OnceLock::new();
    IDS.get_or_init(|| Mutex::new(UserPreviewIdRegistry::default()))
}

/// Renderer id for one immutable registry allocation.
///
/// A monotonic id is preferable to hashing the JPEG on this hot path: it is
/// collision-free for the process, and the `Arc` already is the registry's
/// immutable generation marker. Dead weak entries are pruned only on a miss,
/// keeping steady-state lookups O(1) without growing forever across deletes.
fn user_preview_image_id(template: &Arc<UserSceneTemplate>) -> u64 {
    let allocation = Arc::as_ptr(template) as usize;
    let mut ids = user_preview_ids()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((owner, image_id)) = ids.by_allocation.get(&allocation) {
        if owner
            .upgrade()
            .is_some_and(|held| Arc::ptr_eq(&held, template))
        {
            return *image_id;
        }
    }

    ids.by_allocation
        .retain(|_, (owner, _)| owner.strong_count() > 0);
    let offset = ids.next;
    ids.next = ids.next.wrapping_add(1);
    let image_id = USER_PREVIEW_CACHE_ID_BASE.wrapping_add(offset);
    ids.by_allocation
        .insert(allocation, (Arc::downgrade(template), image_id));
    image_id
}

/// Every saved template as a card, in registry order, minus the ones the
/// search query does not match.
///
/// Search matches the name only: a saved template has no tags or scene to
/// search, and the name is what its author chose to call it.
pub fn user_template_cards(query: &str) -> Vec<UserTemplateCard> {
    let query = query.trim().to_lowercase();
    user_scene_templates()
        .into_iter()
        // Filter before building the view model. A narrow search should not
        // even allocate renderer ids for the cards it does not show.
        .filter(|template| template_matches_query(template, &query))
        .map(|template| UserTemplateCard {
            image_id: user_preview_image_id(&template),
            template,
        })
        .collect()
}

/// Number of saved cards matching `query`, without materialising card view
/// models or touching their renderer ids / JPEG payloads.
pub fn user_template_card_count(query: &str) -> usize {
    let query = query.trim().to_lowercase();
    user_scene_templates()
        .iter()
        .filter(|template| template_matches_query(template, &query))
        .count()
}

fn template_matches_query(template: &UserSceneTemplate, query: &str) -> bool {
    query.is_empty() || template.name.to_lowercase().contains(query)
}

/// Serialized access to the process-global saved-template registry.
///
/// Every Asset Center test reads that registry, and a few write it. Cargo
/// runs them on parallel threads of one process, so without a lock a test
/// that saves a template changes what an unrelated test's grid contains.
/// Mirrors `asset_center_style_cards::style_test_support`.
#[cfg(test)]
pub(crate) mod template_test_support {
    use std::sync::{Mutex, MutexGuard};

    /// Registry lock that restores the empty baseline before releasing it.
    ///
    /// Clearing only on entry leaves the final writer's templates alive for
    /// any later grid test that does not need to mutate the registry. Drop
    /// while the mutex is still held so no other guarded test can observe that
    /// residue between clear and unlock.
    pub(crate) struct UserTemplatesGuard {
        _lock: MutexGuard<'static, ()>,
    }

    impl Drop for UserTemplatesGuard {
        fn drop(&mut self) {
            op_editor_core::user_scene_templates::set_user_scene_templates(Vec::new());
        }
    }

    pub(crate) fn exclusive_user_templates() -> UserTemplatesGuard {
        static LOCK: Mutex<()> = Mutex::new(());
        let lock = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        op_editor_core::user_scene_templates::set_user_scene_templates(Vec::new());
        UserTemplatesGuard { _lock: lock }
    }

    /// Save a minimal template named `name`, returning its id.
    pub(crate) fn save_template(name: &str) -> String {
        let id = op_editor_core::user_scene_templates::allocate_template_id(name);
        op_editor_core::user_scene_templates::load_user_scene_template(
            op_editor_core::user_scene_templates::UserSceneTemplate {
                id: id.clone(),
                name: name.to_string(),
                frames: 2,
                frame_width: 1920,
                frame_height: 1080,
                document: "{\"version\":\"1.0.0\",\"children\":[]}".to_string(),
                preview_jpeg: vec![0xFF, 0xD8, 0xFF, 0xD9],
            },
        )
        .expect("fixture fits the quota");
        id
    }
}

#[cfg(test)]
#[path = "asset_center_template_cards_tests.rs"]
mod asset_center_template_cards_tests;
