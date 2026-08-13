//! Viewport channels — safe-area insets, keyboard occlusion, and the
//! logical → physical size query. The viewer stores these; shells keep
//! sending them so the ABI stays complete for interactive documents.

use crate::lifecycle::call_session;
use crate::{OpEngine, OpStatus};

/// Logical safe-area insets (top-left origin, logical points).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OpInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Update the four logical safe-area insets.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_set_safe_area(
    engine: *mut OpEngine,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let next = OpInsets {
                top,
                right,
                bottom,
                left,
            };
            if session.insets == next {
                return Ok(());
            }
            session.insets = next;
            session.recompute_responsive_layout();
            if !session.user_interacted {
                // Safe-area delivery normally follows `op_create`; fit
                // again now that the editor's actual usable canvas is known.
                session.fit_content_to_viewports();
            }
            session.request_redraw();
            Ok(())
        })
    }
}

/// Update the logical keyboard occlusion height.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_set_keyboard(engine: *mut OpEngine, height: f32) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let next = if height.is_finite() && height > 0.0 {
                height
            } else {
                0.0
            };
            if (session.keyboard - next).abs() <= f32::EPSILON {
                return Ok(());
            }
            session.keyboard = next;
            if !session.user_interacted {
                session.fit_content_to_viewports();
            }
            session.request_redraw();
            Ok(())
        })
    }
}

/// Return the current physical pixel dimensions.
///
/// # Safety
///
/// `engine` must be live and both output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_get_pixel_size(
    engine: *mut OpEngine,
    width: *mut u32,
    height: *mut u32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if width.is_null() || height.is_null() {
                return Err(crate::error::FfiError::invalid(
                    "pixel-size output pointer is null",
                ));
            }
            let (logical_w, logical_h) = session.logical;
            width.write((logical_w * session.dpr).round() as u32);
            height.write((logical_h * session.dpr).round() as u32);
            Ok(())
        })
    }
}
