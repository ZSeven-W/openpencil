use jian_ops_schema::{node::PenNode, PenDocument};

use crate::{
    apply_budget::ValidationBudget,
    apply_tree::{
        document_ids, find_location, find_node, node_id_of, page_nodes, page_nodes_mut,
        remove_node, replace_node, siblings, siblings_mut, subtree_ids, DocumentStats,
    },
    canonical_node_hash,
    finite::validate_finite,
    id_high_water::IdCounterTracker,
    ApplyContext, ApplyLimits, CollabApplyError, CollabOp, CollabTxn, InsertPolicy, PageRef, Role,
};

/// Successful tracked apply output used by a collaboration session.
#[derive(Debug)]
pub struct AppliedTxn {
    pub document: PenDocument,
    pub next_id_counter: Option<u64>,
}

/// Apply a complete transaction to a clone of `document`.
///
/// The input is never mutated. Operations observe earlier operations in the
/// same transaction, while any later failure discards the entire working copy.
pub fn apply_txn(
    document: &PenDocument,
    txn: &CollabTxn,
    context: &ApplyContext,
) -> Result<PenDocument, CollabApplyError> {
    apply_txn_tracked(document, txn, context).map(|applied| applied.document)
}

/// Apply a transaction and return the retained namespace counter after all
/// newly authored ids have been admitted.
pub fn apply_txn_tracked(
    document: &PenDocument,
    txn: &CollabTxn,
    context: &ApplyContext,
) -> Result<AppliedTxn, CollabApplyError> {
    validate_context(context)?;
    let mut budget = ValidationBudget::new(&context.limits);
    let initial_stats =
        validate_document_with_limits_and_budget(document, &context.limits, &mut budget)?;
    let txn_stats = validate_txn_resource_shape_with_budget(txn, &context.limits, &mut budget)?;
    budget.charge(initial_stats.node_count)?;
    validate_finite(document).map_err(|_| CollabApplyError::NonFiniteNumber)?;
    budget.charge(txn_stats.payload_node_count)?;
    validate_finite(txn).map_err(|_| CollabApplyError::NonFiniteNumber)?;

    let mut working = document.clone();
    let mut working_stats = initial_stats;
    let mut id_counters = IdCounterTracker::from_policy(&context.insert_policy);
    for (operation, collab_op) in txn.ops.iter().enumerate() {
        apply_op(
            &mut working,
            collab_op,
            context,
            operation,
            &mut budget,
            &mut id_counters,
        )?;
        working_stats =
            validate_document_with_limits_and_budget(&working, &context.limits, &mut budget)?;
    }
    budget.charge(working_stats.node_count)?;
    validate_finite(&working).map_err(|_| CollabApplyError::NonFiniteNumber)?;
    Ok(AppliedTxn {
        document: working,
        next_id_counter: id_counters.next_counter(),
    })
}

#[derive(Clone, Copy)]
struct TxnResourceStats {
    payload_node_count: usize,
}

pub(crate) fn validate_txn_resource_shape(
    txn: &CollabTxn,
    limits: &ApplyLimits,
) -> Result<(), CollabApplyError> {
    let mut budget = ValidationBudget::new(limits);
    validate_txn_resource_shape_with_budget(txn, limits, &mut budget).map(|_| ())
}

fn validate_txn_resource_shape_with_budget(
    txn: &CollabTxn,
    limits: &ApplyLimits,
    budget: &mut ValidationBudget,
) -> Result<TxnResourceStats, CollabApplyError> {
    if txn.ops.is_empty() {
        return Err(CollabApplyError::EmptyTransaction);
    }
    if txn.ops.len() > limits.max_ops_per_txn {
        return Err(CollabApplyError::TooManyOperations {
            actual: txn.ops.len(),
            limit: limits.max_ops_per_txn,
        });
    }
    let mut payload_node_count = 0_usize;
    for (operation_index, operation) in txn.ops.iter().enumerate() {
        validate_operation_references(operation, limits)?;
        let node = match operation {
            CollabOp::InsertExact { node, .. } | CollabOp::ReplaceExact { node, .. } => node,
            CollabOp::DeleteExact { .. } | CollabOp::MoveExact { .. } => continue,
        };
        let subtree = subtree_ids(node, limits, budget, operation_index, 0)?;
        payload_node_count = payload_node_count.saturating_add(subtree.node_count);
    }
    Ok(TxnResourceStats { payload_node_count })
}

/// Atomically replace `document` only after the complete transaction succeeds.
pub fn apply_txn_in_place(
    document: &mut PenDocument,
    txn: &CollabTxn,
    context: &ApplyContext,
) -> Result<(), CollabApplyError> {
    let next = apply_txn(document, txn, context)?;
    *document = next;
    Ok(())
}

fn apply_op(
    document: &mut PenDocument,
    operation: &CollabOp,
    context: &ApplyContext,
    operation_index: usize,
    budget: &mut ValidationBudget,
    id_counters: &mut IdCounterTracker,
) -> Result<(), CollabApplyError> {
    validate_operation_references(operation, &context.limits)?;
    match operation {
        CollabOp::InsertExact {
            page,
            parent_id,
            index,
            node,
        } => insert_exact(
            document,
            page,
            parent_id.as_deref(),
            wire_index(*index),
            node,
            context,
            operation_index,
            budget,
            id_counters,
        ),
        CollabOp::ReplaceExact {
            page,
            node_id,
            expected_hash,
            node,
        } => replace_exact(
            document,
            page,
            node_id,
            expected_hash,
            node,
            context,
            operation_index,
            budget,
            id_counters,
        ),
        CollabOp::DeleteExact {
            page,
            node_id,
            expected_hash,
        } => delete_exact(
            document,
            page,
            node_id,
            expected_hash,
            context,
            operation_index,
            budget,
        ),
        CollabOp::MoveExact {
            page,
            node_id,
            expected_parent,
            expected_index,
            new_parent,
            new_index,
        } => move_exact(
            document,
            page,
            node_id,
            expected_parent.as_deref(),
            wire_index(*expected_index),
            new_parent.as_deref(),
            wire_index(*new_index),
            context,
            operation_index,
            budget,
        ),
    }
}

fn wire_index(index: u32) -> usize {
    usize::try_from(index).expect("u32 indices fit every supported OpenPencil target")
}

#[allow(clippy::too_many_arguments)]
fn insert_exact(
    document: &mut PenDocument,
    page: &PageRef,
    parent_id: Option<&str>,
    index: usize,
    node: &PenNode,
    context: &ApplyContext,
    operation_index: usize,
    budget: &mut ValidationBudget,
    id_counters: &mut IdCounterTracker,
) -> Result<(), CollabApplyError> {
    let inserted = subtree_ids(node, &context.limits, budget, operation_index, 0)?;
    admit_authored_ids(
        inserted.ids.iter().map(String::as_str),
        inserted.node_count,
        context,
        budget,
        id_counters,
    )?;

    let existing_ids = document_ids(document, &context.limits, budget)?.0;
    budget.charge(inserted.node_count)?;
    if let Some(id) = inserted.ids.iter().find(|id| existing_ids.contains(*id)) {
        return Err(CollabApplyError::DuplicateId { id: id.clone() });
    }

    let roots = page_nodes_mut(document, page, budget)?;
    let siblings = siblings_mut(roots, parent_id, budget)?;
    if index > siblings.len() {
        return Err(CollabApplyError::IndexOutOfBounds {
            index,
            len: siblings.len(),
        });
    }
    siblings.insert(index, node.clone());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn replace_exact(
    document: &mut PenDocument,
    page: &PageRef,
    node_id: &str,
    expected_hash: &crate::CanonicalHash,
    replacement: &PenNode,
    context: &ApplyContext,
    operation_index: usize,
    budget: &mut ValidationBudget,
    id_counters: &mut IdCounterTracker,
) -> Result<(), CollabApplyError> {
    let roots = page_nodes(document, page, budget)?;
    let current =
        find_node(roots, node_id, budget)?.ok_or_else(|| CollabApplyError::NodeNotFound {
            node_id: node_id.to_owned(),
        })?;

    let replacement_id = node_id_of(replacement);
    if replacement_id != node_id {
        return Err(CollabApplyError::ReplacementIdChanged {
            node_id: node_id.to_owned(),
            replacement_id: replacement_id.to_owned(),
        });
    }

    let old = subtree_ids(current, &context.limits, budget, operation_index, 0)?;
    let replacement_info = subtree_ids(
        replacement,
        &context.limits,
        budget,
        operation_index,
        old.node_count,
    )?;
    enforce_hash(node_id, current, expected_hash, old.node_count, budget)?;
    admit_authored_ids(
        replacement_info
            .ids
            .difference(&old.ids)
            .map(std::string::String::as_str),
        replacement_info.node_count,
        context,
        budget,
        id_counters,
    )?;

    let existing_ids = document_ids(document, &context.limits, budget)?.0;
    budget.charge(replacement_info.node_count)?;
    if let Some(id) = replacement_info
        .ids
        .iter()
        .find(|id| existing_ids.contains(*id) && !old.ids.contains(*id))
    {
        return Err(CollabApplyError::DuplicateId { id: id.clone() });
    }

    let roots = page_nodes_mut(document, page, budget)?;
    replace_node(roots, node_id, replacement.clone(), budget)?;
    Ok(())
}

fn delete_exact(
    document: &mut PenDocument,
    page: &PageRef,
    node_id: &str,
    expected_hash: &crate::CanonicalHash,
    context: &ApplyContext,
    operation_index: usize,
    budget: &mut ValidationBudget,
) -> Result<(), CollabApplyError> {
    let roots = page_nodes(document, page, budget)?;
    let current =
        find_node(roots, node_id, budget)?.ok_or_else(|| CollabApplyError::NodeNotFound {
            node_id: node_id.to_owned(),
        })?;
    let old = subtree_ids(current, &context.limits, budget, operation_index, 0)?;
    enforce_hash(node_id, current, expected_hash, old.node_count, budget)?;

    let roots = page_nodes_mut(document, page, budget)?;
    remove_node(roots, node_id, budget)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn move_exact(
    document: &mut PenDocument,
    page: &PageRef,
    node_id: &str,
    expected_parent: Option<&str>,
    expected_index: usize,
    new_parent: Option<&str>,
    new_index: usize,
    context: &ApplyContext,
    operation_index: usize,
    budget: &mut ValidationBudget,
) -> Result<(), CollabApplyError> {
    let roots = page_nodes(document, page, budget)?;
    let (actual_parent, actual_index) =
        find_location(roots, node_id, None, budget)?.ok_or_else(|| {
            CollabApplyError::NodeNotFound {
                node_id: node_id.to_owned(),
            }
        })?;
    if actual_parent.as_deref() != expected_parent {
        return Err(CollabApplyError::ParentMismatch {
            node_id: node_id.to_owned(),
            expected: expected_parent.map(str::to_owned),
            actual: actual_parent,
        });
    }
    if actual_index != expected_index {
        return Err(CollabApplyError::IndexMismatch {
            node_id: node_id.to_owned(),
            expected: expected_index,
            actual: actual_index,
        });
    }

    let moving = find_node(roots, node_id, budget)?.expect("location and lookup must agree");
    subtree_ids(moving, &context.limits, budget, operation_index, 0)?;
    if let Some(new_parent_id) = new_parent {
        siblings(roots, Some(new_parent_id), budget)?;
        if find_node(std::slice::from_ref(moving), new_parent_id, budget)?.is_some() {
            return Err(CollabApplyError::MoveWouldCycle {
                node_id: node_id.to_owned(),
                new_parent_id: new_parent_id.to_owned(),
            });
        }
    }

    let target_len = siblings(roots, new_parent, budget)?.len()
        - usize::from(actual_parent.as_deref() == new_parent);
    if new_index > target_len {
        return Err(CollabApplyError::IndexOutOfBounds {
            index: new_index,
            len: target_len,
        });
    }

    let roots = page_nodes_mut(document, page, budget)?;
    let moving = remove_node(roots, node_id, budget)?.expect("validated node must be removable");
    siblings_mut(roots, new_parent, budget)?.insert(new_index, moving);
    Ok(())
}

fn enforce_hash(
    node_id: &str,
    node: &PenNode,
    expected: &crate::CanonicalHash,
    subtree_node_count: usize,
    budget: &mut ValidationBudget,
) -> Result<(), CollabApplyError> {
    budget.charge(subtree_node_count)?;
    let actual = canonical_node_hash(node)?;
    if &actual != expected {
        return Err(CollabApplyError::HashMismatch {
            node_id: node_id.to_owned(),
            expected: *expected,
            actual,
        });
    }
    Ok(())
}

fn validate_context(context: &ApplyContext) -> Result<(), CollabApplyError> {
    if context.role == Role::Viewer {
        return Err(CollabApplyError::PermissionDenied { role: context.role });
    }
    Ok(())
}

fn admit_authored_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    node_count_upper_bound: usize,
    context: &ApplyContext,
    budget: &mut ValidationBudget,
    id_counters: &mut IdCounterTracker,
) -> Result<(), CollabApplyError> {
    let InsertPolicy::RequireNamespace { .. } = &context.insert_policy else {
        return Ok(());
    };
    budget.charge(node_count_upper_bound)?;
    id_counters.admit(ids)
}

fn validate_operation_references(
    operation: &CollabOp,
    limits: &ApplyLimits,
) -> Result<(), CollabApplyError> {
    let (page, ids): (&PageRef, Vec<&str>) = match operation {
        CollabOp::InsertExact {
            page, parent_id, ..
        } => (page, parent_id.iter().map(String::as_str).collect()),
        CollabOp::ReplaceExact {
            page,
            node_id,
            node,
            ..
        } => (page, vec![node_id, node_id_of(node)]),
        CollabOp::DeleteExact { page, node_id, .. } => (page, vec![node_id]),
        CollabOp::MoveExact {
            page,
            node_id,
            expected_parent,
            new_parent,
            ..
        } => (
            page,
            std::iter::once(node_id.as_str())
                .chain(expected_parent.iter().map(String::as_str))
                .chain(new_parent.iter().map(String::as_str))
                .collect(),
        ),
    };
    if matches!(page, PageRef::PageId(id) if id.is_empty()) || ids.iter().any(|id| id.is_empty()) {
        return Err(CollabApplyError::EmptyId);
    }
    let page_id = match page {
        PageRef::DocumentRoot => None,
        PageRef::PageId(id) => Some(id.as_str()),
    };
    if let Some(actual) = page_id
        .into_iter()
        .chain(ids)
        .map(str::len)
        .find(|actual| *actual > limits.max_identifier_bytes)
    {
        return Err(CollabApplyError::IdentifierTooLong {
            actual,
            limit: limits.max_identifier_bytes,
        });
    }
    Ok(())
}

/// Validate the collaboration tree invariants required for snapshots and ops.
pub fn validate_document(document: &PenDocument) -> Result<(), CollabApplyError> {
    validate_document_with_limits(document, &ApplyLimits::default()).map(|_| ())
}

pub(crate) fn validate_document_with_limits(
    document: &PenDocument,
    limits: &ApplyLimits,
) -> Result<DocumentStats, CollabApplyError> {
    let mut budget = ValidationBudget::new(limits);
    validate_document_with_limits_and_budget(document, limits, &mut budget)
}

fn validate_document_with_limits_and_budget(
    document: &PenDocument,
    limits: &ApplyLimits,
    budget: &mut ValidationBudget,
) -> Result<DocumentStats, CollabApplyError> {
    document_ids(document, limits, budget).map(|(_, stats)| stats)
}
