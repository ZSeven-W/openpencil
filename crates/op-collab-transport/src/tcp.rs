use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use polling::{Event, Events, Poller};
use socket2::{Domain, Protocol, SockRef, Socket, TcpKeepalive, Type};

use crate::{
    read_server_prelude, run_xx_initiator, run_xx_responder, write_server_prelude, ConfigError,
    DeviceStaticKey, EncodedServerPrelude, RuntimeError, SecureConnection, ServerPrelude,
    TransportConfig,
};

/// Connects a raw TCP socket for a manual `SocketAddr` endpoint.
pub fn connect_manual(
    address: SocketAddr,
    config: TransportConfig,
) -> Result<TcpStream, RuntimeError> {
    let config = config.validate()?;
    let stream = TcpStream::connect_timeout(&address, config.timeouts.connect)?;
    prepare_tcp_stream(&stream, config)?;
    Ok(stream)
}

/// Connects, reads and binds the raw server prelude, then completes Noise XX.
///
/// `expected_discovery_id` should be supplied for an mDNS-selected endpoint.
/// Manual-address connections pass `None` and inspect the returned prelude.
pub fn connect_secure_tcp(
    address: SocketAddr,
    local_static: &DeviceStaticKey,
    expected_discovery_id: Option<&str>,
    config: TransportConfig,
) -> Result<(EncodedServerPrelude, SecureConnection<TcpStream>), RuntimeError> {
    connect_secure_tcp_inner(
        address,
        local_static,
        expected_discovery_id,
        config,
        None,
        None,
        None,
    )
}

/// Connects and completes Noise within one caller-owned monotonic deadline.
///
/// This is used when a discovery result contains several addresses: each
/// attempt receives a fair slice of one overall join budget, so unreachable or
/// silent addresses cannot multiply the configured connect/handshake timeout.
pub fn connect_secure_tcp_until(
    address: SocketAddr,
    local_static: &DeviceStaticKey,
    expected_discovery_id: Option<&str>,
    config: TransportConfig,
    deadline: Instant,
) -> Result<(EncodedServerPrelude, SecureConnection<TcpStream>), RuntimeError> {
    connect_secure_tcp_inner(
        address,
        local_static,
        expected_discovery_id,
        config,
        Some(deadline),
        None,
        None,
    )
}

/// Connects and completes Noise within a deadline while allowing a host-owned
/// lifecycle token to interrupt the TCP connect and register the live socket.
///
/// `on_connected` runs after TCP establishment but before any prelude or Noise
/// bytes are read. A host can retain a cloned socket and call
/// [`TcpStream::shutdown`] when `is_cancelled` becomes true, which also
/// interrupts blocking prelude, Noise, and admission reads.
pub fn connect_secure_tcp_until_cancellable(
    address: SocketAddr,
    local_static: &DeviceStaticKey,
    expected_discovery_id: Option<&str>,
    config: TransportConfig,
    deadline: Instant,
    is_cancelled: &dyn Fn() -> bool,
    on_connected: &mut dyn FnMut(&TcpStream) -> std::io::Result<()>,
) -> Result<(EncodedServerPrelude, SecureConnection<TcpStream>), RuntimeError> {
    connect_secure_tcp_inner(
        address,
        local_static,
        expected_discovery_id,
        config,
        Some(deadline),
        Some(is_cancelled),
        Some(on_connected),
    )
}

type ConnectedHook<'a> = &'a mut dyn FnMut(&TcpStream) -> std::io::Result<()>;

fn connect_secure_tcp_inner(
    address: SocketAddr,
    local_static: &DeviceStaticKey,
    expected_discovery_id: Option<&str>,
    config: TransportConfig,
    overall_deadline: Option<Instant>,
    is_cancelled: Option<&dyn Fn() -> bool>,
    mut on_connected: Option<ConnectedHook<'_>>,
) -> Result<(EncodedServerPrelude, SecureConnection<TcpStream>), RuntimeError> {
    let config = config.validate()?;
    let connect_timeout = match overall_deadline {
        Some(deadline) => remaining_timeout(deadline, config.timeouts.connect)?,
        None => config.timeouts.connect,
    };
    let mut stream = match is_cancelled {
        Some(is_cancelled) => connect_tcp_cancellable(address, connect_timeout, is_cancelled)?,
        None => TcpStream::connect_timeout(&address, connect_timeout)?,
    };
    configure_tcp_common(&stream, config)?;
    if is_cancelled.is_some_and(|cancelled| cancelled()) {
        return Err(cancelled_io_error().into());
    }
    if let Some(on_connected) = on_connected.as_mut() {
        on_connected(&stream)?;
    }
    let deadline = handshake_deadline(config, overall_deadline)?;
    let (prelude, noise) = {
        let mut io = DeadlineTcp::new(&mut stream, deadline);
        let prelude = read_server_prelude(&mut io)?;
        if expected_discovery_id
            .is_some_and(|expected| expected != prelude.prelude().discovery_id())
        {
            return Err(RuntimeError::DiscoveryIdMismatch);
        }
        let noise = run_xx_initiator(&mut io, local_static, &prelude)?;
        (prelude, noise)
    };
    prepare_tcp_stream(&stream, config)?;
    let connection = SecureConnection::new(stream, noise, config, Instant::now())?;
    Ok((prelude, connection))
}

const CONNECT_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CONNECT_EVENT_KEY: usize = 1;

fn connect_tcp_cancellable(
    address: SocketAddr,
    timeout: Duration,
    is_cancelled: &dyn Fn() -> bool,
) -> std::io::Result<TcpStream> {
    if is_cancelled() {
        return Err(cancelled_io_error());
    }
    let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "collaboration connect timeout overflowed",
        )
    })?;
    let domain = if address.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_nonblocking(true)?;
    match socket.connect(&address.into()) {
        Ok(()) => {}
        Err(error) if connect_is_in_progress(&error) => {
            wait_for_connect(&socket, deadline, is_cancelled)?;
        }
        Err(error) => return Err(error),
    }
    if is_cancelled() {
        return Err(cancelled_io_error());
    }
    socket.set_nonblocking(false)?;
    Ok(socket.into())
}

fn connect_is_in_progress(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
    ) || error.raw_os_error() == Some(libc::EINPROGRESS)
        || (cfg!(target_os = "windows") && error.raw_os_error() == Some(10_036))
}

fn wait_for_connect(
    socket: &Socket,
    deadline: Instant,
    is_cancelled: &dyn Fn() -> bool,
) -> std::io::Result<()> {
    let poller = Poller::new()?;
    // SAFETY: `socket` remains alive until after `poller` is dropped.
    unsafe {
        poller.add(socket, Event::writable(CONNECT_EVENT_KEY))?;
    }
    let mut events = Events::new();
    loop {
        if is_cancelled() {
            return Err(cancelled_io_error());
        }
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(connect_timeout_error)?;
        events.clear();
        poller.wait(
            &mut events,
            Some(remaining.min(CONNECT_CANCEL_POLL_INTERVAL)),
        )?;
        if events.is_empty() {
            continue;
        }
        if let Some(error) = socket.take_error()? {
            return Err(error);
        }
        match socket.peer_addr() {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotConnected => {
                poller.modify(socket, Event::writable(CONNECT_EVENT_KEY))?;
            }
            Err(error) => return Err(error),
        }
    }
}

fn connect_timeout_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "collaboration connection deadline expired",
    )
}

fn cancelled_io_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Interrupted,
        "collaboration connection cancelled",
    )
}

/// Writes the raw prelude and completes the responder side of Noise XX.
///
/// The caller should hold a [`crate::PendingHandshakeGuard`] before invoking
/// this function so connection-flood limits apply before cryptographic work.
pub fn accept_secure_tcp(
    mut stream: TcpStream,
    local_static: &DeviceStaticKey,
    prelude: &ServerPrelude,
    config: TransportConfig,
) -> Result<SecureConnection<TcpStream>, RuntimeError> {
    let config = config.validate()?;
    configure_tcp_common(&stream, config)?;
    let deadline = handshake_deadline(config, None)?;
    let noise = {
        let mut io = DeadlineTcp::new(&mut stream, deadline);
        let encoded = write_server_prelude(&mut io, prelude)?;
        run_xx_responder(&mut io, local_static, &encoded)?
    };
    prepare_tcp_stream(&stream, config)?;
    SecureConnection::new(stream, noise, config, Instant::now())
}

/// Applies steady-state TCP options after the bounded handshake phase.
pub fn prepare_tcp_stream(stream: &TcpStream, config: TransportConfig) -> Result<(), RuntimeError> {
    let config = config.validate()?;
    configure_tcp_common(stream, config)?;
    stream.set_read_timeout(Some(config.timeouts.read_write))?;
    stream.set_write_timeout(Some(config.timeouts.read_write))?;
    Ok(())
}

fn configure_tcp_common(stream: &TcpStream, config: TransportConfig) -> Result<(), RuntimeError> {
    stream.set_nodelay(true)?;
    let socket = SockRef::from(stream);
    socket.set_keepalive(true)?;
    let keepalive = TcpKeepalive::new()
        .with_time(config.timeouts.heartbeat)
        .with_interval(config.timeouts.heartbeat);
    socket.set_tcp_keepalive(&keepalive)?;
    Ok(())
}

fn handshake_deadline(
    config: TransportConfig,
    overall_deadline: Option<Instant>,
) -> Result<Instant, RuntimeError> {
    let configured = Instant::now()
        .checked_add(config.timeouts.handshake)
        .ok_or(ConfigError::InvalidValue {
            field: "timeouts.handshake",
        })
        .map_err(RuntimeError::from)?;
    Ok(overall_deadline.map_or(configured, |overall| overall.min(configured)))
}

fn remaining_timeout(deadline: Instant, configured: Duration) -> std::io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .map(|remaining| remaining.min(configured))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "collaboration connection deadline expired",
            )
        })
}

struct DeadlineTcp<'a> {
    stream: &'a mut TcpStream,
    deadline: Instant,
}

impl<'a> DeadlineTcp<'a> {
    const fn new(stream: &'a mut TcpStream, deadline: Instant) -> Self {
        Self { stream, deadline }
    }

    fn remaining(&self) -> std::io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|duration| !duration.is_zero())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Noise handshake deadline expired",
                )
            })
    }
}

impl Read for DeadlineTcp<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(buffer)
    }
}

impl Write for DeadlineTcp<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        read_ciphertext_record, AdmissionHello, AdmissionPhase, JoinIntent, TicketVerifier,
        TransferClass, VerifiedTicketClaims,
    };
    use op_collab::{Bye, ByeReason, CollabMessage, Epoch, FrameEnvelope, Role, SessionId};
    use std::net::TcpListener;
    use std::sync::mpsc;

    const ISSUER: &str = "https://issuer.example";
    const SUBJECT: &str = "00000000-0000-0000-0000-000000000001";
    const OWNER_DEVICE: &str = "00000000-0000-0000-0000-000000000002";
    const GUEST_DEVICE: &str = "00000000-0000-0000-0000-000000000003";
    const NOW_UNIX_MS: u64 = 1_000;

    fn server_prelude() -> ServerPrelude {
        ServerPrelude::new(
            "00112233445566778899aabbccddeeff".to_owned(),
            SessionId::from("session"),
            Epoch(1),
        )
        .unwrap()
    }

    fn verifier(owner_static: [u8; 32], guest_static: [u8; 32]) -> impl TicketVerifier {
        move |ticket: &[u8], expected: &[u8; 32], _now: u64| {
            let (static_key, device, expiry) = match ticket {
                b"owner-ticket" => (owner_static, OWNER_DEVICE, 61_000),
                b"owner-renewed" => (owner_static, OWNER_DEVICE, 121_000),
                b"guest-ticket" => (guest_static, GUEST_DEVICE, 61_000),
                _ => return Err(crate::AdmissionError::Verification),
            };
            if static_key != *expected {
                return Err(crate::AdmissionError::StaticKeyMismatch);
            }
            VerifiedTicketClaims::new(
                ISSUER.into(),
                SUBJECT.into(),
                device.into(),
                static_key,
                expiry,
            )
        }
    }

    #[test]
    fn tcp_noise_mutual_admission_activation_frame_and_rate_limit() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let mut config = TransportConfig::default();
        config.rate.byte_burst = crate::MAX_NOISE_PLAINTEXT_BYTES as u64;
        config.rate.bytes_per_second = 1;
        config.validate().unwrap();

        let owner_key = DeviceStaticKey::from_private([2_u8; 32]).unwrap();
        let guest_key = DeviceStaticKey::from_private([1_u8; 32]).unwrap();
        let owner_public = *owner_key.public_key();
        let guest_public = *guest_key.public_key();
        let (ready_tx, ready_rx) = mpsc::channel();

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut connection =
                accept_secure_tcp(stream, &owner_key, &server_prelude(), config).unwrap();
            let local = AdmissionHello::new(b"owner-ticket".to_vec(), JoinIntent::New).unwrap();
            let (_, identity) = connection
                .exchange_admission_responder(
                    &local,
                    &verifier(owner_public, guest_public),
                    ISSUER,
                    SUBJECT,
                    NOW_UNIX_MS,
                    Instant::now(),
                )
                .unwrap();
            assert_eq!(identity.claims().device_id(), GUEST_DEVICE);
            connection.authorize_remote(Role::Editor).unwrap();
            connection.activate(Instant::now()).unwrap();
            assert_eq!(connection.admission_state().phase(), AdmissionPhase::Active);
            let frame = connection.receive_frame().unwrap();
            ready_tx.send(()).unwrap();
            let mut stream = connection.into_inner();
            let unexpected = read_ciphertext_record(&mut stream);
            (frame, unexpected)
        });

        let (prelude, mut connection) = connect_secure_tcp(
            address,
            &guest_key,
            Some("00112233445566778899aabbccddeeff"),
            config,
        )
        .unwrap();
        assert_eq!(prelude.prelude().session_id().as_ref(), "session");
        let local = AdmissionHello::new(b"guest-ticket".to_vec(), JoinIntent::New).unwrap();
        let client_now = Instant::now();
        let (_, identity) = connection
            .exchange_admission_initiator(
                &local,
                &verifier(owner_public, guest_public),
                ISSUER,
                SUBJECT,
                NOW_UNIX_MS,
                client_now,
            )
            .unwrap();
        assert_eq!(identity.claims().device_id(), OWNER_DEVICE);
        connection.authorize_remote(Role::Owner).unwrap();
        connection.activate(Instant::now()).unwrap();

        let frame = FrameEnvelope::new(
            SessionId::from("session"),
            Epoch(1),
            CollabMessage::Bye(Bye {
                reason: ByeReason::Normal,
            }),
        );
        connection.send_frame(&frame, Instant::now()).unwrap();
        ready_rx.recv().unwrap();
        let payload = vec![0_u8; crate::MAX_CHUNK_PAYLOAD + 1];
        assert!(matches!(
            connection.send_transfer(TransferClass::Txn, &payload, Instant::now()),
            Err(RuntimeError::RateLimited)
        ));
        assert!(matches!(
            connection.check_ticket_expiry(client_now + Duration::from_secs(60)),
            Err(RuntimeError::Admission(
                crate::AdmissionError::TicketExpired
            ))
        ));
        drop(connection);

        let (received, unexpected) = server.join().unwrap();
        assert_eq!(received.body().kind(), "bye");
        assert!(
            unexpected.is_err(),
            "rate rejection wrote a partial transfer"
        );
    }

    #[test]
    fn selected_discovery_id_mismatch_fails_before_noise() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let config = TransportConfig::default();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            accept_secure_tcp(
                stream,
                &DeviceStaticKey::from_private([3_u8; 32]).unwrap(),
                &server_prelude(),
                config,
            )
            .is_err()
        });
        let result = connect_secure_tcp(
            address,
            &DeviceStaticKey::from_private([4_u8; 32]).unwrap(),
            Some("ffffffffffffffffffffffffffffffff"),
            config,
        );
        assert!(matches!(result, Err(RuntimeError::DiscoveryIdMismatch)));
        assert!(server.join().unwrap());
    }

    #[test]
    fn expired_overall_connect_deadline_fails_before_socket_io() {
        let result = connect_secure_tcp_until(
            "127.0.0.1:9".parse().unwrap(),
            &DeviceStaticKey::from_private([4_u8; 32]).unwrap(),
            None,
            TransportConfig::default(),
            Instant::now() - Duration::from_millis(1),
        );
        assert!(matches!(
            result,
            Err(RuntimeError::Io(error)) if error.kind() == std::io::ErrorKind::TimedOut
        ));
    }
}
