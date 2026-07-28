use jian_ops_schema::{node::PenNode, PenDocument};
use op_collab::{
    apply_txn, apply_txn_in_place, canonical_document_hash, canonical_node_hash, validate_document,
    ApplyContext, CollabApplyError, CollabOp, CollabTxn, PageRef, PeerNamespace, Role,
};

fn document(json: &str) -> PenDocument {
    serde_json::from_str(json).expect("valid test document")
}

fn node(json: &str) -> PenNode {
    serde_json::from_str(json).expect("valid test node")
}

fn root_ids(document: &PenDocument) -> Vec<String> {
    serde_json::to_value(document).unwrap()["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect()
}

fn set_x(node: &mut PenNode, value: f64) {
    let PenNode::Rectangle(rectangle) = node else {
        panic!("test node must be a rectangle");
    };
    rectangle.base.x = Some(value);
}

fn peer_namespace() -> PeerNamespace {
    PeerNamespace::try_from("peer123").unwrap()
}

#[test]
fn insert_exact_preserves_authored_subtree_ids_and_namespace() {
    let original = document(r#"{"version":"1.0","children":[]}"#);
    let inserted = node(
        r#"{"type":"group","id":"c_peer123_1","children":[
            {"type":"rectangle","id":"c_peer123_2","name":"child"}
        ]}"#,
    );
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: inserted.clone(),
    }]);

    let applied = apply_txn(
        &original,
        &txn,
        &ApplyContext::for_peer_namespace(peer_namespace(), Role::Editor),
    )
    .expect("namespace-owned subtree inserts");

    assert_eq!(applied.children, vec![inserted]);
    assert!(original.children.is_empty(), "input remains immutable");
}

#[test]
fn page_ref_distinguishes_page_from_legacy_document_root() {
    let original = document(
        r#"{
            "version":"1.0",
            "pages":[{"id":"page-a","name":"A","children":[]}],
            "children":[]
        }"#,
    );
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::PageId("page-a".into()),
        parent_id: None,
        index: 0,
        node: node(r#"{"type":"rectangle","id":"page-node"}"#),
    }]);

    let applied = apply_txn(&original, &txn, &ApplyContext::standalone_trusted()).unwrap();

    assert_eq!(applied.children, original.children);
    assert_eq!(applied.pages.as_ref().unwrap()[0].children.len(), 1);
}

#[test]
fn namespace_spoof_is_rejected_recursively() {
    let original = document(r#"{"version":"1.0","children":[]}"#);
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: node(
            r#"{"type":"group","id":"c_peer123_1","children":[
                {"type":"rectangle","id":"c_other_2"}
            ]}"#,
        ),
    }]);

    let error = apply_txn(
        &original,
        &txn,
        &ApplyContext::for_peer_namespace(peer_namespace(), Role::Editor),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CollabApplyError::NamespaceViolation { id, .. } if id == "c_other_2"
    ));
}

#[test]
fn namespace_counter_must_be_a_canonical_u64() {
    let original = document(r#"{"version":"1.0","children":[]}"#);
    for invalid_id in [
        "c_peer123_01",
        "c_peer123_18446744073709551616",
        "c_peer123_not-a-number",
    ] {
        let txn = CollabTxn::new(vec![CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 0,
            node: node(&format!(r#"{{"type":"rectangle","id":"{invalid_id}"}}"#)),
        }]);
        assert!(matches!(
            apply_txn(
                &original,
                &txn,
                &ApplyContext::for_peer_namespace(peer_namespace(), Role::Editor)
            ),
            Err(CollabApplyError::NamespaceViolation { .. })
        ));
    }
}

#[test]
fn duplicate_ids_are_found_through_tabs_and_ref_subtrees() {
    let original = document(r#"{"version":"1.0","children":[]}"#);
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: node(
            r#"{"type":"tabs","id":"c_peer123_1","children":[
                {"type":"ref","id":"c_peer123_2","ref":"component","children":[
                    {"type":"rectangle","id":"c_peer123_2"}
                ]}
            ]}"#,
        ),
    }]);

    assert!(matches!(
        apply_txn(
            &original,
            &txn,
            &ApplyContext::for_peer_namespace(peer_namespace(), Role::Editor)
        ),
        Err(CollabApplyError::DuplicateId { id }) if id == "c_peer123_2"
    ));
}

#[test]
fn invalid_existing_ids_fail_before_any_operation() {
    let original = document(
        r#"{
            "version":"1.0",
            "pages":[{"id":"dup","name":"A","children":[
                {"type":"rectangle","id":"dup"}
            ]}],
            "children":[]
        }"#,
    );
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::PageId("dup".into()),
        parent_id: None,
        index: 1,
        node: node(r#"{"type":"rectangle","id":"new"}"#),
    }]);

    assert!(matches!(
        apply_txn(&original, &txn, &ApplyContext::standalone_trusted()),
        Err(CollabApplyError::DuplicateId { id }) if id == "dup"
    ));
}

#[test]
fn mixed_page_and_legacy_roots_are_rejected() {
    let original = document(
        r#"{
            "version":"1.0",
            "pages":[{"id":"page-a","name":"A","children":[]}],
            "children":[{"type":"rectangle","id":"legacy"}]
        }"#,
    );
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 1,
        node: node(r#"{"type":"rectangle","id":"new"}"#),
    }]);

    assert!(matches!(
        apply_txn(&original, &txn, &ApplyContext::standalone_trusted()),
        Err(CollabApplyError::MixedDocumentRoots)
    ));
}

#[test]
fn document_validation_is_reusable_for_snapshot_install() {
    let cases = [
        (
            r#"{"version":"1.0","pages":[{"id":"","name":"A","children":[]}],"children":[]}"#,
            "empty",
        ),
        (
            r#"{"version":"1.0","pages":[
                {"id":"same","name":"A","children":[]},
                {"id":"same","name":"B","children":[]}
            ],"children":[]}"#,
            "duplicate-page",
        ),
        (
            r#"{"version":"1.0","children":[
                {"type":"rectangle","id":"same"},
                {"type":"rectangle","id":"same"}
            ]}"#,
            "duplicate-node",
        ),
        (
            r#"{"version":"1.0","pages":[{"id":"p","name":"A","children":[]}],
                "children":[{"type":"rectangle","id":"legacy"}]}"#,
            "mixed",
        ),
    ];

    for (json, label) in cases {
        assert!(
            validate_document(&document(json)).is_err(),
            "{label} must fail"
        );
    }
}

#[test]
fn non_finite_insert_and_replace_are_rejected_atomically() {
    let original =
        document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"c_peer123_1","x":1}]}"#);

    let mut inserted = node(r#"{"type":"rectangle","id":"c_peer123_2"}"#);
    set_x(&mut inserted, f64::NAN);
    let insert = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 1,
        node: inserted,
    }]);
    assert!(matches!(
        apply_txn(
            &original,
            &insert,
            &ApplyContext::for_peer_namespace(peer_namespace(), Role::Editor)
        ),
        Err(CollabApplyError::NonFiniteNumber)
    ));

    let mut replacement = original.children[0].clone();
    set_x(&mut replacement, f64::INFINITY);
    let replace = CollabTxn::new(vec![CollabOp::ReplaceExact {
        page: PageRef::DocumentRoot,
        node_id: "c_peer123_1".into(),
        expected_hash: canonical_node_hash(&original.children[0]).unwrap(),
        node: replacement,
    }]);
    assert!(matches!(
        apply_txn(
            &original,
            &replace,
            &ApplyContext::for_peer_namespace(peer_namespace(), Role::Editor)
        ),
        Err(CollabApplyError::NonFiniteNumber)
    ));
    assert_eq!(root_ids(&original), vec!["c_peer123_1"]);
}

#[test]
fn apply_limits_bound_tree_depth_identifier_size_and_validation_work() {
    let original =
        document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"c_peer123_1"}]}"#);
    let mut context = ApplyContext::for_peer_namespace(peer_namespace(), Role::Editor);
    context.limits.max_identifier_bytes = 12;

    let long_id = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 1,
        node: node(r#"{"type":"rectangle","id":"c_peer123_123"}"#),
    }]);
    assert!(matches!(
        apply_txn(&original, &long_id, &context),
        Err(CollabApplyError::IdentifierTooLong { .. })
    ));

    context.limits.max_identifier_bytes = 1_024;
    context.limits.max_tree_depth = 1;
    let nested = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 1,
        node: node(
            r#"{"type":"group","id":"c_peer123_2","children":[
                {"type":"rectangle","id":"c_peer123_3"}
            ]}"#,
        ),
    }]);
    assert!(matches!(
        apply_txn(&original, &nested, &context),
        Err(CollabApplyError::TreeTooDeep { .. })
    ));

    context.limits.max_tree_depth = 256;
    context.limits.max_validation_node_visits_per_txn = 1;
    let replace = CollabTxn::new(vec![CollabOp::ReplaceExact {
        page: PageRef::DocumentRoot,
        node_id: "c_peer123_1".into(),
        expected_hash: canonical_node_hash(&original.children[0]).unwrap(),
        node: original.children[0].clone(),
    }]);
    assert!(matches!(
        apply_txn(&original, &replace, &context),
        Err(CollabApplyError::ValidationBudgetExceeded { .. })
    ));
}

#[test]
fn delete_and_move_bound_the_existing_target_subtree_atomically() {
    let original = document(
        r#"{"version":"1.0","children":[
            {"type":"group","id":"target","children":[
                {"type":"rectangle","id":"child-a"},
                {"type":"rectangle","id":"child-b"}
            ]},
            {"type":"group","id":"destination","children":[]}
        ]}"#,
    );
    let target_hash = canonical_node_hash(&original.children[0]).unwrap();
    let mut context = ApplyContext::standalone_trusted();
    context.limits.max_processed_subtree_nodes_per_op = 2;

    let delete = CollabTxn::new(vec![CollabOp::DeleteExact {
        page: PageRef::DocumentRoot,
        node_id: "target".into(),
        expected_hash: target_hash,
    }]);
    let move_target = CollabTxn::new(vec![CollabOp::MoveExact {
        page: PageRef::DocumentRoot,
        node_id: "target".into(),
        expected_parent: None,
        expected_index: 0,
        new_parent: Some("destination".into()),
        new_index: 0,
    }]);

    for txn in [&delete, &move_target] {
        assert!(matches!(
            apply_txn(&original, txn, &context),
            Err(CollabApplyError::SubtreeTooLarge {
                operation: 0,
                actual: 3,
                limit: 2,
            })
        ));
    }
    assert_eq!(root_ids(&original), ["target", "destination"]);
}

#[test]
fn replace_bounds_old_and_new_subtrees_as_one_operation() {
    let original = document(
        r#"{"version":"1.0","children":[
            {"type":"group","id":"target","children":[
                {"type":"rectangle","id":"old-child"}
            ]}
        ]}"#,
    );
    let replacement = node(
        r#"{"type":"group","id":"target","children":[
            {"type":"rectangle","id":"new-child"}
        ]}"#,
    );
    let txn = CollabTxn::new(vec![CollabOp::ReplaceExact {
        page: PageRef::DocumentRoot,
        node_id: "target".into(),
        expected_hash: canonical_node_hash(&original.children[0]).unwrap(),
        node: replacement,
    }]);
    let mut context = ApplyContext::standalone_trusted();
    context.limits.max_processed_subtree_nodes_per_op = 3;

    assert!(matches!(
        apply_txn(&original, &txn, &context),
        Err(CollabApplyError::SubtreeTooLarge {
            operation: 0,
            actual: 4,
            limit: 3,
        })
    ));
    assert_eq!(
        serde_json::to_value(&original).unwrap()["children"][0]["children"][0]["id"],
        "old-child"
    );
}

#[test]
fn validation_budget_counts_payload_finite_hash_and_document_traversals() {
    let empty = document(r#"{"version":"1.0","children":[]}"#);
    let payload = CollabTxn::new(vec![
        CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 0,
            node: node(
                r#"{"type":"group","id":"a","children":[
                    {"type":"rectangle","id":"a-child"}
                ]}"#,
            ),
        },
        CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 1,
            node: node(
                r#"{"type":"group","id":"b","children":[
                    {"type":"rectangle","id":"b-child"}
                ]}"#,
            ),
        },
    ]);
    let mut payload_context = ApplyContext::standalone_trusted();
    payload_context.limits.max_validation_node_visits_per_txn = 3;
    assert!(matches!(
        apply_txn(&empty, &payload, &payload_context),
        Err(CollabApplyError::ValidationBudgetExceeded {
            actual: 4,
            limit: 3,
        })
    ));

    let single = document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"target"}]}"#);
    let delete = CollabTxn::new(vec![CollabOp::DeleteExact {
        page: PageRef::DocumentRoot,
        node_id: "target".into(),
        expected_hash: canonical_node_hash(&single.children[0]).unwrap(),
    }]);
    let mut finite_context = ApplyContext::standalone_trusted();
    finite_context.limits.max_validation_node_visits_per_txn = 1;
    assert!(matches!(
        apply_txn(&single, &delete, &finite_context),
        Err(CollabApplyError::ValidationBudgetExceeded {
            actual: 2,
            limit: 1,
        })
    ));

    let mut hash_context = ApplyContext::standalone_trusted();
    hash_context.limits.max_validation_node_visits_per_txn = 4;
    assert!(matches!(
        apply_txn(&single, &delete, &hash_context),
        Err(CollabApplyError::ValidationBudgetExceeded {
            actual: 5,
            limit: 4,
        })
    ));

    let existing =
        document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"existing"}]}"#);
    let insert = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 1,
        node: node(r#"{"type":"rectangle","id":"new"}"#),
    }]);
    let mut document_context = ApplyContext::standalone_trusted();
    document_context.limits.max_validation_node_visits_per_txn = 5;
    assert!(matches!(
        apply_txn(&existing, &insert, &document_context),
        Err(CollabApplyError::ValidationBudgetExceeded {
            actual: 6,
            limit: 5,
        })
    ));
}

#[test]
fn validation_budget_failure_after_working_mutation_is_atomic() {
    let mut original = document(r#"{"version":"1.0","children":[]}"#);
    let before = original.clone();
    let txn = CollabTxn::new(vec![
        CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 0,
            node: node(r#"{"type":"rectangle","id":"first"}"#),
        },
        CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 1,
            node: node(r#"{"type":"rectangle","id":"second"}"#),
        },
    ]);
    let mut context = ApplyContext::standalone_trusted();
    context.limits.max_validation_node_visits_per_txn = 9;

    assert!(matches!(
        apply_txn_in_place(&mut original, &txn, &context),
        Err(CollabApplyError::ValidationBudgetExceeded {
            actual: 10,
            limit: 9,
        })
    ));
    assert_eq!(original, before);
}

#[test]
fn viewer_role_cannot_apply_even_a_well_formed_transaction() {
    let original = document(r#"{"version":"1.0","children":[]}"#);
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: node(r#"{"type":"rectangle","id":"new"}"#),
    }]);
    let mut context = ApplyContext::standalone_trusted();
    context.role = Role::Viewer;

    assert!(matches!(
        apply_txn(&original, &txn, &context),
        Err(CollabApplyError::PermissionDenied { role: Role::Viewer })
    ));
}

#[test]
fn transaction_failure_leaves_no_prefix_mutation() {
    let mut original = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"a","name":"A"},
            {"type":"rectangle","id":"b","name":"B"}
        ]}"#,
    );
    let before = original.clone();
    let hash_a = canonical_node_hash(&original.children[0]).unwrap();
    let wrong_hash = canonical_node_hash(&node(r#"{"type":"rectangle","id":"other"}"#)).unwrap();
    let txn = CollabTxn::new(vec![
        CollabOp::DeleteExact {
            page: PageRef::DocumentRoot,
            node_id: "a".into(),
            expected_hash: hash_a,
        },
        CollabOp::DeleteExact {
            page: PageRef::DocumentRoot,
            node_id: "b".into(),
            expected_hash: wrong_hash,
        },
    ]);

    let error =
        apply_txn_in_place(&mut original, &txn, &ApplyContext::standalone_trusted()).unwrap_err();

    assert!(matches!(error, CollabApplyError::HashMismatch { .. }));
    assert_eq!(original, before);
}

#[test]
fn replace_requires_hash_and_stable_root_id() {
    let original =
        document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"a","name":"A"}]}"#);
    let hash = canonical_node_hash(&original.children[0]).unwrap();
    let txn = CollabTxn::new(vec![CollabOp::ReplaceExact {
        page: PageRef::DocumentRoot,
        node_id: "a".into(),
        expected_hash: hash,
        node: node(r#"{"type":"rectangle","id":"renamed","name":"B"}"#),
    }]);

    assert!(matches!(
        apply_txn(&original, &txn, &ApplyContext::standalone_trusted()),
        Err(CollabApplyError::ReplacementIdChanged { .. })
    ));
}

#[test]
fn move_reparents_exactly_and_rejects_cycles() {
    let original = document(
        r#"{"version":"1.0","children":[
            {"type":"group","id":"g","children":[
                {"type":"group","id":"nested","children":[]}
            ]},
            {"type":"rectangle","id":"moving"}
        ]}"#,
    );
    let reparent = CollabTxn::new(vec![CollabOp::MoveExact {
        page: PageRef::DocumentRoot,
        node_id: "moving".into(),
        expected_parent: None,
        expected_index: 1,
        new_parent: Some("g".into()),
        new_index: 1,
    }]);
    let applied = apply_txn(&original, &reparent, &ApplyContext::standalone_trusted()).unwrap();
    let encoded = serde_json::to_value(&applied).unwrap();
    assert_eq!(
        encoded.pointer("/children/0/children/1/id"),
        Some(&serde_json::Value::String("moving".into()))
    );

    let cycle = CollabTxn::new(vec![CollabOp::MoveExact {
        page: PageRef::DocumentRoot,
        node_id: "g".into(),
        expected_parent: None,
        expected_index: 0,
        new_parent: Some("nested".into()),
        new_index: 0,
    }]);
    assert!(matches!(
        apply_txn(&original, &cycle, &ApplyContext::standalone_trusted()),
        Err(CollabApplyError::MoveWouldCycle { .. })
    ));
}

#[test]
fn same_parent_move_index_is_interpreted_after_removal() {
    let original = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"a"},
            {"type":"rectangle","id":"b"},
            {"type":"rectangle","id":"c"},
            {"type":"rectangle","id":"d"}
        ]}"#,
    );
    let forward = CollabTxn::new(vec![CollabOp::MoveExact {
        page: PageRef::DocumentRoot,
        node_id: "a".into(),
        expected_parent: None,
        expected_index: 0,
        new_parent: None,
        new_index: 2,
    }]);
    let backward = CollabTxn::new(vec![CollabOp::MoveExact {
        page: PageRef::DocumentRoot,
        node_id: "d".into(),
        expected_parent: None,
        expected_index: 3,
        new_parent: None,
        new_index: 1,
    }]);

    assert_eq!(
        root_ids(&apply_txn(&original, &forward, &ApplyContext::standalone_trusted()).unwrap()),
        ["b", "c", "a", "d"]
    );
    assert_eq!(
        root_ids(&apply_txn(&original, &backward, &ApplyContext::standalone_trusted()).unwrap()),
        ["a", "d", "b", "c"]
    );
}

#[test]
fn bad_move_index_is_atomic() {
    let mut original = document(
        r#"{"version":"1.0","children":[
            {"type":"rectangle","id":"a"},
            {"type":"rectangle","id":"b"}
        ]}"#,
    );
    let before = original.clone();
    let txn = CollabTxn::new(vec![CollabOp::MoveExact {
        page: PageRef::DocumentRoot,
        node_id: "a".into(),
        expected_parent: None,
        expected_index: 0,
        new_parent: None,
        new_index: 99,
    }]);

    assert!(matches!(
        apply_txn_in_place(&mut original, &txn, &ApplyContext::standalone_trusted()),
        Err(CollabApplyError::IndexOutOfBounds { .. })
    ));
    assert_eq!(original, before);
}

#[test]
fn replay_produces_the_same_document_hash() {
    let original =
        document(r#"{"version":"1.0","children":[{"type":"rectangle","id":"a","name":"A"}]}"#);
    let expected = node(r#"{"type":"rectangle","id":"a","name":"B","x":12.0}"#);
    let txn = CollabTxn::new(vec![CollabOp::ReplaceExact {
        page: PageRef::DocumentRoot,
        node_id: "a".into(),
        expected_hash: canonical_node_hash(&original.children[0]).unwrap(),
        node: expected.clone(),
    }]);

    let replayed = apply_txn(&original, &txn, &ApplyContext::standalone_trusted()).unwrap();
    let authored = document(
        r#"{"version":"1.0","children":[{"type":"rectangle","id":"a","name":"B","x":12.0}]}"#,
    );

    assert_eq!(replayed.children, vec![expected]);
    assert_eq!(
        canonical_document_hash(&replayed).unwrap(),
        canonical_document_hash(&authored).unwrap()
    );
}
