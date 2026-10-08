use super::WidgetHostNative;
use op_editor_core::{EditorState, HomeFamily};
use op_editor_ui::widgets::{host_canvas_geometry, missing_fonts_notice::MissingFontsNotice};

fn host(touch: bool, w: f32, h: f32) -> WidgetHostNative {
    let source = r#"{"version":"1.0.0","children":[{"type":"frame","id":"work","width":1080,"height":1440,"children":[{"type":"text","id":"title","content":"店名与标题","fontFamily":"QA Unknown Notice Face","fontSize":80}]}]}"#;
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.home.visible = true;
    host.install_imported_state(EditorState::from_document(
        jian_ops_schema::load_str(source).unwrap().value,
    ));
    let state = host.editor_state_mut();
    state.editor_ui.home.hide();
    state.editor_ui.touch = touch;
    state.editor_ui.size_class = op_editor_core::size_class::size_class(w, h);
    state
        .editor_ui
        .workspace
        .open_for_reading(HomeFamily::KnowledgeCards, 1_000);
    host
}

#[test]
fn fitted_artwork_clears_the_font_notice_on_phone_tablet_and_desktop() {
    for (touch, w, h) in [
        (true, 390.0, 844.0),
        (true, 1024.0, 768.0),
        (false, 1440.0, 900.0),
    ] {
        let mut host = host(touch, w, h);
        let original = host.editor_state().doc.clone();
        host.apply_workspace_fit(w, h);
        let state = host.editor_state();
        let notice = MissingFontsNotice::for_editor(state)
            .unwrap()
            .rect(w, h)
            .unwrap();
        let canvas = host_canvas_geometry::canvas_rect(state, w, h);
        let top = canvas.origin.y + state.viewport.pan_y;
        let bottom = top + 1440.0 * state.viewport.zoom;
        assert!(
            top >= notice.origin.y + notice.size.y + 12.0,
            "{w}x{h}: artwork {top} under notice {notice:?}"
        );
        assert!(
            bottom <= canvas.origin.y + canvas.size.y,
            "the footer also stays in view"
        );
        assert_eq!(state.doc, original);
    }
}

#[test]
fn dismissing_the_notice_restores_phone_fit_without_changing_canvas_coordinates() {
    let (w, h) = (390.0, 844.0);
    let mut host = host(true, w, h);
    host.frame_reader_board(w, h);
    let original = host.editor_state().doc.clone();
    let canvas = host_canvas_geometry::canvas_rect(host.editor_state(), w, h);
    let viewport = host.editor_state().viewport;
    let rect = MissingFontsNotice::for_editor(host.editor_state())
        .unwrap()
        .rect(w, h)
        .unwrap();
    let close = MissingFontsNotice::dismiss_rect(rect);
    assert!(host.apply_press(close.origin.x + 22.0, close.origin.y + 22.0, w, h));
    host.apply_release_with_viewport(w, h);
    host.sync_reader_stage(w, h);
    assert!(host.editor_state().viewport.zoom >= viewport.zoom);
    assert!(host.editor_state().viewport.pan_y < viewport.pan_y);
    assert_eq!(
        host_canvas_geometry::canvas_rect(host.editor_state(), w, h),
        canvas
    );
    assert_eq!(host.editor_state().doc, original);
}
