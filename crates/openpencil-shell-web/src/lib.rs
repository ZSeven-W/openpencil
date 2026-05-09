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

// Force the wasm32-unknown-unknown libc/libcxx/libm shim to be linked
// even though no Rust code calls it — its `#[no_mangle]` symbols are
// referenced only by the C++ side of the wasm (Skia static lib). Without
// this `extern crate`, cargo would dead-code-eliminate the shim because
// no Rust path imports anything from it.
#[cfg(all(feature = "skia", target_arch = "wasm32", target_os = "unknown"))]
extern crate wasm_libc_shim as _;

use wasm_bindgen::prelude::*;

/// Long-lived shell handle. The smoke HTML must keep this alive (e.g.
/// `window.__opShell = mount("op")`) so closures stored on the shell
/// remain reachable for the page lifetime.
///
/// The stub variant (without `skia` feature) carries no fields and exists
/// only so the wasm32-unknown-unknown CI baseline can compile-check the
/// public surface.
#[wasm_bindgen]
pub struct WebShell {
    #[cfg(feature = "skia")]
    backend: backend::WebBackend,
}

#[cfg(feature = "skia")]
impl WebShell {
    /// Phase A red-rect demo: clear to white, draw a centered red rect,
    /// snapshot to the host canvas. Returns the present error if the
    /// final ImageData round-trip failed — callers MUST propagate this
    /// to JS instead of treating mount as successful when the canvas
    /// stayed blank.
    ///
    /// Phase B+ replaces this with the widget host paint loop.
    fn paint_phase_a(&mut self) -> Result<(), JsValue> {
        use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend};
        self.backend.begin_frame();
        // Clear background.
        self.backend.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(960.0, 640.0),
            },
            Color::WHITE,
        );
        // Centered red rect: 320×120 inside the 960×640 canvas.
        self.backend.fill_rect(
            Rect {
                origin: Point2D::new(320.0, 260.0),
                size: Point2D::new(320.0, 120.0),
            },
            Color::RED,
        );
        self.backend.end_frame();
        if let Some(err) = self.backend.take_present_error() {
            return Err(err);
        }
        Ok(())
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
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;

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
    let mut shell = WebShell { backend };
    // Phase A demo paints synchronously inside mount(); any present error
    // (read_pixels / put_image_data) MUST surface as a JS exception so
    // callers do not see Ok with an unpainted canvas.
    shell.paint_phase_a()?;
    Ok(shell)
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
