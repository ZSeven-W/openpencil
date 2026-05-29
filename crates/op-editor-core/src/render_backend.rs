//! OP `RenderBackend` widget-facing facade (spec v19 §5.2).
//!
//! This is OP's design contract (method-style API: `fill_rect / stroke_rect /
//! draw_text / clip_rect / save / restore / translate / resize / dpi_scale`),
//! which does not line up directly with Jian's
//! `jian_core::render::RenderBackend` (command-buffer style: `new_surface /
//! begin_frame / draw(&DrawOp)` etc.).
//!
//! Implementation paths (per §5.2.1):
//! - `NativeBackend` (shell-native, frame-scoped design from Step 1a onward)
//!   does **not** impl this trait directly; instead it exposes same-named
//!   methods that take an explicit `canvas: &skia_safe::Canvas` argument.
//!   The Step 1a basic_window demo invokes NativeBackend methods inside a
//!   `SharedSkiaContext::with_frame` closure. Once Step 1c+ wires in a real
//!   widget tree we can revisit a `WithCanvas<'a>` newtype to inject the
//!   canvas into a trait impl.
//! - `WebCanvasKitBackend` (shell-web, Step 1b): backed by CanvasKit JS bindings.
//! - `MobileBackend` (Step 1f): backed by Metal / Vulkan / OpenGL ES.
//!
//! This module does **not** pull in skia-safe / Canvas / GL types — shell-core
//! must stay wasm32-clean (per spec §1.2 boundary).

use jian_core::render::{TextAlign, TextRun};

/// 2D coordinate point (spec §5.2 fixes this to `glam::Vec2`).
pub type Point2D = glam::Vec2;

/// Rectangle (origin + size as two Vec2s).
///
/// Derives `PartialEq` so `widgets::LayoutBox` can compare layout
/// results in tests. `Eq` is intentionally NOT derived: `Vec2` carries
/// floats, and exact float equality is only meaningful when callers
/// have been careful about how the values were produced.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub origin: Point2D,
    pub size: Point2D,
}

impl Rect {
    /// Zero-sized rect at the origin. Used by Document::Node as the
    /// "no bounds set" / container-only sentinel, and by tests
    /// constructing throwaway rects.
    pub const ZERO: Rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(0.0, 0.0),
    };

    /// Convenience builder: `Rect::xywh(x, y, w, h)`.
    pub const fn xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(width, height),
        }
    }
}

/// RGBA color (widget facade layer; all components 0.0..=1.0).
///
/// Named constants live at the OP layer (spec v19 round 5 CONCERN-R5-3 fix);
/// `jian_core::scene::Color` is `Color(pub u32)` packed RGBA without RED/BLACK/etc
/// named constants — callers must construct it explicitly via `JianColor::rgb(...)`.
// PartialEq + no Eq: f32 components mean exact equality is rare,
// but Step 3's `Document::Node::fill: Option<Color>` and
// `Stroke { color: Color }` need PartialEq for derived comparisons
// in test assertions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
}

/// OP's TextLayout — defined at the OP layer after v19 removed parley; a thin
/// wrapper over `jian_core::render::TextRun`.
///
/// It does not hold a layout context or glyph cache (shell-core is wasm32-clean
/// and cannot pull in skia/parley/icu); the real layout happens inside
/// NativeBackend::draw_text via `jian_skia::SkiaBackend`'s textlayout feature
/// (skia textlayout, ICU+harfbuzz) for shaping + line breaking.
///
/// In Step 1a, TextLayout is a placeholder for "a collection of already-shaped
/// TextRuns"; caret/selection/bidi/wrap are deferred to Step 1c+. The
/// signatures are pinned to avoid breaking the API in later steps.
#[derive(Debug, Clone)]
pub struct TextLayout {
    runs: Vec<TextRun>,
}

/// Raster image placement mode. Matches the canonical image-fill /
/// image-node modes used by the TS renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageDrawMode {
    #[default]
    Fill,
    Fit,
    Crop,
    Tile,
    Stretch,
}

/// Per-image adjustment values from the image-fill popover. Each
/// value is in the TS slider range `[-100, 100]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ImageAdjustments {
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub tint: f32,
    pub highlights: f32,
    pub shadows: f32,
}

impl ImageAdjustments {
    pub fn is_neutral(self) -> bool {
        self.exposure == 0.0
            && self.contrast == 0.0
            && self.saturation == 0.0
            && self.temperature == 0.0
            && self.tint == 0.0
            && self.highlights == 0.0
            && self.shadows == 0.0
    }
}

impl TextLayout {
    /// Single-run constructor (the only active path in Step 1a; used by the
    /// Phase B/C demo + raster_text_smoke).
    ///
    /// Fields align with `jian_core::render::TextRun`
    /// (`vendor/jian/crates/jian-core/src/render/paint.rs:77-94`):
    /// **TextRun has no `Default` impl**, so every field must be set explicitly.
    /// - `content` / `font_family` / `font_size` / `color` / `origin`: passed in by caller
    /// - `font_weight: 400` (CSS Normal)
    /// - `max_width: 0.0` ("unknown; render at origin with no alignment adjustment")
    /// - `align: TextAlign::Start`
    /// - `line_height: 0.0` ("default")
    pub fn single_run(
        content: &str,
        font_family: &str,
        font_size: f32,
        color: jian_core::scene::Color,
        origin: Point2D,
    ) -> Self {
        let run = TextRun {
            content: content.to_string(),
            font_family: font_family.to_string(),
            font_size,
            font_weight: 400,
            color,
            origin: jian_core::geometry::Point::new(origin.x, origin.y),
            max_width: 0.0,
            align: TextAlign::Start,
            line_height: 0.0,
        };
        Self { runs: vec![run] }
    }

    /// View of the already-shaped TextRun collection.
    pub fn runs(&self) -> &[TextRun] {
        &self.runs
    }

    /// Overwrite every run's CSS-style numeric weight. The convenience
    /// constructor hardcodes 400; canvas-side text from `.op` files
    /// honours `Node.font_weight` via this builder.
    pub fn with_font_weight(mut self, weight: u16) -> Self {
        for r in &mut self.runs {
            r.font_weight = weight;
        }
        self
    }

    /// Translates every run's origin (NativeBackend::draw_text adds this on top
    /// of the widget origin). Returns a new layout; the original is unchanged.
    pub fn translated(&self, offset: Point2D) -> Self {
        let runs = self
            .runs
            .iter()
            .map(|r| {
                let mut r2 = r.clone();
                r2.origin =
                    jian_core::geometry::Point::new(r.origin.x + offset.x, r.origin.y + offset.y);
                r2
            })
            .collect();
        Self { runs }
    }
}

/// Backend abstraction (widget-facing facade, spec §5.2).
///
/// Note: the trait has no `Send` bound — skia-safe types are `!Send`
/// (rust-skia is thread-bound), and the Backend is used solely on the render
/// thread, so cross-thread access is unnecessary.
///
/// In 1a, `NativeBackend` (shell-native) does **not** impl this trait directly
/// (v19 round 3 BLOCK-R3-3 fix — the frame-scoped design avoids cross-frame
/// borrows); instead it exposes the same-named methods with an explicit
/// `canvas: &skia_safe::Canvas` argument. The trait signature contains no
/// Skia / GPU-backend-specific types.
/// Pre-multiply a colour's alpha by a 0..=1 opacity multiplier —
/// shared by the gradient default impls when they degrade to a solid
/// first-stop fill.
fn fold_alpha(c: Color, opacity: f32) -> Color {
    Color {
        a: c.a * opacity.clamp(0.0, 1.0),
        ..c
    }
}

pub trait RenderBackend {
    /// Begin a frame; the backend tracks current frame state internally
    /// (canvas type is not exposed).
    fn begin_frame(&mut self);
    fn end_frame(&mut self);

    // Drawing primitives — widgets call these and never touch the canvas directly.
    fn fill_rect(&mut self, rect: Rect, color: Color);
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32);
    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D);
    fn clip_rect(&mut self, rect: Rect);

    // Step 4 visual lift — line + rounded rect primitives so widgets can
    // draw lucide-style icons and shadcn-style chip / panel / button
    // backgrounds without dropping back to per-backend skia calls.
    /// Stroke a single line segment from `from` to `to`. Used for
    /// icon line art (cursor arrow, T glyph, hash strokes, etc.).
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32);
    /// Filled rectangle with corner radius. `radius` is uniform on
    /// all four corners.
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color);
    /// Stroked rounded rectangle. Mirrors `stroke_rect` + `radius`.
    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32);
    /// Step 5 SVG icons: stroke an SVG path `d` string scaled from
    /// a 24×24 viewBox into a `size × size` square anchored at
    /// `top_left`. Backends parse the d-string via skia's path
    /// parser and render with round caps + joins to match
    /// lucide's visual style. Used by the icon module so widgets
    /// just declare which lucide path to draw.
    fn stroke_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, color: Color, width: f32);

    /// Fill an SVG path scaled from `viewbox` × `viewbox` to
    /// `size × size` anchored at `top_left`. Used for brand
    /// logos whose source paths are designed as filled shapes
    /// (Claude, OpenAI, etc.) rather than lucide's stroked
    /// 24×24 grid. Default-impl is a no-op so backends without
    /// a path-fill primitive degrade gracefully.
    fn fill_svg_path(
        &mut self,
        _d: &str,
        _top_left: Point2D,
        _size: f32,
        _viewbox: f32,
        _color: Color,
    ) {
    }

    /// Fill an arbitrary SVG path fitted into `rect` by the path's
    /// own tight bounds. Document Path nodes use this instead of the
    /// icon-oriented square viewBox helper because imported Figma
    /// vectors often carry negative local coordinates.
    fn fill_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color) {
        self.fill_svg_path(d, rect.origin, 1.0, 1.0, color);
    }

    /// Stroke an arbitrary SVG path fitted into `rect` by the path's
    /// own tight bounds. Default degrades to the icon-oriented path
    /// helper for simple backends.
    fn stroke_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color, width: f32) {
        self.stroke_svg_path(d, rect.origin, rect.size.x.max(rect.size.y), color, width);
    }

    /// Fill an arbitrary SVG path (fitted into `rect` by its own tight
    /// bounds) with a linear gradient. `stops` / `angle_deg` /
    /// `opacity` follow the same convention as
    /// [`fill_round_rect_linear_gradient`]. Default degrades to a
    /// solid first-stop fill so backends without a path-gradient
    /// shader still paint something resembling the gradient.
    fn fill_svg_path_in_rect_linear_gradient(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        _angle_deg: f32,
        opacity: f32,
    ) {
        if let Some((_, c)) = stops.first() {
            self.fill_svg_path_in_rect(d, rect, fold_alpha(*c, opacity));
        }
    }

    /// Paint an inset (inner) shadow for an SVG path fitted into
    /// `rect`: a blurred shadow that bleeds inward from the path
    /// silhouette, offset by `(offset_x, offset_y)`. Default is a
    /// no-op — backends without the primitive simply omit the inset
    /// shadow rather than drawing it as an outer one.
    #[allow(clippy::too_many_arguments)]
    fn fill_inner_shadow_svg_path(
        &mut self,
        _d: &str,
        _rect: Rect,
        _offset_x: f32,
        _offset_y: f32,
        _blur: f32,
        _color: Color,
    ) {
    }

    /// Fill an arbitrary SVG path (fitted into `rect`) with a radial
    /// gradient. Centre / radius fractions follow
    /// [`fill_round_rect_radial_gradient`]. Default degrades to a
    /// solid first-stop fill.
    #[allow(clippy::too_many_arguments)]
    fn fill_svg_path_in_rect_radial_gradient(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        _cx_frac: f32,
        _cy_frac: f32,
        _radius_frac: f32,
        opacity: f32,
    ) {
        if let Some((_, c)) = stops.first() {
            self.fill_svg_path_in_rect(d, rect, fold_alpha(*c, opacity));
        }
    }

    /// Paint a drop shadow: a gaussian-blurred filled rounded
    /// rectangle. `rect` is the already-offset shadow bounds,
    /// `radius` the corner radius (0 for sharp corners, min-half
    /// for an ellipse-shaped shadow), `blur` the gaussian sigma in
    /// device px (caller applies zoom). Painted BEHIND the node's
    /// own fill. Default impl approximates with an un-blurred
    /// translucent fill so non-blur backends still hint at depth;
    /// native + web override with skia's `MaskFilter::blur`.
    fn fill_drop_shadow(&mut self, rect: Rect, radius: f32, blur: f32, color: Color) {
        let _ = blur;
        self.fill_round_rect(rect, radius, color);
    }

    /// Filled ellipse inscribed in `bounds`. Used by Ellipse-kind
    /// document nodes. Default impl falls back to a rounded
    /// rectangle (visually wrong but compiles); native + web
    /// override with `Canvas::draw_oval`.
    fn fill_oval(&mut self, bounds: Rect, color: Color) {
        let radius = bounds.size.x.min(bounds.size.y) / 2.0;
        self.fill_round_rect(bounds, radius, color);
    }

    /// Stroked ellipse inscribed in `bounds`. Pairs with
    /// `fill_oval`. Default-impl behaves like `stroke_round_rect`
    /// at max corner radius.
    fn stroke_oval(&mut self, bounds: Rect, color: Color, width: f32) {
        let radius = bounds.size.x.min(bounds.size.y) / 2.0;
        self.stroke_round_rect(bounds, radius, color, width);
    }

    /// Fill a batch of identical round dots — `centers` are the dot
    /// centres, each painted as a filled circle of `radius`. The
    /// canvas grid calls this once per frame instead of emitting one
    /// draw op per dot (~1000+ on a full viewport), which is the
    /// dominant cost of an empty-canvas pan / drag. The default impl
    /// loops `fill_oval` so any backend stays correct; native
    /// overrides with a single batched `Canvas::draw_points`.
    fn fill_dots(&mut self, centers: &[Point2D], radius: f32, color: Color) {
        for c in centers {
            self.fill_oval(
                Rect {
                    origin: Point2D::new(c.x - radius, c.y - radius),
                    size: Point2D::new(radius * 2.0, radius * 2.0),
                },
                color,
            );
        }
    }

    /// Fill a closed polygon outlined by `points` (≥ 3 vertices).
    /// Default impl emits a `stroke_svg_path` fill — works for
    /// triangles + small polygons; backends with a richer canvas
    /// override.
    fn fill_polygon(&mut self, _points: &[Point2D], _color: Color) {
        // Default no-op so non-overriding backends just skip the
        // fill. Native + web override.
    }

    /// Stroke a closed polygon outlined by `points`. Default impl
    /// walks the points and emits N `stroke_line` calls.
    fn stroke_polygon(&mut self, points: &[Point2D], color: Color, width: f32) {
        if points.len() < 2 {
            return;
        }
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            self.stroke_line(a, b, color, width);
        }
    }

    /// Draw a raster image, aspect-fit + centered inside `rect`.
    /// `image_id` is a process-stable handle the backend keys its
    /// decode cache on; `encoded` is the raw image bytes (PNG /
    /// JPEG / …), consulted only on the first (cache-miss) draw of
    /// that id. Default impl is a no-op — backends without an image
    /// pipeline (or codec support) degrade gracefully and the caller
    /// is expected to have painted a placeholder frame underneath.
    fn draw_image(&mut self, _rect: Rect, _image_id: u64, _encoded: &[u8]) {}

    /// Draw a raster image using an explicit fill/fit/crop/tile mode.
    /// Backends without mode-aware support fall back to `draw_image`.
    fn draw_image_with_mode(
        &mut self,
        rect: Rect,
        image_id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
    ) {
        let _ = mode;
        self.draw_image(rect, image_id, encoded);
    }

    /// Draw a raster image with both placement and image-adjustment
    /// controls. Backends without color-filter support fall back to
    /// mode-aware drawing.
    fn draw_image_with_options(
        &mut self,
        rect: Rect,
        image_id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
        adjustments: ImageAdjustments,
        opacity: f32,
    ) {
        let _ = (adjustments, opacity);
        self.draw_image_with_mode(rect, image_id, encoded, mode);
    }

    /// Fill a (round-)rectangle with a linear gradient. `stops`
    /// carries `(offset, color)` pairs in 0.0..=1.0 with at least one
    /// entry; `angle_deg` is the gradient direction in the canonical
    /// `.op` convention (0° = bottom→top, 90° = left→right, matches
    /// CSS `to-top`); `opacity` is the per-fill multiplier folded
    /// into every stop's alpha. A `radius` of 0 paints a plain rect.
    ///
    /// Default impl degrades to a solid `fill_round_rect` using the
    /// first stop's colour pre-multiplied by `opacity` so backends
    /// without a gradient shader still paint something resembling
    /// the gradient.
    fn fill_round_rect_linear_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        stops: &[(f32, Color)],
        _angle_deg: f32,
        opacity: f32,
    ) {
        if let Some((_, c)) = stops.first() {
            let color = Color {
                r: c.r,
                g: c.g,
                b: c.b,
                a: c.a * opacity.clamp(0.0, 1.0),
            };
            self.fill_round_rect(rect, radius, color);
        }
    }

    /// Fill a (round-)rectangle with a radial gradient. `cx_frac`
    /// / `cy_frac` are 0.0..=1.0 fractions of `rect`'s width / height
    /// for the centre; `radius_frac` is a 0.0..=1.0 fraction of
    /// `max(w, h)` for the outer radius — matches the TS
    /// `pen-renderer` so the same `.op` file paints at the same
    /// radial size on native + web + export. Stops + opacity follow
    /// the same convention as the linear variant.
    ///
    /// Default impl falls back to a solid first-stop fill, same as
    /// the linear variant.
    #[allow(clippy::too_many_arguments)]
    fn fill_round_rect_radial_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        stops: &[(f32, Color)],
        _cx_frac: f32,
        _cy_frac: f32,
        _radius_frac: f32,
        opacity: f32,
    ) {
        if let Some((_, c)) = stops.first() {
            let color = Color {
                r: c.r,
                g: c.g,
                b: c.b,
                a: c.a * opacity.clamp(0.0, 1.0),
            };
            self.fill_round_rect(rect, radius, color);
        }
    }

    // Transform stack.
    fn save(&mut self);
    fn restore(&mut self);
    fn translate(&mut self, offset: Point2D);
    /// Scale the current transform around `pivot`. Used for node
    /// `flipX` / `flipY` parity with the TS renderer.
    fn scale(&mut self, _scale: Point2D, _pivot: Point2D) {}
    /// Rotate the current transform `radians` clockwise about
    /// `pivot` (in current-space coordinates, BEFORE the rotation
    /// is applied). Default impl is a no-op so backends without
    /// rotation support degrade gracefully; native + web override
    /// using skia's `Canvas::rotate(angle, point)`.
    fn rotate(&mut self, _radians: f32, _pivot: Point2D) {}

    // Viewport / DPI.
    fn resize(&mut self, width: u32, height: u32);
    fn dpi_scale(&self) -> f32;

    /// Measure the rendered horizontal advance of `text` at
    /// `font_size`. Used by widgets that need precise centering
    /// or right-alignment without round-trip to a layout pass.
    /// Default impl: rough heuristic (~0.55 × font_size per
    /// ASCII char, ~1.0 × font_size per non-ASCII char). Backends
    /// that own a real typeface (Web + Native) override with
    /// `Font::measure_str` for pixel-accurate positioning.
    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.measure_text_weighted(text, font_size, 400)
    }

    /// Like `measure_text` but resolves the typeface using `weight`
    /// (CSS-style 100-900). Backends that serve a weighted variant
    /// can return wider/narrower advances; the wrap pass uses this
    /// so line breaks decided pre-paint match what `draw_text` lays
    /// down post-paint. Default impl forwards to the heuristic
    /// `measure_text` so wasm builds without a real typeface stay
    /// roughly correct.
    fn measure_text_weighted(&mut self, text: &str, font_size: f32, _weight: u16) -> f32 {
        let mut w = 0.0;
        for c in text.chars() {
            w += if c.is_ascii() {
                font_size * 0.55
            } else {
                font_size
            };
        }
        w
    }
}
