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

use op_editor_ui::{Color, Point2D, Rect, TextLayout};

/// OP-flavoured (`f32` 0..=1 RGBA) → Jian (`u8` packed RGBA) color.
///
/// Spec §5.2 / round 5 CONCERN-R5-3: jian-core `Color` is `Color(pub u32)`
/// without named constants; OP `Color::{RED, GREEN, BLUE, BLACK, WHITE,
/// TRANSPARENT}` lives in shell-core. This helper closes the gap.
pub fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn channel(v: f32) -> u8 {
        // `clamp` first — `as u8` saturates negative to 0 but unconstrained
        // > 1.0 would wrap (e.g. 256.0 → 0). Use `.clamp` then `*255`.
        let scaled = (v.clamp(0.0, 1.0) * 255.0).round();
        scaled as u8
    }
    jian_core::scene::Color::rgba(channel(c.r), channel(c.g), channel(c.b), channel(c.a))
}

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

/// Build a skia `FontStyle` whose weight matches the canonical
/// schema's CSS-style numeric weight (100-900). Width + slant stay
/// at the system defaults (Normal / Upright); italic / stretched
/// variants are a follow-up if `.op` files start authoring them.
fn font_style_for_weight(weight: u16) -> skia_safe::FontStyle {
    let w = weight as i32;
    skia_safe::FontStyle::new(
        skia_safe::font_style::Weight::from(w),
        skia_safe::font_style::Width::NORMAL,
        skia_safe::font_style::Slant::Upright,
    )
}

/// OP `Color` → `skia_safe::Color4f` — used by the direct-canvas
/// helpers (stroke_line / fill_round_rect / stroke_round_rect)
/// that skip the jian DrawOp pipeline.
fn jian_color_to_color4f(c: Color) -> skia_safe::Color4f {
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
mod gradient;

/// Frame-scoped Jian-DrawOp adapter (spec v19 §5.2.1).
///
/// The struct stays alive across frames so its underlying
/// `jian_skia::SkiaBackend` keeps its image cache; the canvas borrow is
/// passed in per-method so `&NativeBackend` and `&Canvas` can coexist
/// inside the same `with_frame` closure without aliasing.
pub struct NativeBackend {
    skia: jian_skia::SkiaBackend,
    dpi: f32,
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
    /// Per-codepoint typeface cache. Populated on first sight of a
    /// non-ASCII character so multi-script chrome (Korean 한국어,
    /// Devanagari हिन्दी, Thai ไทย, Vietnamese precomposed
    /// `Tiếng Việt` …) renders against the right system font
    /// instead of falling through to the single `cjk_typeface`
    /// (which only covers Han / Hiragana / Katakana on most OSes).
    /// Cache key is `(codepoint, weight)` so weight 400 chrome glyphs
    /// and weight 700 canvas headlines don't share an entry. Bold
    /// text from `.op` files (`fontWeight: 700`) re-resolves the
    /// typeface against `FontStyle::new(Weight, Width::NORMAL, …)`
    /// so the system FontMgr returns the bold-variant TTF when one
    /// is installed (TS app parity).
    char_typeface_cache: std::collections::HashMap<(i32, u16), Option<skia_safe::Typeface>>,
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
}

/// Maximum number of decoded chat images held at once. Decoded RGBA
/// is far larger than the encoded source, so the cap is modest.
const IMAGE_CACHE_CAP: usize = 48;

const ROBOTO_TTF: &[u8] = include_bytes!("../../../op-host-web/assets/Roboto-Regular.ttf");

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
            typeface: None,
            typeface_tried: false,
            cjk_typeface: None,
            cjk_typeface_tried: false,
            char_typeface_cache: std::collections::HashMap::new(),
            image_cache: std::collections::HashMap::new(),
            image_cache_order: std::collections::VecDeque::new(),
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

    /// Resolve a typeface that covers `c`. Cached per codepoint —
    /// `FontMgr::match_family_style_character` is fast on first call
    /// but we still avoid the look-up on every chrome paint.
    /// Falls back to the cached CJK typeface, then the cached
    /// Roboto, so a worst-case missing-font system still renders
    /// something rather than dropping the glyph.
    fn typeface_for_char(&mut self, c: char, weight: u16) -> Option<skia_safe::Typeface> {
        // ASCII at weight 400 stays on the bundled Roboto-Regular for
        // hot chrome paint; non-default weight or non-ASCII falls
        // through to the system FontMgr, which honours weight.
        if c.is_ascii() && weight == 400 {
            return self.ensure_typeface().cloned();
        }
        let cp = c as i32;
        let key = (cp, weight);
        if let Some(cached) = self.char_typeface_cache.get(&key) {
            return cached.clone();
        }
        let style = font_style_for_weight(weight);
        let mgr = skia_safe::FontMgr::new();
        let tf = mgr.match_family_style_character("", style, &[], cp);
        let resolved = tf.or_else(|| {
            // CJK fallback path doesn't yet vary by weight — TS app
            // synthesises bold via paint stroke when the family is
            // weight-locked. Mirror that downstream of `draw_text`
            // with `Paint::set_stroke_width` when needed.
            self.ensure_cjk_typeface().cloned()
        });
        self.char_typeface_cache.insert(key, resolved.clone());
        resolved
    }

    /// Split `text` into contiguous segments that share a typeface,
    /// preserving char order. Glyphs without any covering typeface
    /// are bucketed with the previous segment so they at least
    /// occupy space (rather than disappearing).
    fn segment_text(&mut self, text: &str, weight: u16) -> Vec<(skia_safe::Typeface, String)> {
        let mut segments: Vec<(skia_safe::Typeface, String)> = Vec::new();
        for c in text.chars() {
            let tf = self.typeface_for_char(c, weight);
            let Some(tf) = tf else {
                if let Some(last) = segments.last_mut() {
                    last.1.push(c);
                }
                continue;
            };
            match segments.last_mut() {
                Some(last) if last.0.unique_id() == tf.unique_id() => last.1.push(c),
                _ => segments.push((tf, c.to_string())),
            }
        }
        segments
    }

    /// Lazy-init the Step 4 cached Roboto typeface (ASCII path).
    fn ensure_typeface(&mut self) -> Option<&skia_safe::Typeface> {
        if !self.typeface_tried {
            self.typeface = skia_safe::FontMgr::new().new_from_data(ROBOTO_TTF, None);
            self.typeface_tried = true;
        }
        self.typeface.as_ref()
    }

    /// Lazy-resolve a system typeface that has CJK glyph coverage.
    /// Picks whichever font the system FontMgr would use for the
    /// canonical Han ideograph U+4E00 — on macOS this is PingFang SC,
    /// on Linux it's Noto Sans CJK, on Windows it's Microsoft YaHei
    /// or similar. Cached for the lifetime of the backend so we
    /// don't pay the FontMgr lookup more than once.
    fn ensure_cjk_typeface(&mut self) -> Option<&skia_safe::Typeface> {
        if !self.cjk_typeface_tried {
            let mgr = skia_safe::FontMgr::new();
            self.cjk_typeface = mgr.match_family_style_character(
                "",
                skia_safe::FontStyle::default(),
                &[],
                '一' as i32,
            );
            self.cjk_typeface_tried = true;
        }
        self.cjk_typeface.as_ref()
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
        let mut paint = jian_core::render::Paint::solid(to_jian_color(color));
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
                color: to_jian_color(color),
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

    /// Measure the rendered horizontal advance of `text` at
    /// `font_size`. Uses the same per-script typeface dispatch as
    /// `draw_text` so the measurement matches what's painted.
    /// Falls back to a conservative heuristic when typefaces
    /// aren't available.
    pub fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.measure_text_weighted(text, font_size, 400)
    }

    /// Weight-aware text measurement. Resolves the per-codepoint
    /// typeface against `FontStyle::new(Weight, ...)` so wrap-pass
    /// line breaks decided at, say, weight 700 use the same glyph
    /// advances `draw_text` will paint with at weight 700. Without
    /// this the wrap pass measured at 400 and paint at 700 — the
    /// rendered string could then overflow the wrap budget.
    pub fn measure_text_weighted(&mut self, text: &str, font_size: f32, weight: u16) -> f32 {
        let segments = self.segment_text(text, weight);
        if segments.is_empty() {
            return 0.0;
        }
        let mut advance = 0.0_f32;
        for (typeface, segment) in segments {
            let font = skia_safe::Font::new(&typeface, font_size);
            let (a, _) = font.measure_str(&segment, None);
            advance += a;
        }
        advance
    }

    /// Render every shaped run in the layout via cached typefaces +
    /// `Canvas::draw_str` (Step 4 perf fix — see comment on the
    /// `typeface` / `cjk_typeface` fields).
    ///
    /// Two fast paths + one fallback:
    ///   - run is ASCII-only → cached Roboto + draw_str
    ///   - run contains non-ASCII → cached system CJK typeface +
    ///     draw_str (PingFang / Noto CJK / etc.). PingFang covers
    ///     Latin too, so mixed CJK + Latin runs render correctly.
    ///   - the system has no CJK font (rare; Linux without Noto)
    ///     → fall back to jian-skia's textlayout path so glyphs
    ///     don't drop. Still slow on that branch, but functional.
    #[tracing::instrument(skip(self, canvas, layout))]
    pub fn draw_text(&mut self, canvas: &skia_safe::Canvas, layout: &TextLayout, origin: Point2D) {
        let runs: Vec<_> = layout.runs().to_vec();
        for run in runs {
            let segments = self.segment_text(run.content.as_str(), run.font_weight);
            if segments.is_empty() {
                continue;
            }
            let jc = run.color;
            let mut paint = skia_safe::Paint::new(
                skia_safe::Color4f::new(
                    f32::from(jc.r()) / 255.0,
                    f32::from(jc.g()) / 255.0,
                    f32::from(jc.b()) / 255.0,
                    f32::from(jc.a()) / 255.0,
                ),
                None,
            );
            paint.set_anti_alias(true);
            // Synthetic bold for typefaces the system serves at one
            // weight only (notably the bundled Roboto-Regular for
            // ASCII at weight ≥600). Stroke width scales with size
            // so 28pt headline gets the same visual weight relative
            // to its glyph as 13pt body text.
            let synth_bold = run.font_weight >= 600;
            if synth_bold {
                paint.set_style(skia_safe::PaintStyle::StrokeAndFill);
                paint.set_stroke_width(run.font_size * 0.06);
            }
            let mut x = origin.x + run.origin.x;
            let y = origin.y + run.origin.y;
            for (typeface, segment) in segments {
                let font = skia_safe::Font::new(&typeface, run.font_size);
                canvas.draw_str(&segment, (x, y), &font, &paint);
                let (advance, _) = font.measure_str(&segment, None);
                x += advance;
            }
        }
    }

    /// Push a clip region. Spec §5.2.1 / plan Step 14f.
    pub fn clip_rect(&self, canvas: &skia_safe::Canvas, rect: Rect) {
        canvas.clip_rect(to_sk_rect(rect), None, None);
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

    /// Step 5 SVG icons: parse an SVG path `d` string, scale from
    /// a 24×24 viewBox to `size × size` at `top_left`, and stroke
    /// it with round caps + joins (matches lucide's visual style).
    /// Falls back to a no-op when the path string fails to parse —
    /// silently dropping a single icon is better than panicking
    /// the paint loop.
    pub fn stroke_svg_path(
        &self,
        canvas: &skia_safe::Canvas,
        d: &str,
        top_left: Point2D,
        size: f32,
        color: Color,
        width: f32,
    ) {
        let Some(path) = skia_safe::utils::parse_path::from_svg(d) else {
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

    /// Fill an SVG path scaled from `viewbox × viewbox` to
    /// `size × size`. Brand logos ship as filled paths in their
    /// own viewBox; the same parser as `stroke_svg_path` is
    /// reused but paint is configured for `Fill` not `Stroke`.
    pub fn fill_svg_path(
        &self,
        canvas: &skia_safe::Canvas,
        d: &str,
        top_left: Point2D,
        size: f32,
        viewbox: f32,
        color: Color,
    ) {
        let Some(path) = skia_safe::utils::parse_path::from_svg(d) else {
            return;
        };
        let s = size / viewbox;
        let mut matrix = skia_safe::Matrix::new_identity();
        matrix.set_scale_translate((s, s), (top_left.x, top_left.y));
        let path = path.with_transform(&matrix);
        let mut paint = skia_safe::Paint::new(jian_color_to_color4f(color), None);
        paint.set_anti_alias(true);
        canvas.draw_path(&path, &paint);
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
        &self,
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
        let pts: Vec<skia_safe::Point> = centers
            .iter()
            .map(|c| skia_safe::Point::new(c.x, c.y))
            .collect();
        canvas.draw_points(skia_safe::canvas::PointMode::Points, &pts, &paint);
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

    /// Draw the image identified by `id`, aspect-fit + centered
    /// inside `rect`. `encoded` is the raw image bytes, consulted
    /// only on the first (cache-miss) draw. A decode failure paints
    /// nothing — callers paint a placeholder frame underneath.
    pub fn draw_image(&mut self, canvas: &skia_safe::Canvas, rect: Rect, id: u64, encoded: &[u8]) {
        let Some(image) = self.cached_image(id, encoded) else {
            return;
        };
        let fit = contain_rect(rect, image.width() as f32, image.height() as f32);
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        canvas.draw_image_rect(&image, None, to_sk_rect(fit), &paint);
    }
}

#[cfg(test)]
#[path = "skia/tests.rs"]
mod tests;
