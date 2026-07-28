//! Desktop collaboration orchestration.
//!
//! The GUI thread exclusively owns the editor/session actor. Network workers
//! own sockets and framed bytes, and cross this boundary through bounded typed
//! channels plus a winit wake event.

mod actor;
mod admission;
mod auth;
mod effects;
mod effects_wire;
mod network;
mod poll;
mod support;
mod types;

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use actor::{
    random_identifier, set_guest_ui, set_owner_ui, EditorActor, GuestActor, OwnerActor,
    PendingGuestAdmission,
};
use admission::PendingOwnerAdmissions;
use network::{
    DiscoveryNetwork, EventSink, PendingNetworkLaunch, Retirement, SessionNetwork,
    TerminalEventLane, TerminalEventReceiver,
};
use op_collab::{
    CollabMessage, ConnectionKey, Epoch, FrameEnvelope, OpaqueTicket, Role, SessionId,
};
use op_collab_transport::{
    JoinIntent, ResumeHint, SharedQueueBudget, StaticKeyStore, TransportConfig,
};
use op_editor_core::{
    CollabAvailability, CollabConnectionPhase, CollabNoticeKind, CollabPendingEditUi,
    CollabRejectUiCode, CollabUiAction,
};
use op_host_native::WidgetHostNative;
use support::{production_key_store, random_epoch};
use winit::event_loop::EventLoopProxy;

use crate::DesktopEvent;
use types::{
    CollabRuntimeError, CollabRuntimeFailure, CollabStatusEvent, DiscoveredEndpoint, NetworkEvent,
    TaggedNetworkEvent,
};

const MAX_STATUS_EVENTS: usize = 64;

pub(crate) struct DesktopCollabRuntime {
    events: Receiver<TaggedNetworkEvent>,
    event_sender: SyncSender<TaggedNetworkEvent>,
    terminal_event_sender: TerminalEventLane,
    terminal_events: TerminalEventReceiver,
    wake_proxy: Arc<Mutex<Option<EventLoopProxy<DesktopEvent>>>>,
    network: Option<SessionNetwork>,
    discovery: Option<DiscoveryNetwork>,
    retirement: Option<Retirement>,
    pending_network_launch: Option<PendingNetworkLaunch>,
    actor: Option<EditorActor>,
    pending_guest: Option<PendingGuestAdmission>,
    pending_owner: PendingOwnerAdmissions,
    discovered: HashMap<String, DiscoveredEndpoint>,
    last_join: Option<(Vec<SocketAddr>, Option<String>)>,
    pinned_owner_static: Option<[u8; 32]>,
    transaction_active: bool,
    save_as_fork_requested: bool,
    status: VecDeque<CollabStatusEvent>,
    generation: u64,
    clock_start: Instant,
    last_presence_sent: Option<Instant>,
    last_local_presence: Option<op_collab::Presence>,
    pending_local_presence: Option<op_collab::Presence>,
    latest_owner_ticket: Option<OpaqueTicket>,
    key_store: Arc<dyn StaticKeyStore>,
    bridge_budget: SharedQueueBudget,
}

impl DesktopCollabRuntime {
    pub(crate) fn new() -> Self {
        Self::with_key_store(production_key_store())
    }

    fn with_key_store(key_store: Arc<dyn StaticKeyStore>) -> Self {
        let maximum = TransportConfig::default().connections.global_queued_bytes;
        let bridge_budget =
            SharedQueueBudget::new(maximum).expect("default collaboration bridge budget is valid");
        Self::with_key_store_and_bridge_budget(key_store, bridge_budget)
    }

    fn with_key_store_and_bridge_budget(
        key_store: Arc<dyn StaticKeyStore>,
        bridge_budget: SharedQueueBudget,
    ) -> Self {
        let (event_sender, events, terminal_event_sender, terminal_events) =
            network::event_channel();
        Self {
            events,
            event_sender,
            terminal_event_sender,
            terminal_events,
            wake_proxy: Arc::new(Mutex::new(None)),
            network: None,
            discovery: None,
            retirement: None,
            pending_network_launch: None,
            actor: None,
            pending_guest: None,
            pending_owner: PendingOwnerAdmissions::default(),
            discovered: HashMap::new(),
            last_join: None,
            pinned_owner_static: None,
            transaction_active: false,
            save_as_fork_requested: false,
            status: VecDeque::new(),
            generation: 1,
            clock_start: Instant::now(),
            last_presence_sent: None,
            last_local_presence: None,
            pending_local_presence: None,
            latest_owner_ticket: None,
            key_store,
            bridge_budget,
        }
    }

    pub(crate) fn set_wake_proxy(&mut self, proxy: EventLoopProxy<DesktopEvent>) {
        if let Ok(mut slot) = self.wake_proxy.lock() {
            *slot = Some(proxy);
        }
    }

    pub(crate) fn refresh_availability(&mut self, host: &mut WidgetHostNative) -> bool {
        let next = if !op_auth_bridge::collab_ticket_available() {
            CollabAvailability::Unavailable
        } else if host.editor_state().editor_ui.account.is_signed_in() {
            CollabAvailability::Ready
        } else {
            CollabAvailability::SignInRequired
        };
        let collab = &mut host.editor_state_mut().editor_ui.collab;
        if collab.availability == next {
            return false;
        }
        collab.availability = next;
        host.mark_editor_state_dirty();
        true
    }

    pub(crate) fn drain_ui_action(&mut self, host: &mut WidgetHostNative) -> bool {
        let Some(action) = host
            .editor_state_mut()
            .editor_ui
            .collab
            .take_pending_action()
        else {
            return false;
        };
        let result = match action {
            CollabUiAction::Start => self.start_owner(host),
            CollabUiAction::BeginDiscovery => self.begin_discovery(host),
            CollabUiAction::JoinDiscovered { discovery_id } => {
                self.join_discovered(host, &discovery_id)
            }
            CollabUiAction::JoinAddress { endpoint } => self.join_address(host, &endpoint),
            CollabUiAction::Cancel | CollabUiAction::Leave | CollabUiAction::DiscardPending => {
                self.leave(host);
                Ok(())
            }
            CollabUiAction::Retry => self.retry_guest(host),
            CollabUiAction::SaveAsFork => {
                self.save_as_fork_requested = true;
                Ok(())
            }
            CollabUiAction::ApproveAdmissionEditor { request_key } => {
                self.approve_owner_admission(&request_key, Role::Editor, host)
            }
            CollabUiAction::ApproveAdmissionViewer { request_key } => {
                self.approve_owner_admission(&request_key, Role::Viewer, host)
            }
            CollabUiAction::RejectAdmission { request_key } => {
                self.reject_owner_admission(&request_key, host)
            }
        };
        if let Err(error) = result {
            self.fail(host, error.failure);
        }
        host.mark_editor_state_dirty();
        true
    }

    pub(crate) fn take_save_as_fork_request(&mut self) -> bool {
        std::mem::take(&mut self.save_as_fork_requested)
    }

    /// Starts one GUI-owned edit capture. A `false` return means another
    /// gesture already owns the transaction, or no live session is bound.
    pub(crate) fn begin_local_edit(&mut self, host: &mut WidgetHostNative) -> bool {
        if self.transaction_active {
            return false;
        }
        let started = match self.actor.as_mut() {
            Some(EditorActor::Owner(actor)) => actor.session.begin_local_edit(host).is_ok(),
            Some(EditorActor::Guest(actor)) => actor.session.begin_local_edit(host).is_ok(),
            None => return false,
        };
        match started {
            true => {
                self.transaction_active = true;
                true
            }
            false => {
                self.set_notice(host, CollabNoticeKind::Reject(CollabRejectUiCode::ReadOnly));
                false
            }
        }
    }

    pub(crate) fn finish_local_edit(&mut self, host: &mut WidgetHostNative) -> bool {
        if !std::mem::take(&mut self.transaction_active) {
            return false;
        }
        let Some(mut actor) = self.actor.take() else {
            return false;
        };
        let result = match &mut actor {
            EditorActor::Owner(owner) => match owner.session.finish_local_edit(host) {
                Ok(output) => self.route_owner_output(owner, output, host),
                Err(_) => Err(CollabRuntimeError::invalid_session()),
            },
            EditorActor::Guest(guest) => match guest.session.finish_local_edit(host) {
                Ok(output) => self.route_guest_output(guest, output, host),
                Err(_) => Err(CollabRuntimeError::invalid_session()),
            },
        };
        self.actor = Some(actor);
        match result {
            Ok(()) => true,
            Err(error) => {
                // A local editor/core failure can occur after the document
                // changed or the sequencer prepared a commit. Continuing to
                // advertise Active would silently fork owner and guests.
                self.fail_network(host, error.failure);
                false
            }
        }
    }

    /// Route Cmd/Ctrl+Z to M1 selective undo while a collaboration actor is
    /// bound. Returns `false` only in standalone mode, where the native global
    /// history path remains valid.
    pub(crate) fn request_undo(&mut self, host: &mut WidgetHostNative) -> bool {
        let Some(mut actor) = self.actor.take() else {
            return false;
        };
        let result = match &mut actor {
            EditorActor::Owner(owner) => {
                let Some(target) = owner.session.latest_own_undo_target() else {
                    self.actor = Some(actor);
                    self.set_notice(
                        host,
                        CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
                    );
                    return true;
                };
                owner
                    .session
                    .next_own_undo_request(target)
                    .map_err(|_| CollabRuntimeError::invalid_session())
                    .and_then(|request| {
                        owner
                            .session
                            .request_own_undo(request, host)
                            .map_err(|_| CollabRuntimeError::invalid_session())
                    })
                    .and_then(|output| self.route_owner_output(owner, output, host))
            }
            EditorActor::Guest(guest) => {
                let Some(target) = guest.session.latest_undo_target() else {
                    self.actor = Some(actor);
                    self.set_notice(
                        host,
                        CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
                    );
                    return true;
                };
                guest
                    .session
                    .request_undo(target, host)
                    .map_err(|_| CollabRuntimeError::invalid_session())
                    .and_then(|output| self.route_guest_output(guest, output, host))
            }
        };
        self.actor = Some(actor);
        if let Err(error) = result {
            // Once an undo target exists, failures are actor-local sequencing,
            // installation, or reliable-delivery failures. Fail the session
            // closed instead of leaving a potentially divergent Active core.
            self.fail_network(host, error.failure);
        }
        true
    }

    /// Redo remains intentionally unavailable in M1 collaboration.
    pub(crate) fn reject_redo(&self, host: &mut WidgetHostNative) -> bool {
        if self.actor.is_none() {
            return false;
        }
        self.set_notice(
            host,
            CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
        );
        true
    }

    pub(crate) fn leave(&mut self, host: &mut WidgetHostNative) {
        self.retire_workers();
        if let Some(EditorActor::Guest(guest)) = self.actor.as_mut() {
            let _ = guest.session.disconnect(host);
        }
        self.actor = None;
        self.pending_guest = None;
        self.pending_owner.clear();
        self.discovered.clear();
        self.last_join = None;
        self.pinned_owner_static = None;
        self.transaction_active = false;
        self.last_presence_sent = None;
        self.last_local_presence = None;
        self.pending_local_presence = None;
        self.latest_owner_ticket = None;
        host.disable_collaboration_ids();
        let collab = &mut host.editor_state_mut().editor_ui.collab;
        collab.phase = CollabConnectionPhase::Idle;
        collab.pending_edit = CollabPendingEditUi::None;
        collab.clear_authenticated();
        collab.panel.discovered = Arc::new(Vec::new());
        host.mark_editor_state_dirty();
        self.push_status(CollabStatusEvent::SessionEnded);
    }

    pub(crate) fn drain_status_events(&mut self) -> impl Iterator<Item = CollabStatusEvent> + '_ {
        self.status.drain(..)
    }

    fn start_owner(&mut self, host: &mut WidgetHostNative) -> Result<(), CollabRuntimeError> {
        self.require_ready(host)?;
        self.leave(host);
        let session_id = SessionId::from(random_identifier("session")?);
        let epoch = random_epoch()?;
        host.editor_state_mut().editor_ui.collab.phase = CollabConnectionPhase::Starting;
        self.defer_owner_launch(session_id, epoch);
        Ok(())
    }

    fn begin_discovery(&mut self, host: &mut WidgetHostNative) -> Result<(), CollabRuntimeError> {
        self.require_ready(host)?;
        if self.actor.is_some() {
            return Err(CollabRuntimeError::invalid_session());
        }
        self.retire_workers();
        self.discovered.clear();
        host.editor_state_mut().editor_ui.collab.phase = CollabConnectionPhase::Discovering;
        self.defer_discovery_launch();
        Ok(())
    }

    fn join_discovered(
        &mut self,
        host: &mut WidgetHostNative,
        discovery_id: &str,
    ) -> Result<(), CollabRuntimeError> {
        let discovered = self
            .discovered
            .get(discovery_id)
            .cloned()
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        if !discovered.compatible {
            return Err(CollabRuntimeError::invalid_session());
        }
        self.start_guest(
            host,
            discovered.addresses,
            Some(discovered.discovery_id),
            JoinIntent::New,
        )
    }

    fn join_address(
        &mut self,
        host: &mut WidgetHostNative,
        endpoint: &str,
    ) -> Result<(), CollabRuntimeError> {
        let endpoint = endpoint
            .trim()
            .parse()
            .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::InvalidAddress))?;
        self.start_guest(host, vec![endpoint], None, JoinIntent::New)
    }

    fn retry_guest(&mut self, host: &mut WidgetHostNative) -> Result<(), CollabRuntimeError> {
        let (addresses, _) = self
            .last_join
            .clone()
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        let Some(EditorActor::Guest(guest)) = self.actor.as_ref() else {
            return Err(CollabRuntimeError::invalid_session());
        };
        let core = guest.session.core();
        let intent = JoinIntent::Resume(ResumeHint {
            participant_id: core.participant_id().clone(),
            peer_id: core.peer_id().clone(),
            peer_namespace: core.peer_namespace().clone(),
            role: core.role(),
        });
        // Discovery ids rotate while a session is live. Resume is bound by
        // the Noise static, verified ticket, and core resume identity, so the
        // initial mDNS id must not become a stale reconnect pin.
        let expected_remote_static = self
            .pinned_owner_static
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        self.spawn_guest(host, addresses, None, Some(expected_remote_static), intent)
    }

    fn start_guest(
        &mut self,
        host: &mut WidgetHostNative,
        addresses: Vec<SocketAddr>,
        discovery_id: Option<String>,
        intent: JoinIntent,
    ) -> Result<(), CollabRuntimeError> {
        self.require_ready(host)?;
        self.leave(host);
        self.spawn_guest(host, addresses, discovery_id, None, intent)
    }

    fn spawn_guest(
        &mut self,
        host: &mut WidgetHostNative,
        addresses: Vec<SocketAddr>,
        discovery_id: Option<String>,
        expected_remote_static: Option<[u8; 32]>,
        intent: JoinIntent,
    ) -> Result<(), CollabRuntimeError> {
        let endpoint = *addresses
            .first()
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        self.require_ready(host)?;
        self.retire_workers();
        self.pending_guest = None;
        host.editor_state_mut().editor_ui.collab.phase = CollabConnectionPhase::Joining;
        self.last_join = Some((addresses.clone(), discovery_id.clone()));
        self.defer_guest_launch(addresses, discovery_id, expected_remote_static, intent);
        self.push_status(CollabStatusEvent::JoinStarted { endpoint });
        Ok(())
    }

    fn event_sink(&self) -> EventSink {
        let slot = Arc::clone(&self.wake_proxy);
        EventSink::new(
            self.event_sender.clone(),
            self.terminal_event_sender.clone(),
            self.bridge_budget.clone(),
            Arc::new(move || {
                if let Ok(slot) = slot.lock() {
                    if let Some(proxy) = slot.as_ref() {
                        let _ = proxy.send_event(DesktopEvent::CollabWake);
                    }
                }
            }),
            self.generation,
        )
    }

    fn handle_event(&mut self, event: NetworkEvent, host: &mut WidgetHostNative) -> bool {
        let result = match event {
            NetworkEvent::OwnerReady {
                session_id,
                epoch,
                endpoint,
                share_endpoint,
                local_auth,
            } => self.owner_ready(
                session_id,
                epoch,
                endpoint,
                share_endpoint,
                local_auth,
                host,
            ),
            NetworkEvent::PeerAuthenticated {
                connection,
                auth,
                intent,
            } => self.peer_authenticated(connection, auth, intent, host),
            NetworkEvent::GuestAuthenticated {
                connection,
                session_id,
                epoch,
                remote_static,
            } => {
                self.pinned_owner_static = Some(remote_static);
                self.pending_guest = Some(PendingGuestAdmission {
                    connection,
                    session_id,
                    epoch,
                });
                host.editor_state_mut().editor_ui.collab.phase =
                    CollabConnectionPhase::Authenticating;
                Ok(())
            }
            NetworkEvent::Frame { connection, frame } => self.accept_frame(connection, frame, host),
            NetworkEvent::RenewalVerified { connection, auth } => {
                self.complete_renewal(connection, auth, host)
            }
            NetworkEvent::LocalTicketReady { ticket } => self.send_local_renewal(ticket),
            NetworkEvent::ConnectionClosed {
                connection,
                failure,
                remote_bye,
            } => self.connection_closed(connection, failure, remote_bye, host),
            NetworkEvent::Discovery { sessions } => {
                self.update_discovery(sessions, host);
                Ok(())
            }
            NetworkEvent::Failed(failure) => {
                self.fail_network(host, failure);
                Ok(())
            }
            NetworkEvent::Stopped => {
                self.network_stopped(host);
                Ok(())
            }
        };
        if let Err(error) = result {
            self.fail_network(host, error.failure);
        }
        host.mark_editor_state_dirty();
        true
    }

    fn owner_ready(
        &mut self,
        session_id: SessionId,
        epoch: Epoch,
        endpoint: SocketAddr,
        share_endpoint: Option<SocketAddr>,
        auth: op_collab::VerifiedAuthMetadata,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        let mut owner = OwnerActor::new(session_id, epoch, auth, host)?;
        owner.set_share_endpoint(share_endpoint);
        set_owner_ui(host, &owner);
        self.actor = Some(EditorActor::Owner(Box::new(owner)));
        self.push_status(CollabStatusEvent::OwnerStarted { endpoint });
        self.push_status(CollabStatusEvent::SessionActive { role: Role::Owner });
        Ok(())
    }

    fn accept_frame(
        &mut self,
        connection: ConnectionKey,
        frame: FrameEnvelope,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        if self.pending_guest.is_some() && matches!(frame.body(), CollabMessage::Welcome(_)) {
            return self.accept_guest_welcome(connection, frame, host);
        }
        let Some(mut actor) = self.actor.take() else {
            return Err(CollabRuntimeError::invalid_session());
        };
        let result = match &mut actor {
            EditorActor::Owner(owner) => {
                match owner.session.accept_frame(connection, frame, host) {
                    Ok(output) => self.route_owner_output(owner, output, host),
                    Err(_) => self.close_failed_owner_peer(owner, connection, host),
                }
            }
            EditorActor::Guest(guest) => guest
                .session
                .accept_frame(frame, host)
                .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))
                .and_then(|output| self.route_guest_output(guest, output, host)),
        };
        self.actor = Some(actor);
        result
    }

    fn accept_guest_welcome(
        &mut self,
        connection: ConnectionKey,
        frame: FrameEnvelope,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        let pending = self
            .pending_guest
            .take()
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        if pending.connection != connection
            || frame.session_id() != &pending.session_id
            || frame.epoch() != pending.epoch
        {
            return Err(CollabRuntimeError::invalid_session());
        }
        let CollabMessage::Welcome(welcome) = frame.into_body() else {
            return Err(CollabRuntimeError::invalid_session());
        };
        match self.actor.take() {
            Some(EditorActor::Guest(mut guest)) => {
                let replaced_session = guest.session.core().session_id() != &pending.session_id
                    || guest.session.core().epoch() != pending.epoch;
                let resumed = guest
                    .session
                    .resume(pending.session_id, pending.epoch, welcome, host)
                    .map_err(|_| CollabRuntimeError::invalid_session());
                if replaced_session {
                    debug_assert!(resumed.is_err());
                    guest.connection = None;
                    set_guest_ui(host, &guest, CollabConnectionPhase::Ended);
                    self.actor = Some(EditorActor::Guest(guest));
                    self.retire_workers();
                    self.transaction_active = false;
                    self.set_notice(host, CollabNoticeKind::EpochChanged);
                    self.push_status(CollabStatusEvent::SessionEnded);
                    return Ok(());
                }
                let result = match resumed {
                    Ok(output) => {
                        guest.connection = Some(connection);
                        self.route_guest_output(&mut guest, output, host)
                    }
                    Err(error) => Err(error),
                };
                // `GuestSessionCore::resume` deliberately enters Ended when
                // the authenticated owner presents a different session or
                // epoch. Restore the actor before propagating that error so
                // fail_network preserves the confirmed document and pending
                // edit for the terminal Save As fork flow.
                self.actor = Some(EditorActor::Guest(guest));
                result
            }
            Some(actor) => {
                self.actor = Some(actor);
                Err(CollabRuntimeError::invalid_session())
            }
            None => {
                let guest =
                    GuestActor::new(pending.session_id, pending.epoch, welcome, connection, host)?;
                self.actor = Some(EditorActor::Guest(Box::new(guest)));
                Ok(())
            }
        }
    }

    fn complete_renewal(
        &mut self,
        connection: ConnectionKey,
        auth: op_collab::VerifiedAuthMetadata,
        host: &mut WidgetHostNative,
    ) -> Result<(), CollabRuntimeError> {
        let Some(EditorActor::Owner(mut owner)) = self.actor.take() else {
            return Err(CollabRuntimeError::invalid_session());
        };
        let result = match owner.session.complete_renewal(connection, auth) {
            Ok(()) => Ok(()),
            Err(_) => self.close_failed_owner_peer(&mut owner, connection, host),
        };
        self.actor = Some(EditorActor::Owner(owner));
        result
    }

    fn push_status(&mut self, event: CollabStatusEvent) {
        if self.status.len() == MAX_STATUS_EVENTS {
            self.status.pop_front();
        }
        self.status.push_back(event);
    }

    fn advance_generation(&mut self) {
        self.generation = self.generation.checked_add(1).unwrap_or(1);
        let _ =
            op_editor_ui::collab_avatar_runtime::begin_collab_avatar_generation(self.generation);
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.clock_start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod poll_tests;
#[cfg(test)]
mod terminal_tests;
#[cfg(test)]
mod tests;

impl Default for DesktopCollabRuntime {
    fn default() -> Self {
        Self::new()
    }
}
