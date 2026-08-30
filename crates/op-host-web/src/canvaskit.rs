//! CanvasKit rendering path for the Rust web shell.
//!
//! Replaces the from-scratch `wasm32-unknown-unknown` skia build (skia-safe-op
//! + hand-rolled libc++/GL shim) with the official CanvasKit skia WASM artifact.
//!
//! The Rust side owns all widget/draw logic and drives CanvasKit through the
//! thin `op_ck_bridge.js` FFI. `CanvasKitBackend` implements the same
//! `RenderBackend` (`jian_widgets::painter::Painter`) the native desktop
//! backend implements, so all shell-core UI code is shared across platforms.
//!
//! This file is the spine; the implementation lives in sibling modules under
//! `canvaskit/`:
//!
//! * `bindings` — the `op_ck_bridge.js` / `op_ck_image_cache.js` FFI blocks
//! * `convert`  — pure DPR / gradient / enum-code helpers
//! * `backend` / `backend_state` — `CanvasKitBackend`, state methods, and init
//! * `ops`      — the flat `OpCk` draw-call bodies the `RenderBackend` impl forwards to
//! * `inner`    — `CkInner` live shell state + daemon bootstrap
//! * `mount`    — the body of `mount_ck`

use op_editor_ui::{Color, Point2D, Rect, RenderBackend};
use wasm_bindgen::prelude::*;

mod backend;
mod backend_state;
mod bindings;
mod convert;
mod inner;
mod measure;
mod mount;
mod mount_keyboard;
mod ops;
#[cfg(test)]
mod tests;

pub use backend::{init_backend, CanvasKitBackend};
pub use bindings::OpCk;
pub use measure::BrowserMeasure;

/// Mount the full editor chrome on `canvas_id`, rendered via CanvasKit on the
/// GPU, with mouse / wheel / keyboard interactivity. Builds the shared
/// `WidgetHost` (skia-free under this feature) and drives it through
/// `CanvasKitBackend`, behind the same `RenderBackend` the desktop host uses.
#[wasm_bindgen]
pub async fn mount_ck(canvas_id: String) -> Result<(), JsValue> {
    mount::mount_ck(canvas_id).await
}

/// Smoke entry retained for FFI validation (renders AA text + a fill).
#[wasm_bindgen]
pub async fn ck_smoke(canvas_id: String) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let mut be = init_backend(&canvas_id, 1.0, 800, 300).await?;
    be.ck.clear(1.0, 1.0, 1.0, 1.0);
    be.fill_rect(
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(800.0, 60.0),
        },
        Color {
            r: 0.06,
            g: 0.06,
            b: 0.06,
            a: 1.0,
        },
    );
    be.ck.draw_text(
        "OpenPencil Rust -> CanvasKit GPU",
        "",
        20.0,
        40.0,
        28.0,
        400,
        false,
        0.9,
        0.9,
        0.95,
        1.0,
    );
    be.end_frame();
    Ok(())
}
