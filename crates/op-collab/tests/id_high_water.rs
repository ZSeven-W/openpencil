use jian_ops_schema::{node::PenNode, PenDocument};
use op_collab::{
    apply_txn_tracked, AdmissionGrant, ApplyContext, ClientOpId, CollabApplyError, CollabMessage,
    CollabOp, CollabTxn, CommitSeq, ConnectionKey, ConnectionPrincipal, Epoch, FrameEnvelope,
    OwnerEffect, OwnerSessionConfig, OwnerSessionCore, PageRef, ParticipantId, PeerId,
    PeerNamespace, RejectCode, Role, SessionError, SessionId, Submit, VerifiedAuthMetadata,
};

fn document(json: &str) -> PenDocument {
    serde_json::from_str(json).unwrap()
}

fn node(json: &str) -> PenNode {
    serde_json::from_str(json).unwrap()
}

fn namespace_context(next: Option<u64>) -> ApplyContext {
    ApplyContext::for_peer_namespace_at(
        PeerNamespace::try_from("guest").unwrap(),
        Role::Editor,
        next,
    )
}

#[test]
fn tracked_apply_accepts_unordered_authored_counters_and_retains_maximum() {
    let initial = document(r#"{"version":"1.0","children":[]}"#);
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: node(
            r#"{"type":"group","id":"c_guest_5","children":[
                {"type":"rectangle","id":"c_guest_2"},
                {"type":"ellipse","id":"c_guest_4"}
            ]}"#,
        ),
    }]);
    let applied = apply_txn_tracked(&initial, &txn, &namespace_context(Some(2))).unwrap();
    assert_eq!(applied.next_id_counter, Some(6));
}

#[test]
fn counters_below_high_water_and_terminal_u64_are_rejected() {
    let initial = document(r#"{"version":"1.0","children":[]}"#);
    let behind = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: node(r#"{"type":"rectangle","id":"c_guest_4"}"#),
    }]);
    assert!(matches!(
        apply_txn_tracked(&initial, &behind, &namespace_context(Some(5))),
        Err(CollabApplyError::IdCounterBehind {
            actual: 4,
            minimum: 5,
            ..
        })
    ));

    let exhausted = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: node(&format!(
            r#"{{"type":"rectangle","id":"c_guest_{}"}}"#,
            u64::MAX
        )),
    }]);
    assert!(matches!(
        apply_txn_tracked(&initial, &exhausted, &namespace_context(Some(u64::MAX))),
        Err(CollabApplyError::IdCounterExhausted { .. })
    ));
}

#[test]
fn replace_ignores_preserved_ids_and_tracks_only_new_descendants() {
    let initial = document(
        r#"{"version":"1.0","children":[
            {"type":"group","id":"base","children":[
                {"type":"rectangle","id":"old"}
            ]}
        ]}"#,
    );
    let txn = CollabTxn::new(vec![CollabOp::ReplaceExact {
        page: PageRef::DocumentRoot,
        node_id: "base".into(),
        expected_hash: op_collab::canonical_node_hash(&initial.children[0]).unwrap(),
        node: node(
            r#"{"type":"group","id":"base","children":[
                {"type":"rectangle","id":"old"},
                {"type":"ellipse","id":"c_guest_7"}
            ]}"#,
        ),
    }]);
    let applied = apply_txn_tracked(&initial, &txn, &namespace_context(Some(7))).unwrap();
    assert_eq!(applied.next_id_counter, Some(8));
}

fn principal(peer: &str, role: Role) -> ConnectionPrincipal {
    ConnectionPrincipal::from_verified(
        VerifiedAuthMetadata {
            issuer: "issuer".into(),
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
    )
}

fn owner(initial: &PenDocument) -> (OwnerSessionCore, ConnectionKey) {
    let owner_connection = ConnectionKey::new(1).unwrap();
    let guest_connection = ConnectionKey::new(2).unwrap();
    let mut owner = OwnerSessionCore::new(
        SessionId::from("session"),
        Epoch(1),
        CommitSeq(0),
        owner_connection,
        AdmissionGrant::new(
            principal("owner", Role::Owner),
            PeerNamespace::try_from("owner").unwrap(),
        ),
        initial,
        OwnerSessionConfig::default(),
    )
    .unwrap();
    owner
        .activate_peer(
            guest_connection,
            AdmissionGrant::new(
                principal("guest", Role::Editor),
                PeerNamespace::try_from("guest").unwrap(),
            ),
            initial,
        )
        .unwrap();
    (owner, guest_connection)
}

fn insert_submit(counter: u64, id_counter: u64, base_seq: u64) -> Submit {
    Submit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from("guest"),
            local_counter: counter,
        },
        base_seq: CommitSeq(base_seq),
        txn: CollabTxn::new(vec![CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 0,
            node: node(&format!(
                r#"{{"type":"rectangle","id":"c_guest_{id_counter}"}}"#
            )),
        }]),
    }
}

fn submit_frame(submit: Submit) -> FrameEnvelope {
    FrameEnvelope::new(
        SessionId::from("session"),
        Epoch(1),
        CollabMessage::Submit(submit),
    )
}

#[test]
fn session_high_water_advances_only_when_install_finalizes_and_survives_resume() {
    let initial = document(r#"{"version":"1.0","children":[]}"#);
    let (mut owner, connection) = owner(&initial);
    let submit = insert_submit(1, 5, 0);
    let prepared = owner
        .accept_frame(connection, submit_frame(submit.clone()), &initial)
        .unwrap()
        .into_iter()
        .find_map(|effect| match effect {
            OwnerEffect::PrepareInstall(prepared) => Some(*prepared),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        owner
            .peer_progress(&PeerId::from("guest"))
            .unwrap()
            .next_id_counter,
        Some(0)
    );
    owner.abort_prepare(prepared).unwrap();

    let mut prepared = owner
        .accept_frame(connection, submit_frame(submit), &initial)
        .unwrap()
        .into_iter()
        .find_map(|effect| match effect {
            OwnerEffect::PrepareInstall(prepared) => Some(*prepared),
            _ => None,
        })
        .unwrap();
    let installed = prepared.take_candidate_document().unwrap();
    let hash = prepared.candidate_hash();
    owner.finalize_install(prepared, hash).unwrap();
    assert_eq!(
        owner
            .peer_progress(&PeerId::from("guest"))
            .unwrap()
            .next_id_counter,
        Some(6)
    );

    owner.disconnect(connection).unwrap();
    owner
        .resume_peer(
            ConnectionKey::new(3).unwrap(),
            AdmissionGrant::new(
                principal("guest", Role::Editor),
                PeerNamespace::try_from("guest").unwrap(),
            ),
        )
        .unwrap();
    assert_eq!(
        owner
            .peer_progress(&PeerId::from("guest"))
            .unwrap()
            .next_id_counter,
        Some(6)
    );
    assert_eq!(installed.children.len(), 1);
}

#[test]
fn unsupported_candidate_does_not_advance_id_high_water() {
    let initial = document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"base"}]}"#);
    let (mut owner, connection) = owner(&initial);
    let submit = Submit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from("guest"),
            local_counter: 1,
        },
        base_seq: CommitSeq(0),
        txn: CollabTxn::new(vec![
            CollabOp::InsertExact {
                page: PageRef::DocumentRoot,
                parent_id: None,
                index: 1,
                node: node(r#"{"type":"rectangle","id":"c_guest_5"}"#),
            },
            CollabOp::ReplaceExact {
                page: PageRef::DocumentRoot,
                node_id: "base".into(),
                expected_hash: op_collab::canonical_node_hash(&initial.children[0]).unwrap(),
                node: node(r#"{"type":"rectangle","id":"base","visible":false}"#),
            },
        ]),
    };
    let effects = owner
        .accept_frame(connection, submit_frame(submit), &initial)
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [OwnerEffect::Reply {
            message: CollabMessage::Reject(reject),
            ..
        }] if reject.code == RejectCode::UnsupportedEdit
    ));
    assert_eq!(
        owner
            .peer_progress(&PeerId::from("guest"))
            .unwrap()
            .next_id_counter,
        Some(0)
    );
}

#[test]
fn owner_and_activation_reject_namespaces_already_present_in_document() {
    let owner_collision =
        document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"c_owner_0"}]}"#);
    assert!(matches!(
        OwnerSessionCore::new(
            SessionId::from("session"),
            Epoch(1),
            CommitSeq(0),
            ConnectionKey::new(1).unwrap(),
            AdmissionGrant::new(
                principal("owner", Role::Owner),
                PeerNamespace::try_from("owner").unwrap()
            ),
            &owner_collision,
            OwnerSessionConfig::default()
        ),
        Err(SessionError::NamespaceAlreadyPresentInDocument)
    ));

    let guest_collision =
        document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"c_guest_0"}]}"#);
    let mut owner = OwnerSessionCore::new(
        SessionId::from("session"),
        Epoch(1),
        CommitSeq(0),
        ConnectionKey::new(1).unwrap(),
        AdmissionGrant::new(
            principal("owner", Role::Owner),
            PeerNamespace::try_from("owner").unwrap(),
        ),
        &guest_collision,
        OwnerSessionConfig::default(),
    )
    .unwrap();
    assert!(matches!(
        owner.activate_peer(
            ConnectionKey::new(2).unwrap(),
            AdmissionGrant::new(
                principal("guest", Role::Editor),
                PeerNamespace::try_from("guest").unwrap()
            ),
            &guest_collision
        ),
        Err(SessionError::NamespaceAlreadyPresentInDocument)
    ));
}
