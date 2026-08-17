//! `OHNativeWindow` bookkeeping for the player's GPU surface.
//!
//! Ownership contract (the OHOS counterpart of
//! `op-engine-jni/src/window.rs`): an `OHNativeWindow*` is owned by the
//! XComponent, not by this layer — there is no `fromSurface`/`release`
//! reference-count pair to honour. What this module owns instead is the
//! BORROW: which window each XComponent id currently exposes, and which
//! engine (if any) has handed that window to `op_attach_surface`.
//!
//! The borrow is what makes `OnSurfaceDestroyed` safe. The framework
//! guarantees the window stays live only until that callback RETURNS, and the
//! ArkTS listener is notified asynchronously (a threadsafe function), so the
//! shell cannot suspend in time. This module therefore suspends the bound
//! engine synchronously, on its own thread, before the callback returns —
//! exactly the guarantee `op_engine.h` demands of the shell.
//!
//! Every recorded and cleared window emits a paired HiLog line so acceptance
//! scripts can assert leak-free pairing, mirroring the Android player.

#![cfg(all(target_os = "linux", target_env = "ohos"))]

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use napi_ohos::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};

/// Which lifecycle transition an XComponent reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceEvent {
    Created,
    Changed,
    Destroyed,
}

impl SurfaceEvent {
    /// The string the ArkTS listener receives.
    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceEvent::Created => "created",
            SurfaceEvent::Changed => "changed",
            SurfaceEvent::Destroyed => "destroyed",
        }
    }
}

/// One XComponent's current surface. The window is kept as a `usize` so the
/// table stays `Send`; it is only ever turned back into a pointer to hand to
/// `op_attach_surface` on the engine thread.
struct SurfaceSlot {
    window: usize,
    width: u64,
    height: u64,
    /// Engine handle currently borrowing `window` (`0` = nobody).
    engine: i64,
}

type Listener = ThreadsafeFunction<(String, String, f64, f64), ()>;

fn surfaces() -> &'static Mutex<HashMap<String, SurfaceSlot>> {
    static SURFACES: OnceLock<Mutex<HashMap<String, SurfaceSlot>>> = OnceLock::new();
    SURFACES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn listener() -> &'static Mutex<Option<Listener>> {
    static LISTENER: OnceLock<Mutex<Option<Listener>>> = OnceLock::new();
    LISTENER.get_or_init(|| Mutex::new(None))
}

/// Locks a table, RECOVERING from poison: these mutexes are taken from NAPI
/// entry points and from C callbacks, neither of which may panic, so a
/// poisoned lock must never turn into a second panic.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Installs (or clears, with `None`) the ArkTS surface-lifecycle listener.
pub fn set_listener(callback: Option<Listener>) {
    *lock(listener()) = callback;
}

/// Records a lifecycle transition reported by [`crate::xcomponent`].
///
/// On `Destroyed` the bound engine is suspended SYNCHRONOUSLY first (see the
/// module docs), then the slot is dropped; the ArkTS listener is notified
/// afterwards so the shell observes a surface that is already released.
pub fn record_surface_event(
    event: SurfaceEvent,
    id: &str,
    window: *mut c_void,
    width: u64,
    height: u64,
) {
    if matches!(event, SurfaceEvent::Destroyed) {
        let bound = lock(surfaces()).get(id).map_or(0, |slot| slot.engine);
        if bound != 0 {
            // Blocks on the engine thread's barrier; `op_suspend` tears the
            // EGL surface down before this callback returns to the framework.
            let status = crate::bindings::suspend_for_surface_loss(bound);
            crate::hilog::info(
                "OpNapi",
                &format!("surface {id} destroyed: engine {bound} suspended ({status})"),
            );
        }
        lock(surfaces()).remove(id);
        crate::hilog::info("OpNapi", &format!("window cleared {id} {window:p}"));
    } else {
        let mut table = lock(surfaces());
        let slot = table.entry(id.to_owned()).or_insert(SurfaceSlot {
            window: window as usize,
            width,
            height,
            engine: 0,
        });
        slot.window = window as usize;
        slot.width = width;
        slot.height = height;
        drop(table);
        crate::hilog::info(
            "OpNapi",
            &format!("window recorded {id} {window:p} {width}x{height}"),
        );
    }
    notify(event, id, width, height);
}

fn notify(event: SurfaceEvent, id: &str, width: u64, height: u64) {
    let guard = lock(listener());
    let Some(callback) = guard.as_ref() else {
        return;
    };
    callback.call(
        Ok((
            event.as_str().to_owned(),
            id.to_owned(),
            width as f64,
            height as f64,
        )),
        ThreadsafeFunctionCallMode::NonBlocking,
    );
}

/// The live window for an XComponent id, or `None` when no surface is
/// currently created for it.
pub fn window_for(id: &str) -> Option<*mut c_void> {
    let table = lock(surfaces());
    let slot = table.get(id)?;
    if slot.window == 0 {
        return None;
    }
    Some(slot.window as *mut c_void)
}

/// The physical pixel size the XComponent last reported, or `None`.
pub fn size_for(id: &str) -> Option<(u64, u64)> {
    let table = lock(surfaces());
    table.get(id).map(|slot| (slot.width, slot.height))
}

/// Records that `engine` now borrows the window behind `id` (called after a
/// successful `op_attach_surface` / `op_resume`).
pub fn bind_engine(id: &str, engine: i64) {
    if let Some(slot) = lock(surfaces()).get_mut(id) {
        slot.engine = engine;
    }
}

/// Drops every borrow held by `engine` (a confirmed suspend, or teardown).
pub fn unbind_engine(engine: i64) {
    for slot in lock(surfaces()).values_mut() {
        if slot.engine == engine {
            slot.engine = 0;
        }
    }
}
