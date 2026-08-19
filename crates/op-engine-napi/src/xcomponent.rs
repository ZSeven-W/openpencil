//! Hand-rolled OpenHarmony `OH_NativeXComponent` bindings + the module-load
//! hook that adopts the XComponent the ArkUI framework injects into this
//! module's exports.
//!
//! This is the OHOS analogue of the Android player's `Surface` plumbing, and
//! it is the ONLY place raw NDK declarations live (mirroring how
//! `op-engine-jni/src/window.rs` is the only place `ANativeWindow` is
//! declared). Every declaration here is transcribed from
//! `native_interface_xcomponent.h` in `openharmony/arkui_ace_engine`; the
//! surface-lifecycle callback table and the two accessors below are the whole
//! NDK footprint, so no `*-sys` crate is pulled in.
//!
//! Ownership: unlike Android's `ANativeWindow_fromSurface`, an
//! `OHNativeWindow*` handed to `OnSurfaceCreated` is NOT reference-counted by
//! the shell — the XComponent owns it and guarantees it stays live until
//! `OnSurfaceDestroyed` returns. The window is therefore recorded, never
//! acquired/released; see [`crate::window`] for the borrow bookkeeping the
//! engine still needs.
//!
//! Threading: every callback here runs on the ArkUI main (JS) thread, the
//! same thread the NAPI entry points run on.

#![cfg(all(target_os = "linux", target_env = "ohos"))]

use std::ffi::{c_char, c_void, CStr};

use crate::window::{self, SurfaceEvent};

/// Opaque `OH_NativeXComponent` instance.
#[repr(C)]
pub struct OhNativeXComponent {
    _private: [u8; 0],
}

/// `OH_NativeXComponent_Callback` — the surface lifecycle + touch dispatch
/// table. Touch dispatch is intentionally left `None`: the ArkTS shell drives
/// input through the exported `touchEvent` / `pointer` / `editor*` functions
/// (exactly like the Kotlin player), so a native touch path would double
/// every gesture.
#[repr(C)]
pub struct OhNativeXComponentCallback {
    pub on_surface_created: Option<extern "C" fn(*mut OhNativeXComponent, *mut c_void)>,
    pub on_surface_changed: Option<extern "C" fn(*mut OhNativeXComponent, *mut c_void)>,
    pub on_surface_destroyed: Option<extern "C" fn(*mut OhNativeXComponent, *mut c_void)>,
    pub dispatch_touch_event: Option<extern "C" fn(*mut OhNativeXComponent, *mut c_void)>,
}

/// `OH_XCOMPONENT_ID_LEN_MAX` — the id buffer must hold one more byte for the
/// terminating NUL.
const XCOMPONENT_ID_LEN_MAX: usize = 128;

/// `OH_NATIVEXCOMPONENT_RESULT_SUCCESS`.
const RESULT_SUCCESS: i32 = 0;

// `libace_ndk.z.so` ships the XComponent NDK surface on every OHOS API level.
#[link(name = "ace_ndk.z")]
extern "C" {
    fn OH_NativeXComponent_GetXComponentId(
        component: *mut OhNativeXComponent,
        id: *mut c_char,
        size: *mut u64,
    ) -> i32;

    fn OH_NativeXComponent_GetXComponentSize(
        component: *mut OhNativeXComponent,
        window: *const c_void,
        width: *mut u64,
        height: *mut u64,
    ) -> i32;

    fn OH_NativeXComponent_RegisterCallback(
        component: *mut OhNativeXComponent,
        callback: *mut OhNativeXComponentCallback,
    ) -> i32;

    fn OH_NativeXComponent_RegisterKeyEventCallback(
        component: *mut OhNativeXComponent,
        callback: extern "C" fn(*mut OhNativeXComponent, *mut c_void),
    ) -> i32;

    fn OH_NativeXComponent_GetKeyEvent(
        component: *mut OhNativeXComponent,
        event: *mut *mut OhNativeXComponentKeyEvent,
    ) -> i32;

    fn OH_NativeXComponent_GetKeyEventAction(
        event: *mut OhNativeXComponentKeyEvent,
        action: *mut i32,
    ) -> i32;

    fn OH_NativeXComponent_GetKeyEventCode(
        event: *mut OhNativeXComponentKeyEvent,
        code: *mut i32,
    ) -> i32;
}

/// Opaque `OH_NativeXComponent_KeyEvent` (native_xcomponent_key_event.h).
#[repr(C)]
pub struct OhNativeXComponentKeyEvent {
    _private: [u8; 0],
}

/// `OH_NativeXComponent_KeyAction` (native_xcomponent_key_event.h):
/// UNKNOWN = -1, DOWN = 0, UP = 1.
const KEY_ACTION_DOWN: i32 = 0;

/// SURFACE-type XComponents deliver hardware keys on the NATIVE channel
/// only (ArkTS `onKeyEvent` never fires while the surface holds focus), so
/// the key path lives here. Modifier state is tracked from the raw stream;
/// keys without an editor binding fall through to the IME as text.
/// Live hardware-modifier bitmask (this crate's MOD_* bits), maintained from
/// the native key stream and readable by the shell (wheel-zoom needs Ctrl).
static MODIFIERS: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// Whether the ArkTS `ImeConduit` currently holds a system
/// `InputMethodController`. Physical printable keys are injected as engine
/// text ONLY while it does not — otherwise the same keystroke would arrive
/// twice (once here, once as the conduit's `insertText`).
static IME_CONDUIT_ATTACHED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Last logged key-routing signature; `-1` until the first key arrives so the
/// very first keystroke always produces a line. See [`probe_key`].
static KEY_PROBE_SIGNATURE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// The current modifier bitmask.
pub fn current_modifiers() -> i32 {
    MODIFIERS.load(std::sync::atomic::Ordering::Relaxed)
}

/// The ArkTS shell reports its IME-conduit attachment state here; see
/// [`IME_CONDUIT_ATTACHED`].
pub fn set_ime_conduit_attached(attached: bool) {
    IME_CONDUIT_ATTACHED.store(attached, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the ArkTS IME conduit is attached.
pub fn ime_conduit_attached() -> bool {
    IME_CONDUIT_ATTACHED.load(std::sync::atomic::Ordering::Relaxed)
}

extern "C" fn on_key_event(component: *mut OhNativeXComponent, _window: *mut c_void) {
    use std::sync::atomic::Ordering;

    let mut event: *mut OhNativeXComponentKeyEvent = std::ptr::null_mut();
    // SAFETY: the component pointer is live for the duration of the callback.
    if unsafe { OH_NativeXComponent_GetKeyEvent(component, &mut event) } != RESULT_SUCCESS
        || event.is_null()
    {
        return;
    }
    let (mut action, mut code) = (-1_i32, -1_i32);
    // SAFETY: `event` was just produced by the NDK for this callback.
    unsafe {
        let _ = OH_NativeXComponent_GetKeyEventAction(event, &mut action);
        let _ = OH_NativeXComponent_GetKeyEventCode(event, &mut code);
    }
    let down = action == KEY_ACTION_DOWN;
    // KeyCode transcription (oh_key_code.h): shift 2047/2048, ctrl 2072/2073,
    // alt 2045/2046, meta 2076/2077.
    let modifier_bit = match code {
        2047 | 2048 => crate::action::MOD_SHIFT,
        2072 | 2073 => crate::action::MOD_CTRL,
        2045 | 2046 => crate::action::MOD_ALT,
        2076 | 2077 => crate::action::MOD_META,
        _ => 0,
    };
    if modifier_bit != 0 {
        if down {
            MODIFIERS.fetch_or(modifier_bit, Ordering::Relaxed);
        } else {
            MODIFIERS.fetch_and(!modifier_bit, Ordering::Relaxed);
        }
        return;
    }
    if !down {
        return;
    }
    #[cfg(feature = "editor")]
    {
        let engine = crate::window::engine_for(&component_id(component));
        if engine == 0 {
            return;
        }
        let modifiers = MODIFIERS.load(Ordering::Relaxed);
        // TEXT FIRST while an engine input holds the IME and no system
        // conduit is attached. On desktop-class HarmonyOS the XComponent
        // consumes hardware keys here, so the system IME never sees them and
        // never emits `insertText` — this is the only path a typed character
        // can take into a focused chat / property / canvas-text input. It
        // also keeps `[` / `]` from reordering layers mid-word, since those
        // have an unconditional editor binding in `map_key`.
        let ime = crate::bindings_editor::editor_ime_focused(engine);
        let conduit = IME_CONDUIT_ATTACHED.load(Ordering::Relaxed);
        if ime && !conduit {
            if let Some(ch) = crate::action::printable_char(code, modifiers) {
                let status = crate::bindings_editor::editor_text(engine, ch.to_string());
                probe_key(code, modifiers, ime, conduit, "text", status);
                return;
            }
        }
        // Unmapped keys return InvalidArg — nothing else can consume them.
        let status = crate::bindings_input::key_event(engine, code, modifiers);
        probe_key(code, modifiers, ime, conduit, "key", status);
    }
    #[cfg(not(feature = "editor"))]
    {
        let _ = (code, &MODIFIERS);
    }
}

/// Logs the FIRST key that reaches the native channel, and afterwards only a
/// key whose ROUTING changed (IME focus gained/lost, conduit attach/detach,
/// text-vs-editor-key branch, success-vs-failure).
///
/// Diagnosing this lane needs proof that (a) hardware keys arrive at all and
/// (b) which branch claimed them. A line per keystroke answers both but
/// rotates hilog's buffer within seconds of typing, so the signature filter
/// keeps the same evidence at a bounded volume.
#[cfg(feature = "editor")]
fn probe_key(code: i32, modifiers: i32, ime: bool, conduit: bool, route: &str, status: i32) {
    use std::sync::atomic::Ordering;
    let signature = (ime as i32)
        | (conduit as i32) << 1
        | ((route == "text") as i32) << 2
        | ((status != 0) as i32) << 3;
    if KEY_PROBE_SIGNATURE.swap(signature, Ordering::Relaxed) == signature {
        return;
    }
    crate::hilog::info(
        "OpNapi",
        &format!(
            "native key code={code} mods={modifiers} imeFocused={ime} \
             conduit={conduit} route={route} status={status}"
        ),
    );
}

/// The callback table handed to `OH_NativeXComponent_RegisterCallback`. The
/// NDK BORROWS this pointer for the XComponent's whole lifetime, so it must
/// outlive every component — a `static mut` is the standard shape here, and
/// it is only ever read (the contents are a compile-time constant).
static mut CALLBACKS: OhNativeXComponentCallback = OhNativeXComponentCallback {
    on_surface_created: Some(on_surface_created),
    on_surface_changed: Some(on_surface_changed),
    on_surface_destroyed: Some(on_surface_destroyed),
    dispatch_touch_event: None,
};

/// Reads an XComponent's ArkTS-declared `id`. Empty when the NDK refuses or
/// the id is not valid UTF-8 — the caller then drops the component rather
/// than binding it to an unnameable slot.
fn component_id(component: *mut OhNativeXComponent) -> String {
    let mut buffer = [0 as c_char; XCOMPONENT_ID_LEN_MAX + 1];
    let mut length = XCOMPONENT_ID_LEN_MAX as u64;
    // SAFETY: `buffer` has room for `length` bytes plus the NUL the NDK
    // appends; `component` is the live pointer the framework handed us.
    let status =
        unsafe { OH_NativeXComponent_GetXComponentId(component, buffer.as_mut_ptr(), &mut length) };
    if status != RESULT_SUCCESS {
        return String::new();
    }
    // SAFETY: on success the NDK wrote a NUL-terminated id into `buffer`.
    let id = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    id.to_str().unwrap_or_default().to_owned()
}

/// The surface's physical pixel size, or `(0, 0)` when the NDK refuses.
fn component_size(component: *mut OhNativeXComponent, window: *mut c_void) -> (u64, u64) {
    let (mut width, mut height) = (0_u64, 0_u64);
    // SAFETY: both pointers came from the framework and are live for the
    // duration of the callback that is asking.
    let status = unsafe {
        OH_NativeXComponent_GetXComponentSize(component, window, &mut width, &mut height)
    };
    if status == RESULT_SUCCESS {
        (width, height)
    } else {
        (0, 0)
    }
}

extern "C" fn on_surface_created(component: *mut OhNativeXComponent, window: *mut c_void) {
    surface_event(SurfaceEvent::Created, component, window);
}

extern "C" fn on_surface_changed(component: *mut OhNativeXComponent, window: *mut c_void) {
    surface_event(SurfaceEvent::Changed, component, window);
}

extern "C" fn on_surface_destroyed(component: *mut OhNativeXComponent, window: *mut c_void) {
    surface_event(SurfaceEvent::Destroyed, component, window);
}

/// Shared body for the three lifecycle callbacks: record (or clear) the
/// window for the component's id and notify the ArkTS listener. Guarded so a
/// panic can never unwind across the non-unwinding C callback ABI.
fn surface_event(event: SurfaceEvent, component: *mut OhNativeXComponent, window: *mut c_void) {
    let guarded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id = component_id(component);
        if id.is_empty() {
            crate::hilog::error("OpNapi", "XComponent surface event with an unreadable id");
            return;
        }
        let (width, height) = component_size(component, window);
        window::record_surface_event(event, &id, window, width, height);
    }));
    if let Err(payload) = guarded {
        op_engine_jni::engine_thread::drop_guarded(payload);
    }
}

/// Adopts the `OH_NativeXComponent` ArkUI injects into this module's exports
/// under `__NATIVE_XCOMPONENT_OBJ__` and registers the lifecycle callbacks on
/// it. Called once per module load — ArkUI re-runs module registration for
/// each XComponent that names this library, so several components can be
/// adopted over the app's life.
///
/// A module load with no XComponent (the app importing the library before any
/// `XComponent` is mounted) is NOT an error: the function simply returns.
///
/// # Safety
/// `component` must be the pointer unwrapped from the framework-provided
/// export object.
pub unsafe fn adopt(component: *mut OhNativeXComponent) {
    if component.is_null() {
        return;
    }
    // A raw pointer to the static needs no `unsafe`: nothing is read through
    // it here. CALLBACKS is written once at compile time and only ever read,
    // and the NDK borrows the pointer for the component's lifetime, which the
    // `static` outlives.
    let table = &raw mut CALLBACKS;
    // SAFETY: `component` is the framework's live instance.
    let status = unsafe { OH_NativeXComponent_RegisterCallback(component, table) };
    // SAFETY: same component pointer; the fn pointer is 'static.
    let key_status =
        unsafe { OH_NativeXComponent_RegisterKeyEventCallback(component, on_key_event) };
    if key_status != RESULT_SUCCESS {
        crate::hilog::error(
            "OpNapi",
            &format!("RegisterKeyEventCallback failed ({key_status})"),
        );
    }
    if status == RESULT_SUCCESS {
        crate::hilog::info("OpNapi", "XComponent callbacks registered");
    } else {
        crate::hilog::error(
            "OpNapi",
            &format!("OH_NativeXComponent_RegisterCallback failed ({status})"),
        );
    }
}
