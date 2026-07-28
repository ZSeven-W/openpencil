//! Editor clipboard ops — `Cmd+C` / `Cmd+X` / `Cmd+V` parity.
//!
//! The clipboard buffer is a `Vec<PenNode>` carried on
//! `EditorState`. Copy / cut fill it; paste drains it (clones, so
//! repeated paste works). Cut is atomic — a failed delete leg
//! restores the prior clipboard so a no-op cut looks like a no-op.

use crate::id_allocator::{IdAllocError, IdAllocator, SequentialIdAllocator};
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

    /// `Cmd+V`: paste the clipboard nodes, anchored to the active
    /// selection (TS `use-clipboard-shortcuts.ts:66-90`): a container
    /// anchor receives the clones as its last children; a leaf anchor
    /// makes them siblings inserted right after it; no / stale anchor
    /// falls back to top level. Clones offset by `offset_doc_px` and
    /// mint fresh ids; the selection becomes the new ids (anchor =
    /// FIRST pasted id, TS `setSelection(newIds, newIds[0])`).
    /// Returns the new ids, or empty on no-op (empty clipboard / id
    /// overflow).
    pub fn paste_clipboard(&mut self, next_id: &mut u64, offset_doc_px: f64) -> Vec<NodeId> {
        let Ok(mut allocator) = SequentialIdAllocator::for_document(&self.doc, *next_id) else {
            return Vec::new();
        };
        let result = self
            .paste_clipboard_with_allocator(&mut allocator, offset_doc_px)
            .unwrap_or_default();
        if !result.is_empty() {
            *next_id = allocator.next_counter();
        }
        result
    }

    /// Allocator-aware form of [`Self::paste_clipboard`].
    pub fn paste_clipboard_with_allocator(
        &mut self,
        allocator: &mut dyn IdAllocator,
        offset_doc_px: f64,
    ) -> Result<Vec<NodeId>, IdAllocError> {
        if self.clipboard.is_empty() {
            return Ok(Vec::new());
        }
        let mut taken = self.collect_node_ids();
        let originals = self.clipboard.clone();
        let anchor = self.selection.set.first().cloned();
        let mut staged_children = self.active_children().to_vec();
        let children = &mut staged_children;
        // Resolve the paste target before any mutation.
        let (parent, mut insert_index): (Option<NodeId>, Option<usize>) = match anchor
            .as_ref()
            .and_then(|a| walkers::find_node(children, a).map(|n| n.children().is_some()))
        {
            // Container anchor: paste inside, appended at the end.
            Some(true) => (anchor.clone(), None),
            // Leaf anchor: paste as siblings right after it.
            Some(false) => match walkers::find_parent_and_index(children, anchor.as_ref().unwrap())
            {
                Some((p, idx)) => (p, Some(idx + 1)),
                None => (None, None),
            },
            // Nothing selected (or stale id): top-level append.
            None => (None, None),
        };
        let mut new_ids: Vec<NodeId> = Vec::with_capacity(originals.len());
        for original in &originals {
            // TS quirk ported verbatim (use-clipboard-shortcuts.ts:
            // 93-103): pasting a reusable component mints a Ref
            // INSTANCE next to the LIVE component (the
            // `duplicateNode` path), not at the paste anchor — and
            // does not advance the sibling insert cursor. A deleted
            // or no-longer-reusable component falls through to a
            // plain clone at the anchor.
            if let PenNode::Frame(f) = original {
                if f.reusable == Some(true) {
                    let cid = NodeId::new(f.base.id.clone());
                    let live_reusable = walkers::find_node(children, &cid).is_some_and(
                        |n| matches!(n, PenNode::Frame(fr) if fr.reusable == Some(true)),
                    );
                    if live_reusable {
                        if let Some(minted) = crate::mutators::duplicate_in_children_with_allocator(
                            children,
                            &cid,
                            allocator,
                            &mut taken,
                            offset_doc_px,
                        )? {
                            new_ids.push(minted);
                            continue;
                        }
                    }
                }
            }
            let mut clone = walkers::deep_clone_with_allocator(original, allocator, &mut taken)?;
            walkers::translate_subtree(&mut clone, offset_doc_px, offset_doc_px);
            let id = NodeId::new_opt(clone.id_str());
            if !walkers::insert_into_parent(children, parent.as_ref(), insert_index, clone) {
                continue;
            }
            if let Some(id) = id {
                new_ids.push(id);
            }
            if let Some(ix) = insert_index.as_mut() {
                *ix += 1;
            }
        }
        if !new_ids.is_empty() {
            *self.active_children_mut() = staged_children;
            self.selection.anchor = new_ids[0].clone();
            self.selection.set = new_ids.clone();
        }
        Ok(new_ids)
    }
}
