use jian_ops_schema::{node::PenNode, PenDocument};
use op_collab::{
    apply_txn, canonical_document_hash, diff_supported, ApplyContext, ClientOpId, CollabMessage,
    Commit, CommitAuthor, CommitSeq, DiffContext, Epoch, FrameEnvelope, GuestConnectionState,
    GuestEffect, GuestError, GuestSessionConfig, GuestSessionCore, OpaqueTicket, Participant,
    ParticipantId, PeerId, PeerNamespace, PendingCancelReason, PendingEditStatus, Reject,
    RejectCode, RenewTicket, Role, SessionId, Snapshot, Submit, Welcome, WireLimits,
    CANONICAL_HASH_VERSION,
};

fn initial_document() -> PenDocument {
    serde_json::from_str(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"base","x":0,"y":0}
        ]}"#,
    )
    .unwrap()
}

fn set_position(document: &PenDocument, x: f64, y: f64) -> PenDocument {
    let mut document = document.clone();
    let PenNode::Rectangle(rectangle) = &mut document.children[0] else {
        panic!("fixture is a rectangle");
    };
    rectangle.base.x = Some(x);
    rectangle.base.y = Some(y);
    document
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

fn guest(role: Role) -> GuestSessionCore {
    GuestSessionCore::new(
        SessionId::from("session"),
        Epoch(7),
        welcome(role, CommitSeq(0)),
        GuestSessionConfig::default(),
    )
    .unwrap()
}

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from("session"), Epoch(7), message)
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
        .expect("effects contain an install")
}

fn finalize(
    core: &mut GuestSessionCore,
    prepared: op_collab::PreparedGuestInstall,
) -> Vec<GuestEffect> {
    let hash = prepared.candidate_hash();
    core.finalize_install(prepared, hash).unwrap()
}

fn install_initial(core: &mut GuestSessionCore, document: &PenDocument) {
    let effects = core
        .accept_frame(frame(CollabMessage::Snapshot(Box::new(snapshot(
            document,
            CommitSeq(0),
        )))))
        .unwrap();
    let prepared = take_install(effects);
    finalize(core, prepared);
}

#[test]
fn owner_ticket_renewal_is_routed_to_the_noise_bound_verifier() {
    let mut core = guest(Role::Editor);
    let effects = core
        .accept_frame(frame(CollabMessage::RenewTicket(RenewTicket {
            opaque_ticket: OpaqueTicket::new("renewed-owner-ticket".into()).unwrap(),
        })))
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [GuestEffect::VerifyRenewal { ticket }]
            if ticket.expose() == "renewed-owner-ticket"
    ));
}

fn owner_commit(seq: u64, counter: u64, before: &PenDocument, after: &PenDocument) -> Commit {
    let supported = diff_supported(
        before,
        after,
        &DiffContext::new(
            PeerNamespace::try_from("owner").unwrap(),
            Role::Owner,
            Some(0),
        ),
    )
    .unwrap();
    Commit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from("owner"),
            local_counter: counter,
        },
        seq: CommitSeq(seq),
        author: CommitAuthor {
            participant_id: ParticipantId::from("participant-owner"),
            peer_id: PeerId::from("owner"),
        },
        txn: supported.txn,
        doc_hash: canonical_document_hash(after).unwrap(),
    }
}

fn extract_submit(effect: GuestEffect) -> Submit {
    let GuestEffect::Send(CollabMessage::Submit(submit)) = effect else {
        panic!("local edit emits Submit");
    };
    submit
}

fn take_submit(effects: Vec<GuestEffect>) -> Submit {
    effects
        .into_iter()
        .find_map(|effect| match effect {
            GuestEffect::Send(CollabMessage::Submit(submit)) => Some(submit),
            _ => None,
        })
        .expect("effects contain Submit")
}

#[test]
fn authenticated_snapshot_and_own_commit_advance_only_after_install() {
    let initial = initial_document();
    let mut core = guest(Role::Editor);
    assert_eq!(core.state(), GuestConnectionState::AwaitingSnapshot);
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Snapshot(Box::new(snapshot(
            &initial,
            CommitSeq(0),
        )))))
        .unwrap(),
    );
    assert!(core.confirmed_document().is_none());
    finalize(&mut core, prepared);
    assert_eq!(core.state(), GuestConnectionState::Active);
    assert_eq!(core.confirmed_seq(), Some(CommitSeq(0)));

    let desired = set_position(&initial, 10.0, 0.0);
    let submit = extract_submit(core.begin_local_edit(&desired).unwrap());
    assert_eq!(submit.base_seq, CommitSeq(0));
    assert!(core.pending_edit().is_some());

    let committed = apply_txn(
        &initial,
        &submit.txn,
        &ApplyContext::for_peer_namespace(PeerNamespace::try_from("guest").unwrap(), Role::Editor),
    )
    .unwrap();
    let commit = Commit {
        client_op_id: submit.client_op_id,
        seq: CommitSeq(1),
        author: CommitAuthor {
            participant_id: ParticipantId::from("participant-guest"),
            peer_id: PeerId::from("guest"),
        },
        txn: submit.txn,
        doc_hash: canonical_document_hash(&committed).unwrap(),
    };
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(commit)))
            .unwrap(),
    );
    assert_eq!(core.confirmed_seq(), Some(CommitSeq(0)));
    finalize(&mut core, prepared);
    assert_eq!(core.confirmed_seq(), Some(CommitSeq(1)));
    assert!(core.pending_edit().is_none());
    assert_eq!(
        canonical_document_hash(core.confirmed_document().unwrap()).unwrap(),
        canonical_document_hash(&desired).unwrap()
    );
}

#[test]
fn remote_commit_crosses_pending_then_stale_reject_resubmits_new_counter() {
    let initial = initial_document();
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let local = set_position(&initial, 10.0, 0.0);
    let first = extract_submit(core.begin_local_edit(&local).unwrap());

    let remote = set_position(&initial, 0.0, 5.0);
    let commit = owner_commit(1, 1, &initial, &remote);
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(commit)))
            .unwrap(),
    );
    finalize(&mut core, prepared);
    assert_eq!(core.confirmed_seq(), Some(CommitSeq(1)));
    let displayed = core.displayed_document().unwrap();
    assert_eq!(
        canonical_document_hash(displayed).unwrap(),
        canonical_document_hash(&set_position(&initial, 10.0, 5.0)).unwrap()
    );

    let effects = core
        .accept_frame(frame(CollabMessage::Reject(Reject {
            client_op_id: first.client_op_id,
            owner_seq: CommitSeq(1),
            code: RejectCode::StaleBase,
            details: None,
        })))
        .unwrap();
    let GuestEffect::Send(CollabMessage::Submit(retry)) = &effects[0] else {
        panic!("stale pending is resubmitted");
    };
    assert_eq!(retry.client_op_id.local_counter, 2);
    assert_eq!(retry.base_seq, CommitSeq(1));
    let replayed = apply_txn(
        core.confirmed_document().unwrap(),
        &retry.txn,
        &ApplyContext::for_peer_namespace(PeerNamespace::try_from("guest").unwrap(), Role::Editor),
    )
    .unwrap();
    assert_eq!(
        canonical_document_hash(&replayed).unwrap(),
        canonical_document_hash(&set_position(&initial, 10.0, 5.0)).unwrap()
    );
}

#[test]
fn conflicting_remote_property_cancels_pending_after_atomic_install() {
    let initial = initial_document();
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let local = set_position(&initial, 10.0, 0.0);
    core.begin_local_edit(&local).unwrap();

    let remote = set_position(&initial, 20.0, 0.0);
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(owner_commit(
            1, 1, &initial, &remote,
        ))))
        .unwrap(),
    );
    assert!(core.pending_edit().is_some());
    let effects = finalize(&mut core, prepared);
    assert!(core.pending_edit().is_none());
    assert!(effects.iter().any(|effect| matches!(
        effect,
        GuestEffect::PendingCancelled {
            reason: PendingCancelReason::PropertyConflict { node_id, field },
            ..
        } if node_id == "base" && field == "x"
    )));
    assert_eq!(
        canonical_document_hash(core.displayed_document().unwrap()).unwrap(),
        canonical_document_hash(&remote).unwrap()
    );
}

#[test]
fn out_of_order_commits_are_bounded_and_drained_in_sequence() {
    let initial = initial_document();
    let one = set_position(&initial, 1.0, 0.0);
    let two = set_position(&initial, 2.0, 0.0);
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let commit_one = owner_commit(1, 1, &initial, &one);
    let commit_two = owner_commit(2, 2, &one, &two);

    let effects = core
        .accept_frame(frame(CollabMessage::Commit(commit_two)))
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [GuestEffect::Send(CollabMessage::CatchUp(_))]
    ));
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(commit_one)))
            .unwrap(),
    );
    let next_effects = finalize(&mut core, prepared);
    let prepared_two = take_install(next_effects);
    finalize(&mut core, prepared_two);
    assert_eq!(core.confirmed_seq(), Some(CommitSeq(2)));
    assert_eq!(
        canonical_document_hash(core.confirmed_document().unwrap()).unwrap(),
        canonical_document_hash(&two).unwrap()
    );
}

#[test]
fn commit_can_buffer_before_snapshot_and_install_queue_is_explicit() {
    let initial = initial_document();
    let one = set_position(&initial, 1.0, 0.0);
    let mut core = guest(Role::Editor);
    let commit = owner_commit(1, 1, &initial, &one);
    assert!(core
        .accept_frame(frame(CollabMessage::Commit(commit.clone())))
        .unwrap()
        .is_empty());

    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Snapshot(Box::new(snapshot(
            &initial,
            CommitSeq(0),
        )))))
        .unwrap(),
    );
    assert!(core.install_pending());
    assert!(matches!(
        core.accept_frame(frame(CollabMessage::Commit(commit))),
        Err(GuestError::InstallPending)
    ));
    let effects = finalize(&mut core, prepared);
    let commit_install = take_install(effects);
    finalize(&mut core, commit_install);
    assert_eq!(core.confirmed_seq(), Some(CommitSeq(1)));
}

#[test]
fn viewer_and_disconnected_guests_are_hard_read_only_and_epoch_change_ends() {
    let initial = initial_document();
    let mut viewer = guest(Role::Viewer);
    install_initial(&mut viewer, &initial);
    assert!(matches!(
        viewer.begin_local_edit(&set_position(&initial, 1.0, 0.0)),
        Err(GuestError::ViewerReadOnly)
    ));

    let mut editor = guest(Role::Editor);
    install_initial(&mut editor, &initial);
    editor.disconnect();
    assert!(matches!(
        editor.begin_local_edit(&set_position(&initial, 1.0, 0.0)),
        Err(GuestError::Disconnected)
    ));
    assert!(matches!(
        editor.resume(
            SessionId::from("session"),
            Epoch(8),
            welcome(Role::Editor, CommitSeq(0))
        ),
        Err(GuestError::WrongEpoch)
    ));
    assert_eq!(editor.state(), GuestConnectionState::Ended);
}

#[test]
fn replacement_session_change_ends_instead_of_retrying_forever() {
    let initial = initial_document();
    let mut editor = guest(Role::Editor);
    install_initial(&mut editor, &initial);
    editor.disconnect();

    assert!(matches!(
        editor.resume(
            SessionId::from("replacement-session"),
            Epoch(7),
            welcome(Role::Editor, CommitSeq(0))
        ),
        Err(GuestError::WrongSession)
    ));
    assert_eq!(editor.state(), GuestConnectionState::Ended);
    assert!(matches!(
        editor.begin_local_edit(&set_position(&initial, 1.0, 0.0)),
        Err(GuestError::SessionEnded)
    ));
}

#[test]
fn same_epoch_resume_resends_the_exact_pending_submit_without_consuming_a_counter() {
    let initial = initial_document();
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let original = extract_submit(
        core.begin_local_edit(&set_position(&initial, 10.0, 0.0))
            .unwrap(),
    );
    assert_eq!(core.next_client_counter(), Some(2));

    core.disconnect();
    let effects = core
        .resume(
            SessionId::from("session"),
            Epoch(7),
            welcome(Role::Editor, CommitSeq(0)),
        )
        .unwrap();
    let replay = take_submit(effects);

    assert_eq!(replay, original);
    assert_eq!(core.next_client_counter(), Some(2));
    assert_eq!(
        core.pending_edit().map(|pending| pending.status()),
        Some(PendingEditStatus::Submitted)
    );
}

#[test]
fn resume_catches_up_before_replaying_the_old_pending_id() {
    let initial = initial_document();
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let original = extract_submit(
        core.begin_local_edit(&set_position(&initial, 10.0, 0.0))
            .unwrap(),
    );

    core.disconnect();
    let effects = core
        .resume(
            SessionId::from("session"),
            Epoch(7),
            welcome(Role::Editor, CommitSeq(1)),
        )
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [GuestEffect::Send(CollabMessage::CatchUp(catch_up))]
            if catch_up.after_seq == CommitSeq(0)
    ));

    let remote = set_position(&initial, 0.0, 5.0);
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(owner_commit(
            1, 1, &initial, &remote,
        ))))
        .unwrap(),
    );
    let effects = finalize(&mut core, prepared);
    let replay = take_submit(effects);

    assert_eq!(replay, original);
    assert_eq!(replay.base_seq, CommitSeq(0));
    assert_eq!(core.next_client_counter(), Some(2));
}

#[test]
fn resumed_awaiting_catch_up_rebases_with_a_fresh_id_only_after_the_verdict() {
    let initial = initial_document();
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let original = extract_submit(
        core.begin_local_edit(&set_position(&initial, 10.0, 0.0))
            .unwrap(),
    );
    let effects = core
        .accept_frame(frame(CollabMessage::Reject(Reject {
            client_op_id: original.client_op_id.clone(),
            owner_seq: CommitSeq(1),
            code: RejectCode::StaleBase,
            details: None,
        })))
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [GuestEffect::Send(CollabMessage::CatchUp(_))]
    ));
    assert_eq!(
        core.pending_edit().map(|pending| pending.status()),
        Some(PendingEditStatus::AwaitingCatchUp)
    );

    core.disconnect();
    let effects = core
        .resume(
            SessionId::from("session"),
            Epoch(7),
            welcome(Role::Editor, CommitSeq(1)),
        )
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [GuestEffect::Send(CollabMessage::CatchUp(_))]
    ));

    let remote = set_position(&initial, 0.0, 5.0);
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(owner_commit(
            1, 1, &initial, &remote,
        ))))
        .unwrap(),
    );
    let effects = finalize(&mut core, prepared);
    let retry = take_submit(effects);

    assert_eq!(retry.client_op_id.local_counter, 2);
    assert_eq!(retry.base_seq, CommitSeq(1));
    assert_eq!(core.next_client_counter(), Some(3));
}

#[test]
fn cached_own_commit_covered_by_resume_snapshot_clears_pending_and_restores_undo() {
    let initial = initial_document();
    let desired = set_position(&initial, 10.0, 0.0);
    let latest = set_position(&initial, 10.0, 5.0);
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let original = extract_submit(core.begin_local_edit(&desired).unwrap());
    let cached_commit = Commit {
        client_op_id: original.client_op_id.clone(),
        seq: CommitSeq(1),
        author: CommitAuthor {
            participant_id: ParticipantId::from("participant-guest"),
            peer_id: PeerId::from("guest"),
        },
        txn: original.txn.clone(),
        doc_hash: canonical_document_hash(&desired).unwrap(),
    };

    core.disconnect();
    let effects = core
        .resume(
            SessionId::from("session"),
            Epoch(7),
            welcome(Role::Editor, CommitSeq(2)),
        )
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [GuestEffect::Send(CollabMessage::CatchUp(_))]
    ));
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Snapshot(Box::new(snapshot(
            &latest,
            CommitSeq(2),
        )))))
        .unwrap(),
    );
    let effects = finalize(&mut core, prepared);
    assert_eq!(take_submit(effects), original);

    let effects = core
        .accept_frame(frame(CollabMessage::Commit(cached_commit)))
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [GuestEffect::Send(CollabMessage::Applied(applied))]
            if applied.through_seq == CommitSeq(2)
    ));
    assert!(core.pending_edit().is_none());
    assert_eq!(core.undo_targets(), vec![original.client_op_id]);
    assert_eq!(
        canonical_document_hash(core.displayed_document().unwrap()).unwrap(),
        canonical_document_hash(&latest).unwrap()
    );
}

#[test]
fn undo_requests_use_a_counter_domain_independent_from_submits() {
    let initial = initial_document();
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let desired = set_position(&initial, 4.0, 0.0);
    let submit = extract_submit(core.begin_local_edit(&desired).unwrap());
    let target = submit.client_op_id.clone();
    let commit = Commit {
        client_op_id: target.clone(),
        seq: CommitSeq(1),
        author: CommitAuthor {
            participant_id: ParticipantId::from("participant-guest"),
            peer_id: PeerId::from("guest"),
        },
        txn: submit.txn,
        doc_hash: canonical_document_hash(&desired).unwrap(),
    };
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(commit)))
            .unwrap(),
    );
    finalize(&mut core, prepared);

    let effect = core.request_undo(target.clone()).unwrap();
    let GuestEffect::Send(CollabMessage::UndoRequest(request)) = effect else {
        panic!("selective undo emits UndoRequest");
    };
    assert_eq!(request.request_id.local_counter, 1);
    assert_eq!(request.target_client_op_id, target);
    assert_eq!(core.next_undo_counter(), Some(2));
    assert_eq!(
        core.next_client_counter(),
        Some(2),
        "undo must not consume the Submit idempotency counter"
    );
}

#[test]
fn structural_pending_replays_over_unrelated_remote_property_change() {
    let initial: PenDocument = serde_json::from_str(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"a","x":0},
            {"type":"ellipse","id":"b"}
        ]}"#,
    )
    .unwrap();
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let mut reordered = initial.clone();
    reordered.children.swap(0, 1);
    core.begin_local_edit(&reordered).unwrap();

    let mut remote = initial.clone();
    let PenNode::Rectangle(rectangle) = &mut remote.children[0] else {
        panic!("fixture is a rectangle");
    };
    rectangle.base.x = Some(25.0);
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(owner_commit(
            1, 1, &initial, &remote,
        ))))
        .unwrap(),
    );
    finalize(&mut core, prepared);

    let displayed = core.displayed_document().unwrap();
    assert_eq!(
        serde_json::to_value(&displayed.children[0]).unwrap()["id"],
        "b"
    );
    assert_eq!(
        serde_json::to_value(&displayed.children[1]).unwrap()["id"],
        "a"
    );
    let PenNode::Rectangle(rectangle) = &displayed.children[1] else {
        panic!("moved fixture remains a rectangle");
    };
    assert_eq!(rectangle.base.x, Some(25.0));
    assert!(core.pending_edit().is_some());
}

#[test]
fn structural_hash_conflict_cancels_instead_of_overwriting_remote_change() {
    let initial = initial_document();
    let mut core = guest(Role::Editor);
    install_initial(&mut core, &initial);
    let mut deleted = initial.clone();
    deleted.children.clear();
    core.begin_local_edit(&deleted).unwrap();

    let remote = set_position(&initial, 30.0, 0.0);
    let prepared = take_install(
        core.accept_frame(frame(CollabMessage::Commit(owner_commit(
            1, 1, &initial, &remote,
        ))))
        .unwrap(),
    );
    let effects = finalize(&mut core, prepared);
    assert!(core.pending_edit().is_none());
    assert!(effects.iter().any(|effect| matches!(
        effect,
        GuestEffect::PendingCancelled {
            reason: PendingCancelReason::StructuralConflict,
            ..
        }
    )));
    assert_eq!(
        canonical_document_hash(core.displayed_document().unwrap()).unwrap(),
        canonical_document_hash(&remote).unwrap()
    );
}
