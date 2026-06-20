use super::WidgetHostNative;
use op_editor_core::{EditorState, Locale};

#[test]
fn install_imported_state_preserves_live_ui_and_defers_layout() {
    let mut host = WidgetHostNative::new();
    host.editor_state.editor_ui.locale = Locale::ZhCn;
    host.editor_state.editor_ui.sidebar_open = false;
    host.editor_state.editor_ui.figma_import_in_progress = true;
    host.editor_state_dirty = false;

    let mut imported = EditorState::new();
    imported.editor_ui.file_name_display = Some("Dashboard.fig".into());
    imported.editor_ui.locale = Locale::EnUs;
    imported.editor_ui.sidebar_open = true;

    host.install_imported_state(imported);

    assert_eq!(host.editor_state.editor_ui.locale, Locale::ZhCn);
    assert!(!host.editor_state.editor_ui.sidebar_open);
    assert!(!host.editor_state.editor_ui.figma_import_in_progress);
    assert_eq!(
        host.editor_state.editor_ui.file_name_display.as_deref(),
        Some("Dashboard.fig")
    );
    assert!(host.editor_state_dirty);
    assert!(host.layout_scene.pages.is_empty());
}

#[test]
fn install_imported_state_rebuilds_even_when_the_import_matches_the_cache() {
    let mut host = WidgetHostNative::new();
    // Prime the scene-build cache from the current document.
    host.editor_state_dirty = true;
    let _ = host.layout_scene();
    assert!(
        !host.layout_scene.pages.is_empty(),
        "precondition: a scene is built"
    );

    // Import a state whose scene inputs are identical to the cached build (same
    // document + same authored-geometry latch). `install_imported_state` takes
    // (empties) the scene and defers the rebuild; without invalidating the cache
    // the matching-input refresh would skip and leave the canvas blank.
    let mut imported = EditorState::from_document(host.editor_state.doc.clone());
    imported.editor_ui.preserve_authored_geometry =
        host.editor_state.editor_ui.preserve_authored_geometry;
    host.install_imported_state(imported);
    assert!(
        host.layout_scene.pages.is_empty(),
        "import defers (empties) the scene"
    );

    let _ = host.layout_scene();
    assert!(
        !host.layout_scene.pages.is_empty(),
        "import must rebuild the scene even when its inputs match the cache"
    );
}
