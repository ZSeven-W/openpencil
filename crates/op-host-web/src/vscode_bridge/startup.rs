//! Bridge startup-coordination helpers: iframe detection and awaiting the
//! host's `init` (with the 2 s direct-open fallback).
//!
//! Split out of the `vscode_bridge` spine to keep it under the 800-line cap;
//! re-exported there so the mount call sites keep their paths.

use js_sys::{Object, Promise};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::live_sync;

use super::INIT_RESOLVER;

/// True when the page runs inside a frame (`window.self != window.top`) — the
/// VS Code webview case, where `mount_ck` awaits the host's `init` before it
/// bootstraps the daemon services.
pub(crate) fn in_iframe(window: &web_sys::Window) -> bool {
    let self_win = window.self_();
    // `top`/`parent` return a cross-origin-accessible WindowProxy; an identity
    // compare never touches a property so it can't throw. Check both so a host
    // that shadows one (some webview shells) is still detected.
    let differs = |other: Result<Option<web_sys::Window>, JsValue>| {
        other
            .ok()
            .flatten()
            .map(|w| !Object::is(self_win.as_ref(), w.as_ref()))
            .unwrap_or(false)
    };
    differs(window.top()) || differs(window.parent())
}

/// Await the host's `init` (which resolves the promise from the message
/// handler) or a `timeout_ms` fallback, whichever comes first. On timeout the
/// caller proceeds as a direct open (no token). Returns after the promise
/// settles; check [`live_sync::bridge_token`] to learn which path won.
pub(crate) async fn await_init(window: &web_sys::Window, timeout_ms: i32) {
    // The early listener may have buffered the host's `init` and `install`
    // replays it synchronously BEFORE this runs: with the token already
    // stored there is nothing to await, and waiting would burn the whole 2 s
    // fallback after the init had in fact landed.
    if live_sync::bridge_token().is_some() {
        return;
    }
    let window = window.clone();
    let promise = Promise::new(&mut |resolve, _reject| {
        INIT_RESOLVER.with(|r| *r.borrow_mut() = Some(resolve.clone()));
        // Timeout fallback: resolve the same promise so the await unblocks even
        // if no host is listening (standalone browser tab, or a slow host).
        let resolve_timeout = resolve.clone();
        let cb = Closure::once_into_js(move || {
            let _ = resolve_timeout.call0(&JsValue::NULL);
        });
        let _ = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), timeout_ms);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    // Drop the resolver so a late `init` doesn't try to settle a done promise.
    INIT_RESOLVER.with(|r| *r.borrow_mut() = None);
    if live_sync::bridge_token().is_none() {
        web_sys::console::warn_1(&JsValue::from_str(
            "[op-bridge] init not received before timeout; proceeding as direct open",
        ));
    }
}
