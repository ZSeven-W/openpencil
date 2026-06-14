//! Inline canvas text-edit geometry — the single source of truth for
//! how an edited Text node's content maps between byte offsets and
//! painted pixels.
//!
//! [`text_edit_layout`] resolves the SAME line layout the canvas
//! painter uses (`canvas_viewport_paint::paint_text_node` calls it
//! too), so caret xy, selection rects, and click-to-caret hit-testing
//! all agree with the painted glyphs: same CJK-aware wrap
//! (`canvas_viewport_overlay::wrap_text`), same wrap-width tolerance,
//! same per-line alignment, same letter-spacing advance model.
//!
//! All geometry is in DOCUMENT space (the painter applies the
//! viewport transform as a canvas transform); the host converts
//! screen presses to doc points before calling [`TextEditLayout::offset_at_point`].

use crate::layout_scene::{SceneNode, SceneTextAlign};
use crate::{Point2D, Rect, RenderBackend};
use jian_core::text_input::prev_char_boundary;

/// Resolved line layout of a Text node's content.
pub struct TextEditLayout {
    /// Painted lines, in paint order. Never empty (empty content
    /// paints as one empty line).
    pub lines: Vec<String>,
    /// Byte offset of each line's start within the content. Wrap
    /// separators (dropped spaces, `\n`) fall in the gaps between
    /// one line's end and the next line's start.
    pub line_starts: Vec<usize>,
    /// Resolved font size (doc px; painter default applied).
    pub font_size: f32,
    /// Resolved CSS-style weight (painter default applied).
    pub weight: u16,
    /// Line advance in doc px (`font_size × line-height factor`).
    pub line_h: f32,
    /// Extra tracking between glyphs (doc px).
    pub letter_spacing: f32,
    /// Width the per-line alignment distributes against.
    pub align_width: f32,
    /// Node bounds origin (doc space) — the first line's top-left.
    pub origin: Point2D,
    pub text_align: SceneTextAlign,
}

/// Resolve the painted line layout for `node` — paint-parity
/// defaults (13 px font, weight 400, 1.2 line height) and the same
/// wrap decision (`text_wrap` + 5% / half-font-size tolerance) as
/// `paint_text_node`.
pub fn text_edit_layout(backend: &mut dyn RenderBackend, node: &SceneNode) -> TextEditLayout {
    let text = node.text.as_deref().unwrap_or("");
    let font_size = if node.font_size > 0.0 {
        node.font_size
    } else {
        13.0
    };
    let weight = if node.font_weight > 0 {
        node.font_weight
    } else {
        400
    };
    let line_height = if node.line_height > 0.0 {
        node.line_height
    } else {
        1.2
    };
    let wrap_width_doc = if node.text_wrap {
        // Same tolerance as the painter (TS renderer parity): without
        // it CJK strings that exactly fit can wrap differently here
        // than on the painted canvas.
        let doc_width = node.bounds.size.x;
        let tolerance = (doc_width * 0.05).ceil().min((font_size * 0.5).ceil());
        Some(doc_width + tolerance)
    } else {
        None
    };
    let lines: Vec<String> = if let Some(doc_wrap_width) = wrap_width_doc {
        super::canvas_viewport_overlay::wrap_text(backend, text, font_size, doc_wrap_width, weight)
    } else {
        text.split('\n').map(str::to_string).collect()
    };
    let line_starts = line_start_offsets(text, &lines);
    TextEditLayout {
        line_starts,
        font_size,
        weight,
        line_h: font_size * line_height,
        letter_spacing: node.letter_spacing,
        align_width: wrap_width_doc.unwrap_or(node.bounds.size.x),
        origin: node.bounds.origin,
        text_align: node.text_align,
        lines,
    }
}

/// Measure a painted line's full width — substring advance plus
/// letter-spacing BETWEEN glyphs (mirrors the painter).
pub fn measure_line_width(
    backend: &mut dyn RenderBackend,
    line: &str,
    font_size: f32,
    weight: u16,
    letter_spacing: f32,
) -> f32 {
    let base = backend.measure_text_weighted(line, font_size, weight);
    let extra = line.chars().count().saturating_sub(1) as f32 * letter_spacing;
    base + extra
}

impl TextEditLayout {
    /// Byte `(start, end)` range of each painted line within the
    /// content — the contract `text_edit_caret_vertical` consumes.
    pub fn line_ranges(&self) -> Vec<(usize, usize)> {
        self.lines
            .iter()
            .zip(&self.line_starts)
            .map(|(line, start)| (*start, *start + line.len()))
            .collect()
    }

    /// Index of the line the caret at byte `offset` displays on: the
    /// LAST line whose start is `<= offset`, so a caret at an exact
    /// wrap boundary shows at the start of the later line (textarea
    /// behaviour).
    pub fn caret_line(&self, offset: usize) -> usize {
        let mut idx = 0;
        for (i, start) in self.line_starts.iter().enumerate() {
            if *start <= offset {
                idx = i;
            } else {
                break;
            }
        }
        idx
    }

    /// Alignment-adjusted left edge of `line` (doc space).
    fn line_x(&self, backend: &mut dyn RenderBackend, line: &str) -> f32 {
        match self.text_align {
            SceneTextAlign::Left | SceneTextAlign::Justify => self.origin.x,
            SceneTextAlign::Center => {
                let line_w = measure_line_width(
                    backend,
                    line,
                    self.font_size,
                    self.weight,
                    self.letter_spacing,
                );
                self.origin.x + (self.align_width - line_w).max(0.0) / 2.0
            }
            SceneTextAlign::Right => {
                let line_w = measure_line_width(
                    backend,
                    line,
                    self.font_size,
                    self.weight,
                    self.letter_spacing,
                );
                self.origin.x + (self.align_width - line_w).max(0.0)
            }
        }
    }

    /// Horizontal advance of `line`'s first `byte_offset` bytes —
    /// the caret x relative to the line's left edge. Mirrors the
    /// painter's two advance models: substring measure when there is
    /// no letter spacing (whole-line draw), per-char measure plus
    /// spacing AFTER every char when there is (per-char draw).
    fn prefix_width(&self, backend: &mut dyn RenderBackend, line: &str, byte_offset: usize) -> f32 {
        let offset = prev_char_boundary(line, byte_offset.min(line.len()));
        if offset == 0 {
            return 0.0;
        }
        let prefix = &line[..offset];
        if self.letter_spacing.abs() < f32::EPSILON {
            backend.measure_text_weighted(prefix, self.font_size, self.weight)
        } else {
            let mut w = 0.0;
            for ch in prefix.chars() {
                let mut buf = [0u8; 4];
                w += backend.measure_text_weighted(
                    ch.encode_utf8(&mut buf),
                    self.font_size,
                    self.weight,
                ) + self.letter_spacing;
            }
            w
        }
    }

    /// Doc-space caret position for byte `offset`: `(x, line_top_y)`.
    pub fn caret_position(&self, backend: &mut dyn RenderBackend, offset: usize) -> (f32, f32) {
        let line_idx = self.caret_line(offset);
        let line = &self.lines[line_idx];
        let start = self.line_starts[line_idx];
        let local = offset.saturating_sub(start).min(line.len());
        let x = self.line_x(backend, line) + self.prefix_width(backend, line, local);
        let y = self.origin.y + line_idx as f32 * self.line_h;
        (x, y)
    }

    /// One highlight rect per painted line intersecting the byte
    /// range `start..end` (caller passes them ordered). Heights match
    /// the legacy select-all wash (`font_size × 1.2`).
    pub fn selection_rects(
        &self,
        backend: &mut dyn RenderBackend,
        start: usize,
        end: usize,
    ) -> Vec<Rect> {
        let mut out = Vec::new();
        if start >= end {
            return out;
        }
        for (idx, line) in self.lines.iter().enumerate() {
            let line_start = self.line_starts[idx];
            let line_end = line_start + line.len();
            if end <= line_start || start > line_end {
                continue;
            }
            let local_start = start.saturating_sub(line_start).min(line.len());
            let local_end = end.saturating_sub(line_start).min(line.len());
            let line_x = self.line_x(backend, line);
            let x0 = line_x + self.prefix_width(backend, line, local_start);
            let x1 = line_x + self.prefix_width(backend, line, local_end);
            if x1 <= x0 + 0.5 {
                continue;
            }
            out.push(Rect {
                origin: Point2D::new(x0, self.origin.y + idx as f32 * self.line_h),
                size: Point2D::new(x1 - x0, self.font_size * 1.2),
            });
        }
        out
    }

    /// Map a doc-space point to the nearest byte offset — the inverse
    /// of the paint layout. The y picks the line (clamped to the
    /// first/last line, so drags above/below the node still select);
    /// the x walks the line's glyph advances with the midpoint rule.
    pub fn offset_at_point(&self, backend: &mut dyn RenderBackend, point: Point2D) -> usize {
        let line_idx = (((point.y - self.origin.y) / self.line_h).floor() as isize)
            .clamp(0, self.lines.len() as isize - 1) as usize;
        let line = &self.lines[line_idx];
        let start = self.line_starts[line_idx];
        let local_x = point.x - self.line_x(backend, line);
        if local_x <= 0.0 {
            return start;
        }
        start + self.offset_in_line_at_x(backend, line, local_x)
    }

    fn offset_in_line_at_x(
        &self,
        backend: &mut dyn RenderBackend,
        line: &str,
        local_x: f32,
    ) -> usize {
        let mut boundaries: Vec<usize> = line.char_indices().map(|(idx, _)| idx).collect();
        boundaries.push(line.len());
        let char_count = boundaries.len().saturating_sub(1);
        if char_count == 0 {
            return 0;
        }
        let mut lo = 0usize;
        let mut hi = char_count;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let before = self.prefix_width(backend, line, boundaries[mid]);
            let after = self.prefix_width(backend, line, boundaries[mid + 1]);
            if local_x < (before + after) / 2.0 {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        if lo >= char_count {
            line.len()
        } else {
            boundaries[lo]
        }
    }
}

/// Byte offsets where each painted line starts in `text`. The wrap
/// never reorders content, so each line is found left-to-right from
/// the previous line's end (wrap-dropped separators fall in the
/// gaps). Mirrors `ai_chat_input_text::line_start_offsets`.
fn line_start_offsets(text: &str, lines: &[String]) -> Vec<usize> {
    let mut out = Vec::with_capacity(lines.len());
    let mut search_from = 0;
    for line in lines {
        if line.is_empty() {
            // Blank line (`\n\n`) — sits one byte past the previous
            // line's separator when possible.
            let pos = text[search_from..]
                .find('\n')
                .map(|rel| search_from + rel + 1)
                .unwrap_or(search_from)
                .min(text.len());
            out.push(if out.is_empty() { 0 } else { pos });
            search_from = pos;
            continue;
        }
        let next = text[search_from..]
            .find(line.as_str())
            .map(|rel| search_from + rel)
            .unwrap_or(search_from);
        out.push(next);
        search_from = (next + line.len()).min(text.len());
    }
    if out.is_empty() {
        out.push(0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_scene::NodeKind;
    use crate::{Color, TextLayout};

    /// Backend measuring every char at 10 px — exact arithmetic.
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
        fn dpi_scale(&self) -> f32 {
            1.0
        }
        fn measure_text_weighted(&mut self, text: &str, _: f32, _: u16) -> f32 {
            text.chars().count() as f32 * 10.0
        }
    }

    struct CountingBackend {
        measures: usize,
    }

    impl CountingBackend {
        fn reset(&mut self) {
            self.measures = 0;
        }
    }

    impl RenderBackend for CountingBackend {
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
        fn dpi_scale(&self) -> f32 {
            1.0
        }
        fn measure_text_weighted(&mut self, text: &str, _: f32, _: u16) -> f32 {
            self.measures += 1;
            text.chars().count() as f32 * 10.0
        }
    }

    fn text_node(content: &str, w: f32) -> SceneNode {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(100.0, 50.0, w, 40.0);
        node.text = Some(content.to_string());
        node.font_size = 20.0;
        node.line_height = 1.0;
        node
    }

    #[test]
    fn newline_layout_maps_line_starts() {
        let mut b = UniformBackend;
        let layout = text_edit_layout(&mut b, &text_node("hello\nworld", 200.0));
        assert_eq!(layout.lines, vec!["hello", "world"]);
        assert_eq!(layout.line_ranges(), vec![(0, 5), (6, 11)]);
    }

    #[test]
    fn wrapped_layout_skips_dropped_spaces() {
        let mut b = UniformBackend;
        let mut node = text_node("hello world", 60.0);
        node.text_wrap = true;
        // 60 px + tolerance(min(ceil(3), ceil(10)) = 3) = 63 → 6 chars.
        let layout = text_edit_layout(&mut b, &node);
        assert_eq!(layout.lines, vec!["hello ", "world"]);
        assert_eq!(layout.line_ranges(), vec![(0, 6), (6, 11)]);
    }

    #[test]
    fn caret_position_walks_lines() {
        let mut b = UniformBackend;
        let layout = text_edit_layout(&mut b, &text_node("hello\nworld", 200.0));
        // Caret at byte 8 → line 1, col 2 → x = 100 + 20, y = 50 + 20.
        assert_eq!(layout.caret_position(&mut b, 8), (120.0, 70.0));
        // Caret at the very start.
        assert_eq!(layout.caret_position(&mut b, 0), (100.0, 50.0));
        // Caret past the end clamps to the last line's end.
        assert_eq!(layout.caret_position(&mut b, 99), (150.0, 70.0));
    }

    #[test]
    fn offset_at_point_round_trips_caret_position() {
        let mut b = UniformBackend;
        let layout = text_edit_layout(&mut b, &text_node("hello\nworld", 200.0));
        for offset in [0usize, 2, 5, 6, 9, 11] {
            let (x, y) = layout.caret_position(&mut b, offset);
            let hit = layout.offset_at_point(&mut b, Point2D::new(x + 1.0, y + 5.0));
            assert_eq!(hit, offset, "round-trip for offset {offset}");
        }
    }

    #[test]
    fn offset_at_point_clamps_outside_bounds() {
        let mut b = UniformBackend;
        let layout = text_edit_layout(&mut b, &text_node("hello\nworld", 200.0));
        // Far left of line 0 → 0; far right of line 1 → end.
        assert_eq!(layout.offset_at_point(&mut b, Point2D::new(0.0, 55.0)), 0);
        assert_eq!(
            layout.offset_at_point(&mut b, Point2D::new(900.0, 75.0)),
            11
        );
        // Above the first line / below the last line clamp vertically.
        assert_eq!(
            layout.offset_at_point(&mut b, Point2D::new(114.0, -50.0)),
            1
        );
        assert_eq!(
            layout.offset_at_point(&mut b, Point2D::new(114.0, 500.0)),
            7
        );
    }

    #[test]
    fn offset_at_point_uses_midpoint_rule() {
        let mut b = UniformBackend;
        let layout = text_edit_layout(&mut b, &text_node("ab", 200.0));
        // First glyph spans x 100..110; clicks left of 105 → 0, right → 1.
        assert_eq!(layout.offset_at_point(&mut b, Point2D::new(104.0, 55.0)), 0);
        assert_eq!(layout.offset_at_point(&mut b, Point2D::new(106.0, 55.0)), 1);
    }

    #[test]
    fn selection_rects_span_lines() {
        let mut b = UniformBackend;
        let layout = text_edit_layout(&mut b, &text_node("hello\nworld", 200.0));
        // Select bytes 3..9 → "lo" on line 0 + "wor" on line 1.
        let rects = layout.selection_rects(&mut b, 3, 9);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].origin, Point2D::new(130.0, 50.0));
        assert_eq!(rects[0].size.x, 20.0);
        assert_eq!(rects[1].origin, Point2D::new(100.0, 70.0));
        assert_eq!(rects[1].size.x, 30.0);
    }

    #[test]
    fn centered_text_offsets_line_x() {
        let mut b = UniformBackend;
        let mut node = text_node("ab", 200.0);
        node.text_align = crate::layout_scene::SceneTextAlign::Center;
        let layout = text_edit_layout(&mut b, &node);
        // Line width 20 in a 200 box → x starts at 100 + 90.
        assert_eq!(layout.caret_position(&mut b, 0).0, 190.0);
        assert_eq!(layout.offset_at_point(&mut b, Point2D::new(191.0, 55.0)), 0);
    }

    #[test]
    fn left_aligned_line_start_caret_skips_alignment_measurement() {
        let mut b = CountingBackend { measures: 0 };
        let layout = text_edit_layout(&mut b, &text_node("hello", 200.0));

        b.reset();
        assert_eq!(layout.caret_position(&mut b, 0), (100.0, 50.0));
        assert_eq!(
            b.measures, 0,
            "left-aligned line_x should not measure a line whose x is already known"
        );
    }

    #[test]
    fn offset_at_point_uses_logarithmic_prefix_measurements_on_long_lines() {
        let content = "a".repeat(96);
        let mut b = CountingBackend { measures: 0 };
        let layout = text_edit_layout(&mut b, &text_node(&content, 2_000.0));

        b.reset();
        let hit = layout.offset_at_point(&mut b, Point2D::new(904.0, 55.0));

        assert_eq!(hit, 80);
        assert!(
            b.measures <= 20,
            "long-line hit testing should avoid linear prefix measurement, got {} measures",
            b.measures
        );
    }

    #[test]
    fn empty_content_has_one_empty_line() {
        let mut b = UniformBackend;
        let layout = text_edit_layout(&mut b, &text_node("", 200.0));
        assert_eq!(layout.lines, vec![String::new()]);
        assert_eq!(layout.caret_position(&mut b, 0), (100.0, 50.0));
    }
}
