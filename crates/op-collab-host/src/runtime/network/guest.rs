use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use op_collab::{ByeReason, ConnectionKey, Role};
use op_collab_transport::{
    connect_secure_tcp_until_cancellable, AdmissionError, DeviceStaticKey, EncodedServerPrelude,
    JoinIntent, RuntimeError, SecureConnection, SharedQueueBudget, StaticKeyStore, TransportConfig,
    MAX_DISCOVERY_ADDRESSES,
};
use subtle::ConstantTimeEq;

use super::super::auth::{production_verifier, unix_time_ms, LocalAdmission, LocalTicketRenewer};
use super::super::relay::{relay_guest_target, GuestConnectionRoute, GuestRelayRuntime};
use super::super::types::{
    CollabRuntimeFailure, GuestNetworkCommand, NetworkEvent, TerminalNetworkEvent,
};
use super::connection::{
    drive_guest, runtime_failure, DriverControl, DriverIdentity, GuestRenewalContext,
};
use super::guest_confirmation::{await_owner_confirmation, GuestConfirmationOutcome};
use super::guest_identity::{guest_admission_plan, GuestOwnerConfirmation};
use super::transport_diagnostic::report_relay_secure_transport_failure;
use super::EventSink;

pub(super) struct GuestTarget {
    pub(super) route: GuestConnectionRoute,
    pub(super) intent: JoinIntent,
}

const SHUTDOWN_WATCH_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum GuestNetworkPhase {
    Joining = 0,
    Active = 1,
    Done = 2,
}

struct GuestShutdownController {
    requested: Arc<AtomicBool>,
    phase: Arc<AtomicU8>,
    socket: Arc<Mutex<Option<TcpStream>>>,
    done: Arc<AtomicBool>,
    watcher: Option<JoinHandle<()>>,
}

impl GuestShutdownController {
    fn start(
        external: Receiver<ByeReason>,
    ) -> Result<(Self, Receiver<ByeReason>), CollabRuntimeFailure> {
        let requested = Arc::new(AtomicBool::new(false));
        let phase = Arc::new(AtomicU8::new(GuestNetworkPhase::Joining as u8));
        let socket = Arc::new(Mutex::new(None::<TcpStream>));
        let done = Arc::new(AtomicBool::new(false));
        let (forward, forwarded) = mpsc::sync_channel(1);
        let watcher_requested = Arc::clone(&requested);
        let watcher_phase = Arc::clone(&phase);
        let watcher_socket = Arc::clone(&socket);
        let watcher_done = Arc::clone(&done);
        let watcher = std::thread::Builder::new()
            .name("op-collab-guest-shutdown".to_string())
            .spawn(move || loop {
                if watcher_done.load(Ordering::Acquire) {
                    return;
                }
                match external.recv_timeout(SHUTDOWN_WATCH_INTERVAL) {
                    Ok(reason) => {
                        request_guest_shutdown(&watcher_requested, &watcher_phase, &watcher_socket);
                        let _ = forward.try_send(reason);
                        return;
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        request_guest_shutdown(&watcher_requested, &watcher_phase, &watcher_socket);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                }
            })
            .map_err(|_| CollabRuntimeFailure::ResourceLimit)?;
        Ok((
            Self {
                requested,
                phase,
                socket,
                done,
                watcher: Some(watcher),
            },
            forwarded,
        ))
    }

    fn requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }

    fn register_socket(&self, stream: &TcpStream) -> std::io::Result<()> {
        if self.requested() {
            return Err(cancelled_io_error());
        }
        let clone = stream.try_clone()?;
        self.socket
            .lock()
            .map_err(|_| lifecycle_lock_error())?
            .replace(clone);
        if self.requested() {
            self.cancel_join_socket();
            return Err(cancelled_io_error());
        }
        Ok(())
    }

    fn activate(&self) -> bool {
        if self.requested() {
            return false;
        }
        self.phase
            .store(GuestNetworkPhase::Active as u8, Ordering::Release);
        if let Ok(mut socket) = self.socket.lock() {
            socket.take();
        }
        !self.requested()
    }

    fn cancel_join_socket(&self) {
        if let Ok(socket) = self.socket.lock() {
            if let Some(socket) = socket.as_ref() {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
    }

    fn finish(mut self) {
        self.phase
            .store(GuestNetworkPhase::Done as u8, Ordering::Release);
        self.done.store(true, Ordering::Release);
        if let Ok(mut socket) = self.socket.lock() {
            socket.take();
        }
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }
    }
}

fn request_guest_shutdown(
    requested: &AtomicBool,
    phase: &AtomicU8,
    socket: &Mutex<Option<TcpStream>>,
) {
    requested.store(true, Ordering::Release);
    if phase.load(Ordering::Acquire) == GuestNetworkPhase::Joining as u8 {
        if let Ok(socket) = socket.lock() {
            if let Some(socket) = socket.as_ref() {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
    }
}

fn cancelled_io_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Interrupted,
        "guest collaboration launch cancelled",
    )
}

fn lifecycle_lock_error() -> std::io::Error {
    std::io::Error::other("guest collaboration lifecycle lock poisoned")
}

pub(super) fn run(
    sink: EventSink,
    key_store: Arc<dyn StaticKeyStore>,
    target: GuestTarget,
    commands: Receiver<GuestNetworkCommand>,
    shutdown: Receiver<ByeReason>,
) {
    let (controller, forwarded_shutdown) = match GuestShutdownController::start(shutdown) {
        Ok(controller) => controller,
        Err(failure) => {
            let _ = sink.send_terminal(TerminalNetworkEvent::Failed(failure));
            let _ = sink.send_terminal(TerminalNetworkEvent::Stopped);
            return;
        }
    };
    let result = run_inner(
        &sink,
        key_store,
        target,
        commands,
        forwarded_shutdown,
        &controller,
    );
    let cancelled = controller.requested();
    controller.finish();
    if !cancelled {
        if let Err(failure) = result {
            let _ = sink.send_terminal(TerminalNetworkEvent::Failed(failure));
        }
    }
    let _ = sink.send_terminal(TerminalNetworkEvent::Stopped);
}

fn run_inner(
    sink: &EventSink,
    key_store: Arc<dyn StaticKeyStore>,
    target: GuestTarget,
    commands: Receiver<GuestNetworkCommand>,
    shutdown: Receiver<ByeReason>,
    cancellation: &GuestShutdownController,
) -> Result<(), CollabRuntimeFailure> {
    let GuestTarget { route, intent } = target;
    let config = TransportConfig::default();
    let key = Arc::new(
        key_store
            .load_or_generate()
            .map_err(|_| CollabRuntimeFailure::SecureKeyUnavailable)?,
    );
    let verifier = production_verifier().map_err(|error| error.failure)?;
    let local = Arc::new(RwLock::new(
        LocalAdmission::request_cancellable(key.public_key(), verifier.as_ref(), || {
            cancellation.requested()
        })
        .map_err(|error| error.failure)?,
    ));
    let local_auth = local
        .read()
        .map_err(|_| CollabRuntimeFailure::AuthenticationUnavailable)?
        .auth()
        .clone();
    let renewer = LocalTicketRenewer::new(
        *key.public_key(),
        std::sync::Arc::clone(&verifier),
        local_auth.clone(),
    )
    .map_err(|error| error.failure)?;
    let mut relay_runtime = None;
    let (addresses, expected_discovery_id, expected_remote_static, relay_join) = match &route {
        GuestConnectionRoute::Lan {
            addresses,
            discovery_id,
            expected_remote_static,
        } => (
            addresses.clone(),
            discovery_id.clone(),
            *expected_remote_static,
            false,
        ),
        GuestConnectionRoute::Relay(request) => {
            let Some(running) =
                GuestRelayRuntime::start(request, Arc::clone(&key), Arc::clone(&local), &|| {
                    cancellation.requested()
                })?
            else {
                return Ok(());
            };
            let target = relay_guest_target(&running);
            relay_runtime = Some(running);
            (target.0, target.1, target.2, true)
        }
    };
    if addresses.is_empty() {
        return Err(if relay_join {
            CollabRuntimeFailure::RelayUnavailable
        } else {
            CollabRuntimeFailure::Transport
        });
    }
    if addresses.len() > MAX_DISCOVERY_ADDRESSES {
        return Err(CollabRuntimeFailure::ResourceLimit);
    }
    let join_budget = config
        .timeouts
        .connect
        .checked_add(config.timeouts.handshake)
        .ok_or(CollabRuntimeFailure::Transport)?;
    let overall_deadline = Instant::now()
        .checked_add(join_budget)
        .ok_or(CollabRuntimeFailure::Transport)?;
    let connected = connect_address_sequence_cancellable(
        &addresses,
        overall_deadline,
        key.as_ref(),
        expected_discovery_id.as_deref(),
        expected_remote_static.as_ref(),
        config,
        cancellation,
    );
    let (prelude, mut connection) = match connected {
        Ok(connected) => connected,
        Err(error) => {
            let failure = relay_join_failure(&error, relay_join);
            if let Some(relay) = relay_runtime.as_ref() {
                let (relay_phase, relay_failure) = relay.bridge_diagnostic();
                report_relay_secure_transport_failure(failure, relay_phase, relay_failure, &error);
            }
            return Err(failure);
        }
    };
    let remote_static = *connection.remote_static();
    let (hello, expected_issuer, expected_subject) = {
        let local = local
            .read()
            .map_err(|_| CollabRuntimeFailure::AuthenticationUnavailable)?;
        (
            local.hello(intent).map_err(|error| error.failure)?,
            local.expected_issuer().to_owned(),
            local.expected_subject().to_owned(),
        )
    };
    let now_unix_ms = unix_time_ms().map_err(|error| error.failure)?;
    // An invite pins the owner's device key in its signed locator and that pin
    // was checked above, so the account behind the device does not matter. An
    // unpinned LAN join has no such anchor — mDNS is spoofable and nothing
    // else names the peer — so the plan below also demands that a human be
    // shown the verified identity and accept it before anything is admitted.
    let plan = guest_admission_plan(expected_remote_static.as_ref(), &expected_subject);
    let (_remote, identity) = connection
        .exchange_admission_initiator(
            &hello,
            verifier.as_ref(),
            &expected_issuer,
            plan.policy,
            now_unix_ms,
            Instant::now(),
        )
        .map_err(|error| runtime_failure(&error))?;
    if cancellation.requested() {
        return Ok(());
    }
    let connection_id = ConnectionKey::new(1).expect("constant is non-zero");
    // The gate sits ahead of `authorize_remote`, so the peer is still merely
    // identity-verified: no Welcome has been requested, no snapshot, presence,
    // or session name exists, and none can be applied by refusing here.
    if plan.confirmation == GuestOwnerConfirmation::Enforced {
        if !sink.send(NetworkEvent::OwnerIdentityUnconfirmed {
            connection: connection_id,
            auth: identity.to_auth_metadata(),
        }) {
            return Ok(());
        }
        match await_owner_confirmation(&commands, &shutdown, &|| cancellation.requested())? {
            GuestConfirmationOutcome::Confirmed => {}
            GuestConfirmationOutcome::Declined => {
                // Dropping `connection` closes the socket; the typed failure
                // is what the GUI turns into "you did not confirm the host".
                return Err(CollabRuntimeFailure::OwnerIdentityRejected);
            }
            GuestConfirmationOutcome::Cancelled => return Ok(()),
        }
    }
    connection
        .authorize_remote(Role::Owner)
        .map_err(|error| runtime_failure(&error))?;
    connection
        .activate(Instant::now())
        .map_err(|error| runtime_failure(&error))?;
    if !cancellation.activate() {
        return Ok(());
    }

    let session_id = prelude.prelude().session_id().clone();
    let epoch = prelude.prelude().epoch();
    if !sink.send(NetworkEvent::GuestAuthenticated {
        connection: connection_id,
        session_id: session_id.clone(),
        epoch,
        remote_static,
    }) {
        return Ok(());
    }
    let budget = SharedQueueBudget::new(config.connections.global_queued_bytes)
        .map_err(|_| CollabRuntimeFailure::ResourceLimit)?;
    let mut failure = drive_guest(
        connection,
        budget,
        DriverIdentity {
            connection: connection_id,
            session_id,
            epoch,
        },
        DriverControl { commands, shutdown },
        GuestRenewalContext {
            verifier,
            renewer,
            admission: local,
        },
        sink,
    );
    let relay_failure = relay_runtime
        .as_ref()
        .and_then(GuestRelayRuntime::terminal_failure);
    failure = prefer_guest_failure(failure, relay_failure);
    let _ = sink.send_terminal(TerminalNetworkEvent::ConnectionClosed {
        connection: connection_id,
        failure,
        remote_bye: None,
    });
    drop(relay_runtime);
    Ok(())
}

fn prefer_guest_failure(
    inner: Option<CollabRuntimeFailure>,
    relay: Option<CollabRuntimeFailure>,
) -> Option<CollabRuntimeFailure> {
    match inner {
        None | Some(CollabRuntimeFailure::Transport) => relay.or(inner),
        terminal_or_specific => terminal_or_specific,
    }
}

fn relay_join_failure(error: &RuntimeError, relay_join: bool) -> CollabRuntimeFailure {
    if !relay_join {
        return runtime_failure(error);
    }
    match error {
        RuntimeError::DiscoveryIdMismatch
        | RuntimeError::Admission(AdmissionError::StaticKeyMismatch) => {
            CollabRuntimeFailure::RelayInviteUnavailable
        }
        RuntimeError::RateLimited => CollabRuntimeFailure::ResourceLimit,
        _ => CollabRuntimeFailure::RelayUnavailable,
    }
}

fn connect_address_sequence_cancellable(
    addresses: &[SocketAddr],
    overall_deadline: Instant,
    key: &DeviceStaticKey,
    expected_discovery_id: Option<&str>,
    expected_remote_static: Option<&[u8; 32]>,
    config: TransportConfig,
    cancellation: &GuestShutdownController,
) -> Result<(EncodedServerPrelude, SecureConnection<std::net::TcpStream>), RuntimeError> {
    try_address_sequence(addresses, overall_deadline, |endpoint, attempt_deadline| {
        let is_cancelled = || cancellation.requested();
        let mut register = |stream: &TcpStream| cancellation.register_socket(stream);
        let connected = connect_secure_tcp_until_cancellable(
            endpoint,
            key,
            expected_discovery_id,
            config,
            attempt_deadline,
            &is_cancelled,
            &mut register,
        )?;
        if expected_remote_static
            .is_some_and(|expected| !bool::from(connected.1.remote_static().ct_eq(expected)))
        {
            return Err(AdmissionError::StaticKeyMismatch.into());
        }
        Ok(connected)
    })
}

#[cfg(test)]
fn connect_address_sequence(
    addresses: &[SocketAddr],
    overall_deadline: Instant,
    key: &DeviceStaticKey,
    expected_discovery_id: Option<&str>,
    expected_remote_static: Option<&[u8; 32]>,
    config: TransportConfig,
) -> Result<(EncodedServerPrelude, SecureConnection<std::net::TcpStream>), RuntimeError> {
    try_address_sequence(addresses, overall_deadline, |endpoint, attempt_deadline| {
        let connected = op_collab_transport::connect_secure_tcp_until(
            endpoint,
            key,
            expected_discovery_id,
            config,
            attempt_deadline,
        )?;
        if expected_remote_static
            .is_some_and(|expected| !bool::from(connected.1.remote_static().ct_eq(expected)))
        {
            return Err(AdmissionError::StaticKeyMismatch.into());
        }
        Ok(connected)
    })
}

fn try_address_sequence<T, F>(
    addresses: &[SocketAddr],
    overall_deadline: Instant,
    mut attempt: F,
) -> Result<T, RuntimeError>
where
    F: FnMut(SocketAddr, Instant) -> Result<T, RuntimeError>,
{
    debug_assert!(!addresses.is_empty());
    let mut last_error = None;
    for (index, endpoint) in addresses.iter().copied().enumerate() {
        let now = Instant::now();
        if last_error.is_some() && now >= overall_deadline {
            break;
        }
        let slots = u32::try_from(addresses.len() - index).unwrap_or(u32::MAX);
        let remaining = overall_deadline.saturating_duration_since(now);
        let slice = remaining
            .checked_div(slots)
            .filter(|slice| !slice.is_zero())
            .unwrap_or(Duration::from_nanos(1));
        let attempt_deadline = now
            .checked_add(slice)
            .map_or(overall_deadline, |deadline| deadline.min(overall_deadline));
        match attempt(endpoint, attempt_deadline) {
            Ok(connected) => return Ok(connected),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.expect("non-empty address sequence attempts at least once"))
}

#[cfg(test)]
#[path = "guest_tests.rs"]
mod tests;
