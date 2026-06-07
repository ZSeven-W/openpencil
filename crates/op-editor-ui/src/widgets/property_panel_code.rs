//! Code-tab paint for the PropertyPanel. Renders the
//! `op_editor_core::codegen::CodegenState` in its three live phases
//! (Idle / Generating / Complete) plus an Error fallback. The generation
//! pipeline is wired later (P3); this module only PAINTS the state. The
//! Idle empty state is a centered badge + title + buttons (TS parity);
//! the other phases reuse full-width action buttons + status glyphs.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_action::{CodegenAction, PropertyPanelAction};
use crate::widgets::property_panel_inputs::{
    paint_section_label, to_jian_color, INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP,
    SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::codegen::{ChunkStatus, CodegenPhase, CodegenState, Framework};

/// Map a click on the Code tab to its action (framework chip / button), or
/// `None`. Uses the same content origin the painter does (panel left,
/// pinned tab-strip bottom, panel width).
pub fn code_action_hit(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
) -> Option<PropertyPanelAction> {
    let cy0 = panel_rect.origin.y + TAB_HEIGHT;
    let inside = |r: Rect| {
        point.x >= r.origin.x
            && point.x <= r.origin.x + r.size.x
            && point.y >= r.origin.y
            && point.y <= r.origin.y + r.size.y
    };
    code_action_rects(panel_rect.origin.x, cy0, panel_rect.size.x, state)
        .into_iter()
        .find(|(_, r)| inside(*r))
        .map(|(a, _)| PropertyPanelAction::Codegen(a))
}

/// Height of a framework chip + the gap below the chip row.
const CHIP_HEIGHT: f32 = 26.0;
const CHIP_PAD_X: f32 = 9.0;
const CHIP_GAP: f32 = 2.0;
/// Width of each scroll-chevron zone reserved at the strip ends when the
/// framework row overflows. Paint + hit-test inset the chip clip band by
/// this on each side so chips never paint under the chevrons.
const CHEVRON_ZONE_W: f32 = 18.0;
/// Extra space below the chip row for the divider line that separates the
/// framework selector from the phase body (TS parity).
const CHIP_DIVIDER_GAP: f32 = 8.0;
/// Centered Idle empty-state metrics (TS parity).
const BADGE_SIZE: f32 = 56.0;
const IDLE_TOP_PAD: f32 = 28.0;
const IDLE_BTN_H: f32 = 40.0;
const IDLE_BTN_W_MAX: f32 = 220.0;
/// Height of a progress / status row (chunk, planning, assembly).
const PROGRESS_ROW_H: f32 = 24.0;
/// Code-preview area height + the max number of lines it shows.
const CODE_AREA_H: f32 = 220.0;
const CODE_LINE_H: f32 = 16.0;
const CODE_MAX_LINES: usize = 13;

/// Draw a 13px system-ui line of text at the `(px, py)` baseline.
fn draw_line(cx: &mut PaintCx<'_>, text: &str, color: Color, px: f32, py: f32) {
    let layout = TextLayout::single_run(text, "system-ui", 13.0, to_jian_color(color), origin());
    cx.backend.draw_text(&layout, Point2D::new(px, py));
}

fn origin() -> Point2D {
    Point2D::new(0.0, 0.0)
}

/// Geometry for a full-width action button at `y`. Painter + hit-test.
fn full_button_rect(x: f32, y: f32, w: f32) -> Rect {
    Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(w - PAD_X * 2.0, INPUT_HEIGHT),
    }
}

/// Paint a full-width action button at `y`. `filled` → primary fill +
/// primary-foreground label; else a muted outline + foreground label.
/// Returns the y after the button + its trailing gap.
fn paint_full_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label: &str,
    x: f32,
    y: f32,
    w: f32,
    filled: bool,
) -> f32 {
    let btn = full_button_rect(x, y, w);
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
    let tx = btn.origin.x + (btn.size.x - tw) / 2.0;
    draw_line(cx, label, text_color, tx, btn.origin.y + 19.0);
    y + INPUT_HEIGHT + 12.0
}

/// Backend-free advance estimate for a chip label at 13px, matching the
/// `RenderBackend::measure_text` trait-default heuristic (0.55 × size per
/// ASCII char) so painter + hit-test wrap identically. Wire tokens are
/// all ASCII → exact here.
fn chip_label_width(label: &str) -> f32 {
    label.chars().fold(0.0, |w, c| {
        w + if c.is_ascii() { 13.0 * 0.55 } else { 13.0 }
    })
}

/// Total width of the single-row framework strip (all chips + inter-gaps).
fn framework_row_width() -> f32 {
    let mut total = 0.0;
    for (i, fw) in Framework::ALL.iter().enumerate() {
        if i > 0 {
            total += CHIP_GAP;
        }
        total += chip_label_width(fw.as_wire()) + CHIP_PAD_X * 2.0;
    }
    total
}

/// How far the framework strip can scroll before its right edge reaches the
/// usable width (0 when every chip already fits). The host clamps
/// `framework_scroll` to `[0, this]`.
pub fn framework_row_overflow(w: f32) -> f32 {
    (framework_row_width() - (w - PAD_X * 2.0)).max(0.0)
}

/// The y-band `[top, bottom]` of the framework tab strip, given the panel's
/// top-left y. The host routes a wheel over this band into the horizontal
/// `framework_scroll` instead of the panel's vertical scroll.
pub fn framework_row_band(panel_top: f32) -> (f32, f32) {
    let top = panel_top + TAB_HEIGHT + SECTION_HEADER_HEIGHT;
    (top, top + CHIP_HEIGHT)
}

/// Pure-geometry walker for the single-row framework strip — one
/// `(Framework, Rect)` per `Framework::ALL`, laid left→right and shifted by
/// `-scroll`. Chips may fall partly/fully outside the visible band (the
/// painter clips; off-band chips take no in-panel clicks). Paint + hit-test
/// share this so they can't drift.
fn framework_chip_rects(x: f32, y: f32, scroll: f32) -> Vec<(Framework, Rect)> {
    let mut chip_x = x + PAD_X - scroll;
    let mut out = Vec::with_capacity(Framework::ALL.len());
    for fw in Framework::ALL {
        let chip_w = chip_label_width(fw.as_wire()) + CHIP_PAD_X * 2.0;
        out.push((
            fw,
            Rect {
                origin: Point2D::new(chip_x, y),
                size: Point2D::new(chip_w, CHIP_HEIGHT),
            },
        ));
        chip_x += chip_w + CHIP_GAP;
    }
    out
}

/// The y after the single-row chip strip + divider + trailing gap — the
/// start of the phase-specific body.
fn chips_body_top(y: f32) -> f32 {
    y + CHIP_HEIGHT + CHIP_DIVIDER_GAP + SECTION_GAP
}

/// The left + right scroll-chevron zone rects when the strip overflows the
/// usable width, else `None`. Anchored to the padded band ends at chip y.
/// Shared by paint + hit-test so the clickable zones match the painted
/// chevrons exactly.
fn framework_chevron_zones(x: f32, y: f32, w: f32) -> Option<(Rect, Rect)> {
    if framework_row_overflow(w) <= 0.0 {
        return None;
    }
    let band_l = x + PAD_X;
    let band_r = x + w - PAD_X;
    let left = Rect {
        origin: Point2D::new(band_l, y),
        size: Point2D::new(CHEVRON_ZONE_W, CHIP_HEIGHT),
    };
    let right = Rect {
        origin: Point2D::new(band_r - CHEVRON_ZONE_W, y),
        size: Point2D::new(CHEVRON_ZONE_W, CHIP_HEIGHT),
    };
    Some((left, right))
}

/// The framework whose visible (scrolled, band-clipped) chip rect contains
/// `point`, or `None`. Shares `framework_chip_rects` so it agrees with both
/// paint + click hit-test. Chips fully under a chevron zone (when the strip
/// overflows) are excluded so a hover doesn't leak behind the arrows.
pub fn framework_at(x: f32, y: f32, w: f32, point: Point2D, scroll: f32) -> Option<Framework> {
    let usable = (w - PAD_X * 2.0).max(0.0);
    let overflow = framework_row_overflow(w) > 0.0;
    let inset = if overflow { CHEVRON_ZONE_W } else { 0.0 };
    let band_l = x + PAD_X + inset;
    let band_r = x + PAD_X + usable - inset;
    framework_chip_rects(x, y, scroll)
        .into_iter()
        .filter(|(_, r)| r.origin.x + r.size.x > band_l && r.origin.x < band_r)
        .find(|(_, r)| {
            point.x >= r.origin.x.max(band_l)
                && point.x <= (r.origin.x + r.size.x).min(band_r)
                && point.y >= r.origin.y
                && point.y <= r.origin.y + r.size.y
        })
        .map(|(fw, _)| fw)
}

/// Paint a single scroll chevron centred in `zone` over a subtle muted
/// background. `enabled` controls the glyph tint (dimmed at a scroll end).
fn paint_chevron(cx: &mut PaintCx<'_>, theme: &Theme, icon: Icon, zone: Rect, enabled: bool) {
    cx.backend.fill_round_rect(zone, 6.0, theme.muted);
    let glyph = 14.0;
    let color = if enabled {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            zone.origin.x + (zone.size.x - glyph) / 2.0,
            zone.origin.y + (zone.size.y - glyph) / 2.0,
        ),
        glyph,
        color,
        1.6,
    );
}

/// Paint the framework tab row (active = blue filled pill, inactive =
/// plain text) + a divider below it. Returns the y after the strip.
fn paint_framework_chips(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let usable = (w - PAD_X * 2.0).max(0.0);
    // When the strip overflows, scroll chevrons occupy a zone at each end;
    // inset the chip clip band so chips never paint under them.
    let zones = framework_chevron_zones(x, y, w);
    let inset = if zones.is_some() { CHEVRON_ZONE_W } else { 0.0 };
    let band = Rect {
        origin: Point2D::new(x + PAD_X + inset, y),
        size: Point2D::new((usable - inset * 2.0).max(0.0), CHIP_HEIGHT),
    };
    // Clip the strip so scrolled-off chips don't bleed past the panel edges.
    cx.backend.save();
    cx.backend.clip_rect(band);
    for (fw, chip) in framework_chip_rects(x, y, state.framework_scroll) {
        // Skip chips fully outside the visible band (cheap cull).
        if chip.origin.x + chip.size.x < band.origin.x
            || chip.origin.x > band.origin.x + band.size.x
        {
            continue;
        }
        let active = state.framework == fw;
        // Active = blue filled pill; inactive = plain text, with a subtle
        // muted wash when hovered.
        let text_color = if active {
            cx.backend.fill_round_rect(chip, 6.0, theme.primary);
            theme.primary_foreground
        } else {
            if state.framework_hover == Some(fw) {
                cx.backend.fill_round_rect(chip, 6.0, theme.muted);
            }
            theme.muted_foreground
        };
        // Centre the label horizontally + vertically in the chip using the
        // real measured advance (13px baseline ≈ vertical centre + ~0.35em).
        let tw = cx.backend.measure_text(fw.as_wire(), 13.0);
        let tx = chip.origin.x + (chip.size.x - tw) / 2.0;
        let ty = chip.origin.y + chip.size.y / 2.0 + 4.5;
        draw_line(cx, fw.as_wire(), text_color, tx, ty);
    }
    cx.backend.restore();
    // Scroll chevrons (only when the strip overflows). Each sits over a
    // subtle muted background; dim the one that can't scroll further.
    if let Some((left, right)) = zones {
        let max = framework_row_overflow(w);
        paint_chevron(
            cx,
            theme,
            Icon::ChevronLeft,
            left,
            state.framework_scroll > 0.0,
        );
        paint_chevron(
            cx,
            theme,
            Icon::ChevronRight,
            right,
            state.framework_scroll < max,
        );
    }
    // A thin divider line below the strip, spanning the padded width.
    let dy = y + CHIP_HEIGHT + CHIP_DIVIDER_GAP / 2.0;
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x + PAD_X, dy),
            size: Point2D::new(usable, 1.0),
        },
        theme.border,
    );
    chips_body_top(y)
}

/// Map a `ChunkStatus` to its trailing status glyph + tint.
fn status_glyph(theme: &Theme, status: ChunkStatus) -> (Icon, Color) {
    match status {
        ChunkStatus::Pending | ChunkStatus::Skipped => (Icon::Minus, theme.muted_foreground),
        ChunkStatus::Running => (Icon::Loader, theme.primary),
        ChunkStatus::Done => (Icon::Check, theme.primary),
        ChunkStatus::Degraded | ChunkStatus::Failed => (Icon::AlertTriangle, theme.destructive),
    }
}

/// Paint one progress row: left label + a trailing status glyph.
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

/// Glyph for the tri-state planning / assembly rows (None = dim,
/// Some(false) = running, Some(true) = done).
fn phase_glyph(theme: &Theme, done: Option<bool>) -> (Icon, Color) {
    match done {
        None => (Icon::Minus, theme.muted_foreground),
        Some(false) => (Icon::Loader, theme.primary),
        Some(true) => (Icon::Check, theme.primary),
    }
}

/// A centered fixed-height button rect at `top`, clamped to width.
fn idle_btn_rect(x: f32, w: f32, top: f32) -> Rect {
    let bw = (w - 2.0 * PAD_X).min(IDLE_BTN_W_MAX);
    Rect {
        origin: Point2D::new(x + (w - bw) / 2.0, top),
        size: Point2D::new(bw, IDLE_BTN_H),
    }
}

/// y of the centered Idle Generate button. Shared by painter + hit-test.
fn idle_generate_y(state: &CodegenState, body_y: f32) -> f32 {
    // badge + title(16) + subtitle(20) + gap(24), then the button.
    let mut y = body_y + IDLE_TOP_PAD + BADGE_SIZE + 16.0 + 20.0 + 24.0;
    if state.error.is_some() {
        y += PROGRESS_ROW_H;
    }
    y
}

/// Draw `label` centered horizontally in `[x, x+w]` at baseline `py`.
fn draw_centered_line(cx: &mut PaintCx<'_>, text: &str, color: Color, x: f32, w: f32, py: f32) {
    let tw = cx.backend.measure_text(text, 13.0);
    draw_line(cx, text, color, x + (w - tw) / 2.0, py);
}

/// Paint a centered 16px-icon + 13px-label group inside `rect`. `filled`
/// → primary fill; else a borderless text button in the foreground color.
fn paint_centered_icon_label(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    icon: Icon,
    label: &str,
    rect: Rect,
    filled: bool,
) {
    let content_color = if filled {
        cx.backend
            .fill_round_rect(rect, INPUT_RADIUS, theme.primary);
        theme.primary_foreground
    } else {
        theme.foreground
    };
    let icon_sz = 16.0;
    let gap = 6.0;
    let label_w = cx.backend.measure_text(label, 13.0);
    let total = icon_sz + gap + label_w;
    let start_x = rect.origin.x + (rect.size.x - total) / 2.0;
    let cy = rect.origin.y + rect.size.y / 2.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(start_x, cy - icon_sz / 2.0),
        icon_sz,
        content_color,
        1.6,
    );
    draw_line(cx, label, content_color, start_x + icon_sz + gap, cy + 5.0);
}

/// Idle body (centered empty state, TS parity): a sparkle badge, title,
/// subtitle, a primary "Generate <Framework>" button, and a borderless
/// "Export AI Bundle" text button. Any error surfaces above the buttons.
fn paint_idle_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    body_y: f32,
    w: f32,
) -> f32 {
    // 1. Sparkle badge — a light rounded square with a centered icon.
    let badge = Rect {
        origin: Point2D::new(x + (w - BADGE_SIZE) / 2.0, body_y + IDLE_TOP_PAD),
        size: Point2D::new(BADGE_SIZE, BADGE_SIZE),
    };
    cx.backend.fill_round_rect(badge, 12.0, theme.muted);
    let icon_sz = 24.0;
    draw_icon(
        cx.backend,
        Icon::Sparkles,
        Point2D::new(
            badge.origin.x + (BADGE_SIZE - icon_sz) / 2.0,
            badge.origin.y + (BADGE_SIZE - icon_sz) / 2.0,
        ),
        icon_sz,
        theme.primary,
        1.6,
    );
    let badge_bottom = badge.origin.y + BADGE_SIZE;
    // 2. Title — the panel only shows for a live selection, so ≥ 1.
    let n = state.selection_snapshot.len().max(1);
    let title = format!("{n} node selected");
    draw_centered_line(cx, &title, theme.foreground, x, w, badge_bottom + 18.0);
    // 3. Subtitle.
    let sub = "Generate production-ready code";
    draw_centered_line(cx, sub, theme.muted_foreground, x, w, badge_bottom + 38.0);
    let gen = idle_generate_y(state, body_y);
    // 4. Optional error row, above the Generate button.
    if let Some(err) = state.error.as_ref() {
        draw_centered_line(
            cx,
            err,
            theme.destructive,
            x,
            w,
            gen - PROGRESS_ROW_H + 14.0,
        );
    }
    // 5. Generate button — primary fill + sparkle + "Generate <Framework>".
    let generate = format!("Generate {}", state.framework.display_name());
    let gen_rect = idle_btn_rect(x, w, gen);
    paint_centered_icon_label(cx, theme, Icon::Sparkles, &generate, gen_rect, true);
    // 6. Export bundle — borderless text button with a braces icon.
    let by = gen + IDLE_BTN_H + 12.0;
    let bundle = idle_btn_rect(x, w, by);
    paint_centered_icon_label(cx, theme, Icon::Braces, "Export AI Bundle", bundle, false);
    by + IDLE_BTN_H + 12.0
}

/// y of the Generating Cancel button — past header + planning + chunks +
/// assembly rows + the trailing gap. Shared by painter + hit-test.
fn generating_cancel_y(state: &CodegenState, y: f32) -> f32 {
    y + PROGRESS_ROW_H * (3 + state.progress.chunks.len()) as f32 + SECTION_GAP
}

/// Generating body: header + planning/chunk/assembly rows + Cancel button.
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
    // Save / restore around the clipped text so the stateful skia clip
    // doesn't leak into the buttons painted below.
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

/// Paint a small icon + label button at `rect` (Complete-phase row).
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

/// y of the Complete 4-chip action row — past the optional notices and
/// the code-preview area. Shared by painter + hit-test.
fn complete_action_row_y(state: &CodegenState, y: f32) -> f32 {
    let mut y = y;
    if state.degraded {
        y += PROGRESS_ROW_H;
    }
    if !state.assets.is_empty() {
        y += PROGRESS_ROW_H;
    }
    // Mirror `paint_code_area`'s advance (area height + gap) without paint.
    y + CODE_AREA_H + 12.0
}

/// Geometry for the Complete phase's 4 action chips (Copy / Save /
/// Bundle / Regen), spread across the usable width. Painter + hit-test.
fn action_chip_rects(x: f32, y: f32, w: f32) -> [Rect; 4] {
    let usable = w - PAD_X * 2.0;
    let gap = 8.0;
    let btn_w = (usable - gap * 3.0) / 4.0;
    std::array::from_fn(|i| Rect {
        origin: Point2D::new(x + PAD_X + (btn_w + gap) * i as f32, y),
        size: Point2D::new(btn_w, INPUT_HEIGHT),
    })
}

/// Complete body: optional notices, code preview, 4-chip action row.
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
    let actions = [
        (Icon::Copy, "Copy"),
        (Icon::Download, "Save"),
        (Icon::Sparkles, "Bundle"),
        (Icon::RefreshCw, "Regen"),
    ];
    let rects = action_chip_rects(x, y, w);
    for ((icon, label), rect) in actions.iter().zip(rects) {
        paint_action_chip(cx, theme, *icon, label, rect);
    }
    y + INPUT_HEIGHT + 12.0
}

/// y of the Error Regenerate button — past the error-text row.
fn error_regenerate_y(y: f32) -> f32 {
    y + PROGRESS_ROW_H + SECTION_GAP
}

/// Error body: error text (destructive) + a full-width Regenerate button.
fn paint_error_body(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let msg = state.error.as_deref().unwrap_or("Generation failed");
    draw_line(cx, msg, theme.destructive, x + PAD_X, y + 16.0);
    paint_full_button(cx, theme, "Regenerate", x, error_regenerate_y(y), w, true)
}

/// Paint the full Code panel from `state`: framework chip row + a
/// phase-specific body. Returns the y after the painted content.
pub fn paint_code_panel(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    // A faint section label keeps the panel head consistent with Design.
    let mut y = paint_section_label(cx, theme, "Code", x, y, w);
    y = paint_framework_chips(cx, theme, state, x, y, w);
    match state.phase {
        CodegenPhase::Idle => paint_idle_body(cx, theme, state, x, y, w),
        CodegenPhase::Generating => paint_generating_body(cx, theme, state, x, y, w),
        CodegenPhase::Complete => paint_complete_body(cx, theme, state, x, y, w),
        CodegenPhase::Error => paint_error_body(cx, theme, state, x, y, w),
    }
}

/// Hit-test geometry for the Code panel — the clickable rects, in draw
/// order. Takes the SAME `(x, y, w)` origin `paint_code_panel` uses and
/// reuses its shared geometry helpers so paint + hit-test can't drift.
pub fn code_action_rects(
    x: f32,
    y: f32,
    w: f32,
    state: &CodegenState,
) -> Vec<(CodegenAction, Rect)> {
    let mut out: Vec<(CodegenAction, Rect)> = Vec::new();
    // Section label, then the framework chip row, then the phase body.
    let chips_y = y + SECTION_HEADER_HEIGHT;
    // When the strip overflows, the scroll chevrons take precedence over
    // chips at the band ends (pushed first → win the topmost hit-test).
    let zones = framework_chevron_zones(x, chips_y, w);
    if let Some((left, right)) = zones {
        out.push((CodegenAction::ScrollFrameworksLeft, left));
        out.push((CodegenAction::ScrollFrameworksRight, right));
    }
    let inset = if zones.is_some() { CHEVRON_ZONE_W } else { 0.0 };
    let (band_l, band_r) = (x + PAD_X + inset, x + w - PAD_X - inset);
    for (fw, rect) in framework_chip_rects(x, chips_y, state.framework_scroll) {
        // Only chips overlapping the visible (chevron-inset) band are clickable.
        if rect.origin.x + rect.size.x <= band_l || rect.origin.x >= band_r {
            continue;
        }
        out.push((CodegenAction::SelectFramework(fw), rect));
    }
    let body_y = chips_body_top(chips_y);
    match state.phase {
        CodegenPhase::Idle => {
            let gen_y = idle_generate_y(state, body_y);
            out.push((CodegenAction::Generate, idle_btn_rect(x, w, gen_y)));
            let bundle_y = gen_y + IDLE_BTN_H + 12.0;
            out.push((CodegenAction::ExportBundle, idle_btn_rect(x, w, bundle_y)));
        }
        CodegenPhase::Generating => {
            let cancel_y = generating_cancel_y(state, body_y);
            out.push((CodegenAction::Cancel, full_button_rect(x, cancel_y, w)));
        }
        CodegenPhase::Complete => {
            let row_y = complete_action_row_y(state, body_y);
            let [copy, save, bundle, regen] = action_chip_rects(x, row_y, w);
            out.push((CodegenAction::Copy, copy));
            out.push((CodegenAction::Download, save));
            out.push((CodegenAction::ExportBundle, bundle));
            out.push((CodegenAction::Regenerate, regen));
        }
        CodegenPhase::Error => {
            let regen_y = error_regenerate_y(body_y);
            out.push((CodegenAction::Regenerate, full_button_rect(x, regen_y, w)));
        }
    }
    out
}

#[cfg(test)]
#[path = "property_panel_code_tests.rs"]
mod tests;
