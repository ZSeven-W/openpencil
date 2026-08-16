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
