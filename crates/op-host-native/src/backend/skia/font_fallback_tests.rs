//! Native typeface fallback and direct-text paint regressions.

use super::*;
use op_editor_ui::TextLayout;

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
fn cyrillic_design_text_uses_covering_fallback_and_shared_resolver() {
    let _guard = crate::font_registry_test_support::lock();
    const TEXT: &str = "Hello Привет";
    const FAMILY: &str = "Roboto";
    const FONT_SIZE: f32 = 32.0;

    let mut be = NativeBackend::with_dpi(1.0);
    let cache_before_measure = be.family_typeface_cache_len();
    let measured_width = be.measure_text_family(TEXT, FONT_SIZE, FAMILY);
    let cache_after_measure = be.family_typeface_cache_len();
    assert!(
        cache_after_measure > cache_before_measure,
        "measurement must resolve the requested-family glyphs"
    );
    let prefix_width = be.measure_text_family("Hello ", FONT_SIZE, FAMILY);
    assert!(measured_width > prefix_width);
    assert_eq!(
        be.family_typeface_cache_len(),
        cache_after_measure,
        "measuring a cached prefix must not add resolver entries"
    );

    let segments = be
        .font_resolver
        .segment_text(TEXT, Some(FAMILY), 400, false);
    let rendered: String = segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect();
    assert_eq!(rendered, TEXT, "segmentation must preserve the full text");
    for segment in &segments {
        if !segment.text.is_ascii() {
            for c in segment.text.chars() {
                assert_ne!(
                    segment.typeface.unichar_to_glyph(c as i32),
                    0,
                    "segment typeface must cover {c:?} in {:?}",
                    segment.text
                );
            }
        }
    }
    assert!(
        segments.iter().any(|segment| segment.text.contains('П')),
        "segmentation must retain the Cyrillic run"
    );
    assert_eq!(
        be.family_typeface_cache_len(),
        cache_after_measure,
        "segmentation must reuse measurement's resolver entries"
    );

    let mut surface = skia_safe::surfaces::raster_n32_premul((320, 80)).unwrap();
    surface.canvas().clear(skia_safe::Color::WHITE);
    let layout = TextLayout::single_run(
        TEXT,
        FAMILY,
        FONT_SIZE,
        Color::BLACK.to_jian(),
        Point2D::ZERO,
    );
    let origin = Point2D::new(8.0, 48.0);
    be.draw_text(surface.canvas(), &layout, origin);
    assert_eq!(
        be.family_typeface_cache_len(),
        cache_after_measure,
        "paint must reuse measurement's resolver entries"
    );

    let image = surface.image_snapshot();
    let pixels = image.peek_pixels().expect("peek raster pixels");
    let cyrillic_start = (origin.x + prefix_width).floor() as i32;
    let mut non_white = 0usize;
    for y in 0..80 {
        for x in cyrillic_start..320 {
            let color = pixels.get_color((x, y));
            if color.r() < 245 || color.g() < 245 || color.b() < 245 {
                non_white += 1;
            }
        }
    }
    assert!(
        non_white > 20,
        "the Cyrillic portion should paint visible pixels, got {non_white}"
    );
}

#[test]
fn emoji_resolves_to_a_covering_emoji_typeface() {
    let mut be = NativeBackend::with_dpi(1.0);
    let tf = be.typeface_for_char('🍕', 400).expect("a typeface for 🍕");
    assert_ne!(
        tf.unichar_to_glyph('🍕' as i32),
        0,
        "the resolved emoji face must have a glyph for 🍕"
    );
    let family = tf.family_name().to_lowercase();
    assert!(
        family.contains("emoji"),
        "emoji should resolve to an emoji-capable font, got {family}"
    );
}

#[test]
fn emoji_text_paints_colored_pixels() {
    let mut be = NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((96, 96)).unwrap();
    surface.canvas().clear(skia_safe::Color::WHITE);
    let layout = TextLayout::single_run(
        "🍕",
        "system-ui",
        48.0,
        Color::BLACK.to_jian(),
        Point2D::ZERO,
    );

    be.draw_text(surface.canvas(), &layout, Point2D::new(16.0, 64.0));

    let img = surface.image_snapshot();
    let pm = img.peek_pixels().expect("peek raster pixels");
    let mut non_white = 0usize;
    let mut saturated = 0usize;
    for y in 0..96 {
        for x in 0..96 {
            let c = pm.get_color((x, y));
            let r = c.r();
            let g = c.g();
            let b = c.b();
            if r < 245 || g < 245 || b < 245 {
                non_white += 1;
                let max = r.max(g).max(b);
                let min = r.min(g).min(b);
                if max.saturating_sub(min) > 40 {
                    saturated += 1;
                }
            }
        }
    }
    assert!(
        non_white > 20,
        "emoji should paint visible pixels, got {non_white}"
    );
    assert!(
        saturated > 20,
        "emoji should paint colorful pixels, got saturated={saturated} non_white={non_white}"
    );
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
