use jian_ops_schema::{
    image_table::with_save_scope, image_thumbs::ImageThumbSnapshot, node::PenNode, PenDocument,
};
use op_collab::{
    canonical_document_hash, canonical_node_hash, encode_renew_ticket_frame_to_zeroizing_json,
    Applied, Bye, ByeReason, CatchUp, ClientOpId, CollabMessage, CollabOp, CollabTxn, Commit,
    CommitAuthor, CommitSeq, ConnectionPrincipal, Epoch, FrameEnvelope, OpaqueTicket, PageRef,
    Participant, ParticipantId, ParticipantLeft, ParticipantPresence, PeerId, Point, Presence,
    Reject, RejectCode, Role, SessionId, Snapshot, Submit, UndoOutcome, UndoRequest, UndoRequestId,
    UndoResult, VerifiedAuthMetadata, Viewport, Welcome, WireLimits, CANONICAL_HASH_VERSION,
    MAX_OPAQUE_TICKET_BYTES, MAX_TREE_DEPTH,
};

fn peer_op(counter: u64) -> ClientOpId {
    ClientOpId {
        peer_id: PeerId("peer-a".into()),
        local_counter: counter,
    }
}

fn undo_request(counter: u64) -> UndoRequestId {
    UndoRequestId {
        peer_id: PeerId("peer-a".into()),
        local_counter: counter,
    }
}

fn test_node() -> PenNode {
    serde_json::from_str(r#"{"type":"rectangle","id":"c_ns_1","name":"Box"}"#).unwrap()
}

fn nested_frame_document(depth: usize) -> PenDocument {
    let mut child = None;
    for level in (0..depth).rev() {
        let mut node: PenNode =
            serde_json::from_str(&format!(r#"{{"type":"frame","id":"deep-{level}"}}"#)).unwrap();
        let PenNode::Frame(frame) = &mut node else {
            unreachable!("the fixture always builds frames");
        };
        frame.children = child.take().map(|node| vec![node]);
        child = Some(node);
    }
    let mut document: PenDocument =
        serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap();
    if let Some(root) = child {
        document.children.push(root);
    }
    document
}

fn nested_state_document(depth: usize) -> PenDocument {
    let mut default = serde_json::json!(0);
    for _ in 0..depth {
        default = serde_json::json!([default]);
    }
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [],
        "state": {
            "deep": {
                "type": "object",
                "default": default
            }
        }
    }))
    .unwrap()
}

fn collected_image_reference(source: &str) -> String {
    let document: PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{"type": "image", "id": "image-1", "src": source}]
    }))
    .unwrap();
    with_save_scope(&ImageThumbSnapshot::default(), |tables| {
        serde_json::to_value(&document).unwrap();
        let table = serde_json::to_value(tables.images()).unwrap();
        let id = table
            .as_object()
            .and_then(|images| images.keys().next())
            .expect("large fixture source must be collected");
        format!("op-image:{id}")
    })
}

fn test_txn() -> CollabTxn {
    CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: test_node(),
    }])
}

fn frame(body: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId("session-a".into()), Epoch(7), body)
}

fn participant() -> Participant {
    Participant {
        participant_id: ParticipantId("participant-a".into()),
        peer_id: PeerId("peer-a".into()),
        role: Role::Editor,
        display_name: None,
        avatar_url: None,
    }
}

#[test]
fn every_message_kind_round_trips_in_the_common_envelope() {
    let document: PenDocument = serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap();
    let txn = test_txn();
    let messages = vec![
        CollabMessage::Welcome(Welcome {
            participant_id: ParticipantId("participant-a".into()),
            peer_id: PeerId("peer-a".into()),
            role: Role::Editor,
            seq: CommitSeq(3),
            peer_namespace: "ns".into(),
            document_schema_version: "1.0".into(),
            hash_version: CANONICAL_HASH_VERSION,
            limits: WireLimits::default(),
            participants: vec![participant()],
        }),
        CollabMessage::Submit(Submit {
            client_op_id: peer_op(1),
            base_seq: CommitSeq(3),
            txn: txn.clone(),
        }),
        CollabMessage::Commit(Commit {
            client_op_id: peer_op(1),
            seq: CommitSeq(4),
            author: CommitAuthor {
                participant_id: ParticipantId("participant-a".into()),
                peer_id: PeerId("peer-a".into()),
            },
            txn: txn.clone(),
            doc_hash: canonical_document_hash(&document).unwrap(),
        }),
        CollabMessage::Reject(Reject {
            client_op_id: peer_op(2),
            owner_seq: CommitSeq(4),
            code: RejectCode::StaleBase,
            details: None,
        }),
        CollabMessage::CatchUp(CatchUp {
            after_seq: CommitSeq(1),
        }),
        CollabMessage::Snapshot(Box::new(Snapshot {
            seq: CommitSeq(4),
            doc_hash: canonical_document_hash(&document).unwrap(),
            document,
        })),
        CollabMessage::Applied(Applied {
            through_seq: CommitSeq(4),
        }),
        CollabMessage::UndoRequest(UndoRequest {
            request_id: undo_request(3),
            target_client_op_id: peer_op(1),
        }),
        CollabMessage::UndoResult(UndoResult {
            request_id: undo_request(3),
            target_client_op_id: peer_op(1),
            owner_seq: CommitSeq(4),
            outcome: UndoOutcome::Committed,
            compensation_client_op_id: Some(peer_op(4)),
            details: None,
        }),
        CollabMessage::PresenceUpdate(Presence {
            cursor: Some(Point { x: 1.0, y: 2.0 }),
            selection: vec!["c_ns_1".into()],
            viewport: Some(Viewport {
                pan_x: 0.0,
                pan_y: 0.0,
                zoom: 1.0,
            }),
            editing_node: Some("c_ns_1".into()),
        }),
        CollabMessage::PresenceChanged(ParticipantPresence {
            participant_id: ParticipantId("participant-a".into()),
            peer_id: PeerId("peer-a".into()),
            presence: Presence {
                cursor: None,
                selection: vec![],
                viewport: None,
                editing_node: None,
            },
        }),
        CollabMessage::ParticipantJoined(participant()),
        CollabMessage::ParticipantLeft(ParticipantLeft {
            participant_id: ParticipantId("participant-a".into()),
            peer_id: PeerId("peer-a".into()),
        }),
        CollabMessage::Bye(Bye {
            reason: ByeReason::Normal,
        }),
    ];

    for message in messages {
        let original = frame(message);
        let bytes = original.to_json_vec().unwrap();
        let decoded = FrameEnvelope::from_json_slice(&bytes).unwrap();
        assert_eq!(decoded, original);
    }
}

#[test]
fn maximum_tree_depth_snapshot_round_trips_and_next_depth_is_rejected() {
    let document = nested_frame_document(MAX_TREE_DEPTH as usize);
    let doc_hash = canonical_document_hash(&document).unwrap();
    let original = frame(CollabMessage::Snapshot(Box::new(Snapshot {
        seq: CommitSeq(1),
        doc_hash,
        document,
    })));
    let bytes = original.to_json_vec().unwrap();
    let decoded = FrameEnvelope::from_json_slice(&bytes).unwrap();
    assert_eq!(decoded, original);

    let too_deep = nested_frame_document(MAX_TREE_DEPTH as usize + 1);
    let too_deep_hash = canonical_document_hash(&too_deep).unwrap();
    let invalid = frame(CollabMessage::Snapshot(Box::new(Snapshot {
        seq: CommitSeq(1),
        doc_hash: too_deep_hash,
        document: too_deep,
    })));
    assert!(matches!(
        invalid.to_json_vec(),
        Err(op_collab::ProtocolError::InvalidSnapshot(
            op_collab::SnapshotError::InvalidDocument(
                op_collab::CollabApplyError::TreeTooDeep { .. }
            )
        ))
    ));
}

#[test]
fn version_and_unknown_shapes_fail_closed() {
    let original = frame(CollabMessage::Applied(Applied {
        through_seq: CommitSeq(1),
    }));
    let mut value: serde_json::Value =
        serde_json::from_slice(&original.to_json_vec().unwrap()).unwrap();
    value["protocolVersion"] = serde_json::json!(99);
    let error = FrameEnvelope::from_json_slice(&serde_json::to_vec(&value).unwrap()).unwrap_err();
    assert!(matches!(
        error,
        op_collab::ProtocolError::UnsupportedVersion { actual: 99, .. }
    ));

    value["protocolVersion"] = serde_json::json!(1);
    value["unexpected"] = serde_json::json!(true);
    assert!(FrameEnvelope::from_json_slice(&serde_json::to_vec(&value).unwrap()).is_err());

    value.as_object_mut().unwrap().remove("unexpected");
    value["body"]["type"] = serde_json::json!("future_message");
    assert!(FrameEnvelope::from_json_slice(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn transaction_limits_apply_to_inbound_and_outbound_frames() {
    let mut oversized_op_count = test_txn();
    oversized_op_count
        .ops
        .push(oversized_op_count.ops[0].clone());
    let submit = frame(CollabMessage::Submit(Submit {
        client_op_id: peer_op(1),
        base_seq: CommitSeq(0),
        txn: oversized_op_count,
    }));
    let strict_ops = WireLimits {
        max_envelope_bytes: 64 * 1024 * 1024,
        max_txn_bytes: 4 * 1024 * 1024,
        max_ops_per_txn: 1,
        ..WireLimits::default()
    };
    assert!(matches!(
        submit.to_json_vec_with_limits(strict_ops),
        Err(op_collab::ProtocolError::TooManyOperations { .. })
    ));

    let bytes = submit.to_json_vec().unwrap();
    assert!(matches!(
        FrameEnvelope::from_json_slice_with_limits(&bytes, strict_ops),
        Err(op_collab::ProtocolError::TooManyOperations { .. })
    ));

    let strict_bytes = WireLimits {
        max_envelope_bytes: 64 * 1024 * 1024,
        max_txn_bytes: 1,
        max_ops_per_txn: 1_024,
        ..WireLimits::default()
    };
    assert!(matches!(
        submit.to_json_vec_with_limits(strict_bytes),
        Err(op_collab::ProtocolError::TransactionTooLarge { .. })
    ));
}

#[test]
fn envelope_and_semantic_validation_fail_before_use() {
    let valid = frame(CollabMessage::Applied(Applied {
        through_seq: CommitSeq(1),
    }));
    let bytes = valid.to_json_vec().unwrap();
    let tiny = WireLimits {
        max_envelope_bytes: 1,
        ..WireLimits::default()
    };
    assert!(matches!(
        FrameEnvelope::from_json_slice_with_limits(&bytes, tiny),
        Err(op_collab::ProtocolError::EnvelopeTooLarge { .. })
    ));
    assert!(matches!(
        valid.to_json_vec_with_limits(tiny),
        Err(op_collab::ProtocolError::EnvelopeTooLarge { .. })
    ));

    let empty_session = FrameEnvelope::new(
        SessionId(String::new()),
        Epoch(1),
        CollabMessage::Applied(Applied {
            through_seq: CommitSeq(1),
        }),
    );
    assert!(matches!(
        empty_session.to_json_vec(),
        Err(op_collab::ProtocolError::EmptyIdentifier {
            field: "session_id"
        })
    ));
    let strict_identifiers = WireLimits {
        max_identifier_bytes: 4,
        ..WireLimits::default()
    };
    assert!(matches!(
        valid.to_json_vec_with_limits(strict_identifiers),
        Err(op_collab::ProtocolError::IdentifierTooLong {
            field: "session_id",
            ..
        })
    ));
    assert!(matches!(
        FrameEnvelope::from_json_slice_with_limits(&bytes, strict_identifiers),
        Err(op_collab::ProtocolError::IdentifierTooLong {
            field: "session_id",
            ..
        })
    ));

    let invalid_presence = frame(CollabMessage::PresenceUpdate(Presence {
        cursor: None,
        selection: vec![],
        viewport: Some(Viewport {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 0.0,
        }),
        editing_node: None,
    }));
    assert!(matches!(
        invalid_presence.to_json_vec(),
        Err(op_collab::ProtocolError::InvalidPresence {
            field: "viewport.zoom"
        })
    ));

    let invalid_hash = frame(CollabMessage::Welcome(Welcome {
        participant_id: ParticipantId("participant-a".into()),
        peer_id: PeerId("peer-a".into()),
        role: Role::Editor,
        seq: CommitSeq(0),
        peer_namespace: "ns".into(),
        document_schema_version: "1.0".into(),
        hash_version: CANONICAL_HASH_VERSION + 1,
        limits: WireLimits::default(),
        participants: vec![],
    }));
    assert!(matches!(
        invalid_hash.to_json_vec(),
        Err(op_collab::ProtocolError::UnsupportedHashVersion { .. })
    ));

    let invalid_limits = frame(CollabMessage::Welcome(Welcome {
        participant_id: ParticipantId("participant-a".into()),
        peer_id: PeerId("peer-a".into()),
        role: Role::Editor,
        seq: CommitSeq(0),
        peer_namespace: "ns".into(),
        document_schema_version: "1.0".into(),
        hash_version: CANONICAL_HASH_VERSION,
        limits: WireLimits {
            max_txn_bytes: 0,
            ..WireLimits::default()
        },
        participants: vec![],
    }));
    assert!(matches!(
        invalid_limits.to_json_vec(),
        Err(op_collab::ProtocolError::InvalidWireLimit {
            field: "max_txn_bytes",
            ..
        })
    ));

    let invalid_namespace = frame(CollabMessage::Welcome(Welcome {
        participant_id: ParticipantId("participant-a".into()),
        peer_id: PeerId("peer-a".into()),
        role: Role::Editor,
        seq: CommitSeq(0),
        peer_namespace: "not_a_namespace".into(),
        document_schema_version: "1.0".into(),
        hash_version: CANONICAL_HASH_VERSION,
        limits: WireLimits::default(),
        participants: vec![],
    }));
    assert!(matches!(
        invalid_namespace.to_json_vec(),
        Err(op_collab::ProtocolError::InvalidPeerNamespace(_))
    ));
}

#[test]
fn json_nesting_budget_ignores_strings_and_rejects_structural_depth() {
    let text_with_delimiters = "[{".repeat(100);
    let original = frame(CollabMessage::Reject(Reject {
        client_op_id: peer_op(1),
        owner_seq: CommitSeq(1),
        code: RejectCode::InvalidOperation,
        details: Some(text_with_delimiters),
    }));
    let bytes = original.to_json_vec().unwrap();
    assert_eq!(FrameEnvelope::from_json_slice(&bytes).unwrap(), original);

    let over_nested = format!("{}0{}", "[".repeat(49), "]".repeat(49));
    assert!(matches!(
        FrameEnvelope::from_json_slice(over_nested.as_bytes()),
        Err(op_collab::ProtocolError::JsonNestingTooDeep {
            actual: 49,
            limit: 48
        })
    ));

    let document = nested_state_document(49);
    let too_nested = frame(CollabMessage::Snapshot(Box::new(Snapshot {
        seq: CommitSeq(1),
        doc_hash: canonical_document_hash(&document).unwrap(),
        document,
    })));
    assert!(matches!(
        too_nested.to_json_vec(),
        Err(op_collab::ProtocolError::JsonNestingTooDeep { .. })
    ));
}

#[test]
fn ticket_debug_is_redacted_and_size_is_bounded() {
    let ticket = OpaqueTicket::new("highly-sensitive-ticket".into()).unwrap();
    assert_eq!(format!("{ticket:?}"), "OpaqueTicket([REDACTED])");
    assert!(!format!("{ticket:?}").contains("sensitive"));

    assert!(matches!(
        OpaqueTicket::new(String::new()),
        Err(op_collab::OpaqueTicketError::Empty)
    ));
    let error = OpaqueTicket::new("x".repeat(MAX_OPAQUE_TICKET_BYTES + 1)).unwrap_err();
    assert!(matches!(
        error,
        op_collab::OpaqueTicketError::TooLarge { .. }
    ));
}

#[test]
fn wire_rejects_non_finite_transactions_and_snapshots() {
    let mut invalid_node = test_node();
    let PenNode::Rectangle(rectangle) = &mut invalid_node else {
        unreachable!()
    };
    rectangle.base.x = Some(f64::NAN);
    let submit = frame(CollabMessage::Submit(Submit {
        client_op_id: peer_op(1),
        base_seq: CommitSeq(0),
        txn: CollabTxn::new(vec![CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 0,
            node: invalid_node,
        }]),
    }));
    assert!(matches!(
        submit.to_json_vec(),
        Err(op_collab::ProtocolError::NonFiniteNumber)
    ));

    let mut document: PenDocument =
        serde_json::from_str(r#"{"version":"1.0","children":[{"type":"rectangle","id":"n1"}]}"#)
            .unwrap();
    let PenNode::Rectangle(rectangle) = &mut document.children[0] else {
        unreachable!()
    };
    rectangle.base.x = Some(f64::NEG_INFINITY);
    let snapshot = frame(CollabMessage::Snapshot(Box::new(Snapshot {
        seq: CommitSeq(0),
        document,
        doc_hash: op_collab::CanonicalHash::from_bytes([0; 32]),
    })));
    assert!(matches!(
        snapshot.to_json_vec(),
        Err(op_collab::ProtocolError::InvalidSnapshot(
            op_collab::SnapshotError::NonFiniteNumber
        ))
    ));
}

#[test]
fn snapshot_images_are_inline_and_file_table_refs_are_rejected() {
    let source = format!("data:image/png;base64,{}", "A".repeat(5_000));
    let document: PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{"type": "image", "id": "image-1", "src": source}]
    }))
    .unwrap();
    let snapshot = frame(CollabMessage::Snapshot(Box::new(Snapshot {
        seq: CommitSeq(0),
        doc_hash: canonical_document_hash(&document).unwrap(),
        document,
    })));
    let bytes = snapshot.to_json_vec().unwrap();
    let encoded = String::from_utf8(bytes.clone()).unwrap();
    assert!(encoded.contains("data:image/png;base64,"));
    assert!(!encoded.contains("op-image:"));
    FrameEnvelope::from_json_slice(&bytes).unwrap();

    let mut forged: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    forged["body"]["payload"]["document"]["children"][0]["src"] =
        serde_json::json!("op-image:missing");
    let forged = serde_json::to_vec(&forged).unwrap();
    assert!(matches!(
        FrameEnvelope::from_json_slice(&forged),
        Err(op_collab::ProtocolError::ExternalImageReference)
    ));

    let unresolved_document: PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{"type": "image", "id": "image-1", "src": "op-image:missing"}]
    }))
    .unwrap();
    let unresolved_snapshot = frame(CollabMessage::Snapshot(Box::new(Snapshot {
        seq: CommitSeq(0),
        doc_hash: op_collab::CanonicalHash::from_bytes([0; 32]),
        document: unresolved_document,
    })));
    assert!(matches!(
        unresolved_snapshot.to_json_vec(),
        Err(op_collab::ProtocolError::InvalidSnapshot(
            op_collab::SnapshotError::ExternalImageReference
        ))
    ));

    let collision_reference = collected_image_reference(&source);
    let collision_document: PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [
            {"type": "image", "id": "image-1", "src": source},
            {"type": "image", "id": "image-2", "src": collision_reference}
        ]
    }))
    .unwrap();
    assert!(matches!(
        canonical_document_hash(&collision_document),
        Err(op_collab::CanonicalHashError::ExternalImageReference)
    ));
    let collision_snapshot = frame(CollabMessage::Snapshot(Box::new(Snapshot {
        seq: CommitSeq(0),
        doc_hash: op_collab::CanonicalHash::from_bytes([0; 32]),
        document: collision_document,
    })));
    assert!(matches!(
        collision_snapshot.to_json_vec(),
        Err(op_collab::ProtocolError::InvalidSnapshot(
            op_collab::SnapshotError::ExternalImageReference
        ))
    ));
}

#[test]
fn operation_and_directional_presence_shapes_are_frozen() {
    let hash = op_collab::CanonicalHash::from_bytes([0; 32]);
    let operations = CollabTxn::new(vec![
        CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 7,
            node: test_node(),
        },
        CollabOp::ReplaceExact {
            page: PageRef::PageId("page-a".into()),
            node_id: "c_ns_1".into(),
            expected_hash: hash,
            node: test_node(),
        },
        CollabOp::DeleteExact {
            page: PageRef::DocumentRoot,
            node_id: "c_ns_1".into(),
            expected_hash: hash,
        },
        CollabOp::MoveExact {
            page: PageRef::PageId("page-a".into()),
            node_id: "c_ns_1".into(),
            expected_parent: None,
            expected_index: 1,
            new_parent: Some("group-a".into()),
            new_index: 2,
        },
    ]);
    assert_eq!(
        serde_json::to_value(&operations).unwrap(),
        serde_json::json!({
            "ops": [
                {
                    "op": "insert_exact",
                    "page": {"kind": "document_root"},
                    "parent_id": null,
                    "index": 7,
                    "node": {"type": "rectangle", "id": "c_ns_1", "name": "Box"}
                },
                {
                    "op": "replace_exact",
                    "page": {"kind": "page_id", "id": "page-a"},
                    "node_id": "c_ns_1",
                    "expected_hash": hash.to_string(),
                    "node": {"type": "rectangle", "id": "c_ns_1", "name": "Box"}
                },
                {
                    "op": "delete_exact",
                    "page": {"kind": "document_root"},
                    "node_id": "c_ns_1",
                    "expected_hash": hash.to_string()
                },
                {
                    "op": "move_exact",
                    "page": {"kind": "page_id", "id": "page-a"},
                    "node_id": "c_ns_1",
                    "expected_parent": null,
                    "expected_index": 1,
                    "new_parent": "group-a",
                    "new_index": 2
                }
            ]
        })
    );

    let presence = frame(CollabMessage::PresenceChanged(ParticipantPresence {
        participant_id: ParticipantId("participant-a".into()),
        peer_id: PeerId("peer-a".into()),
        presence: Presence {
            cursor: None,
            selection: vec![],
            viewport: None,
            editing_node: None,
        },
    }));
    let value: serde_json::Value =
        serde_json::from_slice(&presence.to_json_vec().unwrap()).unwrap();
    assert_eq!(value["body"]["type"], "presence_changed");
    assert_eq!(
        value["body"]["payload"],
        serde_json::json!({
            "participantId": "participant-a",
            "peerId": "peer-a",
            "presence": {}
        })
    );
}

#[test]
fn wire_indices_are_fixed_width_u32() {
    let submit = frame(CollabMessage::Submit(Submit {
        client_op_id: peer_op(1),
        base_seq: CommitSeq(0),
        txn: test_txn(),
    }));
    let mut value: serde_json::Value =
        serde_json::from_slice(&submit.to_json_vec().unwrap()).unwrap();
    value["body"]["payload"]["txn"]["ops"][0]["index"] = serde_json::json!(u64::from(u32::MAX) + 1);
    assert!(matches!(
        FrameEnvelope::from_json_slice(&serde_json::to_vec(&value).unwrap()),
        Err(op_collab::ProtocolError::Decode(_))
    ));
}

#[test]
fn welcome_ticket_and_undo_field_names_are_frozen() {
    let welcome = frame(CollabMessage::Welcome(Welcome {
        participant_id: ParticipantId("participant-a".into()),
        peer_id: PeerId("peer-a".into()),
        role: Role::Editor,
        seq: CommitSeq(9),
        peer_namespace: "ns".into(),
        document_schema_version: "1.0".into(),
        hash_version: CANONICAL_HASH_VERSION,
        limits: WireLimits::default(),
        participants: vec![participant()],
    }));
    let welcome: serde_json::Value =
        serde_json::from_slice(&welcome.to_json_vec().unwrap()).unwrap();
    assert_eq!(
        welcome["body"]["payload"],
        serde_json::json!({
            "participantId": "participant-a",
            "peerId": "peer-a",
            "role": "editor",
            "seq": 9,
            "peerNamespace": "ns",
            "documentSchemaVersion": "1.0",
            "hashVersion": CANONICAL_HASH_VERSION,
            "limits": serde_json::to_value(WireLimits::default()).unwrap(),
            "participants": [{
                "participantId": "participant-a",
                "peerId": "peer-a",
                "role": "editor"
            }]
        })
    );

    let renewal_ticket = OpaqueTicket::new("opaque".into()).unwrap();
    let renewal = encode_renew_ticket_frame_to_zeroizing_json(
        &SessionId::from("session-a"),
        Epoch(7),
        &renewal_ticket,
        WireLimits::default(),
    )
    .unwrap();
    let renewal: serde_json::Value = serde_json::from_slice(renewal.as_bytes()).unwrap();
    assert_eq!(
        renewal["body"]["payload"],
        serde_json::json!({"opaqueTicket": "opaque"})
    );

    let undo = frame(CollabMessage::UndoResult(UndoResult {
        request_id: undo_request(7),
        target_client_op_id: peer_op(3),
        owner_seq: CommitSeq(9),
        outcome: UndoOutcome::Conflict,
        compensation_client_op_id: None,
        details: None,
    }));
    let undo: serde_json::Value = serde_json::from_slice(&undo.to_json_vec().unwrap()).unwrap();
    assert_eq!(
        undo["body"]["payload"],
        serde_json::json!({
            "requestId": {"peerId": "peer-a", "localCounter": 7},
            "targetClientOpId": {"peerId": "peer-a", "localCounter": 3},
            "ownerSeq": 9,
            "outcome": "conflict"
        })
    );
}

#[test]
fn verified_identity_debug_redacts_fingerprints() {
    let auth = VerifiedAuthMetadata {
        issuer: "issuer".into(),
        subject: "user-123".into(),
        device_id: "device-456".into(),
        proof_binding: "binding-789".into(),
        expires_at_unix_ms: 42,
        display_name: None,
        avatar_url: None,
    };
    let principal = ConnectionPrincipal::from_verified(
        auth,
        ParticipantId("participant-secret".into()),
        PeerId("peer-secret".into()),
        Role::Editor,
    );

    let debug = format!("{principal:?}");
    for secret in [
        "user-123",
        "device-456",
        "binding-789",
        "participant-secret",
        "peer-secret",
    ] {
        assert!(!debug.contains(secret));
    }
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn node_precondition_hash_uses_node_domain() {
    let txn = test_txn();
    let CollabOp::InsertExact { node, .. } = &txn.ops[0] else {
        unreachable!()
    };
    let encoded = serde_json::to_value(&txn).unwrap();
    let hash = canonical_node_hash(node).unwrap().to_string();
    assert_ne!(hash, encoded.to_string());
}
