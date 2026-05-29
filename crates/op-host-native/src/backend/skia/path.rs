use super::{jian_color_to_color4f, to_sk_rect, NativeBackend};
use op_editor_ui::{Color, Point2D, Rect};
use std::hash::{Hash, Hasher};

const SVG_PATH_CACHE_CAP: usize = 2048;
const SVG_RASTER_CACHE_CAP: usize = 256;
const SVG_RASTER_COMPLEXITY_MIN: usize = 4096;
const SVG_RASTER_MAX_PIXELS: i32 = 2_000_000;
const SVG_RASTER_PAD: f32 = 2.0;

pub(super) struct SvgPathCacheEntry {
    d: String,
    path: Option<skia_safe::Path>,
    even_odd: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SvgRasterKey {
    path_key: u64,
    d_len: usize,
    color_rgba: u32,
    size_bits: u32,
    viewbox_bits: u32,
    dpi_bits: u32,
}

pub(super) struct SvgRasterCacheEntry {
    image: skia_safe::Image,
    offset: Point2D,
    size: Point2D,
}

struct SvgRasterRequest<'a> {
    path_key: u64,
    d_len: usize,
    path: &'a skia_safe::Path,
    even_odd: bool,
    size: f32,
    viewbox: f32,
    color: Color,
}

impl NativeBackend {
    fn cached_svg_path(&mut self, d: &str) -> Option<(u64, skia_safe::Path, bool)> {
        let key = svg_path_key(d);
        if let Some(entry) = self.svg_path_cache.get(&key) {
            if entry.d == d {
                return entry.path.clone().map(|path| (key, path, entry.even_odd));
            }
        }

        let path = skia_safe::utils::parse_path::from_svg(d);
        let even_odd = path.is_some() && has_multiple_close_commands(d);
        let was_present = self.svg_path_cache.contains_key(&key);
        self.svg_path_cache.insert(
            key,
            SvgPathCacheEntry {
                d: d.to_string(),
                path: path.clone(),
                even_odd,
            },
        );
        if !was_present {
            self.svg_path_cache_order.push_back(key);
        }
        while self.svg_path_cache.len() > SVG_PATH_CACHE_CAP {
            match self.svg_path_cache_order.pop_front() {
                Some(oldest) => {
                    self.svg_path_cache.remove(&oldest);
                }
                None => break,
            }
        }

        path.map(|path| (key, path, even_odd))
    }

    fn cached_raster_svg_path(
        &mut self,
        req: SvgRasterRequest<'_>,
    ) -> Option<&SvgRasterCacheEntry> {
        let SvgRasterRequest {
            path_key,
            d_len,
            path,
            even_odd,
            size,
            viewbox,
            color,
        } = req;
        if d_len < SVG_RASTER_COMPLEXITY_MIN || !size.is_finite() || !viewbox.is_finite() {
            return None;
        }
        let s = size / viewbox;
        if s <= 0.0 || !s.is_finite() {
            return None;
        }
        let dpi = if self.dpi.is_finite() && self.dpi > 0.0 {
            self.dpi
        } else {
            1.0
        };
        let raster_s = s * dpi;
        let pad_px = (SVG_RASTER_PAD * dpi).ceil();
        let bounds = path.compute_tight_bounds();
        if !bounds.is_finite() || bounds.is_empty() {
            return None;
        }
        let width = (bounds.width() * raster_s).abs().ceil() as i32 + (pad_px as i32 * 2);
        let height = (bounds.height() * raster_s).abs().ceil() as i32 + (pad_px as i32 * 2);
        if width <= 0 || height <= 0 || width.saturating_mul(height) > SVG_RASTER_MAX_PIXELS {
            return None;
        }

        let key = SvgRasterKey {
            path_key,
            d_len,
            color_rgba: color_key(color),
            size_bits: size.to_bits(),
            viewbox_bits: viewbox.to_bits(),
            dpi_bits: dpi.to_bits(),
        };
        if self.svg_raster_cache.contains_key(&key) {
            return self.svg_raster_cache.get(&key);
        }

        let mut surface = skia_safe::surfaces::raster_n32_premul((width, height))?;
        let raster_canvas = surface.canvas();
        raster_canvas.clear(skia_safe::Color::TRANSPARENT);
        let save = raster_canvas.save();
        raster_canvas.translate((
            pad_px - bounds.left() * raster_s,
            pad_px - bounds.top() * raster_s,
        ));
        raster_canvas.scale((raster_s, raster_s));
        let mut raster_path = path.clone();
        if even_odd {
            raster_path.set_fill_type(skia_safe::PathFillType::EvenOdd);
        }
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        raster_canvas.draw_path(&raster_path, &paint);
        raster_canvas.restore_to_count(save);

        let entry = SvgRasterCacheEntry {
            image: surface.image_snapshot(),
            offset: Point2D::new(
                bounds.left() * s - pad_px / dpi,
                bounds.top() * s - pad_px / dpi,
            ),
            size: Point2D::new(width as f32 / dpi, height as f32 / dpi),
        };
        self.svg_raster_cache.insert(key, entry);
        self.svg_raster_cache_order.push_back(key);
        while self.svg_raster_cache.len() > SVG_RASTER_CACHE_CAP {
            match self.svg_raster_cache_order.pop_front() {
                Some(oldest) => {
                    self.svg_raster_cache.remove(&oldest);
                }
                None => break,
            }
        }
        self.svg_raster_cache.get(&key)
    }

    /// Step 5 SVG icons: parse an SVG path `d` string once, scale from
    /// a 24x24 viewBox to `size x size` at `top_left`, and stroke it
    /// with round caps + joins.
    pub fn stroke_svg_path(
        &mut self,
        canvas: &skia_safe::Canvas,
        d: &str,
        top_left: Point2D,
        size: f32,
        color: Color,
        width: f32,
    ) {
        let Some((_, path, _)) = self.cached_svg_path(d) else {
            return;
        };
        let s = size / 24.0;
        let mut matrix = skia_safe::Matrix::new_identity();
        matrix.set_scale_translate((s, s), (top_left.x, top_left.y));
        let path = path.with_transform(&matrix);
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        paint.set_stroke_cap(skia_safe::PaintCap::Round);
        paint.set_stroke_join(skia_safe::PaintJoin::Round);
        canvas.draw_path(&path, &paint);
    }

    /// Stroke an arbitrary document SVG path fitted into `rect` by
    /// its own tight bounds. This mirrors the TS renderer's
    /// `drawPath` transform and handles Figma vectors whose `d`
    /// contains negative local coordinates.
    pub fn stroke_svg_path_in_rect(
        &mut self,
        canvas: &skia_safe::Canvas,
        d: &str,
        rect: Rect,
        color: Color,
        width: f32,
    ) {
        let Some((_, path, _)) = self.cached_svg_path(d) else {
            return;
        };
        let path = fit_path_to_rect(&path, rect);
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        paint.set_stroke_cap(skia_safe::PaintCap::Round);
        paint.set_stroke_join(skia_safe::PaintJoin::Round);
        canvas.draw_path(&path, &paint);
    }

    /// Fill an SVG path scaled from `viewbox x viewbox` to
    /// `size x size`. The parsed base path is cached so Figma imports
    /// with many vector paths do not re-parse every paint frame.
    pub fn fill_svg_path(
        &mut self,
        canvas: &skia_safe::Canvas,
        d: &str,
        top_left: Point2D,
        size: f32,
        viewbox: f32,
        color: Color,
    ) {
        let Some((path_key, path, even_odd)) = self.cached_svg_path(d) else {
            return;
        };
        if let Some(raster) = self.cached_raster_svg_path(SvgRasterRequest {
            path_key,
            d_len: d.len(),
            path: &path,
            even_odd,
            size,
            viewbox,
            color,
        }) {
            let dst = Rect {
                origin: Point2D::new(top_left.x + raster.offset.x, top_left.y + raster.offset.y),
                size: raster.size,
            };
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            canvas.draw_image_rect(&raster.image, None, to_sk_rect(dst), &paint);
            return;
        }
        let s = size / viewbox;
        let mut matrix = skia_safe::Matrix::new_identity();
        matrix.set_scale_translate((s, s), (top_left.x, top_left.y));
        let mut path = path.with_transform(&matrix);
        if even_odd {
            path.set_fill_type(skia_safe::PathFillType::EvenOdd);
        }
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        canvas.draw_path(&path, &paint);
    }

    /// Fill an arbitrary document SVG path fitted into `rect` by its
    /// own tight bounds. Unlike `fill_svg_path`, this is not tied to
    /// a square icon viewBox.
    pub fn fill_svg_path_in_rect(
        &mut self,
        canvas: &skia_safe::Canvas,
        d: &str,
        rect: Rect,
        color: Color,
    ) {
        let Some((_, path, even_odd)) = self.cached_svg_path(d) else {
            return;
        };
        let mut path = fit_path_to_rect(&path, rect);
        if even_odd {
            path.set_fill_type(skia_safe::PathFillType::EvenOdd);
        }
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        canvas.draw_path(&path, &paint);
    }

    /// Fill a document SVG path (fitted into `rect`) with a linear
    /// gradient. Shader endpoints come from `rect` — the same
    /// unrotated AABB the path is fitted into — so when the caller
    /// has a rotation on the canvas matrix the gradient rotates with
    /// the path. Reuses the rect-gradient helpers so native paths and
    /// rects agree on direction + colour interpolation.
    pub fn fill_svg_path_in_rect_linear_gradient(
        &mut self,
        canvas: &skia_safe::Canvas,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        if stops.is_empty() {
            return;
        }
        let Some(path) = self.fitted_svg_path(d, rect) else {
            return;
        };
        let (start, end) = super::gradient::linear_gradient_endpoints(rect, angle_deg);
        let (colors, offsets) = super::gradient::gradient_color_arrays(stops, opacity);
        let grad_colors = skia_safe::gradient::Colors::new(
            &colors[..],
            Some(offsets.as_slice()),
            skia_safe::TileMode::Clamp,
            None,
        );
        let gradient = skia_safe::gradient::Gradient::new(
            grad_colors,
            super::gradient::premul_interpolation(),
        );
        let shader = skia_safe::gradient::shaders::linear_gradient((start, end), &gradient, None);
        self.draw_path_with_gradient(canvas, &path, shader, stops, opacity);
    }

    /// Fill a document SVG path (fitted into `rect`) with a radial
    /// gradient. Centre / outer-radius fractions match the rect
    /// radial helper (`pen-renderer` parity).
    #[allow(clippy::too_many_arguments)]
    pub fn fill_svg_path_in_rect_radial_gradient(
        &mut self,
        canvas: &skia_safe::Canvas,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    ) {
        if stops.is_empty() {
            return;
        }
        let Some(path) = self.fitted_svg_path(d, rect) else {
            return;
        };
        let center = skia_safe::Point::new(
            rect.origin.x + rect.size.x * cx_frac.clamp(0.0, 1.0),
            rect.origin.y + rect.size.y * cy_frac.clamp(0.0, 1.0),
        );
        let outer = ((rect.size.x.max(rect.size.y)) * radius_frac.clamp(0.0, 1.0)).max(0.01);
        let (colors, offsets) = super::gradient::gradient_color_arrays(stops, opacity);
        let grad_colors = skia_safe::gradient::Colors::new(
            &colors[..],
            Some(offsets.as_slice()),
            skia_safe::TileMode::Clamp,
            None,
        );
        let gradient = skia_safe::gradient::Gradient::new(
            grad_colors,
            super::gradient::premul_interpolation(),
        );
        let shader =
            skia_safe::gradient::shaders::radial_gradient((center, outer), &gradient, None);
        self.draw_path_with_gradient(canvas, &path, shader, stops, opacity);
    }

    /// Resolve `d` to a path fitted into `rect`, honouring the cached
    /// even-odd fill rule. `None` when the path string fails to parse.
    fn fitted_svg_path(&mut self, d: &str, rect: Rect) -> Option<skia_safe::Path> {
        let (_, path, even_odd) = self.cached_svg_path(d)?;
        let mut path = fit_path_to_rect(&path, rect);
        if even_odd {
            path.set_fill_type(skia_safe::PathFillType::EvenOdd);
        }
        Some(path)
    }

    /// Draw `path` with `shader`, or degrade to a solid first-stop
    /// fill when skia rejected the (degenerate) gradient.
    fn draw_path_with_gradient(
        &self,
        canvas: &skia_safe::Canvas,
        path: &skia_safe::Path,
        shader: Option<skia_safe::Shader>,
        stops: &[(f32, Color)],
        opacity: f32,
    ) {
        let mut paint = match shader {
            Some(shader) => {
                let mut p = skia_safe::Paint::default();
                p.set_shader(shader);
                p
            }
            None => skia_safe::Paint::new(
                jian_color_to_color4f(super::gradient::fold_opacity(stops[0].1, opacity)),
                None,
            ),
        };
        paint.set_anti_alias(true);
        canvas.draw_path(path, &paint);
    }

    /// Paint an inset (inner) shadow for a document SVG path fitted
    /// into `rect`. Clips to the path silhouette, then draws an
    /// inverse fill (everything *outside* the shape, shifted by the
    /// shadow offset) through a blur mask: the blurred boundary
    /// bleeds inward from the edge and the clip keeps only the
    /// inside, yielding a recessed shadow. `sigma = blur * 0.5`
    /// matches `fill_drop_shadow`.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_inner_shadow_svg_path(
        &mut self,
        canvas: &skia_safe::Canvas,
        d: &str,
        rect: Rect,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        color: Color,
    ) {
        let Some(path) = self.fitted_svg_path(d, rect) else {
            return;
        };
        // Standard inset-shadow recipe, isolated in its own layer so
        // the punch-out never touches the backdrop:
        //   1. fill the whole silhouette with the shadow colour,
        //   2. punch out a blurred, offset copy with `DstOut` — the
        //      blur's soft edge leaves a feathered shadow ring hugging
        //      the inside of the boundary while the centre clears.
        // The outer clip keeps everything inside the silhouette.
        let offset_path = if offset_x != 0.0 || offset_y != 0.0 {
            let mut m = skia_safe::Matrix::new_identity();
            m.set_translate((offset_x, offset_y));
            path.with_transform(&m)
        } else {
            path.clone()
        };

        canvas.save();
        canvas.clip_path(&path, skia_safe::ClipOp::Intersect, true);
        canvas.save_layer(&skia_safe::canvas::SaveLayerRec::default());

        let mut fill = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        fill.set_anti_alias(true);
        canvas.draw_path(&path, &fill);

        let mut cut = skia_safe::Paint::default();
        cut.set_anti_alias(true);
        cut.set_blend_mode(skia_safe::BlendMode::DstOut);
        let sigma = blur * 0.5;
        if sigma > 0.0 {
            if let Some(mask) =
                skia_safe::MaskFilter::blur(skia_safe::BlurStyle::Normal, sigma, false)
            {
                cut.set_mask_filter(mask);
            }
        }
        canvas.draw_path(&offset_path, &cut);

        canvas.restore(); // pop the isolation layer
        canvas.restore(); // pop the silhouette clip
    }

    #[cfg(test)]
    pub(crate) fn svg_path_cache_len(&self) -> usize {
        self.svg_path_cache.len()
    }

    #[cfg(test)]
    pub(crate) fn svg_raster_cache_len(&self) -> usize {
        self.svg_raster_cache.len()
    }
}

fn svg_path_key(d: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    d.hash(&mut hasher);
    hasher.finish()
}

fn has_multiple_close_commands(d: &str) -> bool {
    let mut closes = 0;
    for byte in d.bytes() {
        if byte == b'Z' || byte == b'z' {
            closes += 1;
            if closes > 1 {
                return true;
            }
        }
    }
    false
}

fn fit_path_to_rect(path: &skia_safe::Path, rect: Rect) -> skia_safe::Path {
    let bounds = path.compute_tight_bounds();
    if !bounds.is_finite()
        || !rect.size.x.is_finite()
        || !rect.size.y.is_finite()
        || rect.size.x <= 0.0
        || rect.size.y <= 0.0
    {
        let mut matrix = skia_safe::Matrix::new_identity();
        matrix.set_translate((rect.origin.x, rect.origin.y));
        return path.with_transform(&matrix);
    }
    let native_w = bounds.width();
    let native_h = bounds.height();
    let sx = if native_w.abs() > 0.01 {
        rect.size.x / native_w
    } else {
        1.0
    };
    let sy = if native_h.abs() > 0.01 {
        rect.size.y / native_h
    } else {
        1.0
    };
    let tx = rect.origin.x - bounds.left() * sx;
    let ty = rect.origin.y - bounds.top() * sy;
    let mut matrix = skia_safe::Matrix::new_identity();
    matrix.set_scale_translate((sx, sy), (tx, ty));
    path.with_transform(&matrix)
}

fn color_key(c: Color) -> u32 {
    fn ch(v: f32) -> u32 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u32
    }
    (ch(c.r) << 24) | (ch(c.g) << 16) | (ch(c.b) << 8) | ch(c.a)
}
