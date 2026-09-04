//! Guest connection-path and pinned-key tests.

use super::*;
use op_collab::{Epoch, SessionId};
use op_collab_transport::{
    accept_secure_tcp, write_server_prelude, AdmissionHello, PeerIdentityPolicy, ServerPrelude,
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

fn cancel_when_ready(ready: MpscReceiver<()>, shutdown: SyncSender<ByeReason>) -> JoinHandle<()> {
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
                return;
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
fn precise_inner_failures_outrank_a_racing_relay_local_io_failure() {
    assert_eq!(
        prefer_guest_failure(
            Some(CollabRuntimeFailure::TicketRejected),
            Some(CollabRuntimeFailure::RelayUnavailable),
        ),
        Some(CollabRuntimeFailure::TicketRejected)
    );
    assert_eq!(
        prefer_guest_failure(
            Some(CollabRuntimeFailure::Transport),
            Some(CollabRuntimeFailure::TicketRejected),
        ),
        Some(CollabRuntimeFailure::TicketRejected)
    );
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
        PeerIdentityPolicy::SameAccount { subject: "subject" },
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
    let (impostor, _, impostor_thread) = spawn_noise_endpoint(12, "rotated-impostor-discovery-id");
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
