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

impl OpInsets {
    pub(crate) fn is_valid(self) -> bool {
        [self.top, self.right, self.bottom, self.left]
            .into_iter()
            .all(|value| value.is_finite() && value >= 0.0)
    }
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
            if !next.is_valid() {
                return Err(crate::error::FfiError::invalid(
                    "safe-area insets must be finite and non-negative",
                ));
            }
            if session.insets == next {
                return Ok(());
            }
            session.insets = next;
            session.recompute_responsive_layout();
            session.sync_editor_keyboard_occlusion();
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
            session.sync_editor_keyboard_occlusion();
            session.request_redraw();
            Ok(())
        })
    }
}

#[cfg(all(test, feature = "editor"))]
mod tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};
    use crate::lifecycle::Session;

    const SAMPLE_DOC: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

    fn editor_engine() -> OpEngine {
        OpEngine::new(
            Session::new(CreateOptions {
                document: SAMPLE_DOC.to_owned(),
                width: 800.0,
                height: 600.0,
                dpr: 1.0,
                callbacks: Callbacks::default(),
                asset_base: None,
                editor_mode: true,
                documents_root: None,
            })
            .expect("editor session"),
        )
    }

    #[test]
    fn keyboard_is_local_occlusion_and_does_not_refit_the_editor() {
        let mut engine = editor_engine();
        let engine_ptr = &mut engine as *mut OpEngine;
        assert_eq!(
            unsafe { op_set_safe_area(engine_ptr, 20.0, 10.0, 34.0, 10.0) },
            OpStatus::Ok
        );
        let session = engine.session_mut_for_test();
        let viewport = session.editor_viewport();
        let camera = session.editor().unwrap().editor_state().viewport;
        assert_eq!(viewport, (780.0, 546.0));

        assert_eq!(unsafe { op_set_keyboard(engine_ptr, 300.0) }, OpStatus::Ok);
        let session = engine.session_mut_for_test();
        let host = session.editor().unwrap();
        assert_eq!(session.editor_viewport(), viewport);
        assert_eq!(host.editor_state().viewport, camera);
        assert_eq!(host.keyboard_visible_bottom(viewport.1), 280.0);

        assert_eq!(
            unsafe { op_set_safe_area(engine_ptr, 20.0, 10.0, 50.0, 10.0) },
            OpStatus::Ok
        );
        let session = engine.session_mut_for_test();
        let viewport = session.editor_viewport();
        assert_eq!(viewport, (780.0, 530.0));
        assert_eq!(
            session
                .editor()
                .unwrap()
                .keyboard_visible_bottom(viewport.1),
            280.0,
            "the bottom safe area must not be counted twice"
        );

        assert_eq!(unsafe { op_set_keyboard(engine_ptr, 0.0) }, OpStatus::Ok);
        let session = engine.session_mut_for_test();
        assert_eq!(
            session
                .editor()
                .unwrap()
                .keyboard_visible_bottom(viewport.1),
            viewport.1
        );
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
