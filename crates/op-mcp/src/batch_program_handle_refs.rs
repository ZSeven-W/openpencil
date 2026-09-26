//! Handle references inside an `I()` node body's `children` array.
//!
//! A weak model sometimes builds bottom-up: it inserts the leaves first and
//! then creates their container, listing the leaves by HANDLE —
//! `b9=I(b8, {…dot…})` then `b10=I(b8, {"type":"frame",…,"children":["b9"]})`.
//! `children` is typed `Vec<PenNode>`, so the string entry used to fail the
//! whole container line; the already-inserted leaves then stayed loose in the
//! grandparent with no layout (measured: GLM-5.3-Flash w01 sidebar — 11
//! container lines dropped, the brand mark, count pills and user card
//! scattered as bare leaves).
//!
//! The executor now reads such an entry as "move that node here": the
//! string entries are lifted out of the payload before it is deserialised
//! (so the container itself parses), and once the container is inserted each
//! referenced node is MOVED under it at the position its entry held — the
//! resulting tree is the one the model would have got by writing the same
//! children inline, in the same order. An entry that names no earlier
//! binding, names a node that is gone, repeats a handle, or would create a
//! cycle (the handle is an ancestor of the new container) is dropped with a
//! warning; the container still lands.

use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::node::PenNode;
use op_editor_core::{walkers, EditorCommand, NodeId, PenNodeExt};
use serde_json::Value;

use super::batch_program::ProgramCtx;

/// One string entry lifted out of a `children` array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HandleChildRef {
    /// Position path from the payload root to the container that listed the
    /// entry, counting only inline OBJECT children at each level.
    pub(crate) container_path: Vec<usize>,
    /// Inline object children that preceded this entry in its array.
    pub(crate) inline_before: usize,
    /// The handle as written (trimmed).
    pub(crate) handle: String,
}

/// A lifted entry together with the final id its container received.
pub(crate) struct ResolvedHandleRef {
    reference: HandleChildRef,
    container_id: Option<String>,
}

/// Lift every string entry out of every `children` array in the payload
/// (recursively), in document order. Non-string entries are left in place.
pub(crate) fn extract_handle_children(value: &mut Value) -> Vec<HandleChildRef> {
    let mut refs = Vec::new();
    extract_at(value, &mut Vec::new(), &mut refs);
    refs
}

fn extract_at(value: &mut Value, path: &mut Vec<usize>, out: &mut Vec<HandleChildRef>) {
    let Some(Value::Array(children)) = value
        .as_object_mut()
        .and_then(|object| object.get_mut("children"))
    else {
        return;
    };
    let entries = std::mem::take(children);
    let mut inline_objects = 0usize;
    for entry in entries {
        match entry {
            Value::String(handle) => out.push(HandleChildRef {
                container_path: path.clone(),
                inline_before: inline_objects,
                handle: handle.trim().to_string(),
            }),
            other => {
                if other.is_object() {
                    inline_objects += 1;
                }
                children.push(other);
            }
        }
    }
    let mut object_index = 0usize;
    for child in children.iter_mut() {
        if !child.is_object() {
            continue;
        }
        path.push(object_index);
        extract_at(child, path, out);
        path.pop();
        object_index += 1;
    }
}

/// Pair every lifted entry with the FINAL id of its container, read off the
/// remapped subtree about to be inserted.
pub(crate) fn resolve_containers(
    root: &PenNode,
    refs: Vec<HandleChildRef>,
) -> Vec<ResolvedHandleRef> {
    refs.into_iter()
        .map(|reference| {
            let container_id =
                node_at_path(root, &reference.container_path).map(|n| n.id_str().to_string());
            ResolvedHandleRef {
                reference,
                container_id,
            }
        })
        .collect()
}

fn node_at_path<'a>(root: &'a PenNode, path: &[usize]) -> Option<&'a PenNode> {
    let mut node = root;
    for &index in path {
        node = node.children()?.get(index)?;
    }
    Some(node)
}

/// Move each referenced node under its (now inserted) container, at the
/// position its entry held. Every refusal is a warning, never a line error:
/// the container has already landed and must stay.
pub(crate) fn attach_handle_children(resolved: Vec<ResolvedHandleRef>, ctx: &mut ProgramCtx) {
    let mut placed: BTreeMap<String, usize> = BTreeMap::new();
    let mut moved: BTreeSet<String> = BTreeSet::new();
    for ResolvedHandleRef {
        reference,
        container_id,
    } in resolved
    {
        let handle = reference.handle.as_str();
        let Some(container_id) = container_id else {
            ctx.warn(format!(
                "children entry \"{handle}\" dropped: its container did not survive normalization"
            ));
            continue;
        };
        let Some(node_id) = ctx.bindings.get(handle).cloned() else {
            ctx.warn(format!(
                "children entry \"{handle}\" dropped: no earlier line bound that handle"
            ));
            continue;
        };
        let target = NodeId::new(&container_id);
        let Some(node) = walkers::find_node(ctx.sim.active_children(), &NodeId::new(&node_id))
        else {
            ctx.warn(format!(
                "children entry \"{handle}\" dropped: node {node_id} no longer exists"
            ));
            continue;
        };
        if node_id == container_id || walkers::descendant_contains(node, &target) {
            ctx.warn(format!(
                "children entry \"{handle}\" dropped: {node_id} is an ancestor of the new node \
                 (moving it inside would create a cycle)"
            ));
            continue;
        }
        if !moved.insert(node_id.clone()) {
            ctx.warn(format!(
                "children entry \"{handle}\" dropped: the handle is listed more than once"
            ));
            continue;
        }
        let slot = placed.entry(container_id.clone()).or_insert(0);
        let index = reference.inline_before + *slot;
        let cmd = EditorCommand::MoveNode {
            node_id: NodeId::new(&node_id),
            target_parent: target,
            page_id: ctx.page_id.clone(),
            index: Some(index),
        };
        match ctx.emit(cmd, "move failed") {
            Ok(()) => *slot += 1,
            Err(_) => ctx.warn(format!(
                "children entry \"{handle}\" dropped: {node_id} could not be moved into \
                 {container_id}"
            )),
        }
    }
}
