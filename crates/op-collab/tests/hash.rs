use std::str::FromStr;

use jian_ops_schema::{
    image_table::with_save_scope, image_thumbs::ImageThumbSnapshot, node::PenNode, PenDocument,
};
use op_collab::{
    canonical_document_hash, canonical_json, canonical_node_hash, canonical_txn_hash,
    CanonicalHash, CollabOp, CollabTxn, PageRef, CANONICAL_HASH_ALGORITHM, CANONICAL_HASH_VERSION,
};
use serde_json::json;

fn golden_node() -> PenNode {
    serde_json::from_str(
        r#"{"type":"rectangle","id":"n1","name":"Box","x":1.0,"y":2.0,"width":10.0,"height":20.0}"#,
    )
    .unwrap()
}

#[test]
fn canonical_json_sorts_object_keys_but_preserves_array_order() {
    let left = json!({"z": 1, "a": {"b": 2, "a": 1}, "items": [3, 2, 1]});
    let right: serde_json::Value =
        serde_json::from_str(r#"{"items":[3,2,1],"a":{"a":1,"b":2},"z":1}"#).unwrap();

    assert_eq!(
        canonical_json(&left).unwrap(),
        canonical_json(&right).unwrap()
    );
    assert_ne!(
        canonical_json(&left).unwrap(),
        canonical_json(&json!({"z": 1, "a": {"b": 2, "a": 1}, "items": [1, 2, 3]})).unwrap()
    );
}

#[test]
fn domains_are_distinct_and_contract_is_frozen() {
    let node = golden_node();
    let document: PenDocument = serde_json::from_str(
        r#"{"version":"1.0","name":"Golden","children":[
            {"type":"rectangle","id":"n1","name":"Box","x":1.0,"y":2.0,"width":10.0,"height":20.0}
        ]}"#,
    )
    .unwrap();
    let txn = CollabTxn::new(vec![CollabOp::InsertExact {
        page: PageRef::DocumentRoot,
        parent_id: None,
        index: 0,
        node: node.clone(),
    }]);

    assert_eq!(CANONICAL_HASH_VERSION, 1);
    assert_eq!(CANONICAL_HASH_ALGORITHM, "blake3");
    assert_ne!(
        canonical_document_hash(&document).unwrap(),
        canonical_node_hash(&node).unwrap()
    );
    assert_ne!(
        canonical_node_hash(&node).unwrap(),
        canonical_txn_hash(&txn).unwrap()
    );

    assert_eq!(
        canonical_document_hash(&document).unwrap().to_string(),
        "dfeef86a8306c18da6163c91e0438583e768af2966122e67d96e1cb2149c9346"
    );
    assert_eq!(
        canonical_node_hash(&node).unwrap().to_string(),
        "317f589a59ab5c9ada648b3850f58d8af8de08759850be8fac3e22fedd479a9b"
    );
    assert_eq!(
        canonical_txn_hash(&txn).unwrap().to_string(),
        "c0261b5bbefb751d94b87d546a638280f928110bb1e17764f7302652b16da85b"
    );
}

#[test]
fn canonical_hash_wire_value_is_strict_lowercase_hex() {
    let encoded = canonical_node_hash(&golden_node()).unwrap().to_string();
    assert_eq!(
        CanonicalHash::from_str(&encoded).unwrap().to_string(),
        encoded
    );
    assert!(CanonicalHash::from_str(&encoded.to_uppercase()).is_err());
    assert!(CanonicalHash::from_str("abcd").is_err());
}

#[test]
fn non_finite_numbers_are_rejected_instead_of_becoming_null() {
    for non_finite in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut node = golden_node();
        let PenNode::Rectangle(rectangle) = &mut node else {
            unreachable!()
        };
        rectangle.base.x = Some(non_finite);
        assert!(matches!(
            canonical_node_hash(&node),
            Err(op_collab::CanonicalHashError::NonFiniteNumber)
        ));
    }
}

#[test]
fn canonical_json_freezes_utf8_key_order_escaping_and_float_edges() {
    let value = json!({
        "é": "line\n\"quote\"",
        "😀": 1.0e20,
        "z": -0.0,
    });
    assert_eq!(
        String::from_utf8(canonical_json(&value).unwrap()).unwrap(),
        "{\"z\":-0.0,\"é\":\"line\\n\\\"quote\\\"\",\"😀\":1e+20}"
    );
}

#[test]
fn image_hash_is_independent_of_ambient_file_save_scope() {
    let source = format!("data:image/png;base64,{}", "A".repeat(5_000));
    let document: PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [{"type": "image", "id": "image-1", "src": source}]
    }))
    .unwrap();
    let expected = canonical_document_hash(&document).unwrap();
    let mut second_node = document.children[0].clone();
    let PenNode::Image(image) = &mut second_node else {
        unreachable!()
    };
    image.base.id = "image-2".into();
    let txn = CollabTxn::new(vec![
        CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 0,
            node: document.children[0].clone(),
        },
        CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 1,
            node: second_node,
        },
    ]);
    let expected_txn = canonical_txn_hash(&txn).unwrap();

    with_save_scope(&ImageThumbSnapshot::default(), |tables| {
        assert_eq!(canonical_document_hash(&document).unwrap(), expected);
        assert_eq!(canonical_txn_hash(&txn).unwrap(), expected_txn);
        assert!(
            tables.images_is_empty(),
            "collaboration serialization must not pollute an outer file-save table"
        );
    });

    assert!(matches!(
        canonical_json(&document),
        Err(op_collab::CanonicalHashError::ImageBearingGenericValue)
    ));
}
