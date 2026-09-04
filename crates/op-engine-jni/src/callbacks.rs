//! Engine → Java upcall trampolines.
//!
//! Ordinary UI callbacks run synchronously on the engine thread (inside
//! `op_pointer`, `op_attach_surface`, …). Credential callbacks and redraw wakes
//! are worker-thread exceptions: collaboration workers may need to restart a
//! paused frame pump, so those callbacks attach only for their call. Every
//! trampoline uses a JNI local frame and clears pending Java exceptions before
//! returning.
//!
//! `user_data` for the C callback table is a `*const EngineCtx` owned by the
//! engine record; it outlives every callback and is freed only in the
//! teardown final job, after `op_destroy` returns.

#![cfg(target_os = "android")]

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};

use jni::objects::{GlobalRef, JObject, JValue};
use jni::{JNIEnv, JavaVM};

use op_engine_ffi::{OpCallbacks, OpRuntimeError};
use zeroize::Zeroize;

use crate::engine_thread::{enter_callback_frame, exit_callback_frame};

const COLLAB_CREDENTIAL_BYTES: usize = 32;

struct WipedCredential([i8; COLLAB_CREDENTIAL_BYTES]);

impl Drop for WipedCredential {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Per-engine upcall context; the `user_data` behind the C callback table.
pub struct EngineCtx {
    vm: JavaVM,
    /// The `OpCallbacks` Java receiver (a global ref — valid across
    /// threads and callbacks).
    receiver: GlobalRef,
}

impl EngineCtx {
    pub fn new(vm: JavaVM, receiver: GlobalRef) -> Self {
        Self { vm, receiver }
    }

    /// The engine thread's `JNIEnv`. The engine thread is attached to the VM
    /// permanently at spawn, so `get_env` always succeeds here; a failure
    /// means we are off the engine thread (a bug) and the upcall is skipped.
    fn env(&self) -> Option<JNIEnv<'_>> {
        self.vm.get_env().ok()
    }
}

/// Builds the C callback table pointing at a freshly boxed [`EngineCtx`].
/// The returned raw pointer is the table's `user_data`; the engine record
/// owns it and frees it (`drop_ctx`) in the teardown final job.
pub fn build_callbacks(ctx: Box<EngineCtx>) -> (OpCallbacks, *mut EngineCtx) {
    let raw = Box::into_raw(ctx);
    let table = OpCallbacks {
        size: std::mem::size_of::<OpCallbacks>(),
        user_data: raw as *mut c_void,
        needs_redraw: Some(needs_redraw),
        runtime_error: Some(runtime_error),
        input_focus_changed: Some(input_focus_changed),
        remote_image_request: Some(remote_image_request),
        credential_load: Some(credential_load),
        credential_store_if_absent: Some(credential_store_if_absent),
    };
    (table, raw)
}

/// Frees the boxed context. Called ONCE, on the engine thread, in the
/// teardown final job after `op_destroy` has returned (no further callback
/// can fire).
///
/// # Safety
/// `raw` must be the pointer returned by [`build_callbacks`] and not yet
/// freed.
pub unsafe fn drop_ctx(raw: *mut EngineCtx) {
    if !raw.is_null() {
        drop(unsafe { Box::from_raw(raw) });
    }
}

/// Casts `user_data` back to the borrowed context. Returns `None` for a null
/// pointer (never expected — the table always carries a live context).
///
/// # Safety
/// `user_data` must be a `*const EngineCtx` from [`build_callbacks`] that is
/// still live (guaranteed while any callback can fire).
unsafe fn ctx<'a>(user_data: *mut c_void) -> Option<&'a EngineCtx> {
    (user_data as *const EngineCtx).as_ref()
}

/// Runs `body` inside a JNI local frame with the receiver in hand, then
/// checks-and-clears any pending exception. Missing env / frame errors are
/// swallowed: an upcall must never unwind across the C ABI.
fn upcall(ctx: &EngineCtx, capacity: i32, body: impl FnOnce(&mut JNIEnv, &JObject)) {
    let Some(mut env) = ctx.env() else {
        return;
    };
    let receiver = ctx.receiver.clone();
    // Bracket the callback so a native re-entered from the Java callback
    // (e.g. nativeDestroy) sees `in_callback_frame()` and defers per the
    // no-re-entry rule. The guard restores the depth even if the body panics.
    let _frame = CallbackFrame::enter();
    let _framed = env.with_local_frame(capacity, |env| -> Result<(), jni::errors::Error> {
        // Catch INSIDE the frame so a panic in marshalling can never unwind
        // across the C callback ABI (which would abort).
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| body(env, receiver.as_obj()))) {
            crate::engine_thread::drop_guarded(payload);
        }
        Ok(())
    });
    // Clear any exception the Java callback left pending; describe it first
    // for the log. Both are best-effort.
    if let Ok(true) = env.exception_check() {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

/// Worker-safe variant used only for callbacks whose contract permits an
/// off-engine-thread wake. `attach_current_thread` is also safe when the caller
/// is already the permanently attached engine thread, while a collaboration
/// worker is detached again when the guard leaves scope.
fn attached_upcall(ctx: &EngineCtx, capacity: i32, body: impl FnOnce(&mut JNIEnv, &JObject)) {
    let Ok(mut env) = ctx.vm.attach_current_thread() else {
        return;
    };
    let receiver = ctx.receiver.clone();
    let _frame = CallbackFrame::enter();
    let _framed = env.with_local_frame(capacity, |env| -> Result<(), jni::errors::Error> {
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| body(env, receiver.as_obj()))) {
            crate::engine_thread::drop_guarded(payload);
        }
        Ok(())
    });
    if let Ok(true) = env.exception_check() {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

/// RAII bracket for a C callback frame (drives `close_deferred` routing on
/// callback-origin destroy). Restores the depth on drop, panic or not.
struct CallbackFrame;

impl CallbackFrame {
    fn enter() -> Self {
        enter_callback_frame();
        CallbackFrame
    }
}

impl Drop for CallbackFrame {
    fn drop(&mut self) {
        exit_callback_frame();
    }
}

/// Runs a callback trampoline body under an unwind guard covering the WHOLE
/// trampoline — the context lookup, the C-pointer marshalling, the JNI local
/// frame, and exception cleanup — so a panic anywhere can never cross the
/// non-unwinding C callback ABI.
fn run_trampoline(user_data: *mut c_void, body: impl FnOnce(&EngineCtx)) {
    let guarded = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `user_data` is a live `*const EngineCtx` while any callback
        // can fire (freed only after op_destroy on the engine thread).
        if let Some(ctx) = unsafe { ctx(user_data) } {
            body(ctx);
        }
    }));
    if let Err(payload) = guarded {
        crate::engine_thread::drop_guarded(payload);
    }
}

extern "C" fn needs_redraw(user_data: *mut c_void, has_next_wake: bool, next_wake_ms: u64) {
    run_trampoline(user_data, |ctx| {
        attached_upcall(ctx, 2, |env, receiver| {
            let _ = env.call_method(
                receiver,
                "onNeedsRedraw",
                "(ZJ)V",
                &[
                    JValue::Bool(has_next_wake as u8),
                    JValue::Long(next_wake_ms as i64),
                ],
            );
        });
    });
}

extern "C" fn runtime_error(user_data: *mut c_void, error: *const OpRuntimeError) {
    run_trampoline(user_data, |ctx| {
        let Some(error) = (unsafe { error.as_ref() }) else {
            return;
        };
        let message = unsafe { borrowed_str(error.message_ptr, error.message_len) };
        let source = if error.source_ptr.is_null() {
            None
        } else {
            Some(unsafe { borrowed_str(error.source_ptr, error.source_len) })
        };
        let kind = error.kind;
        upcall(ctx, 4, |env, receiver| {
            let Ok(jmessage) = env.new_string(&message) else {
                return;
            };
            let jsource = match &source {
                Some(s) => match env.new_string(s) {
                    Ok(js) => js.into(),
                    Err(_) => return,
                },
                None => JObject::null(),
            };
            let _ = env.call_method(
                receiver,
                "onRuntimeError",
                "(ILjava/lang/String;Ljava/lang/String;)V",
                &[
                    JValue::Int(kind),
                    JValue::Object(&jmessage.into()),
                    JValue::Object(&jsource),
                ],
            );
        });
    });
}

extern "C" fn input_focus_changed(
    user_data: *mut c_void,
    focused: bool,
    input_kind: i32,
    return_key_hint: i32,
) {
    run_trampoline(user_data, |ctx| {
        upcall(ctx, 2, |env, receiver| {
            let _ = env.call_method(
                receiver,
                "onInputFocusChanged",
                "(ZII)V",
                &[
                    JValue::Bool(focused as u8),
                    JValue::Int(input_kind),
                    JValue::Int(return_key_hint),
                ],
            );
        });
    });
}

extern "C" fn remote_image_request(
    user_data: *mut c_void,
    request_id: u64,
    url_ptr: *const u8,
    url_len: usize,
) {
    run_trampoline(user_data, |ctx| {
        let url = unsafe { borrowed_str(url_ptr, url_len) };
        upcall(ctx, 3, |env, receiver| {
            let Ok(jurl) = env.new_string(&url) else {
                return;
            };
            let _ = env.call_method(
                receiver,
                "onRemoteImageRequest",
                "(JLjava/lang/String;)V",
                &[
                    JValue::Long(request_id as i64),
                    JValue::Object(&jurl.into()),
                ],
            );
        });
    });
}

/// Secure-store callbacks may run off the engine thread, like collaboration
/// redraw wakes. Workers attach for the duration of the call; callbacks whose
/// contracts remain owner-thread-only keep using `get_env` above.
extern "C" fn credential_load(
    user_data: *mut c_void,
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> i32 {
    if out.is_null() || out_len.is_null() {
        return -1;
    }
    // SAFETY: `out_len` was checked and is writable by the callback contract.
    unsafe { out_len.write(0) };
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: EngineCtx outlives every collaboration worker; op_destroy
        // joins those workers before the context is freed.
        let Some(ctx) = (unsafe { ctx(user_data) }) else {
            return -1;
        };
        let Ok(mut env) = ctx.vm.attach_current_thread() else {
            return -1;
        };
        let receiver = ctx.receiver.clone();
        let mut status = -1;
        let framed = env.with_local_frame(4, |env| -> Result<(), jni::errors::Error> {
            let value = env.call_method(receiver.as_obj(), "onCredentialLoad", "()[B", &[]);
            if clear_pending_exception(env) {
                return Ok(());
            }
            let object = value?.l()?;
            if object.is_null() {
                status = 1;
                return Ok(());
            }
            let array = jni::objects::JByteArray::from(object);
            let length = env.get_array_length(&array)? as usize;
            let mut temporary = WipedCredential([0_i8; COLLAB_CREDENTIAL_BYTES]);
            let valid = length == COLLAB_CREDENTIAL_BYTES && capacity >= COLLAB_CREDENTIAL_BYTES;
            let read = valid
                && env
                    .get_byte_array_region(&array, 0, &mut temporary.0)
                    .is_ok();
            let cleared = wipe_java_byte_array(env, &array, length);
            if valid && read && cleared {
                // SAFETY: the caller supplied `capacity` writable bytes and
                // the fixed credential length fits that range.
                unsafe {
                    for (index, byte) in temporary.0.iter().enumerate() {
                        out.add(index).write(*byte as u8);
                    }
                    out_len.write(COLLAB_CREDENTIAL_BYTES);
                }
                status = 0;
            }
            Ok(())
        });
        if framed.is_err() || clear_pending_exception(&mut env) {
            -1
        } else {
            status
        }
    }))
    .unwrap_or(-1)
}

extern "C" fn credential_store_if_absent(
    user_data: *mut c_void,
    value: *const u8,
    value_len: usize,
) -> i32 {
    if value.is_null() || value_len != COLLAB_CREDENTIAL_BYTES {
        return -1;
    }
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: see `credential_load`; the input is borrowed for this call.
        let Some(ctx) = (unsafe { ctx(user_data) }) else {
            return -1;
        };
        let Ok(mut env) = ctx.vm.attach_current_thread() else {
            return -1;
        };
        let receiver = ctx.receiver.clone();
        let mut status = -1;
        let framed = env.with_local_frame(4, |env| -> Result<(), jni::errors::Error> {
            // SAFETY: the C callback contract guarantees this readable range.
            let source = unsafe { std::slice::from_raw_parts(value, COLLAB_CREDENTIAL_BYTES) };
            let mut temporary = WipedCredential([0_i8; COLLAB_CREDENTIAL_BYTES]);
            for (destination, source) in temporary.0.iter_mut().zip(source) {
                *destination = *source as i8;
            }
            let array = env.new_byte_array(COLLAB_CREDENTIAL_BYTES as i32)?;
            if env.set_byte_array_region(&array, 0, &temporary.0).is_err() {
                return Ok(());
            }

            let result = env.call_method(
                receiver.as_obj(),
                "onCredentialStoreIfAbsent",
                "([B)Z",
                &[JValue::Object(array.as_ref())],
            );
            let had_exception = clear_pending_exception(env);
            let cleared = wipe_java_byte_array(env, &array, COLLAB_CREDENTIAL_BYTES);
            if !had_exception && cleared && result?.z()? {
                status = 0;
            }
            Ok(())
        });
        if framed.is_err() || clear_pending_exception(&mut env) {
            -1
        } else {
            status
        }
    }))
    .unwrap_or(-1)
}

fn clear_pending_exception(env: &mut JNIEnv<'_>) -> bool {
    match env.exception_check() {
        Ok(true) => {
            let _ = env.exception_clear();
            true
        }
        Ok(false) => false,
        Err(_) => true,
    }
}

fn wipe_java_byte_array(
    env: &mut JNIEnv<'_>,
    array: &jni::objects::JByteArray<'_>,
    length: usize,
) -> bool {
    let zeros = [0_i8; COLLAB_CREDENTIAL_BYTES];
    let mut offset = 0_usize;
    while offset < length {
        let count = (length - offset).min(zeros.len());
        if env
            .set_byte_array_region(array, offset as i32, &zeros[..count])
            .is_err()
        {
            return false;
        }
        offset += count;
    }
    true
}

/// Copies a borrowed C byte range into an owned `String` (the payload is
/// only valid for the duration of the callback).
///
/// # Safety
/// `pointer` must cover `length` readable bytes for the call's duration.
unsafe fn borrowed_str(pointer: *const u8, length: usize) -> String {
    if pointer.is_null() || length == 0 {
        return String::new();
    }
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    String::from_utf8_lossy(bytes).into_owned()
}
