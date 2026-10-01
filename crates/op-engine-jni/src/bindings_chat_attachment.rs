#![cfg(target_os = "android")]

//! Chat / Studio Home attachment native — split out of `bindings_editor.rs`.

use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jint, jlong};
use jni::JNIEnv;

use op_engine_ffi::OpStatus;

use crate::bindings::{call_status, jstring_bytes};
use crate::ffi_bytes::{non_empty, optional_ptr_len};

/// `OpNative.nativeEditorAttachChatImage` — stage one photo-picker image as
/// a chat attachment (the answer to `SHELL_ACTION_PICK_CHAT_ATTACHMENT`).
/// `mediaType` and `name` may be null; a null or blank value reaches the FFI
/// as `NULL` / `0` so the engine sniffs the type or generates the name. The
/// FFI enforces the 5 MiB cap, the format sniff, and the per-turn limit.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorAttachChatImage<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    data: JByteArray<'local>,
    media_type: JString<'local>,
    file_name: JString<'local>,
) -> jint {
    let Ok(data) = env.convert_byte_array(&data) else {
        return OpStatus::InvalidArg as jint;
    };
    let Some(media_type) = optional_jstring_bytes(&mut env, &media_type) else {
        return OpStatus::InvalidArg as jint;
    };
    let Some(file_name) = optional_jstring_bytes(&mut env, &file_name) else {
        return OpStatus::InvalidArg as jint;
    };
    call_status(engine, move |e| {
        let (media_type_ptr, media_type_len) = optional_ptr_len(media_type.as_deref());
        let (file_name_ptr, file_name_len) = optional_ptr_len(file_name.as_deref());
        // SAFETY: every range is owned by this closure and outlives the
        // synchronous FFI call; nothing is retained past it.
        unsafe {
            op_engine_ffi::op_editor_attach_chat_image(
                e,
                data.as_ptr(),
                data.len(),
                media_type_ptr,
                media_type_len,
                file_name_ptr,
                file_name_len,
            )
        }
    })
}

/// A nullable Java string as owned canonical UTF-8: `Some(None)` for null or
/// blank, `None` only when a non-null string could not be decoded.
fn optional_jstring_bytes(env: &mut JNIEnv, value: &JString) -> Option<Option<Vec<u8>>> {
    if value.is_null() {
        return Some(None);
    }
    jstring_bytes(env, value).map(|bytes| non_empty(Some(bytes)))
}
