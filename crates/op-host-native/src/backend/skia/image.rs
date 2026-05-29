use super::{contain_rect, to_sk_rect, NativeBackend};
use op_editor_ui::{ImageAdjustments, ImageDrawMode, Point2D, Rect};

/// Aspect-cover (`fill` / `crop`) `img_w × img_h` over `outer`, centered.
/// The result fully covers `outer`; the caller clips to `outer`.
pub(super) fn cover_rect(outer: Rect, img_w: f32, img_h: f32) -> Rect {
    if img_w <= 0.0 || img_h <= 0.0 || outer.size.x <= 0.0 || outer.size.y <= 0.0 {
        return outer;
    }
    let scale = (outer.size.x / img_w).max(outer.size.y / img_h);
    let w = img_w * scale;
    let h = img_h * scale;
    Rect {
        origin: Point2D::new(
            outer.origin.x + (outer.size.x - w) / 2.0,
            outer.origin.y + (outer.size.y - h) / 2.0,
        ),
        size: Point2D::new(w, h),
    }
}

impl NativeBackend {
    /// Draw the image identified by `id`, aspect-fit + centered
    /// inside `rect`. Kept as the legacy/default image path for chat
    /// attachments and UI previews.
    pub fn draw_image(&mut self, canvas: &skia_safe::Canvas, rect: Rect, id: u64, encoded: &[u8]) {
        self.draw_image_with_mode(canvas, rect, id, encoded, ImageDrawMode::Fit);
    }

    /// Draw the image identified by `id` using the same placement
    /// modes as the TS renderer's image fill path.
    pub fn draw_image_with_mode(
        &mut self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
    ) {
        self.draw_image_with_options(
            canvas,
            rect,
            id,
            encoded,
            mode,
            ImageAdjustments::default(),
            1.0,
        );
    }

    /// Draw the image identified by `id` using placement and
    /// adjustment controls from the image-fill popover.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_image_with_options(
        &mut self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
        adjustments: ImageAdjustments,
        opacity: f32,
    ) {
        let Some(image) = self.cached_image(id, encoded) else {
            return;
        };
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        // Node-level opacity dims the raster (rasters carry no fill
        // colour to bake the opacity into at scene-build).
        paint.set_alpha_f(opacity.clamp(0.0, 1.0));
        if let Some(matrix) = image_adjustment_matrix(adjustments) {
            paint.set_color_filter(skia_safe::color_filters::matrix_row_major(&matrix, None));
        }

        match mode {
            ImageDrawMode::Fit => {
                let dst = contain_rect(rect, image.width() as f32, image.height() as f32);
                canvas.draw_image_rect(&image, None, to_sk_rect(dst), &paint);
            }
            ImageDrawMode::Stretch => {
                canvas.draw_image_rect(&image, None, to_sk_rect(rect), &paint);
            }
            ImageDrawMode::Tile => {
                draw_tiled_image(canvas, rect, &image, &paint);
            }
            ImageDrawMode::Fill | ImageDrawMode::Crop => {
                let dst = cover_rect(rect, image.width() as f32, image.height() as f32);
                let save = canvas.save();
                canvas.clip_rect(to_sk_rect(rect), None, Some(true));
                canvas.draw_image_rect(&image, None, to_sk_rect(dst), &paint);
                canvas.restore_to_count(save);
            }
        }
    }
}

pub(super) fn image_adjustment_matrix(adjustments: ImageAdjustments) -> Option<[f32; 20]> {
    let exp = adjustments.exposure / 100.0;
    let con = adjustments.contrast / 100.0;
    let sat = adjustments.saturation / 100.0;
    let temp = adjustments.temperature / 100.0;
    let tint = adjustments.tint / 100.0;
    let hi = adjustments.highlights / 100.0;
    let sh = adjustments.shadows / 100.0;
    if adjustments.is_neutral() {
        return None;
    }

    let e = 1.0 + exp * 1.5;
    let c = 1.0 + con;
    let c_off = 0.5 * (1.0 - c);
    let s = 1.0 + sat;
    let (lr, lg, lb) = (0.2126, 0.7152, 0.0722);
    let sr = (1.0 - s) * lr;
    let sg = (1.0 - s) * lg;
    let sb = (1.0 - s) * lb;
    let f = c * e;
    let off_r = c_off + temp * 0.15 + (hi + sh * 0.5) * 0.1;
    let off_g = c_off + tint * 0.15 + (hi + sh * 0.5) * 0.1;
    let off_b = c_off - temp * 0.15 + (hi + sh * 0.5) * 0.1;

    Some([
        f * (sr + s),
        f * sg,
        f * sb,
        0.0,
        off_r,
        f * sr,
        f * (sg + s),
        f * sb,
        0.0,
        off_g,
        f * sr,
        f * sg,
        f * (sb + s),
        0.0,
        off_b,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
    ])
}

fn draw_tiled_image(
    canvas: &skia_safe::Canvas,
    rect: Rect,
    image: &skia_safe::Image,
    paint: &skia_safe::Paint,
) {
    let tile_w = image.width() as f32;
    let tile_h = image.height() as f32;
    if tile_w <= 0.0 || tile_h <= 0.0 || rect.size.x <= 0.0 || rect.size.y <= 0.0 {
        return;
    }

    let save = canvas.save();
    canvas.clip_rect(to_sk_rect(rect), None, Some(true));

    let mut start_x = rect.origin.x + (rect.size.x - tile_w) / 2.0;
    let mut start_y = rect.origin.y + (rect.size.y - tile_h) / 2.0;
    while start_x > rect.origin.x {
        start_x -= tile_w;
    }
    while start_y > rect.origin.y {
        start_y -= tile_h;
    }

    let right = rect.origin.x + rect.size.x;
    let bottom = rect.origin.y + rect.size.y;
    let mut y = start_y;
    while y < bottom {
        let mut x = start_x;
        while x < right {
            canvas.draw_image_rect(
                image,
                None,
                to_sk_rect(Rect::xywh(x, y, tile_w, tile_h)),
                paint,
            );
            x += tile_w;
        }
        y += tile_h;
    }

    canvas.restore_to_count(save);
}
