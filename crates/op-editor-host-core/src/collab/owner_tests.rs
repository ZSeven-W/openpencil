use op_collab::{
    AdmissionGrant, CatchUp, CollabMessage, CommitSeq, ConnectionKey, ConnectionPrincipal, Epoch,
    FrameEnvelope, OwnerEffect, OwnerSessionConfig, ParticipantId, PeerId, PeerNamespace, Role,
    SessionId, UndoOutcome, VerifiedAuthMetadata,
};
use op_editor_core::{EditorState, PenDocument, PenNodeExt};

use super::{
    LocalEditRejection, LocalEditResolution, OwnerEditorError, OwnerEditorLimits,
    OwnerEditorSession,
};

const SESSION: &str = "host-owner-session";
const OWNER_PEER: &str = "owner-peer";
const OWNER_NAMESPACE: &str = "owner-ns";

fn connection(raw: u64) -> ConnectionKey {
    ConnectionKey::new(raw).unwrap()
}

fn document() -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "rectangle",
            "id": "n1",
            "name": "Before",
            "x": 0,
            "y": 0,
            "width": 10,
            "height": 10
        }]
    }))
    .unwrap()
}

fn grant(role: Role, peer: &str, namespace: &str) -> AdmissionGrant {
    AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            VerifiedAuthMetadata {
                issuer: "https://issuer.example".into(),
                subject: format!("subject-{peer}"),
                device_id: format!("device-{peer}"),
                proof_binding: format!("binding-{peer}"),
                expires_at_unix_ms: 10_000,
                display_name: None,
                avatar_url: None,
            },
            ParticipantId::from(format!("participant-{peer}")),
            PeerId::from(peer),
            role,
        ),
        PeerNamespace::try_from(namespace).unwrap(),
    )
}

fn session(editor: &EditorState, limits: OwnerEditorLimits) -> OwnerEditorSession {
    OwnerEditorSession::new(
        SessionId::from(SESSION),
        Epoch(1),
        CommitSeq(0),
        connection(1),
        grant(Role::Owner, OWNER_PEER, OWNER_NAMESPACE),
        editor,
        OwnerSessionConfig::default(),
        limits,
    )
    .unwrap()
}

fn catch_up_frame(session_id: &str) -> FrameEnvelope {
    FrameEnvelope::new(
        SessionId::from(session_id),
        Epoch(1),
        CollabMessage::CatchUp(CatchUp {
            after_seq: CommitSeq(0),
        }),
    )
}

#[test]
fn local_edit_is_verified_then_finalized_before_broadcast() {
    let mut editor = EditorState::from_document(document());
    let mut session = session(&editor, OwnerEditorLimits::default());

    session.begin_local_edit(&editor).unwrap();
    editor.doc.children[0].base_mut().name = Some("After".into());
    let output = session.finish_local_edit(&mut editor).unwrap();

    assert!(matches!(
        output.local_edit,
        Some(LocalEditResolution::Committed(_))
    ));
    assert_eq!(session.core().seq(), CommitSeq(1));
    assert_eq!(
        session.core().document_hash(),
        op_collab::canonical_document_hash(&editor.doc).unwrap()
    );
    assert!(matches!(
        output.effects.as_slice(),
        [OwnerEffect::BroadcastCommit { .. }]
    ));
}

#[test]
fn owner_local_selective_undo_installs_then_routes_one_idempotent_result() {
    let mut editor = EditorState::from_document(document());
    let before = editor.doc.clone();
    let mut session = session(&editor, OwnerEditorLimits::default());

    session.begin_local_edit(&editor).unwrap();
    editor.doc.children[0].base_mut().name = Some("After".into());
    let committed = session.finish_local_edit(&mut editor).unwrap();
    let target = match committed.local_edit {
        Some(LocalEditResolution::Committed(target)) => target,
        other => panic!("expected committed owner edit, got {other:?}"),
    };
    assert_eq!(session.own_undo_targets(), vec![target.clone()]);
    assert_eq!(session.latest_own_undo_target(), Some(target.clone()));

    let request = session.next_own_undo_request(target).unwrap();
    let output = session
        .request_own_undo(request.clone(), &mut editor)
        .unwrap();
    let result = match output.effects.as_slice() {
        [OwnerEffect::UndoCommitted {
            reply_to,
            result,
            commit,
        }] => {
            assert_eq!(*reply_to, connection(1));
            assert_eq!(commit.seq, CommitSeq(2));
            result.clone()
        }
        other => panic!("expected finalized owner undo, got {other:?}"),
    };
    assert_eq!(result.outcome, UndoOutcome::Committed);
    assert_eq!(editor.doc, before);
    assert!(session.own_undo_targets().is_empty());
    assert_eq!(session.core().seq(), CommitSeq(2));

    let replay = session.request_own_undo(request, &mut editor).unwrap();
    assert!(matches!(
        replay.effects.as_slice(),
        [OwnerEffect::Reply {
            to,
            message: CollabMessage::UndoResult(replayed),
        }] if *to == connection(1) && replayed == &result
    ));
    assert_eq!(session.core().seq(), CommitSeq(2));
    assert_eq!(editor.doc, before);
}

#[test]
fn unsupported_local_edit_rolls_back_without_advancing_sequence() {
    let mut editor = EditorState::from_document(document());
    let before = editor.doc.clone();
    let mut session = session(&editor, OwnerEditorLimits::default());

    session.begin_local_edit(&editor).unwrap();
    editor.doc.version = "unsupported-version-change".into();
    let output = session.finish_local_edit(&mut editor).unwrap();

    assert!(matches!(
        output.local_edit,
        Some(LocalEditResolution::Rejected(
            LocalEditRejection::Unsupported(_)
        ))
    ));
    assert_eq!(editor.doc, before);
    assert_eq!(session.core().seq(), CommitSeq(0));
    assert!(output.effects.is_empty());
}

#[test]
fn document_frames_are_bounded_while_a_local_gesture_is_active() {
    let mut editor = EditorState::from_document(document());
    let mut session = session(
        &editor,
        OwnerEditorLimits {
            max_queued_document_frames: 1,
            max_queued_document_bytes: 64 * 1024,
        },
    );
    let editor_connection = connection(2);
    session
        .activate_peer(
            editor_connection,
            grant(Role::Editor, "guest-peer", "guest-ns"),
            &editor,
        )
        .unwrap();
    session.begin_local_edit(&editor).unwrap();

    assert!(session
        .accept_frame(editor_connection, catch_up_frame(SESSION), &mut editor)
        .unwrap()
        .effects
        .is_empty());
    assert!(matches!(
        session.accept_frame(editor_connection, catch_up_frame(SESSION), &mut editor),
        Err(OwnerEditorError::QueueFull)
    ));

    let output = session.finish_local_edit(&mut editor).unwrap();
    assert!(matches!(
        output.local_edit,
        Some(LocalEditResolution::NoChange)
    ));
    assert!(matches!(
        output.effects.as_slice(),
        [OwnerEffect::CommitBatch { commits, .. }] if commits.is_empty()
    ));
}

#[test]
fn bad_queued_peer_is_scoped_and_good_queued_peer_still_flushes() {
    let mut editor = EditorState::from_document(document());
    let mut session = session(&editor, OwnerEditorLimits::default());
    let bad_connection = connection(2);
    let good_connection = connection(3);
    session
        .activate_peer(
            bad_connection,
            grant(Role::Editor, "bad-peer", "bad-ns"),
            &editor,
        )
        .unwrap();
    session
        .activate_peer(
            good_connection,
            grant(Role::Editor, "good-peer", "good-ns"),
            &editor,
        )
        .unwrap();
    session.begin_local_edit(&editor).unwrap();

    session
        .accept_frame(bad_connection, catch_up_frame("wrong-session"), &mut editor)
        .unwrap();
    session
        .accept_frame(bad_connection, catch_up_frame(SESSION), &mut editor)
        .unwrap();
    session
        .accept_frame(good_connection, catch_up_frame(SESSION), &mut editor)
        .unwrap();

    let output = session.finish_local_edit(&mut editor).unwrap();

    assert_eq!(output.failed_connections, vec![bad_connection]);
    assert!(matches!(
        output.effects.as_slice(),
        [OwnerEffect::CommitBatch { to, commits }]
            if *to == good_connection && commits.is_empty()
    ));
    assert!(session
        .core()
        .peer_progress(&PeerId::from("good-peer"))
        .is_some_and(|progress| progress.active));
}

#[test]
fn disconnect_purges_queued_frames_and_releases_their_byte_budget() {
    let mut editor = EditorState::from_document(document());
    let encoded_frame_bytes = catch_up_frame(SESSION).to_json_vec().unwrap().len();
    let mut session = session(
        &editor,
        OwnerEditorLimits {
            max_queued_document_frames: 1,
            max_queued_document_bytes: encoded_frame_bytes,
        },
    );
    let disconnected = connection(2);
    let good_connection = connection(3);
    session
        .activate_peer(
            disconnected,
            grant(Role::Editor, "gone-peer", "gone-ns"),
            &editor,
        )
        .unwrap();
    session
        .activate_peer(
            good_connection,
            grant(Role::Editor, "good-peer", "good-ns"),
            &editor,
        )
        .unwrap();
    session.begin_local_edit(&editor).unwrap();
    session
        .accept_frame(disconnected, catch_up_frame(SESSION), &mut editor)
        .unwrap();

    let disconnected_output = session.disconnect(disconnected).unwrap();
    assert!(disconnected_output.failed_connections.is_empty());
    assert!(matches!(
        disconnected_output.effects.as_slice(),
        [OwnerEffect::Broadcast {
            message: CollabMessage::ParticipantLeft(_)
        }]
    ));

    session
        .accept_frame(good_connection, catch_up_frame(SESSION), &mut editor)
        .unwrap();
    let output = session.finish_local_edit(&mut editor).unwrap();

    assert!(output.failed_connections.is_empty());
    assert!(matches!(
        output.effects.as_slice(),
        [OwnerEffect::CommitBatch { to, commits }]
            if *to == good_connection && commits.is_empty()
    ));
}
