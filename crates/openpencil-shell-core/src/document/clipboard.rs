//! Document clipboard ops (Cmd+C / Cmd+X / Cmd+V parity). Carved
//! off `mutators.rs` to stay under the 800-line cap as the cut
//! atomicity guard grew.

use super::walkers::{deep_clone_with_new_ids, shift_subtree, subtree_size};
use super::{Document, Node, NodeId};

impl Document {
    /// Cmd+C: deep-clone selected nodes (ids preserved) into the
    /// clipboard. True iff anything was copied.
    pub fn copy_selected(&mut self) -> bool {
        if self.selected_set.is_empty() {
            return false;
        }
        let Some(page) = self.active_page() else {
            return false;
        };
        let mut buf: Vec<Node> = Vec::with_capacity(self.selected_set.len());
        for id in &self.selected_set {
            if let Some(node) = page.find(*id) {
                buf.push(node.clone());
            }
        }
        if buf.is_empty() {
            return false;
        }
        self.clipboard = buf;
        true
    }

    /// Copy the selection into the clipboard then delete it.
    /// Returns true when both steps succeeded. Mirrors TS `Cmd+X`.
    ///
    /// Atomic: if the delete leg rejects (e.g. every selected node
    /// is locked) the prior clipboard contents are restored so a
    /// failed cut looks like a no-op to callers. Codex stop-gate:
    /// previously a failed cut left the clipboard mutated — every
    /// downstream paste would have pasted the would-be-cut nodes.
    pub fn cut_selected(&mut self) -> bool {
        let saved_clipboard = std::mem::take(&mut self.clipboard);
        if !self.copy_selected() {
            self.clipboard = saved_clipboard;
            return false;
        }
        if self.delete_selected() {
            return true;
        }
        self.clipboard = saved_clipboard;
        false
    }

    /// Paste clipboard nodes into the active page as top-level
    /// siblings offset by `offset_doc_px`. Mints fresh ids from
    /// `next_id`; replaces selection with the new ids. Returns
    /// the new ids in paste order, or empty on no-op (empty
    /// clipboard or id-allocator overflow). Mirrors TS `Cmd+V`.
    pub fn paste_clipboard(&mut self, next_id: &mut u64, offset_doc_px: f32) -> Vec<NodeId> {
        if self.clipboard.is_empty() {
            return Vec::new();
        }
        let Some(safe) = self.max_node_id().checked_add(1) else {
            return Vec::new();
        };
        *next_id = (*next_id).max(safe);
        // Verify total subtree headroom before any mint so a
        // partially-pasted document is impossible on overflow.
        let total: u64 = self.clipboard.iter().map(subtree_size).sum();
        if next_id.checked_add(total).is_none() {
            return Vec::new();
        }
        // Clone clipboard out so `pages.get_mut` doesn't alias
        // `self.clipboard`.
        let originals = self.clipboard.clone();
        let Some(page) = self.pages.get_mut(self.active_page_index) else {
            return Vec::new();
        };
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(originals.len());
        for original in &originals {
            let mut clone = deep_clone_with_new_ids(original, next_id);
            shift_subtree(&mut clone, offset_doc_px, offset_doc_px);
            new_ids.push(clone.id);
            page.children.push(clone);
        }
        if !new_ids.is_empty() {
            self.selected = *new_ids.last().unwrap();
            self.selected_set = new_ids.clone();
        }
        new_ids
    }
}
