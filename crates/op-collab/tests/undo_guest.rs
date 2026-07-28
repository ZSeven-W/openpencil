use jian_ops_schema::PenDocument;
use op_collab::{
    canonical_document_hash, diff_supported, ClientOpId, CollabMessage, Commit, CommitAuthor,
    CommitSeq, DiffContext, Epoch, FrameEnvelope, GuestEffect, GuestError, GuestSessionConfig,
    GuestSessionCore, Participant, ParticipantId, PeerId, PeerNamespace, Reject, RejectCode, Role,
    SessionId, Snapshot, Submit, UndoOutcome, UndoRequest, UndoResult, Welcome, WireLimits,
    CANONICAL_HASH_VERSION,
};

const SESSION: &str = "guest-undo";
const EPOCH: u64 = 11;

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

fn set_x(document: &PenDocument, x: f64) -> PenDocument {
    let mut value = serde_json::to_value(document).unwrap();
    value["children"][0]["x"] = serde_json::json!(x);
    serde_json::from_value(value).unwrap()
}

fn set_fill(document: &PenDocument, color: &str) -> PenDocument {
    let mut value = serde_json::to_value(document).unwrap();
    value["children"][0]["fill"] = serde_json::json!([{"type":"solid","color":color}]);
    serde_json::from_value(value).unwrap()
}

fn participant(peer: &str, role: Role) -> Participant {
    Participant {
        participant_id: ParticipantId::from(format!("participant-{peer}")),
        peer_id: PeerId::from(peer),
        role,
        display_name: None,
        avatar_url: None,
    }
}

fn welcome(role: Role, seq: CommitSeq) -> Welcome {
    Welcome {
        participant_id: ParticipantId::from("participant-guest"),
        peer_id: PeerId::from("guest"),
        role,
        seq,
        peer_namespace: "guest".into(),
        document_schema_version: "1.0".into(),
        hash_version: CANONICAL_HASH_VERSION,
        limits: WireLimits::default(),
        participants: vec![
            participant("owner", Role::Owner),
            participant("guest", role),
        ],
    }
}

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from(SESSION), Epoch(EPOCH), message)
}

fn snapshot(document: &PenDocument, seq: CommitSeq) -> Snapshot {
    Snapshot {
        seq,
        document: document.clone(),
        doc_hash: canonical_document_hash(document).unwrap(),
    }
}

fn take_install(effects: Vec<GuestEffect>) -> op_collab::PreparedGuestInstall {
    effects
        .into_iter()
        .find_map(|effect| match effect {
            GuestEffect::PrepareInstall(prepared) => Some(*prepared),
            _ => None,
        })
        .expect("effects include one prepared install")
}

fn finalize(
    guest: &mut GuestSessionCore,
    prepared: op_collab::PreparedGuestInstall,
) -> Vec<GuestEffect> {
    let hash = prepared.candidate_hash();
    guest.finalize_install(prepared, hash).unwrap()
}

fn setup(role: Role) -> (GuestSessionCore, PenDocument) {
    let document = initial_document();
    let mut guest = GuestSessionCore::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        welcome(role, CommitSeq(0)),
        GuestSessionConfig::default(),
    )
    .unwrap();
    let prepared = take_install(
        guest
            .accept_frame(frame(CollabMessage::Snapshot(Box::new(snapshot(
                &document,
                CommitSeq(0),
            )))))
            .unwrap(),
    );
    finalize(&mut guest, prepared);
    (guest, document)
}

fn take_submit(effect: GuestEffect) -> Submit {
    match effect {
        GuestEffect::Send(CollabMessage::Submit(submit)) => submit,
        other => panic!("expected Submit, got {other:?}"),
    }
}

fn commit_local(guest: &mut GuestSessionCore, desired: &PenDocument, seq: CommitSeq) -> ClientOpId {
    let submit = take_submit(guest.begin_local_edit(desired).unwrap());
    let client_op_id = submit.client_op_id.clone();
    let commit = Commit {
        client_op_id: client_op_id.clone(),
        seq,
        author: CommitAuthor {
            participant_id: ParticipantId::from("participant-guest"),
            peer_id: PeerId::from("guest"),
        },
        txn: submit.txn,
        doc_hash: canonical_document_hash(desired).unwrap(),
    };
    let prepared = take_install(
        guest
            .accept_frame(frame(CollabMessage::Commit(commit)))
            .unwrap(),
    );
    finalize(guest, prepared);
    client_op_id
}

fn take_undo_request(effect: GuestEffect) -> UndoRequest {
    match effect {
        GuestEffect::Send(CollabMessage::UndoRequest(request)) => request,
        other => panic!("expected UndoRequest, got {other:?}"),
    }
}

fn compensation(
    before: &PenDocument,
    after: &PenDocument,
    client_op_id: ClientOpId,
    seq: CommitSeq,
) -> Commit {
    let supported = diff_supported(
        before,
        after,
        &DiffContext::new(
            PeerNamespace::try_from("guest").unwrap(),
            Role::Editor,
            Some(0),
        ),
    )
    .unwrap();
    Commit {
        client_op_id,
        seq,
        author: CommitAuthor {
            participant_id: ParticipantId::from("participant-guest"),
            peer_id: PeerId::from("guest"),
        },
        txn: supported.txn,
        doc_hash: canonical_document_hash(after).unwrap(),
    }
}

#[test]
fn undo_index_contains_only_installed_property_commits_across_snapshot() {
    let (mut guest, initial) = setup(Role::Editor);
    let rejected_desired = set_x(&initial, 1.0);
    let rejected = take_submit(guest.begin_local_edit(&rejected_desired).unwrap());
    let rollback = take_install(
        guest
            .accept_frame(frame(CollabMessage::Reject(Reject {
                client_op_id: rejected.client_op_id,
                owner_seq: CommitSeq(0),
                code: RejectCode::InvalidOperation,
                details: None,
            })))
            .unwrap(),
    );
    finalize(&mut guest, rollback);
    assert!(guest.undo_targets().is_empty());

    let moved = set_x(&initial, 8.0);
    let property = commit_local(&mut guest, &moved, CommitSeq(1));
    assert_eq!(guest.undo_targets(), vec![property.clone()]);

    let remote = set_fill(&moved, "#0000ff");
    let prepared = take_install(
        guest
            .accept_frame(frame(CollabMessage::Snapshot(Box::new(snapshot(
                &remote,
                CommitSeq(2),
            )))))
            .unwrap(),
    );
    finalize(&mut guest, prepared);
    assert_eq!(guest.undo_targets(), vec![property.clone()]);

    let mut value = serde_json::to_value(&remote).unwrap();
    value["children"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"type":"rectangle","id":"c_guest_1"}));
    let structural: PenDocument = serde_json::from_value(value).unwrap();
    let structural_id = commit_local(&mut guest, &structural, CommitSeq(3));
    assert_eq!(guest.undo_targets(), vec![property]);
    assert!(matches!(
        guest.request_undo(structural_id),
        Err(GuestError::UnknownUndoTarget)
    ));

    let (mut viewer, _) = setup(Role::Viewer);
    assert!(matches!(
        viewer.request_undo(ClientOpId {
            peer_id: PeerId::from("guest"),
            local_counter: 1,
        }),
        Err(GuestError::ViewerReadOnly)
    ));
}

#[test]
fn compensation_commit_can_arrive_before_result_and_is_installed_once() {
    let (mut guest, initial) = setup(Role::Editor);
    let moved = set_x(&initial, 10.0);
    let target = commit_local(&mut guest, &moved, CommitSeq(1));
    let request = take_undo_request(guest.request_undo(target.clone()).unwrap());
    assert!(matches!(
        guest.begin_local_edit(&set_fill(&moved, "#0000ff")),
        Err(GuestError::UndoPending)
    ));

    let compensation_id = ClientOpId {
        peer_id: PeerId::from("guest"),
        local_counter: 2,
    };
    let commit = compensation(&moved, &initial, compensation_id.clone(), CommitSeq(2));
    assert!(guest
        .accept_frame(frame(CollabMessage::Commit(commit)))
        .unwrap()
        .is_empty());
    assert_eq!(guest.confirmed_seq(), Some(CommitSeq(1)));

    let result = UndoResult {
        request_id: request.request_id,
        target_client_op_id: target,
        owner_seq: CommitSeq(2),
        outcome: UndoOutcome::Committed,
        compensation_client_op_id: Some(compensation_id),
        details: None,
    };
    let effects = guest
        .accept_frame(frame(CollabMessage::UndoResult(result.clone())))
        .unwrap();
    assert!(matches!(effects.first(), Some(GuestEffect::UndoResult(_))));
    let prepared = take_install(effects);
    assert_eq!(
        prepared.reason(),
        op_collab::GuestInstallReason::UndoCompensation
    );
    finalize(&mut guest, prepared);
    assert_eq!(guest.confirmed_seq(), Some(CommitSeq(2)));
    assert_eq!(
        canonical_document_hash(guest.confirmed_document().unwrap()).unwrap(),
        canonical_document_hash(&initial).unwrap()
    );
    assert!(guest.undo_targets().is_empty());
    assert!(guest.pending_undo_request().is_none());
    assert_eq!(guest.next_client_counter(), Some(3));
    assert!(guest.document_mutation_allowed());

    let duplicate = guest
        .accept_frame(frame(CollabMessage::UndoResult(result)))
        .unwrap();
    assert!(matches!(duplicate.as_slice(), [GuestEffect::UndoResult(_)]));
    assert_eq!(guest.next_client_counter(), Some(3));
}

#[test]
fn reconnect_resends_pending_undo_and_snapshot_can_cover_compensation() {
    let (mut guest, initial) = setup(Role::Editor);
    let moved = set_x(&initial, 5.0);
    let target = commit_local(&mut guest, &moved, CommitSeq(1));
    let request = take_undo_request(guest.request_undo(target.clone()).unwrap());
    guest.disconnect();

    let effects = guest
        .resume(
            SessionId::from(SESSION),
            Epoch(EPOCH),
            welcome(Role::Editor, CommitSeq(2)),
        )
        .unwrap();
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, GuestEffect::Send(CollabMessage::CatchUp(_)))));
    assert!(effects.iter().any(|effect| {
        matches!(
            effect,
            GuestEffect::Send(CollabMessage::UndoRequest(replayed)) if replayed == &request
        )
    }));
    assert_eq!(guest.undo_targets(), vec![target.clone()]);

    let prepared = take_install(
        guest
            .accept_frame(frame(CollabMessage::Snapshot(Box::new(snapshot(
                &initial,
                CommitSeq(2),
            )))))
            .unwrap(),
    );
    finalize(&mut guest, prepared);
    assert_eq!(guest.undo_targets(), vec![target.clone()]);

    let result = UndoResult {
        request_id: request.request_id,
        target_client_op_id: target,
        owner_seq: CommitSeq(2),
        outcome: UndoOutcome::Committed,
        compensation_client_op_id: Some(ClientOpId {
            peer_id: PeerId::from("guest"),
            local_counter: 2,
        }),
        details: None,
    };
    let effects = guest
        .accept_frame(frame(CollabMessage::UndoResult(result)))
        .unwrap();
    assert!(matches!(effects.as_slice(), [GuestEffect::UndoResult(_)]));
    assert_eq!(guest.confirmed_seq(), Some(CommitSeq(2)));
    assert!(guest.undo_targets().is_empty());
    assert_eq!(guest.next_client_counter(), Some(3));
    assert!(guest.document_mutation_allowed());
}

#[test]
fn conflict_result_removes_unusable_target_without_reserving_submit_id() {
    let (mut guest, initial) = setup(Role::Editor);
    let moved = set_x(&initial, 3.0);
    let target = commit_local(&mut guest, &moved, CommitSeq(1));
    let request = take_undo_request(guest.request_undo(target.clone()).unwrap());
    let effects = guest
        .accept_frame(frame(CollabMessage::UndoResult(UndoResult {
            request_id: request.request_id,
            target_client_op_id: target,
            owner_seq: CommitSeq(2),
            outcome: UndoOutcome::Conflict,
            compensation_client_op_id: None,
            details: Some("newer writer".into()),
        })))
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [GuestEffect::UndoResult(UndoResult {
            outcome: UndoOutcome::Conflict,
            ..
        })]
    ));
    assert!(guest.undo_targets().is_empty());
    assert_eq!(guest.next_client_counter(), Some(2));
    assert_eq!(guest.next_undo_counter(), Some(2));
    assert!(guest.document_mutation_allowed());
}
