//! Serde helpers that isolate collaboration data from Jian's file-save scope.
//!
//! `ImageSrc` intentionally serializes differently while a `.op` file save is
//! collecting its top-level image table. Collaboration hashes and frames must
//! stay self-contained and context-independent, so they serialize inside a
//! nested collector and immediately restore every collected reference inline.

use jian_ops_schema::{
    image_table::{inline_images, visit_image_src_strings_mut, with_save_scope},
    image_thumbs::ImageThumbSnapshot,
};
use serde::Serialize;
use serde_json::{Map, Value};

pub(crate) struct IsolatedValue {
    pub value: Value,
    pub images: Map<String, Value>,
}

pub(crate) fn to_isolated_value<T: Serialize>(
    value: &T,
) -> Result<IsolatedValue, serde_json::Error> {
    with_save_scope(&ImageThumbSnapshot::default(), |tables| {
        let value = serde_json::to_value(value)?;
        let images = match serde_json::to_value(tables.images())? {
            Value::Object(images) => images,
            _ => unreachable!("save image table always serializes as an object"),
        };
        Ok(IsolatedValue { value, images })
    })
}

pub(crate) fn inline_document(value: &mut Value, images: &Map<String, Value>) {
    if images.is_empty() {
        return;
    }
    if let Value::Object(document) = value {
        document.insert("images".to_owned(), Value::Object(images.clone()));
        inline_images(value);
    }
}

pub(crate) fn inline_node(value: &mut Value, images: &Map<String, Value>) {
    if images.is_empty() {
        return;
    }
    let node = std::mem::take(value);
    let mut wrapper = serde_json::json!({
        "children": [node],
        "images": Value::Object(images.clone()),
    });
    inline_images(&mut wrapper);
    *value = wrapper
        .get_mut("children")
        .and_then(Value::as_array_mut)
        .and_then(|children| (!children.is_empty()).then(|| children.remove(0)))
        .expect("node wrapper shape is fixed");
}

pub(crate) fn inline_txn(value: &mut Value, images: &Map<String, Value>) {
    if images.is_empty() {
        return;
    }
    let Some(ops) = value.get_mut("ops").and_then(Value::as_array_mut) else {
        return;
    };
    let mut operation_indexes = Vec::new();
    let mut nodes = Vec::new();
    for (index, operation) in ops.iter_mut().enumerate() {
        if let Some(node) = operation.get_mut("node") {
            operation_indexes.push(index);
            nodes.push(std::mem::take(node));
        }
    }
    if nodes.is_empty() {
        return;
    }

    let mut wrapper = serde_json::json!({
        "children": Value::Array(nodes),
        "images": Value::Object(images.clone()),
    });
    inline_images(&mut wrapper);
    let restored = wrapper
        .get_mut("children")
        .and_then(Value::as_array_mut)
        .map(std::mem::take)
        .expect("transaction image wrapper shape is fixed");
    debug_assert_eq!(operation_indexes.len(), restored.len());
    for (index, node) in operation_indexes.into_iter().zip(restored) {
        *ops[index]
            .get_mut("node")
            .expect("operation node slot was recorded") = node;
    }
}

pub(crate) fn inline_frame(value: &mut Value, images: &Map<String, Value>) {
    let Some(body) = value.get_mut("body") else {
        return;
    };
    let Some(kind) = body.get("type").and_then(Value::as_str).map(str::to_owned) else {
        return;
    };
    let Some(payload) = body.get_mut("payload") else {
        return;
    };
    match kind.as_str() {
        "submit" | "commit" => {
            if let Some(txn) = payload.get_mut("txn") {
                inline_txn(txn, images);
            }
        }
        "snapshot" => {
            if let Some(document) = payload.get_mut("document") {
                inline_document(document, images);
            }
        }
        _ => {}
    }
}

pub(crate) fn frame_has_external_image_ref(value: &mut Value) -> bool {
    let Some(body) = value.get_mut("body") else {
        return false;
    };
    let Some(kind) = body.get("type").and_then(Value::as_str).map(str::to_owned) else {
        return false;
    };
    let Some(payload) = body.get_mut("payload") else {
        return false;
    };
    match kind.as_str() {
        "submit" | "commit" => payload
            .get_mut("txn")
            .is_some_and(txn_has_external_image_ref),
        "snapshot" => payload
            .get_mut("document")
            .is_some_and(document_has_external_image_ref),
        _ => false,
    }
}

pub(crate) fn txn_has_external_image_ref(value: &mut Value) -> bool {
    let Some(ops) = value.get_mut("ops").and_then(Value::as_array_mut) else {
        return false;
    };
    ops.iter_mut()
        .filter_map(|operation| operation.get_mut("node"))
        .any(node_has_external_image_ref)
}

pub(crate) fn document_has_external_image_ref(value: &mut Value) -> bool {
    let mut found = false;
    visit_image_src_strings_mut(value, &mut |source| {
        found |= source.starts_with("op-image:");
    });
    found
}

pub(crate) fn node_has_external_image_ref(value: &mut Value) -> bool {
    let node = std::mem::take(value);
    let mut wrapper = serde_json::json!({ "children": [node] });
    let found = document_has_external_image_ref(&mut wrapper);
    *value = wrapper
        .get_mut("children")
        .and_then(Value::as_array_mut)
        .and_then(|children| (!children.is_empty()).then(|| children.remove(0)))
        .expect("node wrapper shape is fixed");
    found
}
