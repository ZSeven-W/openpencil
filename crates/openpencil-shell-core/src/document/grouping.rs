//! Group / ungroup mutators backing the `Cmd+G` / `Cmd+Shift+G`
//! shortcuts. Both operations are scoped to the active page and
//! preserve paint order: a Group inserts where its first member
//! was; an ungrouped Group splices its children in-place.

use super::*;

impl Document {
    /// Wrap the current selection in a fresh `NodeKind::Group` node
    /// inserted at the position of the first selected sibling. Only
    /// supports same-parent selections (multi-level grouping would
    /// need an LCA walk that is out of scope here). Returns the new
    /// group's id, or `None` when the selection is empty or spans
    /// multiple parents.
    pub fn group_selected(&mut self, next_id: &mut u64) -> Option<NodeId> {
        if self.selected_set.is_empty() {
            return None;
        }
        // Use top-level children only — keeps the v1 behaviour
        // predictable and lines up with TS `groupSelected` which
        // also rejects mixed-parent selections.
        let active = self.active_page_index;
        let page = self.pages.get_mut(active)?;
        let selected: Vec<NodeId> = self.selected_set.clone();
        // Collect the index of every selected top-level child;
        // bail when any selection isn't at the top level.
        let mut indices: Vec<usize> = Vec::new();
        for id in &selected {
            let idx = page.children.iter().position(|c| c.id == *id)?;
            indices.push(idx);
        }
        indices.sort_unstable();
        let insert_at = indices[0];
        // Drain in reverse so removal indices stay valid.
        let mut taken: Vec<Node> = Vec::with_capacity(indices.len());
        for idx in indices.iter().rev() {
            taken.push(page.children.remove(*idx));
        }
        taken.reverse();
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let mut live = self.collect_node_ids();
        let group_id = super::walkers::alloc_n_id(next_id, &mut live)?;
        let group =
            Node::with_children(group_id.as_str(), NodeKind::Group, "Group", taken);
        let page = self.pages.get_mut(active)?;
        page.children.insert(insert_at, group);
        self.selected_set.clear();
        self.selected_set.push(group_id.clone());
        self.selected = group_id.clone();
        Some(group_id)
    }

    /// Replace the selected Group node with its children inline.
    /// No-op when the selection isn't a single Group at the top
    /// level of the active page.
    pub fn ungroup_selected(&mut self) -> bool {
        if self.selected_set.len() != 1 {
            return false;
        }
        let target = self.selected.clone();
        let active = self.active_page_index;
        let Some(page) = self.pages.get_mut(active) else {
            return false;
        };
        let Some(idx) = page.children.iter().position(|c| c.id == target) else {
            return false;
        };
        if !matches!(page.children[idx].kind, NodeKind::Group) {
            return false;
        }
        let group = page.children.remove(idx);
        let new_ids: Vec<NodeId> = group.children.iter().map(|c| c.id.clone()).collect();
        // Splice the children where the group used to sit.
        for (k, child) in group.children.into_iter().enumerate() {
            page.children.insert(idx + k, child);
        }
        if new_ids.is_empty() {
            self.clear_selection();
        } else {
            self.selected_set = new_ids.clone();
            self.selected = new_ids.last().unwrap().clone();
        }
        true
    }
}
