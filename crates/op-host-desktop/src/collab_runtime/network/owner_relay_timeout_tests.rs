use std::io::{Read, Write};
use std::net::Shutdown;

use op_collab::{Epoch, SessionId};
use op_collab_transport::{connect_secure_tcp, ConnectionLimiter, TimeoutConfig};

use super::*;

#[test]
fn relay_stream_uses_the_complete_handshake_window_for_its_first_noise_frame() {
    let mut config = TransportConfig::default();
    config.timeouts.handshake = Duration::from_secs(7);
    config.timeouts.handshake_first_message = Duration::from_millis(250);

    let relay = OwnerStreamSource::Relay.transport_config(config);

    assert_eq!(
        relay.timeouts.handshake_first_message,
        relay.timeouts.handshake
    );
    assert_eq!(relay.timeouts.handshake, Duration::from_secs(7));
    assert_eq!(relay.connections, config.connections);
    assert_eq!(relay.rate, config.rate);
    assert_eq!(relay.wire_limits, config.wire_limits);
}

#[test]
fn lan_stream_keeps_the_short_first_noise_frame_guard() {
    let config = TransportConfig::default();

    assert_eq!(OwnerStreamSource::Lan.transport_config(config), config);
    assert!(config.timeouts.handshake_first_message < config.timeouts.handshake);
}

#[test]
fn relay_stream_accepts_a_first_noise_frame_after_the_lan_guard_expires() {
    let lan_first_message = Duration::from_millis(50);
    let relay_delay = Duration::from_millis(200);
    let config = TransportConfig {
        timeouts: TimeoutConfig {
            handshake: Duration::from_secs(2),
            handshake_first_message: lan_first_message,
            ..TimeoutConfig::default()
        },
        ..TransportConfig::default()
    };
    let relay_config = OwnerStreamSource::Relay.transport_config(config);
    let prelude = ServerPrelude::new(
        "00112233445566778899aabbccddeeff".to_owned(),
        SessionId::from("relay-delayed-noise"),
        Epoch(1),
    )
    .unwrap();
    let expected_discovery_id = prelude.discovery_id().to_owned();

    let owner_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let owner_address = owner_listener.local_addr().unwrap();
    let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy_address = proxy_listener.local_addr().unwrap();
    let limiter = ConnectionLimiter::with_timeouts(config.connections, config.timeouts).unwrap();
    let limiter_for_owner = limiter.clone();
    let owner = std::thread::spawn(move || {
        let (stream, peer) = owner_listener.accept().unwrap();
        let pending = limiter_for_owner.try_begin_handshake(peer.ip()).unwrap();
        accept_secure_tcp_guarded(
            stream,
            &DeviceStaticKey::from_private([41_u8; 32]).unwrap(),
            &prelude,
            relay_config,
            &pending,
        )
        .is_ok()
    });
    let proxy = std::thread::spawn(move || {
        let (mut guest_stream, _) = proxy_listener.accept().unwrap();
        let mut owner_stream = TcpStream::connect(owner_address).unwrap();
        let mut owner_reader = owner_stream.try_clone().unwrap();
        let mut guest_writer = guest_stream.try_clone().unwrap();
        let downstream = std::thread::spawn(move || {
            let _ = std::io::copy(&mut owner_reader, &mut guest_writer);
            let _ = guest_writer.shutdown(Shutdown::Write);
        });

        let mut first_frame = [0_u8; 8 * 1024];
        let first_len = guest_stream.read(&mut first_frame).unwrap();
        assert!(first_len > 0);
        std::thread::sleep(relay_delay);
        owner_stream.write_all(&first_frame[..first_len]).unwrap();
        let _ = std::io::copy(&mut guest_stream, &mut owner_stream);
        let _ = owner_stream.shutdown(Shutdown::Write);
        downstream.join().unwrap();
    });

    let started = Instant::now();
    let (_, connection) = connect_secure_tcp(
        proxy_address,
        &DeviceStaticKey::from_private([42_u8; 32]).unwrap(),
        Some(&expected_discovery_id),
        config,
    )
    .unwrap();
    assert!(started.elapsed() >= relay_delay);
    assert!(started.elapsed() > lan_first_message);
    drop(connection);

    assert!(owner.join().unwrap());
    proxy.join().unwrap();
}
