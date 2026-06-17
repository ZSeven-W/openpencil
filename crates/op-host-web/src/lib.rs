//! OpenPencil web host — web bundle entry.
//!
//! Two build configurations:
//! * `canvaskit` — the production web shell: renders the full editor through
//!   the official CanvasKit skia WASM (GPU) via `mount_ck` (`canvaskit.rs`).
//!   All widget / chat / codegen / file-IO / live-sync logic is shared Rust.
//! * `web` (default) — a wasm32-unknown-unknown-clean stub (no skia, no
//!   CanvasKit) that only compile-checks the public surface for CI; `mount`
//!   below returns a fields-less `WebShell` and never paints.
//!
//! (The from-scratch skia raster WebBackend + its `skia` / `codegen` /
//! `live-sync` features + the WebGL2 Ganesh backend were retired 2026-06-17 —
//! CanvasKit is the only web renderer now, and `skia-safe` reverted to upstream
//! crates.io, which has no wasm32-unknown-unknown target.)

#[cfg(feature = "canvaskit")]
mod canvaskit;
pub mod event;
#[cfg(feature = "canvaskit")]
mod listener;
#[cfg(feature = "canvaskit")]
mod widget_host;
// Backend-agnostic seam: lets the chat / live-sync / codegen / file-IO modules
// drive the CanvasKit `CkInner` through one trait.
#[cfg(feature = "canvaskit")]
mod repaint_ctx;
// Pure web_sys IO (no skia) — compiled always so the `web` stub still
// compile-checks on wasm32 without EMSDK.
mod live_sync;
#[cfg(feature = "canvaskit")]
mod live_sync_glue;
// Shared daemon base-URL resolution (page origin when served by the daemon,
// localhost fallback for the dev smoke page).
#[cfg(feature = "canvaskit")]
mod codegen_bundle;
#[cfg(feature = "canvaskit")]
mod codegen_web;
mod daemon_base;
#[cfg(feature = "canvaskit")]
mod dom_io;
#[cfg(feature = "canvaskit")]
mod file_actions;
#[cfg(feature = "canvaskit")]
mod raf_pump;
#[cfg(feature = "canvaskit")]
mod web_ai_transport;
// Web Iconify bridge — drains the icon picker's remote-search request directly
// against api.iconify.design (CORS-open, same as TS).
#[cfg(feature = "canvaskit")]
mod iconify_web;
// Web chat session — drains `chat.pending_send` / Stop / New Chat and streams
// real AI turns through the daemon's `/api/ai/stream` proxy.
#[cfg(feature = "canvaskit")]
mod web_chat;
// Pure web_sys clipboard/download — Ctrl+C/X in inputs + Figma/file paste.
#[cfg(feature = "canvaskit")]
mod web_clipboard;
#[cfg(feature = "canvaskit")]
mod web_fonts;

#[cfg(not(feature = "canvaskit"))]
use wasm_bindgen::prelude::*;

/// Fields-less stub shell for the `web` compile-guard build. The production
/// CanvasKit build uses `canvaskit::mount_ck` instead and never constructs it.
#[cfg(not(feature = "canvaskit"))]
#[wasm_bindgen]
pub struct WebShell {}

/// Stub mount used by the kickoff §1.2 wasm32-clean compile guard CI.
/// Returns a fields-less `WebShell` after verifying the host has a canvas with
/// the given id; never paints. The production renderer is `mount_ck`
/// (`canvaskit` feature, `canvaskit.rs`).
#[cfg(not(feature = "canvaskit"))]
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
