use std::collections::BTreeMap;

use jian_ops_schema::PenDocument;

use crate::{
    id_high_water::document_contains_namespace,
    session::OwnerSessionCore,
    session_state::{
        same_auth_binding, same_binding, validate_auth_profile, validate_principal, PeerState,
    },
    AdmissionGrant, CollabMessage, CommitSeq, ConnectionKey, ConnectionPrincipal, FrameEnvelope,
    OwnerEffect, Participant, ParticipantLeft, PeerActivation, PeerId, PeerProgress, Role,
    SessionError, Snapshot, VerifiedAuthMetadata, Welcome, CANONICAL_HASH_VERSION,
    FIRST_CLIENT_OP_COUNTER,
};

impl OwnerSessionCore {
    pub fn active_participants(&self) -> Vec<Participant> {
        self.peers
            .values()
            .filter(|peer| peer.connection.is_some())
            .map(PeerState::participant)
            .collect()
    }

    pub fn peer_progress(&self, peer_id: &PeerId) -> Option<PeerProgress> {
        self.peers.get(peer_id.as_ref()).map(PeerState::progress)
    }

    pub fn activate_peer(
        &mut self,
        connection: ConnectionKey,
        grant: AdmissionGrant,
        document: &PenDocument,
    ) -> Result<PeerActivation, SessionError> {
        self.ensure_running()?;
        self.ensure_connection_unused(connection)?;
        let (principal, namespace) = grant.into_parts();
        validate_principal(&principal, self.config)?;
        if principal.role() == Role::Owner {
            return Err(SessionError::MultipleOwners);
        }
        if self.peers.contains_key(principal.peer_id().as_ref()) {
            return Err(SessionError::PeerAlreadyExists);
        }
        if self.peers.len() >= self.config.session_limits.max_participants {
            return Err(SessionError::ParticipantLimit);
        }
        if self
            .peers
            .values()
            .any(|peer| peer.principal.participant_id() == principal.participant_id())
        {
            return Err(SessionError::DuplicateParticipant);
        }
        if self.peers.values().any(|peer| peer.namespace == namespace) {
            return Err(SessionError::DuplicateNamespace);
        }
        let limits = crate::ApplyLimits::from(self.config.wire_limits);
        if document_contains_namespace(document, &namespace, &limits)
            .map_err(SessionError::InvalidInitialDocument)?
        {
            return Err(SessionError::NamespaceAlreadyPresentInDocument);
        }
        let snapshot = self.snapshot_for(document)?;
        let (_, snapshot_message) =
            self.checked_outbound(CollabMessage::Snapshot(Box::new(snapshot)))?;
        let CollabMessage::Snapshot(snapshot) = snapshot_message else {
            unreachable!("checked Snapshot must retain its message kind");
        };
        let peer_id = principal.peer_id().clone();
        let joined = principal.roster_participant();
        self.validate_outbound(CollabMessage::ParticipantJoined(joined.clone()))?;
        self.peers.insert(
            peer_id.as_ref().to_owned(),
            PeerState {
                principal,
                namespace,
                connection: Some(connection),
                next_counter: Some(FIRST_CLIENT_OP_COUNTER),
                next_id_counter: Some(0),
                retired_floor: FIRST_CLIENT_OP_COUNTER,
                results: BTreeMap::new(),
                result_bytes: 0,
                next_undo_counter: Some(FIRST_CLIENT_OP_COUNTER),
                undo_retired_floor: FIRST_CLIENT_OP_COUNTER,
                undo_results: BTreeMap::new(),
                undo_result_bytes: 0,
                applied_through: CommitSeq(0),
            },
        );
        let welcome = self.welcome_for(&peer_id)?;
        if let Err(error) = self.validate_outbound(CollabMessage::Welcome(welcome.clone())) {
            self.peers.remove(peer_id.as_ref());
            return Err(error);
        }
        Ok(PeerActivation {
            connection,
            welcome,
            snapshot: Some(*snapshot),
            joined,
        })
    }

    pub fn resume_peer(
        &mut self,
        connection: ConnectionKey,
        grant: AdmissionGrant,
    ) -> Result<PeerActivation, SessionError> {
        self.ensure_running()?;
        self.ensure_connection_unused(connection)?;
        let (principal, namespace) = grant.into_parts();
        validate_principal(&principal, self.config)?;
        let peer_id = principal.peer_id().clone();
        let previous_principal = {
            let peer = self
                .peers
                .get_mut(peer_id.as_ref())
                .ok_or(SessionError::UnknownPeer)?;
            if peer.connection.is_some() {
                return Err(SessionError::PeerAlreadyActive);
            }
            if peer.namespace != namespace || !same_binding(&peer.principal, &principal) {
                return Err(SessionError::ResumeBindingMismatch);
            }
            if principal.auth().expires_at_unix_ms < peer.principal.auth().expires_at_unix_ms {
                return Err(SessionError::ResumeBindingMismatch);
            }
            let previous_principal = peer.principal.clone();
            peer.principal = principal;
            peer.connection = Some(connection);
            previous_principal
        };
        let joined = self
            .peers
            .get(peer_id.as_ref())
            .expect("resumed peer remains registered")
            .participant();
        let welcome = self.welcome_for(&peer_id)?;
        let outbound_valid = self
            .validate_outbound(CollabMessage::Welcome(welcome.clone()))
            .and_then(|_| self.validate_outbound(CollabMessage::ParticipantJoined(joined.clone())));
        if let Err(error) = outbound_valid {
            let peer = self
                .peers
                .get_mut(peer_id.as_ref())
                .expect("resumed peer remains registered");
            peer.connection = None;
            peer.principal = previous_principal;
            return Err(error);
        }
        Ok(PeerActivation {
            connection,
            welcome,
            snapshot: None,
            joined,
        })
    }

    pub fn complete_renewal(
        &mut self,
        connection: ConnectionKey,
        auth: VerifiedAuthMetadata,
    ) -> Result<(), SessionError> {
        let result = self.complete_renewal_inner(connection, auth);
        if matches!(&result, Err(SessionError::RenewalBindingMismatch)) {
            self.mark_connection_closing(connection);
        }
        result
    }

    fn complete_renewal_inner(
        &mut self,
        connection: ConnectionKey,
        mut auth: VerifiedAuthMetadata,
    ) -> Result<(), SessionError> {
        validate_auth_profile(&auth)?;
        let peer_id = self.peer_id_for_connection(connection)?.clone();
        let peer = self
            .peers
            .get_mut(peer_id.as_ref())
            .expect("active connection has a retained peer");
        if !same_auth_binding(peer.principal.auth(), &auth) {
            return Err(SessionError::RenewalBindingMismatch);
        }
        if auth.expires_at_unix_ms <= peer.principal.auth().expires_at_unix_ms {
            return Err(SessionError::RenewalDidNotExtend);
        }
        // M1 has no ProfileChanged wire message. Freeze the active roster
        // projection until an explicit resume can broadcast ParticipantJoined.
        auth.display_name = peer.principal.auth().display_name.clone();
        auth.avatar_url = peer.principal.auth().avatar_url.clone();
        peer.principal = ConnectionPrincipal::from_verified(
            auth,
            peer.principal.participant_id().clone(),
            peer.principal.peer_id().clone(),
            peer.principal.role(),
        );
        Ok(())
    }

    pub fn disconnect(
        &mut self,
        connection: ConnectionKey,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        let peer_id = self.peer_id_for_bound_connection(connection)?.clone();
        let is_owner = peer_id == self.owner_peer_id;
        let message = if is_owner {
            CollabMessage::Bye(crate::Bye {
                reason: crate::ByeReason::OwnerLeft,
            })
        } else {
            let peer = self
                .peers
                .get(peer_id.as_ref())
                .expect("bound connection has a retained peer");
            CollabMessage::ParticipantLeft(ParticipantLeft {
                participant_id: peer.principal.participant_id().clone(),
                peer_id: peer.principal.peer_id().clone(),
            })
        };
        let (_, message) = self.checked_outbound(message)?;
        self.closing_connections.remove(&connection);
        self.peers
            .get_mut(peer_id.as_ref())
            .expect("bound connection has a retained peer")
            .connection = None;
        if is_owner {
            self.ended = true;
            for peer in self.peers.values_mut() {
                peer.connection = None;
            }
            self.closing_connections.clear();
        }
        Ok(vec![OwnerEffect::Broadcast { message }])
    }

    pub(crate) fn welcome_for(&self, peer_id: &PeerId) -> Result<Welcome, SessionError> {
        let peer = self
            .peers
            .get(peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        let participants = self.active_participants();
        debug_assert_eq!(
            participants
                .iter()
                .filter(|participant| participant.peer_id == *peer_id)
                .count(),
            1
        );
        Ok(Welcome {
            participant_id: peer.principal.participant_id().clone(),
            peer_id: peer.principal.peer_id().clone(),
            role: peer.principal.role(),
            seq: self.seq,
            peer_namespace: peer.namespace.to_string(),
            document_schema_version: self.document_schema_version.clone(),
            hash_version: CANONICAL_HASH_VERSION,
            limits: self.config.wire_limits,
            participants,
        })
    }

    pub(crate) fn peer_id_for_connection(
        &self,
        connection: ConnectionKey,
    ) -> Result<&PeerId, SessionError> {
        if self.closing_connections.contains(&connection) {
            return Err(SessionError::ConnectionClosing);
        }
        self.peer_id_for_bound_connection(connection)
    }

    pub(crate) fn peer_id_for_bound_connection(
        &self,
        connection: ConnectionKey,
    ) -> Result<&PeerId, SessionError> {
        self.peers
            .values()
            .find(|peer| peer.connection == Some(connection))
            .map(|peer| peer.principal.peer_id())
            .ok_or(SessionError::UnknownConnection)
    }

    pub(crate) fn mark_connection_closing(&mut self, connection: ConnectionKey) {
        if self
            .peers
            .values()
            .any(|peer| peer.connection == Some(connection))
        {
            self.closing_connections.insert(connection);
        }
    }

    pub(crate) fn ensure_connection_unused(
        &self,
        connection: ConnectionKey,
    ) -> Result<(), SessionError> {
        if self
            .peers
            .values()
            .any(|peer| peer.connection == Some(connection))
        {
            return Err(SessionError::ConnectionAlreadyBound);
        }
        Ok(())
    }

    pub(crate) fn ensure_running(&self) -> Result<(), SessionError> {
        if self.ended {
            Err(SessionError::SessionEnded)
        } else {
            Ok(())
        }
    }

    pub(crate) fn snapshot_for(&self, document: &PenDocument) -> Result<Snapshot, SessionError> {
        let actual = crate::canonical_document_hash(document)?;
        if actual != self.document_hash {
            return Err(SessionError::DocumentDrift);
        }
        Ok(Snapshot {
            seq: self.seq,
            document: document.clone(),
            doc_hash: actual,
        })
    }

    pub(crate) fn validate_outbound(&self, message: CollabMessage) -> Result<usize, SessionError> {
        self.checked_outbound(message).map(|(size, _)| size)
    }

    pub(crate) fn checked_outbound(
        &self,
        message: CollabMessage,
    ) -> Result<(usize, CollabMessage), SessionError> {
        let frame = FrameEnvelope::new(self.session_id.clone(), self.epoch, message);
        let size = frame
            .to_json_vec_with_limits(self.config.wire_limits)
            .map_err(SessionError::invalid_frame)?
            .len();
        Ok((size, frame.into_body()))
    }
}
