use super::WebSyncClient;

const V3: &str = r#"{"document":{"version":"1.0","children":[]},"version":3}"#;

#[test]
fn pull_merges_each_top_level_field_over_nested_metadata() {
    let client = WebSyncClient::new();
    let nested = client
        .next_document_with_metadata(
            r#"{"document":{"version":"1.0","children":[],"editorMeta":{"activePageIndex":4,"preserveAuthoredGeometry":true}},"version":3}"#,
        )
        .expect("valid response")
        .expect("first version");
    assert_eq!(nested.version, 3);
    assert_eq!(nested.active_page_index, 4);
    assert!(nested.preserve_authored_geometry);

    let active_override = client
        .next_document_with_metadata(
            r#"{"document":{"version":"1.0","children":[],"editorMeta":{"activePageIndex":4,"preserveAuthoredGeometry":true}},"version":3,"activePageIndex":2}"#,
        )
        .expect("valid response")
        .expect("first version");
    assert_eq!(active_override.active_page_index, 2);
    assert!(
        active_override.preserve_authored_geometry,
        "an active-page override must not discard nested geometry mode"
    );

    let preserve_override = client
        .next_document_with_metadata(
            r#"{"document":{"version":"1.0","children":[],"editorMeta":{"activePageIndex":4,"preserveAuthoredGeometry":true}},"version":3,"preserveAuthoredGeometry":false}"#,
        )
        .expect("valid response")
        .expect("first version");
    assert_eq!(preserve_override.active_page_index, 4);
    assert!(!preserve_override.preserve_authored_geometry);

    let legacy = client
        .next_document_with_metadata(V3)
        .expect("legacy response stays valid")
        .expect("first version");
    assert_eq!(legacy.active_page_index, 0);
    assert!(!legacy.preserve_authored_geometry);

    let malformed_optional = client
        .next_document_with_metadata(
            r#"{"document":{"version":"1.0","children":[],"editorMeta":{"activePageIndex":5,"preserveAuthoredGeometry":true}},"version":3,"activePageIndex":"bad","preserveAuthoredGeometry":"yes"}"#,
        )
        .expect("invalid optional metadata does not reject the document")
        .expect("first version");
    assert_eq!(malformed_optional.active_page_index, 5);
    assert!(malformed_optional.preserve_authored_geometry);
}

#[test]
fn sync_applies_active_page_and_geometry_as_one_version() {
    let mut client = WebSyncClient::new();
    let body = r#"{"document":{"version":"1.0","children":[],"editorMeta":{"activePageIndex":6,"preserveAuthoredGeometry":true}},"version":8,"activePageIndex":3}"#;
    let mut applied = None;
    assert!(client
        .sync_with_editor_meta(body, |_doc, version, page, preserve, _scenario| {
            applied = Some((version, page, preserve));
            true
        })
        .expect("valid response"));
    assert_eq!(applied, Some((8, 3, true)));
    assert_eq!(client.applied_version(), 8);
}

#[test]
fn baseline_detects_each_editor_meta_change_and_suppresses_own_echo() {
    let mut client = WebSyncClient::new();
    assert!(client.sync(V3, |_d, _v| true).expect("initial sync"));
    let doc = r#"{"version":"1.0","children":[]}"#;
    client.note_applied_snapshot_with_editor_meta(doc, 2, true);
    assert!(!client.editor_meta_needs_push(2, true));
    assert!(client.editor_meta_needs_push(3, true));
    assert!(!client.should_push_with_editor_meta(doc, 2, true));
    assert!(client.should_push_with_editor_meta(doc, 3, true));
    assert!(client.should_push_with_editor_meta(doc, 2, false));

    client.mark_pushed_with_editor_meta(doc, 3, false, 4);
    assert_eq!(client.applied_version(), 4);
    assert!(!client.should_push_with_editor_meta(doc, 3, false));
    let own_echo = r#"{"document":{"version":"1.0","children":[]},"version":4,"activePageIndex":3,"preserveAuthoredGeometry":false}"#;
    assert!(
        client
            .next_document_with_metadata(own_echo)
            .expect("valid echo")
            .is_none(),
        "a metadata-only push acknowledgement must suppress a full-document echo apply"
    );
}

#[test]
fn push_adds_active_page_and_preserve_mode() {
    let doc = r#"{"version":"1.0","children":[]}"#;
    let body = WebSyncClient::wrap_push_body_with_base_and_editor_meta(doc, 7, 3, true);
    let value: serde_json::Value = serde_json::from_str(&body).expect("push json");
    assert_eq!(value["baseVersion"], 7);
    assert_eq!(value["activePageIndex"], 3);
    assert_eq!(value["preserveAuthoredGeometry"], true);

    let metadata_only =
        WebSyncClient::wrap_push_body_with_base_editor_meta_and_mode(doc, 7, 4, false, true);
    let value: serde_json::Value = serde_json::from_str(&metadata_only).expect("push json");
    assert_eq!(value["metadataOnly"], true);
    assert_eq!(value["activePageIndex"], 4);
}
