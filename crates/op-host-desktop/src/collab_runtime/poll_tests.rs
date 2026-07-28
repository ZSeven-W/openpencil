use op_collab::{
    canonical_document_hash, CollabMessage, ConnectionKey, Epoch, FrameEnvelope,
    GuestConnectionState, Role, SessionId, VerifiedAuthMetadata,
};
use op_collab_transport::JoinIntent;
use op_editor_core::{CollabConnectionPhase, CollabNoticeKind, PenDocument};
use op_host_native::WidgetHostNative;

use super::actor::{set_guest_ui, set_owner_ui, EditorActor, GuestActor, OwnerActor};
use super::network::{
    guest_command_channel_with_capacity_for_test, owner_command_channel_with_capacity_for_test,
};
use super::types::{CollabRuntimeFailure, CollabStatusEvent, NetworkEvent, TerminalNetworkEvent};
use super::DesktopCollabRuntime;

const SESSION: &str = "desktop-collab-poll-race";

fn connection(raw: u64) -> ConnectionKey {
    ConnectionKey::new(raw).expect("non-zero connection")
}

fn auth(index: usize) -> VerifiedAuthMetadata {
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

fn document_named(name: &str) -> PenDocument {
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
fn terminal_observation_drains_causal_data_before_retiring_generation() {
    let remote_document = document_named("Remote authoritative");
    let remote_hash = canonical_document_hash(&remote_document).unwrap();
    let mut owner_host = WidgetHostNative::new();
    owner_host.editor_state_mut().doc = remote_document;
    let mut owner =
        OwnerActor::new(SessionId::from(SESSION), Epoch(1), auth(0), &mut owner_host).unwrap();
    let guest_connection = connection(2);
    let grant = owner.grant_new_peer(auth(1), Role::Editor).unwrap();
    let activation = owner
        .session
        .activate_peer(guest_connection, grant, &owner_host)
        .unwrap();

    let mut host = WidgetHostNative::new();
    host.editor_state_mut().doc = document_named("Local before join");
    host.editor_state_mut().editor_ui.collab.phase = CollabConnectionPhase::Joining;
    let (network, _commands) = guest_command_channel_with_capacity_for_test(8);
    let mut runtime = DesktopCollabRuntime::new();
    runtime.network = Some(network);
    let starting_generation = runtime.generation;
    let sink = runtime.event_sink();
    let session_id = SessionId::from(SESSION);

    assert!(
        runtime.poll_with_after_initial_data_drain(&mut host, move || {
            sink.try_send(NetworkEvent::GuestAuthenticated {
                connection: guest_connection,
                session_id: session_id.clone(),
                epoch: Epoch(1),
                remote_static: [9; 32],
            })
            .unwrap();
            sink.try_send(NetworkEvent::Frame {
                connection: guest_connection,
                frame: FrameEnvelope::new(
                    session_id.clone(),
                    Epoch(1),
                    CollabMessage::Welcome(activation.welcome),
                ),
            })
            .unwrap();
            sink.try_send(NetworkEvent::Frame {
                connection: guest_connection,
                frame: FrameEnvelope::new(
                    session_id,
                    Epoch(1),
                    CollabMessage::Snapshot(Box::new(activation.snapshot.unwrap())),
                ),
            })
            .unwrap();
            assert!(sink.send_terminal(TerminalNetworkEvent::ConnectionClosed {
                connection: guest_connection,
                failure: Some(CollabRuntimeFailure::Transport),
                remote_bye: None,
            }));
            assert!(sink.send_terminal(TerminalNetworkEvent::Stopped));
        })
    );

    assert_eq!(
        canonical_document_hash(&host.editor_state().doc).unwrap(),
        remote_hash,
        "the causally-prior Snapshot must install before terminal retirement"
    );
    assert!(runtime.network.is_none());
    assert_ne!(runtime.generation, starting_generation);
    let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
        panic!("the disconnected guest actor is retained for Retry");
    };
    assert_eq!(
        guest.session.core().state(),
        GuestConnectionState::Disconnected
    );
    assert_eq!(
        host.editor_state().editor_ui.collab.phase,
        CollabConnectionPhase::Reconnecting
    );

    let settled_hash = canonical_document_hash(&host.editor_state().doc).unwrap();
    runtime.poll(&mut host);
    assert_eq!(
        canonical_document_hash(&host.editor_state().doc).unwrap(),
        settled_hash,
        "no stale Snapshot may install after terminal retirement"
    );
    assert!(runtime.network.is_none());
    assert!(!matches!(
        runtime.actor.as_ref(),
        Some(EditorActor::Guest(guest))
            if guest.session.core().state() == GuestConnectionState::Active
    ));
}

#[test]
fn cached_old_generation_terminal_events_are_rechecked_before_handling() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().doc = document_named("Owner document");
    let owner = OwnerActor::new(SessionId::from(SESSION), Epoch(1), auth(0), &mut host).unwrap();
    set_owner_ui(&mut host, &owner);
    let (network, _commands) = owner_command_channel_with_capacity_for_test(8);
    let mut runtime = DesktopCollabRuntime::new();
    runtime.network = Some(network);
    runtime.actor = Some(EditorActor::Owner(Box::new(owner)));
    let sink = runtime.event_sink();

    runtime.poll_with_after_initial_data_drain(&mut host, move || {
        assert!(sink.send_terminal(TerminalNetworkEvent::Failed(
            CollabRuntimeFailure::Transport,
        )));
        assert!(sink.send_terminal(TerminalNetworkEvent::Failed(
            CollabRuntimeFailure::TicketRejected,
        )));
    });

    assert_eq!(
        host.editor_state().editor_ui.collab.notice.unwrap().kind,
        CollabNoticeKind::DisconnectedReadOnly,
        "the stale second failure must not overwrite the retiring event"
    );
    let status: Vec<_> = runtime.drain_status_events().collect();
    assert!(status.contains(&CollabStatusEvent::Failed(CollabRuntimeFailure::Transport)));
    assert!(!status.contains(&CollabStatusEvent::Failed(
        CollabRuntimeFailure::TicketRejected
    )));
}

#[test]
fn different_epoch_welcome_fences_cached_snapshot_and_preserves_notice() {
    let mut original_owner_host = WidgetHostNative::new();
    original_owner_host.editor_state_mut().doc = document_named("Confirmed original");
    let mut original_owner = OwnerActor::new(
        SessionId::from(SESSION),
        Epoch(1),
        auth(0),
        &mut original_owner_host,
    )
    .unwrap();
    let original_connection = connection(2);
    let original_grant = original_owner
        .grant_new_peer(auth(1), Role::Editor)
        .unwrap();
    let original_activation = original_owner
        .session
        .activate_peer(original_connection, original_grant, &original_owner_host)
        .unwrap();

    let mut host = WidgetHostNative::new();
    let mut guest = GuestActor::new(
        SessionId::from(SESSION),
        Epoch(1),
        original_activation.welcome,
        original_connection,
        &mut host,
    )
    .unwrap();
    guest
        .session
        .accept_frame(
            FrameEnvelope::new(
                SessionId::from(SESSION),
                Epoch(1),
                CollabMessage::Snapshot(Box::new(original_activation.snapshot.unwrap())),
            ),
            &mut host,
        )
        .unwrap();
    set_guest_ui(&mut host, &guest, CollabConnectionPhase::Active);
    let confirmed_hash = canonical_document_hash(&host.editor_state().doc).unwrap();

    let mut replacement_owner_host = WidgetHostNative::new();
    replacement_owner_host.editor_state_mut().doc = document_named("Replacement must not install");
    let mut replacement_owner = OwnerActor::new(
        SessionId::from(SESSION),
        Epoch(2),
        auth(2),
        &mut replacement_owner_host,
    )
    .unwrap();
    let retry_connection = connection(3);
    let replacement_grant = replacement_owner
        .grant_new_peer(auth(3), Role::Editor)
        .unwrap();
    let replacement_activation = replacement_owner
        .session
        .activate_peer(retry_connection, replacement_grant, &replacement_owner_host)
        .unwrap();

    let (network, _commands) = guest_command_channel_with_capacity_for_test(8);
    let mut runtime = DesktopCollabRuntime::new();
    runtime.network = Some(network);
    runtime.actor = Some(EditorActor::Guest(Box::new(guest)));
    let stale_generation = runtime.generation;
    let sink = runtime.event_sink();
    sink.try_send(NetworkEvent::GuestAuthenticated {
        connection: retry_connection,
        session_id: SessionId::from(SESSION),
        epoch: Epoch(2),
        remote_static: [7; 32],
    })
    .unwrap();
    sink.try_send(NetworkEvent::Frame {
        connection: retry_connection,
        frame: FrameEnvelope::new(
            SessionId::from(SESSION),
            Epoch(2),
            CollabMessage::Welcome(replacement_activation.welcome),
        ),
    })
    .unwrap();
    sink.try_send(NetworkEvent::Frame {
        connection: retry_connection,
        frame: FrameEnvelope::new(
            SessionId::from(SESSION),
            Epoch(2),
            CollabMessage::Snapshot(Box::new(replacement_activation.snapshot.unwrap())),
        ),
    })
    .unwrap();

    runtime.poll(&mut host);

    assert_ne!(runtime.generation, stale_generation);
    assert!(runtime.network.is_none());
    let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
        panic!("the terminal guest actor is retained for Save As");
    };
    assert_eq!(guest.session.core().state(), GuestConnectionState::Ended);
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
        confirmed_hash,
        "the cached replacement Snapshot must not install after epoch retirement"
    );
}

#[test]
fn retired_full_backlogs_are_purged_before_a_new_launch_can_emit() {
    const LANE_CAPACITY: usize = 256;

    let mut runtime = DesktopCollabRuntime::new();
    let stale_sink = runtime.event_sink();
    for _ in 0..LANE_CAPACITY {
        stale_sink.try_send(NetworkEvent::Stopped).unwrap();
        assert!(stale_sink.send_terminal(TerminalNetworkEvent::Stopped));
    }
    runtime.retire_workers();
    runtime.defer_guest_launch(
        vec!["127.0.0.1:43120".parse().unwrap()],
        None,
        None,
        JoinIntent::New,
    );
    assert!(
        runtime.network.is_none(),
        "defer must not launch before poll purges the retired lanes"
    );
    assert!(runtime.pending_network_launch.is_some());
    let current_generation = runtime.generation;
    let mut host = WidgetHostNative::new();

    assert!(runtime.poll_with_launch_probe(&mut host, |runtime| {
        assert!(
            runtime.take_ready_network_launch_for_test(),
            "the deferred launch becomes ready only after both lanes are clean"
        );
        let current_sink = runtime.event_sink();
        current_sink
            .try_send(NetworkEvent::Discovery {
                sessions: Vec::new(),
            })
            .expect("the retired normal backlog was purged before launch");
        assert!(current_sink.send_terminal(TerminalNetworkEvent::Stopped));
        true
    }));

    let data = runtime
        .events
        .try_recv()
        .expect("the new launch retained its first normal event");
    assert_eq!(data.generation, current_generation);
    let terminal = runtime
        .terminal_events
        .try_recv()
        .expect("the new launch retained its first terminal event");
    assert_eq!(terminal.generation, current_generation);
}
