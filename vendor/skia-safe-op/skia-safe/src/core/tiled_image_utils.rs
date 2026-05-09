use skia_bindings::{self as sb, C_SkTiledImageUtils_DrawImageRect};

use crate::{canvas, prelude::*, scalar, Canvas, Image, Paint, Point, Rect, SamplingOptions};

pub fn draw_image_rect(
    canvas: &Canvas,
    image: &Image,
    src: impl AsRef<Rect>,
    dst: impl AsRef<Rect>,
    sampling: Option<SamplingOptions>,
    paint: Option<&Paint>,
    constraint: impl Into<Option<canvas::SrcRectConstraint>>,
) {
    let sampling = sampling.unwrap_or_default();
    let constraint = constraint.into().unwrap_or(canvas::SrcRectConstraint::Fast);
    unsafe {
        C_SkTiledImageUtils_DrawImageRect(
            canvas.native_mut(),
            image.native(),
            src.as_ref().native(),
            dst.as_ref().native(),
            sampling.native(),
            paint.native_ptr_or_null(),
            constraint,
        )
    }
}

pub fn draw_image(
    canvas: &Canvas,
    image: &Image,
    xy: impl Into<Point>,
    sampling: Option<SamplingOptions>,
    paint: Option<&Paint>,
    constraint: impl Into<Option<canvas::SrcRectConstraint>>,
) {
    let p = xy.into();
    let src = Rect::from_iwh(image.width(), image.height());
    let dst = Rect::from_xywh(p.x, p.y, image.width() as scalar, image.height() as scalar);

    draw_image_rect(canvas, image, src, dst, sampling, paint, constraint)
}

pub const NUM_IMAGE_KEY_VALUES: usize = 6;

pub fn get_image_key_values(image: &Image) -> [u32; NUM_IMAGE_KEY_VALUES] {
    let mut key_values = [0u32; NUM_IMAGE_KEY_VALUES];
    unsafe { sb::C_SkTiledImageUtils_GetImageKeyValues(image.native(), key_values.as_mut_ptr()) }
    key_values
}
