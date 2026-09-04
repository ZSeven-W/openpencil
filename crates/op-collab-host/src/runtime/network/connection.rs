use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use op_collab::{Bye, ByeReason, CollabMessage, ConnectionKey, Epoch, SessionId};
use op_collab_transport::{
    AdmissionError, ConnectionDriver, DriverEvent, InboundTransferPolicy, RuntimeError,
    SharedQueueBudget,
};

use super::super::auth::{
    unix_time_ms, LocalAdmission, LocalTicketRenewer, ProductionTicketVerifier,
};
use super::super::types::{
    CollabRuntimeFailure, GuestNetworkCommand, NetworkEvent, PeerNetworkCommand, RemoteBye,
    TerminalNetworkEvent,
};
use super::connection_queue::queue_command_frame;
use super::shutdown::{retirement_ready, TerminalDrain};
use super::{EventSendError, EventSink};

const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(super) struct DriverIdentity {
    pub(super) connection: ConnectionKey,
    pub(super) session_id: SessionId,
    pub(super) epoch: Epoch,
}

pub(super) struct DriverControl<C> {
    pub(super) commands: Receiver<C>,
    pub(super) shutdown: Receiver<ByeReason>,
}

pub(super) struct GuestRenewalContext {
    pub(super) verifier: Arc<ProductionTicketVerifier>,
    pub(super) renewer: LocalTicketRenewer,
    pub(super) admission: Arc<RwLock<LocalAdmission>>,
}

pub(super) fn drive_owner_peer(
    connection: op_collab_transport::SecureConnection<std::net::TcpStream>,
    shared_budget: SharedQueueBudget,
    identity: DriverIdentity,
    control: DriverControl<PeerNetworkCommand>,
    verifier: Arc<ProductionTicketVerifier>,
    sink: &EventSink,
) -> Option<CollabRuntimeFailure> {
    let DriverIdentity {
        connection: connection_id,
        session_id,
        epoch,
    } = identity;
    let DriverControl { commands, shutdown } = control;
    let mut driver = match ConnectionDriver::new(
        connection,
        shared_budget,
        InboundTransferPolicy::PeerToOwner,
    ) {
        Ok(driver) => driver,
        Err(error) => return Some(runtime_failure(&error)),
    };
    let mut stop_requested = false;
    let mut terminal = None;
    loop {
        if terminal.is_none() {
            match shutdown.try_recv() {
                Ok(reason) => {
                    terminal = Some(TerminalDrain::new(
                        session_id.clone(),
                        epoch,
                        reason,
                        Instant::now(),
                    ));
                }
                Err(TryRecvError::Disconnected) => stop_requested = true,
                Err(TryRecvError::Empty) => {}
            }
        }
        while terminal.is_none() && !stop_requested {
            match commands.try_recv() {
                Ok(PeerNetworkCommand::Send {
                    frame,
                    coalesce_key,
                }) => {
                    let lossy_presence = frame.is_lossy_presence();
                    let (encoded, bridge_reservation) = (*frame).into_parts();
                    let result = queue_command_frame(
                        &mut driver,
                        encoded,
                        coalesce_key,
                        lossy_presence,
                        Instant::now(),
                    );
                    drop(bridge_reservation);
                    if let Err(error) = result {
                        return Some(runtime_failure(&error));
                    }
                }
                Ok(PeerNetworkCommand::VerifyRenewal(ticket)) => {
                    let now_unix_ms = match unix_time_ms() {
                        Ok(now) => now,
                        Err(error) => return Some(error.failure),
                    };
                    match driver.renew_ticket(
                        verifier.as_ref(),
                        ticket.expose().as_bytes(),
                        now_unix_ms,
                        Instant::now(),
                    ) {
                        Ok(identity) => {
                            if let Err(failure) = send_reliable(
                                sink,
                                NetworkEvent::RenewalVerified {
                                    connection: connection_id,
                                    auth: identity.to_auth_metadata(),
                                },
                            ) {
                                return failure;
                            }
                        }
                        Err(error) => return Some(runtime_failure(&error)),
                    }
                }
                Ok(PeerNetworkCommand::Authorize(_)) => {
                    return Some(CollabRuntimeFailure::Protocol);
                }
                Ok(PeerNetworkCommand::Stop) => {
                    stop_requested = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    stop_requested = true;
                    break;
                }
            }
        }
        let now = Instant::now();
        if let Some(drain) = terminal.as_mut() {
            if let Err(error) = drain.try_queue(&mut driver, now) {
                return Some(runtime_failure(&error));
            }
            if drain.complete(&driver, now) {
                return None;
            }
        } else if retirement_ready(stop_requested, driver.has_pending_output()) {
            return None;
        }
        // The peer renews proactively on its own local schedule. Marking this
        // one-shot deadline means the transport now waits only until expiry;
        // a verified RenewTicket rearms both deadlines.
        let _ = driver.ticket_renewal_due(now);
        match driver.poll(now) {
            Ok(poll) => {
                let made_progress = poll.made_progress;
                let rate_ready_at = poll.rate_ready_at;
                if terminal.is_none() && !stop_requested {
                    if let Some(event) = poll.event {
                        match event {
                            DriverEvent::Frame { frame, encoded_len } => {
                                if let Err(failure) =
                                    send_frame(sink, connection_id, frame, encoded_len)
                                {
                                    return failure;
                                }
                            }
                            DriverEvent::Admission(_) => {
                                return Some(CollabRuntimeFailure::Protocol);
                            }
                        }
                    }
                }
                if !made_progress {
                    let terminal_deadline = terminal.as_ref().map(TerminalDrain::deadline);
                    if let Err(error) =
                        wait_for_progress(&mut driver, rate_ready_at, terminal_deadline)
                    {
                        return Some(runtime_failure(&error));
                    }
                }
            }
            Err(error) => return Some(runtime_failure(&error)),
        }
    }
}

pub(super) fn drive_guest(
    connection: op_collab_transport::SecureConnection<std::net::TcpStream>,
    shared_budget: SharedQueueBudget,
    identity: DriverIdentity,
    control: DriverControl<GuestNetworkCommand>,
    renewal: GuestRenewalContext,
    sink: &EventSink,
) -> Option<CollabRuntimeFailure> {
    let DriverIdentity {
        connection: connection_id,
        session_id,
        epoch,
    } = identity;
    let DriverControl { commands, shutdown } = control;
    let GuestRenewalContext {
        verifier,
        mut renewer,
        admission: local_admission,
    } = renewal;
    let mut driver = match ConnectionDriver::new(
        connection,
        shared_budget,
        InboundTransferPolicy::OwnerToGuest,
    ) {
        Ok(driver) => driver,
        Err(error) => return Some(runtime_failure(&error)),
    };
    let mut stop_requested = false;
    let mut terminal = None;
    let mut terminal_failure = None;
    loop {
        if terminal.is_none() {
            match shutdown.try_recv() {
                Ok(reason) => {
                    terminal = Some(TerminalDrain::new(
                        session_id.clone(),
                        epoch,
                        reason,
                        Instant::now(),
                    ));
                }
                Err(TryRecvError::Disconnected) => stop_requested = true,
                Err(TryRecvError::Empty) => {}
            }
        }
        if terminal.is_none() && !stop_requested {
            match renewer.poll(Instant::now()) {
                Ok(Some(renewed)) => {
                    let ticket = match renewed.renewal_ticket() {
                        Ok(ticket) => ticket,
                        Err(error) => return Some(error.failure),
                    };
                    if let Err(error) =
                        LocalAdmission::install_shared_relay_renewal(&local_admission, renewed)
                    {
                        return Some(error.failure);
                    }
                    if let Err(failure) =
                        send_reliable(sink, NetworkEvent::LocalTicketReady { ticket })
                    {
                        return failure;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    terminal_failure = Some(error.failure);
                    terminal = Some(TerminalDrain::new(
                        session_id.clone(),
                        epoch,
                        ByeReason::AuthenticationExpired,
                        Instant::now(),
                    ));
                }
            }
        }
        while terminal.is_none() && !stop_requested {
            match commands.try_recv() {
                Ok(GuestNetworkCommand::Send {
                    frame,
                    coalesce_key,
                }) => {
                    let lossy_presence = frame.is_lossy_presence();
                    let (encoded, bridge_reservation) = (*frame).into_parts();
                    let result = queue_command_frame(
                        &mut driver,
                        encoded,
                        coalesce_key,
                        lossy_presence,
                        Instant::now(),
                    );
                    drop(bridge_reservation);
                    if let Err(error) = result {
                        return Some(runtime_failure(&error));
                    }
                }
                // The confirmation gate ran before this loop was reachable, so
                // a decision arriving here is a duplicate click on a prompt
                // that is already answered. Drop it: it must not be able to
                // re-open or re-decide an admitted connection.
                Ok(GuestNetworkCommand::OwnerIdentityDecision(_)) => {}
                Ok(GuestNetworkCommand::VerifyRenewal(ticket)) => {
                    let now_unix_ms = match unix_time_ms() {
                        Ok(now) => now,
                        Err(error) => return Some(error.failure),
                    };
                    if let Err(error) = driver.renew_ticket(
                        verifier.as_ref(),
                        ticket.expose().as_bytes(),
                        now_unix_ms,
                        Instant::now(),
                    ) {
                        return Some(runtime_failure(&error));
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    stop_requested = true;
                    break;
                }
            }
        }
        let now = Instant::now();
        if let Some(drain) = terminal.as_mut() {
            if let Err(error) = drain.try_queue(&mut driver, now) {
                return terminal_failure.or_else(|| Some(runtime_failure(&error)));
            }
            if drain.complete(&driver, now) {
                return terminal_failure;
            }
        } else if retirement_ready(stop_requested, driver.has_pending_output()) {
            return None;
        }
        let _ = driver.ticket_renewal_due(now);
        match driver.poll(now) {
            Ok(poll) => {
                let made_progress = poll.made_progress;
                let rate_ready_at = poll.rate_ready_at;
                if terminal.is_none() && !stop_requested {
                    if let Some(event) = poll.event {
                        match event {
                            DriverEvent::Frame { frame, encoded_len } => {
                                if let Err(failure) =
                                    send_frame(sink, connection_id, frame, encoded_len)
                                {
                                    return failure;
                                }
                            }
                            DriverEvent::Admission(_) => {
                                return Some(CollabRuntimeFailure::Protocol);
                            }
                        }
                    }
                }
                if !made_progress {
                    let terminal_deadline = terminal.as_ref().map(TerminalDrain::deadline);
                    if let Err(error) =
                        wait_for_progress(&mut driver, rate_ready_at, terminal_deadline)
                    {
                        return terminal_failure.or_else(|| Some(runtime_failure(&error)));
                    }
                }
            }
            Err(error) => return terminal_failure.or_else(|| Some(runtime_failure(&error))),
        }
    }
}

fn send_frame(
    sink: &EventSink,
    connection: ConnectionKey,
    frame: op_collab::FrameEnvelope,
    encoded_len: usize,
) -> Result<(), Option<CollabRuntimeFailure>> {
    if let CollabMessage::Bye(Bye { reason }) = frame.body() {
        let _ = sink.send_terminal(TerminalNetworkEvent::ConnectionClosed {
            connection,
            failure: None,
            remote_bye: Some(RemoteBye {
                session_id: frame.session_id().clone(),
                epoch: frame.epoch(),
                reason: *reason,
            }),
        });
        return Err(None);
    }
    let lossy = super::super::types::is_lossy_presence_frame(&frame);
    match sink.try_send_sized(
        NetworkEvent::Frame { connection, frame },
        encoded_len,
        lossy,
    ) {
        Ok(()) => Ok(()),
        Err(EventSendError::Full) if lossy => Ok(()),
        Err(EventSendError::Full) => Err(Some(CollabRuntimeFailure::ResourceLimit)),
        Err(EventSendError::Disconnected) => Err(None),
    }
}

fn send_reliable(
    sink: &EventSink,
    event: NetworkEvent,
) -> Result<(), Option<CollabRuntimeFailure>> {
    match sink.try_send(event) {
        Ok(()) => Ok(()),
        Err(EventSendError::Full) => Err(Some(CollabRuntimeFailure::ResourceLimit)),
        Err(EventSendError::Disconnected) => Err(None),
    }
}

fn wait_for_progress(
    driver: &mut ConnectionDriver,
    rate_ready_at: Option<Instant>,
    terminal_deadline: Option<Instant>,
) -> Result<(), RuntimeError> {
    let now = Instant::now();
    let command_deadline = now
        .checked_add(COMMAND_POLL_INTERVAL)
        .unwrap_or(now + Duration::from_millis(1));
    let deadline = [
        Some(command_deadline),
        driver.next_deadline(),
        rate_ready_at,
        terminal_deadline,
    ]
    .into_iter()
    .flatten()
    .min()
    .unwrap_or(command_deadline);
    driver.wait_for_io(now, deadline.saturating_duration_since(now))
}

pub(super) fn runtime_failure(error: &RuntimeError) -> CollabRuntimeFailure {
    match error {
        RuntimeError::Admission(
            AdmissionError::Verification
            | AdmissionError::WrongIssuer
            | AdmissionError::WrongSubject
            | AdmissionError::StaticKeyMismatch
            | AdmissionError::Expired
            | AdmissionError::TicketExpired
            | AdmissionError::RenewalIdentityChanged
            | AdmissionError::RenewalDidNotExtend,
        ) => CollabRuntimeFailure::TicketRejected,
        RuntimeError::Queue(_) | RuntimeError::RateLimited => CollabRuntimeFailure::ResourceLimit,
        RuntimeError::Frame(_)
        | RuntimeError::Prelude(_)
        | RuntimeError::ForbiddenInboundClass(_) => CollabRuntimeFailure::Protocol,
        _ => CollabRuntimeFailure::Transport,
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::net::TcpListener;
    use std::sync::mpsc::TryRecvError;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use op_collab::{
        Bye, ByeReason, CollabMessage, ConnectionKey, Epoch, FrameEnvelope, Role, SessionId,
    };
    use op_collab_transport::{
        accept_secure_tcp, connect_secure_tcp, AdmissionHello, AdmissionPhase, ConnectionDriver,
        DeviceStaticKey, DriverEvent, InboundTransferPolicy, JoinIntent, PeerIdentityPolicy,
        ServerPrelude, SharedQueueBudget, TransportConfig, VerifiedTicketClaims,
    };

    use super::super::shutdown::{retirement_ready, TerminalDrain};
    use super::super::{event_channel_with_capacity, EventSink};
    use super::send_frame;
    use crate::runtime::types::NetworkEvent;

    const NOW_UNIX_MS: u64 = 1_000;
    const ISSUER: &str = "https://issuer.example";
    const SUBJECT: &str = "00000000-0000-0000-0000-000000000001";
    const OWNER_DEVICE: &str = "00000000-0000-0000-0000-000000000002";
    const GUEST_DEVICE: &str = "00000000-0000-0000-0000-000000000003";

    fn initial_verifier(
        owner_static: [u8; 32],
        guest_static: [u8; 32],
        expires_at_unix_ms: u64,
    ) -> impl op_collab_transport::TicketVerifier {
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
                ISSUER.to_owned(),
                SUBJECT.to_owned(),
                device.to_owned(),
                static_key,
                expires_at_unix_ms,
            )
        }
    }

    pub(in crate::runtime::network) fn admitted_pair(
        expires_at_unix_ms: u64,
    ) -> (
        op_collab_transport::ConnectionDriver,
        op_collab_transport::ConnectionDriver,
        [u8; 32],
        [u8; 32],
    ) {
        let config = TransportConfig::default();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let endpoint = listener.local_addr().expect("local address");
        let owner_key = DeviceStaticKey::from_private([22_u8; 32]).expect("owner key");
        let guest_key = DeviceStaticKey::from_private([11_u8; 32]).expect("guest key");
        let owner_static = *owner_key.public_key();
        let guest_static = *guest_key.public_key();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let prelude = ServerPrelude::new(
                "00112233445566778899aabbccddeeff".to_owned(),
                SessionId::from("session"),
                Epoch(1),
            )
            .expect("prelude");
            let mut connection =
                accept_secure_tcp(stream, &owner_key, &prelude, config).expect("secure accept");
            let local =
                AdmissionHello::new(b"owner-ticket".to_vec(), JoinIntent::New).expect("hello");
            connection
                .exchange_admission_responder(
                    &local,
                    &initial_verifier(owner_static, guest_static, expires_at_unix_ms),
                    ISSUER,
                    PeerIdentityPolicy::SameAccount { subject: SUBJECT },
                    NOW_UNIX_MS,
                    Instant::now(),
                )
                .expect("owner admission");
            connection
                .authorize_remote(Role::Editor)
                .expect("authorize guest");
            connection.activate(Instant::now()).expect("activate owner");
            connection
        });
        let (_, mut guest) = connect_secure_tcp(
            endpoint,
            &guest_key,
            Some("00112233445566778899aabbccddeeff"),
            config,
        )
        .expect("secure connect");
        let local = AdmissionHello::new(b"guest-ticket".to_vec(), JoinIntent::New).expect("hello");
        guest
            .exchange_admission_initiator(
                &local,
                &initial_verifier(owner_static, guest_static, expires_at_unix_ms),
                ISSUER,
                PeerIdentityPolicy::SameAccount { subject: SUBJECT },
                NOW_UNIX_MS,
                Instant::now(),
            )
            .expect("guest admission");
        guest
            .authorize_remote(Role::Owner)
            .expect("authorize owner");
        guest.activate(Instant::now()).expect("activate guest");
        let owner = server.join().expect("owner thread");
        let budget =
            SharedQueueBudget::new(config.connections.global_queued_bytes).expect("queue budget");
        (
            op_collab_transport::ConnectionDriver::new(
                owner,
                budget.clone(),
                InboundTransferPolicy::PeerToOwner,
            )
            .expect("owner driver"),
            op_collab_transport::ConnectionDriver::new(
                guest,
                budget,
                InboundTransferPolicy::OwnerToGuest,
            )
            .expect("guest driver"),
            owner_static,
            guest_static,
        )
    }

    pub(in crate::runtime::network) fn bye_frame(reason: ByeReason) -> FrameEnvelope {
        FrameEnvelope::new(
            SessionId::from("session"),
            Epoch(1),
            CollabMessage::Bye(Bye { reason }),
        )
    }

    fn poll_until_bye(
        sender: &mut ConnectionDriver,
        receiver: &mut ConnectionDriver,
        expected: ByeReason,
    ) {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let now = Instant::now();
            sender.poll(now).expect("sender poll");
            if let Some(DriverEvent::Frame { frame, .. }) =
                receiver.poll(now).expect("receiver poll").event
            {
                if matches!(
                    frame.body(),
                    CollabMessage::Bye(Bye { reason }) if *reason == expected
                ) {
                    return;
                }
            }
            assert!(Instant::now() < deadline, "terminal frame must arrive");
            std::thread::yield_now();
        }
    }

    #[test]
    fn both_drivers_renew_proactively_and_outlive_the_initial_expiry() {
        let initial_expiry = NOW_UNIX_MS + 100;
        let (mut owner, mut guest, owner_static, guest_static) = admitted_pair(initial_expiry);
        let owner_old_expiry = owner.ticket_expiry_at().expect("owner expiry");
        let guest_old_expiry = guest.ticket_expiry_at().expect("guest expiry");
        let owner_renewal = owner.ticket_renewal_at().expect("owner renewal");
        let guest_renewal = guest.ticket_renewal_at().expect("guest renewal");
        assert!(owner.ticket_renewal_due(owner_renewal));
        assert!(guest.ticket_renewal_due(guest_renewal));

        let renewed = move |ticket: &[u8], expected: &[u8; 32], _now: u64| {
            let (static_key, device) = match ticket {
                b"renewed-owner-ticket" => (owner_static, OWNER_DEVICE),
                b"renewed-guest-ticket" => (guest_static, GUEST_DEVICE),
                _ => return Err(op_collab_transport::AdmissionError::Verification),
            };
            if static_key != *expected {
                return Err(op_collab_transport::AdmissionError::StaticKeyMismatch);
            }
            VerifiedTicketClaims::new(
                ISSUER.to_owned(),
                SUBJECT.to_owned(),
                device.to_owned(),
                static_key,
                NOW_UNIX_MS + 400,
            )
        };
        owner
            .renew_ticket(
                &renewed,
                b"renewed-guest-ticket",
                NOW_UNIX_MS + 80,
                owner_renewal,
            )
            .expect("renew guest on owner");
        guest
            .renew_ticket(
                &renewed,
                b"renewed-owner-ticket",
                NOW_UNIX_MS + 80,
                guest_renewal,
            )
            .expect("renew owner on guest");

        assert!(owner.ticket_expiry_at().expect("new owner expiry") > owner_old_expiry);
        assert!(guest.ticket_expiry_at().expect("new guest expiry") > guest_old_expiry);
        let after_old_expiry = owner_old_expiry.max(guest_old_expiry) + Duration::from_millis(1);
        owner.poll(after_old_expiry).expect("owner stays live");
        guest.poll(after_old_expiry).expect("guest stays live");
        assert_eq!(owner.admission_state().phase(), AdmissionPhase::Active);
        assert_eq!(guest.admission_state().phase(), AdmissionPhase::Active);
    }

    #[test]
    fn active_terminal_drain_retries_after_a_full_driver_queue() {
        let (mut owner, mut guest, _, _) = admitted_pair(NOW_UNIX_MS + 10_000);
        for _ in 0..TransportConfig::default().connections.outbound_queue_items {
            owner
                .queue_frame(&bye_frame(ByeReason::Normal), Instant::now())
                .unwrap();
        }
        let mut drain = TerminalDrain::new(
            SessionId::from("session"),
            Epoch(1),
            ByeReason::OwnerLeft,
            Instant::now(),
        );
        drain.try_queue(&mut owner, Instant::now()).unwrap();
        assert!(!drain.queued());

        let deadline = Instant::now() + Duration::from_secs(1);
        while !drain.queued() {
            let now = Instant::now();
            owner.poll(now).unwrap();
            let _ = guest.poll(now).unwrap();
            drain.try_queue(&mut owner, now).unwrap();
            assert!(Instant::now() < deadline);
        }
        poll_until_bye(&mut owner, &mut guest, ByeReason::OwnerLeft);
    }

    #[test]
    fn reserved_pre_expiry_window_can_queue_and_flush_authentication_expired() {
        let (mut owner, mut guest, _, _) = admitted_pair(NOW_UNIX_MS + 5_000);
        let now = Instant::now();
        assert!(
            owner
                .ticket_expiry_at()
                .unwrap()
                .saturating_duration_since(now)
                >= super::super::TERMINAL_FLUSH_TIMEOUT
        );
        let mut drain = TerminalDrain::new(
            SessionId::from("session"),
            Epoch(1),
            ByeReason::AuthenticationExpired,
            now,
        );
        drain.try_queue(&mut owner, now).unwrap();
        assert!(drain.queued());
        poll_until_bye(&mut owner, &mut guest, ByeReason::AuthenticationExpired);
    }

    #[test]
    fn disconnected_command_receiver_drains_prequeued_driver_output() {
        let (mut owner, mut guest, _, _) = admitted_pair(NOW_UNIX_MS + 10_000);
        owner
            .queue_frame(&bye_frame(ByeReason::Normal), Instant::now())
            .unwrap();
        let (commands, receiver) = std::sync::mpsc::sync_channel::<()>(1);
        drop(commands);
        let stop_requested = matches!(receiver.try_recv(), Err(TryRecvError::Disconnected));
        assert!(!retirement_ready(
            stop_requested,
            owner.has_pending_output()
        ));

        poll_until_bye(&mut owner, &mut guest, ByeReason::Normal);
        assert!(retirement_ready(stop_requested, owner.has_pending_output()));
    }

    #[test]
    fn inbound_owner_left_bypasses_a_full_normal_gui_lane() {
        let (sender, normal, terminal, terminal_receiver) = event_channel_with_capacity(1, 1);
        let sink = EventSink::new(
            sender,
            terminal,
            SharedQueueBudget::new(1024).unwrap(),
            Arc::new(|| {}),
            7,
        );
        sink.try_send(NetworkEvent::Stopped).unwrap();
        let connection = ConnectionKey::new(1).unwrap();
        assert_eq!(
            send_frame(&sink, connection, bye_frame(ByeReason::OwnerLeft), 1),
            Err(None)
        );

        assert!(matches!(
            normal.recv().unwrap().event,
            NetworkEvent::Stopped
        ));
        let terminal = terminal_receiver.try_recv().unwrap();
        assert!(matches!(
            terminal.event,
            NetworkEvent::ConnectionClosed {
                connection: received_connection,
                failure: None,
                remote_bye: Some(remote_bye),
            } if received_connection == connection
                && remote_bye.reason == ByeReason::OwnerLeft
                && remote_bye.session_id == SessionId::from("session")
                && remote_bye.epoch == Epoch(1)
        ));
    }
}
