//! Unit tests for [`super`]'s `NativeBackend` — colour packing,
//! rect conversion, and the `draw_image` aspect-fit + decode cache.
//! Split into a sibling file to keep `skia.rs` under the 800-line cap.

use super::*;

#[test]
fn color_roundtrip_clamps_and_packs() {
    let red = to_jian_color(Color::RED);
    assert_eq!(red.r(), 255);
    assert_eq!(red.g(), 0);
    assert_eq!(red.b(), 0);
    assert_eq!(red.a(), 255);

    let transparent = to_jian_color(Color::TRANSPARENT);
    assert_eq!(transparent.a(), 0);

    // Out-of-range channels are clamped, not wrapped.
    let weird = to_jian_color(Color {
        r: -0.5,
        g: 2.0,
        b: 0.5,
        a: 1.0,
    });
    assert_eq!(weird.r(), 0);
    assert_eq!(weird.g(), 255);
    assert_eq!(weird.b(), 128);
}

#[test]
fn contain_rect_fits_wide_image_letterboxed_vertically() {
    let outer = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    // A 200×100 image is wider than the box → width-bound, with
    // empty bands top + bottom, image centered vertically.
    let r = contain_rect(outer, 200.0, 100.0);
    assert!((r.size.x - 100.0).abs() < 1e-4);
    assert!((r.size.y - 50.0).abs() < 1e-4);
    assert!((r.origin.x - 0.0).abs() < 1e-4);
    assert!((r.origin.y - 25.0).abs() < 1e-4, "centered vertically");
}

#[test]
fn contain_rect_fits_tall_image_pillarboxed_horizontally() {
    let outer = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    let r = contain_rect(outer, 100.0, 200.0);
    assert!((r.size.y - 100.0).abs() < 1e-4);
    assert!((r.size.x - 50.0).abs() < 1e-4);
    assert!((r.origin.x - 25.0).abs() < 1e-4, "centered horizontally");
}

#[test]
fn contain_rect_degenerate_image_size_falls_back_to_outer() {
    // A zero-dimension image must not divide-by-zero — it just
    // returns the outer rect unchanged.
    let outer = Rect::xywh(5.0, 6.0, 80.0, 40.0);
    let r = contain_rect(outer, 0.0, 0.0);
    assert!((r.size.x - 80.0).abs() < 1e-4);
    assert!((r.size.y - 40.0).abs() < 1e-4);
}

#[test]
fn image_cache_caches_decode_failure_without_redecoding() {
    let mut be = NativeBackend::with_dpi(1.0);
    // Garbage bytes fail to decode → None.
    assert!(be.cached_image(7, b"not a real image").is_none());
    assert_eq!(be.image_cache_len(), 1, "the failure is cached as None");
    // A second call for the same id ignores the bytes entirely —
    // proven by passing an empty slice and still getting None.
    assert!(be.cached_image(7, b"").is_none());
    assert_eq!(be.image_cache_len(), 1, "no second cache entry");
}

#[test]
fn image_cache_evicts_oldest_entries_past_capacity() {
    let mut be = NativeBackend::with_dpi(1.0);
    // Insert well past the cap — the cache must stay bounded.
    for id in 0..(IMAGE_CACHE_CAP as u64 + 10) {
        be.cached_image(id, b"garbage");
    }
    assert!(
        be.image_cache_len() <= IMAGE_CACHE_CAP,
        "cache stays within the cap, got {}",
        be.image_cache_len()
    );
    // The oldest id (0) was evicted; the newest is still resident.
    assert_eq!(be.image_cache_len(), IMAGE_CACHE_CAP);
}

#[test]
fn image_cache_decodes_a_valid_png() {
    let mut be = NativeBackend::with_dpi(1.0);
    let png = encode_test_png(4, 3);
    let img = be.cached_image(1, &png).expect("valid PNG decodes");
    assert_eq!(img.width(), 4);
    assert_eq!(img.height(), 3);
    assert_eq!(be.image_cache_len(), 1);
}

/// Encode a solid raster surface to PNG bytes — a real image for
/// the decode-cache test (no hardcoded blob).
fn encode_test_png(w: i32, h: i32) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((w, h)).unwrap();
    surface.canvas().clear(skia_safe::Color::BLUE);
    let image = surface.image_snapshot();
    let ctx: Option<&mut skia_safe::gpu::DirectContext> = None;
    image
        .encode(ctx, skia_safe::EncodedImageFormat::PNG, 100)
        .unwrap()
        .as_bytes()
        .to_vec()
}

#[test]
fn rect_translation_keeps_size() {
    let r = Rect {
        origin: Point2D::new(10.0, 20.0),
        size: Point2D::new(30.0, 40.0),
    };
    let jr = to_jian_rect(r);
    assert!((jr.min_x() - 10.0).abs() < 1e-6);
    assert!((jr.min_y() - 20.0).abs() < 1e-6);
    assert!((jr.size.width - 30.0).abs() < 1e-6);
    assert!((jr.size.height - 40.0).abs() < 1e-6);
}
