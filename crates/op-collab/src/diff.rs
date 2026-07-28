use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::PenDocument;
use serde_json::{Map, Value};

use crate::{
    apply::validate_document_with_limits,
    apply_txn, apply_txn_tracked, canonical_document_hash, canonical_node_hash,
    diff_fields::{collect_property_changes, replacement_with_changes},
    diff_index::TreeIndex,
    diff_structure::plan_structure,
    serde_context::{inline_document, to_isolated_value},
    ApplyContext, CollabOp, CollabTxn, DiffContext, DiffError, EditChanges, NodeFieldChange,
    StructuralChanges, StructuralMove, SupportedDiff,
};

/// Convert a supported authored document edit into deterministic exact
/// collaboration operations.
///
/// M1 deliberately accepts a narrow edit matrix. The generated transaction is
/// replayed from `before` under the peer namespace policy and must reproduce
/// the canonical hash of `after`; otherwise no transaction is returned.
pub fn diff_supported(
    before: &PenDocument,
    after: &PenDocument,
    context: &DiffContext,
) -> Result<SupportedDiff, DiffError> {
    validate_document_with_limits(before, &context.limits).map_err(DiffError::InvalidBefore)?;
    validate_document_with_limits(after, &context.limits).map_err(DiffError::InvalidAfter)?;
    validate_document_and_pages_unchanged(before, after)?;

    let before_index = TreeIndex::build(before)?;
    let after_index = TreeIndex::build(after)?;
    let before_ids: BTreeSet<_> = before_index.nodes.keys().cloned().collect();
    let after_ids: BTreeSet<_> = after_index.nodes.keys().cloned().collect();
    let added: BTreeSet<_> = after_ids.difference(&before_ids).cloned().collect();
    let deleted: BTreeSet<_> = before_ids.difference(&after_ids).cloned().collect();
    let common: BTreeSet<_> = before_ids.intersection(&after_ids).cloned().collect();

    let mut moved = BTreeSet::new();
    for id in &common {
        let original = before_index.entry(id).expect("common id is indexed");
        let desired = after_index.entry(id).expect("common id is indexed");
        if original.page != desired.page {
            return Err(DiffError::CrossPageMove {
                node_id: id.clone(),
            });
        }
        if original.parent != desired.parent || original.index != desired.index {
            moved.insert(id.clone());
        }
    }

    let property_changes = collect_property_changes(&before_index, &after_index)?;
    let has_structure = !(added.is_empty() && deleted.is_empty() && moved.is_empty());
    if has_structure && !property_changes.is_empty() {
        return Err(DiffError::MixedPropertyAndStructure);
    }

    let (planned, operations, changes) = if has_structure {
        let (planned, operations) = plan_structure(
            before,
            &before_index,
            &after_index,
            &added,
            &deleted,
            &moved,
            context,
        )?;
        let moves = moved
            .iter()
            .map(|id| {
                let original = before_index.entry(id).expect("moved id is indexed");
                let desired = after_index.entry(id).expect("moved id is indexed");
                StructuralMove {
                    node_id: id.clone(),
                    from_parent: original.parent.clone(),
                    from_index: original.index,
                    to_parent: desired.parent.clone(),
                    to_index: desired.index,
                }
            })
            .collect();
        (
            planned,
            operations,
            EditChanges::Structure(StructuralChanges {
                inserted_ids: added.into_iter().collect(),
                deleted_ids: deleted.into_iter().collect(),
                moves,
            }),
        )
    } else if property_changes.is_empty() {
        return Err(DiffError::NoDocumentChange);
    } else {
        let (planned, operations) =
            plan_property_changes(before, &after_index, &property_changes, context)?;
        (planned, operations, EditChanges::Property(property_changes))
    };

    if operations.len() > context.limits.max_ops_per_txn {
        return Err(DiffError::TooManyOperations {
            actual: operations.len(),
            limit: context.limits.max_ops_per_txn,
        });
    }
    verify_hash(&planned, after)?;

    let txn = CollabTxn::new(operations);
    let mut apply_context = ApplyContext::for_peer_namespace_at(
        context.namespace.clone(),
        context.role,
        context.next_id_counter,
    );
    apply_context.limits = context.limits;
    let replay = apply_txn_tracked(before, &txn, &apply_context).map_err(DiffError::Replay)?;
    verify_hash(&replay.document, after)?;

    Ok(SupportedDiff { txn, changes })
}

fn plan_property_changes(
    before: &PenDocument,
    after_index: &TreeIndex<'_>,
    changes: &[NodeFieldChange],
    context: &DiffContext,
) -> Result<(PenDocument, Vec<CollabOp>), DiffError> {
    let mut by_node: BTreeMap<&str, Vec<NodeFieldChange>> = BTreeMap::new();
    for change in changes {
        by_node
            .entry(change.node_id.as_str())
            .or_default()
            .push(change.clone());
    }

    let mut working = before.clone();
    let mut operations = Vec::with_capacity(by_node.len());
    for id in &after_index.preorder {
        let Some(node_changes) = by_node.remove(id.as_str()) else {
            continue;
        };
        let current_index = TreeIndex::build(&working)?;
        let current = current_index
            .entry(id)
            .ok_or(DiffError::StructuralPlanningMismatch)?;
        let operation = CollabOp::ReplaceExact {
            page: current.page.clone(),
            node_id: id.clone(),
            expected_hash: canonical_node_hash(current.node)?,
            node: replacement_with_changes(current.node, &node_changes)?,
        };
        let mut apply_context = ApplyContext::standalone_trusted();
        apply_context.limits = context.limits;
        working = apply_txn(
            &working,
            &CollabTxn::new(vec![operation.clone()]),
            &apply_context,
        )
        .map_err(DiffError::Replay)?;
        operations.push(operation);
    }
    if !by_node.is_empty() {
        return Err(DiffError::StructuralPlanningMismatch);
    }
    Ok((working, operations))
}

fn verify_hash(
    actual_document: &PenDocument,
    expected_document: &PenDocument,
) -> Result<(), DiffError> {
    let expected = canonical_document_hash(expected_document)?;
    let actual = canonical_document_hash(actual_document)?;
    if actual == expected {
        Ok(())
    } else {
        Err(DiffError::ReplayHashMismatch { expected, actual })
    }
}

fn validate_document_and_pages_unchanged(
    before: &PenDocument,
    after: &PenDocument,
) -> Result<(), DiffError> {
    let before = inline_document_value(before)?;
    let after = inline_document_value(after)?;
    let before_object = before
        .as_object()
        .expect("typed PenDocument serializes as an object");
    let after_object = after
        .as_object()
        .expect("typed PenDocument serializes as an object");

    let keys: BTreeSet<&str> = before_object
        .keys()
        .chain(after_object.keys())
        .map(String::as_str)
        .collect();
    for key in keys {
        if matches!(key, "children" | "pages") {
            continue;
        }
        if before_object.get(key) != after_object.get(key) {
            return Err(DiffError::UnsupportedDocumentField {
                field: key.to_owned(),
            });
        }
    }

    if pages_without_children(before_object) != pages_without_children(after_object) {
        return Err(DiffError::UnsupportedPageChange);
    }
    Ok(())
}

fn inline_document_value(document: &PenDocument) -> Result<Value, DiffError> {
    let mut isolated = to_isolated_value(document)?;
    inline_document(&mut isolated.value, &isolated.images);
    Ok(isolated.value)
}

fn pages_without_children(document: &Map<String, Value>) -> Option<Vec<Value>> {
    document.get("pages").map(|pages| {
        pages
            .as_array()
            .map(|pages| {
                pages
                    .iter()
                    .map(|page| {
                        let mut page = page.clone();
                        if let Some(object) = page.as_object_mut() {
                            object.remove("children");
                        }
                        page
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![pages.clone()])
    })
}
