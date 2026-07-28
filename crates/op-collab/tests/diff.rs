use jian_ops_schema::{node::PenNode, PenDocument};
use op_collab::{
    apply_txn, canonical_document_hash, canonical_node_hash, diff_supported, AdmissionGrant,
    ApplyContext, ClientOpId, CollabMessage, CollabOp, CollabTxn, CommitSeq, ConnectionKey,
    ConnectionPrincipal, DiffContext, DiffError, EditChanges, Epoch, FrameEnvelope, OwnerEffect,
    OwnerSessionConfig, OwnerSessionCore, PageRef, ParticipantId, PeerId, PeerNamespace,
    RejectCode, Role, SessionId, Submit, VerifiedAuthMetadata,
};

fn document(json: &str) -> PenDocument {
    serde_json::from_str(json).unwrap()
}

fn context() -> DiffContext {
    DiffContext::new(
        PeerNamespace::try_from("guest").unwrap(),
        Role::Editor,
        Some(0),
    )
}

fn replay(before: &PenDocument, txn: &CollabTxn) -> PenDocument {
    apply_txn(
        before,
        txn,
        &ApplyContext::for_peer_namespace(PeerNamespace::try_from("guest").unwrap(), Role::Editor),
    )
    .unwrap()
}

#[test]
fn property_diff_is_deterministic_and_replay_hash_is_exact() {
    let before = document(
        r##"{"version":"1.0","children":[
            {"type":"rectangle","id":"base","x":0,"fill":[{"type":"solid","color":"#000000"}]}
        ]}"##,
    );
    let after = document(
        r##"{"version":"1.0","children":[
            {"type":"rectangle","id":"base","x":12,"fill":[{"type":"solid","color":"#ffffff"}]}
        ]}"##,
    );
    let first = diff_supported(&before, &after, &context()).unwrap();
    let second = diff_supported(&before, &after, &context()).unwrap();
    assert_eq!(first.txn, second.txn);
    let EditChanges::Property(changes) = first.changes else {
        panic!("property-only edit");
    };
    assert_eq!(changes.len(), 2);
    assert_eq!(first.txn.ops.len(), 1, "one exact replace per changed node");
    assert_eq!(
        canonical_document_hash(&replay(&before, &first.txn)).unwrap(),
        canonical_document_hash(&after).unwrap()
    );
}

#[test]
fn structural_diff_reproduces_insert_delete_and_reorder() {
    let before = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"a"},
            {"type":"ellipse","id":"b"}
        ]}"#,
    );
    let after = document(
        r#"{"version":"1.0","children":[
            {"type":"ellipse","id":"b"},
            {"type":"rectangle","id":"c_guest_0"}
        ]}"#,
    );
    let supported = diff_supported(&before, &after, &context()).unwrap();
    let EditChanges::Structure(changes) = &supported.changes else {
        panic!("structural edit");
    };
    assert_eq!(changes.inserted_ids, ["c_guest_0"]);
    assert_eq!(changes.deleted_ids, ["a"]);
    assert!(!changes.moves.is_empty());
    assert_eq!(
        canonical_document_hash(&replay(&before, &supported.txn)).unwrap(),
        canonical_document_hash(&after).unwrap()
    );
}

#[test]
fn unsupported_fields_values_and_mixed_edits_fail_closed() {
    let before = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"base","x":0}
        ]}"#,
    );
    let hidden = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"base","x":0,"visible":false}
        ]}"#,
    );
    assert!(matches!(
        diff_supported(&before, &hidden, &context()),
        Err(DiffError::UnsupportedNodeField { field, .. }) if field == "visible"
    ));

    let image_fill = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"base","x":0,
             "fill":[{"type":"image","url":"data:image/png;base64,AA=="}]}
        ]}"#,
    );
    assert!(matches!(
        diff_supported(&before, &image_fill, &context()),
        Err(DiffError::UnsupportedFieldValue { field, .. }) if field == "fill"
    ));

    let mixed = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"base","x":1},
            {"type":"rectangle","id":"c_guest_0"}
        ]}"#,
    );
    assert!(matches!(
        diff_supported(&before, &mixed, &context()),
        Err(DiffError::MixedPropertyAndStructure)
    ));
}

#[test]
fn document_and_page_metadata_changes_are_not_encoded_as_node_ops() {
    let before = document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"base"}]}"#);
    let version = document(r#"{"version":"2.0","children":[{"type":"rectangle","id":"base"}]}"#);
    assert!(matches!(
        diff_supported(&before, &version, &context()),
        Err(DiffError::UnsupportedDocumentField { field }) if field == "version"
    ));

    let page_before = document(
        r#"{"version":"1.0","pages":[
            {"id":"page","name":"Before","children":[{"type":"rectangle","id":"base"}]}
        ],"children":[]}"#,
    );
    let page_after = document(
        r#"{"version":"1.0","pages":[
            {"id":"page","name":"After","children":[{"type":"rectangle","id":"base"}]}
        ],"children":[]}"#,
    );
    assert!(matches!(
        diff_supported(&page_before, &page_after, &context()),
        Err(DiffError::UnsupportedPageChange)
    ));
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

#[test]
fn owner_rejects_handcrafted_unsupported_replace_and_consumes_result() {
    let initial = document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"base"}]}"#);
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
        &initial,
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
            &initial,
        )
        .unwrap();
    let replacement: PenNode =
        serde_json::from_str(r#"{"type":"rectangle","id":"base","visible":false}"#).unwrap();
    let submit = Submit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from("guest"),
            local_counter: 1,
        },
        base_seq: CommitSeq(0),
        txn: CollabTxn::new(vec![CollabOp::ReplaceExact {
            page: PageRef::DocumentRoot,
            node_id: "base".into(),
            expected_hash: canonical_node_hash(&initial.children[0]).unwrap(),
            node: replacement,
        }]),
    };
    let effects = owner
        .accept_frame(
            guest_connection,
            FrameEnvelope::new(
                SessionId::from("session"),
                Epoch(1),
                CollabMessage::Submit(submit),
            ),
            &initial,
        )
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
            .next_counter,
        Some(2)
    );
    assert_eq!(owner.seq(), CommitSeq(0));
    assert!(!owner.install_pending());
}

#[test]
fn group_and_ungroup_are_structural_transactions_with_exact_replay() {
    let flat = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"a"},
            {"type":"ellipse","id":"b"}
        ]}"#,
    );
    let grouped = document(
        r#"{"version":"1.0","children":[
            {"type":"group","id":"c_guest_0","children":[
                {"type":"rectangle","id":"a"},
                {"type":"ellipse","id":"b"}
            ]}
        ]}"#,
    );
    let group = diff_supported(&flat, &grouped, &context()).unwrap();
    assert_eq!(
        canonical_document_hash(&replay(&flat, &group.txn)).unwrap(),
        canonical_document_hash(&grouped).unwrap()
    );

    let ungroup = diff_supported(
        &grouped,
        &flat,
        &DiffContext::new(
            PeerNamespace::try_from("guest").unwrap(),
            Role::Editor,
            Some(1),
        ),
    )
    .unwrap();
    assert_eq!(
        canonical_document_hash(
            &apply_txn(
                &grouped,
                &ungroup.txn,
                &ApplyContext::for_peer_namespace_at(
                    PeerNamespace::try_from("guest").unwrap(),
                    Role::Editor,
                    Some(1),
                ),
            )
            .unwrap()
        )
        .unwrap(),
        canonical_document_hash(&flat).unwrap()
    );
}
