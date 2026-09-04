use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::runtime::local_edit::LocalEditOutcome;

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

use super::actor::{set_guest_ui, set_owner_ui, EditorActor, GuestActor, OwnerActor};
use super::failure::disconnect_notice;
use super::network::NetworkCommandSendError;
use super::types::{
    CollabRuntimeError, CollabRuntimeFailure, CollabStatusEvent, DiscoveredEndpoint,
    GuestNetworkCommand, OwnerNetworkCommand, RemoteBye,
};
use super::CollabRuntime;
use crate::host::CollabHost;

const GUEST_TO_OWNER_PRESENCE_COALESCE_KEY: u64 = 1;

impl CollabRuntime {
    pub(super) fn update_discovery(
        &mut self,
        sessions: Vec<DiscoveredEndpoint>,
        host: &mut impl CollabHost,
    ) {
        self.discovered = sessions
            .iter()
            .cloned()
            .map(|session| (session.discovery_id.clone(), session))
            .collect();
        let discovered = Arc::new(
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
        let panel = &mut host.editor_state_mut().editor_ui.collab.panel;
        if panel.discovered != discovered {
            panel.hover = None;
            panel.discovered = discovered;
        }
    }

    pub(super) fn require_ready(&self, host: &impl CollabHost) -> Result<(), CollabRuntimeError> {
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
            // Retain one credential for pending authenticated sockets; each peer
            // receives a separately zeroizing encoding borrowed from owner storage.
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

    /// Publish lossy local presence at most once per paint interval.
    pub fn publish_local_presence(
        &mut self,
        host: &mut impl CollabHost,
        cursor: Option<(f64, f64)>,
    ) -> bool {
        self.publish_local_presence_inner(host, cursor, false)
    }

    /// Publish a changed terminal presence without the paint-frame throttle.
    ///
    /// Mobile suspension uses this after clearing its pointer. The enqueue is
    /// still lossy/coalesced, but a recent cursor frame cannot defer the final
    /// `None` until a foreground pump that may no longer run.
    pub fn publish_local_presence_immediately(
        &mut self,
        host: &mut impl CollabHost,
        cursor: Option<(f64, f64)>,
    ) -> bool {
        self.publish_local_presence_inner(host, cursor, true)
    }

    fn publish_local_presence_inner(
        &mut self,
        host: &mut impl CollabHost,
        cursor: Option<(f64, f64)>,
        bypass_throttle: bool,
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
        if !bypass_throttle
            && self
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

    pub fn next_presence_deadline(&self) -> Option<Instant> {
        self.pending_local_presence.as_ref()?;
        self.last_presence_sent?
            .checked_add(Duration::from_millis(33))
    }

    pub(super) fn route_owner_output(
        &mut self,
        owner: &mut OwnerActor,
        output: OwnerEditorOutput,
        host: &mut impl CollabHost,
    ) -> Result<(), CollabRuntimeError> {
        let OwnerEditorOutput {
            effects,
            local_edit,
            failed_connections,
        } = output;
        // Remove failed peers before routing successful effects back to the survivors.
        for connection in failed_connections {
            self.close_failed_owner_peer(owner, connection, host)?;
        }
        if let Some(local) = local_edit {
            // Recorded so `finish_local_edit` can report what actually
            // happened. Swallowing the rejection here is what let a rolled-back
            // push be answered 200.
            self.last_local_edit = Some(match local {
                LocalEditResolution::NoChange => LocalEditOutcome::NoChange,
                LocalEditResolution::Committed(_) => LocalEditOutcome::Committed,
                LocalEditResolution::Rejected(_) => {
                    self.set_notice(
                        host,
                        CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
                    );
                    LocalEditOutcome::Rejected
                }
            });
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
        host: &mut impl CollabHost,
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
        host: &mut impl CollabHost,
    ) -> Result<(), CollabRuntimeError> {
        self.route_guest_output_inner(guest, output, host, true)
    }

    fn route_guest_terminal_output(
        &mut self,
        guest: &mut GuestActor,
        output: GuestEditorOutput,
        host: &mut impl CollabHost,
    ) -> Result<(), CollabRuntimeError> {
        self.route_guest_output_inner(guest, output, host, false)
    }

    fn route_guest_output_inner(
        &mut self,
        guest: &mut GuestActor,
        output: GuestEditorOutput,
        host: &mut impl CollabHost,
        deliver_outbound: bool,
    ) -> Result<(), CollabRuntimeError> {
        if let Some(local) = output.local_edit {
            self.last_local_edit = Some(match local {
                GuestLocalEditResolution::NoChange => LocalEditOutcome::NoChange,
                GuestLocalEditResolution::Submitted => {
                    host.editor_state_mut().editor_ui.collab.pending_edit =
                        CollabPendingEditUi::Submitting;
                    // Submitted, not yet acknowledged — but it IS on the wire
                    // and the local document holds it, so the push landed.
                    LocalEditOutcome::Committed
                }
                GuestLocalEditResolution::Rejected(_) => {
                    self.set_notice(
                        host,
                        CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
                    );
                    LocalEditOutcome::Rejected
                }
            });
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
                GuestEffect::PendingCancelled {
                    reason, changes, ..
                } => {
                    self.observe_pending_cancelled(reason, changes, host);
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
        if phase == CollabConnectionPhase::Ended {
            // Ended keeps the authenticated projection for the fork flow, so
            // `clear_authenticated` never runs here — drop the replay stash
            // explicitly before it can outlive the session that produced it.
            self.clear_discarded_stash(host);
            self.reset_guest_reconnect();
        }
        if phase.is_authenticated() {
            // Status events are the only diagnostic trace a headless or mobile
            // host has (both print them to stderr), so they have to stay a log
            // of what changed. Republishing Active for every routed frame made
            // it a log of what arrived: presence alone repeats it ~30 times a
            // second while the other peer drags, which buries the one
            // `Failed(..)` line that says why a session dropped. Report the
            // transition only.
            let was_active =
                host.editor_state().editor_ui.collab.phase == CollabConnectionPhase::Active;
            set_guest_ui(host, guest, phase);
            self.publish_guest_connection_path(host);
            if phase == CollabConnectionPhase::Active && !was_active {
                self.reset_guest_reconnect();
                self.push_status(CollabStatusEvent::SessionActive {
                    role: guest.session.core().role(),
                });
            }
        } else {
            host.editor_state_mut().editor_ui.collab.set_phase(phase);
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

    fn observe_message(&mut self, message: &CollabMessage, host: &mut impl CollabHost) {
        if let CollabMessage::PresenceChanged(presence) = message {
            self.project_presence(presence, host);
        }
    }

    fn observe_undo_result(&self, result: &UndoResult, host: &mut impl CollabHost) {
        if let Some(notice) = undo_notice(result.outcome, result.details.is_some()) {
            self.set_notice(host, notice);
        }
    }

    fn project_presence(&self, presence: &ParticipantPresence, host: &mut impl CollabHost) {
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

    pub(super) fn connection_closed(
        &mut self,
        connection: ConnectionKey,
        failure: Option<CollabRuntimeFailure>,
        remote_bye: Option<RemoteBye>,
        host: &mut impl CollabHost,
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
        let reconnect_failure = failure.unwrap_or(CollabRuntimeFailure::Transport);
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
                    self.clear_discarded_stash(host);
                } else {
                    set_guest_ui(host, guest, CollabConnectionPhase::Reconnecting);
                    self.set_notice(host, disconnect_notice(reconnect_failure));
                    self.push_status(CollabStatusEvent::Reconnecting);
                }
                Ok(())
            }
        };
        let reconnect = result.is_ok()
            && matches!(
                &actor,
                EditorActor::Guest(guest)
                    if guest.session.core().state()
                        == op_collab::GuestConnectionState::Disconnected
            );
        self.actor = Some(actor);
        if reconnect {
            self.schedule_guest_reconnect(reconnect_failure);
        } else {
            self.reset_guest_reconnect();
        }
        result
    }

    /// Owner-core frame errors are fatal only to the authenticated input
    /// connection. `OwnerSessionCore::accept_frame` marks it closing; complete
    /// the contract by closing transport and disconnecting the retained peer.
    pub(super) fn close_failed_owner_peer(
        &mut self,
        owner: &mut OwnerActor,
        connection: ConnectionKey,
        host: &mut impl CollabHost,
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

    pub(super) fn set_notice(&self, host: &mut impl CollabHost, notice: CollabNoticeKind) {
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
