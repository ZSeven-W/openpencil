//! Native preview hover forwarding coverage.

#![cfg(all(test, feature = "gl-host", not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::EditorState;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1000.0;
const VIEWPORT_H: f32 = 700.0;

fn document() -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(
        r##"{
            "version":"1.1","formatVersion":"1.1","id":"hover",
            "app":{"name":"hover","version":"1","id":"hover"},
            "children":[{
                "type":"switch","id":"preview-switch","x":32,"y":24,
                "width":44,"height":24
            }]
        }"##,
    )
    .expect("native hover fixture parses")
    .value
}

fn screen_at(host: &WidgetHostNative, doc_x: f32, doc_y: f32) -> Point2D {
    let (canvas_x, canvas_y, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let viewport = host.editor_state.viewport;
    Point2D::new(
        canvas_x + viewport.pan_x + doc_x * viewport.zoom,
        canvas_y + viewport.pan_y + doc_y * viewport.zoom,
    )
}

#[test]
fn bare_native_cursor_move_sets_preview_session_hover() {
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(document()));
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    assert!(host.enter_preview((VIEWPORT_W, VIEWPORT_H)));
    host.preview_mode_transition = None;
    host.set_preview_device(
        op_editor_core::PreviewDeviceKind::Canvas,
        VIEWPORT_W,
        VIEWPORT_H,
    );
    host.editor_state_dirty = false;

    let point = screen_at(&host, 40.0, 36.0);
    assert!(host.apply_cursor_move(point.x, point.y));
    assert!(
        host.editor_state_dirty,
        "hover enter must request a repaint"
    );
    assert_eq!(
        host.preview
            .as_ref()
            .expect("preview session")
            .interaction()
            .hovered_node(),
        Some("preview-switch")
    );

    let empty = screen_at(&host, 160.0, 120.0);
    host.editor_state_dirty = false;
    let _ = host.apply_cursor_move(empty.x, empty.y);
    assert!(
        host.editor_state_dirty,
        "hover leave must request a repaint"
    );
    assert_eq!(
        host.preview.as_ref().unwrap().interaction().hovered_node(),
        None,
        "moving within the preview to a dead canvas area clears hover"
    );
}
