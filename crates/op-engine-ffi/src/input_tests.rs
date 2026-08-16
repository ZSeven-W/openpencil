use super::*;
use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{call_session, OpEngine, Session};
use crate::OpStatus;

const LOGICAL: (f32, f32) = (834.0, 1_112.0);
const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn event(
    tracker: &mut GestureTracker,
    view: &mut Viewport,
    id: u32,
    phase: OpPointerPhase,
    x: f32,
    y: f32,
) -> GestureOutcome {
    tracker
        .handle(view, &LayoutScene::default(), LOGICAL, id, phase, x, y)
        .expect("gesture event")
}

fn assert_point_close(actual: Point2D, expected: Point2D) {
    assert!(
        (actual.x - expected.x).abs() < 0.001,
        "x mismatch: actual={}, expected={}",
        actual.x,
        expected.x
    );
    assert!(
        (actual.y - expected.y).abs() < 0.001,
        "y mismatch: actual={}, expected={}",
        actual.y,
        expected.y
    );
}

#[test]
fn pinch_out_and_in_keep_the_second_down_distance_baseline() {
    let mut tracker = GestureTracker::default();
    let mut view = Viewport {
        origin: Point2D::ZERO,
        zoom: 1.5,
    };

    event(
        &mut tracker,
        &mut view,
        1,
        OpPointerPhase::Down,
        100.0,
        100.0,
    );
    event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Down,
        140.0,
        100.0,
    );
    let anchor = tracker.pinch.expect("second Down must anchor pinch");
    assert!((anchor.dist_start - 40.0).abs() < f32::EPSILON);
    assert!((anchor.zoom_start - 1.5).abs() < f32::EPSILON);

    let out = event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Move,
        180.0,
        100.0,
    );
    assert!(out.viewport_changed);
    assert!((view.zoom - 3.0).abs() < 0.001);

    let inward = event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Move,
        120.0,
        100.0,
    );
    assert!(inward.viewport_changed);
    assert!((view.zoom - 0.75).abs() < 0.001);
    assert!((tracker.pinch.unwrap().dist_start - 40.0).abs() < f32::EPSILON);
}

#[test]
fn pinch_keeps_the_original_doc_anchor_under_each_moving_midpoint() {
    let mut tracker = GestureTracker::default();
    let mut view = Viewport {
        origin: Point2D::new(10.0, 20.0),
        zoom: 2.0,
    };

    event(&mut tracker, &mut view, 1, OpPointerPhase::Down, 80.0, 90.0);
    event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Down,
        120.0,
        110.0,
    );
    let doc_anchor = Point2D::new(45.0, 40.0);
    assert_point_close(tracker.pinch.unwrap().doc_anchor, doc_anchor);

    event(
        &mut tracker,
        &mut view,
        1,
        OpPointerPhase::Move,
        60.0,
        120.0,
    );
    assert_point_close(view.view_to_doc(Point2D::new(90.0, 115.0)), doc_anchor);

    event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Move,
        150.0,
        140.0,
    );
    assert_point_close(view.view_to_doc(Point2D::new(105.0, 130.0)), doc_anchor);
}

#[test]
fn lifting_both_fingers_after_a_two_finger_tap_never_completes_a_tap() {
    let mut tracker = GestureTracker::default();
    let mut view = Viewport {
        origin: Point2D::ZERO,
        zoom: 1.0,
    };

    event(&mut tracker, &mut view, 1, OpPointerPhase::Down, 50.0, 50.0);
    event(&mut tracker, &mut view, 2, OpPointerPhase::Down, 70.0, 50.0);
    let first_up = event(&mut tracker, &mut view, 2, OpPointerPhase::Up, 70.0, 50.0);
    assert!(!first_up.tap_completed);
    assert!(tracker.tap_suppressed);

    let last_up = event(&mut tracker, &mut view, 1, OpPointerPhase::Up, 50.0, 50.0);
    assert!(!last_up.tap_completed);
    assert!(!tracker.tap_suppressed);

    event(&mut tracker, &mut view, 3, OpPointerPhase::Down, 50.0, 50.0);
    let fresh_tap = event(&mut tracker, &mut view, 3, OpPointerPhase::Up, 50.0, 50.0);
    assert!(fresh_tap.tap_completed);
}

#[test]
fn cancel_clears_pinch_and_makes_late_up_a_noop() {
    let mut tracker = GestureTracker::default();
    let mut view = Viewport {
        origin: Point2D::ZERO,
        zoom: 1.0,
    };

    event(
        &mut tracker,
        &mut view,
        1,
        OpPointerPhase::Down,
        100.0,
        100.0,
    );
    event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Down,
        140.0,
        100.0,
    );
    event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Move,
        180.0,
        100.0,
    );

    let cancel = event(
        &mut tracker,
        &mut view,
        1,
        OpPointerPhase::Cancel,
        100.0,
        100.0,
    );
    assert!(!cancel.tap_completed);
    assert!(tracker.touches.is_empty());
    assert!(tracker.pinch.is_none());
    assert!(!tracker.panning);
    assert!(!tracker.tap_suppressed);

    for id in [1, 2] {
        let late_up = event(
            &mut tracker,
            &mut view,
            id,
            OpPointerPhase::Up,
            100.0,
            100.0,
        );
        assert!(!late_up.tap_completed);
        assert!(!late_up.viewport_changed);
    }
}

#[test]
fn three_finger_handoff_reanchors_without_jump_or_synthesized_tap() {
    let mut tracker = GestureTracker::default();
    let mut view = Viewport {
        origin: Point2D::ZERO,
        zoom: 1.0,
    };

    event(
        &mut tracker,
        &mut view,
        1,
        OpPointerPhase::Down,
        100.0,
        100.0,
    );
    event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Down,
        140.0,
        100.0,
    );
    event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Move,
        180.0,
        100.0,
    );
    assert!((view.zoom - 2.0).abs() < 0.001);

    let before_handoff = view;
    event(
        &mut tracker,
        &mut view,
        3,
        OpPointerPhase::Down,
        220.0,
        100.0,
    );
    assert!(tracker.pinch.is_none());
    let first_up = event(&mut tracker, &mut view, 1, OpPointerPhase::Up, 100.0, 100.0);
    assert!(!first_up.tap_completed);
    assert_point_close(view.origin, before_handoff.origin);
    assert!((view.zoom - before_handoff.zoom).abs() < 0.001);

    let handoff_anchor = tracker
        .pinch
        .expect("remaining pair must get a fresh anchor");
    assert!((handoff_anchor.dist_start - 40.0).abs() < 0.001);
    assert!((handoff_anchor.zoom_start - 2.0).abs() < 0.001);

    event(
        &mut tracker,
        &mut view,
        2,
        OpPointerPhase::Move,
        180.0,
        100.0,
    );
    assert_point_close(view.origin, before_handoff.origin);
    assert!((view.zoom - before_handoff.zoom).abs() < 0.001);

    event(
        &mut tracker,
        &mut view,
        3,
        OpPointerPhase::Move,
        260.0,
        100.0,
    );
    assert!((view.zoom - 4.0).abs() < 0.001);
    assert_point_close(
        view.view_to_doc(Point2D::new(220.0, 100.0)),
        handoff_anchor.doc_anchor,
    );

    let second_up = event(&mut tracker, &mut view, 2, OpPointerPhase::Up, 180.0, 100.0);
    let last_up = event(&mut tracker, &mut view, 3, OpPointerPhase::Up, 260.0, 100.0);
    assert!(!second_up.tap_completed);
    assert!(!last_up.tap_completed);
}

fn viewer_engine() -> OpEngine {
    OpEngine::new(
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: LOGICAL.0,
            height: LOGICAL.1,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            #[cfg(feature = "editor")]
            editor_mode: false,
        })
        .expect("viewer session"),
    )
}

fn viewer_snapshot(engine: &mut OpEngine) -> (Point2D, f32, bool) {
    let mut snapshot = None;
    let status = unsafe {
        call_session(engine as *mut OpEngine, |session| {
            snapshot = Some((
                session.viewport_origin,
                session.zoom,
                session.user_interacted,
            ));
            Ok(())
        })
    };
    assert_eq!(status, OpStatus::Ok);
    snapshot.expect("session snapshot")
}

#[test]
fn public_pointer_abi_applies_pinch_ratio_and_midpoint_anchor_in_viewer_mode() {
    let mut engine = viewer_engine();
    let engine_ptr = &mut engine as *mut OpEngine;
    let (origin_start, zoom_start, interacted_start) = viewer_snapshot(&mut engine);
    assert!(!interacted_start);
    let midpoint = Point2D::new(250.0, 300.0);
    let doc_anchor = Viewport {
        origin: origin_start,
        zoom: zoom_start,
    }
    .view_to_doc(midpoint);

    let events = [
        (1, OpPointerPhase::Down, 200.0, 300.0),
        (2, OpPointerPhase::Down, 300.0, 300.0),
        (1, OpPointerPhase::Move, 150.0, 300.0),
        (2, OpPointerPhase::Move, 350.0, 300.0),
    ];
    for (index, (id, phase, x, y)) in events.into_iter().enumerate() {
        assert_eq!(
            unsafe { crate::op_pointer(engine_ptr, id, phase as i32, x, y, index as u64) },
            OpStatus::Ok
        );
    }

    let (origin, zoom, interacted) = viewer_snapshot(&mut engine);
    assert!((zoom - zoom_start * 2.0).abs() < 0.001);
    assert!(interacted);
    assert_point_close(Viewport { origin, zoom }.view_to_doc(midpoint), doc_anchor);
}
