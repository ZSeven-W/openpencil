use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
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
use super::super::types::{
    CollabRuntimeFailure, GuestNetworkCommand, NetworkEvent, TerminalNetworkEvent,
};
use super::connection::{drive_guest, runtime_failure, DriverControl, DriverIdentity};
use super::EventSink;

pub(super) struct GuestTarget {
    pub(super) addresses: Vec<SocketAddr>,
    pub(super) expected_discovery_id: Option<String>,
    pub(super) expected_remote_static: Option<[u8; 32]>,
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
    let GuestTarget {
        addresses,
        expected_discovery_id,
        expected_remote_static,
        intent,
    } = target;
    let config = TransportConfig::default();
    if addresses.is_empty() {
        return Err(CollabRuntimeFailure::Transport);
    }
    if addresses.len() > MAX_DISCOVERY_ADDRESSES {
        return Err(CollabRuntimeFailure::ResourceLimit);
    }
    let key = key_store
        .load_or_generate()
        .map_err(|_| CollabRuntimeFailure::SecureKeyUnavailable)?;
    let verifier = production_verifier().map_err(|error| error.failure)?;
    let local = LocalAdmission::request_cancellable(key.public_key(), verifier.as_ref(), || {
        cancellation.requested()
    })
    .map_err(|error| error.failure)?;
    let local_auth = local.auth().clone();
    let renewer = LocalTicketRenewer::new(
        *key.public_key(),
        std::sync::Arc::clone(&verifier),
        local_auth.clone(),
    )
    .map_err(|error| error.failure)?;
    let join_budget = config
        .timeouts
        .connect
        .checked_add(config.timeouts.handshake)
        .ok_or(CollabRuntimeFailure::Transport)?;
    let overall_deadline = Instant::now()
        .checked_add(join_budget)
        .ok_or(CollabRuntimeFailure::Transport)?;
    let (prelude, mut connection) = connect_address_sequence_cancellable(
        &addresses,
        overall_deadline,
        &key,
        expected_discovery_id.as_deref(),
        expected_remote_static.as_ref(),
        config,
        cancellation,
    )
    .map_err(|error| runtime_failure(&error))?;
    let remote_static = *connection.remote_static();
    let hello = local.hello(intent).map_err(|error| error.failure)?;
    let now_unix_ms = unix_time_ms().map_err(|error| error.failure)?;
    connection
        .exchange_admission_initiator(
            &hello,
            verifier.as_ref(),
            local.expected_issuer(),
            local.expected_subject(),
            now_unix_ms,
            Instant::now(),
        )
        .map_err(|error| runtime_failure(&error))?;
    if cancellation.requested() {
        return Ok(());
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

    let connection_id = ConnectionKey::new(1).expect("constant is non-zero");
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
    let failure = drive_guest(
        connection,
        budget,
        DriverIdentity {
            connection: connection_id,
            session_id,
            epoch,
        },
        DriverControl { commands, shutdown },
        verifier,
        renewer,
        sink,
    );
    let _ = sink.send_terminal(TerminalNetworkEvent::ConnectionClosed {
        connection: connection_id,
        failure,
        remote_bye: None,
    });
    Ok(())
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
mod tests {
    use super::*;
    use op_collab::{Epoch, SessionId};
    use op_collab_transport::{
        accept_secure_tcp, write_server_prelude, AdmissionHello, ServerPrelude,
    };
    use std::io::{ErrorKind, Read};
    use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
    use std::sync::mpsc::{Receiver as MpscReceiver, SyncSender};
    use std::thread::JoinHandle;

    fn spawn_noise_endpoint(
        private_key_byte: u8,
        discovery_id: &str,
    ) -> (SocketAddr, [u8; 32], JoinHandle<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let endpoint = listener.local_addr().unwrap();
        let key = DeviceStaticKey::from_private([private_key_byte; 32]).unwrap();
        let public_key = *key.public_key();
        let prelude = ServerPrelude::new(
            discovery_id.to_owned(),
            SessionId::from("pinned-owner-test"),
            Epoch(7),
        )
        .unwrap();
        let thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let connection =
                accept_secure_tcp(stream, &key, &prelude, TransportConfig::default()).unwrap();
            drop(connection);
        });
        (endpoint, public_key, thread)
    }

    fn cancellation_controller() -> (
        GuestShutdownController,
        MpscReceiver<ByeReason>,
        SyncSender<ByeReason>,
    ) {
        let (shutdown, external) = mpsc::sync_channel(1);
        let (controller, forwarded) = GuestShutdownController::start(external).unwrap();
        (controller, forwarded, shutdown)
    }

    fn cancel_when_ready(
        ready: MpscReceiver<()>,
        shutdown: SyncSender<ByeReason>,
    ) -> JoinHandle<()> {
        std::thread::spawn(move || {
            ready.recv().unwrap();
            shutdown.send(ByeReason::Normal).unwrap();
        })
    }

    fn drain_until_closed(mut stream: TcpStream) {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut bytes = [0_u8; 256];
        loop {
            match stream.read(&mut bytes) {
                Ok(0) => return,
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::ConnectionReset | ErrorKind::BrokenPipe
                    ) =>
                {
                    return
                }
                Err(error) => panic!("cancelled join socket did not close: {error}"),
            }
        }
    }

    fn cancellation_prelude(discovery_id: &str) -> ServerPrelude {
        ServerPrelude::new(
            discovery_id.to_owned(),
            SessionId::from("cancelled-guest-join"),
            Epoch(9),
        )
        .unwrap()
    }

    #[test]
    fn address_fallback_reaches_second_loopback_endpoint() {
        let unavailable = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let unavailable_address = unavailable.local_addr().unwrap();
        drop(unavailable);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let loopback = listener.local_addr().unwrap();
        let overall = Instant::now() + Duration::from_secs(1);

        let stream = try_address_sequence(
            &[unavailable_address, loopback],
            overall,
            |endpoint, deadline| {
                let timeout = deadline.saturating_duration_since(Instant::now());
                TcpStream::connect_timeout(&endpoint, timeout).map_err(RuntimeError::Io)
            },
        )
        .expect("second address connects");
        assert_eq!(stream.peer_addr().unwrap(), loopback);
        listener.accept().expect("loopback accepted");
    }

    #[test]
    fn address_attempt_deadlines_share_one_overall_budget() {
        let addresses = (1..=4)
            .map(|port| SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
            .collect::<Vec<_>>();
        let overall = Instant::now() + Duration::from_secs(2);
        let mut deadlines = Vec::new();
        let result = try_address_sequence(&addresses, overall, |_, deadline| {
            deadlines.push(deadline);
            Err::<(), _>(RuntimeError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "test",
            )))
        });
        assert!(result.is_err());
        assert_eq!(deadlines.len(), addresses.len());
        assert!(deadlines.into_iter().all(|deadline| deadline <= overall));
    }

    #[test]
    fn silent_prelude_shutdown_is_bounded_and_next_launch_starts_cleanly() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let endpoint = listener.local_addr().unwrap();
        let (ready, accepted) = mpsc::sync_channel(1);
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            ready.send(()).unwrap();
            drain_until_closed(stream);
        });
        let guest_key = DeviceStaticKey::from_private([21; 32]).unwrap();
        let (controller, forwarded, shutdown) = cancellation_controller();
        let cancel = cancel_when_ready(accepted, shutdown);
        let started = Instant::now();
        let result = connect_address_sequence_cancellable(
            &[endpoint],
            started + Duration::from_secs(10),
            &guest_key,
            None,
            None,
            TransportConfig::default(),
            &controller,
        );
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(controller.requested());
        assert_eq!(
            forwarded.recv_timeout(Duration::from_secs(1)).unwrap(),
            ByeReason::Normal
        );
        controller.finish();
        cancel.join().unwrap();
        server.join().unwrap();

        let (next_endpoint, _, next_server) = spawn_noise_endpoint(22, "fresh-after-cancel");
        let (fresh, _forwarded, keep_alive) = cancellation_controller();
        let started = Instant::now();
        let connected = connect_address_sequence_cancellable(
            &[next_endpoint],
            started + Duration::from_secs(2),
            &guest_key,
            None,
            None,
            TransportConfig::default(),
            &fresh,
        )
        .expect("a retired cancellation token cannot poison the next launch");
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(connected);
        fresh.finish();
        drop(keep_alive);
        next_server.join().unwrap();
    }

    #[test]
    fn silent_noise_shutdown_interrupts_the_registered_socket() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let endpoint = listener.local_addr().unwrap();
        let (ready, prelude_sent) = mpsc::sync_channel(1);
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            write_server_prelude(&mut stream, &cancellation_prelude("silent-noise")).unwrap();
            ready.send(()).unwrap();
            drain_until_closed(stream);
        });
        let guest_key = DeviceStaticKey::from_private([23; 32]).unwrap();
        let (controller, _forwarded, shutdown) = cancellation_controller();
        let cancel = cancel_when_ready(prelude_sent, shutdown);
        let started = Instant::now();
        let result = connect_address_sequence_cancellable(
            &[endpoint],
            started + Duration::from_secs(10),
            &guest_key,
            Some("silent-noise"),
            None,
            TransportConfig::default(),
            &controller,
        );
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(controller.requested());
        controller.finish();
        cancel.join().unwrap();
        server.join().unwrap();
    }

    #[test]
    fn silent_admission_shutdown_interrupts_the_registered_socket() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let endpoint = listener.local_addr().unwrap();
        let owner_key = DeviceStaticKey::from_private([24; 32]).unwrap();
        let (ready, noise_complete) = mpsc::sync_channel(1);
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let connection = accept_secure_tcp(
                stream,
                &owner_key,
                &cancellation_prelude("silent-admission"),
                TransportConfig::default(),
            )
            .unwrap();
            ready.send(()).unwrap();
            drain_until_closed(connection.into_inner());
        });
        let guest_key = DeviceStaticKey::from_private([25; 32]).unwrap();
        let (controller, _forwarded, shutdown) = cancellation_controller();
        let cancel = cancel_when_ready(noise_complete, shutdown);
        let started = Instant::now();
        let (_, mut connection) = connect_address_sequence_cancellable(
            &[endpoint],
            started + Duration::from_secs(10),
            &guest_key,
            Some("silent-admission"),
            None,
            TransportConfig::default(),
            &controller,
        )
        .expect("Noise completes before the admission stall");
        let hello = AdmissionHello::new(vec![1], JoinIntent::New).unwrap();
        let verifier = |_: &[u8], _: &[u8; 32], _: u64| Err(AdmissionError::Verification);
        let result = connection.exchange_admission_initiator(
            &hello,
            &verifier,
            "issuer",
            "subject",
            1,
            Instant::now(),
        );
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(controller.requested());
        controller.finish();
        cancel.join().unwrap();
        server.join().unwrap();
    }

    #[test]
    fn pinned_reconnect_skips_impersonator_and_accepts_owner_after_discovery_rotation() {
        let (impostor, _, impostor_thread) =
            spawn_noise_endpoint(12, "rotated-impostor-discovery-id");
        let (owner, owner_static, owner_thread) =
            spawn_noise_endpoint(13, "rotated-owner-discovery-id");
        let guest_key = DeviceStaticKey::from_private([14; 32]).unwrap();
        let overall = Instant::now() + Duration::from_secs(2);

        let (prelude, connection) = connect_address_sequence(
            &[impostor, owner],
            overall,
            &guest_key,
            None,
            Some(&owner_static),
            TransportConfig::default(),
        )
        .expect("same pinned owner is accepted after discovery id rotation");

        assert_eq!(
            prelude.prelude().discovery_id(),
            "rotated-owner-discovery-id"
        );
        assert!(bool::from(connection.remote_static().ct_eq(&owner_static)));
        drop(connection);
        impostor_thread.join().unwrap();
        owner_thread.join().unwrap();
    }

    #[test]
    fn pinned_reconnect_rejects_an_impersonating_endpoint_before_admission() {
        let (impostor, _, impostor_thread) = spawn_noise_endpoint(15, "impostor-discovery-id");
        let expected_owner = DeviceStaticKey::from_private([16; 32]).unwrap();
        let guest_key = DeviceStaticKey::from_private([17; 32]).unwrap();
        let overall = Instant::now() + Duration::from_secs(1);

        let result = connect_address_sequence(
            &[impostor],
            overall,
            &guest_key,
            None,
            Some(expected_owner.public_key()),
            TransportConfig::default(),
        );
        let error = match result {
            Ok(_) => panic!("a different Noise static must fail before ticket exchange"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            RuntimeError::Admission(AdmissionError::StaticKeyMismatch)
        ));
        impostor_thread.join().unwrap();
    }
}
