use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
    sync::Arc,
};

use jian_ops_schema::PenDocument;

use crate::{
    apply::validate_document_with_limits,
    canonical_document_hash,
    id_high_water::document_contains_namespace,
    session_state::{
        peer_to_owner_message, validate_config, validate_principal, CachedSubmitResult,
        CommitRecord, PeerState,
    },
    session_types::{PreparedKey, PreparedSourceKey},
    AdmissionGrant, ApplyLimits, CanonicalHash, CollabMessage, CommitSeq, ConnectionKey, Epoch,
    FrameEnvelope, OwnerEffect, OwnerSessionConfig, PeerId, PreparedCommit, Role, SessionError,
    SessionId, FIRST_CLIENT_OP_COUNTER,
};

/// Pure owner-side collaboration sequencing and roster state.
///
/// Socket ownership, ticket parsing, and editor installation remain host
/// responsibilities. A successful Submit first emits [`PreparedCommit`];
/// global sequence, commit log, and dedupe state advance only after the host
/// confirms installation.
pub struct OwnerSessionCore {
    pub(crate) session_id: SessionId,
    pub(crate) epoch: Epoch,
    pub(crate) seq: CommitSeq,
    pub(crate) document_schema_version: String,
    pub(crate) document_hash: CanonicalHash,
    pub(crate) owner_peer_id: PeerId,
    pub(crate) config: OwnerSessionConfig,
    pub(crate) peers: BTreeMap<String, PeerState>,
    pub(crate) closing_connections: BTreeSet<ConnectionKey>,
    pub(crate) commit_log: VecDeque<Arc<CommitRecord>>,
    pub(crate) commit_log_bytes: usize,
    pub(crate) undo_ledger: crate::undo_ledger::UndoLedger,
    pub(crate) prepared: Option<PreparedKey>,
    pub(crate) ended: bool,
}

impl fmt::Debug for OwnerSessionCore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerSessionCore")
            .field("session_id", &self.session_id)
            .field("epoch", &self.epoch)
            .field("seq", &self.seq)
            .field("peer_count", &self.peers.len())
            .field("active_peer_count", &self.active_participants().len())
            .field("closing_connection_count", &self.closing_connections.len())
            .field("commit_log_len", &self.commit_log.len())
            .field("install_pending", &self.prepared.is_some())
            .field("ended", &self.ended)
            .finish()
    }
}

impl OwnerSessionCore {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: SessionId,
        epoch: Epoch,
        seq: CommitSeq,
        owner_connection: ConnectionKey,
        owner_grant: AdmissionGrant,
        document: &PenDocument,
        config: OwnerSessionConfig,
    ) -> Result<Self, SessionError> {
        validate_config(config)?;
        let (principal, namespace) = owner_grant.into_parts();
        validate_principal(&principal, config)?;
        if principal.role() != Role::Owner {
            return Err(SessionError::InvalidOwnerGrant);
        }
        let limits = ApplyLimits::from(config.wire_limits);
        validate_document_with_limits(document, &limits)
            .map_err(SessionError::InvalidInitialDocument)?;
        if document_contains_namespace(document, &namespace, &limits)
            .map_err(SessionError::InvalidInitialDocument)?
        {
            return Err(SessionError::NamespaceAlreadyPresentInDocument);
        }
        let document_hash = canonical_document_hash(document)?;
        FrameEnvelope::new(
            session_id.clone(),
            epoch,
            CollabMessage::Snapshot(Box::new(crate::Snapshot {
                seq,
                document: document.clone(),
                doc_hash: document_hash,
            })),
        )
        .to_json_vec_with_limits(config.wire_limits)
        .map_err(SessionError::invalid_frame)?;
        let owner_peer_id = principal.peer_id().clone();
        let mut peers = BTreeMap::new();
        peers.insert(
            owner_peer_id.as_ref().to_owned(),
            PeerState {
                principal,
                namespace,
                connection: Some(owner_connection),
                next_counter: Some(FIRST_CLIENT_OP_COUNTER),
                next_id_counter: Some(0),
                retired_floor: FIRST_CLIENT_OP_COUNTER,
                results: BTreeMap::new(),
                result_bytes: 0,
                next_undo_counter: Some(FIRST_CLIENT_OP_COUNTER),
                undo_retired_floor: FIRST_CLIENT_OP_COUNTER,
                undo_results: BTreeMap::new(),
                undo_result_bytes: 0,
                applied_through: seq,
            },
        );
        Ok(Self {
            session_id,
            epoch,
            seq,
            document_schema_version: document.version.clone(),
            document_hash,
            owner_peer_id,
            config,
            peers,
            closing_connections: BTreeSet::new(),
            commit_log: VecDeque::new(),
            commit_log_bytes: 0,
            undo_ledger: crate::undo_ledger::UndoLedger::default(),
            prepared: None,
            ended: false,
        })
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub const fn seq(&self) -> CommitSeq {
        self.seq
    }

    pub const fn document_hash(&self) -> CanonicalHash {
        self.document_hash
    }

    /// Whether a staged document is awaiting the host's synchronous install.
    ///
    /// A host must queue later document frames until this returns false.
    pub const fn install_pending(&self) -> bool {
        self.prepared.is_some()
    }

    /// Accepts one frame from an active connection.
    ///
    /// Any returned error is fatal for that input connection. The core marks
    /// the binding as closing immediately; the host must close transport and
    /// call [`Self::disconnect`] before a same-epoch resume can bind again.
    pub fn accept_frame(
        &mut self,
        connection: ConnectionKey,
        frame: FrameEnvelope,
        document: &PenDocument,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        let result = self.accept_frame_inner(connection, frame, document);
        if result.is_err() {
            self.mark_connection_closing(connection);
        }
        result
    }

    fn accept_frame_inner(
        &mut self,
        connection: ConnectionKey,
        frame: FrameEnvelope,
        document: &PenDocument,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        self.ensure_running()?;
        if frame.session_id() != &self.session_id {
            return Err(SessionError::WrongSession);
        }
        if frame.epoch() != self.epoch {
            return Err(SessionError::WrongEpoch);
        }
        if !peer_to_owner_message(frame.body()) {
            return Err(SessionError::WrongDirection {
                kind: frame.body().kind(),
            });
        }
        let peer_id = self.peer_id_for_connection(connection)?.clone();
        if !matches!(frame.body(), CollabMessage::Submit(_)) {
            frame
                .encoded_len_with_limits(self.config.wire_limits)
                .map_err(SessionError::invalid_frame)?;
        }
        match frame.into_body() {
            CollabMessage::Submit(submit) => {
                self.handle_submit(connection, &peer_id, submit, document)
            }
            CollabMessage::CatchUp(catch_up) => {
                self.handle_catch_up(connection, catch_up, document)
            }
            CollabMessage::Applied(applied) => {
                self.handle_applied(&peer_id, applied)?;
                Ok(Vec::new())
            }
            CollabMessage::PresenceUpdate(presence) => self.handle_presence(&peer_id, presence),
            CollabMessage::RenewTicket(renewal) => Ok(vec![OwnerEffect::VerifyRenewal {
                connection,
                ticket: renewal.opaque_ticket,
            }]),
            CollabMessage::UndoRequest(request) => {
                if request.request_id.peer_id != peer_id {
                    return Err(SessionError::ClientOpPeerMismatch);
                }
                self.handle_undo_request(connection, &peer_id, request, document)
            }
            CollabMessage::Bye(_) => self.disconnect(connection),
            CollabMessage::Welcome(_)
            | CollabMessage::Commit(_)
            | CollabMessage::Reject(_)
            | CollabMessage::Snapshot(_)
            | CollabMessage::PresenceChanged(_)
            | CollabMessage::ParticipantJoined(_)
            | CollabMessage::ParticipantLeft(_)
            | CollabMessage::UndoResult(_) => {
                unreachable!("direction was checked before dispatch")
            }
        }
    }

    pub fn finalize_install(
        &mut self,
        mut prepared: PreparedCommit,
        installed_hash: CanonicalHash,
    ) -> Result<OwnerEffect, SessionError> {
        self.ensure_running()?;
        let Some(expected) = self.prepared.as_ref() else {
            return Err(SessionError::PreparedCommitMismatch);
        };
        if expected != &prepared.key {
            return Err(SessionError::PreparedCommitMismatch);
        }
        if installed_hash != prepared.commit.doc_hash {
            self.ended = true;
            return Err(SessionError::InstalledHashMismatch {
                expected: prepared.commit.doc_hash,
                actual: installed_hash,
            });
        }
        let peer_id = prepared.key.peer_id.clone();
        let counter = prepared.key.counter;
        let txn_hash = prepared.key.txn_hash;
        let source = prepared.key.source.clone();
        let record = Arc::new(CommitRecord {
            commit: Arc::new(prepared.commit),
            weight: prepared.encoded_commit_size,
        });
        let next_counter_matches = self
            .peers
            .get(peer_id.as_ref())
            .is_some_and(|peer| peer.next_counter == Some(counter));
        if !next_counter_matches {
            self.ended = true;
            return Err(SessionError::PreparedCommitMismatch);
        }
        match (&source, prepared.undo.as_ref()) {
            (PreparedSourceKey::Submit, None) => {}
            (
                PreparedSourceKey::Undo {
                    request_id,
                    target_client_op_id,
                },
                Some(undo),
            ) if undo.result.request_id == *request_id
                && undo.result.target_client_op_id == *target_client_op_id
                && self.peers.get(peer_id.as_ref()).is_some_and(|peer| {
                    peer.next_undo_counter == Some(request_id.local_counter)
                }) => {}
            _ => {
                self.ended = true;
                return Err(SessionError::PreparedCommitMismatch);
            }
        }
        if let Err(error) = self.cache_result(
            &peer_id,
            counter,
            txn_hash,
            CachedSubmitResult::Commit(record.clone()),
        ) {
            self.ended = true;
            return Err(error);
        }
        self.peers
            .get_mut(peer_id.as_ref())
            .expect("prepared peer remains retained")
            .next_id_counter = prepared.next_id_counter;
        let undoable = matches!(source, PreparedSourceKey::Submit);
        let limits = self.config.session_limits;
        self.undo_ledger.record_commit(
            &record.commit.client_op_id,
            record.commit.seq,
            &prepared.changes,
            undoable,
            limits.undo_ledger_entries,
            limits.undo_ledger_bytes,
        );
        let undo_effect = if let Some(undo) = prepared.undo.take() {
            debug_assert!(!undo.reverted_fields.is_empty());
            if let Err(error) = self.cache_undo_result(
                &peer_id,
                undo.result.request_id.local_counter,
                undo.result.target_client_op_id.clone(),
                undo.result.clone(),
            ) {
                self.ended = true;
                return Err(error);
            }
            self.undo_ledger
                .mark_undone(&undo.result.target_client_op_id);
            Some((undo.reply_to, undo.result))
        } else {
            None
        };
        self.push_commit(record.clone());
        self.seq = record.commit.seq;
        self.document_hash = record.commit.doc_hash;
        if let Some(owner) = self.peers.get_mut(self.owner_peer_id.as_ref()) {
            owner.applied_through = self.seq;
        }
        self.prepared = None;
        if let Some((reply_to, result)) = undo_effect {
            Ok(OwnerEffect::UndoCommitted {
                reply_to,
                result,
                commit: record.commit.clone(),
            })
        } else {
            Ok(OwnerEffect::BroadcastCommit {
                commit: record.commit.clone(),
            })
        }
    }

    pub fn abort_prepare(&mut self, prepared: PreparedCommit) -> Result<(), SessionError> {
        let Some(expected) = self.prepared.as_ref() else {
            return Err(SessionError::PreparedCommitMismatch);
        };
        if expected != &prepared.key {
            return Err(SessionError::PreparedCommitMismatch);
        }
        self.prepared = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdmissionGrant, ClientOpId, CollabOp, CollabTxn, ConnectionPrincipal, PageRef,
        ParticipantId, PeerNamespace, Role, Submit, VerifiedAuthMetadata,
    };
    use jian_ops_schema::{node::PenNode, PenDocument};

    fn principal(peer: &str, role: Role) -> ConnectionPrincipal {
        ConnectionPrincipal::from_verified(
            VerifiedAuthMetadata {
                issuer: "test-issuer".into(),
                subject: format!("subject-{peer}"),
                device_id: format!("device-{peer}"),
                proof_binding: format!("binding-{peer}"),
                expires_at_unix_ms: 10_000,
                display_name: None,
                avatar_url: None,
            },
            ParticipantId::from(format!("participant-{peer}")),
            PeerId::from(peer),
            role,
        )
    }

    #[test]
    fn final_client_counter_commits_once_then_enters_exhausted_state() {
        let initial: PenDocument =
            serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap();
        let owner_connection = ConnectionKey::new(1).unwrap();
        let guest_connection = ConnectionKey::new(2).unwrap();
        let mut core = OwnerSessionCore::new(
            SessionId::from("session"),
            Epoch(1),
            CommitSeq(0),
            owner_connection,
            AdmissionGrant::new(
                principal("owner", Role::Owner),
                PeerNamespace::try_from("owner").unwrap(),
            ),
            &initial,
            OwnerSessionConfig::default(),
        )
        .unwrap();
        core.activate_peer(
            guest_connection,
            AdmissionGrant::new(
                principal("guest", Role::Editor),
                PeerNamespace::try_from("guest").unwrap(),
            ),
            &initial,
        )
        .unwrap();
        core.peers.get_mut("guest").unwrap().next_counter = Some(u64::MAX);

        let node: PenNode =
            serde_json::from_str(r#"{"type":"rectangle","id":"c_guest_1"}"#).unwrap();
        let submit = Submit {
            client_op_id: ClientOpId {
                peer_id: PeerId::from("guest"),
                local_counter: u64::MAX,
            },
            base_seq: CommitSeq(0),
            txn: CollabTxn::new(vec![CollabOp::InsertExact {
                page: PageRef::DocumentRoot,
                parent_id: None,
                index: 0,
                node,
            }]),
        };
        let frame = FrameEnvelope::new(
            SessionId::from("session"),
            Epoch(1),
            CollabMessage::Submit(submit.clone()),
        );
        let mut effects = core
            .accept_frame(guest_connection, frame, &initial)
            .unwrap();
        let OwnerEffect::PrepareInstall(prepared) = effects.remove(0) else {
            panic!("current counter must prepare one commit");
        };
        let mut prepared = *prepared;
        let installed = prepared.take_candidate_document().unwrap();
        let installed_hash = prepared.candidate_hash();
        core.finalize_install(prepared, installed_hash).unwrap();

        let progress = core.peer_progress(&PeerId::from("guest")).unwrap();
        assert_eq!(progress.next_counter, None);
        let replay_frame = FrameEnvelope::new(
            SessionId::from("session"),
            Epoch(1),
            CollabMessage::Submit(submit),
        );
        let replay = core
            .accept_frame(guest_connection, replay_frame, &installed)
            .unwrap();
        assert!(matches!(
            replay.as_slice(),
            [OwnerEffect::ReplyCommit { .. }]
        ));
        assert_eq!(core.seq(), CommitSeq(1));
    }
}
