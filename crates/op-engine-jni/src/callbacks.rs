//! Engine → Java upcall trampolines.
//!
//! The C ABI invokes these callbacks synchronously ON the engine thread
//! (inside `op_pointer`, `op_attach_surface`, …). Each trampoline copies
//! its borrowed C payload into owned Java values, calls the one
//! `OpCallbacks` receiver, then clears any pending exception. Every body is
//! bracketed by a JNI local frame so per-upcall Strings never accumulate in
//! the engine thread's local-reference table (the thread stays attached for
//! its whole life).
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

use crate::engine_thread::{enter_callback_frame, exit_callback_frame};

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
        upcall(ctx, 2, |env, receiver| {
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
