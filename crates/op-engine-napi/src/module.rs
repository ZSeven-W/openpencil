//! Node-API module surface: the engine's callback object, the XComponent
//! lifecycle subscription, and the load-time hook that adopts the
//! `OH_NativeXComponent` ArkUI injects into this module's exports.

#![cfg(all(target_os = "linux", target_env = "ohos"))]

use napi_derive_ohos::{module_init, napi};
use napi_ohos::bindgen_prelude::{Env, Object, ToNapiValue};
use napi_ohos::threadsafe_function::ThreadsafeFunction;

use crate::callbacks::{InputFocusFn, NeedsRedrawFn, RemoteImageFn, RuntimeErrorFn};

/// The engine's upcall handlers, passed to `create`. Every handler is
/// optional: an absent one drops its events.
///
/// All four run on ArkUI's event loop (they are threadsafe functions queued
/// from the engine thread), NOT synchronously inside the engine call that
/// produced them.
// `object_to_js = false`: threadsafe functions can be READ from a JS object
// but never handed back out, so only the JS → Rust direction is generated.
#[napi(object, object_to_js = false)]
pub struct EngineCallbacks {
    /// `(hasNextWake, nextWakeMs)` — schedule the next `frame` pump.
    pub on_needs_redraw: Option<NeedsRedrawFn>,
    /// `(kind, message, source)` — a runtime diagnostic.
    pub on_runtime_error: Option<RuntimeErrorFn>,
    /// `(focused, inputKind, returnKeyHint)` — show/hide the IME.
    pub on_input_focus_changed: Option<InputFocusFn>,
    /// `(requestId, url)` — fetch the image and answer with
    /// `remoteImageResult`.
    pub on_remote_image_request: Option<RemoteImageFn>,
}

/// `(event, xcomponentId, width, height)`.
pub type XcomponentListener = ThreadsafeFunction<(String, String, f64, f64), ()>;

/// `setXcomponentListener` — subscribe to XComponent surface lifecycle
/// transitions as `(event, xcomponentId, width, height)`, where `event` is
/// `"created"`, `"changed"`, or `"destroyed"` and the size is in PHYSICAL
/// pixels. Pass `null` to unsubscribe.
///
/// This is the OHOS replacement for Android's `SurfaceHolder.Callback`: ArkTS
/// never sees the native window, so the shell reacts to `created` / `changed`
/// by calling `attachSurface` / `resume` / `resizeWithSafeArea` with the same
/// id.
///
/// `destroyed` is a NOTIFICATION, not a request: the binding has already
/// suspended any engine bound to that surface synchronously (the framework
/// only guarantees the window until its callback returns), so the shell must
/// not race to call `suspend` itself.
#[napi(js_name = "setXcomponentListener")]
pub fn set_xcomponent_listener(callback: Option<XcomponentListener>) {
    crate::window::set_listener(callback);
}

/// Adopts the XComponent the framework injected into `exports`.
///
/// # Safety
/// Called by Node-API with a live env and this module's exports object.
unsafe fn register(
    raw_env: napi_ohos::sys::napi_env,
    raw_exports: napi_ohos::sys::napi_value,
) -> napi_ohos::Result<()> {
    let env = Env::from_raw(raw_env);
    let exports = Object::from_raw(raw_env, raw_exports);
    // Absent when the library is imported before any XComponent names it —
    // not an error, just nothing to adopt yet.
    let Some(injected) = exports.get::<Object>("__NATIVE_XCOMPONENT_OBJ__")? else {
        return Ok(());
    };
    // SAFETY: `injected` is a live object in this env.
    let raw_injected = unsafe { ToNapiValue::to_napi_value(raw_env, injected)? };
    let mut unwrapped: *mut std::ffi::c_void = std::ptr::null_mut();
    // SAFETY: the framework wrapped an `OH_NativeXComponent*` in this object;
    // `napi_unwrap` returns it WITHOUT transferring ownership.
    let status = unsafe { napi_ohos::sys::napi_unwrap(env.raw(), raw_injected, &mut unwrapped) };
    if status != napi_ohos::sys::Status::napi_ok {
        crate::hilog::error("OpNapi", "could not unwrap the injected XComponent");
        return Ok(());
    }
    // SAFETY: `unwrapped` is the framework's live instance pointer.
    unsafe { crate::xcomponent::adopt(unwrapped.cast()) };
    Ok(())
}

#[module_init]
fn init() {
    napi_ohos::bindgen_prelude::register_module_exports(register)
}
