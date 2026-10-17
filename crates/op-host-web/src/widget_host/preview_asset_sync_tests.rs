//! Web cache rebuild forwarding shares the native asset synchronization seam.

#![cfg(all(test, feature = "canvaskit"))]

use super::WidgetHost;
use op_editor_core::EditorState;

#[test]
fn web_scene_rebuild_forwards_accepted_asset_once_to_retained_session() {
    let document = |src: &str| {
        serde_json::from_value(serde_json::json!({
        "version":"1.1","children":[{"type":"image","id":"photo","src":src,"width":100,"height":100}]
    })).unwrap()
    };
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::from_document(document("placeholder://image-search-failed"));
    let mut session = op_preview_core::PreviewSession::enter(
        &host.editor_state.doc,
        (800.0, 600.0),
        &Default::default(),
        0,
        false,
        false,
        jian_core::layout::measure::default_backend(),
        0,
    )
    .unwrap();
    session.bind_asset_source(host.document_epoch, host.editor_state.document_generation());
    host.preview = Some(session);
    host.editor_state.doc = document("op-image:web-complete");
    host.editor_state.mark_document_changed();
    host.mark_dirty();
    host.refresh_layout_scene();
    let session = host.preview.as_mut().unwrap();
    assert!(
        !session.sync_image_assets(
            &host.editor_state.doc,
            host.document_epoch,
            host.editor_state.document_generation()
        ),
        "refresh already synchronized the source; another pass must be a no-op"
    );
    session.reset().unwrap();
    assert!(
        !session.sync_image_assets(
            &host.editor_state.doc,
            host.document_epoch,
            host.editor_state.document_generation()
        ),
        "web completion is retained in reset source as well"
    );
}
