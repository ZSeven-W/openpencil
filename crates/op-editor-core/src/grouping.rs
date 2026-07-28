//! Group / ungroup mutators (`Cmd+G` / `Cmd+Shift+G` parity).
//!
//! Both ops are scoped to the active page's top-level children and
//! preserve paint order: a group inserts where its first member
//! sat; ungroup splices the group's children back in-place.

use crate::id_allocator::{IdAllocError, IdAllocator, SequentialIdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::{make_group, PenNodeExt};
use crate::state::EditorState;
use jian_ops_schema::node::PenNode;

impl EditorState {
    /// Wrap the current selection in a fresh `Group` node inserted
    /// at the position of the first selected sibling. Only supports
    /// same-parent (top-level) selections. Returns the new group's
    /// id, or `None` when the selection is empty / spans parents.
    pub fn group_selected(&mut self, next_id: &mut u64) -> Option<NodeId> {
        let mut allocator = SequentialIdAllocator::for_document(&self.doc, *next_id).ok()?;
        let result = self
            .group_selected_with_allocator(&mut allocator)
            .ok()
            .flatten();
        if result.is_some() {
            *next_id = allocator.next_counter();
        }
        result
    }

    /// Allocator-aware form of [`Self::group_selected`].
    pub fn group_selected_with_allocator(
        &mut self,
        allocator: &mut dyn IdAllocator,
    ) -> Result<Option<NodeId>, IdAllocError> {
        if self.selection.set.is_empty() {
            return Ok(None);
        }
        let selected = self.selection.set.clone();
        // Every selection must be a top-level child for v1 grouping.
        let mut indices: Vec<usize> = Vec::with_capacity(selected.len());
        let children = self.active_children();
        for id in &selected {
            let Some(idx) = children.iter().position(|c| c.id_str() == id.as_str()) else {
                return Ok(None);
            };
            indices.push(idx);
        }
        indices.sort_unstable();
        let mut live = self.collect_node_ids();
        let group_id = allocator.allocate(&mut live)?;
        let children = self.active_children_mut();
        let insert_at = indices[0];
        // Drain in reverse so removal indices stay valid.
        let mut taken: Vec<PenNode> = Vec::with_capacity(indices.len());
        for idx in indices.iter().rev() {
            taken.push(children.remove(*idx));
        }
        taken.reverse();
        let group = make_group(group_id.clone().into(), "Group", taken);
        children.insert(insert_at, group);
        self.selection.set = vec![group_id.clone()];
        self.selection.anchor = group_id.clone();
        Ok(Some(group_id))
    }

    /// Replace the selected `Group` node with its children inline.
    /// No-op when the selection isn't a single top-level Group.
    pub fn ungroup_selected(&mut self) -> bool {
        if self.selection.set.len() != 1 {
            return false;
        }
        let target = self.selection.anchor.clone();
        let children = self.active_children_mut();
        let Some(idx) = children.iter().position(|c| c.id_str() == target.as_str()) else {
            return false;
        };
        if !children[idx].is_group() {
            return false;
        }
        let group = children.remove(idx);
        let inner = group.children().cloned().unwrap_or_default();
        let new_ids: Vec<NodeId> = inner
            .iter()
            .filter_map(|c| NodeId::new_opt(c.id_str()))
            .collect();
        for (k, child) in inner.into_iter().enumerate() {
            children.insert(idx + k, child);
        }
        if new_ids.is_empty() {
            self.clear_selection();
        } else {
            self.selection.anchor = new_ids.last().cloned().unwrap();
            self.selection.set = new_ids;
        }
        true
    }
}
