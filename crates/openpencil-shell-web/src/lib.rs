//! OpenPencil shell — web bundle entry.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate is the web bundle
//! entry. CI invariant requires `cargo check --target wasm32-unknown-
//! unknown -p openpencil-shell-web --no-default-features --features web`
//! to pass on every PR — that path uses the **stub** mount entry below
//! and is purely a wasm32-clean compile guard (no skia, no real render).
//!
//! Phase A onward enables `--features skia` and targets
//! `wasm32-unknown-unknown` via the C-hard pipeline (vendor/skia-safe-op
//! fork + crates/wasm-libc-shim). The same target serves both the
//! compile-guard CI baseline and the real render path; the only
//! difference is the `skia` feature flag and the EMSDK env var (used
//! at build time only — for libcxx headers + emsdk's wasm-aware clang
//! — never linked into the final bundle).

#[cfg(feature = "skia")]
mod backend;
pub mod event;
#[cfg(feature = "skia")]
mod widget_host;

// Force the wasm32-unknown-unknown libc/libcxx/libm shim to be linked
// even though no Rust code calls it — its `#[no_mangle]` symbols are
// referenced only by the C++ side of the wasm (Skia static lib). Without
// this `extern crate`, cargo would dead-code-eliminate the shim because
// no Rust path imports anything from it.
#[cfg(all(feature = "skia", target_arch = "wasm32", target_os = "unknown"))]
extern crate wasm_libc_shim as _;

use wasm_bindgen::prelude::*;

#[cfg(feature = "skia")]
use std::cell::RefCell;
#[cfg(feature = "skia")]
use std::rc::Rc;

#[cfg(feature = "skia")]
use openpencil_shell_core::Modifiers;

/// Long-lived shell handle. The smoke HTML must keep this alive (e.g.
/// `window.__opShell = mount("op")`) so closures stored on the shell
/// remain reachable for the page lifetime.
///
/// The stub variant (without `skia` feature) carries no fields and exists
/// only so the wasm32-unknown-unknown CI baseline can compile-check the
/// public surface.
#[wasm_bindgen]
pub struct WebShell {
    /// Shared inner state. `Rc<RefCell<...>>` is necessary because every
    /// browser closure registered in `mount()` needs its own owned
    /// reference to the backend + host (`'static` is required by
    /// wasm-bindgen's `Closure::new`); each closure clones the Rc and
    /// borrow_muts on dispatch. `WebShell` itself never reads back
    /// through this field directly (all access goes through closure
    /// clones); the field exists to anchor the original ownership so
    /// the Rc is not dropped before `Drop` removes the listeners.
    #[cfg(feature = "skia")]
    #[allow(dead_code)]
    inner: Rc<RefCell<Inner>>,

    /// Registered DOM event listeners. Kept alive for the WebShell's
    /// lifetime so the closures the browser holds remain valid; the
    /// `Drop` impl below removes each listener on shell drop so the
    /// page can navigate away cleanly without dangling JS callbacks.
    /// Codex Phase C2 R1 will exercise this end-to-end.
    #[cfg(feature = "skia")]
    listeners: Vec<Listener>,
}

#[cfg(feature = "skia")]
struct Inner {
    backend: backend::WebBackend,
    host: widget_host::WidgetHost,
}

#[cfg(feature = "skia")]
struct Listener {
    target: web_sys::EventTarget,
    name: &'static str,
    /// Type-erased Closure storage. We use
    /// `Closure<dyn FnMut(JsValue)>` uniformly across event types and
    /// runtime-checked `dyn_into::<SpecificEvent>()` inside each
    /// handler body so `Listener` carries one concrete generic
    /// argument across the entire vec; mismatched synthetic events
    /// from same-page JS skip the handler instead of producing a
    /// wrong-type reference (codex C2.2 R1 CONCERN-3).
    closure: Closure<dyn FnMut(JsValue)>,
}

#[cfg(feature = "skia")]
impl Inner {
    /// Phase B inspector paint. Called from `mount()` for the first
    /// frame and from every closure body after a state mutation.
    /// Returns the present error if the ImageData round-trip failed.
    fn repaint(&mut self) -> Result<(), JsValue> {
        use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend};

        self.backend.begin_frame();
        // Clear to white so widget paints sit on a clean background.
        self.backend.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(960.0, 640.0),
            },
            Color::WHITE,
        );
        // Inspector slice: 280 px wide column on the left.
        self.host.paint(&mut self.backend, 280.0);
        self.backend.end_frame();
        if let Some(err) = self.backend.take_present_error() {
            return Err(err);
        }
        Ok(())
    }
}

/// Build a Jian `Modifiers` bitset from a W3C `KeyboardEvent`. Mirrors
/// the four standard modifier keys; per spec §2.4 we treat
/// `metaKey` (browser) → `Modifiers::CMD` (Jian) — both name the
/// "Cmd on macOS / Win key on Windows / Super on Linux" modifier.
#[cfg(feature = "skia")]
fn modifiers_from_keyboard(event: &web_sys::KeyboardEvent) -> Modifiers {
    let mut m = Modifiers::empty();
    if event.shift_key() {
        m |= Modifiers::SHIFT;
    }
    if event.ctrl_key() {
        m |= Modifiers::CTRL;
    }
    if event.alt_key() {
        m |= Modifiers::ALT;
    }
    if event.meta_key() {
        m |= Modifiers::CMD;
    }
    m
}

/// Register a JS event listener on `target` and store the Closure in
/// `listeners` so it stays alive for the WebShell's lifetime. The
/// helper accepts an `FnMut(SpecificEvent)` and adapts it to the
/// type-erased `Closure<dyn FnMut(JsValue)>` stored in `Listener`.
#[cfg(feature = "skia")]
fn add_listener<E, F>(
    target: &web_sys::EventTarget,
    name: &'static str,
    listeners: &mut Vec<Listener>,
    mut handler: F,
) -> Result<(), JsValue>
where
    E: wasm_bindgen::JsCast + 'static,
    F: FnMut(E) + 'static,
{
    let closure: Closure<dyn FnMut(JsValue)> = Closure::new(move |raw: JsValue| {
        // Use `dyn_into` (runtime-checked) instead of
        // `unchecked_into` so a mismatched synthetic event dispatched
        // by same-page JS (e.g. `new Event("keydown")` rather than
        // `new KeyboardEvent("keydown")`) silently skips the handler
        // instead of producing a wrong-type reference whose getter
        // calls would return garbage. Codex C2.2 R1 CONCERN-3.
        let event: E = match raw.dyn_into::<E>() {
            Ok(e) => e,
            Err(_) => return,
        };
        handler(event);
    });
    target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())?;
    listeners.push(Listener {
        target: target.clone(),
        name,
        closure,
    });
    Ok(())
}

#[cfg(feature = "skia")]
impl Drop for WebShell {
    fn drop(&mut self) {
        for l in self.listeners.drain(..) {
            // Best-effort removal; if the target has already been
            // detached from the DOM the call is a no-op. The Closure
            // is dropped at end of scope, releasing its wasm-bindgen
            // closure slot back to the wasm linear memory pool.
            let _ = l
                .target
                .remove_event_listener_with_callback(l.name, l.closure.as_ref().unchecked_ref());
        }
    }
}

/// Mount the WebShell on the canvas identified by `canvas_id` in the host
/// document. Returns the live shell instance to the caller; the caller
/// MUST keep it alive (`window.__opShell = mount("op")`).
///
/// Errors propagate back to JS as a `JsValue` exception.
///
/// Without the `skia` feature this is a stub that returns the
/// fields-less `WebShell` after validating the canvas element exists
/// — useful only for the kickoff §1.2 wasm32-clean compile guard CI.
#[cfg(feature = "skia")]
#[wasm_bindgen]
pub fn mount(canvas_id: &str) -> Result<WebShell, JsValue> {
    use crate::event::{ime, keyboard};
    use wasm_bindgen::JsCast;
    use web_sys::{CompositionEvent, HtmlCanvasElement, KeyboardEvent};

    // Install the panic hook on first call so panics print to the browser
    // console instead of being swallowed silently.
    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("mount: window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("mount: document unavailable"))?;
    let element = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("mount: canvas '{canvas_id}' not found")))?;
    let canvas = element
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("mount: target element is not <canvas>"))?;

    let backend = backend::WebBackend::new(canvas)?;
    let host = widget_host::WidgetHost::new();
    let inner = Rc::new(RefCell::new(Inner { backend, host }));

    // Phase B inspector paints the first frame synchronously inside
    // mount(); any present error MUST surface as a JS exception so
    // callers do not see Ok with an unpainted canvas.
    inner.borrow_mut().repaint()?;

    let mut listeners: Vec<Listener> = Vec::new();
    // Listener target = `window` rather than the canvas. Keyboard
    // and composition events on the canvas only fire when the
    // canvas has focus and a `tabindex` attribute; the smoke HTML
    // does not set tabindex, and Phase B's static inspector demo
    // doesn't need focus management. Window-level listeners always
    // fire as long as no other element has captured the event.
    // Phase D+ widget chrome may want focus-routed listeners, at
    // which point the registration target gets parameterized.
    let win_target: web_sys::EventTarget = window.clone().into();

    // Codex Phase C gate BLOCK: registering N listeners with `?`
    // is NOT exception-safe — if registration #K fails, the
    // partially-built `listeners` vec drops without our `Drop for
    // WebShell` ever firing (we never reach the `Ok(WebShell {
    // ... })` line), and the K-1 already-registered DOM callbacks
    // outlive their wasm-bindgen Closures. Wrap all registrations
    // in an inner closure that, on Err, drains the partial vec
    // and unregisters everything we managed to land before
    // surfacing the error. The unregister loop is the same one
    // `Drop for WebShell` runs.
    let registration: Result<(), JsValue> = (|listeners: &mut Vec<Listener>| {
        // ----- keyboard: keydown + keyup → WidgetHost::apply_key -----
        {
            let inner_kd = inner.clone();
            add_listener::<KeyboardEvent, _>(
                &win_target,
                "keydown",
                listeners,
                move |evt: KeyboardEvent| {
                    let key_event = keyboard::map_keyboard_parts(
                        &evt.key(),
                        &evt.code(),
                        evt.location(),
                        evt.repeat(),
                        true, // pressed
                        modifiers_from_keyboard(&evt),
                        evt.is_composing(),
                    );
                    let mut inner = inner_kd.borrow_mut();
                    inner.host.apply_key(&key_event);
                    let _ = inner.repaint();
                },
            )?;
        }
        {
            let inner_ku = inner.clone();
            add_listener::<KeyboardEvent, _>(
                &win_target,
                "keyup",
                listeners,
                move |evt: KeyboardEvent| {
                    let key_event = keyboard::map_keyboard_parts(
                        &evt.key(),
                        &evt.code(),
                        evt.location(),
                        evt.repeat(),
                        false, // released
                        modifiers_from_keyboard(&evt),
                        evt.is_composing(),
                    );
                    let mut inner = inner_ku.borrow_mut();
                    inner.host.apply_key(&key_event);
                    let _ = inner.repaint();
                },
            )?;
        }

        // ----- IME: compositionstart / update / end → WidgetHost::apply_ime -----
        // Phase C1 ime mappers do the UTF-16→UTF-8 selection remap when
        // the browser supplies an IME-highlighted segment via
        // `getTargetRanges()`; current browsers expose that range via
        // a method we cannot call on `CompositionEvent` directly through
        // web-sys 0.3.94 without an extra raw `Reflect::get` shim.
        // For Step 1b we forward `data` only; selection lands in Phase D
        // alongside the DOM mirror that already needs Reflect::get.
        {
            let inner_cs = inner.clone();
            add_listener::<CompositionEvent, _>(
                &win_target,
                "compositionstart",
                listeners,
                move |_evt: CompositionEvent| {
                    let mut inner = inner_cs.borrow_mut();
                    inner.host.apply_ime(&ime::composition_start());
                    let _ = inner.repaint();
                },
            )?;
        }
        {
            let inner_cu = inner.clone();
            add_listener::<CompositionEvent, _>(
                &win_target,
                "compositionupdate",
                listeners,
                move |evt: CompositionEvent| {
                    let text = evt.data().unwrap_or_default();
                    let mut inner = inner_cu.borrow_mut();
                    inner.host.apply_ime(&ime::composition_update(text, None));
                    let _ = inner.repaint();
                },
            )?;
        }
        {
            let inner_ce = inner.clone();
            add_listener::<CompositionEvent, _>(
                &win_target,
                "compositionend",
                listeners,
                move |evt: CompositionEvent| {
                    let text = evt.data().unwrap_or_default();
                    let mut inner = inner_ce.borrow_mut();
                    inner.host.apply_ime(&ime::composition_end(text));
                    let _ = inner.repaint();
                },
            )?;
        }
        Ok(())
    })(&mut listeners);

    if let Err(e) = registration {
        // Unwind partial registration. Same body as `Drop for
        // WebShell`; kept inline rather than refactored into a free
        // function because the lifetimes only line up when the
        // `Listener` vec is local to mount().
        for l in listeners.drain(..) {
            let _ = l
                .target
                .remove_event_listener_with_callback(l.name, l.closure.as_ref().unchecked_ref());
        }
        return Err(e);
    }

    Ok(WebShell { inner, listeners })
}

/// Stub mount used by the kickoff §1.2 wasm32-clean compile guard CI.
/// Returns a fields-less `WebShell` after verifying the host has a
/// canvas with the given id; never paints. Real rendering needs the
/// `skia` feature.
#[cfg(not(feature = "skia"))]
#[wasm_bindgen]
pub fn mount(canvas_id: &str) -> Result<WebShell, JsValue> {
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;

    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("mount: window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("mount: document unavailable"))?;
    let element = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("mount: canvas '{canvas_id}' not found")))?;
    let _canvas = element
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("mount: target element is not <canvas>"))?;

    Ok(WebShell {})
}
