//! Preview teardown coverage for whole-document replacement.

#![cfg(all(test, feature = "gl-host", not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::EditorState;

const VIEWPORT_W: f32 = 1000.0;
const VIEWPORT_H: f32 = 700.0;

fn document() -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(
        r##"{
            "version":"1.0.0",
            "children":[{
                "type":"frame","id":"screen","x":0,"y":0,
                "width":390,"height":844,
                "fill":[{"type":"solid","color":"#ffffff"}],
                "children":[]
            }]
        }"##,
    )
    .expect("preview replacement fixture parses")
    .value
}

fn preview_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(document()));
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    assert!(host.enter_preview((VIEWPORT_W, VIEWPORT_H)));
    assert!(host.preview_active());
    assert!(host.preview_device_frame.is_some());
    host.preview_mode_transition = None;
    host
}

fn assert_preview_runtime_gone(host: &WidgetHostNative) {
    assert!(!host.preview_active());
    assert!(host.preview.is_none());
    assert!(host.preview_device_frame.is_none());
    assert!(host.preview_mode_transition.is_none());
    assert!(!host.editor_state.editor_ui.preview.mode);
    assert!(host.editor_state.editor_ui.preview.device.is_none());
}

#[test]
fn replace_editor_state_tears_down_preview() {
    let mut host = preview_host();

    assert!(host.replace_editor_state(EditorState::starter()));

    assert_preview_runtime_gone(&host);
}

#[test]
fn replace_editor_state_during_exit_animation_tears_down_immediately() {
    let mut host = preview_host();
    host.exit_preview();
    assert!(host.preview.is_some());
    assert!(host.preview_mode_transition.is_some());

    assert!(host.replace_editor_state(EditorState::starter()));
    assert_preview_runtime_gone(&host);

    host.settle_mode_transition();
    assert_preview_runtime_gone(&host);
}

#[test]
fn install_open_document_tears_down_preview() {
    let mut host = preview_host();

    host.install_open_document(document(), None, Some("opened.op".to_string()))
        .expect("open document succeeds");

    assert_preview_runtime_gone(&host);
}

#[test]
fn replace_editor_state_strips_preview_mode_from_incoming_state() {
    let mut host = preview_host();
    let mut incoming = EditorState::starter();
    incoming.editor_ui.preview.mode = true;

    assert!(host.replace_editor_state(incoming));

    assert_preview_runtime_gone(&host);
}

#[test]
fn install_imported_state_tears_down_preview() {
    let mut host = preview_host();

    assert!(host.install_imported_state(EditorState::from_document(document())));

    assert_preview_runtime_gone(&host);
}
