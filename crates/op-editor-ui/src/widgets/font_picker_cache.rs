//! Cross-frame memo for `property_panel_typography::font_picker_entries`.
//!
//! The searchable Typography font picker re-lowercases + substring-matches
//! EVERY bundled/system/imported family on every call while the dropdown is
//! open. `PropertyPanel` is rebuilt fresh from `EditorState` every frame (no
//! persistent host-owned instance survives across frames — the same
//! situation as the icon picker / component browser), and it calls
//! `font_picker_entries` from up to five different accessors in a single
//! frame (paint, action dispatch, hover tracking, outside-click contains,
//! max-scroll) — all with the SAME live inputs — plus again on every
//! subsequent repaint even when neither the family lists nor the search
//! query changed.
//!
//! `font_picker_entries` is a PURE function of `(imported_families,
//! system_families, query)` — no per-document identity leaks into its
//! result — so an identical triple always yields an identical result
//! regardless of which `EditorState` produced it. That is what lets this
//! cache skip owner-scoping entirely (unlike `layer_panel_cache`, whose
//! per-document revision counter CAN coincidentally collide across
//! independent documents at the same revision number): the key compares
//! `imported_families` / `system_families` by FULL VALUE equality (never a
//! pointer or a length shortcut), so a collision is impossible except when
//! the inputs are truly byte-identical — in which case serving the cached
//! result is correct by construction.
//!
//! One shared slot suffices (not a per-caller/per-owner set): every caller
//! within a frame reads the SAME live `imported_families` /
//! `system_families` / query off `EditorState`, so they all hit the same
//! slot.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::property_panel_typography::FontPickerEntry;

/// Identity of a cached [`resolve`] result — the two family lists plus the
/// normalized search query, all compared by value.
struct FontEntriesKey {
    imported: Vec<String>,
    system: Vec<String>,
    query: String,
}

thread_local! {
    static CACHE: RefCell<Option<(FontEntriesKey, Rc<Vec<FontPickerEntry>>)>> =
        const { RefCell::new(None) };
    /// Observable rebuild counter — increments only when a fresh
    /// filter pass runs. Lets tests prove a cache hit does not recompute.
    static BUILD_COUNT: Cell<u64> = const { Cell::new(0) };
}

/// Resolve the picker's entries, memoized on `(imported_families,
/// system_families, query)` by value. `build` runs only on a cache miss;
/// `query` must already be the normalized (trimmed + lowercased) search
/// string `build` was constructed against.
///
/// Returns the shared `Rc` directly on both the hit and the miss path —
/// a cache hit is a refcount bump (no per-`FontPickerEntry` clone, no
/// `Vec` reallocation), which is what makes memoizing worthwhile for the
/// common empty-query call: before this, a hit deep-cloned every owned
/// `family: String` in the result on every one of the panel's five
/// per-frame accessors.
pub(crate) fn resolve(
    imported_families: &[String],
    system_families: &[String],
    query: &str,
    build: impl FnOnce() -> Vec<FontPickerEntry>,
) -> Rc<Vec<FontPickerEntry>> {
    let hit = CACHE.with(|cell| {
        cell.borrow().as_ref().and_then(|(key, entries)| {
            (key.imported.as_slice() == imported_families
                && key.system.as_slice() == system_families
                && key.query == query)
                .then(|| entries.clone())
        })
    });
    if let Some(entries) = hit {
        return entries;
    }
    let built = Rc::new(build());
    BUILD_COUNT.with(|c| c.set(c.get() + 1));
    CACHE.with(|cell| {
        *cell.borrow_mut() = Some((
            FontEntriesKey {
                imported: imported_families.to_vec(),
                system: system_families.to_vec(),
                query: query.to_string(),
            },
            built.clone(),
        ));
    });
    built
}

/// Number of fresh filter passes performed so far on this thread — a
/// monotonic counter used by tests to assert cache hits do not recompute.
#[cfg(test)]
pub(crate) fn build_count() -> u64 {
    BUILD_COUNT.with(Cell::get)
}
