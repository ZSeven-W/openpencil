//! Judge-only raster preparation matching the centered slot sampling modes.
//! The downloaded/full asset is kept separately and is never cropped here.

use skia_safe::{surfaces, CubicResampler, Data, EncodedImageFormat, Image, Paint, Rect};

use crate::{ImageJudgeFit, ImageJudgeView};

const MAX_EDGE: f64 = 512.0;
const MAX_SOURCE_PIXELS: u64 = 16_777_216;

pub fn make_judge_thumbnail_for_view(
    bytes: &[u8],
    view: Option<ImageJudgeView>,
) -> Option<Vec<u8>> {
    let Some(view) = view else {
        return super::downscale::make_judge_thumbnail(bytes);
    };
    // Inspect dimensions before either decoder materializes source pixels.
    // Provider bodies are capped separately; a tiny compressed image must not
    // expand into an unbounded raster. The output is at most 512 x 512.
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let (width, height) = reader.into_dimensions().ok()?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
        return None;
    }
    let src = Image::from_encoded(Data::new_copy(bytes)).or_else(|| {
        let decoded = image::load_from_memory(bytes).ok()?;
        let mut png = std::io::Cursor::new(Vec::new());
        decoded.write_to(&mut png, image::ImageFormat::Png).ok()?;
        Image::from_encoded(Data::new_copy(png.get_ref()))
    })?;
    let (w, h) = (f64::from(src.width()), f64::from(src.height()));
    // Preserve source detail up to the judge budget; a 132px card is judged
    // at a legible preview size, not enlarged from a tiny rendered screenshot.
    let (ow, oh) = if view.ratio() >= 1.0 {
        (MAX_EDGE, (MAX_EDGE / view.ratio()).max(1.0))
    } else {
        ((MAX_EDGE * view.ratio()).max(1.0), MAX_EDGE)
    };
    let (ow, oh) = (ow.round() as i32, oh.round() as i32);
    let mut surface = surfaces::raster_n32_premul((ow, oh))?;
    surface.canvas().clear(skia_safe::Color::WHITE);
    let (dw, dh) = match view.fit() {
        ImageJudgeFit::Cover | ImageJudgeFit::Contain => {
            let scale = if view.fit() == ImageJudgeFit::Cover {
                (ow as f64 / w).max(oh as f64 / h)
            } else {
                (ow as f64 / w).min(oh as f64 / h)
            };
            (w * scale, h * scale)
        }
        ImageJudgeFit::Stretch => (ow as f64, oh as f64),
    };
    let dst = Rect::from_xywh(
        ((ow as f64 - dw) / 2.0) as f32,
        ((oh as f64 - dh) / 2.0) as f32,
        dw as f32,
        dh as f32,
    );
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    surface.canvas().draw_image_rect_with_sampling_options(
        &src,
        None,
        dst,
        CubicResampler::mitchell(),
        &paint,
    );
    surface
        .image_snapshot()
        .encode(None, EncodedImageFormat::JPEG, 78)
        .map(|data| data.as_bytes().to_vec())
}

#[cfg(test)]
#[path = "judge_thumbnail_tests.rs"]
mod tests;
