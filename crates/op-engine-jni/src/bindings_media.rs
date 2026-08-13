#![cfg(target_os = "android")]

//! Media/font/page natives — split out of `bindings.rs`.

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};

use jni::objects::{JByteArray, JClass};
use jni::sys::{jint, jlong};
use jni::JNIEnv;

use op_engine_ffi::{
    op_get_page_count, op_register_font, op_remote_image_result, op_set_active_page, OpEngine,
    OpStatus,
};

use crate::bindings::{call_status, jstring_bytes, with_engine};
use crate::engine_thread::STATUS_CLOSING;
use crate::registry::HANDLE_FAILURE;

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeRemoteImageResult<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    request_id: jlong,
    bytes: JByteArray<'local>,
) -> jint {
    let bytes = if bytes.is_null() {
        Vec::new()
    } else {
        match env.convert_byte_array(&bytes) {
            Ok(b) => b,
            Err(_) => return STATUS_CLOSING,
        }
    };
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_remote_image_result(e, request_id as u64, bytes.as_ptr(), bytes.len())
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeRegisterFont<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    bytes: JByteArray<'local>,
) -> jint {
    let Ok(bytes) = env.convert_byte_array(&bytes) else {
        return STATUS_CLOSING;
    };
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_register_font(e, bytes.as_ptr(), bytes.len())
    })
}

/// `OpNative.nativeGetPageCount` — the page count, or `STATUS_CLOSING`.
#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeGetPageCount<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    with_engine(engine, move |e| {
        let mut count = 0u32;
        let status = unsafe { op_engine_ffi::op_get_page_count(e, &mut count) };
        if status == op_engine_ffi::OpStatus::Ok {
            count as i32
        } else {
            status as i32
        }
    })
    .unwrap_or(STATUS_CLOSING)
}

#[no_mangle]
pub extern "system" fn Java_dev_openpencil_player_OpNative_nativeSetActivePage<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    index: jint,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_set_active_page(e, index as u32)
    })
}

/// Runs a `jint`-returning native body under an unwind guard (panic →
/// STATUS_CLOSING).
pub(crate) fn guard_jint(f: impl FnOnce() -> jint) -> jint {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => {
            crate::engine_thread::drop_guarded(payload);
            STATUS_CLOSING
        }
    }
}

// ---- Full-editor mode ----------------------------------------------------
