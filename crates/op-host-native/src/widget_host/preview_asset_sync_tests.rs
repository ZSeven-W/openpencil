//! Native scene rebuilds forward accepted assets into the active preview.

use super::WidgetHostNative;
use op_editor_core::EditorState;

fn document(src: &str) -> jian_ops_schema::PenDocument {
    serde_json::from_value(serde_json::json!({"version":"1.1","children":[{
        "type":"frame","id":"screen","width":375,"height":812,"children":[{
            "type":"image","id":"photo","x":20,"y":20,"width":120,"height":120,"src":src
        }]
    }]}))
    .unwrap()
}

#[test]
fn scene_rebuild_updates_retained_preview_asset_and_save_source() {
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(document(
        "placeholder://image-search-failed",
    )));
    assert!(host.enter_preview((1000.0, 900.0)));
    host.refresh_layout_scene();
    let source = "op-image:accepted-native-photo";
    // Match the background enrich completion: in-place canonical mutation,
    // revision bump, host dirty mark; do not reopen the preview.
    host.editor_state.doc = document(source);
    host.editor_state.mark_document_changed();
    host.mark_editor_state_dirty();
    host.refresh_layout_scene();
    let scene = host.preview_scene_for_test();
    assert_eq!(
        scene
            .active_page()
            .unwrap()
            .find("photo")
            .unwrap()
            .image_src
            .as_deref(),
        Some(source)
    );
    assert!(host.editor_state.editor_ui.preview.mode);
    host.preview.as_mut().unwrap().reset().unwrap();
    assert_eq!(
        host.preview_scene_for_test()
            .active_page()
            .unwrap()
            .find("photo")
            .unwrap()
            .image_src
            .as_deref(),
        Some(source)
    );
    assert!(host
        .install_open_document(document("op-image:different-document"), None, None)
        .is_ok());
    host.refresh_layout_scene();
    assert!(
        host.preview.is_none(),
        "Open ends the old document preview lifetime"
    );
}

#[test]
fn direct_document_generation_replacement_cannot_cross_preview_asset_guard() {
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(document("op-image:original")));
    assert!(host.enter_preview((1000.0, 900.0)));
    host.editor_state
        .replace_document(document("op-image:replacement"));
    host.mark_editor_state_dirty();
    host.refresh_layout_scene();
    assert_eq!(
        host.preview_scene_for_test()
            .active_page()
            .unwrap()
            .find("photo")
            .unwrap()
            .image_src
            .as_deref(),
        Some("op-image:original")
    );
}
