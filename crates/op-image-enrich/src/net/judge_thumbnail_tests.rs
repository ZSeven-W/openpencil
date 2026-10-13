use super::*;

pub(super) fn portrait_bands() -> Vec<u8> {
    let image = image::RgbImage::from_fn(960, 1440, |_, y| {
        if y < 400 {
            image::Rgb([240, 10, 10])
        } else if y > 1040 {
            image::Rgb([10, 10, 240])
        } else {
            image::Rgb([10, 240, 10])
        }
    });
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    png.into_inner()
}

fn preview(fit: ImageJudgeFit) -> image::RgbImage {
    let view = ImageJudgeView::new(132.0, 74.0, fit).unwrap();
    let encoded = make_judge_thumbnail_for_view(&portrait_bands(), Some(view)).unwrap();
    image::load_from_memory(&encoded).unwrap().to_rgb8()
}

#[test]
fn real_landscape_card_sees_only_center_of_portrait() {
    let crop = preview(ImageJudgeFit::Cover);
    assert_eq!(crop.dimensions(), (512, 287));
    // The 960x1440 source's top/bottom action markers are outside the real
    // y=450.91..989.09 cover crop and must not be visible to the judge.
    for y in [0, 143, 286] {
        let [r, g, b] = crop.get_pixel(256, y).0;
        assert!(g > 220 && r < 30 && b < 30, "{r},{g},{b}");
    }
}

#[test]
fn contain_preserves_top_and_bottom_with_slot_letterboxing() {
    let fit = preview(ImageJudgeFit::Contain);
    assert_eq!(fit.dimensions(), (512, 287));
    assert!(fit.get_pixel(256, 20)[0] > 220);
    assert!(fit.get_pixel(256, 270)[2] > 220);
    assert!(fit.get_pixel(10, 143).0.iter().all(|c| *c > 240));
}

#[test]
fn stretch_keeps_whole_source_without_letterboxing() {
    let stretched = preview(ImageJudgeFit::Stretch);
    assert!(stretched.get_pixel(10, 20)[0] > 220);
    assert!(stretched.get_pixel(10, 270)[2] > 220);
}

#[test]
fn unknown_view_retains_full_source_preview() {
    let bytes = portrait_bands();
    let encoded = make_judge_thumbnail_for_view(&bytes, None).unwrap();
    assert_eq!(
        encoded,
        super::super::downscale::make_judge_thumbnail(&bytes).unwrap()
    );
    let decoded = image::load_from_memory(&encoded).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (341, 512));
}

#[test]
fn judge_surface_is_bounded_even_for_extreme_slot_ratios() {
    let source = portrait_bands();
    for (w, h) in [(1e200, 1.0), (1.0, 1e200)] {
        let view = ImageJudgeView::new(w, h, ImageJudgeFit::Cover).unwrap();
        let encoded = make_judge_thumbnail_for_view(&source, Some(view)).unwrap();
        let decoded = image::load_from_memory(&encoded).unwrap();
        assert!(decoded.width() <= 512 && decoded.height() <= 512);
    }
}

#[test]
fn oversized_compressed_source_is_rejected_before_raster_allocation() {
    // The PNG codec's dimensions reader uses the header. A valid small PNG
    // patched to claim enormous dimensions must never allocate that raster.
    let mut bytes = portrait_bands();
    bytes[16..20].copy_from_slice(&100_000_u32.to_be_bytes());
    bytes[20..24].copy_from_slice(&100_000_u32.to_be_bytes());
    let view = ImageJudgeView::new(132.0, 74.0, ImageJudgeFit::Cover).unwrap();
    assert!(make_judge_thumbnail_for_view(&bytes, Some(view)).is_none());
}
