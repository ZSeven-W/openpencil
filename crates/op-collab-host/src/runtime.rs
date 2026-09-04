//! GUI-owned collaboration actors bridged to network workers by bounded channels.
mod actor;
mod admission;
mod auth;
mod discarded_edit;
mod effects;
mod effects_wire;
mod failure;
mod guest_confirmation;
mod guest_reconnect;
mod guest_routes;
pub(crate) mod local_edit;
mod network;
mod poll;
mod region_pref;
pub(crate) mod relay;
mod relay_bootstrap;
mod support;
// Fixtures for downstream test suites; see `test_support` for why this is not
// a production surface. Lives inside `runtime` because standing up an
// activated owner session needs this module's private actor + channel
// internals.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub(crate) mod types;

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
use guest_confirmation::PendingOwnerConfirmation;
use network::{
    DiscoveryNetwork, EventSink, PendingNetworkLaunch, Retirement, SessionNetwork,
    TerminalEventLane, TerminalEventReceiver,
};
use op_collab::{
    CollabMessage, ConnectionKey, Epoch, FrameEnvelope, OpaqueTicket, Role, SessionId,
};
use op_collab_transport::{JoinIntent, SharedQueueBudget, StaticKeyStore, TransportConfig};
use op_editor_core::{
    CollabAvailability, CollabConnectionPhase, CollabNoticeKind, CollabPendingEditUi,
    CollabRejectUiCode, CollabTransportCapabilities, CollabUiAction,
};
use support::{production_key_store, random_epoch};

use crate::host::{CollabHost, CollabWakeNotifier};

use relay::{
    owner_request, EnvironmentRelayLocatorControlPlane, GuestConnectionRoute,
    RelayLocatorControlPlane,
};
use types::{
    CollabRuntimeError, CollabRuntimeFailure, CollabStatusEvent, DiscoveredEndpoint, NetworkEvent,
    TaggedNetworkEvent,
};

const MAX_STATUS_EVENTS: usize = 64;

/// Owner / guest collaboration runtime.
///
/// One instance per editor document. The embedding host constructs it, hands
/// it a wake notifier, and drives it from its own loop:
/// `refresh_availability` → `drain_ui_action` → `poll`, plus the local-edit
/// capture pair around every mutating gesture.
pub struct CollabRuntime {
    /// Result recorded by effect routing and consumed by `finish_local_edit`.
    pub(crate) last_local_edit: Option<crate::runtime::local_edit::LocalEditOutcome>,
    events: Receiver<TaggedNetworkEvent>,
    event_sender: SyncSender<TaggedNetworkEvent>,
    terminal_event_sender: TerminalEventLane,
    terminal_events: TerminalEventReceiver,
    wake_notifier: Arc<Mutex<Option<CollabWakeNotifier>>>,
    network: Option<SessionNetwork>,
    discovery: Option<DiscoveryNetwork>,
    retirement: Option<Retirement>,
    pending_network_launch: Option<PendingNetworkLaunch>,
    actor: Option<EditorActor>,
    pending_guest: Option<PendingGuestAdmission>,
    pending_owner: PendingOwnerAdmissions,
    pending_owner_confirmation: Option<PendingOwnerConfirmation>,
    discovered: HashMap<String, DiscoveredEndpoint>,
    last_join: Option<GuestConnectionRoute>,
    pinned_owner_static: Option<[u8; 32]>,
    guest_reconnect: guest_reconnect::GuestReconnectState,
    transaction_active: bool,
    save_as_fork_requested: bool,
    /// Runtime-owned service-region preference (lazily loaded from disk);
    /// the panel field is just its per-frame projection.
    relay_region_pref: Option<op_editor_core::CollabRelayRegion>,
    /// Property changes of the most recent conflict-discarded local edit,
    /// kept for a user-driven replay via `CollabUiAction::ReapplyDiscarded`.
    discarded_property_edit: Option<Vec<op_collab::NodeFieldChange>>,
    status: VecDeque<CollabStatusEvent>,
    generation: u64,
    clock_start: Instant,
    last_presence_sent: Option<Instant>,
    last_local_presence: Option<op_collab::Presence>,
    pending_local_presence: Option<op_collab::Presence>,
    latest_owner_ticket: Option<OpaqueTicket>,
    key_store: Arc<dyn StaticKeyStore>,
    bridge_budget: SharedQueueBudget,
    relay_locator_control_plane: Arc<dyn RelayLocatorControlPlane>,
    transport_capabilities: CollabTransportCapabilities,
}

struct OwnerReadyState {
    session_id: SessionId,
    epoch: Epoch,
    endpoint: SocketAddr,
    share_endpoint: Option<SocketAddr>,
    auth: op_collab::VerifiedAuthMetadata,
    invite: Option<op_editor_core::CollabInviteCode>,
    connection_path: op_editor_core::CollabConnectionPathUi,
}

impl CollabRuntime {
    pub fn new() -> Self {
        Self::with_relay_locator_control_plane(Arc::new(EnvironmentRelayLocatorControlPlane))
    }

    /// Inject an authenticated control-plane/HSM locator issuer; the default fails closed.
    pub(crate) fn with_relay_locator_control_plane(
        control_plane: Arc<dyn RelayLocatorControlPlane>,
    ) -> Self {
        let mut runtime = Self::with_key_store(production_key_store());
        runtime.relay_locator_control_plane = control_plane;
        runtime
    }

    pub fn with_key_store(key_store: Arc<dyn StaticKeyStore>) -> Self {
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
            last_local_edit: None,
            events,
            event_sender,
            terminal_event_sender,
            terminal_events,
            wake_notifier: Arc::new(Mutex::new(None)),
            network: None,
            discovery: None,
            retirement: None,
            pending_network_launch: None,
            actor: None,
            pending_guest: None,
            pending_owner: PendingOwnerAdmissions::default(),
            pending_owner_confirmation: None,
            discovered: HashMap::new(),
            last_join: None,
            pinned_owner_static: None,
            guest_reconnect: guest_reconnect::GuestReconnectState::default(),
            transaction_active: false,
            save_as_fork_requested: false,
            relay_region_pref: None,
            discarded_property_edit: None,
            status: VecDeque::new(),
            generation: 1,
            clock_start: Instant::now(),
            last_presence_sent: None,
            last_local_presence: None,
            pending_local_presence: None,
            latest_owner_ticket: None,
            key_store,
            bridge_budget,
            relay_locator_control_plane: Arc::new(EnvironmentRelayLocatorControlPlane),
            transport_capabilities: CollabTransportCapabilities::default(),
        }
    }

    /// Restrict transport paths exposed to and accepted from shared chrome.
    pub fn set_transport_capabilities(&mut self, capabilities: CollabTransportCapabilities) {
        self.transport_capabilities = capabilities;
    }

    pub fn set_wake_notifier(&mut self, notifier: CollabWakeNotifier) {
        if let Ok(mut slot) = self.wake_notifier.lock() {
            *slot = Some(notifier);
        }
    }

    // `sync_relay_region` / `preferred_region` / `set_relay_region` live in
    // `region_pref` alongside the persistence they wrap.

    pub fn refresh_availability(&mut self, host: &mut impl CollabHost) -> bool {
        self.sync_relay_region(host);
        let next = if !op_auth_bridge::collab_ticket_available() {
            CollabAvailability::Unavailable
        } else if host.editor_state().editor_ui.account.is_signed_in() {
            CollabAvailability::Ready
        } else {
            CollabAvailability::SignInRequired
        };
        let collab = &mut host.editor_state_mut().editor_ui.collab;
        let mut changed = false;
        if collab.transport_capabilities != self.transport_capabilities {
            collab.transport_capabilities = self.transport_capabilities;
            collab.panel.hover = None;
            changed = true;
        }
        if collab.availability != next {
            collab.set_availability(next);
            changed = true;
        }
        if changed {
            host.mark_editor_state_dirty();
        }
        changed
    }

    pub fn drain_ui_action(&mut self, host: &mut impl CollabHost) -> bool {
        let Some(action) = host
            .editor_state_mut()
            .editor_ui
            .collab
            .take_pending_action()
        else {
            return false;
        };
        if !self.transport_capabilities.supports(&action) {
            self.set_notice(
                host,
                CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
            );
            host.mark_editor_state_dirty();
            return true;
        }
        let result = match action {
            CollabUiAction::OpenCreate => Ok(()),
            CollabUiAction::Start => self.start_owner(host, true),
            CollabUiAction::StartLan => self.start_owner(host, false),
            CollabUiAction::SetRelayRegion { region } => {
                self.set_relay_region(host, region);
                Ok(())
            }
            CollabUiAction::OpenJoin => Ok(()),
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
            CollabUiAction::ReapplyDiscarded => {
                self.reapply_discarded(host);
                Ok(())
            }
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
            action @ (CollabUiAction::ConfirmOwnerIdentity { .. }
            | CollabUiAction::RejectOwnerIdentity { .. }) => {
                self.resolve_owner_confirmation(&action, host)
            }
        };
        if let Err(error) = result {
            self.fail(host, error.failure);
        }
        host.mark_editor_state_dirty();
        true
    }

    /// Owner-assigned id namespace for this peer, once a session assigned one.
    ///
    /// A remote client that creates nodes has to mint their ids from this, or
    /// two peers minting from private counters eventually agree on an id for
    /// two different nodes. The daemon publishes it in the collaboration
    /// projection so the browser can allocate correctly.
    pub fn peer_namespace(&self) -> Option<String> {
        let actor = self.actor.as_ref()?;
        Some(match actor {
            EditorActor::Owner(owner) => owner.peer_namespace().as_str().to_owned(),
            EditorActor::Guest(guest) => guest.session.core().peer_namespace().as_str().to_owned(),
        })
    }

    pub fn take_save_as_fork_request(&mut self) -> bool {
        std::mem::take(&mut self.save_as_fork_requested)
    }

    /// Route Cmd/Ctrl+Z to M1 selective undo; `false` preserves standalone history.
    pub fn request_undo(&mut self, host: &mut impl CollabHost) -> bool {
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
    pub fn reject_redo(&self, host: &mut impl CollabHost) -> bool {
        if self.actor.is_none() {
            return false;
        }
        self.set_notice(
            host,
            CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
        );
        true
    }

    pub fn leave(&mut self, host: &mut impl CollabHost) {
        self.retire_workers();
        if let Some(EditorActor::Guest(guest)) = self.actor.as_mut() {
            let _ = guest.session.disconnect(host);
        }
        self.actor = None;
        self.pending_guest = None;
        self.pending_owner.clear();
        self.clear_owner_confirmation(host);
        self.discovered.clear();
        self.last_join = None;
        self.pinned_owner_static = None;
        self.reset_guest_reconnect();
        self.transaction_active = false;
        self.clear_discarded_stash(host);
        self.last_presence_sent = None;
        self.last_local_presence = None;
        self.pending_local_presence = None;
        self.latest_owner_ticket = None;
        host.disable_collaboration_ids();
        let collab = &mut host.editor_state_mut().editor_ui.collab;
        collab.set_phase(CollabConnectionPhase::Idle);
        collab.pending_edit = CollabPendingEditUi::None;
        collab.clear_authenticated();
        collab.panel.discovered = Arc::new(Vec::new());
        host.mark_editor_state_dirty();
        self.push_status(CollabStatusEvent::SessionEnded);
    }

    pub fn drain_status_events(&mut self) -> impl Iterator<Item = CollabStatusEvent> + '_ {
        self.status.drain(..)
    }

    fn start_owner(
        &mut self,
        host: &mut impl CollabHost,
        use_public_relay: bool,
    ) -> Result<(), CollabRuntimeError> {
        self.require_ready(host)?;
        let relay = if use_public_relay {
            Some(owner_request(
                Arc::clone(&self.relay_locator_control_plane),
                self.preferred_region(),
            )?)
        } else {
            None
        };
        self.leave(host);
        let session_id = SessionId::from(random_identifier("session")?);
        let epoch = random_epoch()?;
        host.editor_state_mut()
            .editor_ui
            .collab
            .set_phase(CollabConnectionPhase::Starting);
        self.defer_owner_launch(session_id, epoch, relay);
        Ok(())
    }

    fn begin_discovery(&mut self, host: &mut impl CollabHost) -> Result<(), CollabRuntimeError> {
        self.require_ready(host)?;
        if self.actor.is_some() {
            return Err(CollabRuntimeError::invalid_session());
        }
        self.retire_workers();
        self.discovered.clear();
        host.editor_state_mut()
            .editor_ui
            .collab
            .set_phase(CollabConnectionPhase::Discovering);
        self.defer_discovery_launch();
        Ok(())
    }

    // `join_discovered` / `join_address` / `retry_guest` live in
    // `guest_routes` (split at the 800-line cap).

    fn start_guest_route(
        &mut self,
        host: &mut impl CollabHost,
        route: GuestConnectionRoute,
        intent: JoinIntent,
    ) -> Result<(), CollabRuntimeError> {
        self.require_ready(host)?;
        self.leave(host);
        self.spawn_guest_route(host, route, intent)
    }

    fn spawn_guest_route(
        &mut self,
        host: &mut impl CollabHost,
        route: GuestConnectionRoute,
        intent: JoinIntent,
    ) -> Result<(), CollabRuntimeError> {
        let endpoint = route.status_endpoint();
        let path = route.connection_path();
        self.require_ready(host)?;
        self.retire_workers();
        self.pending_guest = None;
        host.editor_state_mut()
            .editor_ui
            .collab
            .set_phase(CollabConnectionPhase::Joining);
        self.last_join = Some(route.clone());
        self.defer_guest_launch(route, intent);
        match (endpoint, path) {
            (Some(endpoint), _) => {
                self.push_status(CollabStatusEvent::JoinStarted { endpoint });
            }
            (None, op_editor_core::CollabConnectionPathUi::Relay { home_region }) => {
                self.push_status(CollabStatusEvent::RelayJoinStarted { home_region })
            }
            (None, op_editor_core::CollabConnectionPathUi::Lan) => {
                return Err(CollabRuntimeError::invalid_session());
            }
        }
        Ok(())
    }

    fn event_sink(&self) -> EventSink {
        let slot = Arc::clone(&self.wake_notifier);
        EventSink::new(
            self.event_sender.clone(),
            self.terminal_event_sender.clone(),
            self.bridge_budget.clone(),
            Arc::new(move || {
                if let Ok(slot) = slot.lock() {
                    if let Some(notify) = slot.as_ref() {
                        notify();
                    }
                }
            }),
            self.generation,
        )
    }

    fn handle_event(&mut self, event: NetworkEvent, host: &mut impl CollabHost) -> bool {
        let result = match event {
            NetworkEvent::OwnerReady {
                session_id,
                epoch,
                endpoint,
                share_endpoint,
                local_auth,
                invite,
                connection_path,
            } => self.owner_ready(
                OwnerReadyState {
                    session_id,
                    epoch,
                    endpoint,
                    share_endpoint,
                    auth: local_auth,
                    invite,
                    connection_path,
                },
                host,
            ),
            NetworkEvent::PeerAuthenticated {
                connection,
                auth,
                intent,
            } => self.peer_authenticated(connection, auth, intent, host),
            NetworkEvent::OwnerIdentityUnconfirmed { connection, auth } => {
                self.owner_identity_unconfirmed(connection, auth, host)
            }
            NetworkEvent::GuestAuthenticated {
                connection,
                session_id,
                epoch,
                remote_static,
            } => {
                self.clear_owner_confirmation(host);
                self.pinned_owner_static = Some(remote_static);
                self.pending_guest = Some(PendingGuestAdmission {
                    connection,
                    session_id,
                    epoch,
                });
                host.editor_state_mut()
                    .editor_ui
                    .collab
                    .set_phase(CollabConnectionPhase::Authenticating);
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
        ready: OwnerReadyState,
        host: &mut impl CollabHost,
    ) -> Result<(), CollabRuntimeError> {
        let mut owner = OwnerActor::new(ready.session_id, ready.epoch, ready.auth, host)?;
        owner.set_share_endpoint(ready.share_endpoint);
        set_owner_ui(host, &owner);
        // A relay session whose pairing-code publish failed has no shareable
        // code; surface that instead of showing a silent, uninvitable
        // session. The session itself stays up (LAN share still works).
        if ready.invite.is_none()
            && matches!(
                ready.connection_path,
                op_editor_core::CollabConnectionPathUi::Relay { .. }
            )
        {
            self.set_notice(
                host,
                CollabNoticeKind::Connect(op_editor_core::CollabConnectErrorUi::RelayUnavailable),
            );
        }
        host.editor_state_mut()
            .editor_ui
            .collab
            .set_public_session(ready.invite, ready.connection_path);
        self.actor = Some(EditorActor::Owner(Box::new(owner)));
        self.push_status(CollabStatusEvent::OwnerStarted {
            endpoint: ready.endpoint,
        });
        self.push_status(CollabStatusEvent::SessionActive { role: Role::Owner });
        Ok(())
    }

    fn accept_frame(
        &mut self,
        connection: ConnectionKey,
        frame: FrameEnvelope,
        host: &mut impl CollabHost,
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
        host: &mut impl CollabHost,
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
        let result = match self.actor.take() {
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
                    self.reset_guest_reconnect();
                    self.clear_discarded_stash(host);
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
        };
        if result.is_ok() {
            self.publish_guest_connection_path(host);
        }
        result
    }

    pub(super) fn publish_guest_connection_path(&self, host: &mut impl CollabHost) {
        let Some(route) = self.last_join.as_ref() else {
            return;
        };
        if host
            .editor_state()
            .editor_ui
            .collab
            .authenticated_session()
            .is_none()
        {
            return;
        }
        host.editor_state_mut()
            .editor_ui
            .collab
            .set_public_session(None, route.connection_path());
    }

    fn complete_renewal(
        &mut self,
        connection: ConnectionKey,
        auth: op_collab::VerifiedAuthMetadata,
        host: &mut impl CollabHost,
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
        // Rotating the epoch evicts every earlier participant's cached avatar
        // bytes, and that cache is process-global. In a test binary it is
        // shared with the decode tests, whose queued avatar id goes byte-less
        // the instant this runs — so the rotation takes the same guard they
        // do. Guarding here rather than at each collab test is the point:
        // this is production code, so the test call sites cannot cover it.
        #[cfg(test)]
        let _avatar_guard = crate::lock_avatar_test_registry();
        self.generation = self.generation.checked_add(1).unwrap_or(1);
        let _ =
            op_editor_ui::collab_avatar_runtime::begin_collab_avatar_generation(self.generation);
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.clock_start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod conflict_tests;
#[cfg(test)]
mod poll_tests;
#[cfg(test)]
mod terminal_tests;
#[cfg(test)]
mod tests;

impl Default for CollabRuntime {
    fn default() -> Self {
        Self::new()
    }
}
