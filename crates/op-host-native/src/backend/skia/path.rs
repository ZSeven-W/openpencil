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

fn color_key(c: Color) -> u32 {
    fn ch(v: f32) -> u32 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u32
    }
    (ch(c.r) << 24) | (ch(c.g) << 16) | (ch(c.b) << 8) | ch(c.a)
}
