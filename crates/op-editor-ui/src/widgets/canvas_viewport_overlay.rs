//! Selection overlay + fill/stroke node-paint helpers split out
//! of `canvas_viewport.rs` to keep that file under the 800-line
//! ceiling.

use crate::layout_scene::{SceneGradient, SceneNode, SceneShader, SceneStrokeAlign};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

// The legacy `paint_pen_rubber_band` (node-stroke-colored straight
// line) was retired in favor of `canvas_path_overlay.rs`, which ports
// the TS pen preview verbatim (dashed selection-blue rubber band +
// anchor/handle dots).

/// Paint a 1 px primary-tinted outline around `world_rect` plus
/// 8 handle anchors. Container selections paint with a softened
/// outline alpha so child content stays visually dominant. When
/// `show_handles` is false the outline paints without the 8 grab
/// dots — used for multi-select where only one bounding box per
/// node is shown (Figma parity; handles only on single-select).
pub fn paint_selection_overlay(
    cx: &mut PaintCx<'_>,
    world_rect: Rect,
    theme: &Theme,
    is_container: bool,
    show_handles: bool,
) {
    // Outline — draw as 4 individual stroke_line calls so the
    // skia AA path runs (stroke_rect goes through jian which
    // doesn't enable AA by default).
    let left = world_rect.origin.x;
    let right = world_rect.origin.x + world_rect.size.x;
    let top = world_rect.origin.y;
    let bottom = world_rect.origin.y + world_rect.size.y;
    let stroke_w = 1.0;
    let outline_color = if is_container {
        crate::Color {
            r: theme.primary.r,
            g: theme.primary.g,
            b: theme.primary.b,
            a: theme.primary.a * 0.55,
        }
    } else {
        theme.primary
    };
    cx.backend.stroke_line(
        Point2D::new(left, top),
        Point2D::new(right, top),
        outline_color,
        stroke_w,
    );
    cx.backend.stroke_line(
        Point2D::new(right, top),
        Point2D::new(right, bottom),
        outline_color,
        stroke_w,
    );
    cx.backend.stroke_line(
        Point2D::new(right, bottom),
        Point2D::new(left, bottom),
        outline_color,
        stroke_w,
    );
    cx.backend.stroke_line(
        Point2D::new(left, bottom),
        Point2D::new(left, top),
        outline_color,
        stroke_w,
    );

    if !show_handles {
        return;
    }

    // 8 handle anchors.
    let mid_x = world_rect.origin.x + world_rect.size.x / 2.0;
    let mid_y = world_rect.origin.y + world_rect.size.y / 2.0;
    let anchors = [
        (left, top),
        (mid_x, top),
        (right, top),
        (right, mid_y),
        (right, bottom),
        (mid_x, bottom),
        (left, bottom),
        (left, mid_y),
    ];

    let handle_size = 7.0;
    let half = handle_size / 2.0;
    let radius = 1.0;
    let bg = crate::Color::WHITE;
    for (x, y) in anchors {
        let rect = Rect {
            origin: Point2D::new(x - half, y - half),
            size: Point2D::new(handle_size, handle_size),
        };
        cx.backend.fill_round_rect(rect, radius, bg);
        cx.backend
            .stroke_round_rect(rect, radius, theme.primary, 1.0);
    }
}

/// Paint a node's fill rect followed by its stroke rect. Stroke
/// width is scaled by `zoom` so it stays visually constant under
/// canvas zoom. Effective `fill` is passed explicitly — the scene's
/// `SceneNode.fill` is already `$ref`-resolved by the layout-scene
/// builder, so the caller forwards it straight through.
pub fn paint_fill_then_stroke(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    fill: Option<crate::Color>,
) {
    let r = node.corner_radius * zoom;
    let use_round = r > 0.5;
    // A native SkSL shader fill wins over everything (gradient / solid)
    // when present. Gradients in turn win over solid `fill`. The scene
    // builder leaves the first stop's / fallback colour in `fill` for
    // paint paths that don't grok the richer body, so the shader / gradient
    // body is the more faithful representation here.
    if let Some(shader) = node.shader.as_ref() {
        paint_shader_rect(cx, shader, world_rect, if use_round { r } else { 0.0 });
    } else if let Some(gradient) = node.gradient.as_ref() {
        paint_gradient_rect(cx, gradient, world_rect, if use_round { r } else { 0.0 });
    } else if let Some(fill) = fill {
        if use_round {
            cx.backend.fill_round_rect(world_rect, r, fill);
        } else {
            cx.backend.fill_rect(world_rect, fill);
        }
    }
    if let Some(stroke) = node.stroke {
        if let Some(sides) = stroke
            .sides
            .filter(|sides| !stroke_sides_are_uniform(*sides))
        {
            paint_sided_rect_stroke(cx, world_rect, stroke.color, sides, zoom);
        } else {
            let w = stroke.width * zoom;
            let (rect, r) = align_stroke_rect(world_rect, r, w, stroke.align);
            if use_round {
                cx.backend.stroke_round_rect(rect, r, stroke.color, w);
            } else {
                cx.backend.stroke_rect(rect, stroke.color, w);
            }
        }
    }
}

/// Backends stroke centered on the rect path; INSIDE / OUTSIDE
/// alignment shifts the path by half a width so the painted band
/// lands inside / outside the node edge (Figma defaults to INSIDE).
/// The corner radius follows the shifted path.
pub fn align_stroke_rect(
    rect: Rect,
    radius: f32,
    width: f32,
    align: SceneStrokeAlign,
) -> (Rect, f32) {
    let half = width / 2.0;
    match align {
        SceneStrokeAlign::Center => (rect, radius),
        SceneStrokeAlign::Inside => (
            Rect::xywh(
                rect.origin.x + half,
                rect.origin.y + half,
                (rect.size.x - width).max(0.0),
                (rect.size.y - width).max(0.0),
            ),
            (radius - half).max(0.0),
        ),
        SceneStrokeAlign::Outside => (
            Rect::xywh(
                rect.origin.x - half,
                rect.origin.y - half,
                rect.size.x + width,
                rect.size.y + width,
            ),
            if radius > 0.0 { radius + half } else { 0.0 },
        ),
    }
}

fn stroke_sides_are_uniform(sides: [f32; 4]) -> bool {
    sides
        .iter()
        .all(|side| (*side - sides[0]).abs() < f32::EPSILON)
}

fn paint_sided_rect_stroke(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    color: crate::Color,
    sides: [f32; 4],
    zoom: f32,
) {
    let x0 = rect.origin.x;
    let y0 = rect.origin.y;
    let x1 = rect.origin.x + rect.size.x;
    let y1 = rect.origin.y + rect.size.y;
    let [top, right, bottom, left] = sides;
    if top > 0.0 {
        let width = top * zoom;
        cx.backend.stroke_line(
            Point2D::new(x0, y0 + width / 2.0),
            Point2D::new(x1, y0 + width / 2.0),
            color,
            width,
        );
    }
    if right > 0.0 {
        let width = right * zoom;
        cx.backend.stroke_line(
            Point2D::new(x1 - width / 2.0, y0),
            Point2D::new(x1 - width / 2.0, y1),
            color,
            width,
        );
    }
    if bottom > 0.0 {
        let width = bottom * zoom;
        cx.backend.stroke_line(
            Point2D::new(x0, y1 - width / 2.0),
            Point2D::new(x1, y1 - width / 2.0),
            color,
            width,
        );
    }
    if left > 0.0 {
        let width = left * zoom;
        cx.backend.stroke_line(
            Point2D::new(x0 + width / 2.0, y0),
            Point2D::new(x0 + width / 2.0, y1),
            color,
            width,
        );
    }
}

/// Dispatch a [`SceneGradient`] onto the right `RenderBackend`
/// gradient method. Stops are flattened into `(offset, Color)` pairs
/// because the trait stays free of scene-specific types.
pub(crate) fn paint_gradient_rect(
    cx: &mut PaintCx<'_>,
    gradient: &SceneGradient,
    rect: Rect,
    corner_radius: f32,
) {
    match gradient {
        SceneGradient::Linear {
            angle_deg,
            opacity,
            stops,
        } => {
            let flat: Vec<(f32, Color)> = stops.iter().map(|s| (s.offset, s.color)).collect();
            cx.backend.fill_round_rect_linear_gradient(
                rect,
                corner_radius,
                &flat,
                *angle_deg,
                *opacity,
            );
        }
        SceneGradient::Radial {
            cx: gx,
            cy,
            radius,
            opacity,
            stops,
        } => {
            let flat: Vec<(f32, Color)> = stops.iter().map(|s| (s.offset, s.color)).collect();
            cx.backend.fill_round_rect_radial_gradient(
                rect,
                corner_radius,
                &flat,
                *gx,
                *cy,
                *radius,
                *opacity,
            );
        }
        SceneGradient::Mesh {
            rows,
            cols,
            colors,
            opacity,
        } => {
            cx.backend.fill_round_rect_mesh_gradient(
                rect,
                corner_radius,
                *rows,
                *cols,
                colors,
                *opacity,
            );
        }
    }
}

/// Dispatch a [`SceneShader`] onto the `RenderBackend` shader method.
/// Uniforms are flattened into `(name, values)` slices because the
/// backend trait stays free of scene-specific types. Only the native
/// host runs the program; every other backend paints `fallback` solid.
pub(crate) fn paint_shader_rect(
    cx: &mut PaintCx<'_>,
    shader: &SceneShader,
    rect: Rect,
    corner_radius: f32,
) {
    let uniforms: Vec<(&str, &[f32])> = shader
        .uniforms
        .iter()
        .map(|u| (u.name.as_str(), u.values.as_slice()))
        .collect();
    cx.backend.fill_round_rect_shader(
        rect,
        corner_radius,
        &shader.sksl,
        &uniforms,
        shader.opacity,
        shader.fallback,
    );
}

/// Greedy line-wrap for canvas text nodes with explicit width.
/// Splits on `\n` first (mirrors `pen-renderer/text-renderer.ts`
/// which pre-splits before calling `wrapLine`), then wraps each
/// segment via the per-character-CJK / word-Latin algorithm from
/// `pen-renderer/paint-utils.ts::wrapLine`. Empty segments survive
/// as blank lines so authored paragraph breaks paint as gaps.
pub(super) fn wrap_text(
    backend: &mut dyn crate::RenderBackend,
    text: &str,
    font_size: f32,
    max_w: f32,
    weight: u16,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for segment in text.split('\n') {
        if max_w <= 0.0 || backend.measure_text_weighted(segment, font_size, weight) <= max_w {
            out.push(segment.to_string());
            continue;
        }
        wrap_segment(backend, segment, font_size, max_w, weight, &mut out);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Wrap a single newline-free segment.
fn wrap_segment(
    backend: &mut dyn crate::RenderBackend,
    segment: &str,
    font_size: f32,
    max_w: f32,
    weight: u16,
    out: &mut Vec<String>,
) {
    let chars: Vec<char> = segment.chars().collect();
    let mut current = String::new();
    let mut probe = String::with_capacity(segment.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if is_cjk(ch) {
            let mut ch_buf = [0; 4];
            build_probe(&mut probe, &current, ch.encode_utf8(&mut ch_buf));
            if backend.measure_text_weighted(&probe, font_size, weight) > max_w
                && !current.is_empty()
            {
                out.push(std::mem::take(&mut current));
                current.push(ch);
            } else {
                std::mem::swap(&mut current, &mut probe);
            }
            i += 1;
        } else if ch == ' ' {
            build_probe(&mut probe, &current, " ");
            if backend.measure_text_weighted(&probe, font_size, weight) > max_w
                && !current.is_empty()
            {
                out.push(std::mem::take(&mut current));
            } else {
                std::mem::swap(&mut current, &mut probe);
            }
            i += 1;
        } else {
            let mut word = String::new();
            while i < chars.len() && chars[i] != ' ' && !is_cjk(chars[i]) {
                word.push(chars[i]);
                i += 1;
            }
            build_probe(&mut probe, &current, &word);
            if backend.measure_text_weighted(&probe, font_size, weight) > max_w
                && !current.is_empty()
            {
                out.push(std::mem::take(&mut current));
                current.push_str(&word);
            } else {
                std::mem::swap(&mut current, &mut probe);
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
}

fn build_probe(probe: &mut String, current: &str, suffix: &str) {
    probe.clear();
    probe.push_str(current);
    probe.push_str(suffix);
}

/// Codepoints that wrap per-character (no inter-word spaces).
/// Mirrors `pen-core/layout/text-measure.ts::isCjkCodePoint`.
fn is_cjk(ch: char) -> bool {
    let c = ch as u32;
    (0x4e00..=0x9fff).contains(&c)
        || (0x3400..=0x4dbf).contains(&c)
        || (0x3040..=0x30ff).contains(&c)
        || (0xac00..=0xd7af).contains(&c)
        || (0x3000..=0x303f).contains(&c)
        || (0xff00..=0xffef).contains(&c)
}

#[cfg(test)]
mod wrap_tests {
    use super::*;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

    /// Backend that measures every char as 10 px wide regardless of
    /// glyph — keeps the test arithmetic exact.
    struct UniformBackend;
    impl RenderBackend for UniformBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn fill_oval(&mut self, _: Rect, _: Color) {}
        fn stroke_oval(&mut self, _: Rect, _: Color, _: f32) {}
        fn fill_polygon(&mut self, _: &[Point2D], _: Color) {}
        fn stroke_polygon(&mut self, _: &[Point2D], _: Color, _: f32) {}
        fn rotate(&mut self, _: f32, _: Point2D) {}
        fn fill_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: f32, _: Color) {}
        fn measure_text(&mut self, text: &str, _: f32) -> f32 {
            text.chars().count() as f32 * 10.0
        }
        fn measure_text_weighted(&mut self, text: &str, _: f32, _: u16) -> f32 {
            text.chars().count() as f32 * 10.0
        }
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn ascii_word_breaks_at_word_boundary() {
        let mut b = UniformBackend;
        // "hello world foo" = 15 chars × 10 = 150 px. Budget 100.
        // Greedy walk: "hello " (60) fits, "hello world" (110)
        // exceeds — flush "hello ", start with "world". Then
        // "world " (60), "world foo" (90) fits.
        let lines = wrap_text(&mut b, "hello world foo", 13.0, 100.0, 400);
        assert_eq!(lines, vec!["hello ", "world foo"]);
    }

    #[test]
    fn cjk_breaks_per_character() {
        let mut b = UniformBackend;
        // 8 CJK chars × 10 px = 80. Budget 50 forces a break every
        // 5 chars. Pure split_whitespace would treat the whole
        // string as one word and return a single 80-px line.
        let lines = wrap_text(&mut b, "中文测试段落很长哦", 13.0, 50.0, 400);
        assert!(lines.len() >= 2, "got {:?}", lines);
        for line in &lines {
            assert!(line.chars().count() <= 5, "line too long: {:?}", line);
        }
    }

    #[test]
    fn explicit_newlines_become_line_breaks() {
        let mut b = UniformBackend;
        // "ab\ncd\nef" — each segment fits the 100 px budget, but
        // we still split on `\n`. Pure wrap (no newline handling)
        // would join everything as "abcdef" and return one line.
        let lines = wrap_text(&mut b, "ab\ncd\nef", 13.0, 100.0, 400);
        assert_eq!(lines, vec!["ab", "cd", "ef"]);
    }

    #[test]
    fn blank_lines_survive_in_output() {
        let mut b = UniformBackend;
        // "para1\n\npara2" — the empty segment between the two
        // paragraphs must paint as a blank line so authored gaps
        // survive.
        let lines = wrap_text(&mut b, "para1\n\npara2", 13.0, 200.0, 400);
        assert_eq!(lines, vec!["para1", "", "para2"]);
    }

    #[test]
    fn newlines_split_before_wrap_runs() {
        let mut b = UniformBackend;
        // "hello world\nfoo bar baz" with budget 100. First segment
        // "hello world" = 110 → wraps to ["hello ", "world"].
        // Second segment "foo bar baz" = 110 → ["foo bar ", "baz"].
        let lines = wrap_text(&mut b, "hello world\nfoo bar baz", 13.0, 100.0, 400);
        assert_eq!(lines, vec!["hello ", "world", "foo bar ", "baz"]);
    }

    #[test]
    fn cjk_latin_mix_breaks_within_cjk_run() {
        let mut b = UniformBackend;
        let lines = wrap_text(&mut b, "hello 中文段落", 13.0, 60.0, 400);
        assert!(lines.len() >= 2, "got {:?}", lines);
        assert!(lines[0].starts_with("hello"));
    }

    /// Stub backend whose advance widths depend on the weight —
    /// 10 px per char at weight 400, 14 px per char at weight 700.
    /// Mirrors the real native backend behaviour where bold glyphs
    /// have wider advances; if wrap_text ignored weight, the same
    /// string would break at a different line index.
    struct WeightedBackend;
    impl RenderBackend for WeightedBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn fill_oval(&mut self, _: Rect, _: Color) {}
        fn stroke_oval(&mut self, _: Rect, _: Color, _: f32) {}
        fn fill_polygon(&mut self, _: &[Point2D], _: Color) {}
        fn stroke_polygon(&mut self, _: &[Point2D], _: Color, _: f32) {}
        fn rotate(&mut self, _: f32, _: Point2D) {}
        fn fill_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: f32, _: Color) {}
        fn measure_text(&mut self, text: &str, _: f32) -> f32 {
            text.chars().count() as f32 * 10.0
        }
        fn measure_text_weighted(&mut self, text: &str, _: f32, weight: u16) -> f32 {
            let per_char = if weight >= 600 { 14.0 } else { 10.0 };
            text.chars().count() as f32 * per_char
        }
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn wrap_uses_weighted_advances() {
        // "hello world" at weight 700 = 11 chars × 14 = 154 px.
        // Budget 120 forces a break ("hello " = 6×14=84 fits, +5×14=70 doesn't).
        // At weight 400 (110 px) it'd fit on one line — codex's
        // BLOCK: wrap was measuring at 400 while paint used 700.
        let mut b = WeightedBackend;
        let bold = wrap_text(&mut b, "hello world", 13.0, 120.0, 700);
        let regular = wrap_text(&mut b, "hello world", 13.0, 120.0, 400);
        assert_eq!(regular, vec!["hello world"], "weight 400 fits");
        assert!(bold.len() >= 2, "weight 700 must wrap: {:?}", bold);
    }

    #[test]
    fn probe_builder_reuses_existing_string_allocation() {
        let mut probe = String::with_capacity(64);
        let ptr = probe.as_ptr();

        build_probe(&mut probe, "hello", " world");

        assert_eq!(probe, "hello world");
        assert_eq!(
            probe.as_ptr(),
            ptr,
            "probe construction should reuse the existing String allocation"
        );
    }
}
