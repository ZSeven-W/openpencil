use crate::desc::{Callbacks, CreateOptions, OpPointerPhase};
use crate::lifecycle::{OpEngine, Session};
use crate::OpStatus;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const SAMPLE_DOCUMENT: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn editor_session(callbacks: Callbacks) -> Session {
    Session::new(CreateOptions {
        document: SAMPLE_DOCUMENT.to_owned(),
        width: 800.0,
        height: 600.0,
        dpr: 1.0,
        callbacks,
        asset_base: None,
        editor_mode: true,
        documents_root: None,
    })
    .expect("editor session")
}

fn canvas_point(session: &Session) -> (f32, f32) {
    let (width, height) = session.editor_viewport();
    let host = session.editor().expect("editor host");
    (60..height as usize)
        .step_by(40)
        .flat_map(|y| {
            (40..width as usize)
                .step_by(40)
                .map(move |x| (x as f32, y as f32))
        })
        .find(|(x, y)| host.canvas_doc_point(*x, *y, width, height).is_some())
        .expect("fixture viewport contains canvas")
}

fn non_canvas_point(session: &Session) -> (f32, f32) {
    let point = (4.0, 4.0);
    let (width, height) = session.editor_viewport();
    assert_eq!(
        session
            .editor()
            .expect("editor host")
            .canvas_doc_point(point.0, point.1, width, height),
        None,
        "top-left chrome point must stay outside the canvas"
    );
    point
}

#[test]
fn generic_pointer_keeps_the_primary_touch_and_publishes_only_canvas_points() {
    let mut engine = OpEngine::new(editor_session(Callbacks::default()));
    let canvas = canvas_point(engine.session_mut_for_test());
    let chrome = non_canvas_point(engine.session_mut_for_test());
    let engine_ptr = &mut engine as *mut OpEngine;

    assert_eq!(
        unsafe {
            crate::op_pointer(
                engine_ptr,
                7,
                OpPointerPhase::Down as i32,
                canvas.0,
                canvas.1,
                1,
            )
        },
        OpStatus::Ok
    );
    assert!(engine
        .session_mut_for_test()
        .editor_presence_cursor()
        .is_some());

    // A secondary touch over chrome neither steals nor clears the primary
    // canvas cursor.
    assert_eq!(
        unsafe {
            crate::op_pointer(
                engine_ptr,
                8,
                OpPointerPhase::Down as i32,
                chrome.0,
                chrome.1,
                2,
            )
        },
        OpStatus::Ok
    );
    assert!(engine
        .session_mut_for_test()
        .editor_presence_cursor()
        .is_some());
    assert_eq!(
        unsafe {
            crate::op_pointer(
                engine_ptr,
                8,
                OpPointerPhase::Up as i32,
                chrome.0,
                chrome.1,
                3,
            )
        },
        OpStatus::Ok
    );
    assert!(engine
        .session_mut_for_test()
        .editor_presence_cursor()
        .is_some());

    assert_eq!(
        unsafe {
            crate::op_pointer(
                engine_ptr,
                7,
                OpPointerPhase::Move as i32,
                chrome.0,
                chrome.1,
                4,
            )
        },
        OpStatus::Ok
    );
    assert_eq!(
        engine.session_mut_for_test().editor_presence_cursor(),
        None,
        "a tracked touch over editor chrome publishes cursor None"
    );
    assert_eq!(
        unsafe {
            crate::op_pointer(
                engine_ptr,
                7,
                OpPointerPhase::Up as i32,
                chrome.0,
                chrome.1,
                5,
            )
        },
        OpStatus::Ok
    );
    assert!(engine
        .session_mut_for_test()
        .editor_collab
        .as_ref()
        .expect("collab state")
        .presence_pointer
        .is_none());
}

#[test]
fn legacy_press_move_release_and_hover_follow_some_none_semantics() {
    let mut engine = OpEngine::new(editor_session(Callbacks::default()));
    let canvas = canvas_point(engine.session_mut_for_test());
    let chrome = non_canvas_point(engine.session_mut_for_test());
    let engine_ptr = &mut engine as *mut OpEngine;

    assert_eq!(
        unsafe { crate::op_editor_press(engine_ptr, canvas.0, canvas.1) },
        OpStatus::Ok
    );
    assert!(engine
        .session_mut_for_test()
        .editor_presence_cursor()
        .is_some());
    assert_eq!(
        unsafe { crate::op_editor_move(engine_ptr, chrome.0, chrome.1) },
        OpStatus::Ok
    );
    assert_eq!(engine.session_mut_for_test().editor_presence_cursor(), None);
    assert_eq!(
        unsafe { crate::op_editor_release(engine_ptr, chrome.0, chrome.1) },
        OpStatus::Ok
    );
    assert!(engine
        .session_mut_for_test()
        .editor_collab
        .as_ref()
        .expect("collab state")
        .presence_pointer
        .is_none());

    assert_eq!(
        unsafe { crate::op_editor_hover(engine_ptr, canvas.0, canvas.1) },
        OpStatus::Ok
    );
    assert!(engine
        .session_mut_for_test()
        .editor_presence_cursor()
        .is_some());
    assert_eq!(
        unsafe { crate::op_editor_hover(engine_ptr, chrome.0, chrome.1) },
        OpStatus::Ok
    );
    assert_eq!(engine.session_mut_for_test().editor_presence_cursor(), None);
    assert_eq!(
        unsafe { crate::op_editor_hover(engine_ptr, -1.0, -1.0) },
        OpStatus::Ok
    );
    assert!(engine
        .session_mut_for_test()
        .editor_collab
        .as_ref()
        .expect("collab state")
        .presence_pointer
        .is_none());
}

unsafe extern "C" fn count_redraws(
    user_data: *mut c_void,
    _has_next_wake: bool,
    _next_wake_ms: u64,
) {
    let counter = unsafe { &*(user_data.cast::<AtomicUsize>()) };
    counter.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn presence_only_cursor_motion_requests_a_redraw() {
    let counter = Box::new(AtomicUsize::new(0));
    let callbacks = Callbacks {
        user_data: (&*counter as *const AtomicUsize).cast_mut().cast(),
        needs_redraw: Some(count_redraws),
        ..Callbacks::default()
    };
    let mut session = editor_session(callbacks);
    let (width, height) = session.editor_viewport();

    // Find adjacent canvas points with identical host hover state. The second
    // call is therefore presence-only: `apply_cursor_move` returns false.
    let (first, second) = (60..height as usize)
        .step_by(40)
        .flat_map(|y| {
            (40..(width - 2.0).max(41.0) as usize)
                .step_by(40)
                .map(move |x| ((x as f32, y as f32), (x as f32 + 0.25, y as f32)))
        })
        .find(|(first, second)| {
            let over_canvas = session
                .editor()
                .expect("editor host")
                .canvas_doc_point(first.0, first.1, width, height)
                .is_some()
                && session
                    .editor()
                    .expect("editor host")
                    .canvas_doc_point(second.0, second.1, width, height)
                    .is_some();
            if !over_canvas {
                return false;
            }
            let host = session.editor_mut().expect("editor host");
            let _ = host.apply_cursor_move(first.0, first.1);
            !host.apply_cursor_move(second.0, second.1)
        })
        .expect("fixture has adjacent canvas points with stable hover");
    assert!(session.set_editor_presence_pointer(first.0, first.1));
    counter.store(0, Ordering::Relaxed);

    let mut engine = OpEngine::new(session);
    let engine_ptr = &mut engine as *mut OpEngine;
    assert_eq!(
        unsafe { crate::op_editor_hover(engine_ptr, second.0, second.1) },
        OpStatus::Ok
    );
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

static WORKER_WAKE_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn count_worker_wake(
    _user_data: *mut c_void,
    _has_next_wake: bool,
    _next_wake_ms: u64,
) {
    WORKER_WAKE_CALLS.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn worker_wake_restarts_the_platform_pump_once_per_pending_edge() {
    WORKER_WAKE_CALLS.store(0, Ordering::Relaxed);
    let pending = Arc::new(AtomicBool::new(false));
    let callbacks = Callbacks {
        needs_redraw: Some(count_worker_wake),
        ..Callbacks::default()
    };
    let notify = crate::editor_collab::mobile_wake_notifier(Arc::clone(&pending), callbacks);
    let worker_notify = Arc::clone(&notify);
    std::thread::spawn(move || {
        worker_notify();
        worker_notify();
    })
    .join()
    .unwrap();
    assert_eq!(WORKER_WAKE_CALLS.load(Ordering::Relaxed), 1);
    assert!(pending.swap(false, Ordering::AcqRel));
    notify();
    assert_eq!(WORKER_WAKE_CALLS.load(Ordering::Relaxed), 2);
}

#[test]
fn suspend_forces_terminal_cursor_none_past_the_presence_throttle() {
    let mut session = editor_session(Callbacks::default());
    let baseline = session
        .editor()
        .expect("editor host")
        .editor_state()
        .doc
        .clone();
    let (runtime, lane) = op_collab_host::test_support::owner_session(baseline);
    session.install_editor_collab_runtime_for_test(runtime);
    let point = canvas_point(&session);
    assert!(session.set_editor_presence_pointer(point.0, point.1));

    let _ = session.pump_editor_collab();
    assert_eq!(lane.drain_command_count(), 1, "cursor Some was queued");

    // Suspend immediately, still inside the 33 ms presence interval. The
    // terminal None must nevertheless occupy its own outbound command.
    session.suspend();
    assert_eq!(lane.drain_command_count(), 1, "terminal None was queued");
    assert!(session
        .editor_collab
        .as_ref()
        .expect("collab state")
        .presence_pointer
        .is_none());

    session.shutdown_editor_collab();
    drop(lane);
}
