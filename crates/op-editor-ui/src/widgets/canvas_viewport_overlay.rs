//! Selection overlay + fill/stroke node-paint helpers split out
//! of `canvas_viewport.rs` to keep that file under the 800-line
//! ceiling.

use crate::layout_scene::{LayoutScene, SceneNode};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::Viewport;

/// Pen-tool rubber-band — from the last committed anchor of the
/// in-progress path to the cursor (when both are known). Painted
/// half-alpha so it reads as a draft segment.
///
/// The in-progress path id + cursor doc coord come from the editor's
/// pen-draft state; the path geometry is read from the resolved
/// [`LayoutScene`].
pub fn paint_pen_rubber_band(
    cx: &mut PaintCx<'_>,
    scene: &LayoutScene,
    pen_in_progress: Option<&str>,
    pen_cursor_doc: Option<Point2D>,
    canvas_rect: Rect,
    viewport: &Viewport,
) {
    let (Some(pen_id), Some(cursor_doc)) = (pen_in_progress, pen_cursor_doc) else {
        return;
    };
    let Some(page) = scene.active_page() else {
        return;
    };
    let Some(node) = page.find(pen_id) else {
        return;
    };
    let Some(last) = node.points.last().copied() else {
        return;
    };
    let zoom = viewport.zoom;
    let origin = Point2D::new(
        canvas_rect.origin.x + viewport.pan_x,
        canvas_rect.origin.y + viewport.pan_y,
    );
    let from = Point2D::new(origin.x + last.x * zoom, origin.y + last.y * zoom);
    let to = Point2D::new(
        origin.x + cursor_doc.x * zoom,
        origin.y + cursor_doc.y * zoom,
    );
    let width = node
        .stroke
        .map(|s| (s.width * zoom).max(1.0))
        .unwrap_or((1.5_f32).max(zoom));
    let mut color = node.stroke.map(|s| s.color).unwrap_or(Color::BLACK);
    color.a *= 0.5;
    cx.backend.stroke_line(from, to, color, width);
}

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
    if let Some(fill) = fill {
        if use_round {
            cx.backend.fill_round_rect(world_rect, r, fill);
        } else {
            cx.backend.fill_rect(world_rect, fill);
        }
    }
    if let Some(stroke) = node.stroke {
        if use_round {
            cx.backend
                .stroke_round_rect(world_rect, r, stroke.color, stroke.width * zoom);
        } else {
            cx.backend
                .stroke_rect(world_rect, stroke.color, stroke.width * zoom);
        }
    }
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
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if is_cjk(ch) {
            let mut probe = current.clone();
            probe.push(ch);
            if backend.measure_text_weighted(&probe, font_size, weight) > max_w
                && !current.is_empty()
            {
                out.push(std::mem::take(&mut current));
                current.push(ch);
            } else {
                current = probe;
            }
            i += 1;
        } else if ch == ' ' {
            let mut probe = current.clone();
            probe.push(' ');
            if backend.measure_text_weighted(&probe, font_size, weight) > max_w
                && !current.is_empty()
            {
                out.push(std::mem::take(&mut current));
            } else {
                current = probe;
            }
            i += 1;
        } else {
            let mut word = String::new();
            while i < chars.len() && chars[i] != ' ' && !is_cjk(chars[i]) {
                word.push(chars[i]);
                i += 1;
            }
            let mut probe = current.clone();
            probe.push_str(&word);
            if backend.measure_text_weighted(&probe, font_size, weight) > max_w
                && !current.is_empty()
            {
                out.push(std::mem::take(&mut current));
                current.push_str(&word);
            } else {
                current = probe;
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
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
}
