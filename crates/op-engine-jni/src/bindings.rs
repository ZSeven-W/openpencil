//! JNI natives for `tech.zseven.openpencil.OpNative`.
//!
//! Every native validates its `jlong` handle against the tombstoning
//! [`registry`](crate::registry) first (a closed/unknown handle returns
//! [`STATUS_CLOSING`]), then dispatches the engine work onto that engine's
//! dedicated thread via [`EngineThread`] — engine pointers are only ever
//! dereferenced there. The caller frame owns argument conversion; owned
//! results come back through the blocking barrier.

#![cfg(target_os = "android")]

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::{Arc, OnceLock};

use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JString};
use jni::sys::{jboolean, jfloat, jint, jlong};
use jni::{JNIEnv, JavaVM};

use op_engine_ffi::{
    op_attach_surface, op_background_tick, op_create, op_destroy, op_frame, op_has_background_work,
    op_pointer, op_prefers_light_system_icons, op_resize, op_resize_with_safe_area, op_resume,
    op_set_keyboard, op_set_safe_area, op_suspend, OpCreateDesc, OpEngine, OpStatus, OpSurfaceDesc,
};

use crate::callbacks::{build_callbacks, drop_ctx, EngineCtx};
use crate::registry::{Registry, HANDLE_FAILURE};
use crate::{EngineThread, STATUS_CLOSING};

/// The process `JavaVM`, captured in [`JNI_OnLoad`]. Every engine thread
/// attaches through it and every callback resolves its `JNIEnv` from it.
pub(crate) static VM: OnceLock<JavaVM> = OnceLock::new();

/// The process-global engine registry (handles → records).
fn registry() -> &'static Registry<EngineRecord> {
    static REGISTRY: OnceLock<Registry<EngineRecord>> = OnceLock::new();
    REGISTRY.get_or_init(Registry::new)
}

/// A raw engine pointer. Dereferenced ONLY on the owning engine thread; the
/// `Send` impl carries it across the dispatch barrier and the registry.
#[derive(Clone, Copy)]
struct EnginePtr(*mut OpEngine);
// SAFETY: the pointer is only ever used on the engine thread; the wrapper
// exists solely to move it through the queue and registry.
unsafe impl Send for EnginePtr {}

impl EnginePtr {
    /// Returns the raw pointer. Taking `self` by value makes a closure
    /// capture the whole (Send) wrapper, not the raw field (disjoint
    /// closure capture would otherwise grab the `!Send` pointer).
    fn get(self) -> *mut OpEngine {
        self.0
    }
}

/// The callback context pointer (freed once, in the destroy final job).
#[derive(Clone, Copy)]
struct CtxPtr(*mut EngineCtx);
// SAFETY: freed exactly once on the engine thread after op_destroy.
unsafe impl Send for CtxPtr {}

impl CtxPtr {
    fn get(self) -> *mut EngineCtx {
        self.0
    }
}

/// Per-engine record held in the registry. The thread is behind an `Arc` so
/// a native can clone the dispatch handle out from under the registry lock
/// and release the lock BEFORE the blocking `call()` — otherwise a callback
/// re-entering a native would deadlock against the held registry mutex.
struct EngineRecord {
    thread: Arc<EngineThread>,
    engine: EnginePtr,
    ctx: CtxPtr,
}

/// Reconstructs an owned `JavaVM` handle from the captured one (JavaVM is not
/// `Clone`; the raw VM pointer is process-global and stable).
fn clone_vm(vm: &JavaVM) -> JavaVM {
    // SAFETY: the pointer came from a live JavaVM captured in JNI_OnLoad.
    unsafe { JavaVM::from_raw(vm.get_java_vm_pointer()) }.expect("valid JavaVM pointer")
}

/// Records the process `JavaVM` so engine threads can attach and callbacks
/// can resolve a `JNIEnv`. The JVM calls this exactly once at library load.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "system" fn JNI_OnLoad(vm: *mut jni::sys::JavaVM, _reserved: *mut c_void) -> jint {
    if let Ok(vm) = unsafe { JavaVM::from_raw(vm) } {
        let _ = VM.set(vm);
    }
    jni::sys::JNI_VERSION_1_6
}

/// `OpNative.nativeCreate` — spawns the engine thread, attaches it to the
/// VM, and creates the engine ON that thread. Returns the handle, or `0` on
/// failure (the reason is readable via `nativeLastError(0)`). The whole body
/// is guarded: `EngineThread::spawn` can panic (OS refuses the thread), and
/// that panic must never cross the non-unwinding `extern "system"` boundary.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeCreate<'local>(
    env: JNIEnv<'local>,
    class: JClass<'local>,
    doc: JByteArray<'local>,
    w: jfloat,
    h: jfloat,
    dpr: jfloat,
    receiver: JObject<'local>,
    storage_root: JString<'local>,
    mode: jint,
) -> jlong {
    match catch_unwind(AssertUnwindSafe(|| {
        create_impl(env, class, doc, w, h, dpr, receiver, storage_root, mode)
    })) {
        Ok(handle) => handle,
        Err(payload) => {
            crate::engine_thread::drop_guarded(payload);
            registry().set_create_error("nativeCreate panicked");
            HANDLE_FAILURE
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_impl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    doc: JByteArray<'local>,
    w: jfloat,
    h: jfloat,
    dpr: jfloat,
    receiver: JObject<'local>,
    storage_root: JString<'local>,
    mode: jint,
) -> jlong {
    let Some(vm) = VM.get() else {
        registry().set_create_error("JavaVM not captured (JNI_OnLoad did not run)");
        return HANDLE_FAILURE;
    };

    let doc_bytes = match env.convert_byte_array(&doc) {
        Ok(bytes) => bytes,
        Err(_) => {
            registry().set_create_error("could not read document bytes");
            return HANDLE_FAILURE;
        }
    };
    let storage_root: String = match env.get_string(&storage_root) {
        Ok(value) => value.into(),
        Err(_) => {
            registry().set_create_error("could not read private storage root");
            return HANDLE_FAILURE;
        }
    };
    let receiver: GlobalRef = match env.new_global_ref(&receiver) {
        Ok(r) => r,
        Err(_) => {
            registry().set_create_error("could not globalize the callback receiver");
            return HANDLE_FAILURE;
        }
    };

    // EngineCtx is Send (JavaVM + GlobalRef). It is kept HERE (on the
    // already-attached JNI caller thread) until attachment succeeds — if the
    // engine thread can never attach, dropping the ctx's GlobalRef there
    // would leak (DeleteGlobalRef needs an attached thread), so it is
    // disposed on the caller instead.
    let ctx = Box::new(EngineCtx::new(clone_vm(vm), receiver));
    let thread = Arc::new(EngineThread::spawn("op-engine"));

    // Step 1: attach the engine thread to the VM (permanent — detaches at
    // thread exit) so its callbacks have a JNIEnv.
    let attach_vm = clone_vm(vm);
    let attached = thread.call(move || attach_vm.attach_current_thread_permanently().is_ok());
    if !matches!(attached, crate::Dispatch::Done(true)) {
        thread.close(|| {});
        drop(ctx); // dispose the GlobalRef HERE (attached caller thread)
        registry().set_create_error("engine thread could not attach to the JVM");
        return HANDLE_FAILURE;
    }

    // Step 2: build the callbacks table and create the engine, both on the
    // now-attached engine thread. The !Send OpCallbacks table never leaves
    // it; the ctx is moved in only now (its later disposal is on this
    // attached thread).
    let created = thread.call(move || {
        let (callbacks, ctx_ptr) = build_callbacks(ctx);
        let mut engine: *mut OpEngine = ptr::null_mut();
        let desc = OpCreateDesc {
            size: std::mem::size_of::<OpCreateDesc>(),
            doc_ptr: doc_bytes.as_ptr(),
            doc_len: doc_bytes.len(),
            width: w,
            height: h,
            dpr,
            callbacks: &callbacks,
            asset_base_ptr: ptr::null(),
            asset_base_len: 0,
            mode,
            storage_root_ptr: storage_root.as_ptr(),
            storage_root_len: storage_root.len(),
            // No user-visible documents root yet: Android / HarmonyOS keep
            // the private `<storage_root>/documents` fallback.
            documents_root_ptr: ptr::null(),
            documents_root_len: 0,
        };
        let status = unsafe { op_create(&desc, &mut engine) };
        // `callbacks`/`doc_bytes` stay alive until here.
        (status as i32, engine as usize, ctx_ptr as usize)
    });

    let (status, engine_raw, ctx_raw) = match created {
        crate::Dispatch::Done(v) => v,
        crate::Dispatch::Closing => {
            thread.close(|| {});
            registry().set_create_error("engine thread closed during create");
            return HANDLE_FAILURE;
        }
    };

    if status != 0 || engine_raw == 0 {
        // Free the context on the engine thread (attached), then tear down.
        let ctx = CtxPtr(ctx_raw as *mut EngineCtx);
        thread.close(move || unsafe { drop_ctx(ctx.get()) });
        registry().set_create_error(format!("op_create failed (status {status})"));
        return HANDLE_FAILURE;
    }

    registry().insert(EngineRecord {
        thread,
        engine: EnginePtr(engine_raw as *mut OpEngine),
        ctx: CtxPtr(ctx_raw as *mut EngineCtx),
    })
}

/// `OpNative.nativeLastError` — the last error text for a handle (or the
/// create-failure text for handle `0`); empty for an unknown handle.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeLastError<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jni::sys::jstring {
    guard_jstring(|| {
        let message = registry().last_error(engine);
        match env.new_string(message) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// `OpNative.nativeDestroy` — teardown. Tombstones the handle, then closes
/// the engine thread with a final job that destroys the engine, releases any
/// attached window, and frees the callback context — strictly last, on the
/// engine thread. A callback-origin destroy (engine thread, inside a
/// callback frame) DEFERS via `close_deferred` per the no-re-entry rule;
/// otherwise it blocks on `close`.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeDestroy<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) {
    guard_unit(|| destroy_impl(engine));
}

fn destroy_impl(engine: jlong) {
    let Some(record) = registry().take(engine) else {
        return; // unknown or already destroyed
    };
    let EngineRecord {
        thread,
        engine: engine_ptr,
        ctx,
    } = record;
    let final_job = move || {
        // A Poisoned destroy means an internal panic left the engine (and any
        // EGL surface borrowing an owned window) live: releasing would dangle,
        // so the owned windows are released ONLY after an Ok destroy that
        // guarantees teardown (otherwise they leak — better than a UAF). The
        // callback context follows the same rule: a collaboration worker may
        // still hold its credential callbacks after a failed shutdown, so it
        // is freed only after Ok proves every worker was joined. On failure
        // both engine and context intentionally leak rather than risking UAF.
        let status = unsafe { op_destroy(engine_ptr.get()) };
        if matches!(status, OpStatus::Ok) {
            crate::window::release_all_windows();
            unsafe { drop_ctx(ctx.get()) };
        }
    };
    if thread.is_engine_thread() && crate::engine_thread::in_callback_frame() {
        thread.close_deferred(final_job);
    } else {
        thread.close(final_job);
    }
}

/// Dispatches `f` onto the handle's engine thread and returns its owned
/// result. `None` when the handle is unknown/tombstoned (no dispatch) or the
/// queue is closing — callers map that to `STATUS_CLOSING` / `null`.
pub(crate) fn with_engine<R: Send + 'static>(
    handle: jlong,
    f: impl FnOnce(*mut OpEngine) -> R + Send + 'static,
) -> Option<R> {
    // Clone the dispatch handle + engine pointer under the lock, then RELEASE
    // it before the blocking call — the engine thread must never contend for
    // the registry mutex while a native waits on it (callback re-entry).
    let (thread, engine) = registry().with(handle, |rec| (rec.thread.clone(), rec.engine))?;
    // A panicking engine job is re-raised on THIS (caller) thread by call();
    // catch it at the dispatch boundary so it never crosses the non-unwinding
    // JNI ABI and aborts the process. A panicked call maps to `None` (→
    // STATUS_CLOSING / null), like a closed queue.
    let dispatched = catch_unwind(AssertUnwindSafe(|| thread.call(move || f(engine.get()))));
    match dispatched {
        Ok(crate::Dispatch::Done(r)) => Some(r),
        Ok(crate::Dispatch::Closing) => None,
        Err(payload) => {
            // Guarded disposal: a panic_any payload whose own Drop panics
            // would otherwise re-panic across the JNI ABI.
            crate::engine_thread::drop_guarded(payload);
            None
        }
    }
}

/// Dispatches an engine call returning an `OpStatus`, mapped to the `jint`
/// the Kotlin contract expects (unknown/closing → `STATUS_CLOSING`).
pub(crate) fn call_status(
    handle: jlong,
    f: impl FnOnce(*mut OpEngine) -> OpStatus + Send + 'static,
) -> jint {
    with_engine(handle, move |e| f(e) as jint).unwrap_or(STATUS_CLOSING)
}

/// Reads a (non-null) `JString` into an owned `String`.
#[allow(dead_code)]
fn jstring_to_string(env: &mut JNIEnv, s: &JString) -> Option<String> {
    env.get_string(s).ok().map(|s| s.into())
}

/// Runs a `()`-returning native body under an unwind guard so no panic
/// crosses the non-unwinding `extern "system"` boundary.
fn guard_unit(f: impl FnOnce()) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(f)) {
        crate::engine_thread::drop_guarded(payload);
    }
}

/// Runs a `jstring`-returning native body under an unwind guard (panic →
/// null).
fn guard_jstring(f: impl FnOnce() -> jni::sys::jstring) -> jni::sys::jstring {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(s) => s,
        Err(payload) => {
            crate::engine_thread::drop_guarded(payload);
            ptr::null_mut()
        }
    }
}

// ---- Lifecycle natives ---------------------------------------------------

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeAttachSurface<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    surface: JObject<'local>,
) -> jint {
    attach_or_resume(&mut env, engine, surface, false)
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeResume<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    surface: JObject<'local>,
) -> jint {
    // A null Surface resume is invalid; a real Surface follows the
    // acquire-then-resume path identical to attach.
    if surface.is_null() {
        return STATUS_CLOSING;
    }
    attach_or_resume(&mut env, engine, surface, true)
}

/// Shared acquire → attach/resume → release-on-failure path: the caller
/// frame globalizes the Surface; the window is acquired and released ONLY
/// on the engine thread.
fn attach_or_resume(env: &mut JNIEnv, engine: jlong, surface: JObject, resume: bool) -> jint {
    let Ok(surface) = env.new_global_ref(&surface) else {
        return STATUS_CLOSING;
    };
    with_engine(engine, move |e| {
        let Some(vm) = VM.get() else {
            return OpStatus::InvalidArg as jint;
        };
        let Ok(jenv) = vm.get_env() else {
            return OpStatus::InvalidArg as jint;
        };
        // SAFETY: on the engine thread with its env; surface is a live global.
        let window = unsafe { crate::window::acquire(&jenv, surface.as_obj()) };
        if window.is_null() {
            return OpStatus::InvalidArg as jint;
        }
        let desc = OpSurfaceDesc {
            size: std::mem::size_of::<OpSurfaceDesc>(),
            handle: window.cast::<c_void>(),
        };
        let status = if resume {
            unsafe { op_resume(e, &desc) }
        } else {
            unsafe { op_attach_surface(e, &desc) }
        };
        match status {
            OpStatus::Ok => {
                // Clean transition: the engine dropped its old surface, so all
                // previously-owned windows are released and this becomes the
                // sole owned window.
                unsafe { crate::window::commit_window(window) };
            }
            _ => {
                // Clean failure: no EGL object survives, so release ONLY the
                // fresh acquisition; owned windows untouched.
                unsafe { crate::window::release(window) };
            }
        }
        status as jint
    })
    .unwrap_or(STATUS_CLOSING)
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeSuspend<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jint {
    with_engine(engine, move |e| {
        let status = unsafe { op_suspend(e) };
        // Release the owned windows ONLY after an Ok suspend that guarantees
        // the engine tore its EGL surface down synchronously. Any other
        // non-Ok keeps them (kept until destroy).
        if matches!(status, OpStatus::Ok) {
            crate::window::release_all_windows();
        }
        status as jint
    })
    .unwrap_or(STATUS_CLOSING)
}

/// `OpNative.nativeHasBackgroundWork` — whether this engine has a queued or
/// running user-started generation. Invalid/closing handles fail closed so the
/// Android foreground service never spins forever around a dead engine.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeHasBackgroundWork<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jboolean {
    let active = with_engine(engine, move |e| {
        let mut active = false;
        let status = unsafe { op_has_background_work(e, &mut active) };
        crate::background_work_or_false(status, active)
    })
    .unwrap_or(false);
    active as jboolean
}

/// `OpNative.nativeBackgroundTick` — advance chat/design/image-enrichment
/// work without touching the suspended EGL surface. The C ABI returns whether
/// another tick is needed; any failed/closing engine maps to `false` so service
/// teardown is bounded.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeBackgroundTick<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    now_ms: jlong,
) -> jboolean {
    let active = with_engine(engine, move |e| {
        let mut active = false;
        let status = unsafe { op_background_tick(e, now_ms.max(0) as u64, &mut active) };
        crate::background_work_or_false(status, active)
    })
    .unwrap_or(false);
    active as jboolean
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeResize<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    w: jfloat,
    h: jfloat,
    dpr: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe { op_resize(e, w, h, dpr) })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeResizeWithSafeArea<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    w: jfloat,
    h: jfloat,
    dpr: jfloat,
    t: jfloat,
    r: jfloat,
    b: jfloat,
    l: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_resize_with_safe_area(e, w, h, dpr, t, r, b, l)
    })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeSetSafeArea<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    t: jfloat,
    r: jfloat,
    b: jfloat,
    l: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe { op_set_safe_area(e, t, r, b, l) })
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeSetKeyboard<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    h: jfloat,
) -> jint {
    call_status(engine, move |e| unsafe { op_set_keyboard(e, h) })
}

/// `OpNative.nativePrefersLightSystemIcons` — whether platform bars should
/// use light-colored icons. Any invalid/closing/failed engine returns false.
#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativePrefersLightSystemIcons<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
) -> jboolean {
    let prefers_light = with_engine(engine, move |e| {
        let mut prefers_light = false;
        let status = unsafe { op_prefers_light_system_icons(e, &mut prefers_light) };
        crate::system_icon_preference_or_false(status, prefers_light)
    })
    .unwrap_or(false);
    prefers_light as jboolean
}

#[no_mangle]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativeFrame<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    t_ms: jlong,
) -> jint {
    call_status(engine, move |e| unsafe { op_frame(e, t_ms as u64) })
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_tech_zseven_openpencil_OpNative_nativePointer<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    engine: jlong,
    id: jint,
    phase: jint,
    x: jfloat,
    y: jfloat,
    t_ms: jlong,
) -> jint {
    call_status(engine, move |e| unsafe {
        op_pointer(e, id as u32, phase, x, y, t_ms as u64)
    })
}

// ---- Text editing + IME -------------------------------------------------

pub(crate) fn jstring_bytes(env: &mut JNIEnv, s: &JString) -> Option<Vec<u8>> {
    // `JavaStr::to_bytes()` exposes JNI Modified UTF-8/CESU-8, which is not
    // the UTF-8 every engine ABI string accepts (supplementary scalars such
    // as emoji become two invalid three-byte surrogate encodings). Decode the
    // Java string first, then own canonical UTF-8 before leaving this frame.
    env.get_string(s)
        .ok()
        .map(|value| String::from(value).into_bytes())
}
