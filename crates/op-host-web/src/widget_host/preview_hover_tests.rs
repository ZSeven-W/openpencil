//! CanvasKit preview hover forwarding coverage.

#![cfg(all(test, feature = "canvaskit"))]

use super::WidgetHost;
use op_editor_core::EditorState;
use op_editor_ui::Point2D;
use std::collections::BTreeMap;

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
    .expect("web hover fixture parses")
    .value
}

fn screen_at(host: &WidgetHost, doc_x: f32, doc_y: f32) -> Point2D {
    let (canvas_x, canvas_y, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let viewport = host.editor_state.viewport;
    Point2D::new(
        canvas_x + viewport.pan_x + doc_x * viewport.zoom,
        canvas_y + viewport.pan_y + doc_y * viewport.zoom,
    )
}

#[test]
fn bare_web_cursor_move_sets_preview_session_hover() {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::from_document(document());
    host.editor_state.editor_ui.preview.mode = true;
    host.editor_state_dirty = true;
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    let active_theme = BTreeMap::new();
    host.preview = Some(
        op_preview_core::PreviewSession::enter(
            &host.editor_state.doc,
            (VIEWPORT_W, VIEWPORT_H),
            &active_theme,
            0,
            false,
            false,
            jian_core::layout::measure::default_backend(),
            0,
        )
        .expect("web preview session"),
    );
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
