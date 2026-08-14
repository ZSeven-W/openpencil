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
    paint_section_label, INPUT_HEIGHT, PAD_X, SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use code_i18n::CodePanelStrings;
use op_editor_core::codegen::{CodegenHover, CodegenPhase, CodegenState, Framework};
use op_i18n::Locale;

#[path = "property_panel_code_i18n.rs"]
mod code_i18n;
#[path = "property_panel_code_complete.rs"]
mod complete;
#[path = "property_panel_code_error.rs"]
mod error;
#[path = "property_panel_code_framework.rs"]
mod framework;
#[path = "property_panel_code_generating.rs"]
mod generating;

use framework::{
    chips_body_top, chips_body_top_for, framework_chevron_zones, framework_chevron_zones_for,
    framework_chip_rects, framework_chip_rects_for, paint_framework_chips,
    paint_framework_chips_for, CHEVRON_ZONE_W, TOUCH_TARGET_SIZE,
};
pub use framework::{framework_at, framework_row_band, framework_row_overflow};

pub(crate) use framework::{framework_row_band_for, framework_row_overflow_for};

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
    code_action_hit_with_locale_for_touch(panel_rect, state, point, locale, false)
}

pub(crate) fn code_action_hit_with_locale_for_touch(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
    locale: Locale,
    touch_controls: bool,
) -> Option<PropertyPanelAction> {
    code_action_rects_in_panel_with_locale_for_touch(panel_rect, state, locale, touch_controls)
        .into_iter()
        .find(|(_, r)| r.contains(point))
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
    code_hover_at_with_locale_for_touch(panel_rect, state, point, locale, false)
}

pub(crate) fn code_hover_at_with_locale_for_touch(
    panel_rect: Rect,
    state: &CodegenState,
    point: Point2D,
    locale: Locale,
    touch_controls: bool,
) -> (Option<Framework>, Option<CodegenHover>) {
    let hit =
        code_action_rects_in_panel_with_locale_for_touch(panel_rect, state, locale, touch_controls)
            .into_iter()
            .find(|(_, r)| r.contains(point))
            .map(|(a, _)| a);
    match hit {
        Some(CodegenAction::SelectFramework(fw)) => (Some(fw), None),
        Some(action) => (None, codegen_hover_for_action(action)),
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
    code_body_y_for(panel_rect, false)
}

fn code_body_y_for(panel_rect: Rect, touch_controls: bool) -> f32 {
    let tab_bottom = panel_rect.origin.y + TAB_HEIGHT;
    let chips_y = tab_bottom + SECTION_HEADER_HEIGHT;
    chips_body_top_for(chips_y, touch_controls)
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

pub fn codegen_hover_for_action(action: CodegenAction) -> Option<CodegenHover> {
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
    let layout = TextLayout::single_run(text, "system-ui", 13.0, (color).to_jian(), origin());
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
    // Filled → primary CTA; else an outline (border + hover wash). jian Button
    // owns the fill / border / hover feedback + centered label.
    let variant = if style.filled {
        jian_widgets::components::button::ButtonVariant::Primary
    } else {
        jian_widgets::components::button::ButtonVariant::Outline
    };
    jian_widgets::components::button::Button {
        label,
        icon_paths: None,
        variant,
        enabled: true,
        hovered: style.hovered,
        pressed: false,
        font_size: 13.0,
    }
    .paint(
        cx.backend,
        btn,
        &crate::widgets::button::tokens_from_theme(theme),
    );
    y + INPUT_HEIGHT + 12.0
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
    let tw = text_metrics::measure_chrome(cx.backend, text, 13.0);
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
    // Filled → primary CTA; else a borderless (ghost) text button. jian Button
    // centers the icon+label group and owns the fill / hover feedback; the
    // multi-subpath icon (sparkles / braces) renders in full via `icon_paths`.
    let variant = if filled {
        jian_widgets::components::button::ButtonVariant::Primary
    } else {
        jian_widgets::components::button::ButtonVariant::Ghost
    };
    jian_widgets::components::button::Button {
        label,
        icon_paths: Some(icon.paths()),
        variant,
        enabled: true,
        hovered,
        pressed: false,
        font_size: 13.0,
    }
    .paint(
        cx.backend,
        rect,
        &crate::widgets::button::tokens_from_theme(theme),
    );
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
    // 2. Title — live generation-target count (selection, else the active
    //    page's children; the panel builder pre-fills `selection_snapshot`
    //    with the live ids). An empty page mirrors the TS hardcoded
    //    'No nodes on page' literal (code-panel.tsx:400 — not i18n'd).
    let n = state.selection_snapshot.len();
    let title = if n > 0 {
        strings.selected_nodes(n)
    } else {
        "No nodes on page".to_string()
    };
    draw_centered_line(cx, &title, theme.foreground, x, w, badge_bottom + 18.0);
    // 3. Subtitle.
    let sub = strings.idle_subtitle();
    draw_centered_line(cx, sub, theme.muted_foreground, x, w, badge_bottom + 38.0);
    let gen = idle_generate_y(state, body_y);
    // 4. Optional error row, above the Generate button.
    if let Some(err) = state.error.as_ref() {
        let detail = error::display_error_detail(strings, err);
        let detail = crate::util::ellipsize_to_width(&detail, w - PAD_X * 2.0, |text| {
            text_metrics::measure_chrome(cx.backend, text, 13.0)
        });
        draw_centered_line(
            cx,
            &detail,
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
        None,
        false,
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
    paint_code_panel_in_panel_with_locale_and_pressed(
        cx, theme, state, locale, panel_rect, now_ms, None,
    )
}

pub fn paint_code_panel_in_panel_with_locale_and_pressed(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    locale: Locale,
    panel_rect: Rect,
    now_ms: u64,
    pressed: Option<CodegenHover>,
) -> f32 {
    paint_code_panel_in_panel_with_locale_pressed_and_touch(
        cx, theme, state, locale, panel_rect, now_ms, pressed, false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_code_panel_in_panel_with_locale_pressed_and_touch(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    locale: Locale,
    panel_rect: Rect,
    now_ms: u64,
    pressed: Option<CodegenHover>,
    touch_controls: bool,
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
        pressed,
        touch_controls,
    )
}

#[allow(clippy::too_many_arguments)]
fn paint_code_panel_with_bottom(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    strings: CodePanelStrings,
    layout: CodePanelLayout,
    now_ms: u64,
    pressed: Option<CodegenHover>,
    touch_controls: bool,
) -> f32 {
    // A faint section label keeps the panel head consistent with Design.
    let x = layout.x;
    let w = layout.w;
    let mut y = paint_section_label(cx, theme, strings.title(), x, layout.y, w);
    y = if touch_controls {
        paint_framework_chips_for(cx, theme, state, x, y, w, true)
    } else {
        paint_framework_chips(cx, theme, state, x, y, w)
    };
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
            pressed,
        ),
        CodegenPhase::Error => error::paint_error_body(cx, theme, state, strings, x, y, w),
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
    code_action_rects_with_bottom(
        x,
        y,
        w,
        state,
        None,
        CodePanelStrings::new(Locale::EnUs),
        false,
    )
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
    code_action_rects_in_panel_with_locale_for_touch(panel_rect, state, locale, false)
}

pub(crate) fn code_action_rects_in_panel_with_locale_for_touch(
    panel_rect: Rect,
    state: &CodegenState,
    locale: Locale,
    touch_controls: bool,
) -> Vec<(CodegenAction, Rect)> {
    code_action_rects_with_bottom(
        panel_rect.origin.x,
        panel_rect.origin.y + TAB_HEIGHT,
        panel_rect.size.x,
        state,
        Some(panel_bottom(panel_rect)),
        CodePanelStrings::new(locale),
        touch_controls,
    )
}

fn code_action_rects_with_bottom(
    x: f32,
    y: f32,
    w: f32,
    state: &CodegenState,
    panel_bottom: Option<f32>,
    strings: CodePanelStrings,
    touch_controls: bool,
) -> Vec<(CodegenAction, Rect)> {
    let mut out: Vec<(CodegenAction, Rect)> = Vec::new();
    // Section label, then the framework chip row, then the phase body.
    let chips_y = y + SECTION_HEADER_HEIGHT;
    // When the strip overflows, the scroll chevrons take precedence over
    // chips at the band ends (pushed first → win the topmost hit-test).
    let zones = if touch_controls {
        framework_chevron_zones_for(x, chips_y, w, true)
    } else {
        framework_chevron_zones(x, chips_y, w)
    };
    let framework_interactive = !matches!(state.phase, CodegenPhase::Generating);
    if framework_interactive {
        if let Some((left, right)) = zones {
            out.push((CodegenAction::ScrollFrameworksLeft, left));
            out.push((CodegenAction::ScrollFrameworksRight, right));
        }
    }
    let inset = if zones.is_some() {
        if touch_controls {
            TOUCH_TARGET_SIZE
        } else {
            CHEVRON_ZONE_W
        }
    } else {
        0.0
    };
    let (band_l, band_r) = (x + PAD_X + inset, x + w - PAD_X - inset);
    let chip_rects = if touch_controls {
        framework_chip_rects_for(x, chips_y, w, state.framework_scroll.offset, true)
    } else {
        framework_chip_rects(x, chips_y, w, state.framework_scroll.offset)
    };
    for (fw, rect) in chip_rects {
        // Clamp the clickable rect to the visible (chevron-inset) band so a
        // chip's scrolled-off / clipped portion is NOT clickable (matches the
        // painter's clip and the hover hit-test in `framework_at`).
        let left = rect.origin.x.max(band_l);
        let right = (rect.origin.x + rect.size.x).min(band_r);
        if right - left <= 0.0 || (touch_controls && right - left < TOUCH_TARGET_SIZE) {
            continue; // fully outside the visible band
        }
        let clipped = Rect {
            origin: Point2D::new(left, rect.origin.y),
            size: Point2D::new(right - left, rect.size.y),
        };
        if framework_interactive {
            out.push((CodegenAction::SelectFramework(fw), clipped));
        }
    }
    let body_y = if touch_controls {
        chips_body_top_for(chips_y, true)
    } else {
        chips_body_top(chips_y)
    };
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
            out.extend(error::error_action_rects(state, x, body_y, w));
        }
    }
    out
}

#[cfg(test)]
#[path = "property_panel_code_tests.rs"]
mod tests;
