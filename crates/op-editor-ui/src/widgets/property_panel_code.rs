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
use code_i18n::CodePanelStrings;
use op_editor_core::codegen::{CodegenHover, CodegenPhase, CodegenState, Framework};
use op_i18n::Locale;

#[path = "property_panel_code_i18n.rs"]
mod code_i18n;
#[path = "property_panel_code_complete.rs"]
mod complete;
#[path = "property_panel_code_generating.rs"]
mod generating;

/// Map a click on the Code tab to its action (framework chip / button), or
/// `None`. Uses the same content origin the painter does (panel left,
/// pinned tab-strip bottom, panel width).
pub fn code_action_hit(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
) -> Option<PropertyPanelAction> {
    code_action_hit_with_locale(panel_rect, state, point, Locale::EnUs)
}

pub fn code_action_hit_with_locale(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
    locale: Locale,
) -> Option<PropertyPanelAction> {
    let inside = |r: Rect| {
        point.x >= r.origin.x
            && point.x <= r.origin.x + r.size.x
            && point.y >= r.origin.y
            && point.y <= r.origin.y + r.size.y
    };
    code_action_rects_in_panel_with_locale(panel_rect, state, locale)
        .into_iter()
        .find(|(_, r)| inside(*r))
        .map(|(a, _)| PropertyPanelAction::Codegen(a))
}

/// Map a cursor point over the Code panel to the hover state the hosts store
/// on `CodegenState`. Shares `code_action_rects` with click hit-testing.
pub fn code_hover_at(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
) -> (Option<Framework>, Option<CodegenHover>) {
    code_hover_at_with_locale(panel_rect, state, point, Locale::EnUs)
}

pub fn code_hover_at_with_locale(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
    locale: Locale,
) -> (Option<Framework>, Option<CodegenHover>) {
    let inside = |r: Rect| {
        point.x >= r.origin.x
            && point.x <= r.origin.x + r.size.x
            && point.y >= r.origin.y
            && point.y <= r.origin.y + r.size.y
    };
    let hit = code_action_rects_in_panel_with_locale(panel_rect, state, locale)
        .into_iter()
        .find(|(_, r)| inside(*r))
        .map(|(a, _)| a);
    match hit {
        Some(CodegenAction::SelectFramework(fw)) => (Some(fw), None),
        Some(action) => (None, hover_for_action(action)),
        None => (None, None),
    }
}

/// Map a point inside the generated-code preview to a byte offset in
/// `state.code`. Returns `None` when the Code tab is not in Complete phase
/// or the point is outside the preview card.
pub fn code_text_offset_at(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
) -> Option<usize> {
    code_text_offset_at_in_panel(panel_rect, state, point)
}

pub fn code_preview_rect(panel_rect: Rect, state: &CodegenState) -> Option<Rect> {
    if !matches!(state.phase, CodegenPhase::Complete) {
        return None;
    }
    Some(complete::code_area_rect_for_panel(
        state,
        complete_layout(panel_rect),
    ))
}

pub fn code_preview_max_scroll(panel_rect: Rect, state: &CodegenState) -> Option<f32> {
    if !matches!(state.phase, CodegenPhase::Complete) {
        return None;
    }
    Some(complete::code_preview_max_scroll(
        state,
        complete_layout(panel_rect),
    ))
}

fn complete_layout(panel_rect: Rect) -> complete::CompleteLayout {
    complete::CompleteLayout {
        x: panel_rect.origin.x,
        y: code_body_y(panel_rect),
        w: panel_rect.size.x,
        progress_row_h: PROGRESS_ROW_H,
        panel_bottom: Some(panel_bottom(panel_rect)),
    }
}

fn code_body_y(panel_rect: Rect) -> f32 {
    let tab_bottom = panel_rect.origin.y + TAB_HEIGHT;
    let chips_y = tab_bottom + SECTION_HEADER_HEIGHT;
    chips_body_top(chips_y)
}

fn panel_bottom(panel_rect: Rect) -> f32 {
    panel_rect.origin.y + panel_rect.size.y
}

fn code_text_offset_at_in_panel(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
) -> Option<usize> {
    if !matches!(state.phase, CodegenPhase::Complete) {
        return None;
    }
    complete::code_text_offset_at_in_panel(state, point, complete_layout(panel_rect))
}

/// Height of a framework chip + the gap below the chip row.
const CHIP_HEIGHT: f32 = 22.0;
const CHIP_PAD_X: f32 = 8.0;
const CHIP_FONT_SIZE: f32 = 11.0;
const CHIP_GAP: f32 = 2.0;
/// Width of each scroll-chevron zone reserved at the strip ends when the
/// framework row overflows. Paint + hit-test inset the chip clip band by
/// this on each side so chips never paint under the chevrons.
const CHEVRON_ZONE_W: f32 = 18.0;
/// Extra space below the chip row for the divider line that separates the
/// framework selector from the phase body (TS parity).
const CHIP_DIVIDER_GAP: f32 = 8.0;
/// Centered Idle empty-state metrics (TS parity).
const BADGE_SIZE: f32 = 44.0;
const IDLE_TOP_PAD: f32 = 24.0;
const IDLE_BTN_H: f32 = 34.0;
const IDLE_BTN_W_MAX: f32 = 200.0;
/// Height of a progress / status row (chunk, planning, assembly).
const PROGRESS_ROW_H: f32 = 24.0;

#[derive(Clone, Copy)]
struct CodePanelLayout {
    x: f32,
    y: f32,
    w: f32,
    panel_bottom: Option<f32>,
}

fn hover_for_action(action: CodegenAction) -> Option<CodegenHover> {
    match action {
        CodegenAction::SelectFramework(_) => None,
        CodegenAction::Generate => Some(CodegenHover::Generate),
        CodegenAction::Regenerate => Some(CodegenHover::Regenerate),
        CodegenAction::Cancel => Some(CodegenHover::Cancel),
        CodegenAction::Copy => Some(CodegenHover::Copy),
        CodegenAction::Download => Some(CodegenHover::Download),
        CodegenAction::ExportBundle => Some(CodegenHover::ExportBundle),
        CodegenAction::ScrollFrameworksLeft => Some(CodegenHover::ScrollFrameworksLeft),
        CodegenAction::ScrollFrameworksRight => Some(CodegenHover::ScrollFrameworksRight),
    }
}

fn action_hovered(state: &CodegenState, hover: CodegenHover) -> bool {
    state.action_hover == Some(hover)
}

fn paint_primary_hover_overlay(cx: &mut PaintCx<'_>, rect: Rect, radius: f32) {
    cx.backend.fill_round_rect(
        rect,
        radius,
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.14,
        },
    );
}

fn code_neutral_hover_color(theme: &Theme) -> Color {
    Color {
        r: theme.foreground.r,
        g: theme.foreground.g,
        b: theme.foreground.b,
        a: 0.12,
    }
}

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

#[derive(Debug, Clone, Copy)]
struct FullButtonStyle {
    filled: bool,
    hovered: bool,
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
    style: FullButtonStyle,
) -> f32 {
    let btn = full_button_rect(x, y, w);
    if style.filled {
        cx.backend.fill_round_rect(btn, INPUT_RADIUS, theme.primary);
        if style.hovered {
            paint_primary_hover_overlay(cx, btn, INPUT_RADIUS);
        }
    } else {
        cx.backend.fill_round_rect(btn, INPUT_RADIUS, theme.muted);
        if style.hovered {
            cx.backend
                .fill_round_rect(btn, INPUT_RADIUS, code_neutral_hover_color(theme));
        }
        cx.backend
            .stroke_round_rect(btn, INPUT_RADIUS, theme.border, 1.0);
    }
    let text_color = if style.filled {
        theme.primary_foreground
    } else {
        theme.foreground
    };
    let tw = cx.backend.measure_text(label, 13.0);
    let tx = btn.origin.x + (btn.size.x - tw) / 2.0;
    draw_line(cx, label, text_color, tx, btn.origin.y + 19.0);
    y + INPUT_HEIGHT + 12.0
}

fn framework_tab_label(fw: Framework) -> &'static str {
    match fw {
        Framework::React => "React",
        Framework::Vue => "Vue",
        Framework::Svelte => "Svelte",
        Framework::Html => "HTML",
        Framework::Flutter => "Flutter",
        Framework::SwiftUi => "SwiftUI",
        Framework::Compose => "Compose",
        Framework::ReactNative => "RN",
    }
}

/// Backend-free advance estimate for a chip label at 11px, matching the
/// `RenderBackend::measure_text` trait-default heuristic (0.55 × size per
/// ASCII char) so painter + hit-test wrap identically. TS tab labels are
/// all ASCII, so the same label drives geometry and paint.
fn chip_label_width(label: &str) -> f32 {
    label.chars().fold(0.0, |w, c| {
        w + if c.is_ascii() {
            CHIP_FONT_SIZE * 0.55
        } else {
            CHIP_FONT_SIZE
        }
    })
}

/// Total width of the single-row framework strip (all chips + inter-gaps).
fn framework_row_width() -> f32 {
    let mut total = 0.0;
    for (i, fw) in Framework::ALL.iter().enumerate() {
        if i > 0 {
            total += CHIP_GAP;
        }
        total += chip_label_width(framework_tab_label(*fw)) + CHIP_PAD_X * 2.0;
    }
    total
}

/// True when the framework strip is wider than the usable panel width and
/// must scroll — i.e. the chevron controls appear and the chips inset past
/// them.
fn framework_overflows(w: f32) -> bool {
    framework_row_width() > (w - PAD_X * 2.0).max(0.0)
}

/// The maximum horizontal scroll of the framework strip (0 when it fits).
/// When it overflows, the two chevron zones each consume `CHEVRON_ZONE_W`
/// of the usable width, so the chips scroll within the narrower middle band.
/// The host clamps `framework_scroll` to `[0, this]`.
pub fn framework_row_overflow(w: f32) -> f32 {
    let usable = (w - PAD_X * 2.0).max(0.0);
    if framework_row_width() <= usable {
        return 0.0;
    }
    (framework_row_width() - (usable - 2.0 * CHEVRON_ZONE_W)).max(0.0)
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
fn framework_chip_rects(x: f32, y: f32, w: f32, scroll: f32) -> Vec<(Framework, Rect)> {
    // When the strip overflows, inset past the left chevron zone so the
    // first/last chips sit between the arrows instead of behind them.
    let inset = if framework_overflows(w) {
        CHEVRON_ZONE_W
    } else {
        0.0
    };
    let mut chip_x = x + PAD_X + inset - scroll;
    let mut out = Vec::with_capacity(Framework::ALL.len());
    for fw in Framework::ALL {
        let chip_w = chip_label_width(framework_tab_label(fw)) + CHIP_PAD_X * 2.0;
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
    if !framework_overflows(w) {
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
    framework_chip_rects(x, y, w, scroll)
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
fn paint_chevron(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    icon: Icon,
    zone: Rect,
    enabled: bool,
    hovered: bool,
) {
    cx.backend.fill_round_rect(zone, 6.0, theme.muted);
    if hovered {
        cx.backend
            .fill_round_rect(zone, 6.0, code_neutral_hover_color(theme));
    }
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
    for (fw, chip) in framework_chip_rects(x, y, w, state.framework_scroll) {
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
        // real measured advance (11px baseline ≈ vertical centre + ~0.35em).
        let label = framework_tab_label(fw);
        let tw = cx.backend.measure_text(label, CHIP_FONT_SIZE);
        let tx = chip.origin.x + (chip.size.x - tw) / 2.0;
        let ty = chip.origin.y + chip.size.y / 2.0 + 4.0;
        let layout = TextLayout::single_run(
            label,
            "system-ui",
            CHIP_FONT_SIZE,
            to_jian_color(text_color),
            origin(),
        );
        cx.backend.draw_text(&layout, Point2D::new(tx, ty));
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
            action_hovered(state, CodegenHover::ScrollFrameworksLeft),
        );
        paint_chevron(
            cx,
            theme,
            Icon::ChevronRight,
            right,
            state.framework_scroll < max,
            action_hovered(state, CodegenHover::ScrollFrameworksRight),
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
    hovered: bool,
) {
    let content_color = if filled {
        cx.backend
            .fill_round_rect(rect, INPUT_RADIUS, theme.primary);
        if hovered {
            paint_primary_hover_overlay(cx, rect, INPUT_RADIUS);
        }
        theme.primary_foreground
    } else {
        if hovered {
            cx.backend
                .fill_round_rect(rect, INPUT_RADIUS, code_neutral_hover_color(theme));
        }
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
    strings: CodePanelStrings,
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
    let icon_sz = 18.0;
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
    let title = strings.selected_nodes(n);
    draw_centered_line(cx, &title, theme.foreground, x, w, badge_bottom + 18.0);
    // 3. Subtitle.
    let sub = strings.idle_subtitle();
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
    let generate = strings.generate_framework(state.framework);
    let gen_rect = idle_btn_rect(x, w, gen);
    paint_centered_icon_label(
        cx,
        theme,
        Icon::Sparkles,
        &generate,
        gen_rect,
        true,
        action_hovered(state, CodegenHover::Generate),
    );
    // 6. Export bundle — borderless text button with a braces icon.
    let by = gen + IDLE_BTN_H + 12.0;
    let bundle = idle_btn_rect(x, w, by);
    paint_centered_icon_label(
        cx,
        theme,
        Icon::Braces,
        strings.export_ai_bundle(),
        bundle,
        false,
        action_hovered(state, CodegenHover::ExportBundle),
    );
    by + IDLE_BTN_H + 12.0
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
    strings: CodePanelStrings,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let msg = state
        .error
        .as_deref()
        .unwrap_or(strings.generation_failed());
    draw_line(cx, msg, theme.destructive, x + PAD_X, y + 16.0);
    paint_full_button(
        cx,
        theme,
        strings.regenerate(),
        x,
        error_regenerate_y(y),
        w,
        FullButtonStyle {
            filled: true,
            hovered: action_hovered(state, CodegenHover::Regenerate),
        },
    )
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
    paint_code_panel_at_with_locale(cx, theme, state, Locale::EnUs, x, y, w, 0)
}

/// Paint the full Code panel with a host clock for animated affordances.
pub fn paint_code_panel_at(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    y: f32,
    w: f32,
    now_ms: u64,
) -> f32 {
    paint_code_panel_at_with_locale(cx, theme, state, Locale::EnUs, x, y, w, now_ms)
}

#[allow(clippy::too_many_arguments)]
pub fn paint_code_panel_at_with_locale(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    locale: Locale,
    x: f32,
    y: f32,
    w: f32,
    now_ms: u64,
) -> f32 {
    paint_code_panel_with_bottom(
        cx,
        theme,
        state,
        CodePanelStrings::new(locale),
        CodePanelLayout {
            x,
            y,
            w,
            panel_bottom: None,
        },
        now_ms,
    )
}

/// Paint the full Code panel using the containing PropertyPanel rect. Complete
/// code previews use the panel bottom so they can fill the remaining height.
pub fn paint_code_panel_in_panel(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    panel_rect: Rect,
    now_ms: u64,
) -> f32 {
    paint_code_panel_in_panel_with_locale(cx, theme, state, Locale::EnUs, panel_rect, now_ms)
}

pub fn paint_code_panel_in_panel_with_locale(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    locale: Locale,
    panel_rect: Rect,
    now_ms: u64,
) -> f32 {
    paint_code_panel_with_bottom(
        cx,
        theme,
        state,
        CodePanelStrings::new(locale),
        CodePanelLayout {
            x: panel_rect.origin.x,
            y: panel_rect.origin.y + TAB_HEIGHT,
            w: panel_rect.size.x,
            panel_bottom: Some(panel_bottom(panel_rect)),
        },
        now_ms,
    )
}

fn paint_code_panel_with_bottom(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    strings: CodePanelStrings,
    layout: CodePanelLayout,
    now_ms: u64,
) -> f32 {
    // A faint section label keeps the panel head consistent with Design.
    let x = layout.x;
    let w = layout.w;
    let mut y = paint_section_label(cx, theme, strings.title(), x, layout.y, w);
    y = paint_framework_chips(cx, theme, state, x, y, w);
    match state.phase {
        CodegenPhase::Idle => paint_idle_body(cx, theme, state, strings, x, y, w),
        CodegenPhase::Generating => {
            generating::paint_generating_body(cx, theme, state, strings, x, y, w, now_ms)
        }
        CodegenPhase::Complete => complete::paint_complete_body_in_panel(
            cx,
            theme,
            state,
            strings,
            complete::CompleteLayout {
                x,
                y,
                w,
                progress_row_h: PROGRESS_ROW_H,
                panel_bottom: layout.panel_bottom,
            },
        ),
        CodegenPhase::Error => paint_error_body(cx, theme, state, strings, x, y, w),
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
    code_action_rects_with_bottom(x, y, w, state, None, CodePanelStrings::new(Locale::EnUs))
}

pub fn code_action_rects_in_panel(
    panel_rect: Rect,
    state: &CodegenState,
) -> Vec<(CodegenAction, Rect)> {
    code_action_rects_in_panel_with_locale(panel_rect, state, Locale::EnUs)
}

pub fn code_action_rects_in_panel_with_locale(
    panel_rect: Rect,
    state: &CodegenState,
    locale: Locale,
) -> Vec<(CodegenAction, Rect)> {
    code_action_rects_with_bottom(
        panel_rect.origin.x,
        panel_rect.origin.y + TAB_HEIGHT,
        panel_rect.size.x,
        state,
        Some(panel_bottom(panel_rect)),
        CodePanelStrings::new(locale),
    )
}

fn code_action_rects_with_bottom(
    x: f32,
    y: f32,
    w: f32,
    state: &CodegenState,
    panel_bottom: Option<f32>,
    strings: CodePanelStrings,
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
    for (fw, rect) in framework_chip_rects(x, chips_y, w, state.framework_scroll) {
        // Clamp the clickable rect to the visible (chevron-inset) band so a
        // chip's scrolled-off / clipped portion is NOT clickable (matches the
        // painter's clip and the hover hit-test in `framework_at`).
        let left = rect.origin.x.max(band_l);
        let right = (rect.origin.x + rect.size.x).min(band_r);
        if right - left <= 0.0 {
            continue; // fully outside the visible band
        }
        let clipped = Rect {
            origin: Point2D::new(left, rect.origin.y),
            size: Point2D::new(right - left, rect.size.y),
        };
        out.push((CodegenAction::SelectFramework(fw), clipped));
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
            let cancel_y = generating::generating_cancel_y(state, body_y);
            out.push((CodegenAction::Cancel, full_button_rect(x, cancel_y, w)));
        }
        CodegenPhase::Complete => {
            let row_y = complete::complete_action_row_y_in_panel(
                state,
                complete::CompleteLayout {
                    x,
                    y: body_y,
                    w,
                    progress_row_h: PROGRESS_ROW_H,
                    panel_bottom,
                },
            );
            let [copy, save, bundle, regen] = complete::action_chip_rects(x, row_y, w, strings);
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
