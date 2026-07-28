use op_collab::{
    ByeReason, CollabMessage, ConnectionKey, Epoch, FrameEnvelope, GuestConnectionState,
    OwnerEffect, Role, SessionId, VerifiedAuthMetadata,
};
use op_editor_core::{CollabConnectionPhase, CollabNoticeKind, PenDocument, PenNodeExt};
use op_host_native::WidgetHostNative;

use super::actor::{set_guest_ui, EditorActor, GuestActor, OwnerActor};
use super::network::guest_command_channel_with_capacity_for_test;
use super::types::{NetworkEvent, RemoteBye};
use super::DesktopCollabRuntime;

const SESSION: &str = "desktop-terminal-reconcile";

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

fn runtime_with_remote_commit() -> (
    DesktopCollabRuntime,
    WidgetHostNative,
    std::sync::mpsc::Receiver<super::types::GuestNetworkCommand>,
    ConnectionKey,
    FrameEnvelope,
) {
    let mut owner_host = WidgetHostNative::new();
    owner_host.editor_state_mut().doc = document_named("Before");
    let mut owner =
        OwnerActor::new(SessionId::from(SESSION), Epoch(1), auth(0), &mut owner_host).unwrap();
    let guest_connection = connection(2);
    let grant = owner.grant_new_peer(auth(1), Role::Editor).unwrap();
    let activation = owner
        .session
        .activate_peer(guest_connection, grant, &owner_host)
        .unwrap();

    let mut guest_host = WidgetHostNative::new();
    let mut guest = GuestActor::new(
        SessionId::from(SESSION),
        Epoch(1),
        activation.welcome,
        guest_connection,
        &mut guest_host,
    )
    .unwrap();
    guest
        .session
        .accept_frame(
            FrameEnvelope::new(
                SessionId::from(SESSION),
                Epoch(1),
                CollabMessage::Snapshot(Box::new(activation.snapshot.unwrap())),
            ),
            &mut guest_host,
        )
        .unwrap();
    set_guest_ui(&mut guest_host, &guest, CollabConnectionPhase::Active);

    owner.session.begin_local_edit(&owner_host).unwrap();
    owner_host.editor_state_mut().doc = document_named("Remote authoritative");
    let output = owner.session.finish_local_edit(&mut owner_host).unwrap();
    let commit = output
        .effects
        .into_iter()
        .find_map(|effect| match effect {
            OwnerEffect::BroadcastCommit { commit } => Some(commit),
            _ => None,
        })
        .expect("owner edit emits an authoritative commit");
    let frame = FrameEnvelope::new(
        SessionId::from(SESSION),
        Epoch(1),
        CollabMessage::Commit((*commit).clone()),
    );

    let (network, commands) = guest_command_channel_with_capacity_for_test(8);
    let mut runtime = DesktopCollabRuntime::new();
    runtime.network = Some(network);
    runtime.actor = Some(EditorActor::Guest(Box::new(guest)));
    (runtime, guest_host, commands, guest_connection, frame)
}

#[test]
fn typed_bye_after_queued_commit_preserves_authority_and_exact_terminal_notice() {
    for (reason, notice) in [
        (ByeReason::OwnerLeft, CollabNoticeKind::OwnerLeft),
        (
            ByeReason::AuthenticationExpired,
            CollabNoticeKind::TicketExpired,
        ),
    ] {
        let (mut runtime, mut host, commands, guest_connection, commit) =
            runtime_with_remote_commit();
        assert!(runtime.begin_local_edit(&mut host));
        host.editor_state_mut().doc = document_named("Optimistic gesture");
        runtime.handle_event(
            NetworkEvent::Frame {
                connection: guest_connection,
                frame: commit,
            },
            &mut host,
        );
        assert_eq!(
            host.editor_state().doc.children[0].base().name.as_deref(),
            Some("Optimistic gesture")
        );

        // Prove terminal reconciliation does not depend on an Applied reaching
        // the already-closed network worker.
        drop(commands);
        runtime.handle_event(
            NetworkEvent::ConnectionClosed {
                connection: guest_connection,
                failure: None,
                remote_bye: Some(RemoteBye {
                    session_id: SessionId::from(SESSION),
                    epoch: Epoch(1),
                    reason,
                }),
            },
            &mut host,
        );

        let Some(EditorActor::Guest(guest)) = runtime.actor.as_ref() else {
            panic!("terminal guest is retained for Save As");
        };
        assert_eq!(guest.session.core().state(), GuestConnectionState::Ended);
        assert_eq!(
            host.editor_state().doc.children[0].base().name.as_deref(),
            Some("Remote authoritative")
        );
        assert_eq!(
            host.editor_state().editor_ui.collab.phase,
            CollabConnectionPhase::Ended
        );
        assert_eq!(
            host.editor_state().editor_ui.collab.notice.unwrap().kind,
            notice
        );
        assert!(!runtime.transaction_active);
    }
}
