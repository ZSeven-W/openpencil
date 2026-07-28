use std::{collections::BTreeMap, sync::Arc};

use jian_ops_schema::PenDocument;

use crate::{
    apply_txn, apply_txn_tracked, canonical_node_hash,
    diff_fields::{apply_changes_to_node, current_field_value},
    diff_index::TreeIndex,
    diff_supported,
    guest::GuestSessionCore,
    ApplyContext, CollabMessage, CollabOp, CollabTxn, DiffContext, DiffError, EditChanges,
    GuestConnectionState, GuestEffect, GuestError, NodeFieldChange, PendingCancelReason,
    PendingEdit, PendingEditStatus, Role, Submit,
};

pub(crate) enum PendingRebase {
    Rebased {
        desired: PenDocument,
        txn: CollabTxn,
        changes: EditChanges,
    },
    AlreadySatisfied {
        desired: PenDocument,
    },
    Conflict(PendingCancelReason),
}

pub(crate) type CarriedPending = (
    Arc<PenDocument>,
    Option<PendingEdit>,
    Option<(crate::ClientOpId, PendingCancelReason)>,
);

impl GuestSessionCore {
    /// Build a selective-undo request in its own idempotency counter domain.
    pub fn request_undo(
        &mut self,
        target_client_op_id: crate::ClientOpId,
    ) -> Result<GuestEffect, GuestError> {
        match self.state {
            GuestConnectionState::Active => {}
            GuestConnectionState::AwaitingSnapshot => return Err(GuestError::AwaitingSnapshot),
            GuestConnectionState::Disconnected => return Err(GuestError::Disconnected),
            GuestConnectionState::Ended => return Err(GuestError::SessionEnded),
        }
        if self.role == Role::Viewer {
            return Err(GuestError::ViewerReadOnly);
        }
        if self.prepared.is_some() {
            return Err(GuestError::InstallPending);
        }
        if self.pending.is_some() {
            return Err(GuestError::PendingEditExists);
        }
        if self.pending_undo.is_some() || self.expected_compensation.is_some() {
            return Err(GuestError::UndoPending);
        }
        if target_client_op_id.peer_id != self.peer_id {
            return Err(GuestError::InvalidUndoTarget);
        }
        if !self
            .undo_index
            .iter()
            .any(|entry| entry.client_op_id == target_client_op_id)
        {
            return Err(GuestError::UnknownUndoTarget);
        }
        let counter = self
            .next_undo_counter
            .ok_or(GuestError::UndoCounterExhausted)?;
        self.next_undo_counter = counter.checked_add(1);
        let request = crate::UndoRequest {
            request_id: crate::UndoRequestId {
                peer_id: self.peer_id.clone(),
                local_counter: counter,
            },
            target_client_op_id,
        };
        self.pending_undo = Some(request.clone());
        Ok(GuestEffect::Send(CollabMessage::UndoRequest(request)))
    }

    /// Record one already-authored optimistic local document edit and return
    /// the Submit that the host should send to the owner.
    ///
    /// M1 permits only one pending document edit. Callers must use
    /// [`Self::next_id_counter`] for their collaboration id allocator before
    /// authoring `desired_after`.
    pub fn begin_local_edit(
        &mut self,
        desired_after: &PenDocument,
    ) -> Result<GuestEffect, GuestError> {
        match self.state {
            GuestConnectionState::Active => {}
            GuestConnectionState::AwaitingSnapshot => return Err(GuestError::AwaitingSnapshot),
            GuestConnectionState::Disconnected => return Err(GuestError::Disconnected),
            GuestConnectionState::Ended => return Err(GuestError::SessionEnded),
        }
        if self.role == Role::Viewer {
            return Err(GuestError::ViewerReadOnly);
        }
        if self.prepared.is_some() {
            return Err(GuestError::InstallPending);
        }
        if self.pending.is_some() {
            return Err(GuestError::PendingEditExists);
        }
        if self.pending_undo.is_some() || self.expected_compensation.is_some() {
            return Err(GuestError::UndoPending);
        }
        let confirmed = self
            .confirmed_document
            .as_ref()
            .ok_or(GuestError::NoConfirmedDocument)?;
        let confirmed_seq = self.confirmed_seq.ok_or(GuestError::NoConfirmedDocument)?;
        let counter = self
            .next_client_counter
            .ok_or(GuestError::ClientCounterExhausted)?;
        let id_counter_before = self.next_id_counter;
        let context = DiffContext::new(self.namespace.clone(), self.role, id_counter_before);
        let supported = diff_supported(confirmed, desired_after, &context)?;

        let mut apply_context = ApplyContext::for_peer_namespace_at(
            self.namespace.clone(),
            self.role,
            id_counter_before,
        );
        apply_context.limits = context.limits;
        let applied = apply_txn_tracked(confirmed, &supported.txn, &apply_context)?;
        let client_op_id = crate::ClientOpId {
            peer_id: self.peer_id.clone(),
            local_counter: counter,
        };
        let submit = Submit {
            client_op_id: client_op_id.clone(),
            base_seq: confirmed_seq,
            txn: supported.txn.clone(),
        };
        self.next_client_counter = counter.checked_add(1);
        self.next_id_counter = applied.next_id_counter;
        let desired = Arc::new(desired_after.clone());
        self.displayed_document = Some(desired.clone());
        self.pending = Some(PendingEdit {
            client_op_id,
            base_seq: confirmed_seq,
            before: confirmed.clone(),
            desired_after: desired,
            submitted_txn: supported.txn,
            changes: supported.changes,
            status: PendingEditStatus::Submitted,
            deferred_reject: None,
            id_counter_before,
        });
        Ok(GuestEffect::Send(CollabMessage::Submit(submit)))
    }

    pub(crate) fn retry_stale_pending(
        &mut self,
        mut pending: PendingEdit,
    ) -> Result<Vec<GuestEffect>, GuestError> {
        let confirmed = self
            .confirmed_document
            .as_ref()
            .ok_or(GuestError::NoConfirmedDocument)?
            .clone();
        match rebase_pending(&pending, &confirmed, &self.namespace, self.role)? {
            PendingRebase::Rebased {
                desired,
                txn,
                changes,
            } => {
                let counter = self
                    .next_client_counter
                    .ok_or(GuestError::ClientCounterExhausted)?;
                let client_op_id = crate::ClientOpId {
                    peer_id: self.peer_id.clone(),
                    local_counter: counter,
                };
                let base_seq = self.confirmed_seq.ok_or(GuestError::NoConfirmedDocument)?;
                self.next_client_counter = counter.checked_add(1);
                pending.client_op_id = client_op_id.clone();
                pending.base_seq = base_seq;
                pending.before = confirmed;
                pending.desired_after = Arc::new(desired);
                pending.submitted_txn = txn.clone();
                pending.changes = changes;
                pending.status = PendingEditStatus::Submitted;
                pending.deferred_reject = None;
                self.displayed_document = Some(pending.desired_after.clone());
                self.pending = Some(pending);
                Ok(vec![GuestEffect::Send(CollabMessage::Submit(Submit {
                    client_op_id,
                    base_seq,
                    txn,
                }))])
            }
            PendingRebase::AlreadySatisfied { desired } => {
                self.displayed_document = Some(Arc::new(desired));
                Ok(vec![GuestEffect::PendingCancelled {
                    client_op_id: pending.client_op_id,
                    reason: PendingCancelReason::AlreadySatisfied,
                }])
            }
            PendingRebase::Conflict(reason) => {
                self.stage_pending_rollback(pending.client_op_id, reason)
            }
        }
    }

    pub(crate) fn stage_pending_rollback(
        &mut self,
        client_op_id: crate::ClientOpId,
        reason: PendingCancelReason,
    ) -> Result<Vec<GuestEffect>, GuestError> {
        let confirmed = self
            .confirmed_document
            .as_ref()
            .ok_or(GuestError::NoConfirmedDocument)?
            .clone();
        let confirmed_seq = self.confirmed_seq.ok_or(GuestError::NoConfirmedDocument)?;
        let confirmed_hash = self.confirmed_hash.ok_or(GuestError::NoConfirmedDocument)?;
        let candidate = confirmed.as_ref().clone();
        let effect = self.stage_install(
            crate::GuestInstallReason::RejectRollback,
            candidate,
            crate::guest_types::GuestPostInstall {
                confirmed_document: confirmed.clone(),
                confirmed_seq,
                confirmed_hash,
                displayed_document: confirmed,
                pending: None,
                state: self.state,
                discard_buffer_through: None,
                pending_cancel: Some((client_op_id, reason)),
                undo_index_add: None,
            },
        )?;
        Ok(vec![effect])
    }
}

pub(crate) fn rebase_pending(
    pending: &PendingEdit,
    confirmed: &PenDocument,
    namespace: &crate::PeerNamespace,
    role: Role,
) -> Result<PendingRebase, GuestError> {
    let desired = match &pending.changes {
        EditChanges::Property(changes) => {
            let Some(desired) = rebase_property_changes(confirmed, changes)? else {
                let conflict = property_conflict(confirmed, changes)?;
                return Ok(PendingRebase::Conflict(conflict));
            };
            desired
        }
        EditChanges::Structure(_) => {
            let mut context = ApplyContext::for_peer_namespace_at(
                namespace.clone(),
                role,
                pending.id_counter_before,
            );
            context.limits = crate::ApplyLimits::default();
            match apply_txn(confirmed, &pending.submitted_txn, &context) {
                Ok(desired) => desired,
                Err(_) => {
                    return Ok(PendingRebase::Conflict(
                        PendingCancelReason::StructuralConflict,
                    ));
                }
            }
        }
    };
    let context = DiffContext::new(namespace.clone(), role, pending.id_counter_before);
    match diff_supported(confirmed, &desired, &context) {
        Ok(supported) => Ok(PendingRebase::Rebased {
            desired,
            txn: supported.txn,
            changes: supported.changes,
        }),
        Err(DiffError::NoDocumentChange) => Ok(PendingRebase::AlreadySatisfied { desired }),
        Err(_) => Ok(PendingRebase::Conflict(match &pending.changes {
            EditChanges::Property(changes) => property_conflict(confirmed, changes)?,
            EditChanges::Structure(_) => PendingCancelReason::StructuralConflict,
        })),
    }
}

fn rebase_property_changes(
    confirmed: &PenDocument,
    changes: &[NodeFieldChange],
) -> Result<Option<PenDocument>, GuestError> {
    let current_index = TreeIndex::build(confirmed).map_err(GuestError::UnsupportedEdit)?;
    for change in changes {
        let Some(entry) = current_index.entry(&change.node_id) else {
            return Ok(None);
        };
        if entry.page != change.page {
            return Ok(None);
        }
        let current =
            current_field_value(entry.node, change.field).map_err(GuestError::UnsupportedEdit)?;
        if current != change.before && current != change.desired {
            return Ok(None);
        }
    }

    let mut grouped: BTreeMap<&str, Vec<NodeFieldChange>> = BTreeMap::new();
    for change in changes {
        grouped
            .entry(change.node_id.as_str())
            .or_default()
            .push(change.clone());
    }
    let mut desired = confirmed.clone();
    for (node_id, changes) in grouped {
        let index = TreeIndex::build(&desired).map_err(GuestError::UnsupportedEdit)?;
        let entry = index
            .entry(node_id)
            .ok_or(GuestError::NoConfirmedDocument)?;
        let replacement =
            apply_changes_to_node(entry.node, &changes).map_err(GuestError::UnsupportedEdit)?;
        let operation = CollabOp::ReplaceExact {
            page: entry.page.clone(),
            node_id: node_id.to_owned(),
            expected_hash: canonical_node_hash(entry.node)?,
            node: replacement,
        };
        desired = apply_txn(
            &desired,
            &CollabTxn::new(vec![operation]),
            &ApplyContext::standalone_trusted(),
        )?;
    }
    Ok(Some(desired))
}

fn property_conflict(
    confirmed: &PenDocument,
    changes: &[NodeFieldChange],
) -> Result<PendingCancelReason, GuestError> {
    let index = TreeIndex::build(confirmed).map_err(GuestError::UnsupportedEdit)?;
    for change in changes {
        let Some(entry) = index.entry(&change.node_id) else {
            return Ok(PendingCancelReason::PropertyConflict {
                node_id: change.node_id.clone(),
                field: change.field.wire_name().to_owned(),
            });
        };
        let current =
            current_field_value(entry.node, change.field).map_err(GuestError::UnsupportedEdit)?;
        if entry.page != change.page || (current != change.before && current != change.desired) {
            return Ok(PendingCancelReason::PropertyConflict {
                node_id: change.node_id.clone(),
                field: change.field.wire_name().to_owned(),
            });
        }
    }
    Ok(PendingCancelReason::StructuralConflict)
}

pub(crate) fn carry_pending_over_confirmed(
    pending: &PendingEdit,
    confirmed: &Arc<PenDocument>,
    namespace: &crate::PeerNamespace,
    role: Role,
) -> Result<CarriedPending, GuestError> {
    match rebase_pending(pending, confirmed, namespace, role)? {
        PendingRebase::Rebased {
            desired, changes, ..
        } => {
            let mut rebased = pending.clone();
            rebased.before = confirmed.clone();
            rebased.desired_after = Arc::new(desired);
            rebased.changes = changes;
            Ok((rebased.desired_after.clone(), Some(rebased), None))
        }
        PendingRebase::AlreadySatisfied { desired } => {
            let mut rebased = pending.clone();
            rebased.before = confirmed.clone();
            rebased.desired_after = Arc::new(desired);
            Ok((rebased.desired_after.clone(), Some(rebased), None))
        }
        PendingRebase::Conflict(reason) => Ok((
            confirmed.clone(),
            None,
            Some((pending.client_op_id.clone(), reason)),
        )),
    }
}
