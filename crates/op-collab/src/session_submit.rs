use crate::{
    apply_txn_tracked, canonical_document_hash, canonical_txn_hash, diff_supported,
    session::OwnerSessionCore,
    session_state::{
        reject_code_for_apply, reject_for, CachedSubmitResult, CounterClass, StoredSubmitResult,
    },
    session_types::{PreparedKey, PreparedSourceKey},
    ApplyContext, ApplyLimits, ByeReason, CanonicalHash, ClientOpId, CollabMessage, Commit,
    CommitAuthor, CommitSeq, ConnectionKey, DiffContext, FrameEnvelope, OwnerEffect, PeerId,
    PreparedCommit, RejectCode, Role, SessionError, Submit,
};

impl OwnerSessionCore {
    pub(crate) fn handle_submit(
        &mut self,
        connection: ConnectionKey,
        peer_id: &PeerId,
        submit: Submit,
        document: &jian_ops_schema::PenDocument,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        if submit.client_op_id.peer_id != *peer_id {
            return Err(SessionError::ClientOpPeerMismatch);
        }
        let counter = submit.client_op_id.local_counter;
        let classification = self.classify_counter(peer_id, counter)?;
        match classification {
            CounterClass::Retained(stored) => {
                let frame = FrameEnvelope::new(
                    self.session_id.clone(),
                    self.epoch,
                    CollabMessage::Submit(submit.clone()),
                );
                frame
                    .to_json_vec_with_limits(self.config.wire_limits)
                    .map_err(SessionError::invalid_frame)?;
                let txn_hash = canonical_txn_hash(&submit.txn)?;
                if txn_hash != stored.txn_hash {
                    return Err(SessionError::ClientOpReuse);
                }
                return Ok(vec![stored.result.reply(connection)]);
            }
            CounterClass::Expired => {
                let (_, message) = self.checked_outbound(CollabMessage::Reject(reject_for(
                    &submit.client_op_id,
                    self.seq,
                    RejectCode::ExpiredClientOpId,
                )))?;
                return Ok(vec![OwnerEffect::Reply {
                    to: connection,
                    message,
                }]);
            }
            CounterClass::Gap => {
                let (_, message) = self.checked_outbound(CollabMessage::Reject(reject_for(
                    &submit.client_op_id,
                    self.seq,
                    RejectCode::CounterGap,
                )))?;
                self.mark_connection_closing(connection);
                return Ok(vec![
                    OwnerEffect::Reply {
                        to: connection,
                        message,
                    },
                    OwnerEffect::Close {
                        connection,
                        reason: ByeReason::ProtocolError,
                    },
                ]);
            }
            CounterClass::Current => {}
        }
        if self.prepared.is_some() {
            return Err(SessionError::InstallPending);
        }
        let frame = FrameEnvelope::new(
            self.session_id.clone(),
            self.epoch,
            CollabMessage::Submit(submit.clone()),
        );
        let frame_size = frame
            .to_json_vec_with_limits(self.config.wire_limits)
            .map_err(SessionError::invalid_frame)?
            .len();
        let txn_hash = canonical_txn_hash(&submit.txn)?;
        if frame_size > self.config.session_limits.result_window_bytes_per_peer
            || frame_size > self.config.session_limits.commit_log_bytes
        {
            return self.cache_new_reject(
                connection,
                peer_id,
                &submit.client_op_id,
                txn_hash,
                RejectCode::ResourceLimit,
            );
        }
        let peer = self
            .peers
            .get(peer_id.as_ref())
            .expect("active connection has a retained peer");
        let peer_role = peer.principal.role();
        let peer_namespace = peer.namespace.clone();
        let peer_next_id_counter = peer.next_id_counter;
        let author_participant_id = peer.principal.participant_id().clone();
        let author_peer_id = peer.principal.peer_id().clone();
        if peer_role == Role::Viewer {
            return self.cache_new_reject(
                connection,
                peer_id,
                &submit.client_op_id,
                txn_hash,
                RejectCode::PermissionDenied,
            );
        }
        if submit.base_seq.0 < self.seq.0 {
            return self.cache_new_reject(
                connection,
                peer_id,
                &submit.client_op_id,
                txn_hash,
                RejectCode::StaleBase,
            );
        }
        if submit.base_seq.0 > self.seq.0 {
            return Err(SessionError::FutureBaseSequence);
        }
        let next_seq = self
            .seq
            .0
            .checked_add(1)
            .map(CommitSeq)
            .ok_or(SessionError::SequenceExhausted)?;
        let current_hash = canonical_document_hash(document)?;
        if current_hash != self.document_hash {
            return Err(SessionError::DocumentDrift);
        }
        let mut context = ApplyContext::for_peer_namespace_at(
            peer_namespace.clone(),
            peer_role,
            peer_next_id_counter,
        );
        context.limits = ApplyLimits::from(self.config.wire_limits);
        let applied = match apply_txn_tracked(document, &submit.txn, &context) {
            Ok(candidate) => candidate,
            Err(error) => {
                return self.cache_new_reject(
                    connection,
                    peer_id,
                    &submit.client_op_id,
                    txn_hash,
                    reject_code_for_apply(&error),
                );
            }
        };
        let candidate = applied.document;
        let next_id_counter = applied.next_id_counter;
        let authoritative = match diff_supported(
            document,
            &candidate,
            &DiffContext {
                namespace: peer_namespace,
                role: peer_role,
                next_id_counter: peer_next_id_counter,
                limits: context.limits,
            },
        ) {
            Ok(supported) => supported,
            Err(_) => {
                return self.cache_new_reject(
                    connection,
                    peer_id,
                    &submit.client_op_id,
                    txn_hash,
                    RejectCode::UnsupportedEdit,
                );
            }
        };
        let candidate_hash = canonical_document_hash(&candidate)?;
        let key = PreparedKey {
            peer_id: peer_id.clone(),
            counter,
            txn_hash,
            expected_seq: self.seq,
            source: PreparedSourceKey::Submit,
        };
        let commit = Commit {
            client_op_id: submit.client_op_id,
            seq: next_seq,
            author: CommitAuthor {
                participant_id: author_participant_id,
                peer_id: author_peer_id,
            },
            txn: authoritative.txn,
            doc_hash: candidate_hash,
        };
        let (encoded_commit_size, commit_message) =
            self.checked_outbound(CollabMessage::Commit(commit))?;
        let CollabMessage::Commit(commit) = commit_message else {
            unreachable!("checked Commit must retain its message kind");
        };
        if encoded_commit_size > self.config.session_limits.result_window_bytes_per_peer
            || encoded_commit_size > self.config.session_limits.commit_log_bytes
        {
            return self.cache_new_reject(
                connection,
                peer_id,
                &commit.client_op_id,
                txn_hash,
                RejectCode::ResourceLimit,
            );
        }
        self.prepared = Some(key.clone());
        Ok(vec![OwnerEffect::PrepareInstall(Box::new(
            PreparedCommit {
                key,
                candidate_document: Some(candidate),
                commit,
                encoded_commit_size,
                next_id_counter,
                changes: authoritative.changes,
                undo: None,
            },
        ))])
    }

    fn cache_new_reject(
        &mut self,
        connection: ConnectionKey,
        peer_id: &PeerId,
        client_op_id: &ClientOpId,
        txn_hash: CanonicalHash,
        code: RejectCode,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        let reject = reject_for(client_op_id, self.seq, code);
        let weight = self.validate_outbound(CollabMessage::Reject(reject.clone()))?;
        self.cache_result(
            peer_id,
            client_op_id.local_counter,
            txn_hash,
            CachedSubmitResult::Reject {
                reject: reject.clone(),
                weight,
            },
        )?;
        Ok(vec![OwnerEffect::Reply {
            to: connection,
            message: CollabMessage::Reject(reject),
        }])
    }

    pub(crate) fn cache_result(
        &mut self,
        peer_id: &PeerId,
        counter: u64,
        txn_hash: CanonicalHash,
        result: CachedSubmitResult,
    ) -> Result<(), SessionError> {
        let limits = self.config.session_limits;
        let peer = self
            .peers
            .get_mut(peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        if peer.next_counter != Some(counter) {
            return Err(SessionError::PreparedCommitMismatch);
        }
        peer.next_counter = counter.checked_add(1);
        peer.result_bytes = peer.result_bytes.saturating_add(result.weight());
        peer.results
            .insert(counter, StoredSubmitResult { txn_hash, result });
        while peer.results.len() > limits.result_window_entries
            || peer.result_bytes > limits.result_window_bytes_per_peer
        {
            let Some((&oldest, _)) = peer.results.first_key_value() else {
                break;
            };
            let removed = peer.results.remove(&oldest).expect("oldest result exists");
            peer.result_bytes = peer.result_bytes.saturating_sub(removed.result.weight());
            peer.retired_floor = peer.retired_floor.max(oldest.saturating_add(1));
        }
        Ok(())
    }

    fn classify_counter(
        &self,
        peer_id: &PeerId,
        counter: u64,
    ) -> Result<CounterClass, SessionError> {
        let peer = self
            .peers
            .get(peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        if counter < peer.retired_floor {
            return Ok(CounterClass::Expired);
        }
        if let Some(stored) = peer.results.get(&counter) {
            return Ok(CounterClass::Retained(stored.clone()));
        }
        match peer.next_counter {
            Some(next) if counter == next => Ok(CounterClass::Current),
            Some(next) if counter > next => Ok(CounterClass::Gap),
            Some(_) | None => Ok(CounterClass::Expired),
        }
    }
}
