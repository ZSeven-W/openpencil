//! OP `RenderBackend` widget-facing facade (spec v19 §5.2).
//!
//! 这是 OP 的设计契约（method-style API：`fill_rect / stroke_rect / draw_text /
//! clip_rect / save / restore / translate / resize / dpi_scale`），与 Jian
//! `jian_core::render::RenderBackend`（command-buffer style，`new_surface /
//! begin_frame / draw(&DrawOp)` etc.）不直接重合。
//!
//! 实现路径 (per §5.2.1)：
//! - `NativeBackend` (shell-native, Step 1a 起 frame-scoped 设计)：**不**直接
//!   impl 该 trait，而是 expose 同名 method 但带 `canvas: &skia_safe::Canvas`
//!   显式参数。Step 1a basic_window demo 通过 `SharedSkiaContext::with_frame`
//!   闭包内调 NativeBackend method。Step 1c+ 真接入 widget tree 时再考虑用
//!   `WithCanvas<'a>` newtype 把 canvas 注入 trait impl。
//! - `WebCanvasKitBackend` (shell-web, Step 1b)：内部用 CanvasKit JS binding。
//! - `MobileBackend` (Step 1f)：内部用 Metal / Vulkan / OpenGL ES。
//!
//! 该模块**不**引 skia-safe / Canvas / GL 类型 —— shell-core 必须 wasm32-clean
//! (per spec §1.2 boundary)。

use jian_core::render::{TextAlign, TextRun};

/// 2D 坐标点（spec §5.2 钉死 `glam::Vec2`）。
pub type Point2D = glam::Vec2;

/// 矩形（origin + size 双 Vec2 表示）。
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub origin: Point2D,
    pub size: Point2D,
}

/// RGBA color（widget facade 层；所有分量 0.0..=1.0）。
///
/// 命名常量定义在 OP 这层（spec v19 round 5 CONCERN-R5-3 fix）；
/// `jian_core::scene::Color` 是 `Color(pub u32)` packed RGBA，没有
/// RED/BLACK/etc 命名常量，调用方用 `JianColor::rgb(...)` 显式构造。
#[derive(Debug, Clone, Copy)]
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

/// OP 的 TextLayout —— v19 删 parley 后定义在 OP 这层；薄薄包装
/// `jian_core::render::TextRun`。
///
/// 不持 layout context / glyph cache（shell-core wasm32-clean，不能引
/// skia/parley/icu）；真 layout 在 NativeBackend::draw_text 时通过
/// `jian_skia::SkiaBackend` 的 textlayout feature（skia textlayout,
/// ICU+harfbuzz）做 shape + line break。
///
/// Step 1a TextLayout 仅作 "已 shape 的 TextRun 集合" 占位；
/// caret/selection/bidi/wrap 推 Step 1c+。signatures 钉死，避免后续 Step
/// API break。
#[derive(Debug, Clone)]
pub struct TextLayout {
    runs: Vec<TextRun>,
}

impl TextLayout {
    /// 单 run 构造（Step 1a 唯一活跃路径；Phase B/C demo + raster_text_smoke 用）。
    ///
    /// 字段对齐 `jian_core::render::TextRun` (`vendor/jian/crates/jian-core/src/render/paint.rs:77-94`)：
    /// **TextRun 没有 `Default` impl**，必须显式构造所有字段。
    /// - `content` / `font_family` / `font_size` / `color` / `origin`：调用方传入
    /// - `font_weight: 400` (CSS Normal)
    /// - `max_width: 0.0`（"unknown; render at origin with no alignment adjustment"）
    /// - `align: TextAlign::Start`
    /// - `line_height: 0.0`（"default"）
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

    /// 已 shape 的 TextRun 集合视图。
    pub fn runs(&self) -> &[TextRun] {
        &self.runs
    }

    /// 平移所有 run 的 origin（NativeBackend::draw_text 在 widget origin
    /// 基础上做平移）。返回新 layout，原 layout 不动。
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

/// Backend 抽象（widget-facing facade，spec §5.2）。
///
/// 注：trait 不加 `Send` bound —— skia-safe 类型 `!Send`（rust-skia
/// thread-bound），Backend 在 render thread 内单独使用，无跨线程需求。
///
/// `NativeBackend` (shell-native) 在 1a **不**直接 impl 该 trait（v19 round 3
/// BLOCK-R3-3 fix —— frame-scoped 设计避免跨帧 borrow），而是 expose 同名
/// method 但带 `canvas: &skia_safe::Canvas` 显式参数。trait 签名内不出现任何
/// Skia / GPU-backend 特定类型。
pub trait RenderBackend {
    /// Begin 帧，backend 内部维护 current frame state（不暴露 canvas 类型）。
    fn begin_frame(&mut self);
    fn end_frame(&mut self);

    // 绘图 primitives —— widgets 调这些，不直接操作 canvas。
    fn fill_rect(&mut self, rect: Rect, color: Color);
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32);
    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D);
    fn clip_rect(&mut self, rect: Rect);

    // 变换栈。
    fn save(&mut self);
    fn restore(&mut self);
    fn translate(&mut self, offset: Point2D);

    // viewport / dpi。
    fn resize(&mut self, width: u32, height: u32);
    fn dpi_scale(&self) -> f32;
}
