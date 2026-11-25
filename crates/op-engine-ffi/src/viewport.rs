//! Viewport channels — safe-area insets, keyboard occlusion, and the
//! logical → physical size query. The viewer stores these; shells keep
//! sending them so the ABI stays complete for interactive documents.

use crate::lifecycle::call_session;
use crate::{OpEngine, OpStatus};
use op_editor_ui::Rect;

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

/// Rectangle of the noninteractive fold preview in surface-logical points.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OpFoldPreviewRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl OpFoldPreviewRect {
    /// Shells derive the preview and the insets from shared edges in double
    /// precision, so after the f32 conversion an edge can land a rounding
    /// error past the surface or into the editor viewport. Errors within
    /// half a device pixel (capped at half a point) are clamped onto the
    /// surface and tolerated at the editor edge instead of failing the
    /// whole resize.
    fn checked(
        self,
        width: f32,
        height: f32,
        dpr: f32,
        insets: OpInsets,
    ) -> crate::error::FfiResult<Rect> {
        let Self {
            x,
            y,
            width: w,
            height: h,
        } = self;
        // `max` also maps a NaN dpr to 1; the resize rejects it afterwards.
        let slack = 0.5 / dpr.max(1.0);
        let fits = [x, y, w, h].into_iter().all(f32::is_finite)
            && x >= -slack
            && y >= -slack
            && x + w <= width + slack
            && y + h <= height + slack;
        let (left, top) = (x.max(0.0), y.max(0.0));
        let (right, bottom) = ((x + w).min(width), (y + h).min(height));
        let overlaps = right > insets.left + slack
            && left < width - insets.right - slack
            && bottom > insets.top + slack
            && top < height - insets.bottom - slack;
        if !fits || right <= left || bottom <= top || overlaps {
            return Err(crate::error::FfiError::invalid(
                "fold preview must fit the surface outside the editor viewport",
            ));
        }
        Ok(Rect::xywh(left, top, right - left, bottom - top))
    }
}

/// Resize atomically with an optional read-only fold preview rectangle.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. `preview` must
/// be null or point to a readable `OpFoldPreviewRect` for this call.
#[no_mangle]
pub unsafe extern "C" fn op_resize_with_fold_preview(
    engine: *mut OpEngine,
    width: f32,
    height: f32,
    dpr: f32,
    insets: OpInsets,
    preview: *const OpFoldPreviewRect,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let next = preview
                .as_ref()
                .map(|rect| rect.checked(width, height, dpr, insets))
                .transpose()?;
            session.resize_with_safe_area(width, height, dpr, insets)?;
            session.fold_preview = next;
            #[cfg(feature = "editor")]
            configure_touch_dock(session, next.is_none());
            session.request_redraw();
            Ok(())
        })
    }
}

#[cfg(feature = "editor")]
fn configure_touch_dock(session: &mut crate::lifecycle::Session, single_pane: bool) {
    use op_editor_ui::widgets::mobile_chrome::TOUCH_TARGET;

    let (width, height) = session.editor_viewport();
    let side_status_area =
        session.insets.right >= TOUCH_TARGET && session.insets.left < session.insets.right * 0.5;
    let Some(host) = session.editor.as_mut() else {
        return;
    };
    let ui = &mut host.editor_state_mut().editor_ui;
    let right =
        single_pane && ui.compact_layout() && side_status_area && width < height && height >= 340.0;
    if ui.native_touch_dock != right {
        ui.native_touch_dock = right;
        host.mark_editor_state_dirty();
        if !session.user_interacted {
            host.fit_content_to_viewport(width, height);
        }
    }
}

/// Return the active toolbar slot and selected shape row. Slot -1 hides the toolbar.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. The two output
/// pointers must be writable.
#[cfg(feature = "editor")]
#[no_mangle]
pub unsafe extern "C" fn op_editor_native_dock_state(
    engine: *mut OpEngine,
    slot: *mut i32,
    shape: *mut u32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let (Some(slot), Some(shape)) = (slot.as_mut(), shape.as_mut()) else {
                return Err(crate::error::FfiError::invalid(
                    "native dock state requires an output",
                ));
            };
            let Some(host) = session
                .editor()
                .filter(|host| host.native_touch_dock_visible())
            else {
                *slot = -1;
                return Ok(());
            };
            use op_editor_core::Tool;
            let state = host.editor_state();
            *shape = match state.editor_ui.shape_tool {
                Tool::Ellipse => 1,
                Tool::Polygon => 2,
                Tool::Line => 3,
                _ => 0,
            };
            *slot = if state.editor_ui.shape_picker.open {
                1
            } else {
                match state.tool {
                    Tool::Select => 0,
                    Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line => 1,
                    Tool::Pen => 2,
                    Tool::Text => 3,
                    _ => 5,
                }
            };
            Ok(())
        })
    }
}

/// Dispatch a standard dock action raised by the iOS toolbar.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[cfg(feature = "editor")]
#[no_mangle]
pub unsafe extern "C" fn op_editor_native_dock_press(
    engine: *mut OpEngine,
    index: u32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session
                .editor_mut()?
                .press_native_touch_dock(index as usize)
            {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Run a shape action selected from UIKit's menu.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[cfg(feature = "editor")]
#[no_mangle]
pub unsafe extern "C" fn op_editor_native_shape_press(engine: *mut OpEngine, row: u32) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.editor_mut()?.press_native_shape_menu(row as usize) {
                session.request_redraw();
            }
            Ok(())
        })
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

    #[test]
    fn fold_preview_updates_with_the_viewport_and_rejects_overlap() {
        let mut engine = editor_engine();
        let engine_ptr = &mut engine as *mut OpEngine;
        let insets = OpInsets {
            top: 320.0,
            ..OpInsets::default()
        };
        let preview = OpFoldPreviewRect {
            x: 0.0,
            y: 20.0,
            width: 800.0,
            height: 280.0,
        };
        assert_eq!(
            unsafe { op_resize_with_fold_preview(engine_ptr, 800.0, 640.0, 1.0, insets, &preview) },
            OpStatus::Ok
        );
        let session = engine.session_mut_for_test();
        assert_eq!(session.editor_viewport(), (800.0, 320.0));
        let previous = session.fold_preview;
        assert_eq!(previous, Some(Rect::xywh(0.0, 20.0, 800.0, 280.0)));

        let overlapping = OpFoldPreviewRect {
            y: 100.0,
            ..preview
        };
        assert_eq!(
            unsafe {
                op_resize_with_fold_preview(engine_ptr, 800.0, 700.0, 1.0, insets, &overlapping)
            },
            OpStatus::InvalidArg
        );
        let session = engine.session_mut_for_test();
        assert_eq!(session.logical, (800.0, 640.0));
        assert_eq!(session.fold_preview, previous);

        assert_eq!(
            unsafe {
                op_resize_with_fold_preview(
                    engine_ptr,
                    800.0,
                    640.0,
                    1.0,
                    OpInsets::default(),
                    std::ptr::null(),
                )
            },
            OpStatus::Ok
        );
        assert!(engine.session_mut_for_test().fold_preview.is_none());
    }

    #[test]
    fn fold_preview_clamps_float_rounding_of_shared_edges() {
        let mut engine = editor_engine();
        let engine_ptr = &mut engine as *mut OpEngine;
        // Side-by-side halves sharing the hinge edge, as a shell converting
        // double-precision geometry to f32 can send them: one ulp past the
        // editor's left edge and one ulp past the surface's bottom.
        let insets = OpInsets {
            left: 400.0,
            ..OpInsets::default()
        };
        let preview = OpFoldPreviewRect {
            x: 0.0,
            y: 0.0,
            width: 400.0_f32.next_up(),
            height: 640.0_f32.next_up(),
        };
        assert_eq!(
            unsafe { op_resize_with_fold_preview(engine_ptr, 800.0, 640.0, 3.0, insets, &preview) },
            OpStatus::Ok
        );
        let session = engine.session_mut_for_test();
        assert_eq!(session.logical, (800.0, 640.0));
        let accepted = session.fold_preview;
        assert_eq!(
            accepted,
            Some(Rect::xywh(0.0, 0.0, 400.0_f32.next_up(), 640.0))
        );

        // More than half a device pixel is still a real overlap.
        let overlapping = OpFoldPreviewRect {
            width: 400.2,
            ..preview
        };
        assert_eq!(
            unsafe {
                op_resize_with_fold_preview(engine_ptr, 800.0, 700.0, 3.0, insets, &overlapping)
            },
            OpStatus::InvalidArg
        );
        let session = engine.session_mut_for_test();
        assert_eq!(session.logical, (800.0, 640.0));
        assert_eq!(session.fold_preview, accepted);
    }

    #[test]
    fn portrait_side_status_area_enables_the_native_dock() {
        let mut engine = editor_engine();
        let ptr = &mut engine as *mut OpEngine;
        let side = OpInsets {
            right: 84.0,
            bottom: 34.0,
            ..OpInsets::default()
        };
        for (width, height, insets, expected) in [
            (466.0, 678.0, side, true),
            (678.0, 466.0, side, false),
            (466.0, 678.0, OpInsets::default(), false),
        ] {
            assert_eq!(
                unsafe {
                    op_resize_with_fold_preview(ptr, width, height, 1.0, insets, std::ptr::null())
                },
                OpStatus::Ok
            );
            let session = engine.session_mut_for_test();
            assert_eq!(
                session
                    .editor()
                    .unwrap()
                    .editor_state()
                    .editor_ui
                    .native_touch_dock,
                expected
            );
        }
    }

    #[test]
    fn native_dock_routes_tools_shape_picker_and_more() {
        let mut engine = editor_engine();
        let ptr = &mut engine as *mut OpEngine;
        let insets = OpInsets {
            right: 84.0,
            ..OpInsets::default()
        };
        assert_eq!(
            unsafe {
                op_resize_with_fold_preview(ptr, 466.0, 678.0, 1.0, insets, std::ptr::null())
            },
            OpStatus::Ok
        );
        let mut slot = -1;
        let mut shape = 0;
        assert_eq!(
            unsafe { op_editor_native_dock_state(ptr, &mut slot, &mut shape) },
            OpStatus::Ok
        );
        assert_eq!(slot, 0);
        for (index, tool) in [
            (2, op_editor_core::Tool::Pen),
            (3, op_editor_core::Tool::Text),
            (0, op_editor_core::Tool::Select),
        ] {
            assert_eq!(
                unsafe { op_editor_native_dock_press(ptr, index) },
                OpStatus::Ok
            );
            assert_eq!(
                engine
                    .session_mut_for_test()
                    .editor()
                    .unwrap()
                    .editor_state()
                    .tool,
                tool
            );
        }
        assert_eq!(
            unsafe { op_editor_native_shape_press(ptr, 2) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { op_editor_native_dock_state(ptr, &mut slot, &mut shape) },
            OpStatus::Ok
        );
        assert_eq!((slot, shape), (1, 2));
        assert_eq!(unsafe { op_editor_native_dock_press(ptr, 4) }, OpStatus::Ok);
        assert_eq!(
            unsafe { op_editor_native_dock_state(ptr, &mut slot, &mut shape) },
            OpStatus::Ok
        );
        assert_eq!(slot, -1);
        assert_eq!(
            engine
                .session_mut_for_test()
                .editor()
                .unwrap()
                .editor_state()
                .editor_ui
                .mobile_sheet,
            Some(op_editor_core::size_class::MobileSheetKind::More)
        );
    }

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
