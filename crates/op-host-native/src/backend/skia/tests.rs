//! Unit tests for [`super`]'s `NativeBackend` — colour packing,
//! rect conversion, and the `draw_image` aspect-fit + decode cache.
//! Split into a sibling file to keep `skia.rs` under the 800-line cap.

use super::*;
use op_editor_ui::TextLayout;

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
fn cover_rect_crops_square_image_vertically_in_wide_rect() {
    let outer = Rect::xywh(0.0, 0.0, 360.0, 240.0);
    let r = cover_rect(outer, 200.0, 200.0);
    assert!((r.size.x - 360.0).abs() < 1e-4);
    assert!((r.size.y - 360.0).abs() < 1e-4);
    assert!((r.origin.x - 0.0).abs() < 1e-4);
    assert!(
        (r.origin.y + 60.0).abs() < 1e-4,
        "center-cropped vertically"
    );
}

#[test]
fn image_adjustment_matrix_matches_ts_formula() {
    let matrix = image_adjustment_matrix(op_editor_ui::ImageAdjustments {
        exposure: 100.0,
        contrast: -100.0,
        saturation: 100.0,
        temperature: 100.0,
        tint: -100.0,
        highlights: 100.0,
        shadows: 100.0,
    })
    .expect("non-neutral adjustments produce a color matrix");

    // With contrast = -100%, c = 0, so every RGB multiplier is zero.
    // The visible change comes from the TS offset formula.
    assert!((matrix[0] - 0.0).abs() < 1e-6);
    assert!((matrix[5] - 0.0).abs() < 1e-6);
    assert!((matrix[10] - 0.0).abs() < 1e-6);
    assert!((matrix[4] - 0.80).abs() < 1e-6);
    assert!((matrix[9] - 0.50).abs() < 1e-6);
    assert!((matrix[14] - 0.50).abs() < 1e-6);
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

#[test]
fn explicit_family_typeface_lookup_is_cached() {
    let mut be = NativeBackend::with_dpi(1.0);
    assert_eq!(be.family_typeface_cache_len(), 0);

    let first = be
        .typeface_for_family_char('A', "Georgia", 400)
        .map(|tf| tf.unique_id());
    assert_eq!(be.family_typeface_cache_len(), 1);

    let second = be
        .typeface_for_family_char('A', "Georgia", 400)
        .map(|tf| tf.unique_id());
    assert_eq!(second, first);
    assert_eq!(be.family_typeface_cache_len(), 1);
}

#[test]
fn svg_path_cache_reuses_parsed_paths() {
    let mut be = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((32, 32)).unwrap();
    let canvas = surface.canvas();
    let d = "M0 0 L10 0 L10 10 Z";

    be.fill_svg_path(canvas, d, Point2D::ZERO, 1.0, 1.0, Color::BLACK);
    assert_eq!(be.svg_path_cache_len(), 1);

    be.fill_svg_path(canvas, d, Point2D::new(4.0, 4.0), 2.0, 1.0, Color::RED);
    assert_eq!(be.svg_path_cache_len(), 1);
}

#[test]
fn complex_svg_fill_uses_raster_cache_after_first_paint() {
    let mut be = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((128, 128)).unwrap();
    let canvas = surface.canvas();
    let d = format!("M0 0 L64 0 L64 64 L0 64 Z{}", " ".repeat(4096));

    be.fill_svg_path(canvas, &d, Point2D::ZERO, 1.0, 1.0, Color::BLACK);
    assert_eq!(be.svg_path_cache_len(), 1);
    assert_eq!(be.svg_raster_cache_len(), 1);

    be.fill_svg_path(canvas, &d, Point2D::new(8.0, 8.0), 1.0, 1.0, Color::BLACK);
    assert_eq!(be.svg_path_cache_len(), 1);
    assert_eq!(be.svg_raster_cache_len(), 1);
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
fn linear_gradient_angle_zero_runs_bottom_to_top() {
    // The canonical `.op` convention puts `angle = 0` at "from
    // bottom to top" (CSS `to-top`). Mirrors the TS renderer at
    // `pen-renderer/src/node-renderer.ts:155` which subtracts 90°
    // before projecting onto endpoints.
    let rect = Rect::xywh(0.0, 0.0, 100.0, 50.0);
    let (start, end) = super::gradient::linear_gradient_endpoints(rect, 0.0);
    // Start at the bottom edge centre, end at the top edge centre.
    assert!((start.x - 50.0).abs() < 1e-3, "start x={}", start.x);
    assert!((start.y - 25.0 - 25.0).abs() < 1e-3, "start y={}", start.y);
    assert!((end.x - 50.0).abs() < 1e-3, "end x={}", end.x);
    assert!((end.y - 25.0 + 25.0).abs() < 1e-3, "end y={}", end.y);
}

#[test]
fn linear_gradient_angle_ninety_runs_left_to_right() {
    // `angle = 90` → horizontal, left to right.
    let rect = Rect::xywh(0.0, 0.0, 100.0, 50.0);
    let (start, end) = super::gradient::linear_gradient_endpoints(rect, 90.0);
    assert!((start.x - 0.0).abs() < 1e-3, "start x={}", start.x);
    assert!((start.y - 25.0).abs() < 1e-3, "start y={}", start.y);
    assert!((end.x - 100.0).abs() < 1e-3, "end x={}", end.x);
    assert!((end.y - 25.0).abs() < 1e-3, "end y={}", end.y);
}

#[test]
fn linear_gradient_endpoints_use_ellipse_not_aabb() {
    // At 45°, endpoints must sit on the bounding ellipse — NOT on
    // the AABB diagonal. The earlier AABB-projection trick gave a
    // longer gradient line that diverged from the TS renderer.
    let rect = Rect::xywh(0.0, 0.0, 200.0, 100.0);
    let (start, end) = super::gradient::linear_gradient_endpoints(rect, 45.0);
    // 45° in canonical convention = (angle - 90 = -45°) in screen
    // convention. cos(-45°) = √2/2, sin(-45°) = -√2/2.
    // dx = (√2/2) * 100 ≈ 70.71, dy = (-√2/2) * 50 ≈ -35.36.
    let dx_expected = 200.0 * 0.5 * 0.5_f32.sqrt();
    let dy_expected = -100.0 * 0.5 * 0.5_f32.sqrt();
    assert!((start.x - (100.0 - dx_expected)).abs() < 1e-2);
    assert!((start.y - (50.0 - dy_expected)).abs() < 1e-2);
    assert!((end.x - (100.0 + dx_expected)).abs() < 1e-2);
    assert!((end.y - (50.0 + dy_expected)).abs() < 1e-2);
}

#[test]
fn linear_gradient_path_renders_color_ramp() {
    // A full-rect square path filled with a left→right gradient
    // (white at offset 0, red at offset 1; angle 90° = left→right)
    // must paint a real ramp: the left edge stays green-ish (white),
    // the right edge loses green (red). A solid first-stop fallback
    // would paint the whole path white and fail the assert.
    let mut be = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((40, 40)).unwrap();
    surface.canvas().clear(skia_safe::Color::BLACK);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(40.0, 40.0),
    };
    let stops = [
        (
            0.0,
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
        ),
        (
            1.0,
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        ),
    ];
    be.fill_svg_path_in_rect_linear_gradient(
        surface.canvas(),
        "M0 0 L1 0 L1 1 L0 1 Z",
        rect,
        &stops,
        90.0,
        1.0,
    );
    let img = surface.image_snapshot();
    let pm = img.peek_pixels().expect("peek raster pixels");
    let left = pm.get_color((3, 20));
    let right = pm.get_color((37, 20));
    assert!(
        left.g() as i32 > right.g() as i32 + 60,
        "expected a left→right ramp (left greener than right), got left.g={} right.g={}",
        left.g(),
        right.g()
    );
}

#[test]
fn inner_shadow_path_darkens_edges_not_center() {
    // A full-rect square path with a black inset shadow (offset 0,
    // blur 8) must darken the inside edges while the centre stays
    // near-white. A no-op (or outer-shadow) fallback would leave the
    // edge as bright as the centre.
    let mut be = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((60, 60)).unwrap();
    surface.canvas().clear(skia_safe::Color::WHITE);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(60.0, 60.0),
    };
    let d = "M0 0 L1 0 L1 1 L0 1 Z";
    be.fill_inner_shadow_svg_path(
        surface.canvas(),
        d,
        rect,
        0.0,
        0.0,
        8.0,
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    );
    let img = surface.image_snapshot();
    let pm = img.peek_pixels().expect("peek raster pixels");
    let edge = pm.get_color((2, 30));
    let center = pm.get_color((30, 30));
    assert!(
        (edge.r() as i32) < (center.r() as i32) - 30,
        "inset shadow should darken the edge vs centre: edge.r={} center.r={}",
        edge.r(),
        center.r()
    );
}

#[test]
fn image_draw_respects_node_opacity() {
    // A solid-blue image drawn at 0.5 opacity over white must blend
    // toward white (≈ 50% each); full opacity would leave it pure blue.
    let mut be = NativeBackend::with_dpi(1.0);
    let png = encode_test_png(8, 8);
    let mut surface = skia_safe::surfaces::raster_n32_premul((20, 20)).unwrap();
    surface.canvas().clear(skia_safe::Color::WHITE);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(20.0, 20.0),
    };
    be.draw_image_with_options(
        surface.canvas(),
        rect,
        4242,
        &png,
        op_editor_ui::ImageDrawMode::Stretch,
        op_editor_ui::ImageAdjustments::default(),
        0.5,
        0.0,
    );
    let img = surface.image_snapshot();
    let pm = img.peek_pixels().expect("peek raster pixels");
    let c = pm.get_color((10, 10));
    // 0.5 blue over white ≈ (128,128,255); full opacity would be r=0.
    assert!(
        c.r() > 80 && c.r() < 200,
        "image opacity should blend toward white, got r={}",
        c.r()
    );
    assert!(
        c.b() > 200,
        "blue channel should stay high, got b={}",
        c.b()
    );
}

#[test]
fn korean_hangul_resolves_to_a_covering_typeface() {
    // Regression: Hangul routed to the shared CJK face (resolved from
    // a Chinese ideograph) which lacks Hangul glyphs, so 한국어 painted
    // blank. The resolved face for '한' must actually cover '한'.
    let mut be = NativeBackend::with_dpi(1.0);
    let tf = be.typeface_for_char('한', 400).expect("a typeface for 한");
    assert_ne!(
        tf.unichar_to_glyph('한' as i32),
        0,
        "the resolved Hangul face must have a glyph for 한"
    );
    // Every char of the Korean locale name resolves to a covering face.
    for c in "한국어".chars() {
        let tf = be.typeface_for_char(c, 400).expect("typeface");
        assert_ne!(tf.unichar_to_glyph(c as i32), 0, "missing glyph for {c}");
    }
}

#[test]
fn draw_text_runs_borrows_layout_storage() {
    let layout = TextLayout::single_run(
        "Hello",
        "system-ui",
        13.0,
        Color::BLACK.to_jian(),
        Point2D::ZERO,
    );

    let runs = super::text::draw_text_runs(&layout);

    assert_eq!(
        runs.as_ptr(),
        layout.runs().as_ptr(),
        "native text drawing should not clone TextLayout runs per paint"
    );
}
