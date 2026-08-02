//! Complex-script paint + measure routing.
//!
//! Arabic must reach the Skia paragraph shaper (bidi reordering + contextual
//! joining) instead of the 1:1 cmap fast path, while Latin must stay on the
//! fast path so the documented chrome frame budget is unaffected.
//!
//! macOS-gated for the same reason the sibling font-fallback tests are: they
//! assert against system typefaces, and a bare Linux CI box has no Arabic
//! face to resolve.

use super::*;
use op_editor_ui::TextLayout;

const SIZE: f32 = 40.0;

/// Raw unshaped width — skia's default per-codepoint advances with no
/// joining applied. This is exactly what the cmap fast path produces, so it
/// is the oracle a genuinely shaped measurement has to diverge from.
#[cfg(target_os = "macos")]
fn isolated_width(be: &mut NativeBackend, text: &str) -> f32 {
    let tf = be
        .typeface_for_char(text.chars().next().expect("non-empty sample"), 400)
        .expect("the system covers the sample script");
    let font = skia_safe::Font::new(tf, SIZE);
    font.measure_str(text, None).0
}

/// Ink mass either side of the painted bounding box's own midpoint.
/// Normalising against the ink box rather than the surface keeps the
/// assertion independent of where the run happens to be anchored.
#[cfg(target_os = "macos")]
fn ink_halves(pixels: &skia_safe::Pixmap, w: i32, h: i32) -> (usize, usize) {
    let inked = |x: i32, y: i32| {
        let c = pixels.get_color((x, y));
        c.r() < 245 || c.g() < 245 || c.b() < 245
    };
    let mut min_x = w;
    let mut max_x = -1;
    for y in 0..h {
        for x in 0..w {
            if inked(x, y) {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
    }
    assert!(max_x > min_x, "the sample must paint visible ink");
    let mid = (min_x + max_x) / 2;
    let (mut left, mut right) = (0usize, 0usize);
    for y in 0..h {
        for x in min_x..=max_x {
            if inked(x, y) {
                if x < mid {
                    left += 1;
                } else {
                    right += 1;
                }
            }
        }
    }
    (left, right)
}

#[cfg(target_os = "macos")]
#[test]
fn arabic_measures_with_joining_applied() {
    let mut be = NativeBackend::with_dpi(1.0);

    // Arabic letters join into a connected form. Isolated letterforms carry
    // entry/exit flourishes that the joined run drops, so a shaped width is
    // strictly narrower than the sum of default advances.
    let shaped = be.measure_text_family_styled("مرحبا", SIZE, "", 400, false);
    let isolated = isolated_width(&mut be, "مرحبا");

    assert!(shaped > 0.0, "Arabic must measure to a real width");
    assert!(
        shaped < isolated,
        "joined Arabic ({shaped}) should be narrower than isolated ({isolated})"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn latin_measurement_stays_on_the_unshaped_fast_path() {
    let mut be = NativeBackend::with_dpi(1.0);

    // Guard on the perf fork, not a driver: routing *all* text through the
    // shaper (the alternative design) would change Latin advances via
    // kerning and break this. Latin must keep matching raw cmap advances.
    let measured = be.measure_text_family_styled("Design System", SIZE, "", 400, false);
    let raw = isolated_width(&mut be, "Design System");

    assert!(
        (measured - raw).abs() < 0.01,
        "Latin must stay unshaped: measured {measured}, raw {raw}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn arabic_paints_right_to_left() {
    let mut be = NativeBackend::with_dpi(1.0);

    // Six meems then a lone alef. Meems are dense and round, alef is a thin
    // vertical stroke, so the ink is heavily lopsided toward the meems.
    // Arabic reads right-to-left, so the meems — first in storage order —
    // must paint on the RIGHT. Painting them left is precisely the
    // logical-order bug this routing exists to fix.
    const TEXT: &str = "مممممم ا";
    let (w, h) = (480, 120);

    let mut surface = skia_safe::surfaces::raster_n32_premul((w, h)).unwrap();
    surface.canvas().clear(skia_safe::Color::WHITE);
    let layout = TextLayout::single_run(TEXT, "", SIZE, Color::BLACK.to_jian(), Point2D::ZERO);
    be.draw_text(surface.canvas(), &layout, Point2D::new(20.0, 80.0));

    let image = surface.image_snapshot();
    let pixels = image.peek_pixels().expect("peek raster pixels");
    let (left, right) = ink_halves(&pixels, w, h);

    assert!(
        right > left,
        "meems must paint right of the alef (left ink {left}, right ink {right})"
    );
}

/// Visual proof sheet — not an assertion. Renders real strings from a live
/// document through the fixed `draw_text` alongside the raw `draw_str` call
/// the old code made, so the two can be compared by eye. Ignored by default.
///   cargo test -p op-host-native --lib arabic_proof_sheet -- --ignored
#[cfg(target_os = "macos")]
#[test]
#[ignore]
fn arabic_proof_sheet() {
    let mut be = NativeBackend::with_dpi(1.0);
    let (w, h) = (1040, 460);
    let mut surface = skia_safe::surfaces::raster_n32_premul((w, h)).unwrap();
    surface.canvas().clear(skia_safe::Color::WHITE);

    let label = |be: &mut NativeBackend, canvas: &skia_safe::Canvas, s: &str, x: f32, y: f32| {
        let l = TextLayout::single_run(s, "", 18.0, Color::BLACK.to_jian(), Point2D::ZERO);
        be.draw_text(canvas, &l, Point2D::new(x, y));
    };
    label(&mut be, surface.canvas(), "NEW - shaped + bidi", 30.0, 34.0);
    label(&mut be, surface.canvas(), "OLD - raw draw_str", 560.0, 34.0);

    // Generic samples, each covering a distinct rendering feature:
    // plain joining, the definite article, a shadda diacritic, and digits
    // beside an Arabic unit abbreviation ("50 km").
    let samples = ["مرحبا بالعالم", "اللغة العربية", "معلّم اللغة", "50 ك.م"];
    let size = 34.0;
    for (i, s) in samples.iter().enumerate() {
        let y = 110.0 + (i as f32) * 88.0;
        let layout = TextLayout::single_run(s, "", size, Color::BLACK.to_jian(), Point2D::ZERO);
        be.draw_text(surface.canvas(), &layout, Point2D::new(30.0, y));

        let tf = be
            .typeface_for_char(s.chars().next().unwrap(), 400)
            .expect("Arabic face");
        let font = skia_safe::Font::new(tf, size);
        let mut p = skia_safe::Paint::new(skia_safe::Color4f::new(0.75, 0.1, 0.1, 1.0), None);
        p.set_anti_alias(true);
        surface.canvas().draw_str(s, (560.0, y), &font, &p);
    }

    let image = surface.image_snapshot();
    let data = image
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .expect("encode png");
    let out = std::env::temp_dir().join("openpencil-arabic-proof.png");
    std::fs::write(&out, data.as_bytes()).expect("write png");
    println!("proof sheet written to {}", out.display());
}
