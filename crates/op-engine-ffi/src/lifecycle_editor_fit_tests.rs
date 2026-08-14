use super::*;
use crate::desc::{Callbacks, CreateOptions};

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn editor_session(width: f32, height: f32) -> Session {
    Session::new(CreateOptions {
        document: SAMPLE_DOC.to_owned(),
        width,
        height,
        dpr: 1.0,
        callbacks: Callbacks::default(),
        asset_base: None,
        editor_mode: true,
    })
    .expect("editor session")
}

fn assert_host_is_fitted(session: &Session) {
    let (viewport_w, viewport_h) = session.editor_viewport();
    let host = session.editor().expect("editor host");
    let (_, _, canvas_w, canvas_h) = op_editor_ui::widgets::host_canvas_geometry::canvas_region(
        host.editor_state(),
        viewport_w,
        viewport_h,
    );
    let view = host.editor_state().viewport;
    let expected_zoom = ((canvas_w - 128.0).max(1.0) / 1080.0)
        .min((canvas_h - 128.0).max(1.0) / 1440.0)
        .clamp(0.1, 1.0);
    assert!((view.zoom - expected_zoom).abs() < 0.001);
    assert!((view.pan_x + 540.0 * view.zoom - canvas_w / 2.0).abs() < 0.01);
    assert!((view.pan_y + 720.0 * view.zoom - canvas_h / 2.0).abs() < 0.01);
}

#[test]
fn editor_host_fits_on_create_and_uninteracted_resize() {
    let mut session = editor_session(800.0, 600.0);
    assert_host_is_fitted(&session);
    let initial_zoom = session.editor().unwrap().editor_state().viewport.zoom;

    session.resize(1_200.0, 900.0, 1.0).unwrap();
    assert_host_is_fitted(&session);
    assert!(session.editor().unwrap().editor_state().viewport.zoom > initial_zoom);
}

#[test]
fn resize_resyncs_and_clamps_local_keyboard_occlusion() {
    let mut session = editor_session(800.0, 600.0);
    session.insets = OpInsets {
        top: 20.0,
        right: 10.0,
        bottom: 34.0,
        left: 10.0,
    };
    session.keyboard = 300.0;
    session.sync_editor_keyboard_occlusion();

    session.resize(800.0, 250.0, 1.0).unwrap();
    let viewport = session.editor_viewport();
    assert_eq!(viewport, (780.0, 196.0));
    assert_eq!(
        session
            .editor()
            .unwrap()
            .keyboard_visible_bottom(viewport.1),
        0.0
    );

    session.resize(800.0, 700.0, 1.0).unwrap();
    let viewport = session.editor_viewport();
    assert_eq!(viewport, (780.0, 646.0));
    assert_eq!(
        session
            .editor()
            .unwrap()
            .keyboard_visible_bottom(viewport.1),
        380.0
    );
}

#[test]
fn safe_area_refits_but_user_camera_survives_resize() {
    let mut engine = OpEngine::new(editor_session(800.0, 600.0));
    let engine_ptr = &mut engine as *mut OpEngine;

    assert_eq!(
        unsafe { crate::op_set_safe_area(engine_ptr, 24.0, 10.0, 220.0, 10.0) },
        OpStatus::Ok
    );
    let session = unsafe { &*engine.session.get() };
    assert_host_is_fitted(session);

    assert_eq!(
        unsafe { crate::op_editor_begin_transform(engine_ptr, 400.0, 180.0) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_editor_pan(engine_ptr, 400.0, 180.0, 32.0, 12.0) },
        OpStatus::Ok
    );
    let session = unsafe { &*engine.session.get() };
    assert!(session.user_interacted);
    let panned = session.editor().unwrap().editor_state().viewport;

    assert_eq!(
        unsafe { crate::op_resize(engine_ptr, 1_200.0, 900.0, 1.0) },
        OpStatus::Ok
    );
    let session = unsafe { &*engine.session.get() };
    assert_eq!(session.editor().unwrap().editor_state().viewport, panned);
}
