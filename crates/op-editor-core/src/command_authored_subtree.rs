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
        if parent_id.is_real() {
            match walkers::find_node(self.active_children(), parent_id) {
                Some(parent) if parent.children().is_some() => {}
                _ => return false,
            }
        }

        let live = self.collect_node_ids();
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
            self.active_children_mut().extend(nodes);
        }
        true
    }
}

fn collect_authored_ids(
    node: &PenNode,
    live: &HashSet<NodeId>,
    incoming: &mut HashSet<NodeId>,
) -> bool {
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
