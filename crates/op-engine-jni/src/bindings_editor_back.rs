//! Android system Back dispatch to the shared editor navigation ladder.

use jni::objects::JClass;
use jni::sys::{jboolean, jlong};
use jni::JNIEnv;
use op_engine_ffi::{op_editor_back, op_editor_ime_dismiss, OpStatus};

use crate::bindings::with_engine;

/// System Back reports consumption, unlike the key API's status result.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorBack<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jboolean {
    with_engine(engine, move |e| {
        let mut consumed = false;
        let status = unsafe { op_editor_back(e, &mut consumed) };
        status == OpStatus::Ok && consumed
    })
    .unwrap_or(false) as jboolean
}

/// IME Back releases field focus rather than advancing the navigation ladder.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeEditorImeDismiss<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jboolean {
    with_engine(engine, move |e| {
        let mut dismissed = false;
        let status = unsafe { op_editor_ime_dismiss(e, &mut dismissed) };
        status == OpStatus::Ok && dismissed
    })
    .unwrap_or(false) as jboolean
}
