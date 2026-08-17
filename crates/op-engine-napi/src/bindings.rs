//! NAPI entry points for `libopenpencil.so` — lifecycle, surface, and
//! pointer input.
//!
//! Every entry point validates its handle against the tombstoning
//! [`Registry`] first (a closed/unknown handle returns
//! [`STATUS_CLOSING`](crate::action::STATUS_CLOSING)), then dispatches the
//! engine work onto that engine's dedicated thread — engine pointers are only
//! ever dereferenced there. The ArkTS caller frame owns argument conversion;
//! owned results come back through the blocking barrier.
//!
//! Threading: NAPI entry points run on the ArkUI main (JS) thread and BLOCK
//! it for the duration of the engine call, exactly like the Android player's
//! `nativeFrame` barrier. Keep per-call work bounded; long-running work
//! belongs behind a shell-driven frame pump.

#![cfg(all(target_os = "linux", target_env = "ohos"))]

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::{Arc, OnceLock};

use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::Buffer;
use op_engine_ffi::{
    op_attach_surface, op_create, op_destroy, op_frame, op_get_pixel_size, op_pointer,
    op_prefers_light_system_icons, op_resize, op_resize_with_safe_area, op_resume, op_set_keyboard,
    op_set_safe_area, op_suspend, OpCreateDesc, OpEngine, OpStatus, OpSurfaceDesc,
};
use op_engine_jni::registry::{Registry, HANDLE_FAILURE};
use op_engine_jni::{Dispatch, EngineThread};

use crate::action::STATUS_CLOSING;
use crate::callbacks::{build_callbacks, drop_ctx, EngineCtx};

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
    /// Taking `self` by value makes a closure capture the whole (Send)
    /// wrapper, not the raw field.
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

/// Per-engine record. The thread is behind an `Arc` so an entry point can
/// clone the dispatch handle out from under the registry lock and release the
/// lock BEFORE the blocking `call()`.
struct EngineRecord {
    thread: Arc<EngineThread>,
    engine: EnginePtr,
    ctx: CtxPtr,
}

// ---- Shared dispatch helpers --------------------------------------------

/// Dispatches `f` onto the handle's engine thread and returns its owned
/// result. `None` when the handle is unknown/tombstoned (no dispatch) or the
/// queue is closing — callers map that to `STATUS_CLOSING` / `null`.
pub(crate) fn with_engine<R: Send + 'static>(
    handle: i64,
    f: impl FnOnce(*mut OpEngine) -> R + Send + 'static,
) -> Option<R> {
    // Clone the dispatch handle + engine pointer under the lock, then RELEASE
    // it before the blocking call — the engine thread must never contend for
    // the registry mutex while an entry point waits on it.
    let (thread, engine) = registry().with(handle, |rec| (rec.thread.clone(), rec.engine))?;
    // A panicking engine job is re-raised on THIS thread by call(); catch it
    // at the dispatch boundary so it never crosses the non-unwinding NAPI
    // trampoline. A panicked call maps to `None`, like a closed queue.
    let dispatched = catch_unwind(AssertUnwindSafe(|| thread.call(move || f(engine.get()))));
    match dispatched {
        Ok(Dispatch::Done(r)) => Some(r),
        Ok(Dispatch::Closing) => None,
        Err(payload) => {
            op_engine_jni::engine_thread::drop_guarded(payload);
            None
        }
    }
}

/// Dispatches an engine call returning an `OpStatus`, mapped to the `i32` the
/// ArkTS contract expects (unknown/closing → `STATUS_CLOSING`).
pub(crate) fn call_status(
    handle: i64,
    f: impl FnOnce(*mut OpEngine) -> OpStatus + Send + 'static,
) -> i32 {
    with_engine(handle, move |e| f(e) as i32).unwrap_or(STATUS_CLOSING)
}

/// Runs the two-pass copy-out ABI (`NULL/0` sizes the payload, then a sized
/// buffer receives it) for one of the engine's `*_copy_*` accessors, and
/// decodes the result as UTF-8. `None` whenever any step fails or the payload
/// is empty — the ArkTS side sees `null`.
///
/// Every consumer is an editor-mode accessor, so the viewer lane omits it.
#[cfg(feature = "editor")]
pub(crate) fn copy_out_string(
    handle: i64,
    accessor: unsafe extern "C" fn(*mut OpEngine, *mut u8, usize, *mut usize) -> OpStatus,
) -> Option<String> {
    let bytes = with_engine(handle, move |e| {
        let mut required = 0_usize;
        // SAFETY: the null/zero probe is the ABI's documented sizing call.
        let status = unsafe { accessor(e, ptr::null_mut(), 0, &mut required) };
        if status != OpStatus::Ok || required == 0 {
            return None;
        }
        let mut bytes = vec![0_u8; required];
        // SAFETY: `bytes` covers exactly the length the probe reported.
        let status = unsafe { accessor(e, bytes.as_mut_ptr(), bytes.len(), &mut required) };
        (status == OpStatus::Ok).then_some(bytes)
    })
    .flatten()?;
    String::from_utf8(bytes).ok()
}

// ---- Lifecycle -----------------------------------------------------------

/// `create` — spawns the engine thread and creates the engine ON that thread.
/// Returns the handle, or `0` on failure (the reason is readable via
/// `lastError(0)`).
///
/// `doc` may be null/empty in editor mode (`mode == 1`) to open the canonical
/// blank starter; viewer mode (`mode == 0`) always requires document bytes.
#[napi(js_name = "create")]
#[allow(clippy::too_many_arguments)]
pub fn create(
    doc: Option<Buffer>,
    w: f64,
    h: f64,
    dpr: f64,
    callbacks: crate::EngineCallbacks,
    storage_root: String,
    mode: i32,
) -> i64 {
    match catch_unwind(AssertUnwindSafe(|| {
        create_impl(doc, w, h, dpr, callbacks, storage_root, mode)
    })) {
        Ok(handle) => handle,
        Err(payload) => {
            op_engine_jni::engine_thread::drop_guarded(payload);
            registry().set_create_error("create panicked");
            HANDLE_FAILURE
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_impl(
    doc: Option<Buffer>,
    w: f64,
    h: f64,
    dpr: f64,
    callbacks: crate::EngineCallbacks,
    storage_root: String,
    mode: i32,
) -> i64 {
    let doc_bytes: Vec<u8> = doc.map(|buffer| buffer.to_vec()).unwrap_or_default();
    let ctx = Box::new(EngineCtx {
        needs_redraw: callbacks.on_needs_redraw,
        runtime_error: callbacks.on_runtime_error,
        input_focus_changed: callbacks.on_input_focus_changed,
        remote_image_request: callbacks.on_remote_image_request,
    });
    let thread = Arc::new(EngineThread::spawn("op-engine"));

    // Build the callback table and create the engine, both on the engine
    // thread: the `!Send` table never leaves it, and the context's disposal
    // stays on the thread that will run the teardown job.
    let created = thread.call(move || {
        let (callbacks, ctx_ptr) = build_callbacks(ctx);
        let mut engine: *mut OpEngine = ptr::null_mut();
        let (doc_ptr, doc_len) = if doc_bytes.is_empty() {
            (ptr::null(), 0)
        } else {
            (doc_bytes.as_ptr(), doc_bytes.len())
        };
        let desc = OpCreateDesc {
            size: std::mem::size_of::<OpCreateDesc>(),
            doc_ptr,
            doc_len,
            width: w as f32,
            height: h as f32,
            dpr: dpr as f32,
            callbacks: &callbacks,
            asset_base_ptr: ptr::null(),
            asset_base_len: 0,
            mode,
            storage_root_ptr: storage_root.as_ptr(),
            storage_root_len: storage_root.len(),
        };
        // SAFETY: every borrowed range above outlives this call.
        let status = unsafe { op_create(&desc, &mut engine) };
        (status as i32, engine as usize, ctx_ptr as usize)
    });

    let (status, engine_raw, ctx_raw) = match created {
        Dispatch::Done(v) => v,
        Dispatch::Closing => {
            thread.close(|| {});
            registry().set_create_error("engine thread closed during create");
            return HANDLE_FAILURE;
        }
    };

    if status != 0 || engine_raw == 0 {
        // Free the context on the engine thread, then tear down.
        let ctx = CtxPtr(ctx_raw as *mut EngineCtx);
        // SAFETY: the pointer came from `build_callbacks` and is freed once.
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

/// `lastError` — the last error text for a handle (or the create-failure text
/// for handle `0`); empty for an unknown handle.
#[napi(js_name = "lastError")]
pub fn last_error(engine: i64) -> String {
    registry().last_error(engine)
}

/// `destroy` — teardown. Tombstones the handle, then closes the engine thread
/// with a final job that destroys the engine, drops its surface borrows, and
/// frees the callback context — strictly last, on the engine thread. A
/// callback-origin destroy DEFERS per the no-re-entry rule.
#[napi(js_name = "destroy")]
pub fn destroy(engine: i64) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(|| destroy_impl(engine))) {
        op_engine_jni::engine_thread::drop_guarded(payload);
    }
}

fn destroy_impl(handle: i64) {
    let Some(record) = registry().take(handle) else {
        return; // unknown or already destroyed
    };
    let EngineRecord {
        thread,
        engine: engine_ptr,
        ctx,
    } = record;
    let final_job = move || {
        // A Poisoned destroy means an internal panic left the engine (and any
        // GPU surface borrowing the XComponent's window) live: dropping the
        // borrow record would let the shell tear the surface down under a
        // live engine, so the borrow and the context are released ONLY after
        // an Ok destroy. On failure both intentionally leak rather than
        // risking a use-after-free.
        // SAFETY: the handle was taken from the registry, so no other caller
        // can dispatch onto this engine any more.
        let status = unsafe { op_destroy(engine_ptr.get()) };
        if matches!(status, OpStatus::Ok) {
            crate::window::unbind_engine(handle);
            // SAFETY: the context outlived every callback; freed once here.
            unsafe { drop_ctx(ctx.get()) };
        }
    };
    if thread.is_engine_thread() && op_engine_jni::engine_thread::in_callback_frame() {
        thread.close_deferred(final_job);
    } else {
        thread.close(final_job);
    }
}

// ---- Surface -------------------------------------------------------------

/// `attachSurface` — hand the engine the `OHNativeWindow` currently exposed
/// by the XComponent with this ArkTS `id`.
///
/// This is where the OHOS contract differs from Android: there is no
/// `Surface` object to pass, so the shell names the XComponent and the
/// binding resolves the window recorded by `OnSurfaceCreated`. Returns
/// `OpStatus::InvalidArg` when the id has no live surface.
#[napi(js_name = "attachSurface")]
pub fn attach_surface(engine: i64, xcomponent_id: String) -> i32 {
    attach_or_resume(engine, xcomponent_id, false)
}

/// `resume` — re-acquire the GPU surface after a suspend. Passing `null`
/// resumes without a surface change is NOT supported (mirroring the Android
/// player, which rejects a null Surface): an absent id returns
/// `STATUS_CLOSING`.
#[napi(js_name = "resume")]
pub fn resume(engine: i64, xcomponent_id: Option<String>) -> i32 {
    let Some(id) = xcomponent_id else {
        return STATUS_CLOSING;
    };
    attach_or_resume(engine, id, true)
}

/// Shared resolve → attach/resume → bind path. The window pointer is read on
/// the calling (main) thread — where the XComponent callbacks also run, so
/// the read cannot race a surface change — and handed to the engine thread.
fn attach_or_resume(engine: i64, xcomponent_id: String, resuming: bool) -> i32 {
    let Some(window) = crate::window::window_for(&xcomponent_id) else {
        crate::hilog::error(
            "OpNapi",
            &format!("attach: no live surface for XComponent '{xcomponent_id}'"),
        );
        return OpStatus::InvalidArg as i32;
    };
    let window = window as usize;
    let status = with_engine(engine, move |e| {
        let desc = OpSurfaceDesc {
            size: std::mem::size_of::<OpSurfaceDesc>(),
            handle: window as *mut c_void,
        };
        // SAFETY: the window is owned by the XComponent and stays live until
        // `OnSurfaceDestroyed`, which suspends this engine before returning.
        let status = if resuming {
            unsafe { op_resume(e, &desc) }
        } else {
            unsafe { op_attach_surface(e, &desc) }
        };
        status as i32
    })
    .unwrap_or(STATUS_CLOSING);
    if status == OpStatus::Ok as i32 {
        crate::window::bind_engine(&xcomponent_id, engine);
    }
    status
}

/// `suspend` — blocking barrier: the engine drops its GPU surface before this
/// returns, so the shell may then let the XComponent tear the window down.
#[napi(js_name = "suspend")]
pub fn suspend(engine: i64) -> i32 {
    let status = with_engine(engine, move |e| {
        // SAFETY: dispatched onto the engine's owner thread.
        (unsafe { op_suspend(e) }) as i32
    })
    .unwrap_or(STATUS_CLOSING);
    if status == OpStatus::Ok as i32 {
        crate::window::unbind_engine(engine);
    }
    status
}

/// Suspends an engine that is losing its window inside `OnSurfaceDestroyed`.
/// Separate from [`suspend`] only because the borrow record is cleared by the
/// caller (which already holds the surface table).
pub(crate) fn suspend_for_surface_loss(engine: i64) -> i32 {
    suspend(engine)
}

// ---- Viewport ------------------------------------------------------------

#[napi(js_name = "resize")]
pub fn resize(engine: i64, w: f64, h: f64, dpr: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_resize(e, w as f32, h as f32, dpr as f32)
    })
}

/// `resizeWithSafeArea` — atomic viewport + DPR + safe-area update. Rotation
/// and configuration changes MUST use this rather than separate calls.
#[napi(js_name = "resizeWithSafeArea")]
#[allow(clippy::too_many_arguments)]
pub fn resize_with_safe_area(
    engine: i64,
    w: f64,
    h: f64,
    dpr: f64,
    t: f64,
    r: f64,
    b: f64,
    l: f64,
) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_resize_with_safe_area(
            e, w as f32, h as f32, dpr as f32, t as f32, r as f32, b as f32, l as f32,
        )
    })
}

#[napi(js_name = "setSafeArea")]
pub fn set_safe_area(engine: i64, t: f64, r: f64, b: f64, l: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_set_safe_area(e, t as f32, r as f32, b as f32, l as f32)
    })
}

#[napi(js_name = "setKeyboard")]
pub fn set_keyboard(engine: i64, h: f64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe { op_set_keyboard(e, h as f32) })
}

/// `prefersLightSystemIcons` — whether the status/navigation bars should use
/// light-colored icons. Any invalid/closing/failed engine returns false.
#[napi(js_name = "prefersLightSystemIcons")]
pub fn prefers_light_system_icons(engine: i64) -> bool {
    with_engine(engine, move |e| {
        let mut prefers_light = false;
        // SAFETY: dispatched onto the engine's owner thread.
        let status = unsafe { op_prefers_light_system_icons(e, &mut prefers_light) };
        crate::system_icon_preference_or_false(status, prefers_light)
    })
    .unwrap_or(false)
}

/// `pixelSize` — the current physical pixel dimensions as `[width, height]`,
/// or an empty array when they cannot be read. Lets the shell verify the DPR
/// it passed produced the backing store the XComponent reports.
#[napi(js_name = "pixelSize")]
pub fn pixel_size(engine: i64) -> Vec<u32> {
    with_engine(engine, move |e| {
        let (mut width, mut height) = (0_u32, 0_u32);
        // SAFETY: dispatched onto the engine's owner thread.
        let status = unsafe { op_get_pixel_size(e, &mut width, &mut height) };
        if status == OpStatus::Ok {
            vec![width, height]
        } else {
            Vec::new()
        }
    })
    .unwrap_or_default()
}

// ---- Frame + pointer -----------------------------------------------------

/// `frame` — blocking barrier that pumps and presents ONE frame; the return
/// value is the TRUE frame status. Drive it from an ArkUI vsync callback.
#[napi(js_name = "frame")]
pub fn frame(engine: i64, t_ms: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe { op_frame(e, t_ms as u64) })
}

/// `pointer` — one raw pointer event in `OpPointerPhase` terms (0 = down,
/// 1 = move, 2 = up, 3 = cancel). Prefer [`touch_event`] when forwarding
/// ArkUI touches, whose enum ordering differs.
#[napi(js_name = "pointer")]
pub fn pointer(engine: i64, id: i32, phase: i32, x: f64, y: f64, t_ms: i64) -> i32 {
    // SAFETY: dispatched onto the engine's owner thread.
    call_status(engine, move |e| unsafe {
        op_pointer(e, id as u32, phase, x as f32, y as f32, t_ms as u64)
    })
}

/// `touchEvent` — one ArkUI touch, translated from `TouchType` to
/// `OpPointerPhase`. `id` is the ArkUI finger id, so multi-finger gestures
/// (pinch) work by forwarding every changed touch. An unknown touch type is
/// dropped with `OpStatus::InvalidArg` rather than guessing a phase.
#[napi(js_name = "touchEvent")]
pub fn touch_event(engine: i64, id: i32, touch_type: i32, x: f64, y: f64, t_ms: i64) -> i32 {
    let Some(phase) = crate::action::pointer_phase_from_touch_type(touch_type) else {
        return OpStatus::InvalidArg as i32;
    };
    pointer(engine, id, phase, x, y, t_ms)
}
