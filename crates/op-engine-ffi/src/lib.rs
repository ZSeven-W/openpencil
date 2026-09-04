//! C ABI boundary for native OpenPencil player shells (iOS / Android).
//!
//! The engine renders a canonical `.op` document with the exact painter
//! the desktop editor canvas uses ([`op_editor_ui::widgets::canvas_viewport_paint`]
//! over [`op_host_native::NativeFrameBackend`]), so a phone shows the
//! same pixels as the desktop. The shells stay thin: they own the
//! platform surface (a `CAMetalLayer` on iOS, an `ANativeWindow` on
//! Android), pump frames on their display/Choreographer clock, forward
//! raw touches, and react to the callback table.
//!
//! ## Threading contract
//!
//! An engine is owned by exactly one thread (the thread that called
//! [`op_create`]). Every other call must happen on that thread;
//! [`OpStatus::WrongThread`] is returned otherwise. Most callbacks fire
//! synchronously inside the call that caused them; collaboration workers may
//! also invoke redraw and secure-store callbacks. Shells copy payloads and
//! react asynchronously, and callbacks must never re-enter the engine.
//!
//! ## Coordinates
//!
//! Pointer input uses surface-logical points with a top-left origin
//! (never multiplied by `dpr` — the drawable is scaled, the pointer
//! stream is not). Safe-area and keyboard occlusion are separate
//! logical-point channels.
//!
//! ## Versioning
//!
//! `OpCreateDesc.size` / `OpCallbacks.size` / `OpSurfaceDesc.size` are
//! tail-growable: missing trailing fields read as zero/null; a size
//! larger than the current `sizeof` is invalid.

mod background;
mod desc;
#[cfg(feature = "editor")]
mod editor;
#[cfg(feature = "editor")]
mod editor_auth;
#[cfg(all(feature = "editor", test))]
mod editor_auth_window_tests;
#[cfg(feature = "editor")]
mod editor_builtin_provider;
#[cfg(feature = "editor")]
mod editor_chat;
#[cfg(feature = "editor")]
mod editor_chat_design;
#[cfg(feature = "editor")]
mod editor_chat_design_tools;
#[cfg(feature = "editor")]
mod editor_chat_turn;
#[cfg(feature = "editor")]
mod editor_codegen;
#[cfg(feature = "editor")]
mod editor_collab;
#[cfg(feature = "editor")]
mod editor_collab_presence;
#[cfg(feature = "editor")]
mod editor_document;
#[cfg(all(feature = "editor", test))]
mod editor_document_save_tests;
#[cfg(feature = "editor")]
mod editor_document_shell;
#[cfg(all(feature = "editor", test))]
mod editor_document_shell_tests;
#[cfg(feature = "editor")]
mod editor_export;
#[cfg(feature = "editor")]
mod editor_image_import;
#[cfg(feature = "editor")]
mod editor_image_search;
#[cfg(feature = "editor")]
mod editor_ime;
#[cfg(feature = "editor")]
mod editor_model_discovery;
#[cfg(feature = "editor")]
mod editor_pointer;
#[cfg(all(feature = "editor", test))]
mod editor_pointer_clock_tests;
#[cfg(feature = "editor")]
mod editor_pointer_release;
#[cfg(all(test, feature = "editor"))]
mod editor_presence_tests;
#[cfg(all(feature = "editor", test))]
mod editor_settings_input_tests;
#[cfg(feature = "editor")]
mod editor_template;
#[cfg(feature = "editor")]
mod editor_transform;
mod error;
mod input;
mod lifecycle;
mod lifecycle_initial_document;
mod measure;
mod media;
mod pages;
mod pointer_gate;
mod render;
mod system_chrome;
mod text;
mod viewport;

pub use background::{op_background_tick, op_cancel_background_work, op_has_background_work};
pub use desc::{
    OpCallbacks, OpCreateDesc, OpCredentialLoad, OpCredentialStoreIfAbsent, OpPointerPhase,
    OpRuntimeError, OpSurfaceDesc, OpTextState,
};
#[cfg(feature = "editor")]
pub use editor::{
    op_editor_key, op_editor_locale_code, op_editor_open_document, op_editor_pan, op_editor_pinch,
    op_editor_right_press, op_editor_set_locale, op_editor_take_shell_action, op_editor_text,
    KEY_ARROW_DOWN, KEY_ARROW_LEFT, KEY_ARROW_RIGHT, KEY_ARROW_UP, KEY_BACKSPACE, KEY_DELETE,
    KEY_DUPLICATE, KEY_ENTER, KEY_ESCAPE, KEY_REDO, KEY_UNDO,
};
#[cfg(feature = "editor")]
pub use editor_auth::{
    op_editor_account_snapshot, op_editor_auth_sign_out, op_editor_begin_login,
    op_editor_cancel_login, op_editor_configure_auth, op_editor_copy_login_url, AUTH_REGION_CHINA,
    AUTH_REGION_GLOBAL, SHELL_ACTION_CLOSE_LOGIN_WEBVIEW, SHELL_ACTION_NONE,
    SHELL_ACTION_OPEN_ACCOUNT_CENTER, SHELL_ACTION_OPEN_DOCUMENT,
    SHELL_ACTION_OPEN_LANGUAGE_PICKER, SHELL_ACTION_OPEN_LOGIN_WEBVIEW, SHELL_ACTION_REQUEST_LOGIN,
    SHELL_ACTION_WINDOW_CLOSE, SHELL_ACTION_WINDOW_MINIMIZE, SHELL_ACTION_WINDOW_ZOOM,
};
#[cfg(feature = "editor")]
pub use editor_document_shell::{
    op_editor_cancel_save, op_editor_commit_save, op_editor_configure_save_picker,
    op_editor_copy_save_file_name, op_editor_copy_save_target, op_editor_stage_save_to_path,
    SHELL_ACTION_SAVE_DOCUMENT,
};
#[cfg(feature = "editor")]
pub use editor_export::{
    op_editor_cancel_export, op_editor_copy_export_file_name, op_editor_export_to_path,
    SHELL_ACTION_EXPORT_DOCUMENT,
};
#[cfg(feature = "editor")]
pub use editor_image_import::{op_editor_import_image_or_svg, SHELL_ACTION_IMPORT_IMAGE_OR_SVG};
#[cfg(feature = "editor")]
pub use editor_ime::{
    op_editor_ime_commit, op_editor_ime_focused, op_editor_ime_preedit, op_editor_paste_text,
    op_editor_take_copy_text,
};
#[cfg(feature = "editor")]
pub use editor_pointer::{
    op_editor_cancel_gesture, op_editor_cancel_gesture_at, op_editor_move, op_editor_move_at,
    op_editor_press, op_editor_press_at, op_editor_release, op_editor_release_at,
};
#[cfg(feature = "editor")]
pub use editor_transform::{op_editor_begin_transform, op_editor_hover, op_editor_wheel};
pub use lifecycle::OpEngine;
pub use media::{op_register_font, op_remote_image_result};
pub use pages::{op_get_page_count, op_set_active_page};
#[cfg(feature = "editor")]
pub use system_chrome::op_editor_set_touch_chrome;
pub use system_chrome::op_prefers_light_system_icons;
pub use text::{
    op_ime_cancel_composition, op_ime_commit_composition, op_ime_set_composing_text,
    op_text_backspace, op_text_begin, op_text_caret_rect, op_text_delete_forward, op_text_end,
    op_text_get_state, op_text_insert, op_text_select_range, op_text_set_caret,
};
pub use viewport::{op_get_pixel_size, op_set_keyboard, op_set_safe_area, OpInsets};

use desc::{parse_create, surface_handle};
use error::{clear_create_error, create_error, set_create_error, write_bytes, FfiError, FfiResult};
use lifecycle::{call_session, destroy_engine, Session};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

/// Stable status values returned by every OpenPencil C ABI call.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpStatus {
    Ok = 0,
    InvalidArg = 1,
    BadDocument = 2,
    LayoutError = 3,
    GpuError = 4,
    OutOfMemory = 5,
    WrongThread = 6,
    Suspended = 7,
    Busy = 8,
    NoFocus = 9,
    NotReady = 10,
    Poisoned = 11,
}

/// Create an engine from a versioned descriptor.
///
/// # Safety
///
/// `desc` must expose its declared prefix and `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_create(desc: *const OpCreateDesc, out: *mut *mut OpEngine) -> OpStatus {
    clear_create_error();
    if out.is_null() {
        set_create_error("engine output pointer is null");
        return OpStatus::InvalidArg;
    }
    unsafe { out.write(ptr::null_mut()) };
    let outcome = catch_unwind(AssertUnwindSafe(|| -> FfiResult<*mut OpEngine> {
        let options = unsafe { parse_create(desc) }?;
        let session = Session::new(options)?;
        Ok(Box::into_raw(Box::new(OpEngine::new(session))))
    }));
    match outcome {
        Ok(Ok(engine)) => {
            unsafe { out.write(engine) };
            OpStatus::Ok
        }
        Ok(Err(error)) => {
            set_create_error(error.message);
            error.status
        }
        Err(_) => {
            set_create_error("panic while creating the OpenPencil engine");
            OpStatus::Poisoned
        }
    }
}

/// Destroy an engine on its owner thread.
///
/// # Safety
///
/// `engine` must be live, returned by `op_create`, and not yet destroyed.
#[no_mangle]
pub unsafe extern "C" fn op_destroy(engine: *mut OpEngine) -> OpStatus {
    unsafe { destroy_engine(engine) }
}

/// Copy the last error into a caller-owned byte buffer.
///
/// # Safety
///
/// A non-null `engine` must be live; output pointers must cover their sizes.
#[no_mangle]
pub unsafe extern "C" fn op_last_error(
    engine: *mut OpEngine,
    buffer: *mut u8,
    length: usize,
    required: *mut usize,
) -> OpStatus {
    if engine.is_null() {
        return match catch_unwind(AssertUnwindSafe(|| unsafe {
            write_bytes(create_error().as_bytes(), buffer, length, required)
        })) {
            Ok(Ok(())) => OpStatus::Ok,
            Ok(Err(error)) => {
                set_create_error(error.message);
                error.status
            }
            Err(_) => OpStatus::Poisoned,
        };
    }
    let engine = unsafe { &*engine };
    let message = engine.last_error();
    match unsafe { write_bytes(message.as_bytes(), buffer, length, required) } {
        Ok(()) => OpStatus::Ok,
        Err(error) => error.status,
    }
}

/// Select GPU mode using a borrowed platform surface.
///
/// # Safety
///
/// Pointers must be live and the surface must outlive attach through suspend.
#[no_mangle]
pub unsafe extern "C" fn op_attach_surface(
    engine: *mut OpEngine,
    desc: *const OpSurfaceDesc,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let handle = surface_handle(desc)?;
            session.attach_surface(handle)?;
            session.request_redraw();
            Ok(())
        })
    }
}

/// Suspend rendering and synchronously stop using a borrowed surface.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_suspend(engine: *mut OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            session.suspend();
            Ok(())
        })
    }
}

/// Resume rendering, optionally with a new borrowed GPU surface.
///
/// # Safety
///
/// `engine` must be live; a non-null descriptor must be readable.
#[no_mangle]
pub unsafe extern "C" fn op_resume(engine: *mut OpEngine, desc: *const OpSurfaceDesc) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let handle = if desc.is_null() {
                None
            } else {
                Some(surface_handle(desc)?)
            };
            session.resume(handle)?;
            session.request_redraw();
            Ok(())
        })
    }
}

/// Pump and present one GPU frame.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_frame(engine: *mut OpEngine, now_ms: u64) -> OpStatus {
    unsafe { call_session(engine, |session| session.frame_gpu(now_ms)) }
}

/// Pump and copy one CPU frame into caller-owned RGBA8888 storage.
///
/// # Safety
///
/// `engine` must be live and `buffer` must cover `buffer_len` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_frame_cpu(
    engine: *mut OpEngine,
    now_ms: u64,
    buffer: *mut u8,
    buffer_len: usize,
    stride: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            session.frame_cpu(now_ms, buffer, buffer_len, stride)
        })
    }
}

/// Change the logical viewport and device-pixel ratio.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_resize(
    engine: *mut OpEngine,
    width: f32,
    height: f32,
    dpr: f32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            session.resize(width, height, dpr)?;
            session.request_redraw();
            Ok(())
        })
    }
}

/// Atomically update logical viewport, DPR, and four safe-area insets.
///
/// Mobile shells should use this during rotation/configuration changes so an
/// intermediate size/inset pair cannot trigger a transient responsive class.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_resize_with_safe_area(
    engine: *mut OpEngine,
    width: f32,
    height: f32,
    dpr: f32,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            session.resize_with_safe_area(
                width,
                height,
                dpr,
                viewport::OpInsets {
                    top,
                    right,
                    bottom,
                    left,
                },
            )?;
            session.request_redraw();
            Ok(())
        })
    }
}

/// Deliver one pointer event (surface-logical coordinates).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_pointer(
    engine: *mut OpEngine,
    id: u32,
    phase: i32,
    x: f32,
    y: f32,
    time_ms: u64,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.suspended {
                return Err(FfiError::new(OpStatus::Suspended, "engine is suspended"));
            }
            let phase = match phase {
                0 => OpPointerPhase::Down,
                1 => OpPointerPhase::Move,
                2 => OpPointerPhase::Up,
                3 => OpPointerPhase::Cancel,
                _ => return Err(FfiError::invalid("unknown pointer phase")),
            };
            // A pointer event between frames carries its factual
            // monotonic timestamp. Advance the global clocks (session +
            // editor host + live preview runtime) MONOTONICALLY before
            // EVERY editor pointer route below — the safe-area gate, the
            // collab-suppressed early return, and Cancel included — or
            // velocity-sensing recognizers (Swipe) and the
            // screen-transition gate read the last frame pump's stale
            // time. The event's own timestamp then travels separately
            // through the timestamped host entries below, so a backward
            // event never regresses a clock while still measuring its
            // factual delta.
            session.advance_global_clock(time_ms);
            let (x, y) = session.safe_area_point(x, y);
            if !session.route_safe_area_pointer(id, phase, x, y) {
                return Ok(());
            }
            // Editor mode: the host owns all interaction — press/move/
            // release map 1:1 to pointer phases.
            #[cfg(feature = "editor")]
            if session.editor.is_some() {
                let (w, h) = session.editor_viewport();
                let presence_changed = session.update_editor_presence_pointer(id, phase, x, y);
                if phase != OpPointerPhase::Down
                    && session.take_suppressed_collab_pointer(
                        id,
                        matches!(phase, OpPointerPhase::Up | OpPointerPhase::Cancel),
                    )
                {
                    if presence_changed {
                        session.request_redraw();
                    }
                    return Ok(());
                }
                if phase == OpPointerPhase::Down {
                    if let Some(action) = session.collab_history_at(x, y, w, h) {
                        if session.request_collab_history(action)? || presence_changed {
                            session.request_redraw();
                        }
                        session.suppress_collab_pointer(id);
                        return Ok(());
                    }
                    session.begin_collab_pointer_edit();
                }
                let (changed, camera_changed) = match phase {
                    OpPointerPhase::Down => (
                        session.editor_mut()?.apply_press_at(x, y, w, h, time_ms),
                        false,
                    ),
                    OpPointerPhase::Move => {
                        let host = session.editor_mut()?;
                        let before = host.editor_state().viewport;
                        let changed = host.apply_cursor_move_at(x, y, time_ms);
                        (changed, host.editor_state().viewport != before)
                    }
                    OpPointerPhase::Up => {
                        let release =
                            crate::editor_pointer_release::release_at(session, x, y, time_ms);
                        let template = if release.is_ok() {
                            crate::editor_template::drain_pending_scene_template(session)
                        } else {
                            Ok(false)
                        };
                        let collab_changed = session.finish_collab_pointer_edit();
                        (release? | template? | collab_changed, false)
                    }
                    OpPointerPhase::Cancel => (
                        session.cancel_editor_collab_gesture_at(Some(time_ms))?,
                        false,
                    ),
                };
                if camera_changed {
                    session.user_interacted = true;
                }
                let template_changed = if phase == OpPointerPhase::Up {
                    false
                } else {
                    crate::editor_template::drain_pending_scene_template(session)?
                };
                if changed || template_changed || presence_changed {
                    session.request_redraw();
                }
                return Ok(());
            }
            let changed = crate::input::handle_pointer(session, id, phase, x, y)?;
            if changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_discriminants_are_stable() {
        assert_eq!(OpStatus::Ok as i32, 0);
        assert_eq!(OpStatus::InvalidArg as i32, 1);
        assert_eq!(OpStatus::BadDocument as i32, 2);
        assert_eq!(OpStatus::LayoutError as i32, 3);
        assert_eq!(OpStatus::GpuError as i32, 4);
        assert_eq!(OpStatus::WrongThread as i32, 6);
        assert_eq!(OpStatus::Suspended as i32, 7);
        assert_eq!(OpStatus::NoFocus as i32, 9);
        assert_eq!(OpStatus::Poisoned as i32, 11);
    }

    #[test]
    fn pointer_phase_matches_header_contract() {
        assert_eq!(OpPointerPhase::Down as i32, 0);
        assert_eq!(OpPointerPhase::Move as i32, 1);
        assert_eq!(OpPointerPhase::Up as i32, 2);
        assert_eq!(OpPointerPhase::Cancel as i32, 3);
    }

    #[test]
    fn private_storage_root_is_configured_before_session_construction() {
        let create = include_str!("lib.rs");
        let parse = create
            .find("let options = unsafe { parse_create(desc) }")
            .unwrap();
        let session = create.find("let session = Session::new(options)").unwrap();
        assert!(parse < session);

        let descriptor = include_str!("desc.rs");
        let storage = descriptor
            .find("let storage_root = if storage_root_len")
            .unwrap();
        let configure = descriptor
            .find("op_config_store::configure_user_root(path)")
            .unwrap();
        assert!(storage < configure);
    }
}
