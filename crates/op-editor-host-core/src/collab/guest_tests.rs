use std::sync::Arc;

use op_collab::{
    canonical_document_hash, diff_supported, AdmissionGrant, Bye, ByeReason, ClientOpId,
    CollabMessage, Commit, CommitSeq, ConnectionKey, ConnectionPrincipal, DiffContext, Epoch,
    FrameEnvelope, GuestConnectionState, GuestEffect, GuestSessionConfig, OwnerEffect,
    OwnerSessionConfig, OwnerSessionCore, ParticipantId, PeerId, PeerNamespace, Role, SessionId,
    Submit, UndoOutcome, VerifiedAuthMetadata,
};
use op_editor_core::{EditorState, PenDocument, PenNodeExt};

use super::{GuestEditorLimits, GuestEditorSession, GuestLocalEditResolution};

const SESSION: &str = "host-guest-session";
const OWNER_PEER: &str = "owner-peer";
const OWNER_NAMESPACE: &str = "owner-ns";
const GUEST_PEER: &str = "guest-peer";
const GUEST_NAMESPACE: &str = "guest-ns";

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

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from(SESSION), Epoch(1), message)
}

fn setup() -> (
    OwnerSessionCore,
    PenDocument,
    GuestEditorSession,
    EditorState,
) {
    let owner_document = document();
    let mut owner = OwnerSessionCore::new(
        SessionId::from(SESSION),
        Epoch(1),
        CommitSeq(0),
        connection(1),
        grant(Role::Owner, OWNER_PEER, OWNER_NAMESPACE),
        &owner_document,
        OwnerSessionConfig::default(),
    )
    .unwrap();
    let activation = owner
        .activate_peer(
            connection(2),
            grant(Role::Editor, GUEST_PEER, GUEST_NAMESPACE),
            &owner_document,
        )
        .unwrap();
    let guest = GuestEditorSession::new(
        SessionId::from(SESSION),
        Epoch(1),
        activation.welcome,
        GuestSessionConfig::default(),
        GuestEditorLimits::default(),
    )
    .unwrap();
    let editor = EditorState::from_document(
        serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap(),
    );
    let mut tuple = (owner, owner_document, guest, editor);
    let snapshot = activation.snapshot.expect("new guest gets a snapshot");
    let output = tuple
        .2
        .accept_frame(
            frame(CollabMessage::Snapshot(Box::new(snapshot))),
            &mut tuple.3,
        )
        .unwrap();
    assert!(matches!(
        output.effects.as_slice(),
        [GuestEffect::Send(CollabMessage::Applied(_))]
    ));
    tuple
}

fn finalize_owner_effect(
    owner: &mut OwnerSessionCore,
    document: &mut PenDocument,
    effects: Vec<OwnerEffect>,
) -> Arc<Commit> {
    let mut prepared = effects
        .into_iter()
        .find_map(|effect| match effect {
            OwnerEffect::PrepareInstall(prepared) => Some(*prepared),
            _ => None,
        })
        .expect("owner prepares one install");
    *document = prepared.take_candidate_document().unwrap();
    let hash = canonical_document_hash(document).unwrap();
    match owner.finalize_install(prepared, hash).unwrap() {
        OwnerEffect::BroadcastCommit { commit } => commit,
        effect => panic!("unexpected owner finalize effect: {effect:?}"),
    }
}

#[test]
fn snapshot_local_submit_and_own_commit_converge() {
    let (mut owner, mut owner_document, mut guest, mut editor) = setup();
    assert_eq!(editor.doc, owner_document);

    guest.begin_local_edit(&editor).unwrap();
    editor.doc.children[0].base_mut().name = Some("Guest edit".into());
    let local = guest.finish_local_edit(&mut editor).unwrap();
    assert!(matches!(
        local.local_edit,
        Some(GuestLocalEditResolution::Submitted)
    ));
    let submit = local
        .effects
        .into_iter()
        .find_map(|effect| match effect {
            GuestEffect::Send(CollabMessage::Submit(submit)) => Some(submit),
            _ => None,
        })
        .expect("guest sends submit");

    let effects = owner
        .accept_frame(
            connection(2),
            frame(CollabMessage::Submit(submit)),
            &owner_document,
        )
        .unwrap();
    let commit = finalize_owner_effect(&mut owner, &mut owner_document, effects);
    let output = guest
        .accept_frame(frame(CollabMessage::Commit((*commit).clone())), &mut editor)
        .unwrap();

    assert!(guest.core().pending_edit().is_none());
    assert_eq!(editor.doc, owner_document);
    assert_eq!(
        canonical_document_hash(&editor.doc).unwrap(),
        owner.document_hash()
    );
    assert!(output
        .effects
        .iter()
        .any(|effect| matches!(effect, GuestEffect::Send(CollabMessage::Applied(_)))));
}

#[test]
fn guest_selective_undo_routes_request_result_and_compensation_install() {
    let (mut owner, mut owner_document, mut guest, mut editor) = setup();
    let initial = owner_document.clone();

    guest.begin_local_edit(&editor).unwrap();
    editor.doc.children[0].base_mut().name = Some("Guest edit".into());
    let local = guest.finish_local_edit(&mut editor).unwrap();
    let submit = local
        .effects
        .into_iter()
        .find_map(|effect| match effect {
            GuestEffect::Send(CollabMessage::Submit(submit)) => Some(submit),
            _ => None,
        })
        .expect("guest sends one property Submit");
    let target = submit.client_op_id.clone();
    let effects = owner
        .accept_frame(
            connection(2),
            frame(CollabMessage::Submit(submit)),
            &owner_document,
        )
        .unwrap();
    let commit = finalize_owner_effect(&mut owner, &mut owner_document, effects);
    guest
        .accept_frame(frame(CollabMessage::Commit((*commit).clone())), &mut editor)
        .unwrap();
    assert_eq!(guest.undo_targets(), vec![target.clone()]);
    assert_eq!(guest.latest_undo_target(), Some(target.clone()));

    let outbound = guest.request_undo(target, &mut editor).unwrap();
    let request = outbound
        .effects
        .into_iter()
        .find_map(|effect| match effect {
            GuestEffect::Send(CollabMessage::UndoRequest(request)) => Some(request),
            _ => None,
        })
        .expect("guest routes an UndoRequest");
    let effects = owner
        .accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(request)),
            &owner_document,
        )
        .unwrap();
    let mut prepared = effects
        .into_iter()
        .find_map(|effect| match effect {
            OwnerEffect::PrepareInstall(prepared) => Some(*prepared),
            _ => None,
        })
        .expect("owner prepares compensation install");
    owner_document = prepared.take_candidate_document().unwrap();
    let effect = owner
        .finalize_install(prepared, canonical_document_hash(&owner_document).unwrap())
        .unwrap();
    let (result, compensation) = match effect {
        OwnerEffect::UndoCommitted { result, commit, .. } => (result, commit),
        other => panic!("expected finalized selective undo, got {other:?}"),
    };
    assert_eq!(result.outcome, UndoOutcome::Committed);
    assert_eq!(owner_document, initial);

    let result_output = guest
        .accept_frame(frame(CollabMessage::UndoResult(result)), &mut editor)
        .unwrap();
    assert!(matches!(
        result_output.effects.as_slice(),
        [GuestEffect::UndoResult(_)]
    ));
    let commit_output = guest
        .accept_frame(
            frame(CollabMessage::Commit((*compensation).clone())),
            &mut editor,
        )
        .unwrap();
    assert!(commit_output
        .effects
        .iter()
        .any(|effect| matches!(effect, GuestEffect::Send(CollabMessage::Applied(_)))));
    assert_eq!(editor.doc, initial);
    assert!(guest.undo_targets().is_empty());
    assert!(guest.core().pending_undo_request().is_none());
}

#[test]
fn remote_commit_queued_during_gesture_rebases_the_pending_property_edit() {
    let (mut owner, mut owner_document, mut guest, mut editor) = setup();

    guest.begin_local_edit(&editor).unwrap();
    editor.doc.children[0].base_mut().name = Some("Guest edit".into());

    let mut owner_desired = owner_document.clone();
    owner_desired.children[0].base_mut().x = Some(25.0);
    let owner_txn = diff_supported(
        &owner_document,
        &owner_desired,
        &DiffContext::new(
            PeerNamespace::try_from(OWNER_NAMESPACE).unwrap(),
            Role::Owner,
            Some(0),
        ),
    )
    .unwrap()
    .txn;
    let effects = owner
        .accept_frame(
            connection(1),
            frame(CollabMessage::Submit(Submit {
                client_op_id: ClientOpId {
                    peer_id: PeerId::from(OWNER_PEER),
                    local_counter: 1,
                },
                base_seq: CommitSeq(0),
                txn: owner_txn,
            })),
            &owner_document,
        )
        .unwrap();
    let remote_commit = finalize_owner_effect(&mut owner, &mut owner_document, effects);

    assert!(guest
        .accept_frame(
            frame(CollabMessage::Commit((*remote_commit).clone())),
            &mut editor,
        )
        .unwrap()
        .effects
        .is_empty());
    let output = guest.finish_local_edit(&mut editor).unwrap();

    assert!(matches!(
        output.local_edit,
        Some(GuestLocalEditResolution::Submitted)
    ));
    assert!(guest.core().pending_edit().is_some());
    assert_eq!(
        editor.doc.children[0].base().name.as_deref(),
        Some("Guest edit")
    );
    assert_eq!(editor.doc.children[0].base().x, Some(25.0));
    assert!(output
        .effects
        .iter()
        .any(|effect| matches!(effect, GuestEffect::Send(CollabMessage::Submit(_)))));
}

#[test]
fn terminal_bye_reconciles_queued_commit_after_rolling_back_active_gesture() {
    for reason in [ByeReason::OwnerLeft, ByeReason::AuthenticationExpired] {
        let (mut owner, mut owner_document, mut guest, mut editor) = setup();
        let mut owner_desired = owner_document.clone();
        owner_desired.children[0].base_mut().name = Some("Remote authoritative".into());
        let owner_txn = diff_supported(
            &owner_document,
            &owner_desired,
            &DiffContext::new(
                PeerNamespace::try_from(OWNER_NAMESPACE).unwrap(),
                Role::Owner,
                Some(0),
            ),
        )
        .unwrap()
        .txn;
        let effects = owner
            .accept_frame(
                connection(1),
                frame(CollabMessage::Submit(Submit {
                    client_op_id: ClientOpId {
                        peer_id: PeerId::from(OWNER_PEER),
                        local_counter: 1,
                    },
                    base_seq: CommitSeq(0),
                    txn: owner_txn,
                })),
                &owner_document,
            )
            .unwrap();
        let remote_commit = finalize_owner_effect(&mut owner, &mut owner_document, effects);

        guest.begin_local_edit(&editor).unwrap();
        editor.doc.children[0].base_mut().x = Some(99.0);
        let queued = guest
            .accept_frame(
                frame(CollabMessage::Commit((*remote_commit).clone())),
                &mut editor,
            )
            .unwrap();
        assert!(queued.effects.is_empty());

        let output = guest
            .finish_inbound_stream(Some(frame(CollabMessage::Bye(Bye { reason }))), &mut editor)
            .unwrap();

        assert!(matches!(
            output.effects.as_slice(),
            [
                GuestEffect::Send(CollabMessage::Applied(_)),
                GuestEffect::SessionEnded { reason: ended }
            ] if *ended == reason
        ));
        assert_eq!(editor.doc, owner_document);
        assert_eq!(
            editor.doc.children[0].base().name.as_deref(),
            Some("Remote authoritative")
        );
        assert_eq!(editor.doc.children[0].base().x, Some(0.0));
        assert_eq!(guest.core().state(), GuestConnectionState::Ended);
        assert!(!guest.has_pending_host_work());
    }
}

#[test]
fn host_resume_routes_the_exact_retained_submit_and_keeps_the_optimistic_document() {
    let (mut owner, _owner_document, mut guest, mut editor) = setup();
    guest.begin_local_edit(&editor).unwrap();
    editor.doc.children[0].base_mut().name = Some("Pending across disconnect".into());
    let output = guest.finish_local_edit(&mut editor).unwrap();
    let original = output
        .effects
        .into_iter()
        .find_map(|effect| match effect {
            GuestEffect::Send(CollabMessage::Submit(submit)) => Some(submit),
            _ => None,
        })
        .expect("initial host edit emits Submit");
    let optimistic = editor.doc.clone();

    guest.disconnect(&mut editor).unwrap();
    owner.disconnect(connection(2)).unwrap();
    let activation = owner
        .resume_peer(
            connection(3),
            grant(Role::Editor, GUEST_PEER, GUEST_NAMESPACE),
        )
        .unwrap();
    let resumed = guest
        .resume(
            SessionId::from(SESSION),
            Epoch(1),
            activation.welcome,
            &mut editor,
        )
        .unwrap();
    let replay = resumed
        .effects
        .into_iter()
        .find_map(|effect| match effect {
            GuestEffect::Send(CollabMessage::Submit(submit)) => Some(submit),
            _ => None,
        })
        .expect("host resume routes retained Submit");

    assert_eq!(replay, original);
    assert_eq!(editor.doc, optimistic);
    assert!(guest.core().pending_edit().is_some());
    assert_eq!(guest.core().next_client_counter(), Some(2));
}
