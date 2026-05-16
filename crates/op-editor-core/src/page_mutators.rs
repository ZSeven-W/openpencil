//! Page CRUD mutators — `add_page` / `duplicate_page` /
//! `remove_page` / `rename_page` / `reorder_page` / page switching.
//!
//! `PenDocument.pages` is `Option<Vec<PenPage>>`. A single-page
//! document keeps `pages == None` and edits `doc.children`. The
//! first `add_page` / `duplicate_page` call promotes the document to
//! multi-page: the root `children` migrate into "Page 1" so no
//! nodes are lost.

use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::page::PenPage;
use jian_ops_schema::node::PenNode;

/// Build a bare page with id / name / children. State + lifecycle
/// default to `None`.
fn make_page(id: String, name: String, children: Vec<PenNode>) -> PenPage {
    PenPage {
        id,
        name,
        children,
        state: None,
        lifecycle: None,
    }
}

impl EditorState {
    /// Ensure the document is in multi-page form, migrating the root
    /// `children` into "Page 1" on the first call. Returns a mutable
    /// reference to the page list.
    fn ensure_pages(&mut self) -> &mut Vec<PenPage> {
        if self.doc.pages.is_none() {
            // Mint the page id BEFORE moving the root children out —
            // `max_node_id` must see the nodes that are migrating so
            // the new page id can't collide with one of them.
            let id = self
                .max_node_id()
                .checked_add(1)
                .map(|n| format!("n{n}"))
                .unwrap_or_else(|| "page-1".to_string());
            let root = std::mem::take(&mut self.doc.children);
            self.doc.pages = Some(vec![make_page(id, "Page 1".to_string(), root)]);
        }
        self.doc.pages.as_mut().unwrap()
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

    /// Append a fresh empty page named `"Page N"` and switch to it.
    /// Returns the new index, or `None` on id overflow.
    pub fn add_page(&mut self) -> Option<usize> {
        // Migrate to multi-page form FIRST so the migrated "Page 1"
        // id is part of the id space before the new page id is
        // minted — otherwise both could land on `n{max+1}`.
        self.ensure_pages();
        let next_id = self.max_node_id().checked_add(1)?;
        let pages = self.doc.pages.as_mut().unwrap();
        let n = pages.len() + 1;
        pages.push(make_page(
            format!("n{next_id}"),
            format!("Page {n}"),
            Vec::new(),
        ));
        let new_index = pages.len() - 1;
        self.ui.active_page_index = new_index;
        self.clear_selection();
        Some(new_index)
    }

    /// Duplicate the page at `idx`, inserting the clone after it.
    /// New node ids are minted past `max_node_id`. Switches the
    /// active page to the clone.
    pub fn duplicate_page(&mut self, idx: usize) -> Option<usize> {
        // `ensure_pages` first so a single-page document is migrated
        // before the id space is snapshotted — otherwise the cloned
        // page id could collide with the migrated "Page 1" id.
        self.ensure_pages();
        let mut next_id = self.max_node_id().checked_add(1)?;
        let mut taken = self.collect_node_ids();
        let pages = self.doc.pages.as_ref().unwrap();
        let source = pages.get(idx)?;
        let new_page_id = walkers::alloc_n_id(&mut next_id, &mut taken)?;
        let new_children: Vec<PenNode> = source
            .children
            .iter()
            .map(|c| walkers::deep_clone_with_new_ids(c, &mut next_id, &mut taken))
            .collect();
        let clone = make_page(
            new_page_id.into(),
            format!("{} copy", source.name),
            new_children,
        );
        let new_index = idx + 1;
        self.doc.pages.as_mut().unwrap().insert(new_index, clone);
        self.ui.active_page_index = new_index;
        self.clear_selection();
        Some(new_index)
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
