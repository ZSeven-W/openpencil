//! Spec v19 / plan v7 Task 2 Step 16e: confirm `NativeBackend::draw_text`
//! actually rasterises glyphs through `jian-skia`'s `textlayout`
//! feature path. We don't pixel-match specific glyph shapes (font
//! manager output is platform-specific) — we just check that at least
//! one non-white pixel lands inside the layout's expected bounding box
//! after a black-on-white draw.

use jian_core::scene::Color as JianColor;
use op_editor_ui::{Point2D, TextLayout};
use op_host_native::NativeBackend;

fn count_dark_pixels(pixels: &[u8], stride: usize, x0: i32, y0: i32, x1: i32, y1: i32) -> usize {
    let mut darks = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let off = (y as usize) * stride + (x as usize) * 4;
            // Treat anything with all RGB < 240 as a glyph hit. Glyph
            // anti-aliasing produces lots of grays; pure white = no
            // hit; saturated colour or near-black = hit.
            if pixels[off] < 240 && pixels[off + 1] < 240 && pixels[off + 2] < 240 {
                darks += 1;
            }
        }
    }
    darks
}

#[test]
fn ascii_and_cjk_text_paints_visible_glyphs() {
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((400, 100)).expect("raster_n32_premul allocated");
    surface.canvas().clear(skia_safe::Color::WHITE);

    let mut backend = NativeBackend::with_dpi(1.0);
    let canvas = surface.canvas();

    // Black "Hello 你好" — exercises both single-line ASCII and the
    // CJK path that `jian-skia`'s `textlayout` ParagraphBuilder owns
    // (single-line fallback also handles ASCII fine; the CJK glyphs
    // pin the textlayout feature being live).
    let layout = TextLayout::single_run(
        "Hello 你好",
        "",
        24.0,
        JianColor::rgb(0, 0, 0),
        Point2D::new(10.0, 30.0),
    );
    backend.draw_text(canvas, &layout, Point2D::ZERO);

    // Read pixels.
    let info = skia_safe::ImageInfo::new(
        (400, 100),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    let stride = 400 * 4;
    let mut pixels = vec![0u8; stride * 100];
    let ok = surface.read_pixels(&info, &mut pixels, stride, (0, 0));
    assert!(ok, "raster read_pixels must succeed");

    // Look for at least N dark pixels inside the band (10, 10)..(380, 70).
    // 24-px text starting at y=30 leaves an ascent ~24 px above the
    // baseline; the band is generous enough to absorb font-manager
    // variation.
    let darks = count_dark_pixels(&pixels, stride, 10, 10, 380, 70);
    assert!(
        darks > 50,
        "expected glyph rasterisation in expected band, got {darks} dark pixels"
    );
}
