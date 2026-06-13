//! Text-node painter for the canvas — split out of
//! `canvas_viewport_paint.rs` (800-line cap) when styled text runs
//! landed.
//!
//! Line layout (wrap, line starts, caret/selection geometry) stays on
//! the FLAT string via `canvas_text_edit::text_edit_layout`, exactly
//! like the inline text editor — editing styled text falls back to
//! flat (single-style) editing. Styled rendering maps each painted
//! line back onto the node's `text_runs` byte ranges, so every painted
//! slice carries its segment's font size / weight / fill / italic /
//! underline / strikethrough.
//!
//! v1 approximations (documented, TS parity follow-ups):
//! - WRAP positions and the caret/selection layout are computed with
//!   the node-level font size + weight even when runs override them;
//!   a wrapped line that splits mid-segment paints each slice with its
//!   own style but breaks where the flat-string wrap decided.
//! - All slices on a line share the node's baseline; a run with a
//!   larger `font_size` sits on that same baseline.
//! - Justify distributes the residual width across ASCII-space gaps on
//!   every line except the last (TS `ck.TextAlign.Justify` behaviour);
//!   gap-free (e.g. CJK) lines are left-aligned.

use crate::layout_scene::{SceneNode, SceneTextAlign, SceneTextRun};
use crate::widgets::canvas_text_edit::text_edit_layout;
use crate::widgets::canvas_viewport::EditCaret;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use jian_core::text_input::prev_char_boundary;

/// Fully resolved paint style for one line slice.
#[derive(Clone, Copy)]
struct SliceStyle {
    font_size: f32,
    weight: u16,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color: Color,
}

/// Map a painted line's byte range `[line_start, line_end)` onto run
/// ranges. Returns `(slice_start, slice_end, run_index)` triples in
/// flat-text byte coords; `None` run index = node-level style (gaps,
/// or the whole line when the node has no runs). Pure — unit-tested.
pub(crate) fn slice_ranges(
    runs: &[SceneTextRun],
    line_start: usize,
    line_end: usize,
) -> Vec<(usize, usize, Option<usize>)> {
    if line_start >= line_end {
        return Vec::new();
    }
    if runs.is_empty() {
        return vec![(line_start, line_end, None)];
    }
    let mut out = Vec::new();
    let mut pos = line_start;
    while pos < line_end {
        match runs.iter().position(|r| r.start <= pos && pos < r.end) {
            Some(i) => {
                let end = runs[i].end.min(line_end);
                out.push((pos, end, Some(i)));
                pos = end;
            }
            None => {
                // Gap not covered by any run — node-level style up to
                // the next run start (or the line end).
                let next = runs
                    .iter()
                    .map(|r| r.start)
                    .filter(|s| *s > pos)
                    .min()
                    .unwrap_or(line_end)
                    .min(line_end);
                out.push((pos, next, None));
                pos = next;
            }
        }
    }
    out
}

/// Residual-width share each ASCII-space gap receives when a line is
/// justified. `0.0` when there is nothing to distribute or no gaps
/// (gap-free lines stay left-aligned). Pure — unit-tested.
pub(crate) fn justify_extra_per_gap(residual: f32, line: &str) -> f32 {
    if residual <= 0.0 {
        return 0.0;
    }
    let gaps = line.chars().filter(|c| *c == ' ').count();
    if gaps == 0 {
        0.0
    } else {
        residual / gaps as f32
    }
}

fn resolved_slice_style(
    node: &SceneNode,
    base_font_size: f32,
    base_weight: u16,
    ink: Color,
    run: Option<&SceneTextRun>,
) -> SliceStyle {
    match run {
        None => SliceStyle {
            font_size: base_font_size,
            weight: base_weight,
            italic: node.italic,
            underline: node.underline,
            strikethrough: node.strikethrough,
            color: ink,
        },
        Some(r) => SliceStyle {
            font_size: if r.font_size > 0.0 {
                r.font_size
            } else {
                base_font_size
            },
            weight: if r.font_weight > 0 {
                r.font_weight
            } else {
                base_weight
            },
            italic: r.italic,
            underline: r.underline,
            strikethrough: r.strikethrough,
            color: r.fill.unwrap_or(ink),
        },
    }
}

/// Painted width of a line's slices — per-slice styled measure plus
/// letter-spacing between glyphs (mirrors the flat
/// `measure_line_width` model when the node has no runs).
fn measure_slices(
    backend: &mut dyn RenderBackend,
    text: &str,
    slices: &[(usize, usize, Option<usize>)],
    styles: &[SliceStyle],
    letter_spacing: f32,
) -> f32 {
    let mut w = 0.0;
    let mut chars = 0usize;
    for ((start, end, _), style) in slices.iter().zip(styles) {
        let slice = &text[*start..*end];
        w += backend.measure_text_styled(slice, style.font_size, style.weight, style.italic);
        chars += slice.chars().count();
    }
    w + chars.saturating_sub(1) as f32 * letter_spacing
}

#[allow(clippy::too_many_arguments)]
fn draw_slice(
    backend: &mut dyn RenderBackend,
    slice: &str,
    style: SliceStyle,
    family: &str,
    x: f32,
    baseline_y: f32,
    letter_spacing: f32,
    justify_extra: f32,
) -> (f32, f32) {
    let jc = (style.color).to_jian();
    if letter_spacing.abs() < f32::EPSILON && justify_extra <= 0.0 {
        backend.draw_text(
            &TextLayout::single_run(slice, family, style.font_size, jc, Point2D::ZERO)
                .with_font_weight(style.weight)
                .with_italic(style.italic),
            Point2D::new(x, baseline_y),
        );
        let advance =
            backend.measure_text_styled(slice, style.font_size, style.weight, style.italic);
        return (x + advance, x + advance);
    }
    // Per-char path — letter spacing after every glyph (legacy painter
    // model) plus the justify share after each space.
    let mut cursor = x;
    let mut glyph_end = x;
    for ch in slice.chars() {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        backend.draw_text(
            &TextLayout::single_run(s, family, style.font_size, jc, Point2D::ZERO)
                .with_font_weight(style.weight)
                .with_italic(style.italic),
            Point2D::new(cursor, baseline_y),
        );
        let adv = backend.measure_text_styled(s, style.font_size, style.weight, style.italic);
        glyph_end = cursor + adv;
        cursor = glyph_end + letter_spacing;
        if ch == ' ' {
            cursor += justify_extra;
        }
    }
    (cursor, glyph_end)
}

/// Underline / strikethrough decoration for one painted slice.
/// Offsets follow common raster-text metrics: underline slightly
/// below the baseline, strikethrough near half the x-height above it.
fn decorate_slice(
    backend: &mut dyn RenderBackend,
    style: SliceStyle,
    x0: f32,
    x1: f32,
    baseline_y: f32,
) {
    if (!style.underline && !style.strikethrough) || x1 <= x0 {
        return;
    }
    let thickness = (style.font_size * 0.07).max(1.0);
    if style.underline {
        let y = baseline_y + style.font_size * 0.12;
        backend.stroke_line(
            Point2D::new(x0, y),
            Point2D::new(x1, y),
            style.color,
            thickness,
        );
    }
    if style.strikethrough {
        let y = baseline_y - style.font_size * 0.3;
        backend.stroke_line(
            Point2D::new(x0, y),
            Point2D::new(x1, y),
            style.color,
            thickness,
        );
    }
}

pub(crate) fn paint_text_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    edit_caret: &Option<EditCaret>,
) {
    let editing = edit_caret.as_ref().filter(|c| c.editing == node.id);
    let mut edit_node = None;
    let mut composition_range = None;
    if let Some(c) = editing {
        let base = c.input.text();
        let mut rendered = base.to_owned();
        if let Some(composition) = c.input.composition() {
            if !composition.text.is_empty() {
                let insert_at = prev_char_boundary(base, c.input.caret().min(base.len()));
                rendered.insert_str(insert_at, &composition.text);
                let cursor = prev_char_boundary(&composition.text, composition.cursor);
                composition_range = Some((
                    insert_at,
                    insert_at + composition.text.len(),
                    insert_at + cursor,
                ));
            }
        }
        let mut clone = node.clone();
        clone.text = Some(rendered);
        clone.text_runs.clear();
        edit_node = Some(clone);
    }
    let paint_node = edit_node.as_ref().unwrap_or(node);
    let text = paint_node.text.as_deref().unwrap_or("");
    // Ink colour follows the resolved fill (defaults to near black).
    let ink = node.fill.unwrap_or(Color {
        r: 0.08,
        g: 0.08,
        b: 0.08,
        a: 1.0,
    });
    let family = if paint_node.font_family.trim().is_empty() {
        "system-ui"
    } else {
        paint_node.font_family.as_str()
    };
    // Shared line layout — the same resolution the inline text editor
    // hit-tests against (`canvas_text_edit`), so caret / selection /
    // click-to-caret geometry always matches the painted glyphs.
    // Wrapping is authored DOCUMENT geometry, not viewport geometry:
    // measuring at the zoomed font size could shift CJK line breaks,
    // so the layout works in doc space and the caller applies the
    // viewport scale as a canvas transform. The layout (and therefore
    // the text editor) works on the FLAT string with node-level
    // metrics; styled runs only restyle the painted slices.
    let layout = text_edit_layout(cx.backend, paint_node);
    let font_size = layout.font_size;
    let weight = layout.weight;
    let letter_spacing = layout.letter_spacing;
    // Selection wash behind the glyphs: Cmd/Ctrl+A's whole-content
    // wash and a partial anchor..caret drag share the same painter.
    let selection = editing.and_then(|c| {
        composition_range.is_none().then(|| {
            c.input
                .highlight_range()
                .map(|(start, end)| (start.min(text.len()), end.min(text.len())))
                .filter(|(start, end)| start != end)
        })?
    });
    if let (Some(c), Some((sel_start, sel_end))) = (editing, selection) {
        for hl in layout.selection_rects(cx.backend, sel_start, sel_end) {
            cx.backend.fill_round_rect(hl, 2.0, c.selection_color);
        }
    }
    if !text.is_empty() {
        // TS `pen-renderer` draws text at the node's authored top-left
        // and does not apply `textAlignVertical` during paint. Figma
        // exports already bake vertical placement into `x/y`; applying
        // middle/bottom again shifts imported labels away from their
        // TS positions.
        let first_baseline_y = world_rect.origin.y + font_size;
        let last_line = layout.lines.len().saturating_sub(1);
        for (idx, line) in layout.lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            let line_start = layout.line_starts[idx];
            let line_end = line_start + line.len();
            let slices = slice_ranges(&paint_node.text_runs, line_start, line_end);
            let styles: Vec<SliceStyle> = slices
                .iter()
                .map(|(_, _, run)| {
                    resolved_slice_style(
                        paint_node,
                        font_size,
                        weight,
                        ink,
                        run.map(|i| &paint_node.text_runs[i]),
                    )
                })
                .collect();
            let line_w = measure_slices(cx.backend, text, &slices, &styles, letter_spacing);
            let x0 = match paint_node.text_align {
                SceneTextAlign::Center => {
                    world_rect.origin.x + (layout.align_width - line_w).max(0.0) / 2.0
                }
                SceneTextAlign::Right => {
                    world_rect.origin.x + (layout.align_width - line_w).max(0.0)
                }
                SceneTextAlign::Left | SceneTextAlign::Justify => world_rect.origin.x,
            };
            // Justify spreads the residual width across space gaps on
            // every line except the last (TS Justify parity).
            let justify_extra =
                if paint_node.text_align == SceneTextAlign::Justify && idx != last_line {
                    justify_extra_per_gap((layout.align_width - line_w).max(0.0), line)
                } else {
                    0.0
                };
            let baseline_y = first_baseline_y + idx as f32 * layout.line_h;
            let mut x = x0;
            for ((start, end, _), style) in slices.iter().zip(&styles) {
                let slice = &text[*start..*end];
                let slice_x0 = x;
                let (next_x, glyph_end) = draw_slice(
                    cx.backend,
                    slice,
                    *style,
                    family,
                    x,
                    baseline_y,
                    letter_spacing,
                    justify_extra,
                );
                decorate_slice(cx.backend, *style, slice_x0, glyph_end, baseline_y);
                x = next_x;
            }
        }
    }
    if let Some((start, end, _)) = composition_range {
        let thickness = (1.0 / zoom).max(1.0);
        for rect in layout.selection_rects(cx.backend, start, end) {
            let y = rect.origin.y + font_size * 1.12;
            cx.backend.stroke_line(
                Point2D::new(rect.origin.x, y),
                Point2D::new(rect.origin.x + rect.size.x, y),
                ink,
                thickness,
            );
        }
    }
    // Caret while editing — at the real caret offset (hidden while a
    // selection is active, textarea-style).
    if let Some(c) = editing {
        if selection.is_none() && c.input.caret_visible(c.now_ms) {
            let caret_byte = composition_range
                .map(|(_, _, cursor)| cursor.min(text.len()))
                .unwrap_or_else(|| c.input.caret().min(text.len()));
            let (caret_x, line_top) = layout.caret_position(cx.backend, caret_byte);
            let caret = Rect {
                origin: Point2D::new(caret_x, line_top + 2.0),
                size: Point2D::new((1.0 / zoom).max(1.0), font_size * 1.15),
            };
            cx.backend.fill_rect(caret, ink);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_scene::NodeKind;
    use crate::widgets::PaintCx;

    fn run(start: usize, end: usize) -> SceneTextRun {
        SceneTextRun {
            start,
            end,
            font_size: 0.0,
            font_weight: 0,
            fill: None,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }

    #[test]
    fn slice_ranges_without_runs_is_one_default_slice() {
        assert_eq!(slice_ranges(&[], 3, 9), vec![(3, 9, None)]);
        assert!(slice_ranges(&[], 5, 5).is_empty());
    }

    #[test]
    fn slice_ranges_splits_a_line_at_run_boundaries() {
        // Runs: [0,5) [5,11). Line covers [3,9) → two slices.
        let runs = [run(0, 5), run(5, 11)];
        assert_eq!(
            slice_ranges(&runs, 3, 9),
            vec![(3, 5, Some(0)), (5, 9, Some(1))]
        );
    }

    #[test]
    fn slice_ranges_line_inside_one_run_is_one_slice() {
        // A wrapped mid-segment line stays a single styled slice.
        let runs = [run(0, 20)];
        assert_eq!(slice_ranges(&runs, 6, 13), vec![(6, 13, Some(0))]);
    }

    #[test]
    fn slice_ranges_covers_gaps_with_node_style() {
        // Run only over [4,8); line [0,12) → gap, run, gap.
        let runs = [run(4, 8)];
        assert_eq!(
            slice_ranges(&runs, 0, 12),
            vec![(0, 4, None), (4, 8, Some(0)), (8, 12, None)]
        );
    }

    #[test]
    fn justify_spreads_residual_across_space_gaps() {
        // "ab cd ef" has 2 spaces → 12 / 2 = 6 per gap.
        assert_eq!(justify_extra_per_gap(12.0, "ab cd ef"), 6.0);
        // No gaps (CJK / single word) → no distribution.
        assert_eq!(justify_extra_per_gap(12.0, "汉字汉字"), 0.0);
        // Nothing to distribute.
        assert_eq!(justify_extra_per_gap(0.0, "ab cd"), 0.0);
        assert_eq!(justify_extra_per_gap(-3.0, "ab cd"), 0.0);
    }

    #[derive(Default)]
    struct CaptureBackend {
        origins: Vec<Point2D>,
        contents: Vec<String>,
        weights: Vec<u16>,
        italics: Vec<bool>,
        lines: Vec<(Point2D, Point2D, Color, f32)>,
        fill_rects: Vec<Rect>,
        round_rects: Vec<Rect>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, rect: Rect, _: Color) {
            self.fill_rects.push(rect);
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
            self.origins.push(origin);
            self.italics.push(layout.italic());
            if let Some(run) = layout.runs().first() {
                self.contents.push(run.content.clone());
                self.weights.push(run.font_weight);
            }
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
            self.lines.push((from, to, color, width));
        }
        fn fill_round_rect(&mut self, rect: Rect, _: f32, _: Color) {
            self.round_rects.push(rect);
        }
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

    fn text_node(content: &str) -> SceneNode {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 200.0, 80.0);
        node.text = Some(content.to_string());
        node.font_size = 20.0;
        node.line_height = 1.0;
        node
    }

    #[test]
    fn composition_paints_inline_preedit_underline_and_caret() {
        let node = text_node("ab");
        let mut input = jian_core::text_input::TextInputState::with_text("ab");
        input.set_caret(1, 0);
        input.set_composition("你", "你".len(), 0);
        let edit = EditCaret {
            editing: "t".to_string(),
            input,
            now_ms: 0,
            selection_color: Color::BLUE,
        };
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &Some(edit));

        assert_eq!(backend.contents, vec!["a你b".to_string()]);
        assert_eq!(backend.lines.len(), 1);
        let (from, to, _, _) = backend.lines[0];
        assert_eq!((from.x, to.x), (10.0, 20.0));
        assert_eq!((from.y, to.y), (22.4, 22.4));
        assert_eq!(backend.fill_rects, vec![Rect::xywh(20.0, 2.0, 1.0, 23.0)]);
    }

    #[test]
    fn styled_runs_paint_separate_slices_with_run_weight() {
        let mut node = text_node("boldplain");
        node.text_runs = vec![
            SceneTextRun {
                font_weight: 700,
                ..run(0, 4)
            },
            run(4, 9),
        ];
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &None);

        assert_eq!(
            backend.contents,
            vec!["bold".to_string(), "plain".to_string()]
        );
        assert_eq!(backend.weights, vec![700, 400]);
        // Second slice starts after the first's 4-glyph advance.
        assert_eq!(backend.origins[1].x, 40.0);
    }

    #[test]
    fn italic_run_sets_layout_italic_flag() {
        let mut node = text_node("ab");
        node.text_runs = vec![SceneTextRun {
            italic: true,
            ..run(0, 2)
        }];
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &None);

        assert_eq!(backend.italics, vec![true]);
    }

    #[test]
    fn underline_and_strikethrough_stroke_at_metrics_offsets() {
        let mut node = text_node("abc");
        node.text_runs = vec![SceneTextRun {
            underline: true,
            strikethrough: true,
            ..run(0, 3)
        }];
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &None);

        // Baseline = top + font_size = 20; underline at +2.4, strike at -6.
        assert_eq!(backend.lines.len(), 2);
        let (u_from, u_to, _, _) = backend.lines[0];
        assert_eq!((u_from.y, u_to.y), (22.4, 22.4));
        assert_eq!((u_from.x, u_to.x), (0.0, 30.0));
        let (s_from, _, _, _) = backend.lines[1];
        assert_eq!(s_from.y, 14.0);
    }

    #[test]
    fn justify_paints_per_char_with_spread_gaps_except_last_line() {
        let mut node = text_node("aa bb\ncc");
        node.text_align = SceneTextAlign::Justify;
        // align_width = 200; line 0 "aa bb" = 50 px → residual 150
        // over 1 gap. Line 1 is the last → left-aligned whole-slice.
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &None);

        // Line 0 paints per-char (5 chars), line 1 as one slice.
        assert_eq!(backend.contents.len(), 6);
        // 'b' after the spread space: chars a(0) a(10) ' '(20) → next
        // starts at 30 + 150 = 180.
        assert_eq!(backend.origins[3].x, 180.0);
        assert_eq!(backend.origins[4].x, 190.0);
        // Last line unjustified, starts at the left edge.
        assert_eq!(backend.origins[5].x, 0.0);
        assert_eq!(backend.contents[5], "cc");
    }

    #[test]
    fn segment_boundary_maps_onto_wrapped_lines() {
        // 11-char content wraps at ~6 chars (60 px + tolerance);
        // runs split at byte 8 — the SECOND wrapped line must split
        // into two styled slices at the run boundary.
        let mut node = text_node("hello world");
        node.bounds = Rect::xywh(0.0, 0.0, 60.0, 80.0);
        node.text_wrap = true;
        node.text_runs = vec![
            run(0, 8),
            SceneTextRun {
                font_weight: 700,
                ..run(8, 11)
            },
        ];
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &None);

        // Wrap → "hello " + "world"; line 1 covers bytes 6..11 and
        // crosses the run boundary at 8.
        assert_eq!(
            backend.contents,
            vec!["hello ".to_string(), "wo".to_string(), "rld".to_string()]
        );
        assert_eq!(backend.weights, vec![400, 400, 700]);
        // "rld" starts after "wo"'s 2-glyph advance on line 1.
        assert_eq!(backend.origins[2].x, 20.0);
    }
}
