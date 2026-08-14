//! Framework selector geometry, paint, and hit-test helpers for the Code
//! panel. Kept separate because the selector is a self-contained horizontal
//! scrolling control shared by native and web hosts.

use super::{action_hovered, code_neutral_hover_color};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_inputs::{
    PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::codegen::{CodegenHover, CodegenPhase, CodegenState, Framework};

const CHIP_HEIGHT: f32 = 22.0;
pub(super) const TOUCH_TARGET_SIZE: f32 = 30.0;
const CHIP_PAD_X: f32 = 8.0;
const CHIP_FONT_SIZE: f32 = 11.0;
const CHIP_GAP: f32 = 2.0;
pub(super) const CHEVRON_ZONE_W: f32 = 18.0;
const CHIP_DIVIDER_GAP: f32 = 8.0;

#[derive(Clone, Copy)]
struct FrameworkStripMetrics {
    chip_height: f32,
    chip_min_width: f32,
    chevron_width: f32,
}

fn strip_metrics(touch_controls: bool) -> FrameworkStripMetrics {
    if touch_controls {
        FrameworkStripMetrics {
            chip_height: TOUCH_TARGET_SIZE,
            chip_min_width: TOUCH_TARGET_SIZE,
            chevron_width: TOUCH_TARGET_SIZE,
        }
    } else {
        FrameworkStripMetrics {
            chip_height: CHIP_HEIGHT,
            chip_min_width: 0.0,
            chevron_width: CHEVRON_ZONE_W,
        }
    }
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

fn chip_label_width(label: &str) -> f32 {
    label.chars().fold(0.0, |width, ch| {
        width
            + if ch.is_ascii() {
                CHIP_FONT_SIZE * 0.55
            } else {
                CHIP_FONT_SIZE
            }
    })
}

fn framework_row_width(touch_controls: bool) -> f32 {
    let metrics = strip_metrics(touch_controls);
    Framework::ALL
        .iter()
        .enumerate()
        .map(|(index, framework)| {
            let gap = if index == 0 { 0.0 } else { CHIP_GAP };
            let width = chip_label_width(framework_tab_label(*framework)) + CHIP_PAD_X * 2.0;
            gap + width.max(metrics.chip_min_width)
        })
        .sum()
}

fn framework_overflows(width: f32, touch_controls: bool) -> bool {
    framework_row_width(touch_controls) > (width - PAD_X * 2.0).max(0.0)
}

pub fn framework_row_overflow(width: f32) -> f32 {
    framework_row_overflow_for(width, false)
}

pub(crate) fn framework_row_overflow_for(width: f32, touch_controls: bool) -> f32 {
    let metrics = strip_metrics(touch_controls);
    let usable = (width - PAD_X * 2.0).max(0.0);
    let row_width = framework_row_width(touch_controls);
    if row_width <= usable {
        return 0.0;
    }
    (row_width - (usable - 2.0 * metrics.chevron_width)).max(0.0)
}

pub fn framework_row_band(panel_top: f32) -> (f32, f32) {
    framework_row_band_for(panel_top, false)
}

pub(crate) fn framework_row_band_for(panel_top: f32, touch_controls: bool) -> (f32, f32) {
    let metrics = strip_metrics(touch_controls);
    let top = panel_top + TAB_HEIGHT + SECTION_HEADER_HEIGHT;
    (top, top + metrics.chip_height)
}

pub(super) fn framework_chip_rects(
    x: f32,
    y: f32,
    width: f32,
    scroll: f32,
) -> Vec<(Framework, Rect)> {
    framework_chip_rects_for(x, y, width, scroll, false)
}

pub(super) fn framework_chip_rects_for(
    x: f32,
    y: f32,
    width: f32,
    scroll: f32,
    touch_controls: bool,
) -> Vec<(Framework, Rect)> {
    let metrics = strip_metrics(touch_controls);
    let inset = if framework_overflows(width, touch_controls) {
        metrics.chevron_width
    } else {
        0.0
    };
    let widths: Vec<f32> = Framework::ALL
        .iter()
        .map(|framework| {
            (chip_label_width(framework_tab_label(*framework)) + CHIP_PAD_X * 2.0)
                .max(metrics.chip_min_width)
        })
        .collect();
    let advances: Vec<f32> = widths.iter().map(|width| width + CHIP_GAP).collect();
    let rects = jian_widgets::components::tabs::Tabs::content_rects(
        Point2D::new(x + PAD_X + inset, y),
        &widths,
        &advances,
        metrics.chip_height,
        scroll,
    );
    Framework::ALL.iter().copied().zip(rects).collect()
}

pub(super) fn chips_body_top(y: f32) -> f32 {
    chips_body_top_for(y, false)
}

pub(super) fn chips_body_top_for(y: f32, touch_controls: bool) -> f32 {
    y + strip_metrics(touch_controls).chip_height + CHIP_DIVIDER_GAP + SECTION_GAP
}

pub(super) fn framework_chevron_zones(x: f32, y: f32, width: f32) -> Option<(Rect, Rect)> {
    framework_chevron_zones_for(x, y, width, false)
}

pub(super) fn framework_chevron_zones_for(
    x: f32,
    y: f32,
    width: f32,
    touch_controls: bool,
) -> Option<(Rect, Rect)> {
    if !framework_overflows(width, touch_controls) {
        return None;
    }
    let metrics = strip_metrics(touch_controls);
    let band_left = x + PAD_X;
    let band_right = x + width - PAD_X;
    Some((
        Rect {
            origin: Point2D::new(band_left, y),
            size: Point2D::new(metrics.chevron_width, metrics.chip_height),
        },
        Rect {
            origin: Point2D::new(band_right - metrics.chevron_width, y),
            size: Point2D::new(metrics.chevron_width, metrics.chip_height),
        },
    ))
}

pub fn framework_at(x: f32, y: f32, width: f32, point: Point2D, scroll: f32) -> Option<Framework> {
    framework_at_for_touch(x, y, width, point, scroll, false)
}

pub(crate) fn framework_at_for_touch(
    x: f32,
    y: f32,
    width: f32,
    point: Point2D,
    scroll: f32,
    touch_controls: bool,
) -> Option<Framework> {
    let metrics = strip_metrics(touch_controls);
    let usable = (width - PAD_X * 2.0).max(0.0);
    let inset = if framework_row_overflow_for(width, touch_controls) > 0.0 {
        metrics.chevron_width
    } else {
        0.0
    };
    let band_left = x + PAD_X + inset;
    let band_right = x + PAD_X + usable - inset;
    framework_chip_rects_for(x, y, width, scroll, touch_controls)
        .into_iter()
        .filter(|(_, rect)| rect.origin.x + rect.size.x > band_left && rect.origin.x < band_right)
        .find(|(_, rect)| {
            point.x >= rect.origin.x.max(band_left)
                && point.x <= (rect.origin.x + rect.size.x).min(band_right)
                && point.y >= rect.origin.y
                && point.y <= rect.origin.y + rect.size.y
        })
        .map(|(framework, _)| framework)
}

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

pub(super) fn paint_framework_chips(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    paint_framework_chips_for(cx, theme, state, x, y, width, false)
}

pub(super) fn paint_framework_chips_for(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &CodegenState,
    x: f32,
    y: f32,
    width: f32,
    touch_controls: bool,
) -> f32 {
    let metrics = strip_metrics(touch_controls);
    let usable = (width - PAD_X * 2.0).max(0.0);
    let zones = framework_chevron_zones_for(x, y, width, touch_controls);
    let inset = if zones.is_some() {
        metrics.chevron_width
    } else {
        0.0
    };
    let band = Rect {
        origin: Point2D::new(x + PAD_X + inset, y),
        size: Point2D::new((usable - inset * 2.0).max(0.0), metrics.chip_height),
    };
    cx.backend.save();
    cx.backend.clip_rect(band);
    let labels: Vec<&str> = Framework::ALL
        .iter()
        .map(|framework| framework_tab_label(*framework))
        .collect();
    let rects: Vec<Rect> =
        framework_chip_rects_for(x, y, width, state.framework_scroll.offset, touch_controls)
            .into_iter()
            .map(|(_, chip)| chip)
            .collect();
    let active = Framework::ALL
        .iter()
        .position(|framework| *framework == state.framework)
        .unwrap_or(0);
    let interactive = !matches!(state.phase, CodegenPhase::Generating);
    let hover = interactive
        .then_some(state.framework_hover)
        .flatten()
        .and_then(|hovered| {
            Framework::ALL
                .iter()
                .position(|framework| *framework == hovered)
        });
    jian_widgets::components::tabs::Tabs {
        labels: &labels,
        active,
        hover,
    }
    .paint_content(
        cx.backend,
        &rects,
        jian_widgets::components::tabs::ActiveStyle::PrimaryPill,
        false,
        CHIP_PAD_X,
        CHIP_FONT_SIZE,
        &crate::widgets::button::tokens_from_theme(theme),
    );
    cx.backend.restore();

    if let Some((left, right)) = zones {
        let max = framework_row_overflow_for(width, touch_controls);
        paint_chevron(
            cx,
            theme,
            Icon::ChevronLeft,
            left,
            interactive && state.framework_scroll.offset > 0.0,
            interactive && action_hovered(state, CodegenHover::ScrollFrameworksLeft),
        );
        paint_chevron(
            cx,
            theme,
            Icon::ChevronRight,
            right,
            interactive && state.framework_scroll.offset < max,
            interactive && action_hovered(state, CodegenHover::ScrollFrameworksRight),
        );
    }
    let divider_y = y + metrics.chip_height + CHIP_DIVIDER_GAP / 2.0;
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x + PAD_X, divider_y),
            size: Point2D::new(usable, 1.0),
        },
        theme.border,
    );
    chips_body_top_for(y, touch_controls)
}
