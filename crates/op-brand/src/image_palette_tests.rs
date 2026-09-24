use super::*;
use image::{ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

/// A synthetic "landing page" screenshot: off-white page, a dark headline
/// bar, a coloured CTA block, a second brand colour badge, and a card.
pub(crate) fn landing_png(bg: [u8; 3], ink: [u8; 3], cta: [u8; 3], badge: [u8; 3]) -> Vec<u8> {
    let (w, h) = (320u32, 200u32);
    let img = RgbaImage::from_fn(w, h, |x, y| {
        let px = |c: [u8; 3]| Rgba([c[0], c[1], c[2], 255]);
        let headline = (40..280).contains(&x) && (30..44).contains(&y);
        let body_line = (40..200).contains(&x) && (52..58).contains(&y);
        if headline || body_line {
            px(ink)
        } else if (40..130).contains(&x) && (80..110).contains(&y) {
            px(cta) // primary button
        } else if (150..175).contains(&x) && (88..102).contains(&y) {
            px(badge) // secondary brand badge
        } else if (200..300).contains(&x) && (120..190).contains(&y) {
            px([
                bg[0].saturating_sub(18),
                bg[1].saturating_sub(18),
                bg[2].saturating_sub(16),
            ])
        } else {
            px(bg)
        }
    });
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png)
        .expect("encode png");
    out.into_inner()
}

#[test]
fn quantises_a_generated_png_deterministically() {
    let png = landing_png([250, 248, 244], [24, 24, 32], [230, 80, 20], [20, 120, 200]);
    let a = palette_from_image(&png).unwrap();
    let b = palette_from_image(&png).unwrap();
    assert_eq!(a, b, "same bytes, same palette");
    assert!(a[0].share > 0.5, "the page ground dominates");
    let total: f64 = a.iter().map(|s| s.share).sum();
    assert!((total - 1.0).abs() < 1e-6);
}

#[test]
fn reads_roles_off_the_swatches() {
    let png = landing_png([250, 248, 244], [24, 24, 32], [230, 80, 20], [20, 120, 200]);
    let signals = signals_from_swatches(&palette_from_image(&png).unwrap());
    let mode = &signals.base;
    let bg = mode.background.unwrap();
    assert!(bg.distance(Rgb::new(250, 248, 244)) < 12.0, "{bg:?}");
    let ink = mode.foreground.unwrap();
    assert!(ink.distance(Rgb::new(24, 24, 32)) < 20.0, "{ink:?}");
    let primary = mode.primary.unwrap();
    assert!(
        primary.distance(Rgb::new(230, 80, 20)) < 30.0,
        "{primary:?}"
    );
    assert!(
        mode.extra_brand
            .iter()
            .any(|c| c.distance(Rgb::new(20, 120, 200)) < 30.0),
        "second brand colour kept: {:?}",
        mode.extra_brand
    );
    let card = mode.card.expect("card tone off the ground");
    assert!(card.contrast(bg) < 1.25);
}

#[test]
fn monochrome_screenshot_has_no_primary() {
    let png = landing_png([255, 255, 255], [17, 17, 17], [40, 40, 40], [90, 90, 90]);
    let signals = signals_from_swatches(&palette_from_image(&png).unwrap());
    assert_eq!(signals.base.primary, None);
    assert!(signals.base.is_usable());
}

#[test]
fn rejects_non_images() {
    assert!(matches!(
        palette_from_image(b"not an image"),
        Err(BrandError::ImageDecode { .. })
    ));
}
