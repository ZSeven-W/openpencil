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
use crate::{Color, Point2D, Rect, RenderBackend, TextBaselineRequest, TextLayout};
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
#[cfg(test)]
pub(crate) fn slice_ranges(
    runs: &[SceneTextRun],
    line_start: usize,
    line_end: usize,
) -> Vec<(usize, usize, Option<usize>)> {
    let mut out = Vec::new();
    for_each_slice_range(runs, line_start, line_end, |start, end, run| {
        out.push((start, end, run));
    });
    out
}

fn for_each_slice_range(
    runs: &[SceneTextRun],
    line_start: usize,
    line_end: usize,
    mut f: impl FnMut(usize, usize, Option<usize>),
) {
    if line_start >= line_end {
        return;
    }
    if runs.is_empty() {
        f(line_start, line_end, None);
        return;
    }
    let mut pos = line_start;
    while pos < line_end {
        match runs.iter().position(|r| r.start <= pos && pos < r.end) {
            Some(i) => {
                let end = runs[i].end.min(line_end);
                f(pos, end, Some(i));
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
                f(pos, next, None);
                pos = next;
            }
        }
    }
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

#[allow(clippy::too_many_arguments)]
fn measure_line_slices(
    backend: &mut dyn RenderBackend,
    node: &SceneNode,
    text: &str,
    line_start: usize,
    line_end: usize,
    base_font_size: f32,
    base_weight: u16,
    ink: Color,
    family: &str,
    letter_spacing: f32,
) -> f32 {
    let mut w = 0.0;
    let mut chars = 0usize;
    for_each_slice_range(&node.text_runs, line_start, line_end, |start, end, run| {
        let style = resolved_slice_style(
            node,
            base_font_size,
            base_weight,
            ink,
            run.map(|i| &node.text_runs[i]),
        );
        let slice = &text[start..end];
        w += backend.measure_text_family_styled(
            slice,
            style.font_size,
            family,
            style.weight,
            style.italic,
        );
        chars += slice.chars().count();
    });
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
        let advance = backend.measure_text_family_styled(
            slice,
            style.font_size,
            family,
            style.weight,
            style.italic,
        );
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
        let adv = backend.measure_text_family_styled(
            s,
            style.font_size,
            family,
            style.weight,
            style.italic,
        );
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

/// Below this many device pixels of glyph height, text paints as a
/// translucent bar ("greeking") instead of running the full layout.
const GREEK_TEXT_MAX_DEVICE_PX: f32 = 3.0;

pub fn paint_text_node(
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
    // Greeking LOD: below ~3 device px of glyph height the text is an
    // unreadable smudge, but the full layout below (CJK wrap measuring,
    // per-line measures, per-run typeface segmentation) still costs the
    // same — a zoomed-out text-dense page (4k+ text nodes all visible)
    // paid full shaping on every panned frame. Paint a translucent ink
    // bar over the node bounds instead; an edited node keeps the exact
    // glyph path so caret / selection geometry stays true.
    if editing.is_none() {
        let font_size_doc = if paint_node.font_size > 0.0 {
            paint_node.font_size
        } else {
            13.0
        };
        let device_px = font_size_doc * zoom * cx.backend.dpi_scale();
        if device_px < GREEK_TEXT_MAX_DEVICE_PX {
            if !text.is_empty() {
                cx.backend
                    .fill_rect(world_rect, ink.with_alpha(ink.a * 0.18));
            }
            return;
        }
    }
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
        // Only authored line boxes opt into shaped fallback metrics.
        let baseline_offset = if paint_node.line_height > 0.0 {
            cx.backend.text_first_baseline(&TextBaselineRequest {
                text: layout.lines.first().map_or("", String::as_str),
                font_family: family,
                font_size,
                font_weight: weight,
                italic: paint_node.italic,
                line_height: paint_node.line_height,
            })
        } else {
            cx.backend.text_ascent_family(font_size, family, weight)
        };
        let first_baseline_y = world_rect.origin.y + baseline_offset;
        let last_line = layout.lines.len().saturating_sub(1);
        for (idx, line) in layout.lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            let line_start = layout.line_starts[idx];
            let line_end = line_start + line.len();
            let line_w = measure_line_slices(
                cx.backend,
                paint_node,
                text,
                line_start,
                line_end,
                font_size,
                weight,
                ink,
                family,
                letter_spacing,
            );
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
            for_each_slice_range(
                &paint_node.text_runs,
                line_start,
                line_end,
                |start, end, run| {
                    let style = resolved_slice_style(
                        paint_node,
                        font_size,
                        weight,
                        ink,
                        run.map(|i| &paint_node.text_runs[i]),
                    );
                    let slice = &text[start..end];
                    let slice_x0 = x;
                    let (next_x, glyph_end) = draw_slice(
                        cx.backend,
                        slice,
                        style,
                        family,
                        x,
                        baseline_y,
                        letter_spacing,
                        justify_extra,
                    );
                    decorate_slice(cx.backend, style, slice_x0, glyph_end, baseline_y);
                    x = next_x;
                },
            );
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
mod tests;
