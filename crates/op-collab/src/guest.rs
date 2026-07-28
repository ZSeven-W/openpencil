use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
    sync::Arc,
};

use jian_ops_schema::PenDocument;

use crate::{
    guest_types::{
        CompletedUndoResult, ExpectedCompensation, GuestPostInstall, PreparedGuestKey,
        UndoIndexEntry,
    },
    CanonicalHash, ClientOpId, CollabMessage, Commit, CommitSeq, Epoch, FrameEnvelope,
    GuestConnectionState, GuestEffect, GuestError, GuestSessionConfig, Participant, ParticipantId,
    PeerId, PeerNamespace, PendingEdit, PendingEditStatus, PreparedGuestInstall, Role, SessionId,
    Submit, UndoRequest, Welcome, WireLimits, CANONICAL_HASH_VERSION, FIRST_CLIENT_OP_COUNTER,
    MAX_GUEST_COMMIT_BUFFER_BYTES, MAX_GUEST_COMMIT_BUFFER_ENTRIES, MAX_GUEST_UNDO_INDEX_ENTRIES,
    MAX_SESSION_PARTICIPANTS,
};

pub(crate) struct BufferedCommit {
    pub commit: Commit,
    pub encoded_size: usize,
}

/// Pure guest-side confirmed/pending collaboration state.
///
/// The core never mutates editor state directly. Snapshot, commit, and
/// rollback transitions emit a non-cloneable [`PreparedGuestInstall`] token;
/// callers install its candidate synchronously, then finalize with the actual
/// installed hash. Later document frames must be queued while
/// [`Self::install_pending`] is true.
pub struct GuestSessionCore {
    pub(crate) session_id: SessionId,
    pub(crate) epoch: Epoch,
    pub(crate) participant_id: ParticipantId,
    pub(crate) peer_id: PeerId,
    pub(crate) role: Role,
    pub(crate) namespace: PeerNamespace,
    pub(crate) document_schema_version: String,
    pub(crate) wire_limits: WireLimits,
    pub(crate) advertised_seq: CommitSeq,
    pub(crate) config: GuestSessionConfig,
    pub(crate) participants: BTreeMap<String, Participant>,
    pub(crate) state: GuestConnectionState,
    pub(crate) confirmed_document: Option<Arc<PenDocument>>,
    pub(crate) confirmed_seq: Option<CommitSeq>,
    pub(crate) confirmed_hash: Option<CanonicalHash>,
    pub(crate) displayed_document: Option<Arc<PenDocument>>,
    pub(crate) pending: Option<PendingEdit>,
    pub(crate) next_client_counter: Option<u64>,
    pub(crate) next_undo_counter: Option<u64>,
    pub(crate) undo_index: VecDeque<UndoIndexEntry>,
    pub(crate) pending_undo: Option<UndoRequest>,
    pub(crate) expected_compensation: Option<ExpectedCompensation>,
    pub(crate) completed_undo_results: VecDeque<CompletedUndoResult>,
    pub(crate) next_id_counter: Option<u64>,
    pub(crate) buffered_commits: BTreeMap<u64, BufferedCommit>,
    pub(crate) buffered_commit_bytes: usize,
    pub(crate) catch_up_requested_after: Option<CommitSeq>,
    pub(crate) replay_pending_after_resume: bool,
    pub(crate) pending_replay_sent: bool,
    pub(crate) prepared: Option<PreparedGuestKey>,
    pub(crate) install_generation: u64,
}

impl fmt::Debug for GuestSessionCore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GuestSessionCore")
            .field("session_id", &self.session_id)
            .field("epoch", &self.epoch)
            .field("peer_id", &self.peer_id)
            .field("role", &self.role)
            .field("state", &self.state)
            .field("confirmed_seq", &self.confirmed_seq)
            .field("pending", &self.pending.is_some())
            .field("undo_index", &self.undo_index.len())
            .field("pending_undo", &self.pending_undo.is_some())
            .field("buffered_commits", &self.buffered_commits.len())
            .field(
                "replay_pending_after_resume",
                &self.replay_pending_after_resume,
            )
            .field("pending_replay_sent", &self.pending_replay_sent)
            .field("install_pending", &self.prepared.is_some())
            .finish()
    }
}

impl GuestSessionCore {
    pub fn new(
        session_id: SessionId,
        epoch: Epoch,
        welcome: Welcome,
        config: GuestSessionConfig,
    ) -> Result<Self, GuestError> {
        validate_guest_config(config)?;
        let (namespace, participants) = validate_welcome(&session_id, epoch, &welcome)?;
        Ok(Self {
            session_id,
            epoch,
            participant_id: welcome.participant_id,
            peer_id: welcome.peer_id,
            role: welcome.role,
            namespace,
            document_schema_version: welcome.document_schema_version,
            wire_limits: welcome.limits,
            advertised_seq: welcome.seq,
            config,
            participants,
            state: GuestConnectionState::AwaitingSnapshot,
            confirmed_document: None,
            confirmed_seq: None,
            confirmed_hash: None,
            displayed_document: None,
            pending: None,
            next_client_counter: Some(FIRST_CLIENT_OP_COUNTER),
            next_undo_counter: Some(FIRST_CLIENT_OP_COUNTER),
            undo_index: VecDeque::new(),
            pending_undo: None,
            expected_compensation: None,
            completed_undo_results: VecDeque::new(),
            next_id_counter: Some(0),
            buffered_commits: BTreeMap::new(),
            buffered_commit_bytes: 0,
            catch_up_requested_after: None,
            replay_pending_after_resume: false,
            pending_replay_sent: false,
            prepared: None,
            install_generation: 0,
        })
    }

    /// Rebind an existing guest after a short disconnect.
    ///
    /// Only an identical session/epoch and retained self binding may resume.
    /// A different epoch ends this core so its pending edit cannot leak into a
    /// replacement session.
    pub fn resume(
        &mut self,
        session_id: SessionId,
        epoch: Epoch,
        welcome: Welcome,
    ) -> Result<Vec<GuestEffect>, GuestError> {
        if self.state == GuestConnectionState::Ended {
            return Err(GuestError::SessionEnded);
        }
        if session_id != self.session_id {
            self.state = GuestConnectionState::Ended;
            return Err(GuestError::WrongSession);
        }
        if epoch != self.epoch {
            self.state = GuestConnectionState::Ended;
            return Err(GuestError::WrongEpoch);
        }
        if self.install_pending() {
            return Err(GuestError::InstallPending);
        }
        let (namespace, participants) = validate_welcome(&session_id, epoch, &welcome)?;
        if welcome.participant_id != self.participant_id
            || welcome.peer_id != self.peer_id
            || welcome.role != self.role
            || namespace != self.namespace
            || welcome.document_schema_version != self.document_schema_version
            || welcome.limits != self.wire_limits
        {
            return Err(GuestError::InvalidWelcome {
                reason: "same-epoch resume changed the retained self binding",
            });
        }
        if self
            .confirmed_seq
            .is_some_and(|confirmed| welcome.seq < confirmed)
        {
            return Err(GuestError::InvalidWelcome {
                reason: "same-epoch Welcome sequence regressed",
            });
        }
        if self
            .pending
            .as_ref()
            .and_then(|pending| pending.deferred_reject.as_ref())
            .is_some_and(|reject| reject.owner_seq > welcome.seq)
        {
            return Err(GuestError::InvalidWelcome {
                reason: "same-epoch Welcome is behind a retained pending verdict",
            });
        }
        self.participants = participants;
        self.advertised_seq = welcome.seq;
        self.state = if self.confirmed_document.is_some() {
            GuestConnectionState::Active
        } else {
            GuestConnectionState::AwaitingSnapshot
        };
        // A CatchUp request sent on the dead transport is no longer in flight.
        // Recreate it below from the authenticated Welcome when it is needed.
        self.catch_up_requested_after = None;
        self.replay_pending_after_resume = self.pending.is_some();
        self.pending_replay_sent = false;
        let mut effects = Vec::new();
        if let Some(confirmed) = self.confirmed_seq {
            if welcome.seq > confirmed {
                effects.push(self.catch_up_effect(confirmed));
            } else {
                effects.extend(self.continue_after_install()?);
            }
        }
        if let Some(request) = &self.pending_undo {
            effects.push(GuestEffect::Send(CollabMessage::UndoRequest(
                request.clone(),
            )));
        }
        Ok(effects)
    }

    pub fn disconnect(&mut self) {
        if self.state != GuestConnectionState::Ended {
            self.state = GuestConnectionState::Disconnected;
        }
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub fn participant_id(&self) -> &ParticipantId {
        &self.participant_id
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub const fn role(&self) -> Role {
        self.role
    }

    pub fn peer_namespace(&self) -> &PeerNamespace {
        &self.namespace
    }

    pub const fn state(&self) -> GuestConnectionState {
        self.state
    }

    pub fn wire_limits(&self) -> WireLimits {
        self.wire_limits
    }

    pub fn participants(&self) -> Vec<Participant> {
        self.participants.values().cloned().collect()
    }

    pub fn confirmed_document(&self) -> Option<&PenDocument> {
        self.confirmed_document.as_deref()
    }

    pub const fn confirmed_seq(&self) -> Option<CommitSeq> {
        self.confirmed_seq
    }

    pub const fn confirmed_hash(&self) -> Option<CanonicalHash> {
        self.confirmed_hash
    }

    pub fn displayed_document(&self) -> Option<&PenDocument> {
        self.displayed_document.as_deref()
    }

    pub fn pending_edit(&self) -> Option<&PendingEdit> {
        self.pending.as_ref()
    }

    pub const fn next_client_counter(&self) -> Option<u64> {
        self.next_client_counter
    }

    pub const fn next_id_counter(&self) -> Option<u64> {
        self.next_id_counter
    }

    pub const fn next_undo_counter(&self) -> Option<u64> {
        self.next_undo_counter
    }

    pub fn undo_targets(&self) -> Vec<ClientOpId> {
        self.undo_index
            .iter()
            .map(|entry| entry.client_op_id.clone())
            .collect()
    }

    pub fn latest_undo_target(&self) -> Option<&ClientOpId> {
        self.undo_index.back().map(|entry| &entry.client_op_id)
    }

    pub fn pending_undo_request(&self) -> Option<&UndoRequest> {
        self.pending_undo.as_ref()
    }

    pub const fn install_pending(&self) -> bool {
        self.prepared.is_some()
    }

    pub fn document_mutation_allowed(&self) -> bool {
        self.state == GuestConnectionState::Active
            && self.role != Role::Viewer
            && self.pending.is_none()
            && self.pending_undo.is_none()
            && self.expected_compensation.is_none()
            && self.prepared.is_none()
            && self.next_client_counter.is_some()
    }

    pub fn finalize_install(
        &mut self,
        prepared: PreparedGuestInstall,
        installed_hash: CanonicalHash,
    ) -> Result<Vec<GuestEffect>, GuestError> {
        let Some(expected) = self.prepared.as_ref() else {
            return Err(GuestError::PreparedInstallMismatch);
        };
        if expected != &prepared.key {
            return Err(GuestError::PreparedInstallMismatch);
        }
        if installed_hash != prepared.key.candidate_hash {
            self.state = GuestConnectionState::Ended;
            return Err(GuestError::InstalledHashMismatch {
                expected: prepared.key.candidate_hash,
                actual: installed_hash,
            });
        }

        let connection_state = self.state;
        self.prepared = None;
        self.confirmed_document = Some(prepared.post.confirmed_document);
        self.confirmed_seq = Some(prepared.post.confirmed_seq);
        self.confirmed_hash = Some(prepared.post.confirmed_hash);
        self.displayed_document = Some(prepared.post.displayed_document);
        self.pending = prepared.post.pending;
        if self.pending.is_none() {
            self.replay_pending_after_resume = false;
            self.pending_replay_sent = false;
        }
        if let Some(entry) = prepared.post.undo_index_add {
            self.undo_index.push_back(entry);
            while self.undo_index.len() > self.config.undo_index_entries {
                self.undo_index.pop_front();
            }
        }
        self.state = match connection_state {
            GuestConnectionState::Disconnected | GuestConnectionState::Ended => connection_state,
            GuestConnectionState::AwaitingSnapshot | GuestConnectionState::Active => {
                prepared.post.state
            }
        };
        if self
            .expected_compensation
            .as_ref()
            .is_some_and(|expected| expected.seq <= prepared.post.confirmed_seq)
        {
            self.expected_compensation = None;
        }
        if let Some(through) = prepared.post.discard_buffer_through {
            let stale: Vec<_> = self
                .buffered_commits
                .range(..=through.0)
                .map(|(seq, _)| *seq)
                .collect();
            for seq in stale {
                if let Some(buffered) = self.buffered_commits.remove(&seq) {
                    self.buffered_commit_bytes = self
                        .buffered_commit_bytes
                        .saturating_sub(buffered.encoded_size);
                }
            }
        }
        self.catch_up_requested_after = None;

        let mut effects = Vec::new();
        if self.state == GuestConnectionState::Active {
            effects.push(GuestEffect::Send(CollabMessage::Applied(crate::Applied {
                through_seq: prepared.post.confirmed_seq,
            })));
        }
        if let Some((client_op_id, reason)) = prepared.post.pending_cancel {
            effects.push(GuestEffect::PendingCancelled {
                client_op_id,
                reason,
            });
        }
        effects.extend(self.continue_after_install()?);
        Ok(effects)
    }

    pub fn abort_install(&mut self, prepared: PreparedGuestInstall) -> Result<(), GuestError> {
        let Some(expected) = self.prepared.as_ref() else {
            return Err(GuestError::PreparedInstallMismatch);
        };
        if expected != &prepared.key {
            return Err(GuestError::PreparedInstallMismatch);
        }
        self.prepared = None;
        Ok(())
    }

    pub(crate) fn stage_install(
        &mut self,
        reason: crate::GuestInstallReason,
        candidate: PenDocument,
        post: GuestPostInstall,
    ) -> Result<GuestEffect, GuestError> {
        if self.prepared.is_some() {
            return Err(GuestError::InstallPending);
        }
        let candidate_hash = crate::canonical_document_hash(&candidate)?;
        self.install_generation = self.install_generation.wrapping_add(1);
        let key = PreparedGuestKey {
            generation: self.install_generation,
            seq: post.confirmed_seq,
            candidate_hash,
        };
        self.prepared = Some(key.clone());
        Ok(GuestEffect::PrepareInstall(Box::new(
            PreparedGuestInstall {
                key,
                reason,
                candidate_document: Some(candidate),
                post,
            },
        )))
    }

    pub(crate) fn catch_up_effect(&mut self, after_seq: CommitSeq) -> GuestEffect {
        self.catch_up_requested_after = Some(after_seq);
        GuestEffect::Send(CollabMessage::CatchUp(crate::CatchUp { after_seq }))
    }

    pub(crate) fn replay_pending_submit_after_resume(&mut self) -> Option<GuestEffect> {
        if !self.replay_pending_after_resume
            || self.pending_replay_sent
            || self
                .confirmed_seq
                .is_none_or(|confirmed| confirmed < self.advertised_seq)
        {
            return None;
        }
        let Some(pending) = self.pending.as_ref() else {
            self.replay_pending_after_resume = false;
            self.pending_replay_sent = false;
            return None;
        };
        if pending.status != PendingEditStatus::Submitted {
            return None;
        }
        let submit = Submit {
            client_op_id: pending.client_op_id.clone(),
            base_seq: pending.base_seq,
            txn: pending.submitted_txn.clone(),
        };
        self.pending_replay_sent = true;
        Some(GuestEffect::Send(CollabMessage::Submit(submit)))
    }
}

fn validate_guest_config(config: GuestSessionConfig) -> Result<(), GuestError> {
    validate_limit(
        "commit_buffer_entries",
        config.commit_buffer_entries,
        MAX_GUEST_COMMIT_BUFFER_ENTRIES,
    )?;
    validate_limit(
        "commit_buffer_bytes",
        config.commit_buffer_bytes,
        MAX_GUEST_COMMIT_BUFFER_BYTES,
    )?;
    validate_limit(
        "undo_index_entries",
        config.undo_index_entries,
        MAX_GUEST_UNDO_INDEX_ENTRIES,
    )
}

fn validate_limit(field: &'static str, actual: usize, maximum: usize) -> Result<(), GuestError> {
    if actual == 0 || actual > maximum {
        Err(GuestError::InvalidConfig {
            field,
            actual,
            minimum: 1,
            maximum,
        })
    } else {
        Ok(())
    }
}

fn validate_welcome(
    session_id: &SessionId,
    epoch: Epoch,
    welcome: &Welcome,
) -> Result<(PeerNamespace, BTreeMap<String, Participant>), GuestError> {
    FrameEnvelope::new(
        session_id.clone(),
        epoch,
        CollabMessage::Welcome(welcome.clone()),
    )
    .to_json_vec_with_limits(WireLimits::default())
    .map_err(GuestError::InvalidWelcomeFrame)?;
    if welcome.hash_version != CANONICAL_HASH_VERSION {
        return Err(GuestError::InvalidWelcome {
            reason: "unsupported canonical hash version",
        });
    }
    if welcome.document_schema_version.is_empty() {
        return Err(GuestError::InvalidWelcome {
            reason: "empty document schema version",
        });
    }
    if welcome.role == Role::Owner {
        return Err(GuestError::InvalidWelcome {
            reason: "guest self entry cannot claim owner role",
        });
    }
    if welcome.participants.is_empty() || welcome.participants.len() > MAX_SESSION_PARTICIPANTS {
        return Err(GuestError::InvalidWelcome {
            reason: "roster size is outside the supported session bound",
        });
    }

    let namespace = PeerNamespace::try_from(welcome.peer_namespace.as_str())?;
    let mut participants = BTreeMap::new();
    let mut participant_ids = BTreeSet::new();
    let mut self_count = 0_usize;
    let mut owner_count = 0_usize;
    for participant in &welcome.participants {
        if !participant_ids.insert(participant.participant_id.as_ref().to_owned())
            || participants
                .insert(participant.peer_id.as_ref().to_owned(), participant.clone())
                .is_some()
        {
            return Err(GuestError::InvalidWelcome {
                reason: "roster identities are not unique",
            });
        }
        if participant.role == Role::Owner {
            owner_count += 1;
        }
        if participant.peer_id == welcome.peer_id {
            self_count += 1;
            if participant.participant_id != welcome.participant_id
                || participant.role != welcome.role
            {
                return Err(GuestError::InvalidWelcome {
                    reason: "roster self entry differs from Welcome binding",
                });
            }
        }
    }
    if self_count != 1 {
        return Err(GuestError::InvalidWelcome {
            reason: "roster must contain exactly one self entry",
        });
    }
    if owner_count != 1 {
        return Err(GuestError::InvalidWelcome {
            reason: "roster must contain exactly one owner",
        });
    }
    Ok((namespace, participants))
}
