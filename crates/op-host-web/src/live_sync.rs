//! Live-canvas sync IO for the Rust web shell — polls the web-canvas daemon
//! (`op-host-desktop`'s `web_canvas_server`, run via `--serve-web`) and hands
//! each `/api/mcp/document` response body to a caller-supplied callback.
//!
//! This module is PURE `web_sys` IO (no skia, no `op_editor_core`), so it
//! compile-checks on the wasm32 web stub WITHOUT an EMSDK toolchain — i.e. the
//! highest-risk part of the live-sync glue (the `web_sys` XHR/interval calls) is
//! verified by `cargo check -p op-host-web --target wasm32-unknown-unknown`. The
//! document apply + repaint (which need the skia shell) live in the caller's
//! `on_response` closure, wired up in `mount()` under `#[cfg(feature="skia")]`.
//!
//! The browser round-trip itself (canvas actually repainting) still needs a
//! running shell + browser to verify; this module makes the wire/IO correct.
//!
//! These helpers are wired up by `mount()` under `#[cfg(feature = "skia")]`, but
//! the module is compiled in EVERY build (including the no-skia web stub) so the
//! `web_sys` IO is type-checked without EMSDK — hence the module-wide
//! `allow(dead_code)` (they're "unused" only in the stub-compile-check build;
//! `push` is the local-edit→daemon send helper, ready to wire into the edit path).
#![allow(dead_code)]

use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Poll `{base}/api/mcp/document` every `interval_ms`, handing each response
/// body to `on_response`. `base` is the daemon origin, e.g.
/// `http://127.0.0.1:3100`. `on_response` is `Rc<dyn Fn(String)>` so it can be
/// shared across the interval tick and each request's completion callback.
pub fn start(base: &str, interval_ms: i32, on_response: Rc<dyn Fn(String)>) -> Result<(), JsValue> {
    let url = format!("{base}/api/mcp/document");
    let tick = Closure::<dyn FnMut()>::new(move || poll_once(&url, on_response.clone()));
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("live-sync: window unavailable"))?
        .set_interval_with_callback_and_timeout_and_arguments_0(
            tick.as_ref().unchecked_ref(),
            interval_ms,
        )?;
    tick.forget(); // the interval owns the closure for the page lifetime
    Ok(())
}

/// Issue one async `GET` and pass the response body to `on_response` when it
/// completes. Best-effort: any IO error is silently dropped (the next tick
/// retries).
fn poll_once(url: &str, on_response: Rc<dyn Fn(String)>) {
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        return;
    };
    if xhr.open_with_async("GET", url, true).is_err() {
        return;
    }
    let xhr_for_load = xhr.clone();
    // `once_into_js` self-cleans after a single firing (no leaked closure per
    // request, unlike `forget()`); `onloadend` fires on completion regardless
    // of HTTP status so a non-2xx doesn't strand the callback.
    let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
        if let Ok(Some(text)) = xhr_for_load.response_text() {
            on_response(text);
        }
    });
    xhr.set_onloadend(Some(onloadend.unchecked_ref()));
    let _ = xhr.send();
}

/// Push the local document to the daemon (`POST /api/mcp/document`) — call after
/// a local edit so other clients (and any MCP clients) see it. Best-effort.
pub fn push(base: &str, body: &str) {
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        return;
    };
    let url = format!("{base}/api/mcp/document");
    if xhr.open_with_async("POST", &url, true).is_err() {
        return;
    }
    let _ = xhr.set_request_header("Content-Type", "application/json");
    let _ = xhr.send_with_opt_str(Some(body));
}
