//! Per-section markdown parse cache for the Design-MD panel.
//!
//! `DesignMdPanel::section_lines` parses + word-wraps a section's raw
//! markdown (`parse_blocks` → `parse_inline` → `wrap_runs`) on every
//! call. `layout()` calls it once per EXPANDED section, and `layout()`
//! itself runs from both `paint` and `hit_test` — so a design-md brief
//! with several sections expanded got re-tokenized and re-wrapped on
//! every single repaint and every hit-test probe while the panel
//! stayed open, even though the markdown content and the panel width
//! (`section_lines`'s wrap budget is derived from the constant
//! `DESIGN_MD_PANEL_W`, never the live panel rect) hadn't changed.
//!
//! `DesignMdPanel` is rebuilt fresh from `EditorState` every frame (no
//! persistent host-owned instance survives across frames — the same
//! situation as the icon picker / component browser), so the memo
//! lives here as a thread-local rather than a field on the panel
//! struct. The cached function (`parse_blocks` + `wrap_runs`) is a
//! PURE function of `(content, limit)` — no per-document identity
//! leaks into its result — so an identical `(content, limit)` pair
//! always yields an identical result regardless of which `EditorState`
//! produced it. That is what lets this cache skip owner-scoping
//! entirely (unlike `layer_panel_cache`, whose per-document revision
//! counter CAN coincidentally collide across independent documents at
//! the same revision number): the key compares the raw markdown
//! `content` by FULL STRING VALUE (never a pointer or a length
//! shortcut), so a collision is impossible except when the content is
//! truly byte-identical — in which case serving the cached result is
//! correct by construction, regardless of which document asked.
//!
//! One slot PER SECTION INDEX (0..6) rather than a single shared slot,
//! so several sections expanded at once don't evict each other on
//! every call within the same `layout()` pass.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::design_md_panel::RenderLine;

/// One slot per `SECTION_COUNT` section index (`design_md_panel.rs`).
/// Kept as a literal constant (rather than importing the sibling's
/// private `SECTION_COUNT`) so this module has no compile-time
/// dependency beyond the `RenderLine` type; `resolve` degrades to an
/// always-fresh build for any index that would fall outside it, so a
/// future section-count change can never panic here.
const SLOTS: usize = 6;

/// Identity of a cached section build — the raw markdown content plus
/// the truncation cap `section_content` pairs with it (both parsed
/// together by `parse_blocks`).
struct SectionKey {
    content: String,
    limit: usize,
}

/// One cache slot: the key it was built from plus the built lines.
type CacheSlot = Option<(SectionKey, Rc<Vec<RenderLine>>)>;

thread_local! {
    static CACHE: RefCell<[CacheSlot; SLOTS]> =
        const { RefCell::new([None, None, None, None, None, None]) };
    /// Observable rebuild counter — increments only when a fresh
    /// parse + wrap runs. Lets tests prove a cache hit does not
    /// recompute.
    static BUILD_COUNT: Cell<u64> = const { Cell::new(0) };
}

/// Resolve section `index`'s parsed + wrapped lines, memoized on
/// `(content, limit)` by value. `build` runs only on a cache miss.
pub(super) fn resolve(
    index: u8,
    content: &str,
    limit: usize,
    build: impl FnOnce() -> Vec<RenderLine>,
) -> Rc<Vec<RenderLine>> {
    let slot = index as usize;
    if slot >= SLOTS {
        // Defensive only — `SECTION_COUNT == SLOTS` today, so this
        // never fires in practice. An out-of-range index still gets a
        // correct (just uncached) result rather than a panic.
        return Rc::new(build());
    }
    let hit = CACHE.with(|cell| {
        cell.borrow()[slot].as_ref().and_then(|(key, lines)| {
            (key.content == content && key.limit == limit).then(|| lines.clone())
        })
    });
    if let Some(lines) = hit {
        return lines;
    }
    let built = Rc::new(build());
    BUILD_COUNT.with(|c| c.set(c.get() + 1));
    CACHE.with(|cell| {
        cell.borrow_mut()[slot] = Some((
            SectionKey {
                content: content.to_string(),
                limit,
            },
            built.clone(),
        ));
    });
    built
}

/// Number of fresh parse + wrap builds performed so far on this
/// thread — a monotonic counter used by tests to assert cache hits do
/// not recompute.
#[cfg(test)]
pub(super) fn build_count() -> u64 {
    BUILD_COUNT.with(Cell::get)
}
