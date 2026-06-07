//! Code-tab paint for the PropertyPanel. Renders the
//! `op_editor_core::codegen::CodegenState` in its three live phases
//! (Idle / Generating / Complete) plus an Error fallback. The
//! generation pipeline itself is wired later (P3); this module only
//! PAINTS the state and mirrors the existing property-panel paint
//! idioms (full-width filled action buttons, framework chips, status
//! glyphs) rather than inventing new primitives.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_inputs::{
    paint_section_label, to_jian_color, INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::codegen::{ChunkStatus, CodegenPhase, CodegenState};

/// Height of a framework chip + the gap below the chip row.
const CHIP_HEIGHT: f32 = 26.0;
const CHIP_PAD_X: f32 = 12.0;
const CHIP_GAP: f32 = 6.0;
/// Height of a progress / status row (chunk, planning, assembly).
const PROGRESS_ROW_H: f32 = 24.0;
/// Code-preview area height + the max number of lines it shows.
const CODE_AREA_H: f32 = 220.0;
const CODE_LINE_H: f32 = 16.0;
const CODE_MAX_LINES: usize = 13;

/// Draw a 13px system-ui line of text at `(px, py)` (baseline), using
/// the same `single_run` idiom the rest of the panel uses.
fn draw_line(cx: &mut PaintCx<'_>, text: &str, color: Color, px: f32, py: f32) {
    let layout = TextLayout::single_run(text, "system-ui", 13.0, to_jian_color(color), origin());
    cx.backend.draw_text(&layout, Point2D::new(px, py));
}

fn origin() -> Point2D {
    Point2D::new(0.0, 0.0)
}

/// Paint a full-width action button at `y`. `filled` → primary fill +
/// primary-foreground label; otherwise a muted outline (border stroke)
/// + foreground label. Returns the y after the button + its gap.
fn paint_full_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    x: f32,
    y: f32,
    w: f32,
    filled: bool,
) -> f32 {
    let btn = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(w - PAD_X * 2.0, INPUT_HEIGHT),
    };
    if filled {
        cx.backend.fill_round_rect(btn, INPUT_RADIUS, theme.primary);
    } else {
        cx.backend.fill_round_rect(btn, INPUT_RADIUS, theme.muted);
        cx.backend
            .stroke_round_rect(btn, INPUT_RADIUS, theme.border, 1.0);
    }
    let text_color = if filled {
        theme.primary_foreground
    } else {
        theme.foreground
    };
    let tw = cx.backend.measure_text(label, 13.0);
    draw_line(
        cx,
        label,
        text_color,
        btn.origin.x + (btn.size.x - tw) / 2.0,
        btn.origin.y + 19.0,
    );
    y + INPUT_HEIGHT + 12.0
}

/// Paint the framework tab row: one small rounded-rect chip per
/// `Framework::ALL`, wrapping to a new line when the row exceeds the
/// usable width. The active framework chip uses the primary palette;
/// inactive chips use muted. Returns the y after the (multi-row) strip.
fn paint_framework_chips(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    use op_editor_core::codegen::Framework;
    let left = x + PAD_X;
    let right = x + w - PAD_X;
    let mut chip_x = left;
    let mut chip_y = y;
    for fw in Framework::ALL {
        let label = fw.as_wire();
        let tw = cx.backend.measure_text(label, 13.0);
        let chip_w = tw + CHIP_PAD_X * 2.0;
        // Wrap to a new row when this chip would overflow the usable
        // width (but never wrap the very first chip on a row).
        if chip_x > left && chip_x + chip_w > right {
            chip_x = left;
            chip_y += CHIP_HEIGHT + CHIP_GAP;
        }
        let chip = Rect {
            origin: Point2D::new(chip_x, chip_y),
            size: Point2D::new(chip_w, CHIP_HEIGHT),
        };
        let active = state.framework == fw;
        let (fill, text_color) = if active {
            (theme.primary, theme.primary_foreground)
        } else {
            (theme.muted, theme.muted_foreground)
        };
        cx.backend.fill_round_rect(chip, 6.0, fill);
        draw_line(
            cx,
            label,
            text_color,
            chip.origin.x + CHIP_PAD_X,
            chip.origin.y + 17.0,
        );
        chip_x += chip_w + CHIP_GAP;
    }
    chip_y + CHIP_HEIGHT + SECTION_GAP
}

/// Map a `ChunkStatus` to its trailing status glyph + tint. Pending /
/// Skipped read as a dim Minus, Running as a Loader, Done as a primary
/// Check, Degraded / Failed as an AlertTriangle.
fn status_glyph(theme: &Theme, status: ChunkStatus) -> (Icon, Color) {
    match status {
        ChunkStatus::Pending | ChunkStatus::Skipped => (Icon::Minus, theme.muted_foreground),
        ChunkStatus::Running => (Icon::Loader, theme.primary),
        ChunkStatus::Done => (Icon::Check, theme.primary),
        ChunkStatus::Degraded | ChunkStatus::Failed => (Icon::AlertTriangle, theme.destructive),
    }
}

/// Paint one progress row: left label + a trailing status glyph.
// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
fn paint_progress_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    icon: Icon,
    icon_color: Color,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    draw_line(cx, label, theme.foreground, x + PAD_X, y + 16.0);
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(x + w - PAD_X - 16.0, y + 4.0),
        16.0,
        icon_color,
        1.6,
    );
    y + PROGRESS_ROW_H
}

/// Glyph for the planning / assembly rows, whose state is a tri-state
/// `Option<bool>` (None = not started/dim, Some(false) = running,
/// Some(true) = done).
fn phase_glyph(theme: &Theme, done: Option<bool>) -> (Icon, Color) {
    match done {
        None => (Icon::Minus, theme.muted_foreground),
        Some(false) => (Icon::Loader, theme.primary),
        Some(true) => (Icon::Check, theme.primary),
    }
}

/// Idle body: a hint + a primary "Generate <fw>" button + a secondary
/// "Export AI Bundle" button. Any error is surfaced above the buttons.
fn paint_idle_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    mut y: f32,
    w: f32,
) -> f32 {
    draw_icon(
        cx.backend,
        Icon::Sparkles,
        Point2D::new(x + PAD_X, y),
        20.0,
        theme.muted_foreground,
        1.6,
    );
    draw_line(
        cx,
        "Generate code for the selection",
        theme.muted_foreground,
        x + PAD_X + 28.0,
        y + 15.0,
    );
    y += 28.0 + SECTION_GAP;
    if let Some(err) = state.error.as_ref() {
        draw_line(cx, err, theme.destructive, x + PAD_X, y + 14.0);
        y += PROGRESS_ROW_H;
    }
    let generate = format!("Generate {}", state.framework.as_wire());
    y = paint_full_button(cx, theme, &generate, x, y, w, true);
    y = paint_full_button(cx, theme, "Export AI Bundle", x, y, w, false);
    y
}

/// Generating body: a header, a planning row, one row per chunk, an
/// assembly row, and a full-width Cancel button.
fn paint_generating_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    mut y: f32,
    w: f32,
) -> f32 {
    draw_line(cx, "Generating…", theme.foreground, x + PAD_X, y + 16.0);
    y += PROGRESS_ROW_H;
    let (plan_icon, plan_color) = phase_glyph(theme, state.progress.planning_done);
    y = paint_progress_row(cx, theme, "Planning", plan_icon, plan_color, x, y, w);
    for chunk in &state.progress.chunks {
        let (icon, color) = status_glyph(theme, chunk.status);
        y = paint_progress_row(cx, theme, &chunk.name, icon, color, x, y, w);
    }
    let (asm_icon, asm_color) = phase_glyph(theme, state.progress.assembly_done);
    y = paint_progress_row(cx, theme, "Assembly", asm_icon, asm_color, x, y, w);
    y += SECTION_GAP;
    paint_full_button(cx, theme, "Cancel", x, y, w, false)
}

/// Paint the bordered code-preview area + the lines of `state.code`
/// clipped to it, in a monospace face. Returns the y after the rect.
fn paint_code_area(cx: &mut PaintCx<'_>, theme: &Theme, code: &str, x: f32, y: f32, w: f32) -> f32 {
    let rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(w - PAD_X * 2.0, CODE_AREA_H),
    };
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    cx.backend
        .stroke_round_rect(rect, INPUT_RADIUS, theme.border, 1.0);
    // The backend's clip stack is stateful (skia canvas) — save /
    // restore around the clipped text so the clip doesn't leak into
    // the buttons painted below.
    cx.backend.save();
    cx.backend.clip_rect(rect);
    let mut line_y = rect.origin.y + 16.0;
    // PARITY TODO: scroll + syntax highlight — v1 simply clips overflow.
    for line in code.split('\n').take(CODE_MAX_LINES) {
        let layout = TextLayout::single_run(
            line,
            "monospace",
            12.0,
            to_jian_color(theme.foreground),
            origin(),
        );
        cx.backend
            .draw_text(&layout, Point2D::new(rect.origin.x + 10.0, line_y));
        line_y += CODE_LINE_H;
    }
    cx.backend.restore();
    y + CODE_AREA_H + 12.0
}

/// Paint a small icon + label button at `rect`, used for the Complete
/// phase's action row (Copy / Download / AI Bundle / Regenerate).
fn paint_action_chip(cx: &mut PaintCx<'_>, theme: &Theme, icon: Icon, label: &str, rect: Rect) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    cx.backend
        .stroke_round_rect(rect, INPUT_RADIUS, theme.border, 1.0);
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 8.0, rect.origin.y + 7.0),
        14.0,
        theme.foreground,
        1.4,
    );
    draw_line(
        cx,
        label,
        theme.foreground,
        rect.origin.x + 26.0,
        rect.origin.y + 19.0,
    );
}

/// Complete body: optional degraded warning + asset notice, the code
/// preview area, and an action row (Copy / Download / AI Bundle /
/// Regenerate).
fn paint_complete_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    mut y: f32,
    w: f32,
) -> f32 {
    if state.degraded {
        draw_icon(
            cx.backend,
            Icon::AlertTriangle,
            Point2D::new(x + PAD_X, y + 4.0),
            16.0,
            theme.muted_foreground,
            1.6,
        );
        draw_line(
            cx,
            "Generated with degraded chunks",
            theme.muted_foreground,
            x + PAD_X + 22.0,
            y + 16.0,
        );
        y += PROGRESS_ROW_H;
    }
    if !state.assets.is_empty() {
        let notice = format!("Includes {} asset(s)", state.assets.len());
        draw_line(cx, &notice, theme.muted_foreground, x + PAD_X, y + 16.0);
        y += PROGRESS_ROW_H;
    }
    y = paint_code_area(cx, theme, &state.code, x, y, w);
    // Action row: 4 small buttons spread across the usable width.
    let usable = w - PAD_X * 2.0;
    let gap = 8.0;
    let btn_w = (usable - gap * 3.0) / 4.0;
    let actions = [
        (Icon::Copy, "Copy"),
        (Icon::Download, "Save"),
        (Icon::Sparkles, "Bundle"),
        (Icon::RefreshCw, "Regen"),
    ];
    for (i, (icon, label)) in actions.iter().enumerate() {
        let rect = Rect {
            origin: Point2D::new(x + PAD_X + (btn_w + gap) * i as f32, y),
            size: Point2D::new(btn_w, INPUT_HEIGHT),
        };
        paint_action_chip(cx, theme, *icon, label, rect);
    }
    y + INPUT_HEIGHT + 12.0
}

/// Error body: the error text in the destructive color + a full-width
/// Regenerate button.
fn paint_error_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    mut y: f32,
    w: f32,
) -> f32 {
    let msg = state.error.as_deref().unwrap_or("Generation failed");
    draw_line(cx, msg, theme.destructive, x + PAD_X, y + 16.0);
    y += PROGRESS_ROW_H + SECTION_GAP;
    paint_full_button(cx, theme, "Regenerate", x, y, w, true)
}

/// Paint the full Code panel from `state`. Layout: a framework chip
/// row, then a phase-specific body. Returns the y after the painted
/// content.
pub fn paint_code_panel(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    // A faint section label keeps the panel head consistent with the
    // Design tab's section headers.
    let mut y = paint_section_label(cx, theme, "Code", x, y, w);
    y = paint_framework_chips(cx, theme, state, x, y, w);
    match state.phase {
        CodegenPhase::Idle => paint_idle_body(cx, theme, state, x, y, w),
        CodegenPhase::Generating => paint_generating_body(cx, theme, state, x, y, w),
        CodegenPhase::Complete => paint_complete_body(cx, theme, state, x, y, w),
        CodegenPhase::Error => paint_error_body(cx, theme, state, x, y, w),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::PaintCx;
    use op_editor_core::codegen::{
        AssetMeta, ChunkProgress, ChunkStatus, CodeGenProgress, Framework,
    };

    /// No-op `RenderBackend` for paint smoke tests — counts nothing,
    /// just satisfies the trait so `paint_code_panel` can run headless.
    #[derive(Default)]
    struct StubBackend;
    impl crate::RenderBackend for StubBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn paint(state: &CodegenState) -> f32 {
        let mut backend = StubBackend;
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_code_panel(&mut cx, &Theme::dark(), state, 0.0, 0.0, 300.0)
    }

    #[test]
    fn paint_code_panel_does_not_panic_each_phase() {
        // Idle.
        let idle = CodegenState::default();
        assert!(paint(&idle) > 0.0);

        // Generating with 2 chunks (one running, one pending).
        let generating = CodegenState {
            phase: CodegenPhase::Generating,
            framework: Framework::Vue,
            progress: CodeGenProgress {
                planning_done: Some(true),
                chunks: vec![
                    ChunkProgress {
                        chunk_id: "a".into(),
                        name: "Header".into(),
                        status: ChunkStatus::Running,
                    },
                    ChunkProgress {
                        chunk_id: "b".into(),
                        name: "Footer".into(),
                        status: ChunkStatus::Pending,
                    },
                ],
                assembly_done: None,
            },
            ..CodegenState::default()
        };
        assert!(paint(&generating) > 0.0);

        // Complete with code + one asset, degraded.
        let complete = CodegenState {
            phase: CodegenPhase::Complete,
            code: "fn main() {\n    println!(\"hi\");\n}\n".into(),
            degraded: true,
            assets: vec![AssetMeta {
                relative_path: "logo.svg".into(),
                byte_len: 42,
            }],
            ..CodegenState::default()
        };
        assert!(paint(&complete) > 0.0);

        // Error.
        let error = CodegenState {
            phase: CodegenPhase::Error,
            error: Some("model timeout".into()),
            ..CodegenState::default()
        };
        assert!(paint(&error) > 0.0);
    }
}
