use super::WidgetHostNative;
use jian_ops_schema::PenDocument;
use op_editor_core::EditOrigin;

fn document_with_rect(id: &str) -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "rectangle",
            "id": id,
            "name": "Rect",
            "x": 0,
            "y": 0,
            "width": 10,
            "height": 10
        }]
    }))
    .expect("valid test document")
}

#[test]
fn remote_commit_invalidates_host_caches_without_rotating_document_epoch() {
    let mut host = WidgetHostNative::new();
    let epoch = host.document_epoch();

    host.install_collaboration_document(document_with_rect("peer-1:1"), EditOrigin::RemoteCommit)
        .expect("remote commit installs");

    assert_eq!(host.document_epoch(), epoch);
    assert!(host.editor_state_dirty);
    assert!(host.pan_cache.is_none());
    assert_eq!(host.editor_state().doc.children.len(), 1);
}

#[test]
fn snapshot_rotates_epoch_and_failed_install_is_atomic() {
    let mut host = WidgetHostNative::new();
    let epoch = host.document_epoch();
    host.install_collaboration_document(document_with_rect("peer-1:1"), EditOrigin::Snapshot)
        .expect("snapshot installs");
    assert_eq!(host.document_epoch(), epoch.wrapping_add(1));

    let before = host.editor_state().doc.clone();
    let epoch = host.document_epoch();
    let mut invalid = document_with_rect("duplicate");
    invalid.children.push(invalid.children[0].clone());
    assert!(host
        .install_collaboration_document(invalid, EditOrigin::RemoteCommit)
        .is_err());
    assert_eq!(&host.editor_state().doc, &before);
    assert_eq!(host.document_epoch(), epoch);
}
