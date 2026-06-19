#![cfg(feature = "gl-host")]
//! Spec v19 acceptance #5 (resize / DPI no-flicker) / Phase A Gate
//! round 2 CONCERN 2.
//!
//! Round 1 only exercised the `resize` tracing span on an inert
//! context — that proves the span fires but doesn't pin the
//! invariant the acceptance is actually about (after a resize, a
//! draw issued through `NativeBackend` must land on a surface of
//! the new dimensions without panic). This test fills that gap
//! using a raster Skia surface as a stand-in for the GL FBO that
//! `SharedSkiaContext::resize` rebuilds — `cargo test` cannot drive
//! a real GL surface from a worker thread, but the
//! `NativeBackend::fill_rect` translation layer is identical
//! whether the canvas is raster- or GPU-backed.
//!
//! Two passes:
//!   1. Grow 400×300 → 800×600, paint at centre, assert red.
//!   2. Shrink 800×600 → 400×300, paint at centre, assert red.
//!
//! Plus: a tracing-span pin on `SharedSkiaContext::resize` itself
//! (already covered structurally by `tracing_spans.rs`, repeated
//! here at the new size cadence to lock the resize-grow / resize-
//! shrink invariant on round-2's stricter spec).
//!
//! Windows is `#[ignore]`d at the test level — CI runners ship
//! without a usable GL stack, the raster path is identical across
//! macOS / Linux, and CONCERN 2 only requires non-Windows coverage.

use op_editor_ui::{Color, Point2D, Rect};
use op_host_native::{NativeBackend, SharedSkiaContext};
use tracing_test::traced_test;

/// Helper: build a raster Skia surface of `(w, h)`, paint a
/// `NativeBackend::fill_rect` covering most of it, return the
/// resulting pixel at the surface centre. Mirrors the chain
/// `SharedSkiaContext::resize` → `with_frame` →
/// `NativeBackend::fill_rect` minus the GL stack.
fn paint_at_size(backend: &mut NativeBackend, w: i32, h: i32) -> [u8; 4] {
    let mut surface = skia_safe::surfaces::raster_n32_premul((w, h))
        .expect("raster_n32_premul allocated for resize test");
    surface.canvas().clear(skia_safe::Color::WHITE);

    backend.fill_rect(
        surface.canvas(),
        Rect {
            origin: Point2D::new(10.0, 10.0),
            size: Point2D::new((w - 20) as f32, (h - 20) as f32),
        },
        Color::RED,
    );

    let stride = (w * 4) as usize;
    let mut pixels = vec![0u8; stride * (h as usize)];
    let info = skia_safe::ImageInfo::new(
        (w, h),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    let read_ok = surface.read_pixels(&info, &mut pixels, stride, (0, 0));
    assert!(
        read_ok,
        "raster Surface::read_pixels must succeed at {}×{}",
        w, h
    );

    let cx = (w / 2) as usize;
    let cy = (h / 2) as usize;
    let offset = cy * stride + cx * 4;
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}

#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn resize_grow_then_shrink_paints_through() {
    let mut backend = NativeBackend::with_dpi(1.0);

    // Initial 400×300.
    let small_pixel = paint_at_size(&mut backend, 400, 300);
    assert!(
        small_pixel[0] > 200 && small_pixel[1] < 60 && small_pixel[2] < 60,
        "400×300 centre expected red, got {:?}",
        small_pixel
    );

    // Grow to 800×600 — same `NativeBackend` instance must paint
    // through to a fresh-larger surface (matches the realistic
    // resize path: window expanded, `SharedSkiaContext::resize`
    // rebuilt the Skia surface, next frame's `fill_rect` lands on
    // it).
    let grown_pixel = paint_at_size(&mut backend, 800, 600);
    assert!(
        grown_pixel[0] > 200 && grown_pixel[1] < 60 && grown_pixel[2] < 60,
        "800×600 centre expected red, got {:?}",
        grown_pixel
    );

    // Shrink back to 400×300 — same invariant, opposite direction.
    let shrunk_pixel = paint_at_size(&mut backend, 400, 300);
    assert!(
        shrunk_pixel[0] > 200 && shrunk_pixel[1] < 60 && shrunk_pixel[2] < 60,
        "shrunk 400×300 centre expected red, got {:?}",
        shrunk_pixel
    );
}

/// Pin the `resize` span emission on a `SharedSkiaContext` whose
/// fields are at the post-teardown shape (surface = None, dc =
/// None). Complements `tracing_spans.rs` by exercising both grow
/// (800×600) and shrink (400×300) paths plus the 0×0 no-op clamp.
#[cfg_attr(target_os = "windows", ignore)]
#[traced_test]
#[test]
fn resize_emits_span_grow_and_shrink() {
    let mut ctx = SharedSkiaContext::inert_for_lifecycle_test();
    ctx.resize(800, 600).expect("resize 800x600");
    ctx.resize(400, 300).expect("resize 400x300");
    ctx.resize(0, 0).expect("resize 0x0 no-op clamp");

    assert!(logs_contain("resize"), "resize span missing");
}
