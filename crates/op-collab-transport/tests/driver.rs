use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use jian_ops_schema::PenDocument;
use op_collab::{
    canonical_document_hash, Bye, ByeReason, CollabMessage, CommitSeq, Epoch, FrameEnvelope,
    Presence, Role, SessionId, Snapshot,
};
use op_collab_transport::{
    accept_secure_tcp, connect_secure_tcp, AdmissionHello, ConnectionDriver, DeviceStaticKey,
    DriverEvent, EncodedFrameTransfer, InboundTransferPolicy, JoinIntent, RuntimeError,
    SecureConnection, ServerPrelude, SharedQueueBudget, TicketVerifier, TransportConfig,
    VerifiedTicketClaims,
};

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
    verifier_with_expiry(owner_static, guest_static, 10 * 60 * 1_000 + NOW_UNIX_MS)
}

fn verifier_with_expiry(
    owner_static: [u8; 32],
    guest_static: [u8; 32],
    expires_at_unix_ms: u64,
) -> impl TicketVerifier {
    move |ticket: &[u8], expected: &[u8; 32], _now: u64| {
        let (static_key, device) = match ticket {
            b"owner-ticket" => (owner_static, OWNER_DEVICE),
            b"guest-ticket" => (guest_static, GUEST_DEVICE),
            _ => return Err(op_collab_transport::AdmissionError::Verification),
        };
        if static_key != *expected {
            return Err(op_collab_transport::AdmissionError::StaticKeyMismatch);
        }
        VerifiedTicketClaims::new(
            ISSUER.into(),
            SUBJECT.into(),
            device.into(),
            static_key,
            expires_at_unix_ms,
        )
    }
}

fn admitted_pair(
    config: TransportConfig,
) -> (SecureConnection<TcpStream>, SecureConnection<TcpStream>) {
    admitted_pair_with_expiry(config, 10 * 60 * 1_000 + NOW_UNIX_MS)
}

fn admitted_pair_with_expiry(
    config: TransportConfig,
    expires_at_unix_ms: u64,
) -> (SecureConnection<TcpStream>, SecureConnection<TcpStream>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let owner_key = DeviceStaticKey::from_private([2_u8; 32]).unwrap();
    let guest_key = DeviceStaticKey::from_private([1_u8; 32]).unwrap();
    let owner_public = *owner_key.public_key();
    let guest_public = *guest_key.public_key();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut connection =
            accept_secure_tcp(stream, &owner_key, &server_prelude(), config).unwrap();
        let local = AdmissionHello::new(b"owner-ticket".to_vec(), JoinIntent::New).unwrap();
        connection
            .exchange_admission_responder(
                &local,
                &verifier_with_expiry(owner_public, guest_public, expires_at_unix_ms),
                ISSUER,
                SUBJECT,
                NOW_UNIX_MS,
                Instant::now(),
            )
            .unwrap();
        connection.authorize_remote(Role::Editor).unwrap();
        connection.activate(Instant::now()).unwrap();
        connection
    });

    let (_, mut guest) = connect_secure_tcp(
        address,
        &guest_key,
        Some("00112233445566778899aabbccddeeff"),
        config,
    )
    .unwrap();
    let local = AdmissionHello::new(b"guest-ticket".to_vec(), JoinIntent::New).unwrap();
    guest
        .exchange_admission_initiator(
            &local,
            &verifier_with_expiry(owner_public, guest_public, expires_at_unix_ms),
            ISSUER,
            SUBJECT,
            NOW_UNIX_MS,
            Instant::now(),
        )
        .unwrap();
    guest.authorize_remote(Role::Owner).unwrap();
    guest.activate(Instant::now()).unwrap();
    (server.join().unwrap(), guest)
}

fn driver_pair(mut config: TransportConfig) -> (ConnectionDriver, ConnectionDriver) {
    config.connections.outbound_queue_items = 64;
    let (owner, guest) = admitted_pair(config);
    let budget = SharedQueueBudget::new(config.connections.global_queued_bytes).unwrap();
    (
        ConnectionDriver::new(owner, budget.clone(), InboundTransferPolicy::PeerToOwner).unwrap(),
        ConnectionDriver::new(guest, budget, InboundTransferPolicy::OwnerToGuest).unwrap(),
    )
}

fn driver_pair_with_expiry(
    mut config: TransportConfig,
    expires_at_unix_ms: u64,
) -> (ConnectionDriver, ConnectionDriver) {
    config.connections.outbound_queue_items = 64;
    let (owner, guest) = admitted_pair_with_expiry(config, expires_at_unix_ms);
    let budget = SharedQueueBudget::new(config.connections.global_queued_bytes).unwrap();
    (
        ConnectionDriver::new(owner, budget.clone(), InboundTransferPolicy::PeerToOwner).unwrap(),
        ConnectionDriver::new(guest, budget, InboundTransferPolicy::OwnerToGuest).unwrap(),
    )
}

fn encrypted_driver_pair(
    config: TransportConfig,
) -> (ConnectionDriver, ConnectionDriver, [u8; 32], [u8; 32]) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let owner_key = DeviceStaticKey::from_private([12_u8; 32]).unwrap();
    let guest_key = DeviceStaticKey::from_private([11_u8; 32]).unwrap();
    let owner_public = *owner_key.public_key();
    let guest_public = *guest_key.public_key();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        accept_secure_tcp(stream, &owner_key, &server_prelude(), config).unwrap()
    });
    let (_, guest) = connect_secure_tcp(
        address,
        &guest_key,
        Some("00112233445566778899aabbccddeeff"),
        config,
    )
    .unwrap();
    let owner = server.join().unwrap();
    let budget = SharedQueueBudget::new(config.connections.global_queued_bytes).unwrap();
    (
        ConnectionDriver::new(owner, budget.clone(), InboundTransferPolicy::PeerToOwner).unwrap(),
        ConnectionDriver::new(guest, budget, InboundTransferPolicy::OwnerToGuest).unwrap(),
        owner_public,
        guest_public,
    )
}

fn bye_frame() -> FrameEnvelope {
    FrameEnvelope::new(
        SessionId::from("session"),
        Epoch(1),
        CollabMessage::Bye(Bye {
            reason: ByeReason::Normal,
        }),
    )
}

fn presence_update_frame(editing_node: Option<&str>) -> FrameEnvelope {
    FrameEnvelope::new(
        SessionId::from("session"),
        Epoch(1),
        CollabMessage::PresenceUpdate(Presence {
            cursor: None,
            selection: Vec::new(),
            viewport: None,
            editing_node: editing_node.map(Into::into),
        }),
    )
}

fn large_snapshot_frame() -> FrameEnvelope {
    let source = format!("data:image/png;base64,{}", "A".repeat(130_000));
    let document: PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{"type": "image", "id": "image-1", "src": source}]
    }))
    .unwrap();
    let doc_hash = canonical_document_hash(&document).unwrap();
    FrameEnvelope::new(
        SessionId::from("session"),
        Epoch(1),
        CollabMessage::Snapshot(Box::new(Snapshot {
            seq: CommitSeq(0),
            document,
            doc_hash,
        })),
    )
}

fn take_frame(event: Option<DriverEvent>) -> bool {
    matches!(event, Some(DriverEvent::Frame { .. }))
}

fn wait_admission(driver: &mut ConnectionDriver, now: Instant) -> AdmissionHello {
    for _ in 0..1_000 {
        if let Some(DriverEvent::Admission(hello)) = driver.poll(now).unwrap().event {
            return hello;
        }
        std::thread::yield_now();
    }
    panic!("admission did not arrive");
}

#[test]
fn nonblocking_driver_completes_mutual_admission_without_blocking_reads() {
    let config = TransportConfig::default();
    let (mut owner, mut guest, owner_public, guest_public) = encrypted_driver_pair(config);
    let now = Instant::now();
    let guest_hello = AdmissionHello::new(b"guest-ticket".to_vec(), JoinIntent::New).unwrap();
    guest.queue_admission(&guest_hello, now).unwrap();
    guest.poll(now).unwrap();

    let received_guest = wait_admission(&mut owner, now);
    owner
        .verify_remote_admission(
            &received_guest,
            &verifier(owner_public, guest_public),
            ISSUER,
            SUBJECT,
            NOW_UNIX_MS,
            now,
        )
        .unwrap();
    let owner_hello = AdmissionHello::new(b"owner-ticket".to_vec(), JoinIntent::New).unwrap();
    owner.queue_admission(&owner_hello, now).unwrap();
    owner.authorize_remote(Role::Editor).unwrap();
    owner.activate(now).unwrap();
    owner.poll(now).unwrap();

    let received_owner = wait_admission(&mut guest, now);
    guest
        .verify_remote_admission(
            &received_owner,
            &verifier(owner_public, guest_public),
            ISSUER,
            SUBJECT,
            NOW_UNIX_MS,
            now,
        )
        .unwrap();
    guest.authorize_remote(Role::Owner).unwrap();
    guest.activate(now).unwrap();
    assert_eq!(
        owner.admission_state().phase(),
        op_collab_transport::AdmissionPhase::Active
    );
    assert_eq!(
        guest.admission_state().phase(),
        op_collab_transport::AdmissionPhase::Active
    );
}

#[test]
fn silent_peer_does_not_block_immediate_outbound_send() {
    let (mut owner, mut guest) = driver_pair(TransportConfig::default());
    let now = Instant::now();
    owner.queue_frame(&bye_frame(), now).unwrap();

    for _ in 0..16 {
        let polled = owner.poll(now).unwrap();
        assert!(polled.event.is_none());
        if !polled.has_pending_output {
            break;
        }
    }
    assert!(!owner.has_pending_output());

    let mut received = false;
    for _ in 0..1_000 {
        received |= take_frame(guest.poll(now).unwrap().event);
        if received {
            break;
        }
        std::thread::yield_now();
    }
    assert!(received);
}

#[test]
fn full_driver_queue_allows_same_key_replacement_with_exact_byte_delta() {
    let config = TransportConfig::default();
    let (mut owner, _guest) = driver_pair(config);
    let now = Instant::now();
    let initial = presence_update_frame(None);
    let replacement = presence_update_frame(Some("a-longer-editing-node-id"));
    let initial_len = EncodedFrameTransfer::encode(&initial, config.wire_limits)
        .unwrap()
        .encoded_len();
    let replacement_len = EncodedFrameTransfer::encode(&replacement, config.wire_limits)
        .unwrap()
        .encoded_len();
    assert!(replacement_len > initial_len);

    owner.queue_coalescing_frame(7, &initial, now).unwrap();
    for _ in 1..64 {
        owner.queue_frame(&bye_frame(), now).unwrap();
    }
    let bytes_before = owner.queued_bytes();
    assert_eq!(owner.queued_items(), 64);

    owner.queue_coalescing_frame(7, &replacement, now).unwrap();
    assert_eq!(owner.queued_items(), 64);
    assert_eq!(
        owner.queued_bytes(),
        bytes_before - initial_len + replacement_len
    );
}

#[test]
fn simultaneous_bidirectional_bursts_make_progress_without_a_noise_lock() {
    let (mut owner, mut guest) = driver_pair(TransportConfig::default());
    let now = Instant::now();
    const FRAMES: usize = 32;
    for _ in 0..FRAMES {
        owner.queue_frame(&bye_frame(), now).unwrap();
        guest.queue_frame(&bye_frame(), now).unwrap();
    }

    let mut owner_received = 0;
    let mut guest_received = 0;
    for _ in 0..2_000 {
        owner_received += usize::from(take_frame(owner.poll(now).unwrap().event));
        guest_received += usize::from(take_frame(guest.poll(now).unwrap().event));
        if owner_received == FRAMES && guest_received == FRAMES {
            break;
        }
    }
    assert_eq!(owner_received, FRAMES);
    assert_eq!(guest_received, FRAMES);
    assert!(!owner.has_pending_output());
    assert!(!guest.has_pending_output());
}

#[test]
fn multi_chunk_transfer_waits_for_rate_tokens_and_resumes() {
    let mut config = TransportConfig::default();
    config.rate.byte_burst = op_collab_transport::MAX_NOISE_PLAINTEXT_BYTES as u64;
    config.rate.bytes_per_second = op_collab_transport::MAX_NOISE_PLAINTEXT_BYTES as u64;
    config.rate.record_burst = 1;
    config.rate.records_per_second = 1;
    config.validate().unwrap();
    let (mut owner, mut guest) = driver_pair(config);
    let start = Instant::now() + Duration::from_secs(1);
    owner.queue_frame(&large_snapshot_frame(), start).unwrap();

    let first = owner.poll(start).unwrap();
    assert!(first.has_pending_output);
    assert!(first.waiting_for_rate);
    assert!(guest.poll(start).unwrap().event.is_none());

    let mut received = false;
    for second in 1..10 {
        let now = start + Duration::from_secs(second);
        owner.poll(now).unwrap();
        guest.wait_for_io(now, Duration::from_millis(50)).unwrap();
        received |= take_frame(guest.poll(now).unwrap().event);
        if received {
            break;
        }
    }
    assert!(received);
    assert!(!owner.has_pending_output());
}

#[test]
fn owner_rejects_guest_snapshot_from_the_first_authenticated_chunk() {
    let config = TransportConfig::default();
    let (mut owner, mut guest) = driver_pair(config);
    let now = Instant::now();
    guest.queue_frame(&large_snapshot_frame(), now).unwrap();

    for _ in 0..1_000 {
        guest.poll(now).unwrap();
        match owner.poll(now) {
            Err(RuntimeError::ForbiddenInboundClass(
                op_collab_transport::TransferClass::Snapshot,
            )) => return,
            Ok(_) => std::thread::yield_now(),
            Err(error) => panic!("unexpected owner failure: {error}"),
        }
    }
    panic!("owner did not reject peer-originated snapshot");
}

#[test]
fn authenticated_heartbeat_keeps_live_peer_and_inbound_idle_still_expires() {
    let config = TransportConfig::default();
    let (mut owner, mut guest) = driver_pair(config);
    let start = Instant::now();
    let heartbeat_at = start + config.timeouts.heartbeat;
    let owner_poll = owner.poll(heartbeat_at).unwrap();
    let guest_poll = guest.poll(heartbeat_at).unwrap();
    let owner_receive = owner.poll(heartbeat_at).unwrap();
    assert!(owner_poll.event.is_none());
    assert!(guest_poll.event.is_none());
    assert!(owner_receive.event.is_none());
    assert!(owner_poll.made_progress);
    assert!(guest_poll.made_progress);

    let (mut silent_side, _peer) = driver_pair(config);
    let silent_start = Instant::now();
    silent_side
        .poll(silent_start + config.timeouts.heartbeat)
        .unwrap();
    assert!(matches!(
        silent_side.poll(silent_start + config.timeouts.idle + Duration::from_millis(1)),
        Err(RuntimeError::IdleTimeout)
    ));
}

#[test]
fn renewal_deadline_fires_once_and_successful_renewal_rearms_it() {
    let config = TransportConfig::default();
    let (mut owner, _guest) = driver_pair_with_expiry(config, NOW_UNIX_MS + 100);
    let first_renewal = owner.ticket_renewal_at().unwrap();
    let first_expiry = owner.ticket_expiry_at().unwrap();
    assert_eq!(
        first_expiry.duration_since(first_renewal),
        Duration::from_millis(20)
    );
    assert_eq!(owner.next_deadline(), Some(first_renewal));
    assert!(!owner.ticket_renewal_due(first_renewal - Duration::from_millis(1)));
    assert!(owner.ticket_renewal_due(first_renewal));
    assert!(!owner.ticket_renewal_due(first_renewal));
    assert_eq!(owner.next_deadline(), Some(first_expiry));

    let remote_static = *owner.remote_static();
    let renewed = move |_: &[u8], expected: &[u8; 32], _: u64| {
        assert_eq!(*expected, remote_static);
        VerifiedTicketClaims::new(
            ISSUER.into(),
            SUBJECT.into(),
            GUEST_DEVICE.into(),
            remote_static,
            NOW_UNIX_MS + 300,
        )
    };
    owner
        .renew_ticket(
            &renewed,
            b"renewed-guest-ticket",
            NOW_UNIX_MS + 80,
            first_renewal,
        )
        .unwrap();

    let second_renewal = owner.ticket_renewal_at().unwrap();
    let second_expiry = owner.ticket_expiry_at().unwrap();
    assert_eq!(second_renewal, first_renewal + Duration::from_millis(176));
    assert_eq!(second_expiry, first_renewal + Duration::from_millis(220));
    assert_eq!(owner.next_deadline(), Some(second_renewal));
    assert!(!owner.ticket_renewal_due(first_renewal));
    assert!(owner.ticket_renewal_due(second_renewal));
}
