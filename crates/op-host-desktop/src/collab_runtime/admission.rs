//! GUI-owned owner admission queue and decision handling.
//!
//! Verified identity and resume claims remain here, outside shared paint
//! state. The UI receives only a random request key and an optional retained
//! resume role. Every connection, including a same-epoch resume, requires a
//! fresh owner decision in M1.

use std::collections::HashMap;

use op_collab::{
    AdmissionGrant, CollabMessage, ConnectionKey, ConnectionPrincipal, Role, VerifiedAuthMetadata,
};
use op_collab_transport::JoinIntent;
use op_editor_core::{CollabAdmissionRequestKey, MAX_COLLAB_PENDING_ADMISSIONS};
use op_host_native::WidgetHostNative;

use super::actor::{random_identifier, set_owner_ui, ui_role, EditorActor};
use super::types::{CollabRuntimeError, CollabStatusEvent, OwnerNetworkCommand};
use super::DesktopCollabRuntime;

pub(super) struct PendingOwnerAdmission {
    connection: ConnectionKey,
    auth: VerifiedAuthMetadata,
    intent: JoinIntent,
}

#[derive(Default)]
pub(super) struct PendingOwnerAdmissions {
    by_request: HashMap<CollabAdmissionRequestKey, PendingOwnerAdmission>,
}

impl PendingOwnerAdmissions {
    fn insert(
        &mut self,
        request_key: CollabAdmissionRequestKey,
        admission: PendingOwnerAdmission,
    ) -> bool {
        if self.by_request.len() >= MAX_COLLAB_PENDING_ADMISSIONS
            || self
                .by_request
                .values()
                .any(|pending| pending.connection == admission.connection)
        {
            return false;
        }
        self.by_request.insert(request_key, admission).is_none()
    }

    fn remove(&mut self, request_key: &CollabAdmissionRequestKey) -> Option<PendingOwnerAdmission> {
        self.by_request.remove(request_key)
    }

    pub(super) fn remove_connection(
        &mut self,
        connection: ConnectionKey,
    ) -> Option<(CollabAdmissionRequestKey, PendingOwnerAdmission)> {
        let request_key = self
            .by_request
            .iter()
            .find_map(|(key, pending)| (pending.connection == connection).then(|| key.clone()))?;
        self.by_request
            .remove(&request_key)
            .map(|pending| (request_key, pending))
    }

    pub(super) fn clear(&mut self) {
        self.by_request.clear();
    }
}

impl DesktopCollabRuntime {
    pub(super) fn peer_authenticated(
        &mut self,
        connection: ConnectionKey,
        auth: VerifiedAuthMetadata,
        intent: JoinIntent,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        if !matches!(self.actor, Some(EditorActor::Owner(_))) {
            let _ = self.send_owner(OwnerNetworkCommand::Close { connection });
            return Err(CollabRuntimeError::invalid_session());
        }
        let request_key_value = match random_identifier("admission") {
            Ok(value) => value,
            Err(error) => {
                let _ = self.send_owner(OwnerNetworkCommand::Close { connection });
                return Err(error);
            }
        };
        let request_key = match CollabAdmissionRequestKey::new(request_key_value) {
            Some(key) => key,
            None => {
                let _ = self.send_owner(OwnerNetworkCommand::Close { connection });
                return Err(CollabRuntimeError::invalid_session());
            }
        };
        let resume_role = match &intent {
            JoinIntent::New => None,
            JoinIntent::Resume(hint) => Some(ui_role(hint.role)),
        };
        if !self.pending_owner.insert(
            request_key.clone(),
            PendingOwnerAdmission {
                connection,
                auth,
                intent,
            },
        ) {
            let _ = self.send_owner(OwnerNetworkCommand::Close { connection });
            return Ok(());
        }
        if !host
            .editor_state_mut()
            .editor_ui
            .collab
            .publish_pending_admission(request_key.clone(), resume_role)
        {
            self.pending_owner.remove(&request_key);
            let _ = self.send_owner(OwnerNetworkCommand::Close { connection });
            return Ok(());
        }
        self.push_status(CollabStatusEvent::PeerAuthenticated { connection });
        Ok(())
    }

    pub(super) fn reject_owner_admission(
        &mut self,
        request_key: &CollabAdmissionRequestKey,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        host.editor_state_mut()
            .editor_ui
            .collab
            .remove_pending_admission(request_key);
        let Some(pending) = self.pending_owner.remove(request_key) else {
            return Ok(());
        };
        self.send_owner(OwnerNetworkCommand::Close {
            connection: pending.connection,
        })
    }

    pub(super) fn approve_owner_admission(
        &mut self,
        request_key: &CollabAdmissionRequestKey,
        selected_role: Role,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        host.editor_state_mut()
            .editor_ui
            .collab
            .remove_pending_admission(request_key);
        let Some(pending) = self.pending_owner.remove(request_key) else {
            return Ok(());
        };
        let connection = pending.connection;
        let Some(EditorActor::Owner(mut owner)) = self.actor.take() else {
            let _ = self.send_owner(OwnerNetworkCommand::Close { connection });
            return Err(CollabRuntimeError::invalid_session());
        };
        let result = self.activate_approved_owner_peer(&mut owner, pending, selected_role, host);
        if result.is_err() {
            let _ = self.send_owner(OwnerNetworkCommand::Close { connection });
        }
        set_owner_ui(host, &owner);
        self.actor = Some(EditorActor::Owner(owner));
        result
    }

    fn activate_approved_owner_peer(
        &mut self,
        owner: &mut super::actor::OwnerActor,
        pending: PendingOwnerAdmission,
        selected_role: Role,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        let connection = pending.connection;
        let (is_resume, grant) = match pending.intent {
            JoinIntent::New => (false, owner.grant_new_peer(pending.auth, selected_role)?),
            JoinIntent::Resume(hint) => {
                // A resume is re-approved in M1, but it cannot be used to
                // change role. SessionCore then verifies the same epoch-local
                // participant/peer/namespace and the same auth identity.
                if selected_role != hint.role || selected_role == Role::Owner {
                    return Err(CollabRuntimeError::invalid_session());
                }
                let principal = ConnectionPrincipal::from_verified(
                    pending.auth,
                    hint.participant_id,
                    hint.peer_id,
                    hint.role,
                );
                (true, AdmissionGrant::new(principal, hint.peer_namespace))
            }
        };
        let activation_result = if is_resume {
            owner.session.resume_peer(connection, grant)
        } else {
            owner.session.activate_peer(connection, grant, host)
        };
        let activation = activation_result.map_err(|_| CollabRuntimeError::invalid_session())?;
        debug_assert_eq!(selected_role, activation.welcome.role);

        // The per-peer command channel is ordered: Authorize is consumed by
        // the blocking admission worker before it enters the frame loop, so
        // Welcome/Snapshot can never be observed by an unapproved peer.
        let delivery = (|| {
            self.send_owner(OwnerNetworkCommand::Authorize {
                connection,
                role: activation.welcome.role,
            })?;
            owner.connections.insert(connection);
            self.send_owner_actor_message(
                owner,
                connection,
                CollabMessage::Welcome(activation.welcome),
                None,
            )?;
            // The guest cannot route a renewal until Welcome constructs (or
            // resumes) its core. FIFO still guarantees the current owner
            // ticket arrives before Snapshot and well before the old ticket's
            // expiry.
            if let Some(ticket) = self.latest_owner_ticket.as_ref() {
                self.send_owner_renew_ticket(owner, connection, ticket)?;
            }
            if let Some(snapshot) = activation.snapshot {
                self.send_owner_actor_message(
                    owner,
                    connection,
                    CollabMessage::Snapshot(Box::new(snapshot)),
                    None,
                )?;
            }
            self.broadcast_owner_message(
                owner,
                CollabMessage::ParticipantJoined(activation.joined),
                None,
            )
        })();
        if delivery.is_err() {
            owner.connections.remove(&connection);
            if let Ok(output) = owner.session.disconnect(connection) {
                let _ = self.route_owner_output(owner, output, host);
            }
        }
        delivery
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{Receiver, TryRecvError};

    use op_collab::{
        canonical_document_hash, Epoch, GuestConnectionState, GuestEffect, OpaqueTicket, SessionId,
    };

    use crate::collab_runtime::actor::{set_owner_ui, GuestActor, OwnerActor};
    use crate::collab_runtime::network::owner_command_channel_for_test;

    fn auth(index: usize) -> VerifiedAuthMetadata {
        VerifiedAuthMetadata {
            issuer: "https://issuer.example".into(),
            subject: format!("subject-{index}"),
            device_id: format!("device-{index}"),
            proof_binding: format!("binding-{index}"),
            expires_at_unix_ms: 10_000,
            display_name: None,
            avatar_url: None,
        }
    }

    fn pending(index: usize) -> PendingOwnerAdmission {
        PendingOwnerAdmission {
            connection: ConnectionKey::new(index as u64 + 2).unwrap(),
            auth: auth(index),
            intent: JoinIntent::New,
        }
    }

    fn owner_runtime() -> (
        DesktopCollabRuntime,
        WidgetHostNative,
        Receiver<OwnerNetworkCommand>,
    ) {
        let mut host = WidgetHostNative::new();
        let owner = OwnerActor::new(
            SessionId::from("owner-admission-test"),
            Epoch(1),
            auth(0),
            &mut host,
        )
        .unwrap();
        set_owner_ui(&mut host, &owner);
        let (network, commands) = owner_command_channel_for_test();
        let mut runtime = DesktopCollabRuntime::new();
        runtime.network = Some(network);
        runtime.actor = Some(EditorActor::Owner(Box::new(owner)));
        (runtime, host, commands)
    }

    fn first_request_key(host: &WidgetHostNative) -> CollabAdmissionRequestKey {
        host.editor_state().editor_ui.collab.pending_admissions()[0]
            .request_key()
            .clone()
    }

    #[test]
    fn pending_owner_queue_is_bounded_and_consumed_by_opaque_key() {
        let mut queue = PendingOwnerAdmissions::default();
        for index in 0..MAX_COLLAB_PENDING_ADMISSIONS {
            let key = CollabAdmissionRequestKey::new(format!("request-{index}")).unwrap();
            assert!(queue.insert(key, pending(index)));
        }
        let overflow = CollabAdmissionRequestKey::new("request-overflow").unwrap();
        assert!(!queue.insert(overflow, pending(MAX_COLLAB_PENDING_ADMISSIONS)));

        let first = CollabAdmissionRequestKey::new("request-0").unwrap();
        let removed = queue.remove(&first).expect("pending request");
        assert_eq!(removed.connection, ConnectionKey::new(2).unwrap());
        assert!(queue.remove(&first).is_none());
    }

    #[test]
    fn connection_close_removes_only_its_pending_request() {
        let mut queue = PendingOwnerAdmissions::default();
        let key = CollabAdmissionRequestKey::new("request-9").unwrap();
        let connection = ConnectionKey::new(9).unwrap();
        assert!(queue.insert(
            key.clone(),
            PendingOwnerAdmission {
                connection,
                auth: auth(1),
                intent: JoinIntent::New,
            }
        ));
        assert_eq!(
            queue.remove_connection(connection).map(|(key, _)| key),
            Some(key)
        );
        assert!(queue.remove_connection(connection).is_none());
    }

    #[test]
    fn pending_and_rejected_peer_receive_no_document_or_authorize_command() {
        let (mut runtime, mut host, commands) = owner_runtime();
        let before = canonical_document_hash(&host.editor_state().doc).unwrap();
        let connection = ConnectionKey::new(20).unwrap();
        runtime
            .peer_authenticated(connection, auth(1), JoinIntent::New, &mut host)
            .unwrap();

        assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
        assert_eq!(
            canonical_document_hash(&host.editor_state().doc).unwrap(),
            before
        );
        let Some(EditorActor::Owner(owner)) = runtime.actor.as_ref() else {
            panic!("owner actor");
        };
        assert_eq!(owner.session.core().active_participants().len(), 1);

        let request_key = first_request_key(&host);
        runtime
            .reject_owner_admission(&request_key, &mut host)
            .unwrap();
        let Ok(OwnerNetworkCommand::Close { connection: closed }) = commands.try_recv() else {
            panic!("reject must close the pending socket");
        };
        assert_eq!(closed, connection);
        assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
        assert!(host
            .editor_state()
            .editor_ui
            .collab
            .pending_admissions()
            .is_empty());
        assert_eq!(
            canonical_document_hash(&host.editor_state().doc).unwrap(),
            before
        );
    }

    #[test]
    fn approving_viewer_orders_welcome_before_latest_ticket_and_snapshot() {
        let (mut runtime, mut host, commands) = owner_runtime();
        let before = canonical_document_hash(&host.editor_state().doc).unwrap();
        let connection = ConnectionKey::new(21).unwrap();
        runtime.latest_owner_ticket =
            Some(OpaqueTicket::new("renewed-owner-ticket".to_string()).unwrap());
        runtime
            .peer_authenticated(connection, auth(2), JoinIntent::New, &mut host)
            .unwrap();
        assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));

        let request_key = first_request_key(&host);
        runtime
            .approve_owner_admission(&request_key, Role::Viewer, &mut host)
            .unwrap();

        let Ok(OwnerNetworkCommand::Authorize {
            connection: authorized,
            role,
        }) = commands.recv()
        else {
            panic!("first post-approval command must authorize");
        };
        assert_eq!(authorized, connection);
        assert_eq!(role, Role::Viewer);
        let Ok(OwnerNetworkCommand::Send {
            connection: welcomed,
            frame,
            ..
        }) = commands.recv()
        else {
            panic!("welcome follows authorization");
        };
        assert_eq!(welcomed, connection);
        let frame = frame.into_inner();
        let session_id = frame.session_id().clone();
        let epoch = frame.epoch();
        let CollabMessage::Welcome(welcome) = frame.into_body() else {
            panic!("welcome follows authorization");
        };
        assert_eq!(welcome.role, Role::Viewer);
        let mut guest_host = WidgetHostNative::new();
        let mut guest =
            GuestActor::new(session_id, epoch, welcome, connection, &mut guest_host).unwrap();

        let Ok(OwnerNetworkCommand::Send {
            connection: renewed,
            frame,
            ..
        }) = commands.recv()
        else {
            panic!("latest owner ticket follows Welcome");
        };
        assert_eq!(renewed, connection);
        let renewal = guest
            .session
            .accept_frame(frame.into_inner(), &mut guest_host)
            .expect("AwaitingSnapshot guest accepts renewal");
        assert!(matches!(
            renewal.effects.as_slice(),
            [GuestEffect::VerifyRenewal { .. }]
        ));
        assert_eq!(
            guest.session.core().state(),
            GuestConnectionState::AwaitingSnapshot
        );

        let Ok(OwnerNetworkCommand::Send {
            connection: snapshotted,
            frame,
            ..
        }) = commands.recv()
        else {
            panic!("snapshot follows welcome");
        };
        assert_eq!(snapshotted, connection);
        assert!(matches!(
            frame.decode_for_test().body(),
            CollabMessage::Snapshot(_)
        ));
        assert_eq!(
            canonical_document_hash(&host.editor_state().doc).unwrap(),
            before
        );
        let Some(EditorActor::Owner(owner)) = runtime.actor.as_ref() else {
            panic!("owner actor");
        };
        assert!(owner
            .session
            .core()
            .active_participants()
            .iter()
            .any(|participant| participant.role == Role::Viewer));
    }
}
