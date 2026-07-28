use std::sync::Arc;
use std::time::{Duration, Instant};

use op_collab::{
    Bye, ByeReason, CollabMessage, ConnectionKey, GuestEffect, OwnerEffect, ParticipantPresence,
    Point, Presence, UndoOutcome, UndoResult, Viewport,
};
use op_editor_core::{
    CollabConnectionPhase, CollabNoticeKind, CollabPendingEditUi, CollabRejectUiCode,
    DiscoveredCollabEndpoint, RemotePresenceUi,
};
use op_editor_host_core::collab::{
    GuestEditorOutput, GuestLocalEditResolution, LocalEditResolution, OwnerEditorOutput,
};
use op_host_native::WidgetHostNative;

use super::actor::{set_guest_ui, set_owner_ui, EditorActor, GuestActor, OwnerActor};
use super::network::NetworkCommandSendError;
use super::types::{
    CollabRuntimeError, CollabRuntimeFailure, CollabStatusEvent, DiscoveredEndpoint,
    GuestNetworkCommand, OwnerNetworkCommand, RemoteBye,
};
use super::DesktopCollabRuntime;

const GUEST_TO_OWNER_PRESENCE_COALESCE_KEY: u64 = 1;

impl DesktopCollabRuntime {
    pub(super) fn update_discovery(
        &mut self,
        sessions: Vec<DiscoveredEndpoint>,
        host: &mut WidgetHostNative,
    ) {
        self.discovered = sessions
            .iter()
            .cloned()
            .map(|session| (session.discovery_id.clone(), session))
            .collect();
        host.editor_state_mut().editor_ui.collab.panel.discovered = Arc::new(
            sessions
                .into_iter()
                .filter_map(|session| {
                    Some(DiscoveredCollabEndpoint {
                        discovery_id: session.discovery_id,
                        endpoint: session.addresses.first()?.to_string(),
                        compatible: session.compatible,
                    })
                })
                .collect(),
        );
    }

    pub(super) fn require_ready(&self, host: &WidgetHostNative) -> Result<(), CollabRuntimeError> {
        if op_auth_bridge::collab_ticket_available()
            && host.editor_state().editor_ui.account.is_signed_in()
        {
            Ok(())
        } else {
            Err(CollabRuntimeError::new(
                CollabRuntimeFailure::AuthenticationUnavailable,
            ))
        }
    }

    pub(super) fn send_local_renewal(
        &mut self,
        ticket: op_collab::OpaqueTicket,
    ) -> Result<(), CollabRuntimeError> {
        if matches!(self.actor, Some(EditorActor::Owner(_))) {
            // Pending, already-authenticated sockets have not entered the
            // owner's active connection set yet. Retain the sole credential
            // object; each peer receives a separately zeroizing wire encoding
            // borrowed from this protected owner storage.
            self.latest_owner_ticket = Some(ticket);
            let Some(EditorActor::Owner(owner)) = self.actor.as_ref() else {
                unreachable!("owner role was checked above");
            };
            let retained = self
                .latest_owner_ticket
                .as_ref()
                .expect("owner renewal was just retained");
            return self.broadcast_owner_renew_ticket(owner, retained);
        }
        match self.actor.as_ref() {
            Some(EditorActor::Guest(guest)) => self.send_guest_message(
                guest,
                CollabMessage::RenewTicket(op_collab::RenewTicket {
                    opaque_ticket: ticket,
                }),
                None,
            ),
            None => Err(CollabRuntimeError::invalid_session()),
            Some(EditorActor::Owner(_)) => unreachable!("handled above"),
        }
    }

    /// Publish the newest local cursor/selection snapshot at most once per
    /// paint interval. Presence is lossy and uses the transport coalescing
    /// lane; it never enters the ordered document transaction.
    pub(crate) fn publish_local_presence(
        &mut self,
        host: &mut WidgetHostNative,
        cursor: Option<(f64, f64)>,
    ) -> bool {
        let state = host.editor_state();
        let presence = Presence {
            cursor: cursor.map(|(x, y)| Point { x, y }),
            selection: state
                .selection
                .set
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            viewport: Some(Viewport {
                pan_x: f64::from(state.viewport.pan_x),
                pan_y: f64::from(state.viewport.pan_y),
                zoom: f64::from(state.viewport.zoom),
            }),
            editing_node: state
                .ui
                .text_editing
                .as_ref()
                .map(|id| id.as_str().to_owned()),
        };
        if self.last_local_presence.as_ref() == Some(&presence) {
            self.pending_local_presence = None;
            return false;
        }
        self.pending_local_presence = Some(presence);
        if self
            .last_presence_sent
            .is_some_and(|last| last.elapsed() < Duration::from_millis(33))
        {
            return false;
        }
        let presence = self
            .pending_local_presence
            .take()
            .expect("changed presence is queued");
        let result = match self.actor.as_ref() {
            Some(EditorActor::Owner(owner)) => {
                let Some(participant) = owner
                    .session
                    .core()
                    .active_participants()
                    .into_iter()
                    .find(|participant| participant.role == op_collab::Role::Owner)
                else {
                    return false;
                };
                self.broadcast_owner_message(
                    owner,
                    CollabMessage::PresenceChanged(ParticipantPresence {
                        participant_id: participant.participant_id,
                        peer_id: participant.peer_id,
                        presence: presence.clone(),
                    }),
                    None,
                )
            }
            Some(EditorActor::Guest(guest))
                if guest.session.core().state() == op_collab::GuestConnectionState::Active =>
            {
                self.send_guest_message(
                    guest,
                    CollabMessage::PresenceUpdate(presence.clone()),
                    Some(GUEST_TO_OWNER_PRESENCE_COALESCE_KEY),
                )
            }
            _ => return false,
        };
        match result {
            Ok(()) => {
                self.last_local_presence = Some(presence);
                self.last_presence_sent = Some(Instant::now());
                true
            }
            Err(error) => {
                self.pending_local_presence = None;
                if error.failure != CollabRuntimeFailure::ResourceLimit {
                    self.fail(host, error.failure);
                }
                false
            }
        }
    }

    pub(crate) fn next_presence_deadline(&self) -> Option<Instant> {
        self.pending_local_presence.as_ref()?;
        self.last_presence_sent?
            .checked_add(Duration::from_millis(33))
    }

    pub(super) fn route_owner_output(
        &mut self,
        owner: &mut OwnerActor,
        output: OwnerEditorOutput,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        let OwnerEditorOutput {
            effects,
            local_edit,
            failed_connections,
        } = output;
        // A queued peer frame can fail after a local gesture completes.
        // Remove every offender before routing successful effects so a commit
        // from a good peer is never broadcast back to a protocol-fatal peer.
        for connection in failed_connections {
            self.close_failed_owner_peer(owner, connection, host)?;
        }
        if let Some(local) = local_edit {
            match local {
                LocalEditResolution::NoChange | LocalEditResolution::Committed(_) => {}
                LocalEditResolution::Rejected(_) => {
                    self.set_notice(
                        host,
                        CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
                    );
                }
            }
        }
        for effect in effects {
            self.route_owner_effect(owner, effect, host)?;
        }
        set_owner_ui(host, owner);
        Ok(())
    }

    fn route_owner_effect(
        &mut self,
        owner: &mut OwnerActor,
        effect: OwnerEffect,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        match effect {
            OwnerEffect::Reply { to, message } if owner.is_local_connection(to) => {
                let CollabMessage::UndoResult(result) = message else {
                    return Err(CollabRuntimeError::invalid_session());
                };
                self.observe_undo_result(&result, host);
                Ok(())
            }
            OwnerEffect::Reply { to, message } => {
                self.send_owner_actor_message(owner, to, message, None)
            }
            OwnerEffect::ReplyCommit { to, commit } => {
                self.send_owner_actor_commit(owner, to, &commit)
            }
            OwnerEffect::Broadcast { message } => {
                self.observe_message(&message, host);
                let coalesce_key = coalesce_key_for_message(&message);
                self.broadcast_owner_message(owner, message, coalesce_key)
            }
            OwnerEffect::BroadcastCommit { commit } => self.broadcast_owner_commit(owner, &commit),
            OwnerEffect::CommitBatch { to, commits } => {
                for commit in commits {
                    self.send_owner_actor_commit(owner, to, &commit)?;
                }
                Ok(())
            }
            OwnerEffect::Snapshot { to, snapshot } => {
                self.send_owner_actor_message(owner, to, CollabMessage::Snapshot(snapshot), None)
            }
            OwnerEffect::UndoCommitted {
                reply_to,
                result,
                commit,
            } => {
                if owner.is_local_connection(reply_to) {
                    self.observe_undo_result(&result, host);
                } else {
                    self.send_owner_actor_message(
                        owner,
                        reply_to,
                        CollabMessage::UndoResult(result),
                        None,
                    )?;
                }
                self.broadcast_owner_commit(owner, &commit)
            }
            OwnerEffect::VerifyRenewal { connection, ticket } => {
                self.send_owner(OwnerNetworkCommand::VerifyRenewal {
                    connection,
                    opaque_ticket: ticket,
                })
            }
            OwnerEffect::UndoRequested(_) => Err(CollabRuntimeError::invalid_session()),
            OwnerEffect::Close { connection, reason } => {
                self.send_owner_actor_message(
                    owner,
                    connection,
                    CollabMessage::Bye(Bye { reason }),
                    None,
                )?;
                self.send_owner(OwnerNetworkCommand::Close { connection })
            }
            OwnerEffect::PrepareInstall(_) => Err(CollabRuntimeError::invalid_session()),
        }
    }

    pub(super) fn route_guest_output(
        &mut self,
        guest: &mut GuestActor,
        output: GuestEditorOutput,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        self.route_guest_output_inner(guest, output, host, true)
    }

    fn route_guest_terminal_output(
        &mut self,
        guest: &mut GuestActor,
        output: GuestEditorOutput,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        self.route_guest_output_inner(guest, output, host, false)
    }

    fn route_guest_output_inner(
        &mut self,
        guest: &mut GuestActor,
        output: GuestEditorOutput,
        host: &mut WidgetHostNative,
        deliver_outbound: bool,
    ) -> Result<(), CollabRuntimeError> {
        if let Some(local) = output.local_edit {
            match local {
                GuestLocalEditResolution::NoChange => {}
                GuestLocalEditResolution::Submitted => {
                    host.editor_state_mut().editor_ui.collab.pending_edit =
                        CollabPendingEditUi::Submitting;
                }
                GuestLocalEditResolution::Rejected(_) => {
                    self.set_notice(
                        host,
                        CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
                    );
                }
            }
        }
        for effect in output.effects {
            match effect {
                GuestEffect::Send(message) if deliver_outbound => {
                    let coalesce_key = coalesce_key_for_message(&message);
                    self.send_guest_message(guest, message, coalesce_key)?
                }
                GuestEffect::Send(_) => {}
                GuestEffect::ParticipantJoined(_) | GuestEffect::ParticipantLeft(_) => {}
                GuestEffect::PresenceChanged(presence) => {
                    self.project_presence(&presence, host);
                }
                GuestEffect::VerifyRenewal { ticket } => {
                    self.send_guest(GuestNetworkCommand::VerifyRenewal(ticket))?;
                }
                GuestEffect::PendingCancelled { .. } => {
                    self.set_notice(host, CollabNoticeKind::Reject(CollabRejectUiCode::Conflict));
                }
                GuestEffect::UndoResult(result) => {
                    self.observe_undo_result(&result, host);
                }
                GuestEffect::SessionEnded { reason } => {
                    let notice = match reason {
                        ByeReason::OwnerLeft | ByeReason::Normal => CollabNoticeKind::OwnerLeft,
                        ByeReason::AuthenticationExpired => CollabNoticeKind::TicketExpired,
                        ByeReason::ProtocolError | ByeReason::ResourceLimit => {
                            CollabNoticeKind::Reject(CollabRejectUiCode::Unknown)
                        }
                    };
                    self.set_notice(host, notice);
                    set_guest_ui(host, guest, CollabConnectionPhase::Ended);
                }
                GuestEffect::PrepareInstall(_) => {
                    return Err(CollabRuntimeError::invalid_session());
                }
            }
        }
        host.editor_state_mut().editor_ui.collab.pending_edit =
            if guest.session.core().pending_undo_request().is_some() {
                CollabPendingEditUi::Replaying
            } else if guest.session.core().pending_edit().is_some() {
                CollabPendingEditUi::Submitting
            } else {
                CollabPendingEditUi::None
            };
        let phase = match guest.session.core().state() {
            op_collab::GuestConnectionState::Active => CollabConnectionPhase::Active,
            op_collab::GuestConnectionState::Disconnected => CollabConnectionPhase::Reconnecting,
            op_collab::GuestConnectionState::Ended => CollabConnectionPhase::Ended,
            op_collab::GuestConnectionState::AwaitingSnapshot => {
                CollabConnectionPhase::Authenticating
            }
        };
        if phase.is_authenticated() {
            set_guest_ui(host, guest, phase);
            if phase == CollabConnectionPhase::Active {
                self.push_status(CollabStatusEvent::SessionActive {
                    role: guest.session.core().role(),
                });
            }
        } else {
            host.editor_state_mut().editor_ui.collab.phase = phase;
        }
        Ok(())
    }

    pub(super) fn send_owner(
        &self,
        command: OwnerNetworkCommand,
    ) -> Result<(), CollabRuntimeError> {
        self.network
            .as_ref()
            .ok_or_else(CollabRuntimeError::invalid_session)?
            .send_owner(command)
            .map_err(command_send_error)
    }

    fn send_guest(&self, command: GuestNetworkCommand) -> Result<(), CollabRuntimeError> {
        self.network
            .as_ref()
            .ok_or_else(CollabRuntimeError::invalid_session)?
            .send_guest(command)
            .map_err(command_send_error)
    }

    fn observe_message(&mut self, message: &CollabMessage, host: &mut WidgetHostNative) {
        if let CollabMessage::PresenceChanged(presence) = message {
            self.project_presence(presence, host);
        }
    }

    fn observe_undo_result(&self, result: &UndoResult, host: &mut WidgetHostNative) {
        if let Some(notice) = undo_notice(result.outcome, result.details.is_some()) {
            self.set_notice(host, notice);
        }
    }

    fn project_presence(&self, presence: &ParticipantPresence, host: &mut WidgetHostNative) {
        let participant_key = presence.participant_id.as_ref().to_owned();
        let point = presence
            .presence
            .cursor
            .map(|point| op_editor_core::CollabCanvasPoint {
                x: point.x,
                y: point.y,
            });
        let item = RemotePresenceUi::bounded(
            participant_key,
            point,
            presence.presence.selection.clone(),
            presence.presence.editing_node.clone(),
            self.now_ms(),
        );
        host.editor_state_mut()
            .editor_ui
            .collab
            .queue_presence_update(item);
    }

    pub(super) fn fail(&mut self, host: &mut WidgetHostNative, failure: CollabRuntimeFailure) {
        if matches!(
            failure,
            CollabRuntimeFailure::ResourceLimit | CollabRuntimeFailure::Transport
        ) {
            self.fail_network(host, failure);
            return;
        }
        let notice = if failure.is_authentication() {
            CollabNoticeKind::Reject(CollabRejectUiCode::Authentication)
        } else if failure == CollabRuntimeFailure::ResourceLimit {
            CollabNoticeKind::Reject(CollabRejectUiCode::ResourceLimit)
        } else {
            CollabNoticeKind::Reject(CollabRejectUiCode::Unknown)
        };
        self.set_notice(host, notice);
        self.push_status(CollabStatusEvent::Failed(failure));
        if matches!(self.actor, Some(EditorActor::Guest(_))) {
            if let Some(EditorActor::Guest(guest)) = self.actor.as_ref() {
                set_guest_ui(host, guest, CollabConnectionPhase::Reconnecting);
            }
        } else if self.actor.is_none() {
            self.retire_workers();
            self.pending_guest = None;
            host.disable_collaboration_ids();
            host.editor_state_mut().editor_ui.collab.phase = CollabConnectionPhase::Idle;
        }
        host.mark_editor_state_dirty();
    }

    /// A session-wide network or reliable-delivery failure must never leave
    /// the UI/core claiming Active. Owners fall back to the still-current
    /// standalone document; guests retain confirmed+pending state read-only so
    /// an explicit retry can resume idempotently.
    pub(super) fn fail_network(
        &mut self,
        host: &mut WidgetHostNative,
        failure: CollabRuntimeFailure,
    ) {
        self.push_status(CollabStatusEvent::Failed(failure));
        match self.actor.as_ref() {
            Some(EditorActor::Owner(_)) => {
                self.leave(host);
            }
            Some(EditorActor::Guest(_)) => {
                self.retire_workers();
                self.pending_guest = None;
                self.transaction_active = false;
                if let Some(EditorActor::Guest(guest)) = self.actor.as_mut() {
                    let ended =
                        guest.session.core().state() == op_collab::GuestConnectionState::Ended;
                    let _ = guest.session.disconnect(host);
                    guest.connection = None;
                    if ended {
                        set_guest_ui(host, guest, CollabConnectionPhase::Ended);
                    } else {
                        set_guest_ui(host, guest, CollabConnectionPhase::Reconnecting);
                        self.push_status(CollabStatusEvent::Reconnecting);
                    }
                }
            }
            None => {
                self.retire_workers();
                self.pending_guest = None;
                self.transaction_active = false;
                host.disable_collaboration_ids();
                host.editor_state_mut().editor_ui.collab.phase = CollabConnectionPhase::Idle;
            }
        }
        self.set_notice(host, disconnect_notice(failure));
        host.mark_editor_state_dirty();
    }

    pub(super) fn network_stopped(&mut self, host: &mut WidgetHostNative) {
        if self.network.is_some() {
            self.fail_network(host, CollabRuntimeFailure::Transport);
        }
    }

    pub(super) fn connection_closed(
        &mut self,
        connection: ConnectionKey,
        failure: Option<CollabRuntimeFailure>,
        remote_bye: Option<RemoteBye>,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        if let Some((request_key, _)) = self.pending_owner.remove_connection(connection) {
            host.editor_state_mut()
                .editor_ui
                .collab
                .remove_pending_admission(&request_key);
            // Admission timeout/rejection is scoped to this unauthorised
            // socket and must not degrade the owner's live session.
            return Ok(());
        }
        let active_guest = matches!(
            self.actor.as_ref(),
            Some(EditorActor::Guest(guest)) if guest.connection == Some(connection)
        );
        let pending_guest_connection = self
            .pending_guest
            .as_ref()
            .map(|pending| pending.connection);
        let Some(mut actor) = self.actor.take() else {
            if let Some(failure) = failure {
                return Err(CollabRuntimeError::new(failure));
            }
            return Ok(());
        };
        if matches!(
            &actor,
            EditorActor::Owner(owner) if !owner.connections.contains(&connection)
        ) {
            // Handshake failures and explicitly rejected requests never
            // entered SessionCore.
            self.actor = Some(actor);
            return Ok(());
        }
        if matches!(
            &actor,
            EditorActor::Guest(guest)
                if guest.connection != Some(connection)
                    && pending_guest_connection != Some(connection)
        ) {
            self.actor = Some(actor);
            return Ok(());
        }
        let result = match &mut actor {
            EditorActor::Owner(owner) => {
                owner.connections.remove(&connection);
                let output = owner
                    .session
                    .disconnect(connection)
                    .map_err(|_| CollabRuntimeError::invalid_session())?;
                self.route_owner_output(owner, output, host)?;
                set_owner_ui(host, owner);
                self.push_status(CollabStatusEvent::PeerDisconnected { connection });
                Ok(())
            }
            EditorActor::Guest(guest) => {
                if pending_guest_connection == Some(connection) {
                    self.pending_guest = None;
                }
                if active_guest {
                    let output = guest
                        .session
                        .finish_inbound_stream(remote_bye.map(RemoteBye::into_frame), host)
                        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
                    // EOF makes outbound Applied/CatchUp frames advisory only:
                    // install all received authority and the typed Bye without
                    // depending on an already-closed command lane.
                    self.route_guest_terminal_output(guest, output, host)?;
                }
                guest.connection = None;
                guest
                    .session
                    .disconnect(host)
                    .map_err(|_| CollabRuntimeError::invalid_session())?;
                self.retire_workers();
                self.transaction_active = false;
                if guest.session.core().state() == op_collab::GuestConnectionState::Ended {
                    // A terminal Bye is delivered before the transport EOF.
                    // Preserve OwnerLeft/Ended so Retry cannot replace the
                    // required Save As fork flow.
                    set_guest_ui(host, guest, CollabConnectionPhase::Ended);
                } else {
                    set_guest_ui(host, guest, CollabConnectionPhase::Reconnecting);
                    self.set_notice(
                        host,
                        disconnect_notice(failure.unwrap_or(CollabRuntimeFailure::Transport)),
                    );
                    self.push_status(CollabStatusEvent::Reconnecting);
                }
                Ok(())
            }
        };
        self.actor = Some(actor);
        result
    }

    /// Owner-core frame errors are fatal only to the authenticated input
    /// connection. `OwnerSessionCore::accept_frame` marks it closing; complete
    /// the contract by closing transport and disconnecting the retained peer.
    pub(super) fn close_failed_owner_peer(
        &mut self,
        owner: &mut OwnerActor,
        connection: ConnectionKey,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        let close_result = self.send_owner(OwnerNetworkCommand::Close { connection });
        if !owner.connections.remove(&connection) {
            return close_result;
        }
        let disconnect_result = owner
            .session
            .disconnect(connection)
            .map_err(|_| CollabRuntimeError::invalid_session())
            .and_then(|output| self.route_owner_output(owner, output, host));
        set_owner_ui(host, owner);
        self.push_status(CollabStatusEvent::PeerDisconnected { connection });
        close_result.and(disconnect_result)
    }

    pub(super) fn set_notice(&self, host: &mut WidgetHostNative, notice: CollabNoticeKind) {
        let now = self.now_ms();
        host.editor_state_mut()
            .editor_ui
            .collab
            .set_notice(notice, now);
        host.mark_editor_state_dirty();
    }
}

fn undo_notice(outcome: UndoOutcome, has_details: bool) -> Option<CollabNoticeKind> {
    (outcome != UndoOutcome::Committed || has_details).then_some(CollabNoticeKind::UndoConflict)
}

fn coalesce_key_for_message(message: &CollabMessage) -> Option<u64> {
    match message {
        // Guest-to-owner presence has exactly one participant source per
        // connection, so one latest-value slot is collision-free.
        CollabMessage::PresenceUpdate(_) => Some(GUEST_TO_OWNER_PRESENCE_COALESCE_KEY),
        // Owner broadcasts multiplex participants onto every peer queue. A
        // u64 hash cannot prove collision freedom, so preserve each update.
        CollabMessage::PresenceChanged(_) => None,
        _ => None,
    }
}

pub(super) fn command_send_error(error: NetworkCommandSendError) -> CollabRuntimeError {
    match error {
        NetworkCommandSendError::Full => CollabRuntimeError::resource_limit(),
        NetworkCommandSendError::Disconnected => {
            CollabRuntimeError::new(CollabRuntimeFailure::Transport)
        }
    }
}

pub(super) fn disconnect_notice(failure: CollabRuntimeFailure) -> CollabNoticeKind {
    match failure {
        CollabRuntimeFailure::TicketRejected => CollabNoticeKind::TicketExpired,
        CollabRuntimeFailure::AuthenticationUnavailable => {
            CollabNoticeKind::Reject(CollabRejectUiCode::Authentication)
        }
        CollabRuntimeFailure::ResourceLimit => {
            CollabNoticeKind::Reject(CollabRejectUiCode::ResourceLimit)
        }
        CollabRuntimeFailure::SecureKeyUnavailable
        | CollabRuntimeFailure::ClockUnavailable
        | CollabRuntimeFailure::InvalidAddress
        | CollabRuntimeFailure::InvalidSession
        | CollabRuntimeFailure::Transport
        | CollabRuntimeFailure::Protocol => CollabNoticeKind::DisconnectedReadOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_and_conflicted_undo_results_remain_visible() {
        assert_eq!(undo_notice(UndoOutcome::Committed, false), None);
        assert_eq!(
            undo_notice(UndoOutcome::Committed, true),
            Some(CollabNoticeKind::UndoConflict)
        );
        assert_eq!(
            undo_notice(UndoOutcome::Conflict, false),
            Some(CollabNoticeKind::UndoConflict)
        );
        assert_eq!(
            undo_notice(UndoOutcome::Rejected, false),
            Some(CollabNoticeKind::UndoConflict)
        );
    }

    #[test]
    fn only_single_source_guest_presence_is_coalesced() {
        let presence = Presence {
            cursor: None,
            selection: Vec::new(),
            viewport: None,
            editing_node: None,
        };
        assert_eq!(
            coalesce_key_for_message(&CollabMessage::PresenceUpdate(presence.clone())),
            Some(GUEST_TO_OWNER_PRESENCE_COALESCE_KEY)
        );
        for suffix in ["a", "b"] {
            assert_eq!(
                coalesce_key_for_message(&CollabMessage::PresenceChanged(ParticipantPresence {
                    participant_id: format!("participant-{suffix}").into(),
                    peer_id: format!("peer-{suffix}").into(),
                    presence: presence.clone(),
                })),
                None
            );
        }
    }
}
