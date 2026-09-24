//! The compact (phone) Home's pinned chrome: the bottom nav (创作 / 作品 /
//! 设置) and the top bar with the 普通 / 专业 switch and the settings gear.
//! Split out of `home_surface_paint_compact.rs` at the 800-line cap; pure
//! code motion, the page painters stay in the spine.

use super::super::super::{copy, fade, HomeLayout, HomeSurface, StudioPalette, HOME_TOPBAR_H};
use super::super::cards::text_weighted;
use super::SANS;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::HomeHit;

// ── the pinned bottom nav ──────────────────────────────────────────────

pub(super) fn paint_bottom_nav(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let nav = layout.bottom_nav;
    if nav.size.y <= 0.0 {
        return;
    }
    cx.backend.fill_rect(nav, palette.topbar);
    cx.backend.stroke_line(
        Point2D::new(nav.origin.x, nav.origin.y),
        Point2D::new(nav.origin.x + nav.size.x, nav.origin.y),
        palette.topbar_line,
        1.0,
    );
    for (index, (rect, (label, icon))) in layout
        .nav_items
        .iter()
        .zip([
            (copy::home_str(locale, "home.nav.create"), Icon::Sparkles),
            (
                copy::home_str(locale, "home.nav.projects"),
                Icon::LayoutGrid,
            ),
            (
                copy::home_str(locale, "home.nav.settings"),
                Icon::SlidersHorizontal,
            ),
        ])
        .enumerate()
    {
        let hit = match index {
            0 => HomeHit::NavCreate,
            1 => HomeHit::NavProjects,
            _ => HomeHit::NavSettings,
        };
        // The page on show is the active tab: 创作 unless the 作品
        // page replaced it.
        let active = index == usize::from(surface.works_page());
        let enabled = true;
        let pressed = surface.state.pressed == Some(hit) && enabled;
        let color = if pressed || active {
            palette.blue
        } else if enabled {
            fade(palette.ink, 0.6)
        } else {
            fade(palette.ink, 0.38)
        };
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(
                rect.origin.x + (rect.size.x - 21.0) / 2.0,
                rect.origin.y + 12.0,
            ),
            21.0,
            color,
            1.7,
        );
        let label_w = cx.backend.measure_text_family(label, 10.0, SANS);
        text_weighted(
            cx,
            label,
            Point2D::new(
                rect.origin.x + (rect.size.x - label_w).max(0.0) / 2.0,
                rect.origin.y + 47.0,
            ),
            10.0,
            color,
            if active { 600 } else { 500 },
        );
    }
}

// ── the pinned top bar ─────────────────────────────────────────────────

pub(super) fn paint_top_bar(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    width: f32,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    cx.backend
        .fill_rect(Rect::xywh(0.0, 0.0, width, HOME_TOPBAR_H), palette.topbar);
    cx.backend.stroke_line(
        Point2D::new(0.0, HOME_TOPBAR_H),
        Point2D::new(width, HOME_TOPBAR_H),
        palette.topbar_line,
        1.0,
    );
    // Brand mark, 26 px, 16 in from the left (no desktop traffic
    // lights own a phone's first column).
    let mark = Rect::xywh(16.0, (HOME_TOPBAR_H - 26.0) / 2.0, 26.0, 26.0);
    if !crate::widgets::login_modal::paint_brand_logo_png(cx.backend, mark) {
        cx.backend
            .fill_round_rect(mark, 6.0, fade(palette.blue, 0.25));
    }
    text_weighted(
        cx,
        "OpenPencil",
        Point2D::new(
            mark.origin.x + mark.size.x + 8.0,
            jian_widgets::centered_text_baseline_y(mark, 17.0),
        ),
        17.0,
        palette.ink,
        700,
    );

    // The 普通 / 专业 segmented control (professional.css `.mode-switch`).
    let switch = layout.mode_switch;
    let normal = layout.mode_normal;
    let professional = layout.professional;
    let normal_pressed = surface.state.pressed == Some(HomeHit::ModeNormal);
    let professional_pressed = surface.state.pressed == Some(HomeHit::Professional);
    cx.backend.fill_round_rect(switch, 10.0, palette.segment_bg);
    cx.backend
        .stroke_round_rect(switch, 10.0, palette.segment_line, 1.0);
    // Home IS the normal mode, so 普通 always paints selected here.
    cx.backend.fill_round_rect(normal, 7.0, palette.panel);
    for (rect, label, on, pressed) in [
        (
            normal,
            copy::home_str(locale, "home.mode.normal"),
            true,
            normal_pressed,
        ),
        (
            professional,
            copy::home_str(locale, "home.mode.professional"),
            false,
            professional_pressed,
        ),
    ] {
        let label_w = cx.backend.measure_text_family(label, 12.0, SANS);
        let color = if on {
            palette.blue
        } else if pressed {
            fade(palette.ink, 0.8)
        } else {
            fade(palette.ink, 0.6)
        };
        text_weighted(
            cx,
            label,
            Point2D::new(
                rect.origin.x + (rect.size.x - label_w) / 2.0,
                jian_widgets::centered_text_baseline_y(rect, 12.0),
            ),
            12.0,
            color,
            if on { 600 } else { 500 },
        );
    }

    // The settings gear (44 pt target, shares NavSettings).
    let gear = layout.settings;
    let gear_pressed = surface.state.pressed == Some(HomeHit::NavSettings);
    if gear_pressed {
        cx.backend.fill_round_rect(gear, 12.0, palette.button_hover);
    }
    draw_icon(
        cx.backend,
        Icon::SlidersHorizontal,
        Point2D::new(
            gear.origin.x + (gear.size.x - 20.0) / 2.0,
            gear.origin.y + (gear.size.y - 20.0) / 2.0,
        ),
        20.0,
        fade(palette.ink, 0.8),
        1.7,
    );
}
