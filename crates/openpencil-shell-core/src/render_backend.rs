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

    // Transform stack.
    fn save(&mut self);
    fn restore(&mut self);
    fn translate(&mut self, offset: Point2D);

    // Viewport / DPI.
    fn resize(&mut self, width: u32, height: u32);
    fn dpi_scale(&self) -> f32;
}
