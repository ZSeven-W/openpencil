//! The embedded coffee demo photo the Home explore cards still draw
//! (the screenshot-tutorial card's corner phone).
//!
//! The App task's example art used to be a hand-drawn phone / counter
//! window mock-up here. It now paints the baked preview of the scene
//! template the App example opens as its instant draft
//! (`coffee-order-app` / `coffee-counter-desktop`), like every other
//! task, so the picture and the draft can no longer disagree.

use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::{ImageAdjustments, ImageDrawMode, Rect, RenderBackend};

/// Stable cache id for the embedded demo photo (mirrors the login
/// modal's brand-logo id policy).
const COFFEE_DEMO_IMAGE_ID: u64 = 0x484f_4d45_4346_4631;
const COFFEE_DEMO_JPG: &[u8] = include_bytes!("../../assets/home_examples/coffee-demo.jpg");

/// Draw the demo coffee photo, fill-cropped into `rect`.
/// `saturation` uses the image-adjustment slider scale (0 = neutral,
/// CSS `saturate(.6)` ≈ −40). Returns `false` while the first decode
/// is still in flight so the caller can paint its placeholder.
pub(super) fn draw_coffee_photo(
    backend: &mut dyn RenderBackend,
    rect: Rect,
    opacity: f32,
    saturation: f32,
) -> bool {
    if !has_cached_image_bytes(COFFEE_DEMO_IMAGE_ID) {
        store_remote_image_bytes(COFFEE_DEMO_IMAGE_ID, COFFEE_DEMO_JPG.to_vec());
    }
    let max_edge = required_raster_edge(rect, backend.dpi_scale());
    let sharp = backend.image_decoded(COFFEE_DEMO_IMAGE_ID, COFFEE_DEMO_JPG, max_edge);
    if !sharp {
        note_pending_decode(COFFEE_DEMO_IMAGE_ID, max_edge);
    }
    if sharp || backend.image_resident(COFFEE_DEMO_IMAGE_ID) {
        let adjustments = ImageAdjustments {
            saturation,
            ..ImageAdjustments::default()
        };
        backend.draw_image_with_options(
            rect,
            COFFEE_DEMO_IMAGE_ID,
            COFFEE_DEMO_JPG,
            ImageDrawMode::Fill,
            adjustments,
            opacity,
            0.0,
        );
        true
    } else {
        false
    }
}
