#![cfg(target_os = "android")]

//! Text + IME natives — split out of `bindings.rs`.

use jni::objects::{JClass, JObject, JString};
use jni::sys::{jint, jlong};
use jni::JNIEnv;

use crate::bindings::{call_status, jstring_bytes, with_engine, VM};
use crate::bindings_media::guard_jint;
use crate::STATUS_CLOSING;

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextBegin<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    node_id: JString<'local>,
) -> jint {
    let Some(bytes) = jstring_bytes(&mut env, &node_id) else {
        return STATUS_CLOSING;
    };
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_begin(e, bytes.as_ptr(), bytes.len())
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextEnd<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe { op_engine_ffi::op_text_end(e) })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextInsert<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    text: JString<'local>,
) -> jint {
    let Some(bytes) = jstring_bytes(&mut env, &text) else {
        return STATUS_CLOSING;
    };
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_insert(e, bytes.as_ptr(), bytes.len())
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextBackspace<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_backspace(e)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextDeleteForward<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_delete_forward(e)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextSetCaret<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    offset: jint,
    extend: jni::sys::jboolean,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_set_caret(e, offset as u32, extend != 0)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextSelectRange<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    anchor: jint,
    focus: jint,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_text_select_range(e, anchor as u32, focus as u32)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeImeSetComposingText<'local>(
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
        op_engine_ffi::op_ime_set_composing_text(
            e,
            bytes.as_ptr(),
            bytes.len(),
            sel_start as u32,
            sel_end as u32,
        )
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeImeCommitComposition<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_ime_commit_composition(e)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeImeCancelComposition<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_engine_ffi::op_ime_cancel_composition(e)
    })
}

/// Owned snapshot of the surrounding-text state (the C `text_ptr` is only
/// valid until the next engine call, so it is copied here on the engine
/// thread).
struct OwnedTextState {
    status: jint,
    text: String,
    selection_start: i32,
    selection_end: i32,
    has_composing: bool,
    composing_start: i32,
    composing_end: i32,
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextGetState<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    out: JObject<'local>,
) -> jint {
    guard_jint(|| {
        let state = with_engine(engine, move |e| {
            let mut state = op_engine_ffi::OpTextState {
                text_ptr: std::ptr::null(),
                text_len: 0,
                selection_start: 0,
                selection_end: 0,
                has_composing: false,
                composing_start: 0,
                composing_end: 0,
            };
            let status = unsafe { op_engine_ffi::op_text_get_state(e, &mut state) };
            // Copy the borrowed text out on the engine thread.
            let text = if status == op_engine_ffi::OpStatus::Ok
                && !state.text_ptr.is_null()
                && state.text_len > 0
            {
                unsafe {
                    String::from_utf8_lossy(std::slice::from_raw_parts(
                        state.text_ptr,
                        state.text_len,
                    ))
                    .into_owned()
                }
            } else {
                String::new()
            };
            OwnedTextState {
                status: status as i32,
                text,
                selection_start: state.selection_start as i32,
                selection_end: state.selection_end as i32,
                has_composing: state.has_composing,
                composing_start: state.composing_start as i32,
                composing_end: state.composing_end as i32,
            }
        })
        .unwrap_or(OwnedTextState {
            status: STATUS_CLOSING,
            text: String::new(),
            selection_start: 0,
            selection_end: 0,
            has_composing: false,
            composing_start: 0,
            composing_end: 0,
        });

        if state.status != 0 {
            return state.status;
        }
        let Ok(jtext) = env.new_string(&state.text) else {
            return STATUS_CLOSING;
        };
        if let Err(error) = env.set_field(
            &out,
            "text",
            "Ljava/lang/String;",
            jni::objects::JValue::Object(&jtext.into()),
        ) {
            let _ = error;
            return STATUS_CLOSING;
        }
        for (field, value) in [
            ("selectionStart", state.selection_start),
            ("selectionEnd", state.selection_end),
            ("composingStart", state.composing_start),
            ("composingEnd", state.composing_end),
        ] {
            if env
                .set_field(&out, field, "I", jni::objects::JValue::Int(value))
                .is_err()
            {
                return STATUS_CLOSING;
            }
        }
        if env
            .set_field(
                &out,
                "hasComposing",
                "Z",
                jni::objects::JValue::Bool(state.has_composing as u8),
            )
            .is_err()
        {
            return STATUS_CLOSING;
        }
        0
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeTextCaretRect<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    out: jni::sys::jfloatArray,
) -> jint {
    guard_jint(|| {
        let rect = with_engine(engine, move |e| {
            let mut rect = [0.0f32; 4];
            let status = unsafe { op_engine_ffi::op_text_caret_rect(e, rect.as_mut_ptr()) };
            (status as i32, rect)
        })
        .unwrap_or((STATUS_CLOSING, [0.0; 4]));
        if rect.0 != 0 {
            return rect.0;
        }
        // The engine thread is permanently attached; resolve its env.
        let Some(env) = VM.get().and_then(|vm| vm.get_env().ok()) else {
            return STATUS_CLOSING;
        };
        // SAFETY: `out` is the caller's writable jfloatArray (the JNI
        // contract of `nativeTextCaretRect`).
        let array = unsafe { jni::objects::JFloatArray::from_raw(out) };
        if env.set_float_array_region(&array, 0, &rect.1).is_err() {
            return STATUS_CLOSING;
        }
        0
    })
}

// ---- Remote images + fonts + pages --------------------------------------
