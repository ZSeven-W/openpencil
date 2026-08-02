use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError, TrySendError};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use op_collab::{ByeReason, ConnectionKey, Epoch, SessionId};
use op_collab_transport::{
    accept_secure_tcp_guarded, ConnectionLimiter, DeviceStaticKey, DiscoveryError,
    DiscoveryPublisher, JoinIntent, PeerIdentityPolicy, PendingHandshakeGuard, ServerPrelude,
    SharedQueueBudget, StaticKeyStore, TransportConfig,
};
use socket2::{Domain, Protocol, Socket, Type};

use super::super::auth::{
    production_verifier, unix_time_ms, LocalAdmission, LocalTicketRenewer, ProductionTicketVerifier,
};
use super::super::relay::{OwnerRelayRuntime, RelayOwnerRequest};
use super::super::types::{
    CollabRuntimeFailure, NetworkEvent, OwnerNetworkCommand, PeerNetworkCommand,
    TerminalNetworkEvent,
};
use super::connection::{drive_owner_peer, runtime_failure, DriverControl, DriverIdentity};
use super::owner_lifecycle::{PeerControl, PeerPhase, PeerRegistry};
use super::share_endpoint::select_share_endpoint;
use super::EventSink;

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const PEER_COMMAND_CAPACITY: usize = 64;
const PEER_TERMINAL_CAPACITY: usize = 1;
/// Human decision window after Noise and ticket verification. This is
/// deliberately independent from the short transport admission-exchange
/// timeout, while remaining bounded and holding a pending-handshake slot.
const OWNER_APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);

pub(super) struct OwnerTarget {
    pub(super) bind_address: SocketAddr,
    pub(super) session_id: SessionId,
    pub(super) epoch: Epoch,
    pub(super) relay: Option<RelayOwnerRequest>,
}

pub(super) fn run(
    sink: EventSink,
    key_store: Arc<dyn StaticKeyStore>,
    target: OwnerTarget,
    commands: Receiver<OwnerNetworkCommand>,
    shutdown: Receiver<ByeReason>,
) {
    let result = run_inner(&sink, key_store, target, commands, shutdown);
    if let Err(failure) = result {
        let _ = sink.send_terminal(TerminalNetworkEvent::Failed(failure));
    }
    let _ = sink.send_terminal(TerminalNetworkEvent::Stopped);
}

fn run_inner(
    sink: &EventSink,
    key_store: Arc<dyn StaticKeyStore>,
    target: OwnerTarget,
    commands: Receiver<OwnerNetworkCommand>,
    shutdown: Receiver<ByeReason>,
) -> Result<(), CollabRuntimeFailure> {
    let OwnerTarget {
        bind_address,
        session_id,
        epoch,
        relay,
    } = target;
    let config = TransportConfig::default();
    let key = Arc::new(
        key_store
            .load_or_generate()
            .map_err(|_| CollabRuntimeFailure::SecureKeyUnavailable)?,
    );
    let verifier = production_verifier().map_err(|error| error.failure)?;
    let initial_local =
        LocalAdmission::request_cancellable(key.public_key(), verifier.as_ref(), || {
            matches!(shutdown.try_recv(), Ok(_) | Err(TryRecvError::Disconnected))
        })
        .map_err(|error| error.failure)?;
    let local_auth = initial_local.auth().clone();
    let mut local_renewer =
        LocalTicketRenewer::new(*key.public_key(), Arc::clone(&verifier), local_auth.clone())
            .map_err(|error| error.failure)?;
    let local = Arc::new(RwLock::new(initial_local));
    let relay = match relay {
        Some(request) => Some(OwnerRelayRuntime::start(
            request,
            Arc::clone(&key),
            Arc::clone(&local),
            &session_id,
            epoch,
        )?),
        None => None,
    };
    let (listener, dual_stack) = bind_owner_listener(bind_address)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| CollabRuntimeFailure::Transport)?;
    let endpoint = listener
        .local_addr()
        .map_err(|_| CollabRuntimeFailure::Transport)?;
    let share_endpoint = select_share_endpoint(endpoint, dual_stack);
    let mut publisher = DiscoveryPublisher::start_for_listener(endpoint, dual_stack).ok();
    let discovery_id = match publisher.as_ref() {
        Some(publisher) => publisher.discovery_id().to_owned(),
        None => random_hex_identifier()?,
    };
    let mut prelude = Arc::new(
        ServerPrelude::new(discovery_id, session_id.clone(), epoch)
            .map_err(|_| CollabRuntimeFailure::Protocol)?,
    );
    let (invite, connection_path) = match relay.as_ref() {
        Some(relay) => (relay.invite(), relay.path()),
        None => (None, op_editor_core::CollabConnectionPathUi::Lan),
    };
    if !sink.send(NetworkEvent::OwnerReady {
        session_id: session_id.clone(),
        epoch,
        endpoint,
        share_endpoint,
        local_auth,
        invite,
        connection_path,
    }) {
        return Ok(());
    }

    // Built from this runtime's timeouts, not the defaults, so the silent-peer
    // reclaim window tracks whatever `config` actually configured.
    let limiter = ConnectionLimiter::with_timeouts(config.connections, config.timeouts)
        .map_err(|_| CollabRuntimeFailure::ResourceLimit)?;
    let shared_budget = SharedQueueBudget::new(config.connections.global_queued_bytes)
        .map_err(|_| CollabRuntimeFailure::ResourceLimit)?;
    let (done_sender, done_receiver) = mpsc::channel();
    let mut peers = PeerRegistry::new();
    let mut next_connection = Some(2_u64);

    loop {
        let now = Instant::now();
        match shutdown.try_recv() {
            Ok(reason) => {
                peers.set_exit_reason(reason);
                return Ok(());
            }
            Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }
        if let Some(active_publisher) = publisher.as_mut() {
            if now >= active_publisher.next_rotation_at() {
                match active_publisher.rotate_if_due(now) {
                    Ok(true) => {
                        // The advertised discovery id and the server prelude
                        // advance in the same owner-network turn, before any
                        // newly accepted socket snapshots the prelude.
                        prelude = Arc::new(
                            ServerPrelude::new(
                                active_publisher.discovery_id().to_owned(),
                                session_id.clone(),
                                epoch,
                            )
                            .map_err(|_| CollabRuntimeFailure::Protocol)?,
                        );
                    }
                    Ok(false) => {}
                    Err(error) => {
                        // A transient rotation error restores the old record
                        // and schedules a short retry. Retain that publisher;
                        // dropping it here would turn the recovery path into a
                        // permanent discovery outage.
                        if terminal_rotation_failure(&error, active_publisher.is_stopped()) {
                            publisher.take();
                        }
                    }
                }
            }
        }
        match local_renewer.poll(now) {
            Ok(Some(renewed)) => {
                let ticket = match renewed.renewal_ticket() {
                    Ok(ticket) => ticket,
                    Err(error) => {
                        peers.set_exit_reason(owner_local_terminal_reason(error.failure));
                        return Err(error.failure);
                    }
                };
                let Ok(mut local) = local.write() else {
                    let failure = CollabRuntimeFailure::AuthenticationUnavailable;
                    peers.set_exit_reason(owner_local_terminal_reason(failure));
                    return Err(failure);
                };
                *local = renewed;
                if !sink.send(NetworkEvent::LocalTicketReady { ticket }) {
                    return Ok(());
                }
            }
            Ok(None) => {}
            Err(error) => {
                peers.set_exit_reason(owner_local_terminal_reason(error.failure));
                return Err(error.failure);
            }
        }
        while let Ok(connection) = done_receiver.try_recv() {
            peers.reap(connection);
        }
        loop {
            match commands.try_recv() {
                Ok(command) => {
                    if !route_command(command, &mut peers)? {
                        return Ok(());
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }
        match accept_owner_stream(&listener, relay.as_ref(), &prelude) {
            Ok(Some(accepted)) => {
                let AcceptedOwnerStream {
                    stream,
                    address,
                    prelude: accepted_prelude,
                    source,
                } = accepted;
                let Some(raw_connection) = next_connection else {
                    continue;
                };
                next_connection = raw_connection.checked_add(1);
                let Some(connection_id) = ConnectionKey::new(raw_connection) else {
                    continue;
                };
                let pending = match limiter.try_begin_handshake(address.ip()) {
                    Ok(guard) => guard,
                    Err(_) => continue,
                };
                let cancel = stream
                    .try_clone()
                    .map_err(|_| CollabRuntimeFailure::Transport)?;
                let phase = Arc::new(AtomicU8::new(PeerPhase::Joining as u8));
                let (sender, receiver) = mpsc::sync_channel(PEER_COMMAND_CAPACITY);
                let (terminal, terminal_receiver) = mpsc::sync_channel(PEER_TERMINAL_CAPACITY);
                let thread = spawn_peer(PeerArgs {
                    sink: sink.clone(),
                    key: Arc::clone(&key),
                    local: Arc::clone(&local),
                    verifier: Arc::clone(&verifier),
                    prelude: accepted_prelude,
                    shared_budget: shared_budget.clone(),
                    config: source.transport_config(config),
                    connection_id,
                    session_id: session_id.clone(),
                    epoch,
                    stream,
                    pending,
                    commands: receiver,
                    shutdown: terminal_receiver,
                    phase: Arc::clone(&phase),
                    done: done_sender.clone(),
                })?;
                peers.insert(
                    connection_id,
                    PeerControl {
                        commands: sender,
                        shutdown: terminal,
                        cancel: Some(cancel),
                        phase,
                        thread: Some(thread),
                    },
                );
            }
            Ok(None) => {
                // The channel wait wakes immediately for Stop/send work while
                // bounding listener admission latency. This avoids the old
                // unconditional 200 Hz idle poll.
                match commands.recv_timeout(ACCEPT_POLL_INTERVAL) {
                    Ok(command) => {
                        if !route_command(command, &mut peers)? {
                            return Ok(());
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return Ok(()),
                }
            }
            Err(_) => return Err(CollabRuntimeFailure::Transport),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnerStreamSource {
    Lan,
    Relay,
}

impl OwnerStreamSource {
    fn transport_config(self, mut config: TransportConfig) -> TransportConfig {
        if self == Self::Relay {
            // The relay already authenticated and bounded the public socket.
            // Its local bridge may receive the initiator's first Noise frame
            // after a public-network round trip, so retain the complete Noise
            // handshake window instead of applying the direct-LAN DoS guard.
            config.timeouts.handshake_first_message = config.timeouts.handshake;
        }
        config
    }
}

struct AcceptedOwnerStream {
    stream: TcpStream,
    address: SocketAddr,
    prelude: Arc<ServerPrelude>,
    source: OwnerStreamSource,
}

fn accept_owner_stream(
    lan_listener: &TcpListener,
    relay: Option<&OwnerRelayRuntime>,
    lan_prelude: &Arc<ServerPrelude>,
) -> std::io::Result<Option<AcceptedOwnerStream>> {
    if let Some(relay) = relay {
        match relay.accept() {
            Ok((stream, address)) => {
                return Ok(Some(AcceptedOwnerStream {
                    stream,
                    address,
                    prelude: relay.prelude(),
                    source: OwnerStreamSource::Relay,
                }));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error),
        }
    }
    match lan_listener.accept() {
        Ok((stream, address)) => Ok(Some(AcceptedOwnerStream {
            stream,
            address,
            prelude: Arc::clone(lan_prelude),
            source: OwnerStreamSource::Lan,
        })),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error),
    }
}

fn route_command(
    command: OwnerNetworkCommand,
    peers: &mut PeerRegistry,
) -> Result<bool, CollabRuntimeFailure> {
    match command {
        OwnerNetworkCommand::Authorize { connection, role } => {
            send_peer(peers, connection, PeerNetworkCommand::Authorize(role))?;
        }
        OwnerNetworkCommand::Send {
            connection,
            frame,
            coalesce_key,
        } => {
            send_peer(
                peers,
                connection,
                PeerNetworkCommand::Send {
                    frame,
                    coalesce_key,
                },
            )?;
        }
        OwnerNetworkCommand::VerifyRenewal {
            connection,
            opaque_ticket,
        } => {
            send_peer(
                peers,
                connection,
                PeerNetworkCommand::VerifyRenewal(opaque_ticket),
            )?;
        }
        OwnerNetworkCommand::Close { connection } => {
            send_peer(peers, connection, PeerNetworkCommand::Stop)?;
        }
    }
    Ok(true)
}

fn send_peer(
    peers: &mut PeerRegistry,
    connection: ConnectionKey,
    command: PeerNetworkCommand,
) -> Result<(), CollabRuntimeFailure> {
    let Some(control) = peers.peers.get(&connection) else {
        return Ok(());
    };
    match control.commands.try_send(command) {
        Ok(()) => Ok(()),
        Err(TrySendError::Full(PeerNetworkCommand::Send { frame, .. }))
            if frame.is_lossy_presence() =>
        {
            Ok(())
        }
        Err(TrySendError::Full(_)) => Err(CollabRuntimeFailure::ResourceLimit),
        Err(TrySendError::Disconnected(_)) => {
            peers.reap(connection);
            Ok(())
        }
    }
}

struct PeerArgs {
    sink: EventSink,
    key: Arc<DeviceStaticKey>,
    local: Arc<RwLock<LocalAdmission>>,
    verifier: Arc<ProductionTicketVerifier>,
    prelude: Arc<ServerPrelude>,
    shared_budget: SharedQueueBudget,
    config: TransportConfig,
    connection_id: ConnectionKey,
    session_id: SessionId,
    epoch: Epoch,
    stream: TcpStream,
    pending: PendingHandshakeGuard,
    commands: Receiver<PeerNetworkCommand>,
    shutdown: Receiver<ByeReason>,
    phase: Arc<AtomicU8>,
    done: Sender<ConnectionKey>,
}

fn spawn_peer(args: PeerArgs) -> Result<JoinHandle<()>, CollabRuntimeFailure> {
    std::thread::Builder::new()
        .name(format!("op-collab-peer-{}", args.connection_id.get()))
        .spawn(move || run_peer(args))
        .map_err(|_| CollabRuntimeFailure::ResourceLimit)
}

fn run_peer(args: PeerArgs) {
    let sink = args.sink.clone();
    let connection_id = args.connection_id;
    let done = args.done.clone();
    let phase = Arc::clone(&args.phase);
    let failure = run_peer_inner(args);
    phase.store(PeerPhase::Done as u8, Ordering::Release);
    let _ = sink.send_terminal(TerminalNetworkEvent::ConnectionClosed {
        connection: connection_id,
        failure,
        remote_bye: None,
    });
    let _ = done.send(connection_id);
}

fn run_peer_inner(args: PeerArgs) -> Option<CollabRuntimeFailure> {
    let PeerArgs {
        sink,
        key,
        local,
        verifier,
        prelude,
        shared_budget,
        config,
        connection_id,
        session_id,
        epoch,
        stream,
        pending,
        commands,
        shutdown,
        phase,
        done: _,
    } = args;
    // Guarded accept enforces the first-message socket deadline. A silent peer
    // exits this worker before dropping its continuously charged pending seat,
    // so live sockets/threads can never outnumber the global guard ceiling.
    let mut connection = match accept_secure_tcp_guarded(stream, &key, &prelude, config, &pending) {
        Ok(connection) => connection,
        Err(error) => return Some(runtime_failure(&error)),
    };
    let (hello, expected_issuer) = {
        let local = match local.read() {
            Ok(local) => local,
            Err(_) => return Some(CollabRuntimeFailure::AuthenticationUnavailable),
        };
        let hello = match local.hello(JoinIntent::New) {
            Ok(hello) => hello,
            Err(error) => return Some(error.failure),
        };
        (hello, local.expected_issuer().to_owned())
    };
    let now_unix_ms = match unix_time_ms() {
        Ok(now) => now,
        Err(error) => return Some(error.failure),
    };
    // Cross-account collaboration: a guest from any account the trusted issuer
    // vouches for may reach this point. Nothing here decides whether it joins —
    // the owner does, from the approval prompt below, which is shown the
    // verified identity and which the admission state machine makes
    // unskippable (`Active` is reachable only through `OwnerAuthorized`).
    let (remote, identity) = match connection.exchange_admission_responder(
        &hello,
        verifier.as_ref(),
        &expected_issuer,
        PeerIdentityPolicy::AnyIssuedAccount,
        now_unix_ms,
        Instant::now(),
    ) {
        Ok(admission) => admission,
        Err(error) => return Some(runtime_failure(&error)),
    };
    phase.store(PeerPhase::Approval as u8, Ordering::Release);
    if !sink.send(NetworkEvent::PeerAuthenticated {
        connection: connection_id,
        auth: identity.to_auth_metadata(),
        intent: remote.intent().clone(),
    }) {
        return None;
    }
    let role = match await_owner_approval(&commands, &shutdown) {
        Ok(Some(role)) => role,
        Ok(None) => return None,
        Err(failure) => return Some(failure),
    };
    if let Err(error) = connection.authorize_remote(role) {
        return Some(runtime_failure(&error));
    }
    if let Err(error) = connection.activate(Instant::now()) {
        return Some(runtime_failure(&error));
    }
    let _active = match pending.activate() {
        Ok(active) => active,
        Err(_) => return Some(CollabRuntimeFailure::ResourceLimit),
    };
    phase.store(PeerPhase::Active as u8, Ordering::Release);
    drive_owner_peer(
        connection,
        shared_budget,
        DriverIdentity {
            connection: connection_id,
            session_id,
            epoch,
        },
        DriverControl { commands, shutdown },
        verifier,
        &sink,
    )
}

fn await_owner_approval(
    commands: &Receiver<PeerNetworkCommand>,
    shutdown: &Receiver<ByeReason>,
) -> Result<Option<op_collab::Role>, CollabRuntimeFailure> {
    let deadline = Instant::now()
        .checked_add(OWNER_APPROVAL_TIMEOUT)
        .ok_or(CollabRuntimeFailure::Transport)?;
    loop {
        match shutdown.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => return Ok(None),
            Err(TryRecvError::Empty) => {}
        }
        let now = Instant::now();
        if now >= deadline {
            return Ok(None);
        }
        let wait = deadline
            .saturating_duration_since(now)
            .min(ACCEPT_POLL_INTERVAL);
        match commands.recv_timeout(wait) {
            Ok(PeerNetworkCommand::Authorize(role)) => return Ok(Some(role)),
            Ok(PeerNetworkCommand::Stop) => return Ok(None),
            Ok(PeerNetworkCommand::Send { .. } | PeerNetworkCommand::VerifyRenewal(_)) => {
                return Err(CollabRuntimeFailure::Protocol);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(None),
        }
    }
}

const fn owner_local_terminal_reason(_failure: CollabRuntimeFailure) -> ByeReason {
    ByeReason::AuthenticationExpired
}

fn random_hex_identifier() -> Result<String, CollabRuntimeFailure> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| CollabRuntimeFailure::SecureKeyUnavailable)?;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    Ok(encoded)
}

fn bind_owner_listener(requested: SocketAddr) -> Result<(TcpListener, bool), CollabRuntimeFailure> {
    if requested.is_ipv6() && requested.ip().is_unspecified() {
        if let Ok(listener) = bind_dual_stack_listener(requested) {
            return Ok((listener, true));
        }
        let fallback = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), requested.port());
        return TcpListener::bind(fallback)
            .map(|listener| (listener, false))
            .map_err(|_| CollabRuntimeFailure::Transport);
    }
    TcpListener::bind(requested)
        .map(|listener| (listener, false))
        .map_err(|_| CollabRuntimeFailure::Transport)
}

fn bind_dual_stack_listener(requested: SocketAddr) -> std::io::Result<TcpListener> {
    let socket = Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_only_v6(false)?;
    socket.bind(&requested.into())?;
    socket.listen(128)?;
    Ok(socket.into())
}

fn terminal_rotation_failure(error: &DiscoveryError, publisher_stopped: bool) -> bool {
    publisher_stopped || matches!(error, DiscoveryError::PublisherStopped)
}

#[cfg(test)]
mod approval_timeout_tests {
    use op_collab::ByeReason;

    use super::*;

    #[test]
    fn owner_approval_window_is_human_sized_but_bounded() {
        assert!(OWNER_APPROVAL_TIMEOUT >= Duration::from_secs(60));
        assert!(OWNER_APPROVAL_TIMEOUT <= Duration::from_secs(5 * 60));
        assert!(
            OWNER_APPROVAL_TIMEOUT > TransportConfig::default().timeouts.admission,
            "human approval must not weaken the Noise/ticket exchange deadline"
        );
    }

    #[test]
    fn owner_idle_wait_keeps_stop_latency_bounded() {
        assert!(ACCEPT_POLL_INTERVAL >= Duration::from_millis(10));
        assert!(ACCEPT_POLL_INTERVAL <= Duration::from_millis(50));
    }

    #[test]
    fn approval_shutdown_wins_over_a_queued_authorize_command() {
        let (commands, command_receiver) = mpsc::sync_channel(1);
        let (shutdown, shutdown_receiver) = mpsc::sync_channel(1);
        commands
            .send(PeerNetworkCommand::Authorize(op_collab::Role::Editor))
            .unwrap();
        shutdown.send(ByeReason::OwnerLeft).unwrap();

        assert_eq!(
            await_owner_approval(&command_receiver, &shutdown_receiver).unwrap(),
            None
        );
        assert!(matches!(
            command_receiver.try_recv(),
            Ok(PeerNetworkCommand::Authorize(op_collab::Role::Editor))
        ));
    }

    #[test]
    fn owner_local_auth_failure_uses_authentication_expired_terminal_reason() {
        assert_eq!(
            owner_local_terminal_reason(CollabRuntimeFailure::TicketRejected),
            ByeReason::AuthenticationExpired
        );
    }

    #[test]
    fn transient_discovery_rotation_failure_keeps_retryable_publisher() {
        assert!(!terminal_rotation_failure(
            &DiscoveryError::UnregisterFailed,
            false
        ));
        assert!(terminal_rotation_failure(
            &DiscoveryError::PublisherStopped,
            true
        ));
    }

    #[test]
    fn owner_listener_prefers_dual_stack_and_has_ipv4_fallback() {
        let requested = SocketAddr::new(IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED), 0);
        let (listener, dual_stack) = bind_owner_listener(requested).expect("owner listener");
        let port = listener.local_addr().unwrap().port();

        if dual_stack {
            TcpStream::connect(SocketAddr::new(
                IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
                port,
            ))
            .expect("IPv6 loopback reaches dual-stack owner");
            listener.accept().expect("accept IPv6 loopback");
        }
        TcpStream::connect(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
            .expect("IPv4 loopback reaches owner listener");
        listener.accept().expect("accept IPv4 loopback");
    }

    #[test]
    fn explicit_ipv4_listener_reports_single_stack() {
        let requested = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let (listener, dual_stack) = bind_owner_listener(requested).expect("IPv4 listener");
        assert!(!dual_stack);
        let endpoint = listener.local_addr().unwrap();
        TcpStream::connect(endpoint).expect("IPv4 loopback reaches fallback policy");
        listener.accept().expect("accept IPv4 loopback");
    }
}

#[cfg(test)]
#[path = "owner_queue_tests.rs"]
mod queue_tests;

#[cfg(test)]
#[path = "owner_relay_timeout_tests.rs"]
mod relay_timeout_tests;
