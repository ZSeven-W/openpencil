use crate::{
    canonical_document_hash, canonical_txn_hash, diff_supported,
    session::OwnerSessionCore,
    session_state::{StoredUndoResult, UndoCounterClass},
    session_types::{PreparedKey, PreparedSourceKey, PreparedUndoFinalize},
    ApplyLimits, ByeReason, ClientOpId, CollabMessage, Commit, CommitAuthor, CommitSeq,
    ConnectionKey, DiffContext, EditChanges, FrameEnvelope, OwnerEffect, PeerId, PreparedCommit,
    Role, SessionError, UndoOutcome, UndoRequest, UndoResult,
};

impl OwnerSessionCore {
    /// Retained, successfully committed owner-local property operations.
    ///
    /// The order is oldest to newest. Structural commits, rejected submits,
    /// compensation commits, and already-resolved targets are excluded.
    pub fn own_undo_targets(&self) -> Vec<ClientOpId> {
        self.undo_ledger.targets_for_peer(&self.owner_peer_id)
    }

    pub fn latest_own_undo_target(&self) -> Option<ClientOpId> {
        self.own_undo_targets().pop()
    }

    /// Allocate the next owner-local undo idempotency token without consuming
    /// it. The host retains this value until the resulting install is
    /// finalized or aborted, and may replay the exact token after uncertainty.
    pub fn next_own_undo_request(
        &self,
        target_client_op_id: ClientOpId,
    ) -> Result<UndoRequest, SessionError> {
        if !self
            .own_undo_targets()
            .iter()
            .any(|target| target == &target_client_op_id)
        {
            return Err(SessionError::OwnUndoTargetUnavailable);
        }
        let peer = self
            .peers
            .get(self.owner_peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        let counter = peer
            .next_undo_counter
            .ok_or(SessionError::UndoCounterExhausted)?;
        Ok(UndoRequest {
            request_id: crate::UndoRequestId {
                peer_id: self.owner_peer_id.clone(),
                local_counter: counter,
            },
            target_client_op_id,
        })
    }

    /// Route an owner-local undo token through the same authenticated frame,
    /// strict-counter, ledger, and two-phase install path as a remote peer.
    pub fn request_own_undo(
        &mut self,
        request: UndoRequest,
        document: &jian_ops_schema::PenDocument,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        let connection = self
            .peers
            .get(self.owner_peer_id.as_ref())
            .and_then(|peer| peer.connection)
            .ok_or(SessionError::UnknownConnection)?;
        self.accept_frame(
            connection,
            FrameEnvelope::new(
                self.session_id.clone(),
                self.epoch,
                CollabMessage::UndoRequest(request),
            ),
            document,
        )
    }

    pub(crate) fn handle_undo_request(
        &mut self,
        connection: ConnectionKey,
        peer_id: &PeerId,
        request: UndoRequest,
        document: &jian_ops_schema::PenDocument,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        let counter = request.request_id.local_counter;
        match self.classify_undo_counter(peer_id, counter)? {
            UndoCounterClass::Retained(stored) => {
                if stored.target_client_op_id != request.target_client_op_id {
                    return Err(SessionError::UndoRequestReuse);
                }
                return Ok(vec![OwnerEffect::Reply {
                    to: connection,
                    message: CollabMessage::UndoResult(stored.result),
                }]);
            }
            UndoCounterClass::Expired => {
                return self.reply_uncached_undo(
                    connection,
                    request,
                    UndoOutcome::Rejected,
                    "undo request id is outside the retained result window",
                );
            }
            UndoCounterClass::Gap => {
                let mut effects = self.reply_uncached_undo(
                    connection,
                    request,
                    UndoOutcome::Rejected,
                    "undo request counter has a gap",
                )?;
                self.mark_connection_closing(connection);
                effects.push(OwnerEffect::Close {
                    connection,
                    reason: ByeReason::ProtocolError,
                });
                return Ok(effects);
            }
            UndoCounterClass::Current => {}
        }
        if self.prepared.is_some() {
            return Err(SessionError::InstallPending);
        }
        let current_hash = canonical_document_hash(document)?;
        if current_hash != self.document_hash {
            return Err(SessionError::DocumentDrift);
        }

        let peer = self
            .peers
            .get(peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        let peer_role = peer.principal.role();
        let peer_namespace = peer.namespace.clone();
        let peer_next_id_counter = peer.next_id_counter;
        let compensation_counter = peer.next_counter;
        let author = CommitAuthor {
            participant_id: peer.principal.participant_id().clone(),
            peer_id: peer.principal.peer_id().clone(),
        };

        if peer_role == Role::Viewer {
            return self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Rejected,
                None,
                "viewer role cannot undo collaboration edits",
            );
        }
        if request.target_client_op_id.peer_id != *peer_id {
            return self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Rejected,
                None,
                "only this peer's committed property edits can be undone",
            );
        }
        let Some(record) = self.undo_ledger.record(&request.target_client_op_id) else {
            return self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Rejected,
                None,
                "target is not a retained property commit; structural undo and redo are disabled",
            );
        };
        if record.author_peer_id != *peer_id || record.seq > self.seq {
            return self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Rejected,
                None,
                "undo target does not belong to this peer",
            );
        }
        if record.undone {
            return self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Conflict,
                None,
                "target was already selectively undone",
            );
        }
        let Some(compensation_counter) = compensation_counter else {
            return self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Rejected,
                None,
                "client operation counter is exhausted",
            );
        };

        let limits = ApplyLimits::from(self.config.wire_limits);
        let selection = self.undo_ledger.build_selection(
            document,
            &request.target_client_op_id,
            &record,
            limits,
        )?;
        if selection.reverted_fields.is_empty() {
            let target = request.target_client_op_id.clone();
            let effects = self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Conflict,
                None,
                "all target fields have newer writers",
            )?;
            self.undo_ledger.mark_undone(&target);
            return Ok(effects);
        }
        let supported = match diff_supported(
            document,
            &selection.candidate,
            &DiffContext {
                namespace: peer_namespace,
                role: peer_role,
                next_id_counter: peer_next_id_counter,
                limits,
            },
        ) {
            Ok(supported) => supported,
            Err(_) => {
                return self.cache_current_undo_reply(
                    connection,
                    peer_id,
                    request,
                    UndoOutcome::Rejected,
                    None,
                    "selective undo compensation is not supported for the current document",
                );
            }
        };
        if !matches!(supported.changes, EditChanges::Property(_)) {
            return self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Rejected,
                None,
                "selective undo unexpectedly required a structural edit",
            );
        }
        let next_seq = self
            .seq
            .0
            .checked_add(1)
            .map(CommitSeq)
            .ok_or(SessionError::SequenceExhausted)?;
        let compensation_client_op_id = ClientOpId {
            peer_id: peer_id.clone(),
            local_counter: compensation_counter,
        };
        let txn_hash = canonical_txn_hash(&supported.txn)?;
        let commit = Commit {
            client_op_id: compensation_client_op_id.clone(),
            seq: next_seq,
            author,
            txn: supported.txn,
            doc_hash: canonical_document_hash(&selection.candidate)?,
        };
        let (encoded_commit_size, message) =
            self.checked_outbound(CollabMessage::Commit(commit))?;
        let CollabMessage::Commit(commit) = message else {
            unreachable!("checked Commit must retain its message kind");
        };
        if encoded_commit_size > self.config.session_limits.result_window_bytes_per_peer
            || encoded_commit_size > self.config.session_limits.commit_log_bytes
        {
            return self.cache_current_undo_reply(
                connection,
                peer_id,
                request,
                UndoOutcome::Rejected,
                None,
                "selective undo compensation exceeds the session resource limit",
            );
        }
        let details = (selection.reverted_fields.len() < selection.total_fields).then(|| {
            format!(
                "reverted {} of {} fields; skipped fields have newer writers",
                selection.reverted_fields.len(),
                selection.total_fields
            )
        });
        let result = UndoResult {
            request_id: request.request_id.clone(),
            target_client_op_id: request.target_client_op_id.clone(),
            owner_seq: next_seq,
            outcome: UndoOutcome::Committed,
            compensation_client_op_id: Some(compensation_client_op_id),
            details,
        };
        self.validate_outbound(CollabMessage::UndoResult(result.clone()))?;
        let key = PreparedKey {
            peer_id: peer_id.clone(),
            counter: compensation_counter,
            txn_hash,
            expected_seq: self.seq,
            source: PreparedSourceKey::Undo {
                request_id: request.request_id,
                target_client_op_id: request.target_client_op_id,
            },
        };
        self.prepared = Some(key.clone());
        Ok(vec![OwnerEffect::PrepareInstall(Box::new(
            PreparedCommit {
                key,
                candidate_document: Some(selection.candidate),
                commit,
                encoded_commit_size,
                next_id_counter: peer_next_id_counter,
                changes: supported.changes,
                undo: Some(PreparedUndoFinalize {
                    reply_to: connection,
                    result,
                    reverted_fields: selection.reverted_fields,
                }),
            },
        ))])
    }

    fn reply_uncached_undo(
        &self,
        connection: ConnectionKey,
        request: UndoRequest,
        outcome: UndoOutcome,
        details: &'static str,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        let result = self.undo_result(request, outcome, None, Some(details.to_owned()));
        let (_, message) = self.checked_outbound(CollabMessage::UndoResult(result))?;
        Ok(vec![OwnerEffect::Reply {
            to: connection,
            message,
        }])
    }

    fn cache_current_undo_reply(
        &mut self,
        connection: ConnectionKey,
        peer_id: &PeerId,
        request: UndoRequest,
        outcome: UndoOutcome,
        compensation_client_op_id: Option<ClientOpId>,
        details: &'static str,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        let counter = request.request_id.local_counter;
        let target = request.target_client_op_id.clone();
        let result = self.undo_result(
            request,
            outcome,
            compensation_client_op_id,
            Some(details.to_owned()),
        );
        self.cache_undo_result(peer_id, counter, target, result.clone())?;
        Ok(vec![OwnerEffect::Reply {
            to: connection,
            message: CollabMessage::UndoResult(result),
        }])
    }

    fn undo_result(
        &self,
        request: UndoRequest,
        outcome: UndoOutcome,
        compensation_client_op_id: Option<ClientOpId>,
        details: Option<String>,
    ) -> UndoResult {
        UndoResult {
            request_id: request.request_id,
            target_client_op_id: request.target_client_op_id,
            owner_seq: self.seq,
            outcome,
            compensation_client_op_id,
            details,
        }
    }

    pub(crate) fn cache_undo_result(
        &mut self,
        peer_id: &PeerId,
        counter: u64,
        target_client_op_id: ClientOpId,
        result: UndoResult,
    ) -> Result<(), SessionError> {
        let weight = self.validate_outbound(CollabMessage::UndoResult(result.clone()))?;
        let limits = self.config.session_limits;
        let peer = self
            .peers
            .get_mut(peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        if peer.next_undo_counter != Some(counter) {
            return Err(SessionError::PreparedCommitMismatch);
        }
        peer.next_undo_counter = counter.checked_add(1);
        peer.undo_result_bytes = peer.undo_result_bytes.saturating_add(weight);
        peer.undo_results.insert(
            counter,
            StoredUndoResult {
                target_client_op_id,
                result,
                weight,
            },
        );
        while peer.undo_results.len() > limits.result_window_entries
            || peer.undo_result_bytes > limits.result_window_bytes_per_peer
        {
            let Some((&oldest, _)) = peer.undo_results.first_key_value() else {
                break;
            };
            let removed = peer
                .undo_results
                .remove(&oldest)
                .expect("oldest undo result exists");
            peer.undo_result_bytes = peer.undo_result_bytes.saturating_sub(removed.weight);
            peer.undo_retired_floor = peer.undo_retired_floor.max(oldest.saturating_add(1));
        }
        Ok(())
    }

    pub(crate) fn classify_undo_counter(
        &self,
        peer_id: &PeerId,
        counter: u64,
    ) -> Result<UndoCounterClass, SessionError> {
        let peer = self
            .peers
            .get(peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        if counter < peer.undo_retired_floor {
            return Ok(UndoCounterClass::Expired);
        }
        if let Some(stored) = peer.undo_results.get(&counter) {
            return Ok(UndoCounterClass::Retained(stored.clone()));
        }
        match peer.next_undo_counter {
            Some(next) if counter == next => Ok(UndoCounterClass::Current),
            Some(next) if counter > next => Ok(UndoCounterClass::Gap),
            Some(_) | None => Ok(UndoCounterClass::Expired),
        }
    }
}
