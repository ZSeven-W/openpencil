//! Unit tests for [`super`]'s `NativeBackend` — colour packing,
//! rect conversion, and the `draw_image` aspect-fit + decode cache.
//! Split into a sibling file to keep `skia.rs` under the 800-line cap.

use super::*;
use jian_core::layout::measure::{FontStyleKind, MeasureBackend, MeasureRequest, StyledRun};

#[path = "tests/image_cache.rs"]
mod image_cache;
#[path = "tests/shader.rs"]
mod shader;
#[path = "tests/text_shaping.rs"]
mod text_shaping;
#[path = "tests/vector_paint.rs"]
mod vector_paint;

#[test]
fn color_roundtrip_clamps_and_packs() {
    let red = (Color::RED).to_jian();
    assert_eq!(red.r(), 255);
    assert_eq!(red.g(), 0);
    assert_eq!(red.b(), 0);
    assert_eq!(red.a(), 255);

    let transparent = (Color::TRANSPARENT).to_jian();
    assert_eq!(transparent.a(), 0);

    // Out-of-range channels are clamped, not wrapped.
    let weird = (Color {
        r: -0.5,
        g: 2.0,
        b: 0.5,
        a: 1.0,
    })
    .to_jian();
    assert_eq!(weird.r(), 0);
    assert_eq!(weird.g(), 255);
    assert_eq!(weird.b(), 128);
}

#[test]
fn dot_point_buffer_reuses_capacity_between_batches() {
    let mut be = NativeBackend::with_dpi(1.0);
    let large: Vec<Point2D> = (0..256).map(|i| Point2D::new(i as f32, 0.0)).collect();
    let small = [Point2D::new(1.0, 2.0), Point2D::new(3.0, 4.0)];

    assert_eq!(be.prepare_dot_points(&large).len(), 256);
    let large_capacity = be.dot_point_buffer.capacity();
    assert!(large_capacity >= 256);

    assert_eq!(be.prepare_dot_points(&small).len(), 2);
    assert_eq!(
        be.dot_point_buffer.capacity(),
        large_capacity,
        "native grid dot conversion should reuse its allocation across frames"
    );
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

#[test]
fn system_font_resolver_accepts_enumerated_names_and_rejects_unknown_ones() {
    let Some(installed) = enumerate_system_font_families().into_iter().next() else {
        return;
    };
    let unknown = "OpenPencil Definitely Missing Font 4E8A1B27".to_string();
    let candidates = vec![installed.clone(), unknown, "bad\0family".to_string()];

    assert_eq!(
        resolvable_system_font_families(&candidates),
        vec![installed]
    );
}

#[cfg(target_os = "windows")]
#[test]
fn directwrite_resolves_both_yahei_faces_when_collection_is_installed() {
    let installed = enumerate_system_font_families();
    if !installed.iter().any(|family| {
        family.eq_ignore_ascii_case("Microsoft YaHei")
            || family.eq_ignore_ascii_case("Microsoft YaHei UI")
    }) {
        return;
    }
    let candidates = vec![
        "Microsoft YaHei".to_string(),
        "Microsoft YaHei UI".to_string(),
    ];

    assert_eq!(resolvable_system_font_families(&candidates), candidates);
}
