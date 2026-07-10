//! Per-frame layer-row model cache.
//!
//! `LayerPanel::from_editor` re-walks the whole active-page `PenNode`
//! tree and allocates ~4 `String`s per node EVERY frame, then
//! `layers_content_width` measures every label. Paint was already
//! virtualized (`visible_row_range`), but the DATA build was not — so
//! caret-blink / chat-streaming / cursor-coalescing repaints, which do
//! not touch the layer tree, still paid the full walk + measure.
//!
//! This caches the built row model (`pages` + `items` + both content
//! widths) in a thread-local single slot, keyed on the CHEAP inputs
//! that actually change the row set / labels:
//!
//! * `document_revision` — the monotonic content-revision counter on
//!   `EditorState`; ANY node id / name / kind / visibility / lock /
//!   children edit bumps it, so it stands in for a value-hash of the
//!   whole tree (page names live on the doc too, so they ride it).
//! * `active_page_index` — selects which page's children walk and which
//!   page row reads `active`.
//! * a fingerprint of the collapsed-layer set — collapse is stored on
//!   `editor_ui` (NOT the doc, so it never bumps the revision) and it
//!   changes BOTH the `collapsed` flag AND the flattened row set (a
//!   collapsed container hides its subtree).
//! * a fingerprint of the inline-rename draft — it rewrites the target
//!   row's label + `renaming` flag.
//!
//! ## Why selection / hover are DELIBERATELY absent from the key
//!
//! Selection and hover (`selection`, `hovered_layer_id`,
//! `hovered_page_index`) only tint / reveal affordances — they change no
//! label, depth, order, or content width. They are therefore stripped
//! from the cached `LayerItem` / `PageItem` entirely and applied by the
//! panel as a LIVE OVERLAY at paint / hit-test time (see
//! `LayerPanel::is_row_selected` / `is_row_hovered` / `is_page_hovered`).
//! Keeping them out of the key is what lets a selection-only or
//! hover-only change still hit the cache — the common case while the
//! user clicks around or sweeps the mouse over rows.
//!
//! ## Owner scoping (why, and how it differs from the transcript cache)
//!
//! The key uses a PER-DOCUMENT revision counter, not a global value
//! hash: two independent `EditorState`s each start at revision 0, so
//! host A (revision 3) and host B (revision 3) can carry DIFFERENT trees
//! under an IDENTICAL key. A single process-wide thread-local slot must
//! therefore never serve host B's rows to host A. Every persistent host
//! pulls ONE opaque `owner` id ([`next_owner`]) and passes it to the hot
//! (paint) resolve; the slot is served ONLY when `slot_owner == owner`
//! AND the key matches. Unlike the transcript cache — whose full
//! value-hash key lets it serve on key-match regardless of owner — this
//! cache makes owner part of the READ predicate.
//!
//! Owner ROTATION: page switches do NOT rotate — `active_page_index` is in
//! the key, so a switch invalidates via the key itself. WHOLE-DOCUMENT
//! replacements MUST rotate (hosts call `force_rotate_layer_panel_owner()`
//! at every open / new / import / live-sync / MCP-replace seam): a fresh
//! document restarts the revision counter at 0, so without rotation the
//! (revision, page) key can alias the previous document's rows. Beyond
//! that, the owner keeps two different panels (two hosts, or a host and a
//! unit test) that share a worker thread from cross-serving on a
//! coincidental revision-counter collision.
//!
//! [`UNOWNED`] (`0`) is the cache-BYPASS sentinel: `from_editor`
//! (unit tests + non-hot host call sites like click / scroll / geometry
//! / a11y) resolves under it and ALWAYS gets a fresh build that never
//! reads or writes the shared slot. That keeps those paths trivially
//! correct while the per-frame paint path (owned) reaps the cache.
//!
//! Precedent: `ai_chat_transcript_cache.rs` (the owner-scoped
//! thread-local slot pattern) and `op-pen-loader/src/measure_cache.rs`.

use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use op_editor_core::ui_draft::LayerContextTarget;
use op_editor_core::EditorState;

use super::layer_panel::{LayerItem, PageItem};

/// Process-global allocator of opaque per-panel-instance owner ids. Each
/// persistent host pulls ONE id at construction and passes it to the hot
/// paint resolve, so the single thread-local slot is scoped to whichever
/// panel last owned it. `0` is the [`UNOWNED`] bypass sentinel, so the
/// counter starts at `1` and never hands it out.
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

/// Cache-bypass sentinel. A resolve under this owner ALWAYS builds fresh
/// and never touches the shared slot — used by `from_editor` (unit tests
/// + non-per-frame host call sites).
pub(crate) const UNOWNED: u64 = 0;

/// Allocate a fresh, process-unique panel-owner id. Hosts allocate one at
/// construction and REALLOCATE after every whole-document replacement
/// (`force_rotate_layer_panel_owner`); an id is stable only between
/// replacements.
pub(crate) fn next_owner() -> u64 {
    NEXT_OWNER.fetch_add(1, Ordering::Relaxed)
}

/// The cached, styling-neutral row model. `pages` / `items` are held
/// behind `Rc` so a cache hit clones them with two refcount bumps and
/// zero per-row `String` allocation. Selection / hover are NOT baked in
/// (see the module docs) — the panel overlays them live.
pub(crate) struct CachedLayerRows {
    pub pages: Rc<Vec<PageItem>>,
    pub items: Rc<Vec<LayerItem>>,
    pub pages_content_width: f32,
    pub layers_content_width: f32,
}

/// Identity of a cached build. Selection / hover are intentionally
/// absent (they are a live paint-time overlay, never a rebuild trigger).
#[derive(PartialEq)]
struct LayerRowKey {
    revision: u64,
    active_page_index: usize,
    collapsed_fp: u64,
    rename_fp: u64,
}

impl LayerRowKey {
    fn new(state: &EditorState) -> Self {
        Self {
            revision: state.document_revision(),
            active_page_index: state.ui.active_page_index,
            collapsed_fp: collapsed_fingerprint(state),
            rename_fp: rename_fingerprint(state),
        }
    }
}

/// Order-independent fingerprint of the collapsed-layer set. `HashSet`
/// iteration order is unspecified, so the per-element hashes are combined
/// commutatively (wrapping add) and mixed with the set length; distinct
/// ids produce distinct 64-bit hashes, so this identifies the set.
fn collapsed_fingerprint(state: &EditorState) -> u64 {
    let collapsed = &state.editor_ui.collapsed_layers;
    let mut acc: u64 = 0;
    for id in collapsed {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        acc = acc.wrapping_add(h.finish());
    }
    let mut h = DefaultHasher::new();
    collapsed.len().hash(&mut h);
    acc.hash(&mut h);
    h.finish()
}

/// Fingerprint of the inline-rename draft — its target plus the current
/// draft text (which becomes the target row's label). The caret /
/// selection inside the input are NOT hashed: they never change the row
/// model (the panel clones the live `TextInputState` for the rename
/// caret every frame, outside the cache).
fn rename_fingerprint(state: &EditorState) -> u64 {
    let mut h = DefaultHasher::new();
    match state.ui.layer_rename.as_ref() {
        None => 0u8.hash(&mut h),
        Some(rename) => {
            1u8.hash(&mut h);
            match &rename.target {
                LayerContextTarget::Layer(id) => {
                    0u8.hash(&mut h);
                    id.hash(&mut h);
                }
                LayerContextTarget::Page(i) => {
                    1u8.hash(&mut h);
                    i.hash(&mut h);
                }
            }
            rename.input.text().hash(&mut h);
        }
    }
    h.finish()
}

thread_local! {
    /// The single row-model slot, tagged with the `owner` of whichever
    /// panel last resolved it. Served only to that same owner on a key
    /// match; every other resolve rebuilds (and re-stamps the slot).
    static CACHE: RefCell<Option<(u64, LayerRowKey, Rc<CachedLayerRows>)>> =
        const { RefCell::new(None) };
    /// Observable rebuild counter — increments only when a fresh build
    /// runs. Lets tests prove a cache hit does not recompute.
    static BUILD_COUNT: Cell<u64> = const { Cell::new(0) };
}

/// Owner-scoped resolve. When `owner` is [`UNOWNED`] the shared slot is
/// bypassed entirely (always a fresh build, no read, no write). For a
/// real owner the slot is served ONLY when both the owner AND the key
/// match; otherwise `build` runs and its result is stored under `owner`.
/// `build` runs while no cache borrow is held.
pub(crate) fn resolve_owned(
    owner: u64,
    state: &EditorState,
    build: impl FnOnce() -> CachedLayerRows,
) -> Rc<CachedLayerRows> {
    if owner == UNOWNED {
        // Bypass: a fresh build that never reads or writes the shared
        // slot, so it can never cross-pair with an owned panel's rows.
        return Rc::new(build());
    }
    let key = LayerRowKey::new(state);
    // Fast path: same owner AND same key → clone the Rc out (the borrow
    // is dropped before `build` could ever run).
    let hit = CACHE.with(|cell| {
        cell.borrow()
            .as_ref()
            .and_then(|(slot_owner, slot_key, rows)| {
                (*slot_owner == owner && *slot_key == key).then(|| rows.clone())
            })
    });
    if let Some(rows) = hit {
        return rows;
    }
    let built = Rc::new(build());
    BUILD_COUNT.with(|c| c.set(c.get() + 1));
    CACHE.with(|cell| {
        *cell.borrow_mut() = Some((owner, key, built.clone()));
    });
    built
}

/// Number of fresh row-model builds performed so far on this thread — a
/// monotonic counter used by tests to assert cache hits do not recompute.
#[cfg(test)]
pub(crate) fn layer_row_build_count() -> u64 {
    BUILD_COUNT.with(Cell::get)
}
