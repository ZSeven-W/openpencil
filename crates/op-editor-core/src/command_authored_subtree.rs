//! Authored-id subtree insertion for layered design skeletons.

use std::collections::HashSet;

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::node::PenNode;

impl EditorState {
    pub(crate) fn cmd_insert_authored_subtree(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> bool {
        if nodes.is_empty() {
            return false;
        }
        let mut nodes = nodes;
        let replacement = crate::command_root_replace::prepare_root_frame_replacement(
            self.active_children(),
            &mut nodes,
            parent_id,
        );
        if parent_id.is_real() {
            // Accept any container (matches `cmd_insert_subtree`), including an
            // empty one whose `children` is still `None` — the insert below
            // initializes it via `children_mut()` (`get_or_insert_with`). Using
            // `children().is_some()` here would wrongly REJECT a valid insert
            // under an empty Frame/Group.
            match walkers::find_node(self.active_children(), parent_id) {
                Some(parent) if parent.is_container() => {}
                _ => return false,
            }
        }

        let mut live = self.collect_node_ids();
        if let Some(replacement) = replacement.as_ref() {
            live.remove(crate::command_root_replace::replacement_node_id(
                replacement,
            ));
        }
        let mut incoming = HashSet::new();
        if !nodes
            .iter()
            .all(|node| collect_authored_ids(node, &live, &mut incoming))
        {
            return false;
        }

        if parent_id.is_real() {
            let Some(parent) = walkers::find_node_mut(self.active_children_mut(), parent_id) else {
                return false;
            };
            let Some(children) = parent.children_mut() else {
                return false;
            };
            children.extend(nodes);
        } else {
            let roots = self.active_children_mut();
            if let Some(replacement) = replacement.as_ref() {
                if !crate::command_root_replace::remove_root_frame_replacement(roots, replacement) {
                    return false;
                }
            }
            roots.extend(nodes);
        }
        true
    }
}

fn collect_authored_ids(
    node: &PenNode,
    live: &HashSet<NodeId>,
    incoming: &mut HashSet<NodeId>,
) -> bool {
    // Every authored node MUST carry a valid (non-empty), doc-unique id — that
    // is the whole point of the authored path (vs `cmd_insert_subtree`, which
    // remaps ids and so tolerates an empty/garbage id). Callers that mint ids
    // (e.g. `batch_design`, which assigns a fresh `n{N}` to every node via
    // `remap_subtree_ids_mapping` before emitting `InsertAuthoredSubtree`)
    // always satisfy this; an empty id here is a malformed input and rejected.
    let Some(id) = NodeId::new_opt(node.id_str()) else {
        return false;
    };
    if live.contains(&id) || !incoming.insert(id) {
        return false;
    }
    node.children()
        .map(|children| {
            children
                .iter()
                .all(|child| collect_authored_ids(child, live, incoming))
        })
        .unwrap_or(true)
}
