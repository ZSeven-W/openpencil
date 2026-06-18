//! `NativeBackend` — frame-scoped widget facade backed by `jian_skia`.
//!
//! Spec v19 §5.2.1 (round 3 BLOCK-R3-3 fix): `NativeBackend` does **not**
//! own a canvas borrow and does **not** carry a `'a` lifetime. Every
//! drawing method takes `canvas: &skia_safe::Canvas` (immutable —
//! skia-safe 0.97 `Canvas::*` are all `&self`) and forwards the work to
//! `jian_skia::SkiaBackend::draw_on_canvas`. Callers obtain the borrow
//! inside `SharedSkiaContext::with_frame(|canvas, glow| { … })` and pass
//! it through.
//!
//! The backend deliberately does **not** `impl RenderBackend` for OP's
//! widget-facing trait (spec §5.2.1 mirror surface, no direct impl in
//! Step 1a; saved for Step 1c+ widget tree).

use op_editor_ui::{Color, Point2D, Rect};

/// OP `Rect` (`origin / size: Vec2`) → Jian `euclid::Rect<f32>`.
pub fn to_jian_rect(r: Rect) -> jian_core::geometry::Rect {
    jian_core::geometry::Rect::new(
        jian_core::geometry::Point::new(r.origin.x, r.origin.y),
        jian_core::geometry::Size::new(r.size.x, r.size.y),
    )
}

/// `skia_safe::Rect` from an OP `Rect` — used by `clip_rect`.
/// Aspect-fit (`contain`) `img_w × img_h` inside `outer`, centered.
/// The result never exceeds `outer` on either axis; a degenerate
/// (zero-dimension) image size returns `outer` unchanged so the
/// caller's frame still paints something. Used by `draw_image` so
/// the chat transcript can hand a plain box and let the backend
/// (which knows the decoded dimensions) place the picture.
fn contain_rect(outer: Rect, img_w: f32, img_h: f32) -> Rect {
    if img_w <= 0.0 || img_h <= 0.0 || outer.size.x <= 0.0 || outer.size.y <= 0.0 {
        return outer;
    }
    let scale = (outer.size.x / img_w).min(outer.size.y / img_h);
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

// `pub(super)` so the sibling `gradient.rs` impl block can convert
// rects without re-implementing the helper.
pub(super) fn to_sk_rect(r: Rect) -> skia_safe::Rect {
    skia_safe::Rect::from_xywh(r.origin.x, r.origin.y, r.size.x, r.size.y)
}

/// OP `Color` → `skia_safe::Color4f` — used by the direct-canvas
/// helpers (stroke_line / fill_round_rect / stroke_round_rect)
/// that skip the jian DrawOp pipeline.
pub(super) fn jian_color_to_color4f(c: Color) -> skia_safe::Color4f {
    skia_safe::Color4f::new(
        c.r.clamp(0.0, 1.0),
        c.g.clamp(0.0, 1.0),
        c.b.clamp(0.0, 1.0),
        c.a.clamp(0.0, 1.0),
    )
}

// Gradient shader paint methods (`fill_round_rect_linear_gradient`
// / `_radial_gradient`) and their helpers live in the sibling
// `gradient.rs` so this spine stays under the 800-line cap. The
// methods are added to `NativeBackend` via a sibling `impl` block.
mod font_script;
mod gradient;
mod image;
mod path;
mod text;
#[cfg(test)]
use image::{cover_rect, image_adjustment_matrix};

/// Frame-scoped Jian-DrawOp adapter (spec v19 §5.2.1).
///
/// The struct stays alive across frames so its underlying
/// `jian_skia::SkiaBackend` keeps its image cache; the canvas borrow is
/// passed in per-method so `&NativeBackend` and `&Canvas` can coexist
/// inside the same `with_frame` closure without aliasing.
pub struct NativeBackend {
    skia: jian_skia::SkiaBackend,
    dpi: f32,
    font_mgr: skia_safe::FontMgr,
    /// Lazy-initialised typeface backed by the embedded Roboto TTF
    /// (shared with shell-web). Step 4 perf fix: jian-skia's
    /// `textlayout` path allocates a fresh `FontCollection` +
    /// `FontMgr` per `DrawOp::Text` (~15ms each on M1), so a chrome
    /// frame with ~30 text draws cost ~600ms. We bypass that path
    /// entirely for chrome by caching typefaces here and rendering
    /// via `Canvas::draw_str`.
    ///
    /// `typeface` covers ASCII (Roboto). `cjk_typeface` is resolved
    /// lazily via `FontMgr::match_family_style_character` so non-
    /// ASCII labels (localized CJK chrome strings) render through a
    /// system CJK font (PingFang on macOS,
    /// Noto CJK on Linux/Windows) without re-paying jian-skia's
    /// per-call FontCollection cost.
    typeface: Option<skia_safe::Typeface>,
    typeface_tried: bool,
    cjk_typeface: Option<skia_safe::Typeface>,
    cjk_typeface_tried: bool,
    /// Korean (Hangul) face — resolved separately from `cjk_typeface`
    /// because the Han-ideograph match returns a Chinese font without
    /// Hangul coverage.
    korean_typeface: Option<skia_safe::Typeface>,
    korean_typeface_tried: bool,
    /// Default-family per-codepoint typeface cache, keyed by
    /// `(codepoint, weight, italic)` — the slant bit keeps italic
    /// resolutions from shadowing upright ones.
    char_typeface_cache: std::collections::HashMap<(i32, u16, bool), Option<skia_safe::Typeface>>,
    /// Explicit-family typeface cache for selected text / font picker
    /// previews, keyed by `(primary family, codepoint, weight, italic)`.
    family_typeface_cache:
        std::collections::HashMap<(String, i32, u16, bool), Option<skia_safe::Typeface>>,
    /// Decoded-image cache keyed by [`op_editor_core::ChatImage::id`].
    /// `draw_image` decodes the raw bytes once on first sight; later
    /// frames reuse the cached `Image`. A decode failure is cached as
    /// `None` so a corrupt attachment isn't re-decoded every frame.
    /// Bounded to [`IMAGE_CACHE_CAP`] entries — `image_cache_order`
    /// tracks insertion order so the oldest decode is evicted once
    /// the cap is exceeded (a long image-heavy chat can't grow the
    /// decoded-image set without bound; an evicted image re-decodes
    /// on its next paint).
    image_cache: std::collections::HashMap<u64, Option<skia_safe::Image>>,
    image_cache_order: std::collections::VecDeque<u64>,
    svg_path_cache: std::collections::HashMap<u64, path::SvgPathCacheEntry>,
    svg_path_cache_order: std::collections::VecDeque<u64>,
    svg_raster_cache: std::collections::HashMap<path::SvgRasterKey, path::SvgRasterCacheEntry>,
    svg_raster_cache_order: std::collections::VecDeque<path::SvgRasterKey>,
    dot_point_buffer: Vec<skia_safe::Point>,
}

/// Maximum number of decoded chat images held at once. Decoded RGBA
/// is far larger than the encoded source, so the cap is modest.
const IMAGE_CACHE_CAP: usize = 48;

const ROBOTO_TTF: &[u8] = include_bytes!("../../assets/Roboto-Regular.ttf");

/// Union of every CJK codepoint that appears in the editor chrome
/// and the settings modal. Pre-warmed at `NativeBackend::new` so the
/// first cross-tab paint doesn't synchronously call
/// `FontMgr::match_family_style_character` for ~50 fresh glyphs.
const PREWARM_CJK_CODEPOINTS: &str = "\
设置\
内置服务商添加直接配置密钥无需工具\
尚未\
连接外部兼容的\
你可以在中设置额外的环境变量\
模型\
服务器已停止运行中端口启动\
终端集成将在重启后生效\
升级版本请重新安装确保兼容性\
断开链接\
代码\
设计图层填充描边效果导出创建组件\
弹性布局\
位置尺寸不透明度旋转\
形状矩形椭圆多边形直线钢笔\
画布缩放\
新建未命名页面\
帧组文本路径\
撤销重做\
你可以在\
普通粗体斜体下划线删除线\
左中右两端对齐分散\
颜色\
红绿蓝色调饱和度明亮\
搜索准备就绪\
图片生成";

impl NativeBackend {
    /// Spec §5.2.1 / plan v7 Task 2 Step 11: take an externally
    /// constructed `SkiaBackend` so callers can re-use one across
    /// `NativeBackend` / Jian host adapters that rely on the same
    /// image cache. The convenience `with_dpi` covers the common
    /// "fresh backend" path.
    pub fn new(skia: jian_skia::SkiaBackend, dpi: f32) -> Self {
        let mut this = Self {
            skia,
            dpi,
            font_mgr: skia_safe::FontMgr::new(),
            typeface: None,
            typeface_tried: false,
            cjk_typeface: None,
            cjk_typeface_tried: false,
            korean_typeface: None,
            korean_typeface_tried: false,
            char_typeface_cache: std::collections::HashMap::new(),
            family_typeface_cache: std::collections::HashMap::new(),
            image_cache: std::collections::HashMap::new(),
            image_cache_order: std::collections::VecDeque::new(),
            svg_path_cache: std::collections::HashMap::new(),
            svg_path_cache_order: std::collections::VecDeque::new(),
            svg_raster_cache: std::collections::HashMap::new(),
            svg_raster_cache_order: std::collections::VecDeque::new(),
            dot_point_buffer: Vec::new(),
        };
        // Pre-warm the per-codepoint typeface cache with every CJK
        // glyph that appears in the chrome (top bar, layer panel,
        // settings modal, etc.). Without this, the first paint that
        // touches a previously unseen codepoint pays the cost of a
        // `FontMgr::match_family_style_character` call — visible to
        // the user as a stutter the first time a tab is opened.
        for c in PREWARM_CJK_CODEPOINTS.chars() {
            let _ = this.typeface_for_char(c, 400);
        }
        this
    }

    /// Convenience constructor for tests and the basic-window demo.
    pub fn with_dpi(dpi: f32) -> Self {
        Self::new(jian_skia::SkiaBackend::new(), dpi)
    }

    /// `SharedSkiaContext` doesn't change the DPI per-frame, so callers
    /// (e.g. `WindowEvent::ScaleFactorChanged` handler) push it down
    /// here.
    pub fn set_dpi(&mut self, dpi: f32) {
        self.dpi = dpi;
    }

    /// Logical→physical scale factor (`window.scale_factor()`).
    pub fn dpi_scale(&self) -> f32 {
        self.dpi
    }

    // ── Frame markers ───────────────────────────────────────────────────

    /// Mirrors `RenderBackend::begin_frame`; jian-skia v0.0.1's
    /// `begin_frame` is a buffer-replay marker that needs a `SkiaSurface`
    /// — OP holds the GPU surface itself, so this is a no-op. The
    /// `canvas` argument is kept for API symmetry with the future
    /// `WithCanvas<'a>` impl.
    pub fn begin_frame(&mut self, _canvas: &skia_safe::Canvas) {}

    /// Mirrors `RenderBackend::end_frame`. No-op for the same reason as
    /// `begin_frame`; `SharedSkiaContext::present` flushes + swaps.
    pub fn end_frame(&mut self, _canvas: &skia_safe::Canvas) {}

    // ── Drawing primitives ──────────────────────────────────────────────

    /// Direct `DrawOp` dispatch — used by callers that already build
    /// `jian_core::render::DrawOp` (e.g. `jian_host_desktop::scene::collect_draws_with_state`).
    /// Spec §5.2.1 round 5 CONCERN-R5-2: this is the only public path to
    /// `self.skia.draw_on_canvas` so `skia` can stay private.
    #[tracing::instrument(skip_all)]
    pub fn draw_op(&mut self, canvas: &skia_safe::Canvas, op: &jian_core::render::DrawOp) {
        self.skia.draw_on_canvas(canvas, op);
    }

    /// Filled rectangle. Translates `(rect, color)` → `DrawOp::Rect` with
    /// a solid `Paint`.
    #[tracing::instrument(skip(self, canvas))]
    pub fn fill_rect(&mut self, canvas: &skia_safe::Canvas, rect: Rect, color: Color) {
        // `Paint::solid` hardcodes `opacity: 1.0`, so the colour's
        // alpha was dropped — a translucent fill (e.g. the 12 %
        // marquee-selection band) painted fully opaque. Carry the
        // alpha through `Paint.opacity` the way `stroke_rect` does.
        let mut paint = jian_core::render::Paint::solid((color).to_jian());
        paint.opacity = color.a.clamp(0.0, 1.0);
        let op = jian_core::render::DrawOp::Rect {
            rect: to_jian_rect(rect),
            paint,
        };
        self.draw_op(canvas, &op);
    }

    /// Stroked rectangle with a fixed width.
    #[tracing::instrument(skip(self, canvas))]
    pub fn stroke_rect(
        &mut self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        color: Color,
        width: f32,
    ) {
        let paint = jian_core::render::Paint {
            fill: None,
            stroke: Some(jian_core::render::StrokeOp {
                color: (color).to_jian(),
                width,
            }),
            opacity: color.a.clamp(0.0, 1.0),
        };
        let op = jian_core::render::DrawOp::Rect {
            rect: to_jian_rect(rect),
            paint,
        };
        self.draw_op(canvas, &op);
    }

    /// Push a clip region. Spec §5.2.1 / plan Step 14f.
    pub fn clip_rect(&self, canvas: &skia_safe::Canvas, rect: Rect) {
        canvas.clip_rect(to_sk_rect(rect), None, None);
    }

    /// Intersect the clip with a rounded rectangle — `clipContent`
    /// containers clip children to their rounded corners (TS parity:
    /// `canvas.clipRRect(..., ClipOp.Intersect, true)`).
    pub fn clip_round_rect(&self, canvas: &skia_safe::Canvas, rect: Rect, radius: f32) {
        let rrect = skia_safe::RRect::new_rect_xy(to_sk_rect(rect), radius, radius);
        canvas.clip_rrect(rrect, skia_safe::ClipOp::Intersect, true);
    }

    /// Stroke a single line segment. Step 4 visual lift addition —
    /// `jian_core::render::DrawOp` lacks a `Line` variant, so this
    /// bypasses jian and calls `Canvas::draw_line` directly. Same
    /// shape as `clip_rect`'s direct-canvas pattern. Skia stroke cap
    /// is set to `Round` so icon endpoints look like the lucide-react
    /// reference.
    pub fn stroke_line(
        &self,
        canvas: &skia_safe::Canvas,
        from: Point2D,
        to: Point2D,
        color: Color,
        width: f32,
    ) {
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        paint.set_stroke_cap(skia_safe::PaintCap::Round);
        canvas.draw_line((from.x, from.y), (to.x, to.y), &paint);
    }

    /// Filled rounded rectangle — used for shadcn-style chip / panel /
    /// button surfaces. Bypasses jian for the same reason as
    /// `stroke_line`.
    pub fn fill_round_rect(
        &self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        radius: f32,
        color: Color,
    ) {
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        // Skia paints aren't AA by default — chip / button edges
        // come out stair-stepped without this. Same call mirrored
        // into stroke_round_rect / stroke_line / stroke_svg_path.
        paint.set_anti_alias(true);
        canvas.draw_round_rect(to_sk_rect(rect), radius, radius, &paint);
    }

    /// Drop shadow — a gaussian-blurred filled rounded rectangle.
    /// `blur` is the CSS-style blur radius (doc-px × zoom, applied
    /// by the caller); skia's mask-filter takes a sigma, and the
    /// CSS blur-radius → sigma conversion is `radius / 2`. A zero
    /// blur degrades to a crisp filled round rect.
    pub fn fill_drop_shadow(
        &self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        radius: f32,
        blur: f32,
        color: Color,
    ) {
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        let sigma = blur * 0.5;
        if sigma > 0.0 {
            if let Some(mask) =
                skia_safe::MaskFilter::blur(skia_safe::BlurStyle::Normal, sigma, false)
            {
                paint.set_mask_filter(mask);
            }
        }
        canvas.draw_round_rect(to_sk_rect(rect), radius, radius, &paint);
    }

    /// Stroked rounded rectangle. Pairs with `fill_round_rect` for
    /// outlined chips / buttons.
    pub fn stroke_round_rect(
        &self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        radius: f32,
        color: Color,
        width: f32,
    ) {
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        canvas.draw_round_rect(to_sk_rect(rect), radius, radius, &paint);
    }

    /// Filled ellipse inscribed in `bounds`. Uses skia's native
    /// oval primitive so the curve is properly anti-aliased.
    pub fn fill_oval(&self, canvas: &skia_safe::Canvas, bounds: Rect, color: Color) {
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        canvas.draw_oval(to_sk_rect(bounds), &paint);
    }

    /// Stroked ellipse inscribed in `bounds`.
    pub fn stroke_oval(&self, canvas: &skia_safe::Canvas, bounds: Rect, color: Color, width: f32) {
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        canvas.draw_oval(to_sk_rect(bounds), &paint);
    }

    /// Fill a closed polygon outlined by `points`. Builds a fresh
    /// `Path` per call; cheap for triangles + handful-of-vertex
    /// shapes.
    pub fn fill_polygon(&self, canvas: &skia_safe::Canvas, points: &[Point2D], color: Color) {
        if points.len() < 3 {
            return;
        }
        // skia-safe 0.97 splits path construction onto `PathBuilder`;
        // `Path::new()` itself is immutable for traversal.
        let mut builder = skia_safe::PathBuilder::new();
        builder.move_to((points[0].x, points[0].y));
        for p in &points[1..] {
            builder.line_to((p.x, p.y));
        }
        builder.close();
        let path = builder.detach();
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        canvas.draw_path(&path, &paint);
    }

    /// Fill a batch of identical round dots in one draw call.
    /// `PointMode::Points` with a round stroke cap paints each point
    /// as a filled circle of diameter `2 * radius` — so the canvas
    /// grid (~1000+ dots on a full viewport) costs a single batched
    /// skia op per frame instead of one `draw_round_rect` per dot.
    pub fn fill_dots(
        &mut self,
        canvas: &skia_safe::Canvas,
        centers: &[Point2D],
        radius: f32,
        color: Color,
    ) {
        if centers.is_empty() {
            return;
        }
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        paint.set_stroke(true);
        paint.set_stroke_cap(skia_safe::PaintCap::Round);
        paint.set_stroke_width(radius * 2.0);
        let pts = self.prepare_dot_points(centers);
        canvas.draw_points(skia_safe::canvas::PointMode::Points, pts, &paint);
    }

    fn prepare_dot_points(&mut self, centers: &[Point2D]) -> &[skia_safe::Point] {
        self.dot_point_buffer.clear();
        self.dot_point_buffer.reserve(centers.len());
        self.dot_point_buffer
            .extend(centers.iter().map(|c| skia_safe::Point::new(c.x, c.y)));
        &self.dot_point_buffer
    }

    /// Save the current canvas state. Returns the save count so
    /// `restore_to` can pop back to it. (`Canvas::save` returns the
    /// pre-save count; we pass it through.)
    pub fn save(&self, canvas: &skia_safe::Canvas) -> usize {
        canvas.save()
    }

    /// Pop the most recent save.
    pub fn restore(&self, canvas: &skia_safe::Canvas) {
        canvas.restore();
    }

    /// Restore the canvas state stack down to a specific count returned
    /// by [`save`]. Mirrors `Canvas::restore_to_count`.
    pub fn restore_to(&self, canvas: &skia_safe::Canvas, count: usize) {
        canvas.restore_to_count(count);
    }

    /// Translate the current canvas matrix.
    pub fn translate(&self, canvas: &skia_safe::Canvas, offset: Point2D) {
        canvas.translate((offset.x, offset.y));
    }

    /// Scale the current canvas matrix around `pivot`.
    pub fn scale(&self, canvas: &skia_safe::Canvas, scale: Point2D, pivot: Point2D) {
        canvas.translate((pivot.x, pivot.y));
        canvas.scale((scale.x, scale.y));
        canvas.translate((-pivot.x, -pivot.y));
    }

    /// Rotate the current canvas matrix `radians` clockwise about
    /// `pivot`. Skia's `rotate_with_pivot` takes degrees, so the
    /// conversion happens here.
    pub fn rotate(&self, canvas: &skia_safe::Canvas, radians: f32, pivot: Point2D) {
        let degrees = radians.to_degrees();
        canvas.rotate(degrees, Some(skia_safe::Point::new(pivot.x, pivot.y)));
    }

    /// No-op; surface resize is owned by `SharedSkiaContext::resize`.
    pub fn resize(&mut self, _width: u32, _height: u32) {}

    /// Decode + cache the image for `id`. The first call decodes
    /// `encoded`; later calls reuse the cached result (or cached
    /// `None` on a decode failure) and ignore `encoded`. The cache is
    /// bounded — once it exceeds [`IMAGE_CACHE_CAP`] the oldest entry
    /// is evicted (it re-decodes on its next paint).
    pub(crate) fn cached_image(&mut self, id: u64, encoded: &[u8]) -> Option<skia_safe::Image> {
        if let Some(hit) = self.image_cache.get(&id) {
            return hit.clone();
        }
        let decoded = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(encoded));
        self.image_cache.insert(id, decoded.clone());
        self.image_cache_order.push_back(id);
        while self.image_cache.len() > IMAGE_CACHE_CAP {
            match self.image_cache_order.pop_front() {
                Some(oldest) => {
                    self.image_cache.remove(&oldest);
                }
                None => break,
            }
        }
        decoded
    }

    /// Number of cached image entries — test accessor.
    #[cfg(test)]
    pub(crate) fn image_cache_len(&self) -> usize {
        self.image_cache.len()
    }

    #[cfg(test)]
    pub(crate) fn family_typeface_cache_len(&self) -> usize {
        self.family_typeface_cache.len()
    }
}

/// Enumerate every installed font family via Skia's `FontMgr` —
/// feeds the property panel's font-family picker (the native
/// counterpart of the browser Local Font Access API the TS
/// `use-system-fonts.ts` hook queries). Names come back in manager
/// order; the caller sorts / dedupes.
pub fn enumerate_system_font_families() -> Vec<String> {
    let mgr = skia_safe::FontMgr::new();
    let count = mgr.count_families();
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let name = mgr.family_name(i);
        if !name.trim().is_empty() {
            out.push(name);
        }
    }
    out
}

#[cfg(test)]
#[path = "skia/tests.rs"]
mod tests;
