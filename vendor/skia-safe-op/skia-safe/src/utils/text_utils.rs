use skia_bindings::SkTextUtils;

use crate::{prelude::*, Canvas, EncodedText, Font, Paint, Path, Point};

pub use skia_bindings::SkTextUtils_Align as Align;
variant_name!(Align::Center);

pub fn draw_str(
    canvas: &Canvas,
    text: impl AsRef<str>,
    p: impl Into<Point>,
    font: &Font,
    paint: &Paint,
    align: Align,
) {
    draw_text(canvas, text.as_ref(), p, font, paint, align)
}

pub fn draw_text(
    canvas: &Canvas,
    text: impl EncodedText,
    p: impl Into<Point>,
    font: &Font,
    paint: &Paint,
    align: Align,
) {
    let (ptr, size, encoding) = text.as_raw();
    let p = p.into();
    unsafe {
        SkTextUtils::Draw(
            canvas.native_mut(),
            ptr,
            size,
            encoding.into_native(),
            p.x,
            p.y,
            font.native(),
            paint.native(),
            align,
        )
    }
}

impl Canvas {
    pub fn draw_str_align(
        &self,
        text: impl AsRef<str>,
        p: impl Into<Point>,
        font: &Font,
        paint: &Paint,
        align: Align,
    ) -> &Self {
        self.draw_text_align(text.as_ref(), p, font, paint, align)
    }

    pub fn draw_text_align(
        &self,
        text: impl EncodedText,
        p: impl Into<Point>,
        font: &Font,
        paint: &Paint,
        align: Align,
    ) -> &Self {
        draw_text(self, text, p, font, paint, align);
        self
    }
}

pub fn get_path(text: impl EncodedText, p: impl Into<Point>, font: &Font) -> Path {
    let (ptr, size, encoding) = text.as_raw();
    let p = p.into();
    let mut path = Path::default();
    unsafe {
        SkTextUtils::GetPath(
            ptr,
            size,
            encoding.into_native(),
            p.x,
            p.y,
            font.native(),
            path.native_mut(),
        )
    }
    path
}

impl Path {
    pub fn from_str(text: impl AsRef<str>, p: impl Into<Point>, font: &Font) -> Self {
        get_path(text.as_ref(), p, font)
    }
}
