use std::sync::Arc;

use crate::{
    apply_txn, canonical_document_hash,
    guest::{BufferedCommit, GuestSessionCore},
    guest_pending::carry_pending_over_confirmed,
    guest_types::{CompletedUndoResult, ExpectedCompensation, GuestPostInstall, UndoIndexEntry},
    id_high_water::document_contains_namespace,
    verify_snapshot, ApplyContext, ApplyLimits, ByeReason, CollabMessage, Commit, FrameEnvelope,
    GuestConnectionState, GuestEffect, GuestError, GuestInstallReason, Participant,
    ParticipantLeft, PendingCancelReason, PendingEditStatus, Reject, RejectCode, Role, UndoOutcome,
    UndoResult,
};

impl GuestSessionCore {
    /// Accept one owner-authenticated frame.
    ///
    /// Presence and roster frames may proceed while an editor install is
    /// pending. The host must queue Commit, Snapshot, and Reject frames until
    /// [`Self::install_pending`] becomes false.
    pub fn accept_frame(&mut self, frame: FrameEnvelope) -> Result<Vec<GuestEffect>, GuestError> {
        if self.state == GuestConnectionState::Ended {
            return Err(GuestError::SessionEnded);
        }
        if self.state == GuestConnectionState::Disconnected {
            return Err(GuestError::Disconnected);
        }
        if frame.session_id() != &self.session_id {
            return Err(GuestError::WrongSession);
        }
        if frame.epoch() != self.epoch {
            return Err(GuestError::WrongEpoch);
        }
        if !owner_to_guest_message(frame.body()) {
            return Err(GuestError::WrongDirection {
                kind: frame.body().kind(),
            });
        }
        if self.install_pending()
            && matches!(
                frame.body(),
                CollabMessage::Commit(_)
                    | CollabMessage::Snapshot(_)
                    | CollabMessage::Reject(_)
                    | CollabMessage::Welcome(_)
            )
        {
            return Err(GuestError::InstallPending);
        }
        let encoded_size = frame
            .encoded_len_with_limits(self.wire_limits)
            .map_err(GuestError::InvalidFrame)?;
        match frame.into_body() {
            CollabMessage::Welcome(welcome) => {
                self.resume(self.session_id.clone(), self.epoch, welcome)
            }
            CollabMessage::Snapshot(snapshot) => self.handle_snapshot(*snapshot),
            CollabMessage::Commit(commit) => self.handle_commit(commit, encoded_size),
            CollabMessage::Reject(reject) => self.handle_reject(reject),
            CollabMessage::ParticipantJoined(participant) => {
                self.handle_participant_joined(participant)
            }
            CollabMessage::ParticipantLeft(left) => self.handle_participant_left(left),
            CollabMessage::PresenceChanged(presence) => {
                let participant = self
                    .participants
                    .get(presence.peer_id.as_ref())
                    .ok_or(GuestError::RosterConflict)?;
                if participant.participant_id != presence.participant_id {
                    return Err(GuestError::RosterConflict);
                }
                Ok(vec![GuestEffect::PresenceChanged(presence)])
            }
            CollabMessage::UndoResult(result) => self.handle_undo_result(result),
            CollabMessage::RenewTicket(renewal) => Ok(vec![GuestEffect::VerifyRenewal {
                ticket: renewal.opaque_ticket,
            }]),
            CollabMessage::Bye(bye) => {
                self.state = GuestConnectionState::Ended;
                Ok(vec![GuestEffect::SessionEnded { reason: bye.reason }])
            }
            CollabMessage::Submit(_)
            | CollabMessage::CatchUp(_)
            | CollabMessage::Applied(_)
            | CollabMessage::UndoRequest(_)
            | CollabMessage::PresenceUpdate(_) => {
                unreachable!("direction was checked before dispatch")
            }
        }
    }

    fn handle_snapshot(
        &mut self,
        snapshot: crate::Snapshot,
    ) -> Result<Vec<GuestEffect>, GuestError> {
        let limits = ApplyLimits::from(self.wire_limits);
        verify_snapshot(&snapshot, &limits)?;
        if snapshot.document.version != self.document_schema_version {
            return Err(GuestError::SnapshotSchemaMismatch);
        }
        if let Some(confirmed) = self.confirmed_seq {
            if snapshot.seq < confirmed {
                return Err(GuestError::SnapshotSequenceRegressed);
            }
            if snapshot.seq == confirmed {
                if self.confirmed_hash == Some(snapshot.doc_hash) {
                    return Ok(vec![GuestEffect::Send(CollabMessage::Applied(
                        crate::Applied {
                            through_seq: confirmed,
                        },
                    ))]);
                }
                return Err(GuestError::CommitSequenceCollision { seq: snapshot.seq });
            }
        } else {
            if snapshot.seq != self.advertised_seq {
                return Err(GuestError::InitialSnapshotSequenceMismatch);
            }
            if document_contains_namespace(&snapshot.document, &self.namespace, &limits)? {
                return Err(GuestError::NamespaceAlreadyPresentInInitialSnapshot);
            }
        }

        let confirmed = Arc::new(snapshot.document);
        let (displayed, pending, pending_cancel) = if let Some(pending) = &self.pending {
            carry_pending_over_confirmed(pending, &confirmed, &self.namespace, self.role)?
        } else {
            (confirmed.clone(), None, None)
        };
        let candidate = displayed.as_ref().clone();
        let post = GuestPostInstall {
            confirmed_document: confirmed,
            confirmed_seq: snapshot.seq,
            confirmed_hash: snapshot.doc_hash,
            displayed_document: displayed,
            pending,
            state: GuestConnectionState::Active,
            discard_buffer_through: Some(snapshot.seq),
            pending_cancel,
            undo_index_add: None,
        };
        Ok(vec![self.stage_install(
            GuestInstallReason::Snapshot,
            candidate,
            post,
        )?])
    }

    fn handle_commit(
        &mut self,
        commit: Commit,
        encoded_size: usize,
    ) -> Result<Vec<GuestEffect>, GuestError> {
        self.validate_commit_author(&commit)?;
        if commit.seq.0 == 0 {
            return Err(GuestError::InvalidCommitSequence);
        }
        let Some(confirmed_seq) = self.confirmed_seq else {
            self.buffer_commit(commit, encoded_size)?;
            return Ok(Vec::new());
        };
        if commit.seq <= confirmed_seq {
            if self.replay_pending_after_resume
                && commit.author.peer_id == self.peer_id
                && self
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.client_op_id == commit.client_op_id)
            {
                self.resolve_covered_pending_commit(&commit)?;
            }
            return Ok(vec![GuestEffect::Send(CollabMessage::Applied(
                crate::Applied {
                    through_seq: confirmed_seq,
                },
            ))]);
        }
        if commit.seq.0 != confirmed_seq.0.saturating_add(1) {
            self.buffer_commit(commit, encoded_size)?;
            if self.catch_up_requested_after.is_none() {
                return Ok(vec![self.catch_up_effect(confirmed_seq)]);
            }
            return Ok(Vec::new());
        }
        if self.waiting_for_undo_result(&commit) {
            self.buffer_commit(commit, encoded_size)?;
            return Ok(Vec::new());
        }
        Ok(vec![self.prepare_commit_install(commit)?])
    }

    pub(crate) fn prepare_commit_install(
        &mut self,
        commit: Commit,
    ) -> Result<GuestEffect, GuestError> {
        self.validate_commit_author(&commit)?;
        let confirmed = self
            .confirmed_document
            .as_ref()
            .ok_or(GuestError::NoConfirmedDocument)?;
        let confirmed_seq = self.confirmed_seq.ok_or(GuestError::NoConfirmedDocument)?;
        if commit.seq.0 != confirmed_seq.0.saturating_add(1) {
            return Err(GuestError::CommitSequenceCollision { seq: commit.seq });
        }
        let mut context = ApplyContext::standalone_trusted();
        context.limits = ApplyLimits::from(self.wire_limits);
        let candidate_confirmed = Arc::new(apply_txn(confirmed, &commit.txn, &context)?);
        let actual = canonical_document_hash(&candidate_confirmed)?;
        if actual != commit.doc_hash {
            return Err(GuestError::CommitHashMismatch {
                expected: commit.doc_hash,
                actual,
            });
        }

        let own_commit = commit.author.peer_id == self.peer_id;
        let (displayed, pending, pending_cancel, reason, undo_index_add) = if own_commit {
            if let Some(pending) = self.pending.as_ref() {
                if pending.client_op_id != commit.client_op_id
                    || canonical_document_hash(&pending.desired_after)? != commit.doc_hash
                {
                    return Err(GuestError::PendingCommitMismatch);
                }
                let undo_index_add = matches!(pending.changes, crate::EditChanges::Property(_))
                    .then(|| UndoIndexEntry {
                        client_op_id: commit.client_op_id.clone(),
                        seq: commit.seq,
                    });
                (
                    candidate_confirmed.clone(),
                    None,
                    None,
                    GuestInstallReason::OwnCommit,
                    undo_index_add,
                )
            } else {
                let expected = self
                    .expected_compensation
                    .as_ref()
                    .ok_or(GuestError::PendingCommitMismatch)?;
                if expected.client_op_id != commit.client_op_id || expected.seq != commit.seq {
                    return Err(GuestError::UndoCompensationMismatch);
                }
                (
                    candidate_confirmed.clone(),
                    None,
                    None,
                    GuestInstallReason::UndoCompensation,
                    None,
                )
            }
        } else if let Some(pending) = &self.pending {
            let (displayed, pending, cancel) = carry_pending_over_confirmed(
                pending,
                &candidate_confirmed,
                &self.namespace,
                self.role,
            )?;
            (
                displayed,
                pending,
                cancel,
                GuestInstallReason::RemoteCommit,
                None,
            )
        } else {
            (
                candidate_confirmed.clone(),
                None,
                None,
                GuestInstallReason::RemoteCommit,
                None,
            )
        };
        let candidate = displayed.as_ref().clone();
        let post = GuestPostInstall {
            confirmed_document: candidate_confirmed,
            confirmed_seq: commit.seq,
            confirmed_hash: commit.doc_hash,
            displayed_document: displayed,
            pending,
            state: GuestConnectionState::Active,
            discard_buffer_through: None,
            pending_cancel,
            undo_index_add,
        };
        self.stage_install(reason, candidate, post)
    }

    fn handle_undo_result(&mut self, result: UndoResult) -> Result<Vec<GuestEffect>, GuestError> {
        if result.request_id.peer_id != self.peer_id
            || result.target_client_op_id.peer_id != self.peer_id
        {
            return Err(GuestError::UndoResultMismatch);
        }
        if let Some(completed) = self
            .completed_undo_results
            .iter()
            .find(|completed| completed.has_request_id(&result.request_id))
        {
            return if completed.matches(&result) {
                Ok(vec![GuestEffect::UndoResult(result)])
            } else {
                Err(GuestError::UndoResultMismatch)
            };
        }
        let pending = self
            .pending_undo
            .as_ref()
            .ok_or(GuestError::UndoResultMismatch)?;
        if pending.request_id != result.request_id
            || pending.target_client_op_id != result.target_client_op_id
        {
            return Err(GuestError::UndoResultMismatch);
        }
        let target = self
            .undo_index
            .iter()
            .find(|entry| entry.client_op_id == result.target_client_op_id)
            .ok_or(GuestError::UnknownUndoTarget)?;
        if result.owner_seq < target.seq {
            return Err(GuestError::UndoResultMismatch);
        }

        match result.outcome {
            UndoOutcome::Committed => {
                let compensation = result
                    .compensation_client_op_id
                    .as_ref()
                    .ok_or(GuestError::UndoCompensationMismatch)?;
                let next = self
                    .next_client_counter
                    .ok_or(GuestError::UndoCompensationMismatch)?;
                if compensation.peer_id != self.peer_id
                    || compensation.local_counter != next
                    || result.owner_seq.0 == 0
                {
                    return Err(GuestError::UndoCompensationMismatch);
                }
                self.next_client_counter = next.checked_add(1);
                if self
                    .confirmed_seq
                    .is_none_or(|confirmed| confirmed < result.owner_seq)
                {
                    self.expected_compensation = Some(ExpectedCompensation {
                        client_op_id: compensation.clone(),
                        seq: result.owner_seq,
                    });
                }
            }
            UndoOutcome::Conflict | UndoOutcome::Rejected => {
                if result.compensation_client_op_id.is_some() {
                    return Err(GuestError::UndoResultMismatch);
                }
            }
        }
        self.undo_index
            .retain(|entry| entry.client_op_id != result.target_client_op_id);
        self.pending_undo = None;
        self.completed_undo_results
            .push_back(CompletedUndoResult::from_result(&result));
        while self.completed_undo_results.len() > self.config.undo_index_entries {
            self.completed_undo_results.pop_front();
        }

        let mut effects = vec![GuestEffect::UndoResult(result)];
        if self.state == GuestConnectionState::Active && !self.install_pending() {
            effects.extend(self.continue_after_install()?);
        }
        Ok(effects)
    }

    fn handle_reject(&mut self, reject: Reject) -> Result<Vec<GuestEffect>, GuestError> {
        if reject.client_op_id.peer_id != self.peer_id {
            return Err(GuestError::RejectPeerMismatch);
        }
        let Some(pending) = self.pending.as_mut() else {
            return Ok(Vec::new());
        };
        if pending.client_op_id != reject.client_op_id {
            return Ok(Vec::new());
        }
        let confirmed_seq = self.confirmed_seq.ok_or(GuestError::NoConfirmedDocument)?;
        if reject.owner_seq > confirmed_seq {
            pending.status = PendingEditStatus::AwaitingCatchUp;
            pending.deferred_reject = Some(reject);
            if self.catch_up_requested_after.is_none() {
                return Ok(vec![self.catch_up_effect(confirmed_seq)]);
            }
            return Ok(Vec::new());
        }

        let pending = self.pending.take().expect("matching pending exists");
        self.replay_pending_after_resume = false;
        self.pending_replay_sent = false;
        if reject.code == RejectCode::StaleBase {
            self.retry_stale_pending(pending)
        } else {
            self.stage_pending_rollback(
                pending.client_op_id,
                PendingCancelReason::Rejected(reject.code),
            )
        }
    }

    pub(crate) fn continue_after_install(&mut self) -> Result<Vec<GuestEffect>, GuestError> {
        if self.state != GuestConnectionState::Active {
            return Ok(Vec::new());
        }
        let Some(confirmed_seq) = self.confirmed_seq else {
            return Ok(Vec::new());
        };
        let next = confirmed_seq.0.saturating_add(1);
        if self
            .buffered_commits
            .get(&next)
            .is_some_and(|buffered| self.waiting_for_undo_result(&buffered.commit))
        {
            return Ok(Vec::new());
        }
        if let Some(buffered) = self.buffered_commits.remove(&next) {
            self.buffered_commit_bytes = self
                .buffered_commit_bytes
                .saturating_sub(buffered.encoded_size);
            return Ok(vec![self.prepare_commit_install(buffered.commit)?]);
        }

        let deferred_ready = self.pending.as_ref().is_some_and(|pending| {
            pending
                .deferred_reject
                .as_ref()
                .is_some_and(|reject| reject.owner_seq <= confirmed_seq)
        });
        if deferred_ready {
            self.replay_pending_after_resume = false;
            self.pending_replay_sent = false;
            let mut pending = self.pending.take().expect("deferred pending exists");
            let reject = pending
                .deferred_reject
                .take()
                .expect("deferred reject was checked");
            if reject.code == RejectCode::StaleBase {
                return self.retry_stale_pending(pending);
            }
            return self.stage_pending_rollback(
                pending.client_op_id,
                PendingCancelReason::Rejected(reject.code),
            );
        }

        if self
            .buffered_commits
            .first_key_value()
            .is_some_and(|(seq, _)| *seq > next)
            && self.catch_up_requested_after.is_none()
        {
            return Ok(vec![self.catch_up_effect(confirmed_seq)]);
        }
        if let Some(replay) = self.replay_pending_submit_after_resume() {
            return Ok(vec![replay]);
        }
        Ok(Vec::new())
    }

    fn resolve_covered_pending_commit(&mut self, commit: &Commit) -> Result<(), GuestError> {
        let pending = self
            .pending
            .as_ref()
            .expect("matching resumed pending was checked");
        if pending.submitted_txn != commit.txn {
            return Err(GuestError::PendingCommitMismatch);
        }
        let undoable = matches!(pending.changes, crate::EditChanges::Property(_));
        self.pending = None;
        self.displayed_document = self.confirmed_document.clone();
        self.replay_pending_after_resume = false;
        self.pending_replay_sent = false;
        if undoable
            && !self
                .undo_index
                .iter()
                .any(|entry| entry.client_op_id == commit.client_op_id)
        {
            self.undo_index.push_back(UndoIndexEntry {
                client_op_id: commit.client_op_id.clone(),
                seq: commit.seq,
            });
            while self.undo_index.len() > self.config.undo_index_entries {
                self.undo_index.pop_front();
            }
        }
        Ok(())
    }

    fn buffer_commit(&mut self, commit: Commit, encoded_size: usize) -> Result<(), GuestError> {
        if let Some(existing) = self.buffered_commits.get(&commit.seq.0) {
            if existing.commit == commit {
                return Ok(());
            }
            return Err(GuestError::CommitSequenceCollision { seq: commit.seq });
        }
        if self.buffered_commits.len() >= self.config.commit_buffer_entries {
            return Err(GuestError::CommitBufferEntriesExceeded {
                limit: self.config.commit_buffer_entries,
            });
        }
        if self
            .buffered_commit_bytes
            .checked_add(encoded_size)
            .is_none_or(|bytes| bytes > self.config.commit_buffer_bytes)
        {
            return Err(GuestError::CommitBufferBytesExceeded {
                limit: self.config.commit_buffer_bytes,
            });
        }
        self.buffered_commit_bytes += encoded_size;
        self.buffered_commits.insert(
            commit.seq.0,
            BufferedCommit {
                commit,
                encoded_size,
            },
        );
        Ok(())
    }

    fn waiting_for_undo_result(&self, commit: &Commit) -> bool {
        commit.author.peer_id == self.peer_id
            && self.pending.is_none()
            && self.pending_undo.is_some()
            && self.expected_compensation.is_none()
    }

    fn validate_commit_author(&self, commit: &Commit) -> Result<(), GuestError> {
        let participant = self
            .participants
            .get(commit.author.peer_id.as_ref())
            .ok_or(GuestError::UnknownCommitAuthor)?;
        if participant.participant_id != commit.author.participant_id
            || participant.role == Role::Viewer
            || commit.client_op_id.peer_id != commit.author.peer_id
        {
            return Err(GuestError::UnknownCommitAuthor);
        }
        Ok(())
    }

    fn handle_participant_joined(
        &mut self,
        participant: Participant,
    ) -> Result<Vec<GuestEffect>, GuestError> {
        if let Some(existing) = self.participants.get(participant.peer_id.as_ref()) {
            return if existing == &participant {
                Ok(Vec::new())
            } else {
                Err(GuestError::RosterConflict)
            };
        }
        if self
            .participants
            .values()
            .any(|existing| existing.participant_id == participant.participant_id)
            || self.participants.len() >= crate::MAX_SESSION_PARTICIPANTS
            || (participant.role == Role::Owner
                && self
                    .participants
                    .values()
                    .any(|existing| existing.role == Role::Owner))
        {
            return Err(GuestError::RosterConflict);
        }
        self.participants
            .insert(participant.peer_id.as_ref().to_owned(), participant.clone());
        Ok(vec![GuestEffect::ParticipantJoined(participant)])
    }

    fn handle_participant_left(
        &mut self,
        left: ParticipantLeft,
    ) -> Result<Vec<GuestEffect>, GuestError> {
        let participant = self
            .participants
            .get(left.peer_id.as_ref())
            .ok_or(GuestError::RosterConflict)?;
        if participant.participant_id != left.participant_id {
            return Err(GuestError::RosterConflict);
        }
        let owner_left = participant.role == Role::Owner;
        let self_left = participant.peer_id == self.peer_id;
        self.participants.remove(left.peer_id.as_ref());
        let mut effects = vec![GuestEffect::ParticipantLeft(left)];
        if owner_left || self_left {
            self.state = GuestConnectionState::Ended;
            effects.push(GuestEffect::SessionEnded {
                reason: if owner_left {
                    ByeReason::OwnerLeft
                } else {
                    ByeReason::Normal
                },
            });
        }
        Ok(effects)
    }
}

fn owner_to_guest_message(message: &CollabMessage) -> bool {
    matches!(
        message,
        CollabMessage::Welcome(_)
            | CollabMessage::Commit(_)
            | CollabMessage::Reject(_)
            | CollabMessage::Snapshot(_)
            | CollabMessage::UndoResult(_)
            | CollabMessage::RenewTicket(_)
            | CollabMessage::PresenceChanged(_)
            | CollabMessage::ParticipantJoined(_)
            | CollabMessage::ParticipantLeft(_)
            | CollabMessage::Bye(_)
    )
}
