use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use crate::host::HeadlessCollabHost;
use op_collab::{
    canonical_document_hash, Bye, ByeReason, CollabMessage, ConnectionKey, Epoch, FrameEnvelope,
    GuestConnectionState, OpaqueTicket, ParticipantPresence, Point, Presence, Role, SessionId,
    VerifiedAuthMetadata,
};
use op_collab_transport::{encode_frame_transfer, m1_wire_limits, SharedQueueBudget};
use op_editor_core::{
    CollabAvailability, CollabConnectionPhase, CollabNoticeKind, CollabPanelHover,
    CollabPendingEditUi, CollabRejectUiCode, CollabTransportCapabilities, CollabUiAction,
    PenDocument,
};

use super::actor::{set_owner_ui, EditorActor, OwnerActor, PendingGuestAdmission};
use super::network::{
    guest_command_channel_with_capacity_for_test, owner_command_channel_with_capacity_for_test,
    Retirement,
};
use super::types::{
    CollabRuntimeFailure, GuestNetworkCommand, NetworkEvent, OwnerNetworkCommand, RemoteBye,
};
use super::CollabRuntime;

pub(super) const SESSION: &str = "desktop-collab-test";

pub(super) fn connection(raw: u64) -> ConnectionKey {
    ConnectionKey::new(raw).expect("non-zero connection")
}

pub(super) fn auth(index: usize) -> VerifiedAuthMetadata {
    VerifiedAuthMetadata {
        issuer: "https://issuer.example".into(),
        subject: format!("subject-{index}"),
        device_id: format!("device-{index}"),
        proof_binding: format!("binding-{index}"),
        expires_at_unix_ms: 10_000,
        display_name: None,
        avatar_url: None,
    }
}

pub(super) fn document_named(name: &str) -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "rectangle",
            "id": "shared-node",
            "name": name,
            "x": 0,
            "y": 0,
            "width": 20,
            "height": 20
        }]
    }))
    .unwrap()
}

#[test]
fn availability_refresh_clears_hover_from_the_previous_screen() {
    let mut runtime = CollabRuntime::new();
    let mut host = HeadlessCollabHost::new();
    let collab = &mut host.editor_state_mut().editor_ui.collab;
    collab.availability = CollabAvailability::Ready;
    collab.panel.hover = Some(CollabPanelHover::Start);

    assert!(runtime.refresh_availability(&mut host));
    assert_eq!(host.editor_state().editor_ui.collab.panel.hover, None);
}

#[test]
fn relay_only_capability_is_projected_and_rejects_injected_lan_actions() {
    let mut runtime = CollabRuntime::new();
    runtime.set_transport_capabilities(CollabTransportCapabilities::RELAY_AND_MANUAL_JOIN);
    let mut host = HeadlessCollabHost::new();

    assert!(runtime.refresh_availability(&mut host));
    assert_eq!(
        host.editor_state().editor_ui.collab.transport_capabilities,
        CollabTransportCapabilities::RELAY_AND_MANUAL_JOIN
    );

    host.editor_state_mut().editor_ui.collab.pending_action = Some(CollabUiAction::StartLan);
    assert!(runtime.drain_ui_action(&mut host));
    assert!(runtime.pending_network_launch.is_none());
    assert!(matches!(
        host.editor_state().editor_ui.collab.notice,
        Some(op_editor_core::CollabNotice {
            kind: CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported),
            ..
        })
    ));
}

#[test]
fn owner_ready_projects_share_address_and_relay_invite_after_authentication() {
    let mut runtime = CollabRuntime::new();
    let mut host = HeadlessCollabHost::new();
    let listener = "0.0.0.0:43120".parse().unwrap();
    let share = "192.168.1.20:43120".parse().unwrap();
    let invite = op_editor_core::CollabInviteCode::new("opc1_owner-only-route").unwrap();

    assert!(runtime.handle_event(
        NetworkEvent::OwnerReady {
            session_id: SessionId::from(SESSION),
            epoch: Epoch(1),
            endpoint: listener,
            share_endpoint: Some(share),
            local_auth: auth(0),
            invite: Some(invite.clone()),
            connection_path: op_editor_core::CollabConnectionPathUi::Relay {
                home_region: op_editor_core::CollabRelayRegion::China,
            },
        },
        &mut host,
    ));

    let session = host
        .editor_state()
        .editor_ui
        .collab
        .authenticated_session()
        .expect("authenticated owner session");
    assert_eq!(
        session
            .share_endpoint
            .as_ref()
            .map(op_editor_core::CollabShareEndpoint::as_str),
        Some("192.168.1.20:43120")
    );
    let public = host
        .editor_state()
        .editor_ui
        .collab
        .public_session()
        .expect("authenticated public session");
    assert_eq!(public.invite(), Some(&invite));
    assert_eq!(
        public.connection(),
        Some(op_editor_core::CollabConnectionPathUi::Relay {
            home_region: op_editor_core::CollabRelayRegion::China,
        })
    );
}

fn owner_runtime(
    capacity: usize,
) -> (
    CollabRuntime,
    HeadlessCollabHost,
    Receiver<OwnerNetworkCommand>,
    ConnectionKey,
) {
    let mut host = HeadlessCollabHost::new();
    host.editor_state_mut().doc = document_named("Before");
    let mut owner =
        OwnerActor::new(SessionId::from(SESSION), Epoch(1), auth(0), &mut host).unwrap();
    let peer = connection(2);
    let grant = owner.grant_new_peer(auth(1), Role::Editor).unwrap();
    owner.session.activate_peer(peer, grant, &host).unwrap();
    owner.connections.insert(peer);
    set_owner_ui(&mut host, &owner);

    let (network, commands) = owner_command_channel_with_capacity_for_test(capacity);
    let mut runtime = CollabRuntime::new();
    runtime.network = Some(network);
    runtime.actor = Some(EditorActor::Owner(Box::new(owner)));
    (runtime, host, commands, peer)
}

pub(super) fn guest_runtime(
    capacity: usize,
) -> (
    CollabRuntime,
    HeadlessCollabHost,
    Receiver<GuestNetworkCommand>,
    ConnectionKey,
    op_collab::Welcome,
) {
    let (runtime, host, commands, connection, welcome, _owner, _owner_host) =
        super::conflict_tests::guest_runtime_with_owner(capacity);
    (runtime, host, commands, connection, welcome)
}

#[test]
fn reliable_owner_delivery_failure_falls_back_to_standalone() {
    let (mut runtime, mut host, _commands, _) = owner_runtime(1);
    runtime
        .send_owner(OwnerNetworkCommand::Close {
            connection: connection(99),
        })
        .expect("fill owner command lane");
    assert!(runtime.begin_local_edit(&mut host));
    host.editor_state_mut().doc = document_named("Changed");

    assert_eq!(
        runtime.finish_local_edit(&mut host),
        crate::runtime::local_edit::LocalEditOutcome::Failed {
            document_rolled_back: false
        }
    );
    assert!(runtime.actor.is_none());
    assert!(runtime.network.is_none());
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Idle
    );
    assert_eq!(
        canonical_document_hash(&host.editor_state().doc).unwrap(),
        canonical_document_hash(&document_named("Changed")).unwrap()
    );
}

#[test]
fn commit_broadcast_reuses_one_encoded_allocation_across_peer_commands() {
    let (mut runtime, mut host, commands, first_peer) = owner_runtime(8);
    let second_peer = connection(3);
    let Some(EditorActor::Owner(owner)) = runtime.actor.as_mut() else {
        panic!("owner actor");
    };
    let grant = owner.grant_new_peer(auth(2), Role::Editor).unwrap();
    owner
        .session
        .activate_peer(second_peer, grant, &host)
        .unwrap();
    owner.connections.insert(second_peer);

    assert!(runtime.begin_local_edit(&mut host));
    host.editor_state_mut().doc = document_named("Shared encoded commit");
    assert!(!matches!(
        runtime.finish_local_edit(&mut host),
        crate::runtime::local_edit::LocalEditOutcome::Failed { .. }
    ));

    let mut queued = Vec::new();
    for _ in 0..2 {
        let OwnerNetworkCommand::Send {
            connection, frame, ..
        } = commands.recv_timeout(Duration::from_secs(1)).unwrap()
        else {
            panic!("commit broadcast queues one frame per peer");
        };
        queued.push((connection, frame));
    }
    queued.sort_by_key(|(connection, _)| connection.get());
    assert_eq!(queued[0].0, first_peer);
    assert_eq!(queued[1].0, second_peer);
    assert!(queued[0].1.shares_storage_with(&queued[1].1));
}

#[test]
fn reliable_guest_delivery_failure_becomes_disconnected_read_only() {
    let (mut runtime, mut host, _commands, _, _) = guest_runtime(1);
    runtime
        .network
        .as_ref()
        .unwrap()
        .send_guest(GuestNetworkCommand::VerifyRenewal(
            OpaqueTicket::new("renewal-command".to_owned()).unwrap(),
        ))
        .expect("fill guest command lane");

    runtime.handle_event(
        NetworkEvent::LocalTicketReady {
            ticket: OpaqueTicket::new("guest-renewal".to_string()).unwrap(),
        },
        &mut host,
    );

    let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
        panic!("guest actor must be retained for retry/fork");
    };
    assert_eq!(
        guest.session.core().state(),
        GuestConnectionState::Disconnected
    );
    assert!(runtime.network.is_none());
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Reconnecting
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.notice.unwrap().kind,
        CollabNoticeKind::Reject(CollabRejectUiCode::ResourceLimit)
    );
}

#[test]
fn fatal_owner_network_events_never_leave_active_ui() {
    for event in [
        NetworkEvent::Failed(CollabRuntimeFailure::Transport),
        NetworkEvent::Stopped,
    ] {
        let (mut runtime, mut host, _commands, _) = owner_runtime(4);
        runtime.handle_event(event, &mut host);
        assert!(runtime.actor.is_none());
        assert!(runtime.network.is_none());
        assert_eq!(
            host.editor_state().editor_ui.collab.phase,
            CollabConnectionPhase::Idle
        );
    }
}

#[test]
fn owner_local_auth_failure_survives_following_stopped_event() {
    let (mut runtime, mut host, _commands, _) = owner_runtime(4);
    runtime.handle_event(
        NetworkEvent::Failed(CollabRuntimeFailure::TicketRejected),
        &mut host,
    );
    runtime.handle_event(NetworkEvent::Stopped, &mut host);

    assert!(runtime.actor.is_none());
    assert!(runtime.network.is_none());
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Idle
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.notice.unwrap().kind,
        CollabNoticeKind::TicketExpired
    );
    assert!(runtime.next_reconnect_deadline().is_none());
}

#[test]
fn guest_local_auth_failure_survives_following_stopped_event() {
    let (mut runtime, mut host, _commands, _, _) = guest_runtime(4);
    runtime.handle_event(
        NetworkEvent::Failed(CollabRuntimeFailure::TicketRejected),
        &mut host,
    );
    runtime.handle_event(NetworkEvent::Stopped, &mut host);

    let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
        panic!("guest retains confirmed state for retry or fork");
    };
    assert_eq!(
        guest.session.core().state(),
        GuestConnectionState::Disconnected
    );
    assert!(runtime.network.is_none());
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Reconnecting
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.notice.unwrap().kind,
        CollabNoticeKind::TicketExpired
    );
    assert!(runtime.next_reconnect_deadline().is_none());
}

#[test]
fn unacknowledged_retired_generation_blocks_a_new_worker() {
    let mut runtime = CollabRuntime::new();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = released.recv();
    });
    runtime.retirement = Some(Retirement::start(vec![worker], std::sync::Arc::new(|| {})));

    assert_eq!(
        runtime.require_worker_slot().unwrap_err().failure,
        CollabRuntimeFailure::ResourceLimit
    );
    release.send(()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !runtime.reap_retirement() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(runtime.require_worker_slot().is_ok());
}

#[test]
fn avatar_completion_from_a_retired_session_generation_is_rejected() {
    let _guard = crate::lock_avatar_test_registry();
    let mut runtime = CollabRuntime::new();
    runtime.advance_generation();

    let key = "generation-avatar";
    let url = "https://cdn.example/generation-avatar.png";
    assert!(op_editor_ui::collab_avatar_runtime::register_collab_avatar_url(key, Some(url)));
    assert!(op_editor_ui::collab_avatar_runtime::collab_avatar_image(key).is_none());
    let stale = op_editor_ui::collab_avatar_runtime::take_collab_avatar_requests(1)
        .pop()
        .expect("first session queues an avatar");

    runtime.advance_generation();
    let mut png = vec![0; 32];
    png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    png[8..12].copy_from_slice(&13_u32.to_be_bytes());
    png[12..16].copy_from_slice(b"IHDR");
    png[16..20].copy_from_slice(&16_u32.to_be_bytes());
    png[20..24].copy_from_slice(&16_u32.to_be_bytes());
    assert!(
        !op_editor_ui::collab_avatar_runtime::complete_collab_avatar_request(
            &stale,
            Some(png.clone())
        ),
        "a late worker from the retired session cannot restore its bytes"
    );

    assert!(op_editor_ui::collab_avatar_runtime::register_collab_avatar_url(key, Some(url)));
    assert!(op_editor_ui::collab_avatar_runtime::collab_avatar_image(key).is_none());
    let current = op_editor_ui::collab_avatar_runtime::take_collab_avatar_requests(1)
        .pop()
        .expect("next session queues a fresh avatar");
    assert!(
        op_editor_ui::collab_avatar_runtime::complete_collab_avatar_request(&current, Some(png))
    );
    assert!(op_editor_ui::collab_avatar_runtime::collab_avatar_image(key).is_some());
}

#[test]
fn invalid_owner_frame_closes_only_the_offending_peer() {
    let (mut runtime, mut host, commands, peer) = owner_runtime(4);
    runtime.handle_event(
        NetworkEvent::Frame {
            connection: peer,
            frame: FrameEnvelope::new(
                SessionId::from(SESSION),
                Epoch(2),
                CollabMessage::Bye(Bye {
                    reason: ByeReason::Normal,
                }),
            ),
        },
        &mut host,
    );

    let Some(EditorActor::Owner(owner)) = runtime.actor.as_ref() else {
        panic!("owner remains active");
    };
    assert!(!owner.connections.contains(&peer));
    assert!(runtime.network.is_some());
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Active
    );
    assert!(matches!(
        commands.try_recv(),
        Ok(OwnerNetworkCommand::Close { connection }) if connection == peer
    ));
}

#[test]
fn owner_left_is_not_overwritten_by_transport_eof() {
    let (mut runtime, mut host, _commands, connection, _) = guest_runtime(4);
    runtime.handle_event(
        NetworkEvent::Frame {
            connection,
            frame: FrameEnvelope::new(
                SessionId::from(SESSION),
                Epoch(1),
                CollabMessage::Bye(Bye {
                    reason: ByeReason::OwnerLeft,
                }),
            ),
        },
        &mut host,
    );
    runtime.handle_event(
        NetworkEvent::ConnectionClosed {
            connection,
            failure: None,
            remote_bye: None,
        },
        &mut host,
    );

    let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
        panic!("ended guest actor");
    };
    assert_eq!(guest.session.core().state(), GuestConnectionState::Ended);
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Ended
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.notice.unwrap().kind,
        CollabNoticeKind::OwnerLeft
    );
}

#[test]
fn authentication_expired_is_not_overwritten_by_transport_eof() {
    let (mut runtime, mut host, _commands, connection, _) = guest_runtime(4);
    runtime.handle_event(
        NetworkEvent::ConnectionClosed {
            connection,
            failure: None,
            remote_bye: Some(RemoteBye {
                session_id: SessionId::from(SESSION),
                epoch: Epoch(1),
                reason: ByeReason::AuthenticationExpired,
            }),
        },
        &mut host,
    );
    runtime.handle_event(
        NetworkEvent::ConnectionClosed {
            connection,
            failure: None,
            remote_bye: None,
        },
        &mut host,
    );

    let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
        panic!("ended guest actor");
    };
    assert_eq!(guest.session.core().state(), GuestConnectionState::Ended);
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Ended
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.notice.unwrap().kind,
        CollabNoticeKind::TicketExpired
    );
}

#[test]
fn guest_projects_owner_presence() {
    let (mut runtime, mut host, _commands, connection, _) = guest_runtime(4);
    let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
        panic!("guest actor");
    };
    let owner = guest
        .session
        .core()
        .participants()
        .into_iter()
        .find(|participant| participant.role == Role::Owner)
        .unwrap();
    runtime.handle_event(
        NetworkEvent::Frame {
            connection,
            frame: FrameEnvelope::new(
                SessionId::from(SESSION),
                Epoch(1),
                CollabMessage::PresenceChanged(ParticipantPresence {
                    participant_id: owner.participant_id,
                    peer_id: owner.peer_id,
                    presence: Presence {
                        cursor: Some(Point { x: 12.0, y: 34.0 }),
                        selection: vec!["shared-node".to_string()],
                        viewport: None,
                        editing_node: None,
                    },
                }),
            ),
        },
        &mut host,
    );
    assert!(host
        .editor_state_mut()
        .editor_ui
        .collab
        .flush_presence(runtime.now_ms()));
    let presence = host.editor_state().editor_ui.collab.presence();
    assert_eq!(presence.len(), 1);
    assert_eq!(presence[0].cursor.unwrap().x, 12.0);
}

#[test]
fn ticket_rejection_on_close_has_specific_notice() {
    let (mut runtime, mut host, _commands, connection, _) = guest_runtime(4);
    runtime.handle_event(
        NetworkEvent::ConnectionClosed {
            connection,
            failure: Some(CollabRuntimeFailure::TicketRejected),
            remote_bye: None,
        },
        &mut host,
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Reconnecting
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.notice.unwrap().kind,
        CollabNoticeKind::TicketExpired
    );
}

#[test]
fn retry_against_new_epoch_ends_without_replaying_pending_edit() {
    let (mut runtime, mut host, commands, original_connection, welcome) = guest_runtime(8);
    assert!(runtime.begin_local_edit(&mut host));
    host.editor_state_mut().doc = document_named("Changed");
    assert!(!matches!(
        runtime.finish_local_edit(&mut host),
        crate::runtime::local_edit::LocalEditOutcome::Failed { .. }
    ));
    assert!(matches!(
        commands.recv_timeout(Duration::from_secs(1)).unwrap(),
        GuestNetworkCommand::Send {
            frame,
            ..
        } if matches!(frame.decode_for_test().body(), CollabMessage::Submit(_))
    ));
    let optimistic_hash = canonical_document_hash(&host.editor_state().doc).unwrap();

    runtime.handle_event(
        NetworkEvent::ConnectionClosed {
            connection: original_connection,
            failure: None,
            remote_bye: None,
        },
        &mut host,
    );
    runtime.wait_for_worker_slot_for_test();
    assert!(runtime.network.is_none());
    let (network, retry_commands) = guest_command_channel_with_capacity_for_test(8);
    runtime.network = Some(network);
    let retry_connection = connection(3);
    runtime.pending_guest = Some(PendingGuestAdmission {
        connection: retry_connection,
        session_id: SessionId::from(SESSION),
        epoch: Epoch(2),
    });
    runtime.handle_event(
        NetworkEvent::Frame {
            connection: retry_connection,
            frame: FrameEnvelope::new(
                SessionId::from(SESSION),
                Epoch(2),
                CollabMessage::Welcome(welcome),
            ),
        },
        &mut host,
    );

    let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
        panic!("terminal guest actor is retained");
    };
    assert_eq!(guest.session.core().state(), GuestConnectionState::Ended);
    assert!(guest.session.core().pending_edit().is_some());
    assert_eq!(
        host.editor_state().editor_ui.collab.pending_edit,
        CollabPendingEditUi::Submitting
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Ended
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.notice.unwrap().kind,
        CollabNoticeKind::EpochChanged
    );
    assert_eq!(
        canonical_document_hash(&host.editor_state().doc).unwrap(),
        optimistic_hash
    );
    while let Ok(command) = retry_commands.try_recv() {
        assert!(
            !matches!(
                command,
                GuestNetworkCommand::Send {
                    frame,
                    ..
                } if matches!(frame.decode_for_test().body(), CollabMessage::Submit(_))
            ),
            "old pending edit must not be proposed into a replacement epoch"
        );
    }
}

#[test]
fn outbound_bridge_budget_fails_reliable_and_drops_presence() {
    let (mut runtime, _host, commands, peer) = owner_runtime(8);
    runtime.bridge_budget = SharedQueueBudget::new(1).unwrap();
    let Some(EditorActor::Owner(owner)) = runtime.actor.take() else {
        panic!("owner actor");
    };

    let reliable = runtime.send_owner_actor_message(
        &owner,
        peer,
        CollabMessage::Bye(Bye {
            reason: ByeReason::Normal,
        }),
        None,
    );
    assert_eq!(
        reliable.unwrap_err().failure,
        CollabRuntimeFailure::ResourceLimit
    );
    assert_eq!(runtime.bridge_budget.used().unwrap(), 0);

    runtime
        .send_owner_actor_message(
            &owner,
            peer,
            CollabMessage::PresenceChanged(ParticipantPresence {
                participant_id: owner
                    .session
                    .core()
                    .active_participants()
                    .into_iter()
                    .find(|participant| participant.role == Role::Owner)
                    .unwrap()
                    .participant_id,
                peer_id: owner
                    .session
                    .core()
                    .active_participants()
                    .into_iter()
                    .find(|participant| participant.role == Role::Owner)
                    .unwrap()
                    .peer_id,
                presence: Presence {
                    cursor: None,
                    selection: Vec::new(),
                    viewport: None,
                    editing_node: None,
                },
            }),
            Some(1),
        )
        .expect("presence is lossy when bridge bytes are exhausted");
    assert_eq!(runtime.bridge_budget.used().unwrap(), 0);
    assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
    runtime.actor = Some(EditorActor::Owner(owner));
}

#[test]
fn inbound_bridge_reservation_is_held_through_gui_frame_handling() {
    let (mut runtime, mut host, commands, peer) = owner_runtime(8);
    let presence = Presence {
        cursor: Some(Point { x: 5.0, y: 8.0 }),
        selection: vec!["shared-node".to_string()],
        viewport: None,
        editing_node: None,
    };
    let (session_id, epoch, participant) = {
        let Some(EditorActor::Owner(owner)) = runtime.actor.as_ref() else {
            panic!("owner actor");
        };
        (
            owner.session.core().session_id().clone(),
            owner.session.core().epoch(),
            owner
                .session
                .core()
                .active_participants()
                .into_iter()
                .find(|participant| participant.role == Role::Editor)
                .unwrap(),
        )
    };
    let inbound = FrameEnvelope::new(
        session_id.clone(),
        epoch,
        CollabMessage::PresenceUpdate(presence.clone()),
    );
    let outbound = FrameEnvelope::new(
        session_id,
        epoch,
        CollabMessage::PresenceChanged(ParticipantPresence {
            participant_id: participant.participant_id,
            peer_id: participant.peer_id,
            presence,
        }),
    );
    let inbound_len = encode_frame_transfer(&inbound, m1_wire_limits())
        .unwrap()
        .1
        .len();
    let outbound_len = encode_frame_transfer(&outbound, m1_wire_limits())
        .unwrap()
        .1
        .len();
    runtime.bridge_budget = SharedQueueBudget::new(inbound_len + outbound_len - 1).unwrap();
    let sink = runtime.event_sink();

    sink.try_send_sized(
        NetworkEvent::Frame {
            connection: peer,
            frame: inbound,
        },
        inbound_len,
        false,
    )
    .unwrap();
    assert_eq!(runtime.bridge_budget.used().unwrap(), inbound_len);

    assert!(runtime.poll(&mut host));
    assert_eq!(runtime.bridge_budget.used().unwrap(), 0);
    assert!(
        matches!(commands.try_recv(), Err(TryRecvError::Empty)),
        "presence broadcast must see the inbound reservation and drop"
    );
}
