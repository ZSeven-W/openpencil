//! Current native saves must not be mistaken for legacy documents.
use super::{
    super::load_canonical_with_compatibility, write_normalized_source_with_current_schema,
};
use serde_json::{json, Value};

fn changed_paths(before: &Value, after: &Value, path: &str, changes: &mut Vec<String>) {
    if before == after {
        return;
    }
    match (before, after) {
        (Value::Object(a), Value::Object(b)) => {
            for key in a
                .keys()
                .chain(b.keys())
                .collect::<std::collections::BTreeSet<_>>()
            {
                changed_paths(
                    a.get(key).unwrap_or(&Value::Null),
                    b.get(key).unwrap_or(&Value::Null),
                    &format!("{path}/{key}"),
                    changes,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                changed_paths(a, b, &format!("{path}/{index}"), changes);
            }
        }
        _ => changes.push(format!(
            "{path}: {:?} -> {:?}",
            before.to_string().chars().take(80).collect::<String>(),
            after.to_string().chars().take(80).collect::<String>()
        )),
    }
}

#[test]
#[ignore = "set OPENPENCIL_COMPAT_REPLAY_SOURCE to a retained native save; read-only"]
fn retained_native_save_does_not_request_legacy_upgrade() {
    let path = std::env::var("OPENPENCIL_COMPAT_REPLAY_SOURCE").expect("retained source path");
    let source = std::fs::read_to_string(&path).unwrap();
    let loaded = load_canonical_with_compatibility(&source).unwrap();
    let mut normalized = Vec::new();
    write_normalized_source_with_current_schema(
        &mut normalized,
        &source,
        crate::extract_editor_meta(&source).unwrap_or_default(),
    )
    .unwrap();
    let before: Value = serde_json::from_str(&source).unwrap();
    let after: Value = serde_json::from_slice(&normalized).unwrap();
    let mut paths = Vec::new();
    changed_paths(&before, &after, "", &mut paths);
    println!(
        "compatibility={:?}\nnormalization_diff={}",
        loaded.compatibility,
        serde_json::to_string_pretty(&paths).unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        source,
        "read-only replay"
    );
    assert!(!loaded.compatibility.needs_schema_upgrade());
}

#[test]
fn native_writer_photo_fills_and_search_fallback_roundtrip_without_upgrade() {
    let source = json!({
        "version":"1.0.0","children":[
            {"id":"hero","type":"rectangle","width":375,"height":240,
             "fill":[{"type":"image","url":"https://example.com/workout.jpg","mode":"crop"}]},
            {"id":"fallback","type":"rectangle","width":64,"height":64,
             "fill":[{"type":"image","url":"placeholder://image-search-failed","mode":"crop"}]},
            {"id":"image-node","type":"image","src":"https://example.com/exercise.jpg","width":64,"height":64}
        ]
    });
    let document = jian_ops_schema::load_str(&source.to_string())
        .unwrap()
        .value;
    let state = op_editor_core::EditorState::from_document(document);
    let mut saved = Vec::new();
    jian_ops_schema::image_table::write_document_with_extension(
        &mut saved,
        &state.doc,
        &jian_ops_schema::image_thumbs::capture_snapshot(),
        "editorMeta",
        &crate::EditorMeta::default(),
    )
    .unwrap();
    let saved = String::from_utf8(saved).unwrap();
    let reopened = load_canonical_with_compatibility(&saved).unwrap();
    assert_eq!(reopened.loaded.value, state.doc);
    assert_eq!(reopened.compatibility, Default::default());
}

#[test]
fn missing_src_on_a_real_legacy_image_node_still_requests_upgrade() {
    let source = json!({"version":"0.7.5","children":[
        {"id":"legacy-photo","type":"image","width":64,"height":64},
        {"id":"canonical-fill","type":"rectangle","width":64,"height":64,
         "fill":[{"type":"image","url":"placeholder://image-search-failed","mode":"crop"}]}
    ]})
    .to_string();
    let loaded = load_canonical_with_compatibility(&source).unwrap();
    assert!(loaded.compatibility.normalized_legacy);
    assert!(loaded.compatibility.needs_schema_upgrade());
    let mut normalized = Vec::new();
    write_normalized_source_with_current_schema(
        &mut normalized,
        &source,
        crate::EditorMeta::default(),
    )
    .unwrap();
    let value: Value = serde_json::from_slice(&normalized).unwrap();
    assert_eq!(value["children"][0]["src"], "");
    assert!(value["children"][1]["fill"][0].get("src").is_none());
    assert_eq!(
        value["children"][1]["fill"][0]["url"],
        "placeholder://image-search-failed"
    );
    assert!(
        !load_canonical_with_compatibility(&String::from_utf8(normalized).unwrap())
            .unwrap()
            .compatibility
            .needs_schema_upgrade()
    );
}
