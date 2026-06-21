//! Shared empty-root-frame replacement for generated design inserts.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use jian_ops_schema::node::PenNode;

pub(crate) struct RootFrameReplacement {
    node_id: NodeId,
}

pub(crate) fn prepare_root_frame_replacement(
    roots: &[PenNode],
    nodes: &mut [PenNode],
    parent_id: &NodeId,
) -> Option<RootFrameReplacement> {
    if parent_id.is_real() || nodes.len() != 1 || !matches!(nodes[0], PenNode::Frame(_)) {
        return None;
    }
    let empty = roots.iter().find(|node| is_empty_frame(node))?;
    if let Some(x) = empty.base().x {
        nodes[0].base_mut().x = Some(x);
    }
    if let Some(y) = empty.base().y {
        nodes[0].base_mut().y = Some(y);
    }
    Some(RootFrameReplacement {
        node_id: NodeId::new(empty.id_str()),
    })
}

pub(crate) fn remove_root_frame_replacement(
    roots: &mut Vec<PenNode>,
    replacement: &RootFrameReplacement,
) -> bool {
    let Some(index) = roots
        .iter()
        .position(|node| node.id_str() == replacement.node_id.as_str())
    else {
        return false;
    };
    roots.remove(index);
    true
}

pub(crate) fn replacement_node_id(replacement: &RootFrameReplacement) -> &NodeId {
    &replacement.node_id
}

fn is_empty_frame(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_)) && node.children().map(|c| c.is_empty()).unwrap_or(true)
}
