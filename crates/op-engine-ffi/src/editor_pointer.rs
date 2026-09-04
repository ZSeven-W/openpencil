//! Full-editor pointer ABI: the plain and timestamped press/move/release/
//! cancel entry points the mobile shells drive.
//!
//! Carved out of `editor.rs` (pure code motion + the `_at` additions) to keep
//! that spine under the repo's 800-line cap. Every old `op_editor_press` /
//! `op_editor_move` / `op_editor_release` / `op_editor_cancel_gesture` wrapper
//! reuses the SAME internal implementation as its `_at` twin, stamped with the
//! session's current clock — so old and new binary consumers share one code
//! path and never nest `call_session`.
//!
//! Clock contract:
//! - The internal implementation advances the engine's GLOBAL clocks
//!   ([`Session::now_ms`] and, through it, `WidgetHostNative::now_ms` and the
//!   live preview runtime) MONOTONICALLY to the event's timestamp BEFORE any
//!   early return — safe-area miss, pointer-capture miss, collaboration
//!   suppression, and Cancel included — so a pointer event between frames can
//!   never leave a downstream gate reading the last frame pump's stale time.
//! - The factual event time is carried INDEPENDENTLY into the host's
//!   live-preview pointer path via the scoped `apply_press_at` /
//!   `apply_cursor_move_at` / `apply_release_with_viewport_at` /
//!   `cancel_native_touch_gestures_at` context: a frame pump at 2000 followed
//!   by Down(t=950) + Move(t=1050) keeps every global clock at 2000 while the
//!   Swipe recognizer measures the factual 100 ms delta.

use crate::error::FfiResult;
use crate::lifecycle::Session;
use crate::OpStatus;

/// Pointer press (single finger). The host decides what it means —
/// canvas node select/drag, marquee, panel press, text-edit caret…
/// Stamped with the session's current clock. Prefer
/// [`op_editor_press_at`] when the shell already has the event's own
/// monotonic timestamp.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_press(engine: *mut crate::OpEngine, x: f32, y: f32) -> OpStatus {
    unsafe {
        crate::lifecycle::call_session(engine, |session| {
            editor_press(session, x, y, session.now_ms)
        })
    }
}

/// Pointer press with the event's factual monotonic timestamp
/// (`time_ms`). The session/global clocks advance monotonically to
/// `time_ms` first (never backward); the preview runtime still receives
/// the event's own `time_ms`.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_press_at(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
    time_ms: u64,
) -> OpStatus {
    unsafe {
        crate::lifecycle::call_session(engine, |session| editor_press(session, x, y, time_ms))
    }
}

fn editor_press(session: &mut Session, x: f32, y: f32, time_ms: u64) -> FfiResult<()> {
    // Global clocks first: every early return below (safe-area miss,
    // pointer-capture miss, collab-history suppression) must still advance the
    // session/host/preview clocks monotonically to the event's timestamp.
    session.advance_global_clock(time_ms);
    if !session.safe_area_contains_surface_point(x, y) {
        if session.clear_editor_presence_pointer() {
            session.request_redraw();
        }
        return Ok(());
    }
    let (w, h) = session.editor_viewport();
    let (editor_x, editor_y) = session.editor_point(x, y);
    let presence_changed = session.set_editor_presence_pointer(editor_x, editor_y);
    if let Some(action) = session.collab_history_at(editor_x, editor_y, w, h) {
        if session.request_collab_history(action)? || presence_changed {
            session.request_redraw();
        }
        return Ok(());
    }
    if !session.begin_editor_pointer_capture(x, y) {
        if presence_changed {
            session.request_redraw();
        }
        return Ok(());
    }
    session.begin_collab_pointer_edit();
    let changed = session
        .editor_mut()?
        .apply_press_at(editor_x, editor_y, w, h, time_ms);
    if changed || presence_changed {
        session.request_redraw();
    }
    crate::editor_template::drain_pending_scene_template(session)?;
    Ok(())
}

/// Pointer move (single finger). Stamped with the session's current clock;
/// prefer [`op_editor_move_at`] when the shell has the event timestamp.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_move(engine: *mut crate::OpEngine, x: f32, y: f32) -> OpStatus {
    unsafe {
        crate::lifecycle::call_session(engine, |session| editor_move(session, x, y, session.now_ms))
    }
}

/// Pointer move with the event's factual monotonic timestamp (`time_ms`).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_move_at(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
    time_ms: u64,
) -> OpStatus {
    unsafe { crate::lifecycle::call_session(engine, |session| editor_move(session, x, y, time_ms)) }
}

fn editor_move(session: &mut Session, x: f32, y: f32, time_ms: u64) -> FfiResult<()> {
    session.advance_global_clock(time_ms);
    if !session.editor_pointer_captured() {
        return Ok(());
    }
    let (x, y) = session.editor_point(x, y);
    let presence_changed = session.set_editor_presence_pointer(x, y);
    let (changed, camera_changed) = {
        let host = session.editor_mut()?;
        let before = host.editor_state().viewport;
        let changed = host.apply_cursor_move_at(x, y, time_ms);
        (changed, host.editor_state().viewport != before)
    };
    if camera_changed {
        session.user_interacted = true;
    }
    if changed || presence_changed {
        session.request_redraw();
    }
    Ok(())
}

/// Pointer release (single finger). Stamped with the session's current clock;
/// prefer [`op_editor_release_at`] when the shell has the event timestamp.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_release(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
) -> OpStatus {
    unsafe {
        crate::lifecycle::call_session(engine, |session| {
            editor_release(session, x, y, session.now_ms)
        })
    }
}

/// Pointer release with the gesture endpoint's factual monotonic timestamp
/// (`time_ms`).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_release_at(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
    time_ms: u64,
) -> OpStatus {
    unsafe {
        crate::lifecycle::call_session(engine, |session| editor_release(session, x, y, time_ms))
    }
}

fn editor_release(session: &mut Session, x: f32, y: f32, time_ms: u64) -> FfiResult<()> {
    session.advance_global_clock(time_ms);
    let presence_changed = session.clear_editor_presence_pointer();
    if !session.end_editor_pointer_capture() {
        if presence_changed {
            session.request_redraw();
        }
        return Ok(());
    }
    let (x, y) = session.editor_point(x, y);
    let release = crate::editor_pointer_release::release_at(session, x, y, time_ms);
    let template = if release.is_ok() {
        crate::editor_template::drain_pending_scene_template(session)
    } else {
        Ok(false)
    };
    let collab_changed = session.finish_collab_pointer_edit();
    let changed = release? | template? | collab_changed | presence_changed;
    if changed {
        session.request_redraw();
    }
    Ok(())
}

/// Cancel the active editor pointer gesture without dispatching a release or
/// committing any release-delayed action. Stamped with the session's current
/// clock; prefer [`op_editor_cancel_gesture_at`] when the shell has the
/// event timestamp.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_cancel_gesture(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        crate::lifecycle::call_session(engine, |session| {
            editor_cancel_gesture(session, session.now_ms)
        })
    }
}

/// Cancel the active editor pointer gesture with the platform cancel's
/// monotonic timestamp (`time_ms`). The clocks advance monotonically first;
/// the runtime Cancel dispatch carries `time_ms`.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_cancel_gesture_at(
    engine: *mut crate::OpEngine,
    time_ms: u64,
) -> OpStatus {
    unsafe {
        crate::lifecycle::call_session(engine, |session| editor_cancel_gesture(session, time_ms))
    }
}

fn editor_cancel_gesture(session: &mut Session, time_ms: u64) -> FfiResult<()> {
    session.advance_global_clock(time_ms);
    let changed = session.cancel_editor_collab_gesture_at(Some(time_ms))?;
    session.reset_editor_pointer_capture();
    if changed {
        session.request_redraw();
    }
    Ok(())
}
