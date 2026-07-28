//! Page CRUD mutators — `add_page` / `duplicate_page` /
//! `remove_page` / `rename_page` / `reorder_page` / page switching.
//!
//! `PenDocument.pages` is `Option<Vec<PenPage>>`. A single-page
//! document keeps `pages == None` and edits `doc.children`. The
//! first `add_page` / `duplicate_page` call promotes the document to
//! multi-page: the root `children` migrate into "Page 1" so no
//! nodes are lost.

use crate::command_node::{build_leaf_node, remap_subtree_ids_with_allocator};
use crate::fills::set_primary_fill_hex;
use crate::id_allocator::{IdAllocError, IdAllocator, SequentialIdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;

/// Display name of the hidden master-store page that holds an imported
/// component library's reusable masters. It is a side store, not a
/// rendered design — kept off the active design page so scaffolding,
/// role/cleanup passes and page scoring never see the masters.
pub const COMPONENTS_PAGE_NAME: &str = "Components";

/// Build a bare page with id / name / children. State + lifecycle
/// default to `None`.
fn make_page(id: String, name: String, children: Vec<PenNode>) -> PenPage {
    PenPage {
        id,
        name,
        children,
        background_color: None,
        state: None,
        lifecycle: None,
    }
}

fn make_blank_page_frame(id: &NodeId) -> Option<PenNode> {
    let mut frame = build_leaf_node("frame", id.as_str(), "Frame", 0, 0, 1200, 800)?;
    set_primary_fill_hex(&mut frame, "#FFFFFF");
    Some(frame)
}

impl EditorState {
    /// Ensure the document is in multi-page form, migrating the root
    /// `children` into "Page 1" when no pages exist. Covers both
    /// `pages: None` (the single-page fallback) and `pages: Some([])`
    /// (legal-but-empty multi-page) — without the empty-vec branch,
    /// any nodes that landed in `doc.children` while `pages` was
    /// `Some([])` (via the read/write fallback in `active_children`)
    /// would be stranded the moment `add_page` minted a fresh Page 1
    /// alongside them.
    fn ensure_pages_with_allocator(
        &mut self,
        allocator: &mut dyn IdAllocator,
        taken: &mut std::collections::HashSet<NodeId>,
    ) -> Result<(), IdAllocError> {
        let needs_init = self.doc.pages.as_ref().is_none_or(|pages| pages.is_empty());
        if needs_init {
            // Mint the page id BEFORE moving the root children out —
            // `max_node_id` must see the nodes that are migrating so
            // the new page id can't collide with one of them.
            let id = allocator.allocate(taken)?;
            let root = std::mem::take(&mut self.doc.children);
            self.doc.pages = Some(vec![make_page(id.into(), "Page 1".to_string(), root)]);
        }
        Ok(())
    }

    /// Switch the active page to `idx`. False when out of bounds.
    pub fn set_active_page(&mut self, idx: usize) -> bool {
        if idx >= self.page_count() {
            return false;
        }
        if self.ui.active_page_index == idx {
            return true;
        }
        self.ui.active_page_index = idx;
        self.clear_selection();
        true
    }

    /// Authored infinite-canvas background of the active explicit page.
    /// Legacy single-page documents have no page metadata and report `None`
    /// until the first background write promotes them losslessly.
    pub fn active_page_background_color(&self) -> Option<&str> {
        let pages = self.doc.pages.as_ref()?;
        if pages.is_empty() {
            return None;
        }
        pages
            .get(self.ui.active_page_index.min(pages.len() - 1))
            .and_then(|page| page.background_color.as_deref())
    }

    /// Set or clear the active page's infinite-canvas background. A non-empty
    /// write on a legacy `pages: None` (or legal `Some([])`) document promotes
    /// root children into one explicit page within this single mutation.
    /// `None` and blank strings both mean the old omitted/default state.
    pub fn set_active_page_background_color(&mut self, color: Option<String>) -> bool {
        let Ok(mut allocator) = SequentialIdAllocator::for_document(&self.doc, 1) else {
            return false;
        };
        self.set_active_page_background_color_with_allocator(color, &mut allocator)
            .unwrap_or(false)
    }

    /// Allocator-aware form of [`Self::set_active_page_background_color`].
    pub fn set_active_page_background_color_with_allocator(
        &mut self,
        color: Option<String>,
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        let color = color.and_then(|color| {
            let color = color.trim();
            (!color.is_empty()).then(|| color.to_string())
        });
        let needs_page = self.doc.pages.as_ref().is_none_or(|pages| pages.is_empty());
        if needs_page {
            if color.is_none() {
                return Ok(false);
            }
            let mut taken = self.collect_node_ids();
            self.ensure_pages_with_allocator(allocator, &mut taken)?;
        }
        let pages = self
            .doc
            .pages
            .as_mut()
            .expect("ensure_pages initialized pages");
        let index = self.ui.active_page_index.min(pages.len() - 1);
        if pages[index].background_color == color {
            return Ok(false);
        }
        pages[index].background_color = color;
        Ok(true)
    }

    /// Append a fresh empty page named `"Page N"` and switch to it.
    /// Returns the new index, or `None` on id overflow.
    pub fn add_page(&mut self) -> Option<usize> {
        self.add_page_with_name(None)
    }

    /// Append a fresh empty page with an optional display name and
    /// switch to it. Empty / whitespace-only custom names are rejected.
    pub fn add_page_with_name(&mut self, name: Option<String>) -> Option<usize> {
        self.add_page_with_name_and_children(name, None)
    }

    /// Append a fresh page with optional caller-provided children and
    /// switch to it. External child ids are always remapped into this
    /// document's id space, matching `InsertSubtree`.
    pub fn add_page_with_name_and_children(
        &mut self,
        name: Option<String>,
        children: Option<Vec<PenNode>>,
    ) -> Option<usize> {
        let mut allocator = SequentialIdAllocator::for_document(&self.doc, 1).ok()?;
        self.add_page_with_allocator(name, children, &mut allocator)
            .ok()
            .flatten()
    }

    /// Allocator-aware page insertion used by collaboration sessions.
    pub fn add_page_with_allocator(
        &mut self,
        name: Option<String>,
        children: Option<Vec<PenNode>>,
        allocator: &mut dyn IdAllocator,
    ) -> Result<Option<usize>, IdAllocError> {
        let custom_name = match name {
            Some(name) if name.trim().is_empty() => return Ok(None),
            Some(name) => Some(name),
            None => None,
        };
        let before_doc = self.doc.clone();
        let before_page = self.ui.active_page_index;
        let before_selection = self.selection.clone();
        let result = (|| {
            // Migrate to multi-page form FIRST so the migrated "Page 1"
            // id is part of the id space before the new page id is minted.
            let mut taken = self.collect_node_ids();
            self.ensure_pages_with_allocator(allocator, &mut taken)?;
            let page_id = allocator.allocate(&mut taken)?;
            let page_children = match children {
                Some(mut children) => {
                    remap_subtree_ids_with_allocator(&mut children, allocator, &mut taken)?;
                    children
                }
                None => {
                    let frame_id = allocator.allocate(&mut taken)?;
                    let Some(frame) = make_blank_page_frame(&frame_id) else {
                        return Ok(None);
                    };
                    vec![frame]
                }
            };
            let pages = self.doc.pages.as_mut().unwrap();
            let n = pages.len() + 1;
            let page_name = custom_name.unwrap_or_else(|| format!("Page {n}"));
            pages.push(make_page(page_id.into(), page_name, page_children));
            let new_index = pages.len() - 1;
            self.ui.active_page_index = new_index;
            self.clear_selection();
            Ok(Some(new_index))
        })();
        if !matches!(&result, Ok(Some(_))) {
            self.doc = before_doc;
            self.ui.active_page_index = before_page;
            self.selection = before_selection;
        }
        result
    }

    /// Append the reusable masters of an imported component library
    /// onto a dedicated, hidden [`COMPONENTS_PAGE_NAME`] page — NOT the
    /// active design page. Keeps `active_children()` clean (only the
    /// design) so the orchestrator's scaffold + role/cleanup passes are
    /// unaffected, while the masters stay in `doc.pages` where the
    /// document-wide component lookup (`ComponentLibrary::from_document`
    /// + `ref_resolve::resolve_refs_for_canvas`) still finds them.
    ///
    /// Master ids are preserved verbatim (NO remapping) so `ref` nodes
    /// keep resolving to their targets. Masters are deduped by id
    /// against whatever already lives on the components page, so a
    /// re-import is idempotent.
    ///
    /// Returns the number of masters actually appended (post-dedup).
    /// The active page index is preserved: a single-page document is
    /// first migrated so its design becomes page 0, and the components
    /// page is appended after it, so the caller's active page keeps
    /// pointing at the design.
    pub fn append_components_page_masters(&mut self, masters: Vec<PenNode>) -> usize {
        let Ok(mut allocator) = SequentialIdAllocator::for_document(&self.doc, 1) else {
            return 0;
        };
        self.append_components_page_masters_with_allocator(masters, &mut allocator)
            .unwrap_or(0)
    }

    /// Allocator-aware form of [`Self::append_components_page_masters`].
    pub fn append_components_page_masters_with_allocator(
        &mut self,
        masters: Vec<PenNode>,
        allocator: &mut dyn IdAllocator,
    ) -> Result<usize, IdAllocError> {
        if masters.is_empty() {
            return Ok(0);
        }
        // Preserve the active design page across the migration: a
        // single-page document moves its `doc.children` into page 0,
        // and the components page is appended at the end.
        let active = self.ui.active_page_index;
        let before_doc = self.doc.clone();
        let before_selection = self.selection.clone();
        let mut taken = self.collect_node_ids();
        if let Err(error) = self.ensure_pages_with_allocator(allocator, &mut taken) {
            self.doc = before_doc;
            self.ui.active_page_index = active;
            self.selection = before_selection;
            return Err(error);
        }

        // Find (or create) the dedicated components page.
        let page_idx = match self
            .doc
            .pages
            .as_ref()
            .unwrap()
            .iter()
            .position(|p| p.name == COMPONENTS_PAGE_NAME)
        {
            Some(idx) => idx,
            None => {
                // Mint a non-colliding page id without disturbing the
                // master ids (which must stay verbatim for refs).
                let page_id = match allocator.allocate(&mut taken) {
                    Ok(page_id) => page_id,
                    Err(error) => {
                        self.doc = before_doc;
                        self.ui.active_page_index = active;
                        self.selection = before_selection;
                        return Err(error);
                    }
                };
                let pages = self.doc.pages.as_mut().unwrap();
                pages.push(make_page(
                    page_id.into(),
                    COMPONENTS_PAGE_NAME.to_string(),
                    Vec::new(),
                ));
                pages.len() - 1
            }
        };

        let pages = self.doc.pages.as_mut().unwrap();
        let page = &mut pages[page_idx];
        let mut existing: std::collections::HashSet<String> = page
            .children
            .iter()
            .map(|n| n.id_str().to_string())
            .collect();
        let mut added = 0usize;
        for master in masters {
            let id = master.id_str().to_string();
            if existing.contains(&id) {
                continue;
            }
            existing.insert(id);
            page.children.push(master);
            added += 1;
        }

        // The components page is hidden side storage — never the active
        // page. Restore the caller's active index (page 0 = design).
        self.ui.active_page_index = active;
        Ok(added)
    }

    /// Duplicate the page at `idx`, inserting the clone after it.
    /// New node ids are minted past `max_node_id`. Switches the
    /// active page to the clone.
    pub fn duplicate_page(&mut self, idx: usize) -> Option<usize> {
        self.duplicate_page_with_name(idx, None)
    }

    /// Duplicate the page at `idx`, optionally overriding the clone's
    /// display name. Empty / whitespace-only custom names are rejected.
    pub fn duplicate_page_with_name(&mut self, idx: usize, name: Option<String>) -> Option<usize> {
        let mut allocator = SequentialIdAllocator::for_document(&self.doc, 1).ok()?;
        self.duplicate_page_with_allocator(idx, name, &mut allocator)
            .ok()
            .flatten()
    }

    /// Allocator-aware page duplication used by collaboration sessions.
    pub fn duplicate_page_with_allocator(
        &mut self,
        idx: usize,
        name: Option<String>,
        allocator: &mut dyn IdAllocator,
    ) -> Result<Option<usize>, IdAllocError> {
        let custom_name = match name {
            Some(name) if name.trim().is_empty() => return Ok(None),
            Some(name) => Some(name),
            None => None,
        };
        let before_doc = self.doc.clone();
        let before_page = self.ui.active_page_index;
        let before_selection = self.selection.clone();
        let result = (|| {
            // `ensure_pages` first so a single-page document is migrated
            // before the id space is snapshotted.
            let mut taken = self.collect_node_ids();
            self.ensure_pages_with_allocator(allocator, &mut taken)?;
            let pages = self.doc.pages.as_ref().unwrap();
            let Some(source) = pages.get(idx) else {
                return Ok(None);
            };
            let new_page_id = allocator.allocate(&mut taken)?;
            let new_children: Result<Vec<PenNode>, IdAllocError> = source
                .children
                .iter()
                .map(|child| walkers::deep_clone_with_allocator(child, allocator, &mut taken))
                .collect();
            let clone_name = custom_name.unwrap_or_else(|| format!("{} copy", source.name));
            let clone = PenPage {
                id: new_page_id.into(),
                name: clone_name,
                children: new_children?,
                background_color: source.background_color.clone(),
                state: source.state.clone(),
                lifecycle: source.lifecycle.clone(),
            };
            let new_index = idx + 1;
            self.doc.pages.as_mut().unwrap().insert(new_index, clone);
            self.ui.active_page_index = new_index;
            self.clear_selection();
            Ok(Some(new_index))
        })();
        if !matches!(&result, Ok(Some(_))) {
            self.doc = before_doc;
            self.ui.active_page_index = before_page;
            self.selection = before_selection;
        }
        result
    }

    /// Set a page's name directly. Rejects out-of-range indices and
    /// empty / whitespace-only names.
    pub fn rename_page(&mut self, idx: usize, name: impl Into<String>) -> bool {
        let name: String = name.into();
        if name.trim().is_empty() {
            return false;
        }
        let Some(pages) = self.doc.pages.as_mut() else {
            // Single-page document: only index 0 is valid, and the
            // implicit page has no name field to write.
            return false;
        };
        let Some(page) = pages.get_mut(idx) else {
            return false;
        };
        page.name = name;
        true
    }

    /// Move a page from `from` to `to`. `to` is clamped into range.
    /// Keeps `active_page_index` pointing at the same logical page.
    pub fn reorder_page(&mut self, from: usize, to: usize) -> bool {
        let Some(pages) = self.doc.pages.as_mut() else {
            return false;
        };
        if from >= pages.len() {
            return false;
        }
        let to = to.min(pages.len().saturating_sub(1));
        if from == to {
            return false;
        }
        let page = pages.remove(from);
        pages.insert(to, page);
        let active = self.ui.active_page_index;
        if active == from {
            self.ui.active_page_index = to;
        } else if from < active && to >= active {
            self.ui.active_page_index = active - 1;
        } else if from > active && to <= active {
            self.ui.active_page_index = active + 1;
        }
        true
    }

    /// Swap the page at `idx` with the previous one. No-op at 0.
    pub fn move_page_up(&mut self, idx: usize) -> bool {
        if idx == 0 {
            return false;
        }
        self.reorder_page(idx, idx - 1)
    }

    /// Swap the page at `idx` with the next one. No-op at the end.
    pub fn move_page_down(&mut self, idx: usize) -> bool {
        let count = self.doc.pages.as_ref().map(|p| p.len()).unwrap_or(0);
        if idx + 1 >= count {
            return false;
        }
        self.reorder_page(idx, idx + 1)
    }

    /// Remove the page at `idx`. No-op when out-of-range OR when
    /// only one page remains. Adjusts `active_page_index` + clears
    /// the selection.
    pub fn remove_page(&mut self, idx: usize) -> bool {
        let Some(pages) = self.doc.pages.as_mut() else {
            return false;
        };
        if idx >= pages.len() || pages.len() <= 1 {
            return false;
        }
        pages.remove(idx);
        let len = pages.len();
        if self.ui.active_page_index >= len {
            self.ui.active_page_index = len - 1;
        } else if idx < self.ui.active_page_index {
            self.ui.active_page_index -= 1;
        }
        self.clear_selection();
        true
    }

    /// Rename a node by id on the active page. True when the node
    /// was found. Rejects whitespace-only names.
    pub fn rename_node(&mut self, id: &NodeId, name: impl Into<String>) -> bool {
        let name: String = name.into();
        if name.trim().is_empty() {
            return false;
        }
        let Some(node) = walkers::find_node_mut(self.active_children_mut(), id) else {
            return false;
        };
        use crate::pen_node_ext::PenNodeExt;
        node.base_mut().name = Some(name);
        true
    }
}
