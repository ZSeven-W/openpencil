#![cfg(target_os = "android")]

//! Editor-mode natives — split out of `bindings.rs`.

use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jfloat, jint, jlong, jstring};
use jni::JNIEnv;

use op_engine_ffi::{
    op_editor_account_snapshot, op_editor_auth_sign_out, op_editor_begin_login,
    op_editor_cancel_export, op_editor_cancel_login, op_editor_cancel_save, op_editor_commit_save,
    op_editor_configure_auth, op_editor_configure_save_picker, op_editor_copy_export_file_name,
    op_editor_copy_login_url, op_editor_copy_save_file_name, op_editor_copy_save_target,
    op_editor_export_to_path, op_editor_locale_code, op_editor_open_document, op_editor_set_locale,
    op_editor_stage_save_to_path, op_editor_take_shell_action, OpStatus, SHELL_ACTION_NONE,
};

use crate::bindings::{call_status, jstring_bytes, with_engine};
use crate::engine_thread::STATUS_CLOSING;

/// `OpNative.nativeEditorConfigureAuth` — initialize the real mobile auth
/// backend with a private shell-owned storage directory and the regional SSO
/// deployment the shell resolved (an `OpAuthRegion` code).
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorConfigureAuth<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    storage_dir: JString<'local>,
    device_name: JString<'local>,
    app_version: JString<'local>,
    region: jint,
) -> jint {
    let Some(storage_dir) = jstring_bytes(&mut env, &storage_dir) else {
        return OpStatus::InvalidArg as jint;
    };
    let Some(device_name) = jstring_bytes(&mut env, &device_name) else {
        return OpStatus::InvalidArg as jint;
    };
    let Some(app_version) = jstring_bytes(&mut env, &app_version) else {
        return OpStatus::InvalidArg as jint;
    };
    call_status(engine, move |e| unsafe {
        op_editor_configure_auth(
            e,
            storage_dir.as_ptr(),
            storage_dir.len(),
            device_name.as_ptr(),
            device_name.len(),
            app_version.as_ptr(),
            app_version.len(),
            region,
        )
    })
}

/// `OpNative.nativeEditorAccountSnapshot` — copies the JSON account snapshot
/// for the native account center, or returns null when it cannot be read.
/// The snapshot is re-read per call and never consumed.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorAccountSnapshot<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jstring {
    let bytes = with_engine(engine, move |e| {
        let mut required = 0usize;
        let status =
            unsafe { op_editor_account_snapshot(e, std::ptr::null_mut(), 0, &mut required) };
        if status != OpStatus::Ok || required == 0 {
            return None;
        }
        let mut bytes = vec![0_u8; required];
        let status = unsafe {
            op_editor_account_snapshot(e, bytes.as_mut_ptr(), bytes.len(), &mut required)
        };
        (status == OpStatus::Ok).then_some(bytes)
    })
    .flatten();
    let Some(bytes) = bytes else {
        return std::ptr::null_mut();
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return std::ptr::null_mut();
    };
    env.new_string(text)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// `OpNative.nativeEditorSignOut` — revoke the device session and clear the
/// engine's account mirror (native account-center sign out).
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorSignOut<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe { op_editor_auth_sign_out(e) })
}

/// `OpNative.nativeEditorTakeLoginUrl` — atomically copies and consumes the
/// pending UTF-8 verification URL, or returns null when none is ready.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorTakeLoginUrl<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jstring {
    let bytes = with_engine(engine, move |e| {
        let mut required = 0usize;
        let status = unsafe { op_editor_copy_login_url(e, std::ptr::null_mut(), 0, &mut required) };
        if status != OpStatus::Ok || required == 0 {
            return None;
        }
        let mut bytes = vec![0_u8; required];
        let status =
            unsafe { op_editor_copy_login_url(e, bytes.as_mut_ptr(), bytes.len(), &mut required) };
        (status == OpStatus::Ok).then_some(bytes)
    })
    .flatten();
    let Some(bytes) = bytes else {
        return std::ptr::null_mut();
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return std::ptr::null_mut();
    };
    env.new_string(text)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// `OpNative.nativeEditorTakeCopyText` — atomically copies and consumes the
/// engine's pending copy-to-clipboard text (collab invite / share address,
/// MCP config, chat copy buttons), or returns null when none is pending.
/// The shell polls after each frame and writes the system clipboard.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorTakeCopyText<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jstring {
    let bytes = with_engine(engine, move |e| {
        let mut required = 0usize;
        let status = unsafe {
            op_engine_ffi::op_editor_take_copy_text(e, std::ptr::null_mut(), 0, &mut required)
        };
        if status != OpStatus::Ok || required == 0 {
            return None;
        }
        let mut bytes = vec![0_u8; required];
        let status = unsafe {
            op_engine_ffi::op_editor_take_copy_text(
                e,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut required,
            )
        };
        (status == OpStatus::Ok).then_some(bytes)
    })
    .flatten();
    let Some(bytes) = bytes else {
        return std::ptr::null_mut();
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return std::ptr::null_mut();
    };
    env.new_string(text)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// `OpNative.nativeEditorCancelLogin` — user-dismissed WebView cancellation.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorCancelLogin<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe { op_editor_cancel_login(e) })
}

/// `OpNative.nativeEditorExportFileName` — copies the frozen export's UTF-8
/// file name, or returns null when none is pending. Unlike the login-URL
/// bridge, reading the name does not consume the artifact; only a successful
/// staged write or an explicit cancel does.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorExportFileName<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jstring {
    let bytes = with_engine(engine, move |e| {
        let mut required = 0usize;
        let status =
            unsafe { op_editor_copy_export_file_name(e, std::ptr::null_mut(), 0, &mut required) };
        if status != OpStatus::Ok || required == 0 {
            return None;
        }
        let mut bytes = vec![0_u8; required];
        let status = unsafe {
            op_editor_copy_export_file_name(e, bytes.as_mut_ptr(), bytes.len(), &mut required)
        };
        (status == OpStatus::Ok).then_some(bytes)
    })
    .flatten();
    let Some(bytes) = bytes else {
        return std::ptr::null_mut();
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return std::ptr::null_mut();
    };
    env.new_string(text)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// `OpNative.nativeEditorExportToPath` — writes the frozen export into a new
/// app-private absolute staging file, consuming the artifact on success.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorExportToPath<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    path: JString<'local>,
) -> jint {
    let Some(path) = jstring_bytes(&mut env, &path) else {
        return OpStatus::InvalidArg as jint;
    };
    call_status(engine, move |e| unsafe {
        op_editor_export_to_path(e, path.as_ptr(), path.len())
    })
}

/// `OpNative.nativeEditorCancelExport` — discard the frozen export when the
/// platform save UI cannot run (or the user abandoned it).
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorCancelExport<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe { op_editor_cancel_export(e) })
}

// ---- Picker-backed Save / Save As ----------------------------------------
//
// The Kotlin side of this bridge is `MainActivity.beginDocumentSave`. See
// `op-engine-ffi/src/editor_document_shell.rs` for the round trip these five
// natives make up.

/// `OpNative.nativeEditorConfigureSavePicker` — declare that this shell
/// routes Save / Save As through `ACTION_CREATE_DOCUMENT`.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorConfigureSavePicker<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    enabled: jboolean,
) -> jint {
    let enabled = enabled != 0;
    call_status(engine, move |e| unsafe {
        op_editor_configure_save_picker(e, enabled)
    })
}

/// `OpNative.nativeEditorSaveFileName` — the pending save's suggested file
/// name, or null when no save is pending. Reading never consumes it.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorSaveFileName<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jstring {
    copy_engine_string(env, engine, op_editor_copy_save_file_name)
}

/// `OpNative.nativeEditorSaveTarget` — the durable destination handle a plain
/// Save should rewrite, or null when the shell must show the picker.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorSaveTarget<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jstring {
    copy_engine_string(env, engine, op_editor_copy_save_target)
}

/// `OpNative.nativeEditorStageSaveToPath` — write the canonical `.op` bytes
/// into a NEW app-private staging file. Does not mark the document saved.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorStageSaveToPath<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    path: JString<'local>,
) -> jint {
    let Some(path) = jstring_bytes(&mut env, &path) else {
        return OpStatus::InvalidArg as jint;
    };
    call_status(engine, move |e| unsafe {
        op_editor_stage_save_to_path(e, path.as_ptr(), path.len())
    })
}

/// `OpNative.nativeEditorCommitSave` — the staged bytes reached the picked
/// content URI; bind it and mark the document saved.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorCommitSave<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    handle: JString<'local>,
    display_name: JString<'local>,
) -> jint {
    let Some(handle) = jstring_bytes(&mut env, &handle) else {
        return OpStatus::InvalidArg as jint;
    };
    let Some(display_name) = jstring_bytes(&mut env, &display_name) else {
        return OpStatus::InvalidArg as jint;
    };
    call_status(engine, move |e| unsafe {
        op_editor_commit_save(
            e,
            handle.as_ptr(),
            handle.len(),
            display_name.as_ptr(),
            display_name.len(),
        )
    })
}

/// `OpNative.nativeEditorCancelSave` — the picker was dismissed, or the copy
/// into the picked URI failed. The document stays dirty either way.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorCancelSave<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    failed: jboolean,
) -> jint {
    let failed = failed != 0;
    call_status(engine, move |e| unsafe { op_editor_cancel_save(e, failed) })
}

/// Shared "size query, then copy, then `new_string`" body for the engine's
/// `copy_out`-shaped string getters. A zero required length (the engine's
/// "nothing to report" answer) and every failure come back as null.
fn copy_engine_string<'local>(
    env: JNIEnv<'local>,
    engine: jlong,
    read: unsafe extern "C" fn(
        *mut op_engine_ffi::OpEngine,
        *mut u8,
        usize,
        *mut usize,
    ) -> OpStatus,
) -> jstring {
    let bytes = with_engine(engine, move |e| {
        let mut required = 0usize;
        let status = unsafe { read(e, std::ptr::null_mut(), 0, &mut required) };
        if status != OpStatus::Ok || required == 0 {
            return None;
        }
        let mut bytes = vec![0_u8; required];
        let status = unsafe { read(e, bytes.as_mut_ptr(), bytes.len(), &mut required) };
        (status == OpStatus::Ok).then_some(bytes)
    })
    .flatten();
    let Some(bytes) = bytes else {
        return std::ptr::null_mut();
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return std::ptr::null_mut();
    };
    env.new_string(text)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// `OpNative.nativeEditorBeginLogin` — start the device flow after the shell
/// configured the auth runtime (RequestLogin follow-up).
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorBeginLogin<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe { op_editor_begin_login(e) })
}

/// `OpNative.nativeEditorSetLocale` — apply a UI locale by BCP-47 tag.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorSetLocale<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    tag: JString<'local>,
) -> jint {
    let Some(tag) = jstring_bytes(&mut env, &tag) else {
        return OpStatus::InvalidArg as jint;
    };
    call_status(engine, move |e| unsafe {
        op_editor_set_locale(e, tag.as_ptr(), tag.len())
    })
}

/// `OpNative.nativeEditorLocaleCode` — the current UI locale's BCP-47 tag.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorLocaleCode<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jstring {
    let bytes = with_engine(engine, move |e| {
        let mut required = 0usize;
        let status = unsafe { op_editor_locale_code(e, std::ptr::null_mut(), 0, &mut required) };
        if status != OpStatus::Ok || required == 0 {
            return None;
        }
        let mut bytes = vec![0_u8; required];
        let status =
            unsafe { op_editor_locale_code(e, bytes.as_mut_ptr(), bytes.len(), &mut required) };
        (status == OpStatus::Ok).then_some(bytes)
    })
    .flatten();
    let Some(bytes) = bytes else {
        return std::ptr::null_mut();
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return std::ptr::null_mut();
    };
    env.new_string(text)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// `OpNative.nativeEditorTakeShellAction` — returns an `OpShellAction` value.
/// Engine failures are negative so they cannot alias a valid action; an
/// unknown/closing handle uses the existing `STATUS_CLOSING` sentinel.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorTakeShellAction<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    with_engine(engine, move |e| {
        let mut action = SHELL_ACTION_NONE;
        let status = unsafe { op_editor_take_shell_action(e, &mut action) };
        if status == OpStatus::Ok {
            action
        } else {
            -(status as jint)
        }
    })
    .unwrap_or(STATUS_CLOSING)
}

/// `OpNative.nativeEditorOpenDocument` — install platform-picked bytes and an
/// optional display name, returning the stable `OpStatus` integer.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorOpenDocument<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    document: JByteArray<'local>,
    file_name: JString<'local>,
) -> jint {
    let Ok(document) = env.convert_byte_array(&document) else {
        return OpStatus::InvalidArg as jint;
    };
    let file_name = if file_name.is_null() {
        None
    } else {
        let Some(bytes) = jstring_bytes(&mut env, &file_name) else {
            return OpStatus::InvalidArg as jint;
        };
        Some(bytes)
    };
    call_status(engine, move |e| {
        let (file_name_ptr, file_name_len) = file_name
            .as_ref()
            .map_or((std::ptr::null(), 0), |bytes| (bytes.as_ptr(), bytes.len()));
        unsafe {
            op_editor_open_document(
                e,
                document.as_ptr(),
                document.len(),
                file_name_ptr,
                file_name_len,
            )
        }
    })
}

/// `OpNative.nativeEditorImportImageOrSvg` — return one bounded platform-picked
/// raster image or SVG to the editor. The FFI performs the authoritative
/// content validation and collaboration permission check on the owner thread.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorImportImageOrSvg<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    data: JByteArray<'local>,
    file_name: JString<'local>,
) -> jint {
    let Ok(data) = env.convert_byte_array(&data) else {
        return OpStatus::InvalidArg as jint;
    };
    let Some(file_name) = jstring_bytes(&mut env, &file_name) else {
        return OpStatus::InvalidArg as jint;
    };
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_import_image_or_svg(
            e,
            data.as_ptr(),
            data.len(),
            file_name.as_ptr(),
            file_name.len(),
        )
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorPress<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_press(e, x, y)
    })
}

/// `OpNative.nativeEditorPressAt` — press with the event's factual
/// monotonic timestamp (`MotionEvent.eventTime`). The signed `jlong`
/// clamps at zero before the `u64` ABI (a negative value is never a
/// valid clock).
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorPressAt<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
    t_ms: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_press_at(e, x, y, t_ms.max(0) as u64)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorMove<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_move(e, x, y)
    })
}

/// `OpNative.nativeEditorMoveAt` — move with `MotionEvent.eventTime`.
/// Signed `jlong` clamps at zero before `u64`.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorMoveAt<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
    t_ms: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_move_at(e, x, y, t_ms.max(0) as u64)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorRelease<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_release(e, x, y)
    })
}

/// `OpNative.nativeEditorReleaseAt` — release with the gesture endpoint's
/// factual `MotionEvent.eventTime`. Signed `jlong` clamps at zero before
/// `u64`.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorReleaseAt<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
    t_ms: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_release_at(e, x, y, t_ms.max(0) as u64)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorCancelGesture<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_cancel_gesture(e)
    })
}

/// `OpNative.nativeEditorCancelGestureAt` — cancel with the synthetic
/// `SystemClock.uptimeMillis()` timestamp. Signed `jlong` clamps at zero
/// before `u64`.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorCancelGestureAt<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    t_ms: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_cancel_gesture_at(e, t_ms.max(0) as u64)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorRightPress<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_right_press(e, x, y)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorBeginTransform<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_begin_transform(e, x, y)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorPan<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
    dx: jfloat,
    dy: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_pan(e, x, y, dx, dy)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorPinch<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    x: jfloat,
    y: jfloat,
    delta_y: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_pinch(e, x, y, delta_y)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorText<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    text: JString<'local>,
) -> jint {
    let Some(bytes) = jstring_bytes(&mut env, &text) else {
        return STATUS_CLOSING;
    };
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_text(e, bytes.as_ptr(), bytes.len())
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorKey<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    key: jint,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_key(e, key)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorImePreedit<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    text: JString<'local>,
    sel_start: jint,
    sel_end: jint,
) -> jint {
    let Some(bytes) = jstring_bytes(&mut env, &text) else {
        return STATUS_CLOSING;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return OpStatus::InvalidArg as jint;
    };
    let sel_start = crate::utf16_offset_to_utf8_byte(text, sel_start);
    let sel_end = crate::utf16_offset_to_utf8_byte(text, sel_end);
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_ime_preedit(e, bytes.as_ptr(), bytes.len(), sel_start, sel_end)
    })
}

/// `OpNative.nativeEditorPasteText` — paste clipboard text into whichever
/// text input owns the keyboard (long-press paste menu).
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorPasteText<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    text: JString<'local>,
) -> jint {
    let Some(bytes) = jstring_bytes(&mut env, &text) else {
        return STATUS_CLOSING;
    };
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_paste_text(e, bytes.as_ptr(), bytes.len())
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorImeCommit<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    text: JString<'local>,
) -> jint {
    let Some(bytes) = jstring_bytes(&mut env, &text) else {
        return STATUS_CLOSING;
    };
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_ime_commit(e, bytes.as_ptr(), bytes.len())
    })
}

/// `OpNative.nativeEditorImeFocused` — whether the editor host currently
/// holds the IME (show/hide the system keyboard).
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorImeFocused<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jni::sys::jboolean {
    let focused = with_engine(engine, move |e| {
        let mut focused = false;
        let status = unsafe { op_engine_ffi::op_editor_ime_focused(e, &mut focused) };
        status == op_engine_ffi::OpStatus::Ok && focused
    })
    .unwrap_or(false);
    focused as jni::sys::jboolean
}
