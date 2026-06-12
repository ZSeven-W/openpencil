//! Live-canvas sync IO for the Rust web shell — the pure `web_sys` plumbing
//! (interval timer, one-shot GET, one-shot POST) the live-sync glue drives to
//! talk to the web-canvas daemon (`op-host-desktop`'s `web_canvas_server`,
//! run via `--serve-web` / `op start --web`).
//!
//! This module is PURE `web_sys` IO (no skia, no `op_editor_core`), so it
//! compile-checks on the wasm32 web stub WITHOUT an EMSDK toolchain — i.e. the
//! highest-risk part of the live-sync glue (the `web_sys` XHR/interval calls)
//! is verified by `cargo check -p op-host-web --target wasm32-unknown-unknown`.
//! The protocol decisions (version gating, push baselines, apply + repaint)
//! live in `op_editor_core::web_sync` + `crate::live_sync_glue`, both
//! host-unit-tested.
#![allow(dead_code)]

use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Run `tick` every `interval_ms` for the page lifetime (the interval owns
/// the closure — same `forget()` idiom the previous document poll used).
pub fn start_interval(interval_ms: i32, tick: Rc<dyn Fn()>) -> Result<(), JsValue> {
    let cb = Closure::<dyn FnMut()>::new(move || tick());
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("live-sync: window unavailable"))?
        .set_interval_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            interval_ms,
        )?;
    cb.forget(); // the interval owns the closure for the page lifetime
    Ok(())
}

/// Issue one async `GET` and pass the response body to `on_response` when it
/// completes. Returns `false` when the request could not even start (the
/// callback will then never fire — callers must not park on it). `onloadend`
/// fires on completion regardless of HTTP status, so a non-2xx body still
/// reaches the callback (the protocol parsers reject it there).
pub fn get(url: &str, on_response: Rc<dyn Fn(String)>) -> bool {
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        return false;
    };
    if xhr.open_with_async("GET", url, true).is_err() {
        return false;
    }
    let xhr_for_load = xhr.clone();
    // `once_into_js` self-cleans after a single firing (no leaked closure per
    // request, unlike `forget()`).
    let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
        let text = xhr_for_load
            .response_text()
            .ok()
            .flatten()
            .unwrap_or_default();
        on_response(text);
    });
    xhr.set_onloadend(Some(onloadend.unchecked_ref()));
    xhr.send().is_ok()
}

/// Issue one async JSON `POST`. `on_response` (when given) receives the
/// response body on completion — including error/empty bodies, so an
/// in-flight latch held by the caller is always released. Returns `false`
/// when the request could not start (the callback will then never fire).
pub fn post_json(url: &str, body: &str, on_response: Option<Rc<dyn Fn(String)>>) -> bool {
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        return false;
    };
    if xhr.open_with_async("POST", url, true).is_err() {
        return false;
    }
    let _ = xhr.set_request_header("Content-Type", "application/json");
    if let Some(on_response) = on_response {
        let xhr_for_load = xhr.clone();
        let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
            let text = xhr_for_load
                .response_text()
                .ok()
                .flatten()
                .unwrap_or_default();
            on_response(text);
        });
        xhr.set_onloadend(Some(onloadend.unchecked_ref()));
    }
    xhr.send_with_opt_str(Some(body)).is_ok()
}
