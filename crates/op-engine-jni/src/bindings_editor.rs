#![cfg(target_os = "android")]

//! Editor-mode natives — split out of `bindings.rs`.

use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jfloat, jint, jlong, jstring};
use jni::JNIEnv;

use op_engine_ffi::{
    op_editor_cancel_login, op_editor_configure_auth, op_editor_copy_login_url,
    op_editor_open_document, op_editor_take_shell_action, OpStatus, SHELL_ACTION_NONE,
};

use crate::bindings::{call_status, jstring_bytes, with_engine};
use crate::engine_thread::STATUS_CLOSING;

/// `OpNative.nativeEditorConfigureAuth` — initialize the real mobile auth
/// backend with a private shell-owned storage directory.
#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorConfigureAuth<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    storage_dir: JString<'local>,
    device_name: JString<'local>,
    app_version: JString<'local>,
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
        )
    })
}

/// `OpNative.nativeEditorTakeLoginUrl` — atomically copies and consumes the
/// pending UTF-8 verification URL, or returns null when none is ready.
#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorTakeLoginUrl<'local>(
    mut env: JNIEnv<'local>,
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

/// `OpNative.nativeEditorCancelLogin` — user-dismissed WebView cancellation.
#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorCancelLogin<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe { op_editor_cancel_login(e) })
}

/// `OpNative.nativeEditorTakeShellAction` — returns an `OpShellAction` value.
/// Engine failures are negative so they cannot alias a valid action; an
/// unknown/closing handle uses the existing `STATUS_CLOSING` sentinel.
#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorTakeShellAction<'local>(
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
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorOpenDocument<'local>(
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

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorPress<'local>(
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

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorMove<'local>(
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

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorRelease<'local>(
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

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorCancelGesture<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_cancel_gesture(e)
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorRightPress<'local>(
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
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorBeginTransform<'local>(
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
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorPan<'local>(
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
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorPinch<'local>(
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
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorText<'local>(
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
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorKey<'local>(
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
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorImePreedit<'local>(
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
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_editor_ime_preedit(
            e,
            bytes.as_ptr(),
            bytes.len(),
            sel_start as usize,
            sel_end as usize,
        )
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorImeCommit<'local>(
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
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeEditorImeFocused<'local>(
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
