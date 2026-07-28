use jian_ops_schema::PenDocument;
use op_collab::{
    canonical_document_hash, AdmissionGrant, Applied, ByeReason, CatchUp, ClientOpId,
    CollabMessage, CollabOp, CollabTxn, Commit, CommitSeq, ConnectionKey, ConnectionPrincipal,
    Epoch, FrameEnvelope, OwnerEffect, OwnerSessionConfig, OwnerSessionCore, PageRef,
    ParticipantId, PeerActivation, PeerId, PeerNamespace, Presence, Reject, RejectCode, Role,
    SessionError, SessionId, Submit, VerifiedAuthMetadata,
};
use std::sync::Arc;

const SESSION: &str = "session-a";
const EPOCH: u64 = 7;
const OWNER_PARTICIPANT: &str = "participant-owner";
const OWNER_PEER: &str = "peer-owner";
const OWNER_NAMESPACE: &str = "owner-ns";
const EDITOR_PARTICIPANT: &str = "participant-editor";
const EDITOR_PEER: &str = "peer-editor";
const EDITOR_NAMESPACE: &str = "editor-ns";
const VIEWER_PARTICIPANT: &str = "participant-viewer";
const VIEWER_PEER: &str = "peer-viewer";
const VIEWER_NAMESPACE: &str = "viewer-ns";
const INITIAL_EXPIRY: u64 = 100;

fn connection(raw: u64) -> ConnectionKey {
    ConnectionKey::new(raw).expect("non-zero test connection")
}

fn document() -> PenDocument {
    serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap()
}

fn verified(peer_id: &str, expires_at_unix_ms: u64) -> VerifiedAuthMetadata {
    VerifiedAuthMetadata {
        issuer: "https://issuer.example".into(),
        subject: format!("subject-{peer_id}"),
        device_id: format!("device-{peer_id}"),
        proof_binding: format!("proof-{peer_id}"),
        expires_at_unix_ms,
        display_name: None,
        avatar_url: None,
    }
}

fn grant(role: Role, participant_id: &str, peer_id: &str, namespace: &str) -> AdmissionGrant {
    AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            verified(peer_id, INITIAL_EXPIRY),
            ParticipantId::from(participant_id),
            PeerId::from(peer_id),
            role,
        ),
        PeerNamespace::try_from(namespace).unwrap(),
    )
}

fn new_core(config: OwnerSessionConfig, seq: CommitSeq) -> (OwnerSessionCore, PenDocument) {
    let document = document();
    let core = OwnerSessionCore::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        seq,
        connection(1),
        grant(Role::Owner, OWNER_PARTICIPANT, OWNER_PEER, OWNER_NAMESPACE),
        &document,
        config,
    )
    .unwrap();
    (core, document)
}

fn activate(
    core: &mut OwnerSessionCore,
    document: &PenDocument,
    role: Role,
    connection: ConnectionKey,
) -> PeerActivation {
    let (participant, peer, namespace) = match role {
        Role::Editor => (EDITOR_PARTICIPANT, EDITOR_PEER, EDITOR_NAMESPACE),
        Role::Viewer => (VIEWER_PARTICIPANT, VIEWER_PEER, VIEWER_NAMESPACE),
        Role::Owner => panic!("the owner is activated by OwnerSessionCore::new"),
    };
    core.activate_peer(
        connection,
        grant(role, participant, peer, namespace),
        document,
    )
    .unwrap()
}

fn setup_peer(
    role: Role,
    config: OwnerSessionConfig,
    seq: CommitSeq,
) -> (OwnerSessionCore, PenDocument, PeerActivation) {
    let (mut core, document) = new_core(config, seq);
    let activation = activate(&mut core, &document, role, connection(2));
    (core, document, activation)
}

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from(SESSION), Epoch(EPOCH), message)
}

fn insert_txn(namespace: &str, node_counter: u64, index: u32) -> CollabTxn {
    let node = serde_json::from_value(serde_json::json!({
        "type": "rectangle",
        "id": format!("c_{namespace}_{node_counter}"),
        "name": format!("node-{node_counter}"),
    }))
    .unwrap();
    CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index,
        node,
    }])
}

fn submit(
    peer_id: &str,
    counter: u64,
    base_seq: u64,
    namespace: &str,
    node_counter: u64,
    index: u32,
) -> Submit {
    Submit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from(peer_id),
            local_counter: counter,
        },
        base_seq: CommitSeq(base_seq),
        txn: insert_txn(namespace, node_counter, index),
    }
}

fn take_prepare(effects: Vec<OwnerEffect>) -> op_collab::PreparedCommit {
    assert_eq!(effects.len(), 1);
    match effects.into_iter().next().unwrap() {
        OwnerEffect::PrepareInstall(prepared) => *prepared,
        other => panic!("expected PrepareInstall, got {other:?}"),
    }
}

fn take_reply(effects: Vec<OwnerEffect>, expected: ConnectionKey) -> CollabMessage {
    assert_eq!(effects.len(), 1);
    match effects.into_iter().next().unwrap() {
        OwnerEffect::Reply { to, message } => {
            assert_eq!(to, expected);
            message
        }
        other => panic!("expected Reply, got {other:?}"),
    }
}

fn take_commit_reply(effects: Vec<OwnerEffect>, expected: ConnectionKey) -> Arc<Commit> {
    assert_eq!(effects.len(), 1);
    match effects.into_iter().next().unwrap() {
        OwnerEffect::ReplyCommit { to, commit } => {
            assert_eq!(to, expected);
            commit
        }
        other => panic!("expected Commit reply, got {other:?}"),
    }
}

fn take_reject(effects: Vec<OwnerEffect>, expected: ConnectionKey) -> Reject {
    match take_reply(effects, expected) {
        CollabMessage::Reject(reject) => reject,
        other => panic!("expected Reject, got {other:?}"),
    }
}

fn finalize(
    core: &mut OwnerSessionCore,
    document: &mut PenDocument,
    effects: Vec<OwnerEffect>,
) -> Arc<Commit> {
    let mut prepared = take_prepare(effects);
    let candidate_hash = prepared.candidate_hash();
    let candidate = prepared
        .take_candidate_document()
        .expect("candidate is available exactly once");
    *document = candidate;
    let installed_hash = canonical_document_hash(document).unwrap();
    assert_eq!(installed_hash, candidate_hash);
    let effect = core.finalize_install(prepared, installed_hash).unwrap();
    match effect {
        OwnerEffect::BroadcastCommit { commit } => commit,
        other => panic!("expected Commit broadcast, got {other:?}"),
    }
}

fn accept_submit(
    core: &mut OwnerSessionCore,
    document: &PenDocument,
    connection: ConnectionKey,
    submit: Submit,
) -> Result<Vec<OwnerEffect>, SessionError> {
    core.accept_frame(connection, frame(CollabMessage::Submit(submit)), document)
}

#[test]
fn activation_welcome_contains_one_consistent_self_and_complete_roster() {
    let (mut core, document) = new_core(OwnerSessionConfig::default(), CommitSeq(0));
    let activation = activate(&mut core, &document, Role::Editor, connection(2));
    assert_eq!(activation.connection, connection(2));
    assert_eq!(
        activation.welcome.participant_id,
        activation.joined.participant_id
    );
    assert_eq!(activation.welcome.peer_id, activation.joined.peer_id);
    assert_eq!(activation.welcome.role, activation.joined.role);
    assert_eq!(activation.welcome.peer_namespace, EDITOR_NAMESPACE);
    assert_eq!(activation.welcome.seq, CommitSeq(0));
    assert_eq!(
        activation
            .welcome
            .participants
            .iter()
            .filter(|participant| participant.peer_id == activation.welcome.peer_id)
            .count(),
        1
    );
    assert_eq!(activation.welcome.participants.len(), 2);
    assert!(activation
        .welcome
        .participants
        .iter()
        .any(|participant| participant.peer_id == PeerId::from(OWNER_PEER)));
    let snapshot = activation.snapshot.expect("new peer receives a snapshot");
    assert_eq!(snapshot.seq, CommitSeq(0));
    assert_eq!(
        snapshot.doc_hash,
        canonical_document_hash(&document).unwrap()
    );
}

#[test]
fn activation_rejects_duplicate_peer_participant_and_namespace() {
    let (mut core, document) = new_core(OwnerSessionConfig::default(), CommitSeq(0));
    activate(&mut core, &document, Role::Editor, connection(2));
    let duplicate_peer = core
        .activate_peer(
            connection(3),
            grant(
                Role::Editor,
                "participant-new",
                EDITOR_PEER,
                "namespace-new",
            ),
            &document,
        )
        .unwrap_err();
    assert!(matches!(duplicate_peer, SessionError::PeerAlreadyExists));
    let duplicate_participant = core
        .activate_peer(
            connection(3),
            grant(
                Role::Editor,
                EDITOR_PARTICIPANT,
                "peer-new",
                "namespace-new",
            ),
            &document,
        )
        .unwrap_err();
    assert!(matches!(
        duplicate_participant,
        SessionError::DuplicateParticipant
    ));
    let duplicate_namespace = core
        .activate_peer(
            connection(3),
            grant(
                Role::Editor,
                "participant-new",
                "peer-new",
                EDITOR_NAMESPACE,
            ),
            &document,
        )
        .unwrap_err();
    assert!(matches!(
        duplicate_namespace,
        SessionError::DuplicateNamespace
    ));
    assert_eq!(core.active_participants().len(), 2);
}

#[test]
fn frame_binding_rejects_wrong_session_and_epoch() {
    let (mut core, document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let presence = Presence {
        cursor: None,
        selection: Vec::new(),
        viewport: None,
        editing_node: None,
    };
    let wrong_session = FrameEnvelope::new(
        SessionId::from("session-b"),
        Epoch(EPOCH),
        CollabMessage::PresenceUpdate(presence.clone()),
    );
    assert!(matches!(
        core.accept_frame(connection(2), wrong_session, &document),
        Err(SessionError::WrongSession)
    ));
    let wrong_epoch = FrameEnvelope::new(
        SessionId::from(SESSION),
        Epoch(EPOCH + 1),
        CollabMessage::PresenceUpdate(presence),
    );
    assert!(matches!(
        core.accept_frame(connection(2), wrong_epoch, &document),
        Err(SessionError::WrongEpoch)
    ));
}

#[test]
fn owner_to_peer_message_is_rejected_in_peer_direction() {
    let (mut core, document, activation) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let inbound = frame(CollabMessage::ParticipantJoined(activation.joined));
    assert!(matches!(
        core.accept_frame(connection(2), inbound, &document),
        Err(SessionError::WrongDirection {
            kind: "participant_joined"
        })
    ));
}

#[test]
fn submit_cannot_spoof_the_connection_peer_id() {
    let (mut core, document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let spoofed = submit("peer-attacker", 1, 0, EDITOR_NAMESPACE, 1, 0);
    assert!(matches!(
        accept_submit(&mut core, &document, connection(2), spoofed),
        Err(SessionError::ClientOpPeerMismatch)
    ));
}

#[test]
fn presence_broadcast_injects_identity_from_the_connection() {
    let (mut core, document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let presence = Presence {
        cursor: Some(op_collab::Point { x: 4.0, y: 8.0 }),
        selection: vec!["selected-node".into()],
        viewport: Some(op_collab::Viewport {
            pan_x: 1.0,
            pan_y: 2.0,
            zoom: 1.5,
        }),
        editing_node: Some("editing-node".into()),
    };
    let effects = core
        .accept_frame(
            connection(2),
            frame(CollabMessage::PresenceUpdate(presence.clone())),
            &document,
        )
        .unwrap();
    assert_eq!(effects.len(), 1);
    match effects.into_iter().next().unwrap() {
        OwnerEffect::Broadcast {
            message: CollabMessage::PresenceChanged(changed),
        } => {
            assert_eq!(
                changed.participant_id,
                ParticipantId::from(EDITOR_PARTICIPANT)
            );
            assert_eq!(changed.peer_id, PeerId::from(EDITOR_PEER));
            assert_eq!(changed.presence, presence);
        }
        other => panic!("expected PresenceChanged broadcast, got {other:?}"),
    }
}

#[test]
fn first_submit_advances_only_after_install_and_binds_hash_seq_and_author() {
    let (mut core, mut document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let initial_hash = core.document_hash();
    let operation = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    let effects = accept_submit(&mut core, &document, connection(2), operation.clone()).unwrap();
    let mut prepared = take_prepare(effects);
    let candidate_hash = prepared.candidate_hash();

    assert_eq!(core.seq(), CommitSeq(0));
    assert_eq!(core.document_hash(), initial_hash);
    assert_eq!(prepared.commit_seq(), CommitSeq(1));
    assert_ne!(candidate_hash, initial_hash);
    let before_finalize = core.peer_progress(&PeerId::from(EDITOR_PEER)).unwrap();
    assert_eq!(before_finalize.next_counter, Some(1));
    assert_eq!(before_finalize.retained_results, 0);
    let candidate = prepared.take_candidate_document().unwrap();
    assert_eq!(canonical_document_hash(&candidate).unwrap(), candidate_hash);
    document = candidate;
    let effect = core
        .finalize_install(prepared, canonical_document_hash(&document).unwrap())
        .unwrap();
    let commit = match effect {
        OwnerEffect::BroadcastCommit { commit } => commit,
        other => panic!("expected Commit broadcast, got {other:?}"),
    };

    assert_eq!(commit.client_op_id, operation.client_op_id);
    assert_eq!(commit.seq, CommitSeq(1));
    assert_eq!(
        commit.author.participant_id,
        ParticipantId::from(EDITOR_PARTICIPANT)
    );
    assert_eq!(commit.author.peer_id, PeerId::from(EDITOR_PEER));
    assert_eq!(commit.doc_hash, candidate_hash);
    assert_eq!(core.seq(), CommitSeq(1));
    assert_eq!(
        core.document_hash(),
        canonical_document_hash(&document).unwrap()
    );
    let progress = core.peer_progress(&PeerId::from(EDITOR_PEER)).unwrap();
    assert_eq!(progress.next_counter, Some(2));
    assert_eq!(progress.retained_results, 1);
    assert_eq!(progress.applied_through, CommitSeq(0));
    assert_eq!(
        core.peer_progress(&PeerId::from(OWNER_PEER))
            .unwrap()
            .applied_through,
        CommitSeq(1)
    );
}

#[test]
fn committed_submit_replay_returns_the_original_commit_exactly_once() {
    let (mut core, mut document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let operation = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    let effects = accept_submit(&mut core, &document, connection(2), operation.clone()).unwrap();
    let committed = finalize(&mut core, &mut document, effects);
    let replay = accept_submit(&mut core, &document, connection(2), operation).unwrap();
    let replayed = take_commit_reply(replay, connection(2));
    assert_eq!(replayed, committed);
    assert!(Arc::ptr_eq(&replayed, &committed));
    assert_eq!(core.seq(), CommitSeq(1));
    assert_eq!(
        core.peer_progress(&PeerId::from(EDITOR_PEER))
            .unwrap()
            .next_counter,
        Some(2)
    );
}

#[test]
fn retained_submit_replay_ignores_a_changed_base_sequence() {
    let (mut core, mut document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let original = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    let effects = accept_submit(&mut core, &document, connection(2), original.clone()).unwrap();
    let committed = finalize(&mut core, &mut document, effects);
    let mut replay = original;
    replay.base_seq = CommitSeq(u64::MAX);

    let replayed = take_commit_reply(
        accept_submit(&mut core, &document, connection(2), replay).unwrap(),
        connection(2),
    );
    assert_eq!(replayed, committed);
    assert!(Arc::ptr_eq(&replayed, &committed));
}

#[test]
fn retained_client_op_id_with_a_different_transaction_is_fatal() {
    let (mut core, mut document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let original = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    let effects = accept_submit(&mut core, &document, connection(2), original).unwrap();
    finalize(&mut core, &mut document, effects);
    let changed = submit(EDITOR_PEER, 1, 1, EDITOR_NAMESPACE, 99, 1);
    assert!(matches!(
        accept_submit(&mut core, &document, connection(2), changed),
        Err(SessionError::ClientOpReuse)
    ));
    assert_eq!(core.seq(), CommitSeq(1));
}

#[test]
fn viewer_permission_reject_is_consumed_and_replayed_exactly_once() {
    let (mut core, document, _) =
        setup_peer(Role::Viewer, OwnerSessionConfig::default(), CommitSeq(0));
    let operation = submit(VIEWER_PEER, 1, 0, VIEWER_NAMESPACE, 1, 0);
    let first = take_reject(
        accept_submit(&mut core, &document, connection(2), operation.clone()).unwrap(),
        connection(2),
    );
    assert_eq!(first.code, RejectCode::PermissionDenied);
    let progress = core.peer_progress(&PeerId::from(VIEWER_PEER)).unwrap();
    assert_eq!(progress.next_counter, Some(2));
    assert_eq!(progress.retained_results, 1);
    let replayed = take_reject(
        accept_submit(&mut core, &document, connection(2), operation).unwrap(),
        connection(2),
    );
    assert_eq!(replayed, first);
    assert_eq!(core.seq(), CommitSeq(0));
}

#[test]
fn counter_gap_replies_and_closes_without_consuming_the_counter() {
    let (mut core, document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let gap = submit(EDITOR_PEER, 2, 0, EDITOR_NAMESPACE, 2, 0);
    let effects = accept_submit(&mut core, &document, connection(2), gap).unwrap();

    assert_eq!(effects.len(), 2);
    assert!(effects.iter().any(|effect| matches!(
        effect,
        OwnerEffect::Reply {
            to,
            message: CollabMessage::Reject(reject)
        } if *to == connection(2) && reject.code == RejectCode::CounterGap
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        OwnerEffect::Close {
            connection: target,
            reason: ByeReason::ProtocolError
        } if *target == connection(2)
    )));
    let progress = core.peer_progress(&PeerId::from(EDITOR_PEER)).unwrap();
    assert_eq!(progress.next_counter, Some(1));
    assert_eq!(progress.retained_results, 0);
    let current = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    assert!(matches!(
        accept_submit(&mut core, &document, connection(2), current.clone()),
        Err(SessionError::ConnectionClosing)
    ));
    core.disconnect(connection(2)).unwrap();
    core.resume_peer(
        connection(3),
        grant(
            Role::Editor,
            EDITOR_PARTICIPANT,
            EDITOR_PEER,
            EDITOR_NAMESPACE,
        ),
    )
    .unwrap();
    assert!(matches!(
        accept_submit(&mut core, &document, connection(3), current)
            .unwrap()
            .as_slice(),
        [OwnerEffect::PrepareInstall(_)]
    ));
}

#[test]
fn retired_result_returns_expired_without_advancing_progress() {
    let mut config = OwnerSessionConfig::default();
    config.session_limits.result_window_entries = 1;
    let (mut core, mut document, _) = setup_peer(Role::Editor, config, CommitSeq(0));
    let first = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    let effects = accept_submit(&mut core, &document, connection(2), first.clone()).unwrap();
    finalize(&mut core, &mut document, effects);
    let second = submit(EDITOR_PEER, 2, 1, EDITOR_NAMESPACE, 2, 1);
    let effects = accept_submit(&mut core, &document, connection(2), second).unwrap();
    finalize(&mut core, &mut document, effects);

    let progress = core.peer_progress(&PeerId::from(EDITOR_PEER)).unwrap();
    assert_eq!(progress.next_counter, Some(3));
    assert_eq!(progress.retired_floor, 2);
    assert_eq!(progress.retained_results, 1);
    let expired = take_reject(
        accept_submit(&mut core, &document, connection(2), first).unwrap(),
        connection(2),
    );
    assert_eq!(expired.code, RejectCode::ExpiredClientOpId);
    assert_eq!(expired.owner_seq, CommitSeq(2));
    assert_eq!(
        core.peer_progress(&PeerId::from(EDITOR_PEER)).unwrap(),
        progress
    );
}

#[test]
fn exhausted_owner_sequence_rejects_without_consuming_the_submit() {
    let (mut core, document, _) = setup_peer(
        Role::Editor,
        OwnerSessionConfig::default(),
        CommitSeq(u64::MAX),
    );
    let operation = submit(EDITOR_PEER, 1, u64::MAX, EDITOR_NAMESPACE, 1, 0);
    assert!(matches!(
        accept_submit(&mut core, &document, connection(2), operation),
        Err(SessionError::SequenceExhausted)
    ));
    let progress = core.peer_progress(&PeerId::from(EDITOR_PEER)).unwrap();
    assert_eq!(progress.next_counter, Some(1));
    assert_eq!(progress.retained_results, 0);
    assert_eq!(core.seq(), CommitSeq(u64::MAX));
}

#[test]
fn catch_up_replays_retained_commits_and_snapshots_a_log_gap() {
    let mut config = OwnerSessionConfig::default();
    config.session_limits.commit_log_entries = 1;
    let (mut core, mut document, _) = setup_peer(Role::Editor, config, CommitSeq(0));
    let first = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    let effects = accept_submit(&mut core, &document, connection(2), first).unwrap();
    finalize(&mut core, &mut document, effects);
    let second = submit(EDITOR_PEER, 2, 1, EDITOR_NAMESPACE, 2, 1);
    let effects = accept_submit(&mut core, &document, connection(2), second).unwrap();
    let second_commit = finalize(&mut core, &mut document, effects);

    let retained = core
        .accept_frame(
            connection(2),
            frame(CollabMessage::CatchUp(CatchUp {
                after_seq: CommitSeq(1),
            })),
            &document,
        )
        .unwrap();
    assert!(matches!(
        retained.as_slice(),
        [OwnerEffect::CommitBatch { to, commits }]
            if *to == connection(2)
                && commits.len() == 1
                && Arc::ptr_eq(&commits[0], &second_commit)
    ));

    let gap = core
        .accept_frame(
            connection(2),
            frame(CollabMessage::CatchUp(CatchUp {
                after_seq: CommitSeq(0),
            })),
            &document,
        )
        .unwrap();
    match gap.as_slice() {
        [OwnerEffect::Snapshot { to, snapshot }] => {
            assert_eq!(*to, connection(2));
            assert_eq!(snapshot.seq, CommitSeq(2));
            assert_eq!(snapshot.document, document);
            assert_eq!(snapshot.doc_hash, core.document_hash());
        }
        other => panic!("expected gap Snapshot, got {other:?}"),
    }

    let current = core
        .accept_frame(
            connection(2),
            frame(CollabMessage::CatchUp(CatchUp {
                after_seq: CommitSeq(2),
            })),
            &document,
        )
        .unwrap();
    assert!(matches!(
        current.as_slice(),
        [OwnerEffect::CommitBatch { commits, .. }] if commits.is_empty()
    ));
}

#[test]
fn applied_prunes_commit_log_but_not_submit_dedupe_results() {
    let (mut core, mut document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let operation = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    let effects = accept_submit(&mut core, &document, connection(2), operation.clone()).unwrap();
    let committed = finalize(&mut core, &mut document, effects);

    let applied = core
        .accept_frame(
            connection(2),
            frame(CollabMessage::Applied(Applied {
                through_seq: CommitSeq(1),
            })),
            &document,
        )
        .unwrap();
    assert!(applied.is_empty());
    assert_eq!(
        core.peer_progress(&PeerId::from(EDITOR_PEER))
            .unwrap()
            .applied_through,
        CommitSeq(1)
    );

    let catch_up = core
        .accept_frame(
            connection(2),
            frame(CollabMessage::CatchUp(CatchUp {
                after_seq: CommitSeq(0),
            })),
            &document,
        )
        .unwrap();
    assert!(matches!(
        catch_up.as_slice(),
        [OwnerEffect::Snapshot { snapshot, .. }] if snapshot.seq == CommitSeq(1)
    ));

    let replayed = take_commit_reply(
        accept_submit(&mut core, &document, connection(2), operation).unwrap(),
        connection(2),
    );
    assert_eq!(replayed, committed);
    assert!(Arc::ptr_eq(&replayed, &committed));
}

#[test]
fn disconnect_and_resume_preserve_counter_and_dedupe_state() {
    let (mut core, mut document, _) =
        setup_peer(Role::Editor, OwnerSessionConfig::default(), CommitSeq(0));
    let first = submit(EDITOR_PEER, 1, 0, EDITOR_NAMESPACE, 1, 0);
    let effects = accept_submit(&mut core, &document, connection(2), first.clone()).unwrap();
    let committed = finalize(&mut core, &mut document, effects);

    let left = core.disconnect(connection(2)).unwrap();
    assert!(matches!(
        left.as_slice(),
        [OwnerEffect::Broadcast {
            message: CollabMessage::ParticipantLeft(participant)
        }] if participant.peer_id == PeerId::from(EDITOR_PEER)
    ));
    let disconnected = core.peer_progress(&PeerId::from(EDITOR_PEER)).unwrap();
    assert!(!disconnected.active);
    assert_eq!(disconnected.next_counter, Some(2));
    assert_eq!(disconnected.retained_results, 1);

    let resumed = core
        .resume_peer(
            connection(3),
            grant(
                Role::Editor,
                EDITOR_PARTICIPANT,
                EDITOR_PEER,
                EDITOR_NAMESPACE,
            ),
        )
        .unwrap();
    assert!(resumed.snapshot.is_none());
    assert_eq!(resumed.welcome.seq, CommitSeq(1));
    assert_eq!(resumed.welcome.participants.len(), 2);
    assert!(
        core.peer_progress(&PeerId::from(EDITOR_PEER))
            .unwrap()
            .active
    );

    let replayed = take_commit_reply(
        accept_submit(&mut core, &document, connection(3), first).unwrap(),
        connection(3),
    );
    assert_eq!(replayed, committed);
    assert!(Arc::ptr_eq(&replayed, &committed));

    let second = submit(EDITOR_PEER, 2, 1, EDITOR_NAMESPACE, 2, 1);
    assert!(matches!(
        accept_submit(&mut core, &document, connection(3), second)
            .unwrap()
            .as_slice(),
        [OwnerEffect::PrepareInstall(_)]
    ));
}
