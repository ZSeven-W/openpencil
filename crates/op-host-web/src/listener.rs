//! DOM listener plumbing shared by `mount()` (lib.rs) and the
//! browser file-IO glue (`dom_io`). Extracted verbatim from `lib.rs`
//! when the file-IO listeners pushed it past the repo's 800-line
//! file cap; behaviour is unchanged.

use wasm_bindgen::prelude::*;

use op_editor_ui::Modifiers;

pub(crate) struct Listener {
    pub(crate) target: web_sys::EventTarget,
    pub(crate) name: &'static str,
    /// Type-erased Closure storage. We use
    /// `Closure<dyn FnMut(JsValue)>` uniformly across event types and
    /// runtime-checked `dyn_into::<SpecificEvent>()` inside each
    /// handler body so `Listener` carries one concrete generic
    /// argument across the entire vec; mismatched synthetic events
    /// from same-page JS skip the handler instead of producing a
    /// wrong-type reference (codex C2.2 R1 CONCERN-3).
    pub(crate) closure: Closure<dyn FnMut(JsValue)>,
}

/// Build a Jian `Modifiers` bitset from a W3C `KeyboardEvent`. Mirrors
/// the four standard modifier keys; per spec §2.4 we treat
/// `metaKey` (browser) → `Modifiers::CMD` (Jian) — both name the
/// "Cmd on macOS / Win key on Windows / Super on Linux" modifier.
pub(crate) fn modifiers_from_keyboard(event: &web_sys::KeyboardEvent) -> Modifiers {
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
pub(crate) fn add_listener<E, F, T>(
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

/// Wall-clock ms since navigation. Falls back to 0 if either
/// `window` or `performance` is unavailable (e.g. WebWorker).
pub(crate) fn now_ms_perf() -> u64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() as u64)
        .unwrap_or(0)
}
