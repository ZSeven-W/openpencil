use super::Session;
use crate::desc::{Callbacks, CreateOptions};
use crate::{OpEngine, OpStatus};
use op_editor_core::PropertyFocus;

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn editor_engine() -> OpEngine {
    OpEngine::new(
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: 834.0,
            height: 1_112.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
        })
        .expect("editor session"),
    )
}

#[test]
fn single_finger_canvas_pan_marks_user_interaction_and_survives_resize() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    let (start_x, start_y, before) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let host = session.editor().expect("editor host");
        let (canvas_x, canvas_y, _, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (
            canvas_x + 12.0,
            canvas_y + canvas_h / 2.0,
            host.editor_state().viewport,
        )
    };

    assert_eq!(
        unsafe { crate::op_editor_press(engine_ptr, start_x, start_y) },
        OpStatus::Ok
    );
    assert!(!engine.session_mut_for_test().user_interacted);
    assert_eq!(
        unsafe { crate::op_editor_move(engine_ptr, start_x + 24.0, start_y + 12.0) },
        OpStatus::Ok
    );
    let session = engine.session_mut_for_test();
    assert!(session.user_interacted);
    assert_ne!(session.editor().unwrap().editor_state().viewport, before);

    assert_eq!(
        unsafe { crate::op_editor_release(engine_ptr, start_x + 24.0, start_y + 12.0) },
        OpStatus::Ok
    );
    let panned = engine
        .session_mut_for_test()
        .editor()
        .unwrap()
        .editor_state()
        .viewport;
    assert_eq!(
        unsafe { crate::op_resize(engine_ptr, 1_194.0, 834.0, 1.0) },
        OpStatus::Ok
    );
    assert_eq!(
        engine
            .session_mut_for_test()
            .editor()
            .unwrap()
            .editor_state()
            .viewport,
        panned
    );
}

#[test]
fn generic_pointer_move_also_marks_canvas_camera_interaction() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    let (start_x, start_y) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let host = session.editor().expect("editor host");
        let (canvas_x, canvas_y, _, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (canvas_x + 12.0, canvas_y + canvas_h / 2.0)
    };

    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 1, 0, start_x, start_y, 1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 1, 1, start_x + 20.0, start_y, 2) },
        OpStatus::Ok
    );
    assert!(engine.session_mut_for_test().user_interacted);
}

#[test]
fn direct_editor_pinch_updates_zoom_pan_and_interaction_state() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    let insets = crate::viewport::OpInsets {
        top: 47.0,
        right: 13.0,
        bottom: 31.0,
        left: 29.0,
    };
    assert_eq!(
        unsafe {
            crate::op_set_safe_area(
                engine_ptr,
                insets.top,
                insets.right,
                insets.bottom,
                insets.left,
            )
        },
        OpStatus::Ok
    );
    let (surface_x, surface_y, before) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let host = session.editor().expect("editor host");
        let (canvas_x, canvas_y, canvas_w, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (
            insets.left + canvas_x + canvas_w * 0.6,
            insets.top + canvas_y + canvas_h * 0.4,
            host.editor_state().viewport,
        )
    };

    assert_eq!(
        unsafe { crate::op_editor_begin_transform(engine_ptr, surface_x, surface_y) },
        OpStatus::Ok
    );
    assert!(engine.session_mut_for_test().editor_transform_captured());
    assert_eq!(
        unsafe { crate::op_editor_pinch(engine_ptr, surface_x, surface_y, 160.0) },
        OpStatus::Ok
    );

    let session = engine.session_mut_for_test();
    let after = session.editor().unwrap().editor_state().viewport;
    assert!(after.zoom > before.zoom, "pinch must increase zoom");
    assert_ne!(after.pan_x, before.pan_x, "zoom anchor must update pan x");
    assert_ne!(after.pan_y, before.pan_y, "zoom anchor must update pan y");
    assert!(session.user_interacted);
    assert_eq!(session.insets, insets, "pinch must not alter safe bands");
}

#[test]
fn generic_pointer_uses_safe_area_local_editor_geometry() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { crate::op_set_safe_area(engine_ptr, 47.0, 13.0, 31.0, 29.0) },
        OpStatus::Ok
    );
    let (surface_x, surface_y) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, _) = session.editor_viewport();
        let host = session.editor().expect("editor host");
        let bar = op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_rect(
            host.editor_state(),
            viewport_w,
        );
        let layers = op_editor_ui::widgets::MobileAppBar::layers_rect(bar);
        (
            session.insets.left + layers.origin.x + layers.size.x / 2.0,
            session.insets.top + layers.origin.y + layers.size.y / 2.0,
        )
    };

    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 1, 0, surface_x, surface_y, 1) },
        OpStatus::Ok
    );
    assert_eq!(
        engine
            .session_mut_for_test()
            .editor()
            .unwrap()
            .editor_state()
            .editor_ui
            .mobile_sheet,
        Some(op_editor_core::size_class::MobileSheetKind::Layers),
        "generic input must subtract the asymmetric safe-area origin before host hit-testing"
    );
}

#[test]
fn generic_pointer_ignores_streams_starting_in_all_safe_area_bands() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { crate::op_set_safe_area(engine_ptr, 47.0, 13.0, 31.0, 29.0) },
        OpStatus::Ok
    );
    let (interior_x, interior_y, before_viewport) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let (safe_left, safe_top) = (session.insets.left, session.insets.top);
        let host = session.editor_mut().expect("editor host");
        let state = host.editor_state_mut();
        state.set_single_selection(op_editor_core::NodeId::new("f18"));
        state.editor_ui.mobile_sheet =
            Some(op_editor_core::size_class::MobileSheetKind::Properties);
        state.ui.property_focus = Some(PropertyFocus::PositionX);
        state.ui.property_input.set_text("123");
        let (canvas_x, canvas_y, _, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (
            safe_left + canvas_x + 12.0,
            safe_top + canvas_y + canvas_h / 2.0,
            host.editor_state().viewport,
        )
    };
    assert!(
        engine
            .session_mut_for_test()
            .editor()
            .unwrap()
            .text_input_focus_active(),
        "property focus must be active before the system-band gestures"
    );

    let band_starts = [
        (14.0, 500.0),
        (400.0, 23.0),
        (828.0, 500.0),
        (400.0, 1_096.0),
    ];
    for (index, (band_x, band_y)) in band_starts.into_iter().enumerate() {
        let id = index as u32 + 10;
        assert_eq!(
            unsafe { crate::op_pointer(engine_ptr, id, 0, band_x, band_y, id as u64 * 10) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe {
                crate::op_pointer(
                    engine_ptr,
                    id,
                    1,
                    interior_x,
                    interior_y,
                    id as u64 * 10 + 1,
                )
            },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe {
                crate::op_pointer(
                    engine_ptr,
                    id,
                    2,
                    interior_x,
                    interior_y,
                    id as u64 * 10 + 2,
                )
            },
            OpStatus::Ok
        );

        let session = engine.session_mut_for_test();
        let host = session.editor().unwrap();
        assert_eq!(host.editor_state().viewport, before_viewport);
        assert_eq!(
            host.editor_state().selection.anchor,
            op_editor_core::NodeId::new("f18")
        );
        assert_eq!(
            host.editor_state().ui.property_focus,
            Some(PropertyFocus::PositionX)
        );
        assert!(host.text_input_focus_active());
        assert!(!session.user_interacted);
    }
}

#[test]
fn generic_pointer_keeps_editor_capture_after_moving_into_safe_band() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { crate::op_set_safe_area(engine_ptr, 47.0, 13.0, 31.0, 29.0) },
        OpStatus::Ok
    );
    let (start_x, start_y, before) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let (safe_left, safe_top) = (session.insets.left, session.insets.top);
        let host = session.editor().expect("editor host");
        let (canvas_x, canvas_y, _, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (
            safe_left + canvas_x + 12.0,
            safe_top + canvas_y + canvas_h / 2.0,
            host.editor_state().viewport,
        )
    };

    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 40, 0, start_x, start_y, 1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 40, 1, 0.0, start_y, 2) },
        OpStatus::Ok
    );
    let session = engine.session_mut_for_test();
    assert_ne!(session.editor().unwrap().editor_state().viewport, before);
    assert!(session.user_interacted);
    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 40, 2, 0.0, start_y, 3) },
        OpStatus::Ok
    );
}

#[test]
fn direct_editor_apis_ignore_all_safe_area_band_starts() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { crate::op_set_safe_area(engine_ptr, 47.0, 13.0, 31.0, 29.0) },
        OpStatus::Ok
    );
    let (interior_x, interior_y, before_viewport) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let (safe_left, safe_top) = (session.insets.left, session.insets.top);
        let host = session.editor_mut().expect("editor host");
        let state = host.editor_state_mut();
        state.set_single_selection(op_editor_core::NodeId::new("f18"));
        state.editor_ui.mobile_sheet =
            Some(op_editor_core::size_class::MobileSheetKind::Properties);
        state.ui.property_focus = Some(PropertyFocus::PositionX);
        state.ui.property_input.set_text("123");
        let (canvas_x, canvas_y, _, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (
            safe_left + canvas_x + 12.0,
            safe_top + canvas_y + canvas_h / 2.0,
            host.editor_state().viewport,
        )
    };
    let band_starts = [
        (14.0, 500.0),
        (400.0, 23.0),
        (828.0, 500.0),
        (400.0, 1_096.0),
    ];

    for (band_x, band_y) in band_starts {
        assert_eq!(
            unsafe { crate::op_editor_press(engine_ptr, band_x, band_y) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { crate::op_editor_move(engine_ptr, interior_x, interior_y) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { crate::op_editor_release(engine_ptr, interior_x, interior_y) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { crate::op_editor_right_press(engine_ptr, band_x, band_y) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { crate::op_editor_cancel_gesture(engine_ptr) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { crate::op_editor_begin_transform(engine_ptr, band_x, band_y) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { crate::op_editor_pan(engine_ptr, interior_x, interior_y, 30.0, 12.0) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { crate::op_editor_pinch(engine_ptr, interior_x, interior_y, 20.0) },
            OpStatus::Ok
        );

        let session = engine.session_mut_for_test();
        let host = session.editor().unwrap();
        assert_eq!(host.editor_state().viewport, before_viewport);
        assert_eq!(
            host.editor_state().selection.anchor,
            op_editor_core::NodeId::new("f18")
        );
        assert_eq!(
            host.editor_state().ui.property_focus,
            Some(PropertyFocus::PositionX)
        );
        assert!(host.text_input_focus_active());
        assert!(!session.user_interacted);
    }
}

#[test]
fn direct_transform_ownership_comes_from_second_finger_down() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { crate::op_set_safe_area(engine_ptr, 47.0, 13.0, 31.0, 29.0) },
        OpStatus::Ok
    );
    let (x, y, before) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let (safe_left, safe_top) = (session.insets.left, session.insets.top);
        let host = session.editor().expect("editor host");
        let (canvas_x, canvas_y, canvas_w, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (
            safe_left + canvas_x + canvas_w / 2.0,
            safe_top + canvas_y + canvas_h / 2.0,
            host.editor_state().viewport,
        )
    };

    // Band Down stays suppressed even though the first Move is over content.
    assert_eq!(
        unsafe { crate::op_editor_begin_transform(engine_ptr, 14.0, y) },
        OpStatus::Ok
    );
    assert!(!engine.session_mut_for_test().editor_transform_captured());
    assert_eq!(
        unsafe { crate::op_editor_pan(engine_ptr, x, y, 40.0, 12.0) },
        OpStatus::Ok
    );
    assert_eq!(
        engine
            .session_mut_for_test()
            .editor()
            .unwrap()
            .editor_state()
            .viewport,
        before
    );

    // Content Down captures before movement begins and permits transforms.
    assert_eq!(
        unsafe { crate::op_editor_cancel_gesture(engine_ptr) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_editor_begin_transform(engine_ptr, x, y) },
        OpStatus::Ok
    );
    assert!(engine.session_mut_for_test().editor_transform_captured());
    assert_eq!(
        unsafe { crate::op_editor_pan(engine_ptr, 14.0, y, 40.0, 12.0) },
        OpStatus::Ok
    );
    assert!(
        engine.session_mut_for_test().editor_transform_captured(),
        "content-owned transform must stay captured when its first Move enters a band"
    );
    assert_eq!(
        unsafe { crate::op_editor_pan(engine_ptr, x, y, 40.0, 12.0) },
        OpStatus::Ok
    );
    assert_ne!(
        engine
            .session_mut_for_test()
            .editor()
            .unwrap()
            .editor_state()
            .viewport,
        before
    );
}

#[test]
fn direct_editor_pointer_keeps_capture_after_moving_into_safe_band() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { crate::op_set_safe_area(engine_ptr, 47.0, 13.0, 31.0, 29.0) },
        OpStatus::Ok
    );
    let (start_x, start_y, before) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let (safe_left, safe_top) = (session.insets.left, session.insets.top);
        let host = session.editor().expect("editor host");
        let (canvas_x, canvas_y, _, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (
            safe_left + canvas_x + 12.0,
            safe_top + canvas_y + canvas_h / 2.0,
            host.editor_state().viewport,
        )
    };

    assert_eq!(
        unsafe { crate::op_editor_press(engine_ptr, start_x, start_y) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_editor_move(engine_ptr, 0.0, start_y) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_editor_release(engine_ptr, 0.0, start_y) },
        OpStatus::Ok
    );
    let session = engine.session_mut_for_test();
    assert_ne!(session.editor().unwrap().editor_state().viewport, before);
    assert!(session.user_interacted);
}

#[test]
fn generic_pointer_cancel_discards_pending_blank_tap_and_unknown_phase_is_rejected() {
    let mut engine = editor_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    let (start_x, start_y, before) = {
        let session = engine.session_mut_for_test();
        let (viewport_w, viewport_h) = session.editor_viewport();
        let host = session.editor_mut().expect("editor host");
        host.editor_state_mut()
            .set_single_selection(op_editor_core::NodeId::new("f18"));
        let (canvas_x, canvas_y, _, canvas_h) =
            op_editor_ui::widgets::host_canvas_geometry::canvas_region(
                host.editor_state(),
                viewport_w,
                viewport_h,
            );
        (
            canvas_x + 12.0,
            canvas_y + canvas_h / 2.0,
            host.editor_state().viewport,
        )
    };

    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 1, 0, start_x, start_y, 1) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 1, 3, start_x, start_y, 2) },
        OpStatus::Ok
    );
    // A move and up after Cancel must not pan or replay the delayed blank tap.
    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 1, 1, start_x + 24.0, start_y, 3) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 1, 2, start_x + 24.0, start_y, 4) },
        OpStatus::Ok
    );
    let session = engine.session_mut_for_test();
    let host = session.editor().unwrap();
    assert_eq!(host.editor_state().viewport, before);
    assert_eq!(
        host.editor_state().selection.anchor,
        op_editor_core::NodeId::new("f18")
    );
    assert!(!session.user_interacted);

    assert_eq!(
        unsafe { crate::op_pointer(engine_ptr, 1, 99, start_x, start_y, 5) },
        OpStatus::InvalidArg
    );
}
