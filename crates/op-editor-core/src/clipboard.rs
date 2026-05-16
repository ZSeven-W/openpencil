//! Editor clipboard ops — `Cmd+C` / `Cmd+X` / `Cmd+V` parity.
//!
//! The clipboard buffer is a `Vec<PenNode>` carried on
//! `EditorState`. Copy / cut fill it; paste drains it (clones, so
//! repeated paste works). Cut is atomic — a failed delete leg
//! restores the prior clipboard so a no-op cut looks like a no-op.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers::{self, find_node};
use jian_ops_schema::node::PenNode;

impl EditorState {
    /// `Cmd+C`: deep-clone the selected nodes (ids preserved) into
    /// the clipboard. True iff anything was copied.
    pub fn copy_selected(&mut self) -> bool {
        if self.selection.set.is_empty() {
            return false;
        }
        let mut buf: Vec<PenNode> = Vec::with_capacity(self.selection.set.len());
        for id in &self.selection.set {
            if let Some(node) = find_node(self.active_children(), id) {
                buf.push(node.clone());
            }
        }
        if buf.is_empty() {
            return false;
        }
        self.clipboard = buf;
        true
    }

    /// `Cmd+X`: copy the selection then delete it. Atomic — a
    /// failed delete restores the prior clipboard.
    pub fn cut_selected(&mut self) -> bool {
        let saved = std::mem::take(&mut self.clipboard);
        if !self.copy_selected() {
            self.clipboard = saved;
            return false;
        }
        if self.delete_selected() {
            return true;
        }
        self.clipboard = saved;
        false
    }

    /// `Cmd+V`: paste the clipboard nodes into the active page as
    /// top-level siblings offset by `offset_doc_px`. Mints fresh
    /// ids; replaces the selection with the new ids. Returns the new
    /// ids, or empty on no-op (empty clipboard / id overflow).
    pub fn paste_clipboard(&mut self, next_id: &mut u64, offset_doc_px: f64) -> Vec<NodeId> {
        if self.clipboard.is_empty() {
            return Vec::new();
        }
        let Some(safe) = self.max_node_id().checked_add(1) else {
            return Vec::new();
        };
        *next_id = (*next_id).max(safe);
        // Verify total subtree headroom before any mint.
        let total: u64 = self.clipboard.iter().map(walkers::subtree_size).sum();
        if next_id.checked_add(total).is_none() {
            return Vec::new();
        }
        let mut taken = self.collect_node_ids();
        let originals = self.clipboard.clone();
        let children = self.active_children_mut();
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(originals.len());
        for original in &originals {
            let mut clone = walkers::deep_clone_with_new_ids(original, next_id, &mut taken);
            walkers::translate_subtree(&mut clone, offset_doc_px, offset_doc_px);
            if let Some(id) = NodeId::new_opt(clone.id_str()) {
                new_ids.push(id);
            }
            children.push(clone);
        }
        if !new_ids.is_empty() {
            self.selection.anchor = new_ids.last().cloned().unwrap();
            self.selection.set = new_ids.clone();
        }
        new_ids
    }
}
