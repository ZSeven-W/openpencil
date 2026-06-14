//! OpenPencil web host — web bundle entry.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate is the web bundle
//! entry. CI invariant requires `cargo check --target wasm32-unknown-
//! unknown -p op-host-web --no-default-features --features web`
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

// Hidden accessibility DOM mirror (#58) — sr-only landmarks, live
// regions and operable tool buttons beside the canvas, synced from
// `EditorState` on a coarse cadence. Needs the real host, so gated
// like the other skia modules. See `a11y.rs` for the honest v1 scope.
#[cfg(feature = "skia")]
mod a11y;
#[cfg(feature = "skia")]
mod backend;
#[cfg(feature = "skia")]
mod boolean_ops;
pub mod event;
#[cfg(feature = "skia")]
mod listener;
#[cfg(feature = "skia")]
mod widget_host;
// Pure web_sys IO (no skia) — compiled always so it compile-checks on the
// wasm32 web stub without EMSDK; only `mount()` (skia) actually wires it up.
mod live_sync;
// Bidirectional live-canvas sync glue (pull/apply + push + selection sync) —
// the protocol decisions over the pure `live_sync` IO. Needs the skia shell
// (apply + repaint) and the document pipeline, so gated like the other
// document modules; `codegen` (the production bundle) enables it.
#[cfg(feature = "live-sync")]
mod live_sync_glue;
// Shared daemon base-URL resolution (page origin when served by the daemon,
// localhost fallback for the dev smoke page). Pure-logic core + a thin
// `window.location` wrapper; compiled always like `live_sync`.
mod daemon_base;

// Browser file-IO P0 cluster — the `pending_file_action` consumer
// (Save / Open / Export / Import dialogs become Blob downloads +
// hidden `<input type=file>` pickers), DOM paste routing, and file
// drag-drop ingestion. Gated behind `codegen` (the full browser
// build) with the rest of the document-pipeline deps (serde_json /
// jian-ops-schema / op-figma) so neither the `web` stub nor a plain
// `skia` build pulls them.
#[cfg(feature = "codegen")]
mod dom_io;
#[cfg(feature = "codegen")]
mod file_actions;
// Hidden IME composition target builder — extracted from `mount()`
// to keep this file under the 800-line cap.
#[cfg(feature = "skia")]
mod ime_target;

// P4b web AI-streaming foundation — gated behind the `codegen` feature (which
// pulls `skia` + `op-codegen`), so the wasm32-clean stub baseline never
// compiles them. UNVERIFIED: these need an EMSDK wasm32 build + a browser; run
// tools/check-wasm-bundle.sh.
#[cfg(feature = "codegen")]
mod codegen_bundle;
#[cfg(feature = "codegen")]
mod codegen_web;
#[cfg(feature = "codegen")]
mod raf_pump;
#[cfg(feature = "codegen")]
mod web_ai_transport;
// Web Iconify bridge — drains the icon picker's remote-search request
// directly against api.iconify.design (CORS-open, same as TS).
#[cfg(feature = "codegen")]
mod iconify_web;
// Web chat session — drains `chat.pending_send` / Stop / New Chat and
// streams real AI turns through the daemon's `/api/ai/stream` proxy.
#[cfg(feature = "codegen")]
mod web_chat;
#[cfg(feature = "codegen")]
mod web_clipboard;

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
use listener::{add_listener, modifiers_from_keyboard, now_ms_perf, Listener};

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

    /// Interval handle driving the hidden accessibility layer's
    /// coarse state→DOM sync (`a11y::start_pump`). Dropping it clears
    /// the browser interval; `None` when the pump could not start
    /// (missing `window`) — the layer then only refreshes on hidden-
    /// button activations.
    #[cfg(feature = "skia")]
    a11y_pump: Option<a11y::A11yPump>,
}

#[cfg(feature = "skia")]
struct Inner {
    backend: backend::WebBackend,
    host: widget_host::WidgetHost,
    /// Hidden accessibility DOM layer (`a11y.rs`). `None` until
    /// `a11y::wire` mounts it at the tail of `mount()` registration;
    /// kept on `Inner` so both the interval pump and the hidden
    /// buttons' click handlers reach it through the shared `Rc`.
    a11y: Option<a11y::A11yLayer>,
    #[cfg(feature = "codegen")]
    caret_raf_running: bool,
}

// `Listener` + `add_listener` + `modifiers_from_keyboard` +
// `now_ms_perf` live in `listener.rs` — extracted to keep this file
// under the 800-line cap once the browser file-IO glue landed.

#[cfg(feature = "skia")]
impl Inner {
    /// Phase B inspector paint. Called from `mount()` for the first
    /// frame and from every closure body after a state mutation.
    /// Returns the present error if the ImageData round-trip failed.
    fn repaint(&mut self) -> Result<(), JsValue> {
        use op_editor_ui::{Color, Point2D, Rect, RenderBackend};

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

#[cfg(feature = "codegen")]
pub(crate) fn ensure_caret_blink_pump(inner: &Rc<RefCell<Inner>>) {
    {
        let mut guard = inner.borrow_mut();
        if guard.caret_raf_running || !guard.host.caret_animation_active() {
            return;
        }
        guard.caret_raf_running = true;
    }

    let inner_tick = inner.clone();
    let tick = Rc::new(move || {
        let mut guard = inner_tick.borrow_mut();
        guard.host.set_now_ms(now_ms_perf());
        if !guard.host.caret_animation_active() {
            guard.caret_raf_running = false;
            return false;
        }
        let _ = guard.repaint();
        true
    });
    raf_pump::start(tick);
}

#[cfg(feature = "skia")]
impl Drop for WebShell {
    fn drop(&mut self) {
        // Stop the a11y sync pump first (clears the interval), then
        // tear its hidden DOM layer down with the rest of the shell.
        self.a11y_pump = None;
        if let Some(layer) = self.inner.borrow_mut().a11y.take() {
            layer.unmount();
        }
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
#[cfg(feature = "skia")]
#[wasm_bindgen]
pub fn mount(canvas_id: &str) -> Result<WebShell, JsValue> {
    use crate::event::{ime, keyboard};
    use wasm_bindgen::JsCast;
    use web_sys::{CompositionEvent, HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent};

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

    let backend = backend::WebBackend::new(canvas.clone())?;
    let host = widget_host::WidgetHost::new();
    let inner = Rc::new(RefCell::new(Inner {
        backend,
        host,
        a11y: None,
        #[cfg(feature = "codegen")]
        caret_raf_running: false,
    }));

    // Phase B inspector paints the first frame synchronously inside
    // mount(); any present error MUST surface as a JS exception so
    // callers do not see Ok with an unpainted canvas.
    inner.borrow_mut().repaint()?;

    // Hidden IME composition target (codex Phase C stop-hook fix) —
    // builder + rationale live in `ime_target.rs` (extracted to keep
    // this file under the 800-line cap when the a11y wiring landed).
    let ime_textarea = ime_target::create_hidden_ime_textarea(&document)?;

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
                    drop(inner);
                    #[cfg(feature = "codegen")]
                    ensure_caret_blink_pump(&inner_cs);
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
                    drop(inner);
                    #[cfg(feature = "codegen")]
                    ensure_caret_blink_pump(&inner_cu);
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
                    drop(inner);
                    #[cfg(feature = "codegen")]
                    ensure_caret_blink_pump(&inner_ce);
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
                    {
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
                        // `inner` borrow dropped here before the codegen drain,
                        // which re-borrows `inner` inside `start_codegen`.
                    }
                    #[cfg(feature = "codegen")]
                    ensure_caret_blink_pump(&inner_md);
                    // A Code-panel Generate / Regenerate / Cancel click raised a
                    // pending flag during `apply_press`; launch (or abort) the
                    // codegen run now that the borrow is released.
                    #[cfg(feature = "codegen")]
                    codegen_web::drain_codegen_flags(&inner_md);
                    // File-menu / export-dialog / figma-modal / shape-picker /
                    // fill-image presses raise `pending_file_action`; consume it
                    // the same borrow-released way (the handlers re-borrow
                    // `inner` for serialization / pickers / repaint).
                    #[cfg(feature = "codegen")]
                    dom_io::drain_pending_file_action(&inner_md);
                    // Chat Send / Stop / New Chat presses raise their
                    // `chat.pending_*` flags during `apply_press`; launch /
                    // abort the streaming turn now that the borrow is released.
                    #[cfg(feature = "codegen")]
                    web_chat::drain_chat_flags(&inner_md);
                    // An icon-picker Load-more press raised the remote
                    // Iconify search request; fire the browser fetch
                    // chain now that the borrow is released.
                    #[cfg(feature = "codegen")]
                    iconify_web::drain_iconify_request(&inner_md);
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
                    use op_editor_core::ReorderDirection;
                    // Ignore keystrokes that are part of an in-flight IME
                    // composition — the hidden textarea's composition
                    // events own them, and the committed string lands
                    // through `apply_ime` on compositionend. Without this
                    // guard CJK input would inject the raw latin
                    // keystrokes AND the commit (double input).
                    if evt.is_composing() {
                        return;
                    }
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
                            consumed = inner.host.apply_settings_caret(false)
                                || inner.host.apply_chat_model_picker_caret(false)
                                || inner.host.apply_rename_caret(false)
                                || inner.host.apply_property_caret(false)
                                || inner.host.apply_nudge(-nudge, 0.0);
                        }
                        "ArrowRight" if !is_mod => {
                            consumed = inner.host.apply_settings_caret(true)
                                || inner.host.apply_chat_model_picker_caret(true)
                                || inner.host.apply_rename_caret(true)
                                || inner.host.apply_property_caret(true)
                                || inner.host.apply_nudge(nudge, 0.0);
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
                            // With the codegen browser-IO build the DOM `paste`
                            // listener owns Cmd+V (system Figma-HTML first, then
                            // the focused text input, then the internal node
                            // clipboard — mirroring the native priority); also
                            // consuming it here would double-paste. Plain skia
                            // builds keep the internal node clipboard binding.
                            #[cfg(not(feature = "codegen"))]
                            {
                                consumed = inner.host.apply_paste();
                            }
                        }
                        "z" if is_mod && !shift => {
                            consumed = inner.host.apply_undo();
                        }
                        "Z" if is_mod && shift => {
                            consumed = inner.host.apply_redo();
                        }
                        // Cmd+Shift+K — toggle the UIKit browser (TS
                        // `editor-layout.tsx`). With Shift held the
                        // W3C `key` is uppercase.
                        "k" | "K" if is_mod && shift => {
                            consumed = inner.host.apply_toggle_component_browser();
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
                    // Release the borrow before the chat drain below —
                    // `drain_chat_flags` re-borrows `inner` (and the rAF pump
                    // it starts borrows it again on later frames).
                    drop(inner);
                    #[cfg(feature = "codegen")]
                    if consumed {
                        ensure_caret_blink_pump(&inner_kt);
                    }
                    // Enter routed through `apply_send` raised
                    // `chat.pending_send` (begin_send); launch the streaming
                    // turn now that the borrow is released.
                    #[cfg(feature = "codegen")]
                    web_chat::drain_chat_flags(&inner_kt);
                },
            )?;
        }

        // ----- browser file IO: DOM paste + file drag-drop -----
        // Clipboard paste (Figma HTML / plain text / internal node
        // clipboard) on the window; dragover / dragleave / drop on the
        // canvas (drives the painted `file_drop_active` overlay and
        // routes dropped .op/.pen/.fig/image files through the same
        // ingestion the file menu uses).
        #[cfg(feature = "codegen")]
        dom_io::register_io_listeners(&inner, &canvas, &win_target, listeners)?;

        // ----- hidden accessibility DOM mirror (#58) -----
        // Mounts the sr-only landmark layer + registers its button
        // listeners into the shared vec. The layer lands on
        // `Inner.a11y` BEFORE its listeners register, so the unwind
        // below tears it down on a mid-registration failure.
        a11y::wire(&document, &canvas, &inner, listeners)?;

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
        if let Some(layer) = inner.borrow_mut().a11y.take() {
            layer.unmount();
        }
        ime_textarea.remove();
        return Err(e);
    }

    // Start the a11y mirror's coarse sync pump (see `a11y.rs` for the
    // cadence rationale). Best-effort — a missing window leaves the
    // hidden layer refreshing only on its own button activations.
    let a11y_pump = a11y::start_pump(&inner);

    // Model discovery: ask the daemon for its model catalog once so the chat
    // model picker lists real models. Async + best-effort — a missing daemon
    // leaves the catalog empty and chat sends use the "default" model.
    #[cfg(feature = "codegen")]
    web_chat::fetch_models(&inner);

    // Bidirectional live-canvas sync: pull external MCP/CLI document writes
    // into this canvas (version probe + fetch/apply) AND push local edits +
    // selection back to the daemon. See `live_sync_glue` for the TS
    // `use-mcp-sync.ts` parity notes + documented transport divergences.
    #[cfg(feature = "live-sync")]
    live_sync_glue::start(&inner);

    Ok(WebShell {
        inner,
        listeners,
        ime_target: ime_textarea,
        a11y_pump,
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
