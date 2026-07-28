use jian_ops_schema::PenDocument;
use op_collab::{
    canonical_document_hash, diff_supported, AdmissionGrant, ClientOpId, CollabMessage, Commit,
    CommitSeq, ConnectionKey, ConnectionPrincipal, DiffContext, Epoch, FrameEnvelope, OwnerEffect,
    OwnerSessionConfig, OwnerSessionCore, ParticipantId, PeerId, PeerNamespace, Role, SessionError,
    SessionId, Submit, UndoOutcome, UndoRequest, UndoRequestId, UndoResult, VerifiedAuthMetadata,
};
use std::sync::Arc;

const SESSION: &str = "undo-session";
const EPOCH: u64 = 9;

fn connection(raw: u64) -> ConnectionKey {
    ConnectionKey::new(raw).unwrap()
}

fn grant(peer: &str, role: Role) -> AdmissionGrant {
    AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            VerifiedAuthMetadata {
                issuer: "test-issuer".into(),
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
        PeerNamespace::try_from(format!("{peer}-ns")).unwrap(),
    )
}

fn initial_document() -> PenDocument {
    serde_json::from_str(
        r##"{"version":"1.0","children":[{
            "type":"rectangle",
            "id":"base",
            "x":0,
            "fill":[{"type":"solid","color":"#00ff00"}]
        }]}"##,
    )
    .unwrap()
}

fn edit(document: &PenDocument, x: Option<f64>, color: Option<&str>) -> PenDocument {
    let mut value = serde_json::to_value(document).unwrap();
    if let Some(x) = x {
        value["children"][0]["x"] = serde_json::json!(x);
    }
    if let Some(color) = color {
        value["children"][0]["fill"] = serde_json::json!([{"type":"solid","color":color}]);
    }
    serde_json::from_value(value).unwrap()
}

fn field_values(document: &PenDocument) -> (f64, String) {
    let value = serde_json::to_value(document).unwrap();
    (
        value["children"][0]["x"].as_f64().unwrap(),
        value["children"][0]["fill"][0]["color"]
            .as_str()
            .unwrap()
            .to_owned(),
    )
}

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from(SESSION), Epoch(EPOCH), message)
}

fn setup(config: OwnerSessionConfig) -> (OwnerSessionCore, PenDocument) {
    let document = initial_document();
    let mut core = OwnerSessionCore::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        CommitSeq(0),
        connection(1),
        grant("owner", Role::Owner),
        &document,
        config,
    )
    .unwrap();
    core.activate_peer(connection(2), grant("a", Role::Editor), &document)
        .unwrap();
    core.activate_peer(connection(3), grant("b", Role::Editor), &document)
        .unwrap();
    (core, document)
}

fn take_prepare(effects: Vec<OwnerEffect>) -> op_collab::PreparedCommit {
    assert_eq!(effects.len(), 1);
    match effects.into_iter().next().unwrap() {
        OwnerEffect::PrepareInstall(prepared) => *prepared,
        other => panic!("expected PrepareInstall, got {other:?}"),
    }
}

fn submit_edit(
    core: &mut OwnerSessionCore,
    document: &mut PenDocument,
    connection: ConnectionKey,
    peer: &str,
    counter: u64,
    after: PenDocument,
) -> Arc<Commit> {
    let supported = diff_supported(
        document,
        &after,
        &DiffContext::new(
            PeerNamespace::try_from(format!("{peer}-ns")).unwrap(),
            Role::Editor,
            Some(0),
        ),
    )
    .unwrap();
    let submit = Submit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from(peer),
            local_counter: counter,
        },
        base_seq: core.seq(),
        txn: supported.txn,
    };
    let mut prepared = take_prepare(
        core.accept_frame(connection, frame(CollabMessage::Submit(submit)), document)
            .unwrap(),
    );
    *document = prepared.take_candidate_document().unwrap();
    let hash = canonical_document_hash(document).unwrap();
    match core.finalize_install(prepared, hash).unwrap() {
        OwnerEffect::BroadcastCommit { commit } => commit,
        other => panic!("expected Commit broadcast, got {other:?}"),
    }
}

fn undo_request(counter: u64, target: ClientOpId) -> UndoRequest {
    UndoRequest {
        request_id: UndoRequestId {
            peer_id: PeerId::from("a"),
            local_counter: counter,
        },
        target_client_op_id: target,
    }
}

fn take_undo_reply(effects: Vec<OwnerEffect>, expected: ConnectionKey) -> UndoResult {
    assert_eq!(effects.len(), 1);
    match effects.into_iter().next().unwrap() {
        OwnerEffect::Reply {
            to,
            message: CollabMessage::UndoResult(result),
        } => {
            assert_eq!(to, expected);
            result
        }
        other => panic!("expected UndoResult reply, got {other:?}"),
    }
}

#[test]
fn later_writer_on_same_field_wins_and_undo_conflict_is_exactly_once() {
    let (mut core, mut document) = setup(OwnerSessionConfig::default());
    let red = edit(&document, None, Some("#ff0000"));
    let a = submit_edit(&mut core, &mut document, connection(2), "a", 1, red);
    let blue = edit(&document, None, Some("#0000ff"));
    let b = submit_edit(&mut core, &mut document, connection(3), "b", 1, blue);

    let request = undo_request(1, a.client_op_id.clone());
    let result = take_undo_reply(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(request.clone())),
            &document,
        )
        .unwrap(),
        connection(2),
    );
    assert_eq!(result.outcome, UndoOutcome::Conflict);
    assert_eq!(result.owner_seq, CommitSeq(2));
    assert_eq!(field_values(&document).1, "#0000ff");

    let replay = take_undo_reply(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(request.clone())),
            &document,
        )
        .unwrap(),
        connection(2),
    );
    assert_eq!(replay, result);
    assert_eq!(
        core.peer_progress(&PeerId::from("a"))
            .unwrap()
            .next_undo_counter,
        Some(2)
    );

    let reused = undo_request(1, b.client_op_id.clone());
    assert!(matches!(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(reused)),
            &document
        ),
        Err(SessionError::UndoRequestReuse)
    ));
}

#[test]
fn different_field_undo_survives_commit_log_gc_and_finalizes_atomically() {
    let mut config = OwnerSessionConfig::default();
    config.session_limits.commit_log_entries = 1;
    let (mut core, mut document) = setup(config);
    let moved = edit(&document, Some(10.0), None);
    let a = submit_edit(&mut core, &mut document, connection(2), "a", 1, moved);
    let blue = edit(&document, None, Some("#0000ff"));
    submit_edit(&mut core, &mut document, connection(3), "b", 1, blue);
    let before_undo_hash = canonical_document_hash(&document).unwrap();
    let request = undo_request(1, a.client_op_id.clone());

    let first = take_prepare(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(request.clone())),
            &document,
        )
        .unwrap(),
    );
    assert_eq!(core.seq(), CommitSeq(2));
    assert_eq!(core.document_hash(), before_undo_hash);
    assert_eq!(
        field_values(first.candidate_document().unwrap()),
        (0.0, "#0000ff".into())
    );
    core.abort_prepare(first).unwrap();
    assert_eq!(
        core.peer_progress(&PeerId::from("a")).unwrap().next_counter,
        Some(2)
    );
    assert_eq!(
        core.peer_progress(&PeerId::from("a"))
            .unwrap()
            .next_undo_counter,
        Some(1)
    );

    let mut prepared = take_prepare(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(request.clone())),
            &document,
        )
        .unwrap(),
    );
    document = prepared.take_candidate_document().unwrap();
    let effect = core
        .finalize_install(prepared, canonical_document_hash(&document).unwrap())
        .unwrap();
    let result = match effect {
        OwnerEffect::UndoCommitted {
            reply_to,
            result,
            commit,
        } => {
            assert_eq!(reply_to, connection(2));
            assert_eq!(commit.seq, CommitSeq(3));
            assert_eq!(commit.client_op_id.local_counter, 2);
            result
        }
        other => panic!("expected finalized undo, got {other:?}"),
    };
    assert_eq!(result.outcome, UndoOutcome::Committed);
    assert_eq!(result.owner_seq, CommitSeq(3));
    assert_eq!(
        result
            .compensation_client_op_id
            .as_ref()
            .unwrap()
            .local_counter,
        2
    );
    assert_eq!(field_values(&document), (0.0, "#0000ff".into()));
    let progress = core.peer_progress(&PeerId::from("a")).unwrap();
    assert_eq!(progress.next_counter, Some(3));
    assert_eq!(progress.next_undo_counter, Some(2));

    core.disconnect(connection(2)).unwrap();
    core.resume_peer(connection(4), grant("a", Role::Editor))
        .unwrap();
    let replay = take_undo_reply(
        core.accept_frame(
            connection(4),
            frame(CollabMessage::UndoRequest(request)),
            &document,
        )
        .unwrap(),
        connection(4),
    );
    assert_eq!(replay, result);
}

#[test]
fn partial_undo_reverts_only_fields_still_owned_by_target() {
    let (mut core, mut document) = setup(OwnerSessionConfig::default());
    let moved_red = edit(&document, Some(12.0), Some("#ff0000"));
    let a = submit_edit(&mut core, &mut document, connection(2), "a", 1, moved_red);
    let blue = edit(&document, None, Some("#0000ff"));
    submit_edit(&mut core, &mut document, connection(3), "b", 1, blue);

    let mut prepared = take_prepare(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(undo_request(
                1,
                a.client_op_id.clone(),
            ))),
            &document,
        )
        .unwrap(),
    );
    assert_eq!(
        field_values(prepared.candidate_document().unwrap()),
        (0.0, "#0000ff".into())
    );
    document = prepared.take_candidate_document().unwrap();
    let result = match core
        .finalize_install(prepared, canonical_document_hash(&document).unwrap())
        .unwrap()
    {
        OwnerEffect::UndoCommitted { result, .. } => result,
        other => panic!("expected finalized partial undo, got {other:?}"),
    };
    assert_eq!(result.outcome, UndoOutcome::Committed);
    assert!(result.details.unwrap().contains("reverted 1 of 2 fields"));
    assert_eq!(field_values(&document), (0.0, "#0000ff".into()));
}

#[test]
fn owner_local_undo_api_reuses_the_same_two_phase_and_dedupe_path() {
    let (mut core, mut document) = setup(OwnerSessionConfig::default());
    let moved = edit(&document, Some(7.0), None);
    let owner_commit = submit_edit(&mut core, &mut document, connection(1), "owner", 1, moved);
    assert_eq!(
        core.own_undo_targets(),
        vec![owner_commit.client_op_id.clone()]
    );
    assert_eq!(
        core.latest_own_undo_target(),
        Some(owner_commit.client_op_id.clone())
    );

    let request = core
        .next_own_undo_request(owner_commit.client_op_id.clone())
        .unwrap();
    let prepared = take_prepare(core.request_own_undo(request.clone(), &document).unwrap());
    core.abort_prepare(prepared).unwrap();
    let mut prepared = take_prepare(core.request_own_undo(request.clone(), &document).unwrap());
    document = prepared.take_candidate_document().unwrap();
    let result = match core
        .finalize_install(prepared, canonical_document_hash(&document).unwrap())
        .unwrap()
    {
        OwnerEffect::UndoCommitted {
            reply_to, result, ..
        } => {
            assert_eq!(reply_to, connection(1));
            result
        }
        other => panic!("expected owner-local finalized undo, got {other:?}"),
    };
    assert_eq!(result.outcome, UndoOutcome::Committed);
    assert_eq!(field_values(&document).0, 0.0);
    assert!(core.own_undo_targets().is_empty());

    let replay = take_undo_reply(
        core.request_own_undo(request, &document).unwrap(),
        connection(1),
    );
    assert_eq!(replay, result);
}

#[test]
fn viewer_structural_targets_and_undo_window_fail_closed() {
    let mut config = OwnerSessionConfig::default();
    config.session_limits.result_window_entries = 1;
    let (mut core, mut document) = setup(config);
    core.activate_peer(connection(4), grant("viewer", Role::Viewer), &document)
        .unwrap();

    let viewer_request = UndoRequest {
        request_id: UndoRequestId {
            peer_id: PeerId::from("viewer"),
            local_counter: 1,
        },
        target_client_op_id: ClientOpId {
            peer_id: PeerId::from("viewer"),
            local_counter: 1,
        },
    };
    let denied = take_undo_reply(
        core.accept_frame(
            connection(4),
            frame(CollabMessage::UndoRequest(viewer_request.clone())),
            &document,
        )
        .unwrap(),
        connection(4),
    );
    assert_eq!(denied.outcome, UndoOutcome::Rejected);
    let replay = take_undo_reply(
        core.accept_frame(
            connection(4),
            frame(CollabMessage::UndoRequest(viewer_request)),
            &document,
        )
        .unwrap(),
        connection(4),
    );
    assert_eq!(replay, denied);

    let mut value = serde_json::to_value(&document).unwrap();
    value["children"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"type":"rectangle","id":"c_a-ns_1"}));
    let inserted: PenDocument = serde_json::from_value(value).unwrap();
    let structural = submit_edit(&mut core, &mut document, connection(2), "a", 1, inserted);
    let structural_result = take_undo_reply(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(undo_request(
                1,
                structural.client_op_id.clone(),
            ))),
            &document,
        )
        .unwrap(),
        connection(2),
    );
    assert_eq!(structural_result.outcome, UndoOutcome::Rejected);

    let missing = |request_counter, target_counter| {
        undo_request(
            request_counter,
            ClientOpId {
                peer_id: PeerId::from("a"),
                local_counter: target_counter,
            },
        )
    };
    let second = take_undo_reply(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(missing(2, 77))),
            &document,
        )
        .unwrap(),
        connection(2),
    );
    assert_eq!(second.outcome, UndoOutcome::Rejected);
    let progress = core.peer_progress(&PeerId::from("a")).unwrap();
    assert_eq!(progress.undo_retired_floor, 2);
    assert_eq!(progress.retained_undo_results, 1);
    let expired = take_undo_reply(
        core.accept_frame(
            connection(2),
            frame(CollabMessage::UndoRequest(missing(1, 1))),
            &document,
        )
        .unwrap(),
        connection(2),
    );
    assert_eq!(expired.outcome, UndoOutcome::Rejected);
    assert_eq!(
        core.peer_progress(&PeerId::from("a"))
            .unwrap()
            .next_undo_counter,
        Some(3)
    );
}
