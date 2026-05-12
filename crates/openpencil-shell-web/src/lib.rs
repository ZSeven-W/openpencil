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

    /// Hidden `<textarea>` that owns the IME composition target. The
    /// composition listeners are registered on THIS element (not on
    /// the window), so a browser-chrome IME context (URL bar, devtools
    /// search box, etc.) cannot mutate the inspector's TextInput
    /// state. The element stays focused for the page lifetime so CJK
    /// IME sequences route here. Phase D widget chrome will swap
    /// programmatic focus management in; Phase B static demo just
    /// keeps focus pinned on this element. (Codex Phase C stop-hook
    /// finding: "CJK IME listeners have no owned editable/focus
    /// target".)
    #[cfg(feature = "skia")]
    ime_target: web_sys::HtmlElement,
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

        // Pull the actual canvas dimensions from the backend
        // every frame so a host that swaps the `<canvas>` width
        // attribute (responsive layouts, devtool resizing,
        // future programmatic mount with a different size)
        // gets a layout that matches reality. Codex Step 3
        // stop-hook "web repaint ignores actual canvas size"
        // caught the prior hardcoded `960.0` — this swap also
        // resolves the "web smoke paints only the toolbar"
        // regression since the smoke canvas is 960×640 and the
        // first frame still gets the full width.
        let viewport_w = self.backend.canvas_width() as f32;
        let viewport_h = self.backend.canvas_height() as f32;

        self.backend.begin_frame();
        // Clear to white so widget paints sit on a clean background.
        self.backend.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_w, viewport_h),
            },
            Color::WHITE,
        );
        self.host.paint(&mut self.backend, viewport_w, viewport_h);
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
fn add_listener<E, F, T>(
    target: &T,
    name: &'static str,
    listeners: &mut Vec<Listener>,
    mut handler: F,
) -> Result<(), JsValue>
where
    E: wasm_bindgen::JsCast + 'static,
    F: FnMut(E) + 'static,
    T: Clone + Into<web_sys::EventTarget>,
{
    // Clone the target into an owned EventTarget once; we need both
    // `&EventTarget` for the registration call and an owned
    // EventTarget to push into the Listener for later cleanup. The
    // generic over T (`HtmlElement`, `EventTarget`, …) lets call
    // sites pass a window-target or an HtmlElement-target without
    // an explicit cast.
    let target_owned: web_sys::EventTarget = target.clone().into();
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
    target_owned.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())?;
    listeners.push(Listener {
        target: target_owned,
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
        // Tear down the hidden IME textarea so leaving the page does
        // not leave an orphan node + the browser does not ship dead
        // composition state into the next document.
        self.ime_target.remove();
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
/// Wall-clock ms since navigation. Falls back to 0 if either
/// `window` or `performance` is unavailable (e.g. WebWorker).
#[cfg(feature = "skia")]
fn now_ms_perf() -> u64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() as u64)
        .unwrap_or(0)
}

#[cfg(feature = "skia")]
#[wasm_bindgen]
pub fn mount(canvas_id: &str) -> Result<WebShell, JsValue> {
    use crate::event::{ime, keyboard};
    use wasm_bindgen::JsCast;
    use web_sys::{
        CompositionEvent, HtmlCanvasElement, HtmlElement, KeyboardEvent, MouseEvent, WheelEvent,
    };

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

    // Codex Phase C stop-hook fix: create a hidden <textarea> that
    // owns the IME composition target. Without this, registering
    // composition listeners on `window` would surface IME activity
    // from the browser's URL bar / devtools / any other editable
    // element on the page as if it were directed at our inspector
    // TextInput. The textarea is positioned off-screen + transparent
    // + aria-hidden so it does not interfere with screen readers or
    // visual layout; programmatic focus on it routes the user's IME
    // composition through our owned listeners.
    let ime_textarea = document
        .create_element("textarea")
        .map_err(|e| {
            JsValue::from_str(&format!(
                "mount: could not create hidden IME textarea: {:?}",
                e
            ))
        })?
        .dyn_into::<HtmlElement>()
        .map_err(|_| JsValue::from_str("mount: textarea is not HtmlElement"))?;
    ime_textarea.set_attribute(
        "style",
        "position:fixed;left:-9999px;top:0;width:1px;height:1px;\
         opacity:0;pointer-events:none;",
    )?;
    ime_textarea.set_attribute("aria-hidden", "true")?;
    ime_textarea.set_attribute("tabindex", "-1")?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("mount: document.body unavailable"))?;
    body.append_child(&ime_textarea)?;
    // Best-effort focus — focus() returns Err if the document is not
    // visible yet (e.g. tab in background), but the listener
    // registration still works once the user gives the page focus.
    let _ = ime_textarea.focus();

    let mut listeners: Vec<Listener> = Vec::new();
    // Keyboard events still go on window (the user expects shortcuts
    // like Cmd+S to fire regardless of which element has focus); the
    // codex stop-hook concern was about IME, not keyboard.
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
            add_listener::<KeyboardEvent, _, _>(
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
            add_listener::<KeyboardEvent, _, _>(
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
        // Listener target is the hidden textarea (codex Phase C
        // stop-hook fix), NOT the window — only IME composition
        // routed through our owned editable target reaches the
        // inspector. Browser-chrome compositions stay isolated.
        //
        // Phase C1 ime mappers do the UTF-16→UTF-8 selection remap when
        // the browser supplies an IME-highlighted segment via
        // `getTargetRanges()`; current browsers expose that range via
        // a method we cannot call on `CompositionEvent` directly through
        // web-sys 0.3.94 without an extra raw `Reflect::get` shim.
        // For Step 1b we forward `data` only; selection lands in Phase D
        // alongside the DOM mirror that already needs Reflect::get.
        {
            let inner_cs = inner.clone();
            add_listener::<CompositionEvent, _, _>(
                &ime_textarea,
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
            add_listener::<CompositionEvent, _, _>(
                &ime_textarea,
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
            add_listener::<CompositionEvent, _, _>(
                &ime_textarea,
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

        // ----- mouse: down / move / up / wheel → WidgetHost -----
        // Step 5: chrome interactions, infinite-canvas pan/zoom,
        // chat panel input. Listeners attached to the canvas
        // element so events outside the canvas don't fire.
        let canvas_target: web_sys::EventTarget = canvas.clone().into();
        {
            let inner_md = inner.clone();
            add_listener::<MouseEvent, _, _>(
                &canvas_target,
                "mousedown",
                listeners,
                move |evt: MouseEvent| {
                    let button = evt.button();
                    if button != 0 && button != 2 {
                        return;
                    }
                    let mut inner = inner_md.borrow_mut();
                    inner.host.set_modifier_shift(evt.shift_key());
                    inner.host.set_now_ms(now_ms_perf());
                    let (w, h) = (
                        inner.backend.canvas_width() as f32,
                        inner.backend.canvas_height() as f32,
                    );
                    let x = evt.offset_x() as f32;
                    let y = evt.offset_y() as f32;
                    let consumed = if button == 2 {
                        inner.host.apply_right_press(x, y, w, h)
                    } else {
                        inner.host.apply_press(x, y, w, h)
                    };
                    if consumed {
                        let _ = inner.repaint();
                    }
                },
            )?;
        }
        // Suppress browser's native context menu over the canvas so
        // the right-click is reserved for our layer-row menu.
        {
            add_listener::<MouseEvent, _, _>(
                &canvas_target,
                "contextmenu",
                listeners,
                move |evt: MouseEvent| {
                    evt.prevent_default();
                },
            )?;
        }
        {
            let inner_mm = inner.clone();
            add_listener::<MouseEvent, _, _>(
                &canvas_target,
                "mousemove",
                listeners,
                move |evt: MouseEvent| {
                    let mut inner = inner_mm.borrow_mut();
                    let h = inner.backend.canvas_height() as f32;
                    let x = evt.offset_x() as f32;
                    let y = evt.offset_y() as f32;
                    let hover_changed = inner.host.update_layer_hover(x, y, h);
                    let consumed = inner.host.apply_cursor_move(x, y);
                    if consumed || hover_changed {
                        let _ = inner.repaint();
                    }
                },
            )?;
        }
        {
            let inner_mu = inner.clone();
            add_listener::<MouseEvent, _, _>(
                &canvas_target,
                "mouseup",
                listeners,
                move |evt: MouseEvent| {
                    if evt.button() != 0 {
                        return;
                    }
                    let mut inner = inner_mu.borrow_mut();
                    let (w, h) = (
                        inner.backend.canvas_width() as f32,
                        inner.backend.canvas_height() as f32,
                    );
                    let consumed = inner.host.apply_release_with_viewport(w, h);
                    if consumed {
                        let _ = inner.repaint();
                    }
                },
            )?;
        }
        {
            let inner_wh = inner.clone();
            add_listener::<WheelEvent, _, _>(
                &canvas_target,
                "wheel",
                listeners,
                move |evt: WheelEvent| {
                    // Prevent page scroll; we own the gesture.
                    evt.prevent_default();
                    let mut inner = inner_wh.borrow_mut();
                    let (w, h) = (
                        inner.backend.canvas_width() as f32,
                        inner.backend.canvas_height() as f32,
                    );
                    // W3C deltaY: positive = scroll-down. Invert
                    // so wheel-up zooms in, wheel-down zooms out
                    // (matches the TS canvas convention).
                    let delta = -evt.delta_y() as f32;
                    let consumed = inner.host.apply_wheel(
                        evt.offset_x() as f32,
                        evt.offset_y() as f32,
                        delta,
                        w,
                        h,
                    );
                    if consumed {
                        let _ = inner.repaint();
                    }
                },
            )?;
        }

        // ----- chat input: keyboard text → WidgetHost::apply_text -----
        // Separate keydown listener that drives the chat input when
        // it's focused. Runs alongside the existing keyboard
        // listener above (which forwards Jian KeyEvents to the
        // widget tree); the chat path checks `focused` itself so
        // typing only routes when the user has clicked the input.
        {
            let inner_kt = inner.clone();
            add_listener::<KeyboardEvent, _, _>(
                &win_target,
                "keydown",
                listeners,
                move |evt: KeyboardEvent| {
                    use openpencil_shell_core::document::ReorderDirection;
                    let mut inner = inner_kt.borrow_mut();
                    let key = evt.key();
                    let is_mod = evt.meta_key() || evt.ctrl_key();
                    let shift = evt.shift_key();
                    let nudge = if shift { 10.0 } else { 1.0 };
                    let mut consumed = false;
                    match key.as_str() {
                        // Named-key editor shortcuts only fire
                        // when no Cmd / Ctrl is held — matches
                        // the native shell so Cmd+Backspace,
                        // Cmd+Arrow, Cmd+Enter etc. don't
                        // silently mutate editor state on top of
                        // their OS / browser bindings.
                        "Backspace" if !is_mod => {
                            consumed = inner.host.apply_backspace();
                        }
                        "Delete" if !is_mod => {
                            consumed = inner.host.apply_delete();
                        }
                        "Enter" if !is_mod => {
                            consumed = inner.host.apply_send();
                        }
                        "Escape" if !is_mod => {
                            consumed = inner.host.apply_escape();
                        }
                        "ArrowUp" if !is_mod => {
                            consumed = inner.host.apply_nudge(0.0, -nudge);
                        }
                        "ArrowDown" if !is_mod => {
                            consumed = inner.host.apply_nudge(0.0, nudge);
                        }
                        "ArrowLeft" if !is_mod => {
                            consumed = inner.host.apply_nudge(-nudge, 0.0);
                        }
                        "ArrowRight" if !is_mod => {
                            consumed = inner.host.apply_nudge(nudge, 0.0);
                        }
                        "[" if !is_mod => {
                            consumed = inner.host.apply_reorder(ReorderDirection::Down);
                        }
                        "]" if !is_mod => {
                            consumed = inner.host.apply_reorder(ReorderDirection::Up);
                        }
                        "d" if is_mod && !shift => {
                            consumed = inner.host.apply_duplicate();
                        }
                        "a" if is_mod && !shift => {
                            consumed = inner.host.apply_select_all();
                        }
                        "c" if is_mod && !shift => {
                            consumed = inner.host.apply_copy();
                        }
                        "x" if is_mod && !shift => {
                            consumed = inner.host.apply_cut();
                        }
                        "v" if is_mod && !shift => {
                            consumed = inner.host.apply_paste();
                        }
                        "z" if is_mod && !shift => {
                            consumed = inner.host.apply_undo();
                        }
                        "Z" if is_mod && shift => {
                            consumed = inner.host.apply_redo();
                        }
                        "y" if is_mod && !shift => {
                            consumed = inner.host.apply_redo();
                        }
                        _ => {
                            // Suppress apply_text whenever Cmd /
                            // Ctrl is held — Cmd-anything that
                            // isn't bound above must NOT type
                            // into a focused chat / property
                            // input. Otherwise Cmd+Shift+D (and
                            // other unbound chords) would inject
                            // "D" into the focused input.
                            if !is_mod {
                                // Single-char `key` strings represent
                                // typed printable characters.
                                let mut chars = key.chars();
                                if let (Some(c), None) = (chars.next(), chars.next()) {
                                    if !c.is_control() && inner.host.apply_text(c) {
                                        consumed = true;
                                    }
                                }
                            }
                        }
                    }
                    if consumed {
                        evt.prevent_default();
                        let _ = inner.repaint();
                    }
                },
            )?;
        }

        Ok(())
    })(&mut listeners);

    if let Err(e) = registration {
        // Unwind partial registration. Same body as `Drop for
        // WebShell`; kept inline rather than refactored into a free
        // function because the lifetimes only line up when the
        // `Listener` vec is local to mount(). Also detach the IME
        // textarea so a failed mount does not leave an orphan node
        // in the DOM.
        for l in listeners.drain(..) {
            let _ = l
                .target
                .remove_event_listener_with_callback(l.name, l.closure.as_ref().unchecked_ref());
        }
        ime_textarea.remove();
        return Err(e);
    }

    Ok(WebShell {
        inner,
        listeners,
        ime_target: ime_textarea,
    })
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
