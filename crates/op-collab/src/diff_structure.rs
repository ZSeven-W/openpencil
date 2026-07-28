use std::collections::BTreeSet;

use jian_ops_schema::{node::PenNode, PenDocument};

use crate::{
    apply_tree::{child_slot, child_slot_mut},
    apply_txn, canonical_node_hash,
    diff_fields::validate_new_node,
    diff_index::{is_m1_basic_kind, node_kind, TreeIndex},
    ApplyContext, CollabOp, CollabTxn, DiffContext, DiffError,
};

pub(crate) fn plan_structure(
    before: &PenDocument,
    before_index: &TreeIndex<'_>,
    after_index: &TreeIndex<'_>,
    added: &BTreeSet<String>,
    deleted: &BTreeSet<String>,
    moved: &BTreeSet<String>,
    context: &DiffContext,
) -> Result<(PenDocument, Vec<CollabOp>), DiffError> {
    validate_structural_nodes(before_index, after_index, added, deleted, moved)?;
    let mut working = before.clone();
    let mut operations = Vec::new();

    for id in &after_index.preorder {
        if !added.contains(id) {
            continue;
        }
        let desired = after_index
            .entry(id)
            .expect("after preorder contains indexed nodes");
        let current = TreeIndex::build(&working)?;
        let index = u32::try_from(current.sibling_len(&desired.page, desired.parent.as_deref()))
            .map_err(|_| DiffError::IndexOverflow)?;
        let operation = CollabOp::InsertExact {
            page: desired.page.clone(),
            parent_id: desired.parent.clone(),
            index,
            node: node_shell(desired.node),
        };
        apply_planned(&mut working, &operation, context)?;
        operations.push(operation);
    }

    normalize_target_order(&mut working, after_index, context, &mut operations)?;

    for id in &before_index.preorder {
        if !deleted.contains(id) {
            continue;
        }
        let original = before_index
            .entry(id)
            .expect("before preorder contains indexed nodes");
        if original
            .parent
            .as_ref()
            .is_some_and(|parent| deleted.contains(parent))
        {
            continue;
        }
        let current = TreeIndex::build(&working)?;
        let current_entry = current
            .entry(id)
            .ok_or(DiffError::StructuralPlanningMismatch)?;
        let operation = CollabOp::DeleteExact {
            page: current_entry.page.clone(),
            node_id: id.clone(),
            expected_hash: canonical_node_hash(current_entry.node)?,
        };
        apply_planned(&mut working, &operation, context)?;
        operations.push(operation);
    }

    normalize_target_order(&mut working, after_index, context, &mut operations)?;
    Ok((working, operations))
}

fn normalize_target_order(
    working: &mut PenDocument,
    desired: &TreeIndex<'_>,
    context: &DiffContext,
    operations: &mut Vec<CollabOp>,
) -> Result<(), DiffError> {
    for id in &desired.preorder {
        let target = desired
            .entry(id)
            .expect("desired preorder contains indexed nodes");
        let current = TreeIndex::build(working)?;
        let current_entry = current
            .entry(id)
            .ok_or(DiffError::StructuralPlanningMismatch)?;
        if current_entry.page != target.page {
            return Err(DiffError::CrossPageMove {
                node_id: id.clone(),
            });
        }
        if current_entry.parent == target.parent && current_entry.index == target.index {
            continue;
        }
        let operation = CollabOp::MoveExact {
            page: target.page.clone(),
            node_id: id.clone(),
            expected_parent: current_entry.parent.clone(),
            expected_index: current_entry.index,
            new_parent: target.parent.clone(),
            new_index: target.index,
        };
        apply_planned(working, &operation, context)?;
        operations.push(operation);
    }
    Ok(())
}

fn validate_structural_nodes(
    before: &TreeIndex<'_>,
    after: &TreeIndex<'_>,
    added: &BTreeSet<String>,
    deleted: &BTreeSet<String>,
    moved: &BTreeSet<String>,
) -> Result<(), DiffError> {
    for id in added {
        let node = after
            .entry(id)
            .expect("added ids come from after index")
            .node;
        validate_new_node(node)?;
    }
    for id in deleted {
        let node = before
            .entry(id)
            .expect("deleted ids come from before index")
            .node;
        require_basic(id, node)?;
    }
    for id in moved {
        let before_entry = before.entry(id).expect("moved ids are common");
        let after_entry = after.entry(id).expect("moved ids are common");
        require_basic(id, before_entry.node)?;
        if before_entry.page != after_entry.page {
            return Err(DiffError::CrossPageMove {
                node_id: id.clone(),
            });
        }
    }
    Ok(())
}

fn require_basic(id: &str, node: &PenNode) -> Result<(), DiffError> {
    if is_m1_basic_kind(node) {
        Ok(())
    } else {
        Err(DiffError::UnsupportedNodeKind {
            node_id: id.to_owned(),
            kind: node_kind(node).to_owned(),
        })
    }
}

fn node_shell(node: &PenNode) -> PenNode {
    let mut shell = node.clone();
    let child_shape = child_slot(node).cloned().flatten().map(|_| Vec::new());
    if let Some(slot) = child_slot_mut(&mut shell) {
        *slot = child_shape;
    }
    shell
}

fn apply_planned(
    document: &mut PenDocument,
    operation: &CollabOp,
    context: &DiffContext,
) -> Result<(), DiffError> {
    let mut apply_context = ApplyContext::standalone_trusted();
    apply_context.limits = context.limits;
    *document = apply_txn(
        document,
        &CollabTxn::new(vec![operation.clone()]),
        &apply_context,
    )
    .map_err(DiffError::Replay)?;
    Ok(())
}
