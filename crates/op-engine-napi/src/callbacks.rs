//! Engine → ArkTS upcall trampolines.
//!
//! The Android player calls back into Java synchronously from the engine
//! thread. Node-API has no synchronous cross-thread call, so every upcall
//! here goes through a napi threadsafe function: the engine thread queues the
//! payload and returns immediately, and ArkUI's event loop runs the JS
//! handler. Payloads are therefore always OWNED copies made on the engine
//! thread (the C pointers are only valid for the duration of the call).
//!
//! `user_data` for the C callback table is a `*mut EngineCtx` owned by the
//! engine record; it outlives every callback and is freed only in the
//! teardown final job, after `op_destroy` returns.
//!
//! Secure-store (`credential_*`) callbacks are deliberately NOT installed:
//! they must return a value synchronously from a collaboration worker thread,
//! which Node-API cannot do. With both left null the engine falls back to its
//! own `CollabRuntime` key store (see `editor_collab::runtime_for_callbacks`)
//! instead of a platform keystore — the documented OHOS limitation.

#![cfg(all(target_os = "linux", target_env = "ohos"))]

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};

use napi_ohos::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use op_engine_ffi::{OpCallbacks, OpRuntimeError};

/// `(hasNextWake, nextWakeMs)`.
pub type NeedsRedrawFn = ThreadsafeFunction<(bool, i64), ()>;
/// `(kind, message, source)`.
pub type RuntimeErrorFn = ThreadsafeFunction<(i32, String, Option<String>), ()>;
/// `(focused, inputKind, returnKeyHint)`.
pub type InputFocusFn = ThreadsafeFunction<(bool, i32, i32), ()>;
/// `(requestId, url)`.
pub type RemoteImageFn = ThreadsafeFunction<(i64, String), ()>;

/// Per-engine upcall context; the `user_data` behind the C callback table.
/// Every field is optional so an ArkTS shell can subscribe to only what it
/// handles (a missing handler drops the event, exactly like a Java receiver
/// whose method throws).
#[derive(Default)]
pub struct EngineCtx {
    pub needs_redraw: Option<NeedsRedrawFn>,
    pub runtime_error: Option<RuntimeErrorFn>,
    pub input_focus_changed: Option<InputFocusFn>,
    pub remote_image_request: Option<RemoteImageFn>,
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
        // See the module docs: Node-API cannot answer a synchronous
        // worker-thread upcall, so the engine keeps its own key store.
        credential_load: None,
        credential_store_if_absent: None,
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

/// Casts `user_data` back to the borrowed context.
///
/// # Safety
/// `user_data` must be a `*mut EngineCtx` from [`build_callbacks`] that is
/// still live (guaranteed while any callback can fire).
unsafe fn ctx<'a>(user_data: *mut c_void) -> Option<&'a EngineCtx> {
    (user_data as *const EngineCtx).as_ref()
}

/// Runs a trampoline body under an unwind guard covering the WHOLE
/// trampoline — context lookup, C-pointer marshalling, and the queue push —
/// so a panic can never cross the non-unwinding C callback ABI.
///
/// The callback frame is bracketed for the same reason the Android layer does
/// it: a `destroy` re-entered from an ArkTS handler must defer instead of
/// blocking. (With threadsafe functions the JS handler runs later, off the
/// engine thread, so this is belt-and-braces — but the rule is the engine's,
/// not the binding's.)
fn run_trampoline(user_data: *mut c_void, body: impl FnOnce(&EngineCtx)) {
    let guarded = catch_unwind(AssertUnwindSafe(|| {
        let _frame = CallbackFrame::enter();
        // SAFETY: `user_data` is a live `*mut EngineCtx` while any callback
        // can fire (freed only after op_destroy on the engine thread).
        if let Some(ctx) = unsafe { ctx(user_data) } {
            body(ctx);
        }
    }));
    if let Err(payload) = guarded {
        op_engine_jni::engine_thread::drop_guarded(payload);
    }
}

/// RAII bracket for a C callback frame. Restores the depth on drop, panic or
/// not.
struct CallbackFrame;

impl CallbackFrame {
    fn enter() -> Self {
        op_engine_jni::engine_thread::enter_callback_frame();
        CallbackFrame
    }
}

impl Drop for CallbackFrame {
    fn drop(&mut self) {
        op_engine_jni::engine_thread::exit_callback_frame();
    }
}

extern "C" fn needs_redraw(user_data: *mut c_void, has_next_wake: bool, next_wake_ms: u64) {
    run_trampoline(user_data, |ctx| {
        if let Some(callback) = ctx.needs_redraw.as_ref() {
            callback.call(
                Ok((has_next_wake, next_wake_ms as i64)),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
    });
}

extern "C" fn runtime_error(user_data: *mut c_void, error: *const OpRuntimeError) {
    run_trampoline(user_data, |ctx| {
        // SAFETY: the engine passes a borrowed, non-null payload for the
        // duration of the call.
        let Some(error) = (unsafe { error.as_ref() }) else {
            return;
        };
        // SAFETY: both ranges are readable for the duration of the call.
        let message = unsafe { borrowed_str(error.message_ptr, error.message_len) };
        let source = if error.source_ptr.is_null() {
            None
        } else {
            Some(unsafe { borrowed_str(error.source_ptr, error.source_len) })
        };
        if let Some(callback) = ctx.runtime_error.as_ref() {
            callback.call(
                Ok((error.kind, message, source)),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
    });
}

extern "C" fn input_focus_changed(
    user_data: *mut c_void,
    focused: bool,
    input_kind: i32,
    return_key_hint: i32,
) {
    run_trampoline(user_data, |ctx| {
        if let Some(callback) = ctx.input_focus_changed.as_ref() {
            callback.call(
                Ok((focused, input_kind, return_key_hint)),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
    });
}

extern "C" fn remote_image_request(
    user_data: *mut c_void,
    request_id: u64,
    url_ptr: *const u8,
    url_len: usize,
) {
    run_trampoline(user_data, |ctx| {
        // SAFETY: the range is readable for the duration of the call.
        let url = unsafe { borrowed_str(url_ptr, url_len) };
        if let Some(callback) = ctx.remote_image_request.as_ref() {
            callback.call(
                Ok((request_id as i64, url)),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
    });
}

/// Copies a borrowed C byte range into an owned `String` (the payload is only
/// valid for the duration of the callback).
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
