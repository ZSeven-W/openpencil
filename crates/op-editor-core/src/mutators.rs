//! `impl EditorState` editor mutators — ported from
//! `openpencil-shell-core::document`'s `impl Document`, retargeted
//! onto the canonical `jian_ops_schema::PenDocument`.
//!
//! ## Page model
//!
//! `PenDocument` carries `pages: Option<Vec<PenPage>>` plus a root
//! `children: Vec<PenNode>`. When `pages` is `Some`, the editor
//! works on the active page's `children`; when `pages` is `None`,
//! the document is single-page and the editor works on the root
//! `children` directly. [`EditorState::active_children`] /
//! [`EditorState::active_children_mut`] hide that fork so the
//! mutators stay page-model-agnostic.

use crate::geometry::{own_bounds, union_aggregate_bounds, DocRect};
use crate::history::EditorSnapshot;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::selection::SelectionState;
use crate::state::EditorState;
use crate::ui_draft::PropertyFocus;
use crate::walkers::{
    self, find_node, find_node_mut, reorder_in_children, ReorderDirection,
};
use jian_ops_schema::node::PenNode;
use std::collections::HashSet;

/// Largest number of undo entries kept (matches shell-core's cap).
const HISTORY_CAP: usize = 100;

impl EditorState {
    // --- Active-page node access -------------------------------------

    /// The active page's children — `doc.pages[active]` when the
    /// document is multi-page, else the root `doc.children`.
    pub fn active_children(&self) -> &[PenNode] {
        match self.doc.pages.as_ref() {
            Some(pages) => match pages.get(self.ui.active_page_index) {
                Some(page) => &page.children,
                None => &[],
            },
            None => &self.doc.children,
        }
    }

    /// Mutable form of [`EditorState::active_children`].
    pub fn active_children_mut(&mut self) -> &mut Vec<PenNode> {
        let idx = self.ui.active_page_index;
        match self.doc.pages.as_mut() {
            Some(pages) => {
                let len = pages.len();
                let i = if idx < len { idx } else { 0 };
                // A multi-page document always has at least page 0;
                // an empty `pages` is normalized away by the
                // page-mutator layer, so this index is safe.
                &mut pages[i.min(len.saturating_sub(1))].children
            }
            None => &mut self.doc.children,
        }
    }

    /// Number of pages — 1 when the document uses the single-page
    /// fallback (no `pages` list).
    pub fn page_count(&self) -> usize {
        self.doc.pages.as_ref().map(|p| p.len().max(1)).unwrap_or(1)
    }

    // --- Queries -----------------------------------------------------

    /// The anchor-selected node on the active page, or `None`.
    pub fn selected_node(&self) -> Option<&PenNode> {
        if !self.selection.anchor.is_real() {
            return None;
        }
        find_node(self.active_children(), &self.selection.anchor)
    }

    /// True when `id` is in the active selection set.
    pub fn is_selected(&self, id: &NodeId) -> bool {
        self.selection.contains(id)
    }

    /// Number of nodes in the active selection set.
    pub fn selection_count(&self) -> usize {
        self.selection.len()
    }

    /// Whether `id` resolves to an editable node on the active page.
    /// Hidden (`visible == Some(false)`) + locked nodes are not
    /// editable; everything else is.
    pub fn is_editable(&self, id: &NodeId) -> bool {
        let Some(node) = find_node(self.active_children(), id) else {
            return false;
        };
        node_editable(node)
    }

    /// Stricter form of [`EditorState::is_editable`] — every
    /// descendant must also be editable. Gates destructive ops so a
    /// locked / hidden child protects its ancestor.
    pub fn is_subtree_editable(&self, id: &NodeId) -> bool {
        let Some(node) = find_node(self.active_children(), id) else {
            return false;
        };
        subtree_all_editable(node)
    }

    /// Largest editor-minted `n{N}` id suffix anywhere in the
    /// document (0 when no `n{N}` id exists). Seeds the new-node-id
    /// allocator.
    pub fn max_node_id(&self) -> u64 {
        let mut max = 0u64;
        if let Some(pages) = self.doc.pages.as_ref() {
            for page in pages {
                max = max.max(walkers::parse_n_id(&page.id).unwrap_or(0));
                for child in &page.children {
                    max = max.max(walkers::max_id_walk(child));
                }
            }
        }
        for child in &self.doc.children {
            max = max.max(walkers::max_id_walk(child));
        }
        max
    }

    /// Every node + page id live in the document. Backs the
    /// allocator's collision check.
    pub fn collect_node_ids(&self) -> HashSet<NodeId> {
        let mut out = HashSet::new();
        if let Some(pages) = self.doc.pages.as_ref() {
            for page in pages {
                if let Some(id) = NodeId::new_opt(page.id.as_str()) {
                    out.insert(id);
                }
                walkers::collect_ids(&page.children, &mut out);
            }
        }
        walkers::collect_ids(&self.doc.children, &mut out);
        out
    }

    /// Union of `aggregate_bounds` across the selected nodes.
    pub fn selection_bounds(&self) -> Option<DocRect> {
        union_aggregate_bounds(self.active_children(), &self.selection.set)
    }

    /// First duplicate id found in the document, or `None`.
    pub fn find_duplicate_id(&self) -> Option<NodeId> {
        let mut seen: HashSet<String> = HashSet::new();
        if let Some(pages) = self.doc.pages.as_ref() {
            for page in pages {
                if !seen.insert(page.id.clone()) {
                    return NodeId::new_opt(page.id.as_str());
                }
                if let Some(dup) = walkers::find_duplicate(&page.children, &mut seen) {
                    return Some(dup);
                }
            }
        }
        walkers::find_duplicate(&self.doc.children, &mut seen)
    }

    // --- Selection ---------------------------------------------------

    /// Replace the selection with `id` + anchor on it. A NONE id
    /// clears the selection. Idempotent.
    pub fn set_single_selection(&mut self, id: NodeId) {
        if id.is_real() {
            self.selection = SelectionState {
                anchor: id.clone(),
                set: vec![id],
            };
        } else {
            self.clear_selection();
        }
    }

    /// Shift-click semantics: toggle `id` in / out of the set,
    /// keeping the anchor invariant (anchor == last entry).
    pub fn toggle_selection(&mut self, id: NodeId) {
        if !id.is_real() {
            return;
        }
        if let Some(pos) = self.selection.set.iter().position(|n| *n == id) {
            self.selection.set.remove(pos);
            self.selection.anchor =
                self.selection.set.last().cloned().unwrap_or(NodeId::NONE);
        } else {
            self.selection.anchor = id.clone();
            self.selection.set.push(id);
        }
    }

    /// Clear both anchor + set. Idempotent.
    pub fn clear_selection(&mut self) {
        self.selection = SelectionState::empty();
    }

    /// Alias for [`EditorState::clear_selection`] — Escape's last
    /// tier in the editor host.
    pub fn deselect_all(&mut self) {
        self.clear_selection();
    }

    /// Select every top-level node on the active page. Anchor is the
    /// last node. False when the page has no children.
    pub fn select_all_top_level(&mut self) -> bool {
        let ids: Vec<NodeId> = self
            .active_children()
            .iter()
            .filter_map(|n| NodeId::new_opt(n.id_str()))
            .collect();
        if ids.is_empty() {
            return false;
        }
        self.selection.anchor = ids.last().cloned().unwrap_or(NodeId::NONE);
        self.selection.set = ids;
        true
    }

    // --- History -----------------------------------------------------

    /// Snapshot the editor's undoable state without pushing it.
    pub fn snapshot_for_history(&self) -> EditorSnapshot {
        EditorSnapshot {
            doc: self.doc.clone(),
            selection: self.selection.clone(),
            active_page_index: self.ui.active_page_index,
        }
    }

    /// Push a snapshot onto the undo stack + clear redo. Cap = 100.
    pub fn history_push_past(&mut self, snap: EditorSnapshot) {
        self.history.past.push_back(snap);
        if self.history.past.len() > HISTORY_CAP {
            self.history.past.pop_front();
        }
        self.history.future.clear();
    }

    /// Push the current state onto the undo stack. Call BEFORE a
    /// transactional change so undo reverts to here.
    pub fn commit_history(&mut self) {
        let snap = self.snapshot_for_history();
        self.history_push_past(snap);
    }

    /// Restore the editor state from a snapshot.
    fn restore(&mut self, snap: EditorSnapshot) {
        self.doc = snap.doc;
        self.selection = snap.selection;
        self.ui.active_page_index = snap.active_page_index;
    }

    /// Undo the last change. False when the undo stack is empty.
    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.history.past.pop_back() else {
            return false;
        };
        let cur = self.snapshot_for_history();
        self.history.future.push_back(cur);
        self.restore(prev);
        true
    }

    /// Redo the last undone change. False when the redo stack is empty.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.history.future.pop_back() else {
            return false;
        };
        let cur = self.snapshot_for_history();
        self.history.past.push_back(cur);
        self.restore(next);
        true
    }

    // --- Node flag toggles -------------------------------------------

    /// Toggle the `visible` flag on the node. True on success.
    /// `visible == None` is treated as visible, so the first toggle
    /// hides it.
    pub fn toggle_node_hidden(&mut self, id: &NodeId) -> bool {
        let Some(node) = find_node_mut(self.active_children_mut(), id) else {
            return false;
        };
        let base = node.base_mut();
        let now_visible = base.visible.unwrap_or(true);
        base.visible = Some(!now_visible);
        true
    }

    /// Toggle the `locked` flag on the node. True on success.
    pub fn toggle_node_locked(&mut self, id: &NodeId) -> bool {
        let Some(node) = find_node_mut(self.active_children_mut(), id) else {
            return false;
        };
        let base = node.base_mut();
        base.locked = Some(!base.locked.unwrap_or(false));
        true
    }

    // --- Geometry ----------------------------------------------------

    /// Overwrite the anchor node's rotation (radians, clockwise).
    /// The canonical schema stores rotation in degrees, so this
    /// converts on write. No-op on a locked / hidden / missing node.
    pub fn set_selected_rotation(&mut self, radians: f32) {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return;
        }
        if let Some(node) = find_node_mut(self.active_children_mut(), &sel) {
            node.base_mut().rotation = Some((radians as f64).to_degrees());
        }
    }

    /// Overwrite the anchor node's axis-aligned bounds (doc-space
    /// `x`/`y` + `width`/`height`). No-op when the node is locked /
    /// hidden / missing.
    pub fn set_selected_bounds(&mut self, bounds: DocRect) {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return;
        }
        if let Some(node) = find_node_mut(self.active_children_mut(), &sel) {
            // Only write a real rect — container nodes that derive
            // their size from children keep deriving it.
            let own = own_bounds(node);
            if own.w > 0.0 || own.h > 0.0 {
                node.base_mut().x = Some(bounds.x);
                node.base_mut().y = Some(bounds.y);
                node.set_width_px(bounds.w);
                node.set_height_px(bounds.h);
            } else {
                node.base_mut().x = Some(bounds.x);
                node.base_mut().y = Some(bounds.y);
            }
        }
    }

    /// Translate every node in the selection set by `(dx, dy)` doc
    /// px. Containers cascade; an ancestor-already-in-set dedup
    /// stops descendants shifting twice.
    pub fn translate_selected(&mut self, dx: f64, dy: f64) {
        if self.selection.set.is_empty() {
            return;
        }
        let editable: Vec<NodeId> = self
            .selection
            .set
            .iter()
            .filter(|id| self.is_editable(id))
            .cloned()
            .collect();
        if editable.is_empty() {
            return;
        }
        let children = self.active_children_mut();
        for target in &editable {
            if !walkers::is_ancestor_in_set(children, target, &editable) {
                if let Some(node) = find_node_mut(children, target) {
                    walkers::translate_subtree(node, dx, dy);
                }
            }
        }
    }

    /// Apply a parsed numeric property edit to the anchor node.
    /// True on a real, editable selection.
    pub fn commit_property_edit(&mut self, focus: PropertyFocus, value: f32) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        let v = value as f64;
        match focus {
            PropertyFocus::PositionX => node.base_mut().x = Some(v),
            PropertyFocus::PositionY => node.base_mut().y = Some(v),
            PropertyFocus::SizeW => node.set_width_px(v.max(0.0)),
            PropertyFocus::SizeH => node.set_height_px(v.max(0.0)),
            PropertyFocus::Rotation => {
                // Property panel ships degrees; schema stores degrees.
                node.base_mut().rotation = Some(v);
            }
            // Corner-radius, hex + opacity edits route through
            // dedicated setters (not a single base-field write).
            PropertyFocus::PositionR
            | PropertyFocus::StrokeWidth
            | PropertyFocus::Opacity
            | PropertyFocus::FillHex
            | PropertyFocus::StrokeHex => {}
        }
        true
    }

    // --- Tree ops ----------------------------------------------------

    /// Remove every editable node in the selection set from its
    /// parent. Locked / hidden subtrees are protected. True on
    /// success; selection collapses to the kept (protected) ids.
    pub fn delete_selected(&mut self) -> bool {
        if self.selection.set.is_empty() {
            return false;
        }
        let (deletable, kept): (Vec<NodeId>, Vec<NodeId>) = self
            .selection
            .set
            .iter()
            .cloned()
            .partition(|id| self.is_subtree_editable(id));
        if deletable.is_empty() {
            return false;
        }
        let children = self.active_children_mut();
        let mut removed_any = false;
        for id in &deletable {
            if walkers::remove_from_children(children, id) {
                removed_any = true;
            }
        }
        if removed_any {
            self.selection.set = kept;
            self.selection.anchor =
                self.selection.set.last().cloned().unwrap_or(NodeId::NONE);
            true
        } else {
            false
        }
    }

    /// Deep-clone every selected node with fresh ids, inserting each
    /// clone as the next sibling offset by `offset_doc_px`. Returns
    /// the new anchor id (last clone) on success.
    pub fn duplicate_selected(
        &mut self,
        next_id: &mut u64,
        offset_doc_px: f64,
    ) -> Option<NodeId> {
        if self.selection.set.is_empty() {
            return None;
        }
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let mut taken = self.collect_node_ids();
        let targets = self.selection.set.clone();
        let children = self.active_children_mut();
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(targets.len());
        for target in &targets {
            if let Some(new_id) = duplicate_in_children(
                children,
                target,
                next_id,
                &mut taken,
                offset_doc_px,
            ) {
                new_ids.push(new_id);
            }
        }
        if new_ids.is_empty() {
            return None;
        }
        self.selection.anchor = new_ids.last().cloned().unwrap();
        self.selection.set = new_ids;
        Some(self.selection.anchor.clone())
    }

    /// Bump the anchor node up / down one position among its
    /// siblings. True on success.
    pub fn reorder_selected(&mut self, direction: ReorderDirection) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() {
            return false;
        }
        reorder_in_children(self.active_children_mut(), &sel, direction)
    }

    /// Move `source` to be a sibling immediately before `anchor`.
    /// Cross-parent reparenting supported. Cycle / lock / missing
    /// guarded.
    pub fn reorder_before(&mut self, source: NodeId, anchor: NodeId) -> bool {
        self.reorder_relative(source, anchor, true)
    }

    /// Move `source` to be a sibling immediately after `anchor`.
    pub fn reorder_after(&mut self, source: NodeId, anchor: NodeId) -> bool {
        self.reorder_relative(source, anchor, false)
    }

    /// Move `source` so it becomes the LAST child of `parent`.
    pub fn reorder_into(&mut self, source: NodeId, parent: NodeId) -> bool {
        if source == parent || !source.is_real() || !parent.is_real() {
            return false;
        }
        if !self.is_subtree_editable(&source) {
            return false;
        }
        let children = self.active_children();
        let Some(source_ref) = find_node(children, &source) else {
            return false;
        };
        if walkers::descendant_contains(source_ref, &parent)
            || find_node(children, &parent).is_none()
        {
            return false;
        }
        let children = self.active_children_mut();
        let Some(node) = walkers::extract_node(children, &source) else {
            return false;
        };
        walkers::append_into(children, &parent, node).is_ok()
    }

    fn reorder_relative(&mut self, source: NodeId, anchor: NodeId, before: bool) -> bool {
        if source == anchor || !source.is_real() || !anchor.is_real() {
            return false;
        }
        if !self.is_subtree_editable(&source) {
            return false;
        }
        let children = self.active_children();
        let Some(source_ref) = find_node(children, &source) else {
            return false;
        };
        if walkers::descendant_contains(source_ref, &anchor) {
            return false;
        }
        if !walkers::contains_node(children, &anchor) {
            return false;
        }
        let children = self.active_children_mut();
        let Some(node) = walkers::extract_node(children, &source) else {
            return false;
        };
        let r = if before {
            walkers::insert_before_in_children(children, &anchor, node)
        } else {
            walkers::insert_after_in_children(children, &anchor, node)
        };
        r.is_ok()
    }

    /// Light invariant check — Err on first violation: out-of-range
    /// active page index, or a duplicate node id.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(dup) = self.find_duplicate_id() {
            return Err(format!("duplicate NodeId: {dup:?}"));
        }
        if self.ui.active_page_index >= self.page_count() {
            return Err(format!(
                "active_page_index {} out of range (page_count={})",
                self.ui.active_page_index,
                self.page_count()
            ));
        }
        Ok(())
    }
}

// --- Free helpers ----------------------------------------------------

/// True when a node is editable — not hidden, not locked.
fn node_editable(node: &PenNode) -> bool {
    let base = node.base();
    base.visible.unwrap_or(true) && !base.locked.unwrap_or(false)
}

/// True when `node` and every descendant are editable.
fn subtree_all_editable(node: &PenNode) -> bool {
    if !node_editable(node) {
        return false;
    }
    match node.children() {
        Some(children) => children.iter().all(subtree_all_editable),
        None => true,
    }
}

/// Recursive helper for `duplicate_selected` — finds the target,
/// deep-clones it with fresh ids, offsets the clone, and inserts it
/// as the next sibling. Returns the clone's id.
fn duplicate_in_children(
    children: &mut Vec<PenNode>,
    target: &NodeId,
    next_id: &mut u64,
    taken: &mut HashSet<NodeId>,
    offset: f64,
) -> Option<NodeId> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == target.as_str()) {
        let size = walkers::subtree_size(&children[idx]);
        next_id.checked_add(size)?;
        let mut clone = walkers::deep_clone_with_new_ids(&children[idx], next_id, taken);
        walkers::translate_subtree(&mut clone, offset, offset);
        let new_id = NodeId::new_opt(clone.id_str())?;
        children.insert(idx + 1, clone);
        return Some(new_id);
    }
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            if let Some(new_id) =
                duplicate_in_children(grand, target, next_id, taken, offset)
            {
                return Some(new_id);
            }
        }
    }
    None
}
