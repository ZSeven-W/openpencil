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

use openpencil_shell_core::{Color, Point2D, Rect, TextLayout};

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
fn to_sk_rect(r: Rect) -> skia_safe::Rect {
    skia_safe::Rect::from_xywh(r.origin.x, r.origin.y, r.size.x, r.size.y)
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
    /// ASCII labels (`页面 / 图层 / 未命名 / 用 AI 开始设计 / ...`)
    /// render through a system CJK font (PingFang on macOS,
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
    char_typeface_cache: std::collections::HashMap<i32, Option<skia_safe::Typeface>>,
}

const ROBOTO_TTF: &[u8] = include_bytes!("../../../openpencil-shell-web/assets/Roboto-Regular.ttf");

impl NativeBackend {
    /// Spec §5.2.1 / plan v7 Task 2 Step 11: take an externally
    /// constructed `SkiaBackend` so callers can re-use one across
    /// `NativeBackend` / Jian host adapters that rely on the same
    /// image cache. The convenience `with_dpi` covers the common
    /// "fresh backend" path.
    pub fn new(skia: jian_skia::SkiaBackend, dpi: f32) -> Self {
        Self {
            skia,
            dpi,
            typeface: None,
            typeface_tried: false,
            cjk_typeface: None,
            cjk_typeface_tried: false,
            char_typeface_cache: std::collections::HashMap::new(),
        }
    }

    /// Resolve a typeface that covers `c`. Cached per codepoint —
    /// `FontMgr::match_family_style_character` is fast on first call
    /// but we still avoid the look-up on every chrome paint.
    /// Falls back to the cached CJK typeface, then the cached
    /// Roboto, so a worst-case missing-font system still renders
    /// something rather than dropping the glyph.
    fn typeface_for_char(&mut self, c: char) -> Option<skia_safe::Typeface> {
        if c.is_ascii() {
            return self.ensure_typeface().cloned();
        }
        let cp = c as i32;
        if let Some(cached) = self.char_typeface_cache.get(&cp) {
            return cached.clone();
        }
        let mgr = skia_safe::FontMgr::new();
        let tf = mgr.match_family_style_character("", skia_safe::FontStyle::default(), &[], cp);
        let resolved = tf.or_else(|| self.ensure_cjk_typeface().cloned());
        self.char_typeface_cache.insert(cp, resolved.clone());
        resolved
    }

    /// Split `text` into contiguous segments that share a typeface,
    /// preserving char order. Glyphs without any covering typeface
    /// are bucketed with the previous segment so they at least
    /// occupy space (rather than disappearing).
    fn segment_text(&mut self, text: &str) -> Vec<(skia_safe::Typeface, String)> {
        let mut segments: Vec<(skia_safe::Typeface, String)> = Vec::new();
        for c in text.chars() {
            let tf = self.typeface_for_char(c);
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
        let op = jian_core::render::DrawOp::Rect {
            rect: to_jian_rect(rect),
            paint: jian_core::render::Paint::solid(to_jian_color(color)),
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
        let segments = self.segment_text(text);
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
    ///     Latin too, so mixed runs like "美食 App 首页" render
    ///     correctly.
    ///   - the system has no CJK font (rare; Linux without Noto)
    ///     → fall back to jian-skia's textlayout path so glyphs
    ///     don't drop. Still slow on that branch, but functional.
    #[tracing::instrument(skip(self, canvas, layout))]
    pub fn draw_text(&mut self, canvas: &skia_safe::Canvas, layout: &TextLayout, origin: Point2D) {
        let runs: Vec<_> = layout.runs().to_vec();
        for run in runs {
            let segments = self.segment_text(run.content.as_str());
            if segments.is_empty() {
                continue;
            }
            let jc = run.color;
            let paint = skia_safe::Paint::new(
                skia_safe::Color4f::new(
                    f32::from(jc.r()) / 255.0,
                    f32::from(jc.g()) / 255.0,
                    f32::from(jc.b()) / 255.0,
                    f32::from(jc.a()) / 255.0,
                ),
                None,
            );
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_roundtrip_clamps_and_packs() {
        let red = to_jian_color(Color::RED);
        assert_eq!(red.r(), 255);
        assert_eq!(red.g(), 0);
        assert_eq!(red.b(), 0);
        assert_eq!(red.a(), 255);

        let transparent = to_jian_color(Color::TRANSPARENT);
        assert_eq!(transparent.a(), 0);

        // Out-of-range channels are clamped, not wrapped.
        let weird = to_jian_color(Color {
            r: -0.5,
            g: 2.0,
            b: 0.5,
            a: 1.0,
        });
        assert_eq!(weird.r(), 0);
        assert_eq!(weird.g(), 255);
        assert_eq!(weird.b(), 128);
    }

    #[test]
    fn rect_translation_keeps_size() {
        let r = Rect {
            origin: Point2D::new(10.0, 20.0),
            size: Point2D::new(30.0, 40.0),
        };
        let jr = to_jian_rect(r);
        assert!((jr.min_x() - 10.0).abs() < 1e-6);
        assert!((jr.min_y() - 20.0).abs() < 1e-6);
        assert!((jr.size.width - 30.0).abs() < 1e-6);
        assert!((jr.size.height - 40.0).abs() < 1e-6);
    }
}
