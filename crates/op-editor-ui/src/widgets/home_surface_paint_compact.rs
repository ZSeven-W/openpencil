//! The compact (phone) paint pass for the Studio Home surface: hero,
//! 2×4 task grid, compact composer, ONE featured example, the pinned
//! bottom nav, and the 普通 / 专业 top bar switch. Shares the wide
//! variant's palette, copy adapter, art painters, and entrance
//! choreography; only the metrics differ (prototype `mobile.css`).

use super::super::layout::compact::BOTTOM_NAV_H;
use super::super::{
    copy, fade, model, HomeEnterBlock, HomeLayout, HomeSurface, StudioPalette, HOME_TOPBAR_H,
};
use super::cards::{paint_marker, paint_sticker, text, text_weighted};
use super::sections::fade_all;
use super::{art_phase, enter_phase};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use jian_widgets::components::text_area::TextArea;
use jian_widgets::{Painter, Tokens};
use op_editor_core::{HomeFamily, HomeHit};

const SANS: &str = "system-ui";
const INPUT_FONT: f32 = 14.0;

pub(super) fn paint_home_compact(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect) {
    let layout = surface.layout(rect.size.x, rect.size.y);
    let palette = StudioPalette::for_mode(surface.ui.effective_theme_mode());
    cx.backend.fill_rect(rect, palette.page);
    let shown_at = surface.ui.motion_stamp(surface.state.shown_at_ms);
    let enter = |block| enter_phase(block, shown_at, surface.now_ms);

    // The 作品 page swaps the page column; the pinned chrome stays.
    if surface.works_page() {
        cx.backend.save();
        cx.backend.clip_rect(Rect::xywh(
            0.0,
            HOME_TOPBAR_H,
            rect.size.x,
            (rect.size.y - HOME_TOPBAR_H - BOTTOM_NAV_H).max(0.0),
        ));
        super::super::works::paint_works_page(surface, cx, rect, palette);
        cx.backend.restore();
        paint_bottom_nav(surface, cx, &layout, palette);
        paint_top_bar(surface, cx, &layout, rect.size.x, palette);
        return;
    }

    // A focused composer folds the grid, the example and the bottom nav
    // away (`collapse_for_focused_composer`); their rects are empty.
    let focused = surface.state.composer_focused;
    let bottom_band = if focused { 0.0 } else { BOTTOM_NAV_H };

    // ── the scrolling page column ─────────────────────────────────────
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        0.0,
        HOME_TOPBAR_H,
        rect.size.x,
        (rect.size.y - HOME_TOPBAR_H - bottom_band).max(0.0),
    ));
    paint_hero(
        surface,
        cx,
        &layout,
        enter(HomeEnterBlock::Welcome),
        palette,
    );
    if !focused {
        paint_grid(surface, cx, &layout, enter(HomeEnterBlock::Tabs), palette);
    }
    let (panels_dy, panels_alpha) = enter(HomeEnterBlock::Panels);
    paint_composer(surface, cx, &layout, panels_dy, panels_alpha, palette);
    if !focused {
        paint_example(
            surface,
            cx,
            &layout,
            enter(HomeEnterBlock::Explore),
            palette,
        );
    }
    cx.backend.restore();

    // ── the pinned chrome ─────────────────────────────────────────────
    if !focused {
        paint_bottom_nav(surface, cx, &layout, palette);
    }
    paint_top_bar(surface, cx, &layout, rect.size.x, palette);

    if surface.state.connect_card_open {
        super::super::connect::paint_connect_card(surface, cx, &layout, palette);
    }
}

fn shift(rect: Rect, dy: f32) -> Rect {
    Rect::xywh(rect.origin.x, rect.origin.y + dy, rect.size.x, rect.size.y)
}

// ── hero ───────────────────────────────────────────────────────────────

fn paint_hero(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    (dy, alpha): (f32, f32),
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let palette = fade_all(palette, alpha);
    let lead = copy::home_str(locale, "home.welcome.titleLead");
    let marked = copy::home_str(locale, "home.welcome.titleMarked");
    // Measure at the weight the title is drawn with, and place the marked
    // word by the whole line's advance: a regular-weight measure of the lead
    // alone came up short (and may drop its trailing space), so in Russian
    // the two words ran together.
    let full_w =
        cx.backend
            .measure_text_family_styled(&format!("{lead}{marked}"), 27.0, SANS, 720, false);
    let marked_w = cx
        .backend
        .measure_text_family_styled(marked, 27.0, SANS, 720, false);
    let lead_w = (full_w - marked_w).max(0.0);
    let baseline = layout.welcome.origin.y + dy + 28.0;
    text_weighted(
        cx,
        lead,
        Point2D::new(layout.welcome.origin.x, baseline),
        27.0,
        palette.ink,
        720,
    );
    let marked_x = layout.welcome.origin.x + lead_w;
    paint_marker(cx, marked_x, baseline, marked_w, palette);
    text_weighted(
        cx,
        marked,
        Point2D::new(marked_x, baseline),
        27.0,
        palette.ink,
        720,
    );
    text(
        cx,
        copy::home_str(locale, "home.welcome.sub"),
        Point2D::new(
            layout.welcome_sub.origin.x,
            layout.welcome_sub.origin.y + dy + 13.0,
        ),
        12.0,
        palette.sub,
    );
}

// ── the 2×4 task grid ──────────────────────────────────────────────────

fn paint_grid(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    (dy, alpha): (f32, f32),
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let palette = fade_all(palette, alpha);
    for (index, family) in HomeFamily::ALL.into_iter().enumerate() {
        paint_tile(
            surface,
            cx,
            shift(layout.tabs[index], dy),
            family,
            copy::task_icon(family),
            copy::task_copy(locale, family, surface.state.draft_for(family)).name,
            false,
            palette,
        );
    }
    // The weakened 空白画布 eighth tile (prototype `.mobile-task.blank`).
    paint_tile(
        surface,
        cx,
        shift(layout.new_canvas, dy),
        HomeFamily::AppUi,
        Icon::Plus,
        copy::home_str(locale, "home.recent.newCanvas"),
        true,
        palette,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_tile(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    family: HomeFamily,
    icon: Icon,
    label: &str,
    weakened: bool,
    palette: StudioPalette,
) {
    if rect.size.x <= 0.0 {
        return;
    }
    let selected = !weakened && surface.state.task == family;
    let hovered = !weakened && surface.state.hover == Some(HomeHit::Tab(family));
    // Radius 12 shell: transparent at rest, #EDF4FF + #D3E3FF selected.
    if selected {
        cx.backend
            .fill_round_rect(rect, 12.0, palette.tab_selected_fill);
        cx.backend
            .stroke_round_rect(rect, 12.0, palette.tab_selected_line, 1.0);
    } else if hovered {
        cx.backend
            .fill_round_rect(rect, 12.0, palette.tab_hover_fill);
    }
    let color = if weakened {
        fade(palette.ink, 0.55)
    } else if selected {
        palette.tab_selected_ink
    } else {
        fade(palette.ink, 0.75)
    };
    // Icon 22 over label 11, stacked with the prototype's 6 px gap.
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            rect.origin.x + (rect.size.x - 22.0) / 2.0,
            rect.origin.y + 9.0,
        ),
        22.0,
        color,
        1.6,
    );
    let label_w = cx.backend.measure_text_family(label, 11.0, SANS);
    text_weighted(
        cx,
        label,
        Point2D::new(
            rect.origin.x + (rect.size.x - label_w).max(0.0) / 2.0,
            rect.origin.y + 40.0,
        ),
        11.0,
        color,
        if selected { 600 } else { 500 },
    );
}

// ── the compact composer ───────────────────────────────────────────────

fn studio_tokens(palette: StudioPalette) -> Tokens {
    let mut tokens = Tokens::light();
    tokens.foreground = palette.ink;
    tokens.muted_foreground = palette.placeholder;
    tokens.primary = palette.blue;
    tokens
}

fn paint_composer(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    rise: f32,
    alpha: f32,
    palette: StudioPalette,
) {
    let palette = fade_all(palette, alpha);
    let locale = surface.ui.locale;
    let composer = shift(layout.composer, rise);
    // Radius 17 card with the prototype's soft shadow.
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            composer.origin.x + 4.0,
            composer.origin.y + 7.0,
            composer.size.x - 8.0,
            composer.size.y - 5.0,
        ),
        10.0,
        12.0,
        fade(palette.ink, 0.07),
    );
    cx.backend.fill_round_rect(composer, 17.0, palette.panel);
    cx.backend
        .stroke_round_rect(composer, 17.0, palette.line, 1.0);

    // Label + segmented options on one 44 px row.
    let task_copy = copy::task_copy(locale, surface.state.task, surface.state.task_draft());
    let label_row = shift(layout.label_row, rise);
    text_weighted(
        cx,
        task_copy.label,
        Point2D::new(
            label_row.origin.x,
            jian_widgets::centered_text_baseline_y(label_row, 13.0),
        ),
        13.0,
        palette.ink,
        600,
    );
    let labels = copy::segment_labels(locale, surface.state.task);
    let selected = copy::segment_index(surface.state.task, surface.state.task_draft());
    let segment = shift(layout.segment, rise);
    if segment.size.x > 0.0 {
        cx.backend.fill_round_rect(segment, 8.0, palette.segment_bg);
        for (index, label) in labels.iter().enumerate() {
            let option = shift(layout.segment_options[index], rise);
            let on = index as u8 == selected;
            if on {
                cx.backend.fill_round_rect(option, 6.0, palette.blue_soft);
            }
            let label_w = cx.backend.measure_text_family(label, 11.0, SANS);
            text_weighted(
                cx,
                label,
                Point2D::new(
                    option.origin.x + (option.size.x - label_w) / 2.0,
                    jian_widgets::centered_text_baseline_y(option, 11.0),
                ),
                11.0,
                if on {
                    palette.blue
                } else {
                    fade(palette.ink, 0.58)
                },
                if on { 600 } else { 400 },
            );
        }
    }

    // The 85 px input box.
    let input_box = shift(layout.input_box, rise);
    let engaged =
        !surface.state.draft.is_empty() || matches!(surface.state.hover, Some(HomeHit::Sheet));
    cx.backend
        .fill_round_rect(input_box, 10.0, palette.surface_input);
    cx.backend.stroke_round_rect(
        input_box,
        10.0,
        if engaged {
            Color::rgb_u8(0x74, 0xA7, 0xFF)
        } else {
            palette.input_line
        },
        1.0,
    );
    let text_rect = Rect::xywh(
        input_box.origin.x + 1.0,
        input_box.origin.y + 7.0,
        input_box.size.x - 2.0,
        input_box.size.y - 14.0,
    );
    let text_area = TextArea {
        state: &surface.state.input,
        placeholder: task_copy.placeholder,
        // Phones dismiss the software keyboard when the composer blurs, so
        // the caret must go with it. Desktop Home keeps the surface-level
        // reading (it IS the composer) in `home_surface_paint_panels.rs`.
        focused: surface.state.composer_focused,
        font_size: INPUT_FONT,
        now_ms: surface.now_ms,
        pad_x: 10.0,
        max_visible_lines: 4,
    };
    let mut backend = crate::widgets::text_input_backend::BaselineAdjustingBackend {
        inner: cx.backend,
        baseline_delta_y: 12.0,
    };
    backend.save();
    backend.clip_round_rect(input_box, 10.0);
    text_area.paint(&mut backend, text_rect, &studio_tokens(palette));
    backend.restore();

    // The inline replace-confirm strip rides above the input's bottom.
    if surface.state.replace_pending {
        let strip = shift(layout.replace_strip, rise);
        cx.backend.fill_round_rect(strip, 8.0, palette.raised);
        cx.backend
            .stroke_round_rect(strip, 8.0, fade(palette.blue, 0.4), 1.0);
        text(
            cx,
            copy::home_str(locale, "home.replace.title"),
            Point2D::new(
                strip.origin.x + 10.0,
                jian_widgets::centered_text_baseline_y(strip, 11.0),
            ),
            11.0,
            palette.ink,
        );
        for (rect, _hit, label_key, primary) in [
            (
                layout.replace_keep,
                HomeHit::ReplaceKeep,
                "home.replace.keep",
                false,
            ),
            (
                layout.replace_use,
                HomeHit::ReplaceConfirm,
                "home.replace.use",
                true,
            ),
        ] {
            let button = shift(rect, rise);
            let label = copy::home_str(locale, label_key);
            cx.backend.fill_round_rect(
                button,
                7.0,
                if primary {
                    palette.blue
                } else {
                    palette.raised
                },
            );
            if !primary {
                cx.backend
                    .stroke_round_rect(button, 7.0, palette.raised_line, 1.0);
            }
            let label_w = cx.backend.measure_text_family(label, 11.0, SANS);
            text(
                cx,
                label,
                Point2D::new(
                    button.origin.x + (button.size.x - label_w) / 2.0,
                    jian_widgets::centered_text_baseline_y(button, 11.0),
                ),
                11.0,
                if primary { Color::WHITE } else { palette.ink },
            );
        }
    }

    // Tools: 加图片 / 加链接 (17 px icons, 11 px labels).
    for (rect, hit, label, icon) in [
        (
            layout.screenshot,
            HomeHit::Attachment,
            copy::home_str(locale, "home.tools.screenshot"),
            Icon::Image,
        ),
        (
            layout.reference_link,
            HomeHit::ReferenceLink,
            copy::home_str(locale, "home.tools.link"),
            Icon::Link,
        ),
    ] {
        let rect = shift(rect, rise);
        let hovered = surface.state.hover == Some(hit);
        let color = if hovered {
            palette.blue
        } else {
            fade(palette.ink, 0.72)
        };
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(rect.origin.x, rect.origin.y + (rect.size.y - 17.0) / 2.0),
            17.0,
            color,
            1.6,
        );
        text(
            cx,
            label,
            Point2D::new(
                rect.origin.x + 22.0,
                jian_widgets::centered_text_baseline_y(rect, 11.0),
            ),
            11.0,
            color,
        );
    }

    // Submit: model chip + 开始设计.
    model::paint_model_chip(surface, cx, shift(layout.model_chip, rise), palette);
    let send = shift(layout.send, rise);
    // Same rule as the wide composer: an empty box starts from the
    // example, so the slab is always live (see `HomeSendMode`).
    let mode = surface.send_mode();
    let connect_mode = mode == op_editor_core::HomeSendMode::Connect;
    let send_hover = surface.state.hover == Some(HomeHit::Send);
    let fill = if !connect_mode && send_hover {
        palette.blue_hover
    } else {
        palette.blue
    };
    cx.backend.fill_round_rect(send, 10.0, fill);
    let send_ink = Color::WHITE;
    let send_label = copy::home_str(locale, copy::send_label_key(mode));
    let label_size = copy::fit_label_size(cx.backend, send_label, 13.0, send.size.x - 12.0 - 24.0);
    let label_w = cx.backend.measure_text_family(send_label, label_size, SANS);
    text_weighted(
        cx,
        send_label,
        Point2D::new(
            send.origin.x + (send.size.x - label_w - 16.0 - 8.0) / 2.0,
            jian_widgets::centered_text_baseline_y(send, label_size),
        ),
        label_size,
        send_ink,
        550,
    );
    draw_icon(
        cx.backend,
        Icon::ArrowRight,
        Point2D::new(
            send.origin.x + (send.size.x - label_w - 16.0 - 8.0) / 2.0 + label_w + 8.0,
            send.origin.y + (send.size.y - 16.0) / 2.0,
        ),
        16.0,
        send_ink,
        2.0,
    );
    if surface.send_example_hint_visible() {
        super::panels::paint_send_example_hint(surface, cx, send, palette);
    }
}

// ── the featured example ───────────────────────────────────────────────

fn paint_example(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    (dy, alpha): (f32, f32),
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let palette = fade_all(palette, alpha);
    // Section heading (prototype `.section-heading h2`, 14 px).
    let heading = shift(layout.explore_heading, dy);
    text_weighted(
        cx,
        copy::home_str(locale, "home.explore.title"),
        Point2D::new(
            heading.origin.x,
            jian_widgets::centered_text_baseline_y(heading, 14.0),
        ),
        14.0,
        palette.ink,
        650,
    );

    let card = shift(layout.preview, dy);
    let task_copy = copy::task_copy(locale, surface.state.task, surface.state.task_draft());
    // The card is one tap target (use_example == preview), filled with
    // the prototype's #EEF4FF example tint.
    let hovered = surface.state.hover == Some(HomeHit::UseExample)
        || surface.state.hover == Some(HomeHit::BackToWorkspace);
    cx.backend.fill_round_rect(card, 14.0, palette.preview);
    cx.backend
        .stroke_round_rect(card, 14.0, palette.preview_line, 1.0);
    if hovered {
        cx.backend
            .stroke_round_rect(card, 14.0, fade(palette.blue, 0.35), 1.0);
    }

    // Left text column: sticker, title, desc, 用这个开始.
    let column = shift(layout.preview_heading, dy);
    paint_sticker(
        cx,
        Point2D::new(column.origin.x + 58.0, column.origin.y),
        copy::home_str(locale, "home.preview.sticker"),
        palette,
    );
    // The 22 px sticker sits on the column top; the title's baseline clears
    // it (at +30 its glyphs ran into the sticker), and both lines stop at the
    // column edge instead of running under the art on the right.
    let title = crate::util::ellipsize_to_width(task_copy.example_title, column.size.x, |s| {
        cx.backend.measure_text_family(s, 14.0, SANS)
    });
    text_weighted(
        cx,
        &title,
        Point2D::new(column.origin.x, column.origin.y + 40.0),
        14.0,
        palette.ink,
        650,
    );
    let desc = crate::util::ellipsize_to_width(task_copy.example_desc, column.size.x, |s| {
        cx.backend.measure_text_family(s, 10.0, SANS)
    });
    text(
        cx,
        &desc,
        Point2D::new(column.origin.x, column.origin.y + 58.0),
        10.0,
        palette.preview_desc,
    );
    let use_row = shift(layout.preview_footer, dy);
    let back_to_workspace = surface.ui.workspace.active;
    let use_label = if back_to_workspace {
        copy::home_str(locale, "workspace.backToWorkspace")
    } else {
        copy::home_str(locale, "home.preview.use")
    };
    let use_color = if hovered {
        palette.blue_hover
    } else {
        palette.blue
    };
    let label_w = cx.backend.measure_text_family(use_label, 10.0, SANS);
    text(
        cx,
        use_label,
        Point2D::new(
            use_row.origin.x,
            jian_widgets::centered_text_baseline_y(use_row, 10.0),
        ),
        10.0,
        use_color,
    );
    draw_icon(
        cx.backend,
        Icon::ArrowUpRight,
        Point2D::new(
            use_row.origin.x + label_w + 3.0,
            use_row.origin.y + (use_row.size.y - 12.0) / 2.0,
        ),
        12.0,
        use_color,
        1.6,
    );

    // Right art area with the shared art-in switch motion.
    let art = shift(layout.preview_art, dy);
    let phase = art_phase(
        surface.ui.motion_stamp(surface.state.art_switched_at_ms),
        surface.now_ms,
    );
    let art = shift(art, (1.0 - phase) * 6.0);
    cx.backend.save();
    cx.backend.clip_round_rect(art, 8.0);
    let (template, aspect) = super::cards::task_art(surface);
    super::cards::paint_template_paper(cx, art, template, palette, aspect, 0.2 + 0.8 * phase);
    cx.backend.restore();
}

#[path = "home_surface_paint_compact_chrome.rs"]
mod chrome;
use chrome::{paint_bottom_nav, paint_top_bar};
