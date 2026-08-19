//! Explicit ownership seam for direct two-finger editor transforms.

use crate::lifecycle::call_session;
use crate::OpStatus;

/// Begin a pan/pinch stream at the second-finger Down midpoint.
///
/// Subsequent pan/pinch events keep this safe-area ownership even when the
/// midpoint crosses a platform band. `op_editor_cancel_gesture` ends it.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_begin_transform(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let _ = session.editor_mut()?;
            if session.cancel_editor_collab_gesture()? {
                session.request_redraw();
            }
            session.reset_editor_pointer_capture();
            session.begin_editor_transform(x, y);
            Ok(())
        })
    }
}

/// Desktop-chrome hover: cursor motion WITHOUT a pressed button. Unlike
/// [`op_editor_move`] this does not require pointer capture — it drives
/// hover highlighting exactly like the desktop binary's cursor-move path.
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_hover(engine: *mut crate::OpEngine, x: f32, y: f32) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if !session.safe_area_contains_surface_point(x, y) {
                return Ok(());
            }
            let (x, y) = session.editor_point(x, y);
            let changed = session.editor_mut()?.apply_cursor_move(x, y);
            if changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Desktop-chrome wheel/trackpad scroll at a cursor position. Routes through
/// the same panel-aware wheel logic as the desktop binary: over a side panel
/// it scrolls that panel, over the canvas it pans, and `zoom != 0` promotes
/// the vertical delta to a zoom around the cursor (Ctrl+wheel). Needs no
/// pointer or transform capture.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_wheel(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    zoom: i32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let (w, h) = session.editor_viewport();
            let (x, y) = session.editor_point(x, y);
            let (changed, camera_changed) = {
                let host = session.editor_mut()?;
                let before = host.editor_state().viewport;
                // Trackpad semantics (the desktop's PixelDelta path): panels
                // under the cursor scroll, the canvas pans, Ctrl zooms.
                let changed = if zoom != 0 {
                    host.apply_pinch_gesture(x, y, dy, w, h)
                } else {
                    host.apply_pan_gesture(x, y, dx, dy, w, h)
                };
                (changed, host.editor_state().viewport != before)
            };
            if camera_changed {
                session.user_interacted = true;
            }
            if changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}
