//! skia-safe surface bring-up for wasm32-unknown-unknown (Step 1b §2.2,
//! post C-hard.2 lock-in 2026-05-09).
//!
//! Phase A path: raster N32_PREMUL surface, presented to the host `<canvas>`
//! via `put_image_data`. Phase A round 2 may add a GPU `GrContext` over
//! WebGL2 once raster is proven; the trait surface in `backend/mod.rs` is
//! agnostic, so swapping the surface constructor is a self-contained change.

use wasm_bindgen::JsValue;

/// Allocate a raster surface backed by N32_PREMUL pixels. `width` /
/// `height` are clamped to ≥ 1 by the caller (`WebBackend::new`).
pub fn make_raster_surface(width: u32, height: u32) -> Result<skia_safe::Surface, JsValue> {
    skia_safe::surfaces::raster_n32_premul((width as i32, height as i32))
        .ok_or_else(|| JsValue::from_str("skia_safe::surfaces::raster_n32_premul returned None"))
}
