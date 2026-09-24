//! Immediate-mode paint pass for the Studio Home surface: the pinned
//! top bar, the welcome headline with its yellow marker, the task tab
//! row, the two panels (`home_surface_paint_panels.rs`), the 更多
//! popover, and the two lower sections (`home_surface_paint_sections.rs`:
//! the explore cards and the 最近项目 row).

use super::copy::{self, SANS};
use super::paint::cards::{paint_marker, text, text_weighted};
use super::palette::fade;
use super::{
    layout::tab_copy_gap, HomeEnterBlock, HomeLayout, HomeSurface, StudioPalette, HOME_TOPBAR_H,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::HomeHit;

/// cubic-bezier(.22,1,.36,1) — the prototype's `--ease`, which is
/// ease-out-quint to within a pixel over the 600 ms windows used here.
pub(super) fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}

/// One block's entrance phase at `now_ms`: `(rise_offset_y, alpha)`.
/// The wide-variant choreography (prototype `enter .6s var(--ease)`):
/// welcome / tabs / panels / explore / recent stagger 0/60/120/180/240
/// ms, each fading 0→1 and rising 10 px over 600 ms. A
/// `shown_at_ms` of 0 means "not started" — the surface paints
/// settled (`t = 1`). Blocks paint `dy` px BELOW their final rect.
pub(super) fn enter_phase(block: HomeEnterBlock, shown_at_ms: u64, now_ms: u64) -> (f32, f32) {
    if shown_at_ms == 0 {
        return (0.0, 1.0);
    }
    const STAGGER_MS: u64 = 60;
    const RISE_MS: u64 = 600;
    const RISE_PX: f32 = 10.0;
    let index = match block {
        HomeEnterBlock::Welcome => 0,
        HomeEnterBlock::Tabs => 1,
        HomeEnterBlock::Panels => 2,
        HomeEnterBlock::Explore => 3,
        HomeEnterBlock::Recent => 4,
    };
    let start = shown_at_ms + index * STAGGER_MS;
    let t = (now_ms.saturating_sub(start)).min(RISE_MS) as f32 / RISE_MS as f32;
    let eased = ease_out_quint(t);
    ((1.0 - eased) * RISE_PX, eased)
}

/// The example-art switch phase at `now_ms` (0 → 1 over
/// [`op_editor_core::HOME_ART_SWITCH_MS`], quint-eased like the
/// prototype's `art-in` curve); 1 when nothing is switching. Paint maps
/// it to opacity .2→1, rise 8 px and scale .985→1.
pub(super) fn art_phase(switched_at_ms: u64, now_ms: u64) -> f32 {
    if switched_at_ms == 0 {
        return 1.0;
    }
    let window = op_editor_core::HOME_ART_SWITCH_MS;
    ease_out_quint(now_ms.saturating_sub(switched_at_ms).min(window) as f32 / window as f32)
}

/// The explore card's hover-lift phase, eased 0 → 1 over
/// [`op_editor_core::editor_ui_state::home::HOME_HOVER_LIFT_MS`]. A
/// `since_ms` of 0 means the host never stamped the hover change — the
/// phase settles at 1 so an unstamped hover degrades to an instant
/// lift instead of freezing mid-rise.
pub(super) fn hover_lift_phase(since_ms: u64, now_ms: u64) -> f32 {
    if since_ms == 0 {
        return 1.0;
    }
    let window = op_editor_core::editor_ui_state::home::HOME_HOVER_LIFT_MS;
    ease_out_quint(now_ms.saturating_sub(since_ms).min(window) as f32 / window as f32)
}

/// The explore card's hover-lift offset at `now_ms`, working in BOTH
/// directions from one stamp: a hovered card rises to −4 px, a card
/// the cursor just left descends from −4 px back to rest (prototype
/// `.example-card{transition:transform .3s}` → `translateY(-4px)`).
pub(super) fn hover_lift_dy(hovered: bool, since_ms: u64, now_ms: u64) -> f32 {
    let phase = hover_lift_phase(since_ms, now_ms);
    if hovered {
        -4.0 * phase
    } else {
        -4.0 + 4.0 * phase
    }
}

/// Letter-spaced text (per-glyph advance + `spacing`).
fn spaced_text(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    color: Color,
    weight: u16,
    spacing: f32,
) {
    let mut x = origin.x;
    for character in content.chars() {
        let glyph = character.to_string();
        text_weighted(cx, &glyph, Point2D::new(x, origin.y), size, color, weight);
        x += cx.backend.measure_text_family(&glyph, size, SANS) + spacing;
    }
}

fn spaced_width(cx: &mut PaintCx<'_>, content: &str, size: f32, spacing: f32) -> f32 {
    content
        .chars()
        .map(|character| {
            cx.backend
                .measure_text_family(&character.to_string(), size, SANS)
                + spacing
        })
        .sum::<f32>()
        - spacing
}

pub(super) fn paint_home(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect) {
    // The phone (touch + compact) composition is a separate metric set
    // over the same state — see `home_surface_paint_compact.rs`.
    if surface.ui.compact_layout() {
        compact::paint_home_compact(surface, cx, rect);
        return;
    }
    let layout = surface.layout(rect.size.x, rect.size.y);
    let palette = StudioPalette::for_mode(surface.ui.effective_theme_mode());
    cx.backend.fill_rect(rect, palette.page);
    let shown_at = surface.ui.motion_stamp(surface.state.shown_at_ms);
    let enter = |block| enter_phase(block, shown_at, surface.now_ms);

    // ── the scrolling page column ─────────────────────────────────────
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        0.0,
        HOME_TOPBAR_H,
        rect.size.x,
        (rect.size.y - HOME_TOPBAR_H).max(0.0),
    ));
    paint_welcome(
        surface,
        cx,
        &layout,
        enter(HomeEnterBlock::Welcome),
        palette,
    );
    paint_tabs(
        surface,
        cx,
        &layout,
        rect.size.x,
        enter(HomeEnterBlock::Tabs),
        palette,
    );
    let (panels_dy, panels_alpha) = enter(HomeEnterBlock::Panels);
    super::paint::panels::paint_composer(surface, cx, &layout, panels_dy, panels_alpha);
    super::paint::panels::paint_preview(surface, cx, &layout, panels_dy, panels_alpha);
    if surface.state.more_open {
        paint_more_popover(surface, cx, &layout, palette);
    }
    sections::paint_explore(
        surface,
        cx,
        &layout,
        enter(HomeEnterBlock::Explore),
        palette,
    );
    sections::paint_recent(surface, cx, &layout, enter(HomeEnterBlock::Recent), palette);
    cx.backend.restore();

    // ── the pinned top bar ────────────────────────────────────────────
    paint_top_bar(surface, cx, &layout, rect.size.x, palette);

    if surface.state.connect_card_open {
        super::connect::paint_connect_card(surface, cx, &layout, palette);
    }
    super::focus::paint_home_focus_ring(surface, cx, &layout, palette);
}

fn paint_top_bar(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    width: f32,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    // The bar and its hairline span the window, not the content column —
    // sized to the column they stopped short under the right-hand actions.
    cx.backend
        .fill_rect(Rect::xywh(0.0, 0.0, width, HOME_TOPBAR_H), palette.topbar);
    cx.backend.stroke_line(
        Point2D::new(0.0, HOME_TOPBAR_H),
        Point2D::new(width, HOME_TOPBAR_H),
        palette.topbar_line,
        1.0,
    );
    // Official mark, 34×34 (the PNG carries internal padding, so its
    // layout box overshoots the visible glyph). 80 px in from the left:
    // the macOS traffic lights own the first ~70 px of a borderless
    // window, so the prototype's 30 px would put the mark under them.
    let mark = Rect::xywh(80.0, (HOME_TOPBAR_H - 34.0) / 2.0, 34.0, 34.0);
    if !crate::widgets::login_modal::paint_brand_logo_png(cx.backend, mark) {
        cx.backend
            .fill_round_rect(mark, 7.0, fade(palette.blue, 0.25));
    }
    // OpenPencil, 20 px w700, −0.7 tracking.
    let name_x = mark.origin.x + mark.size.x + 10.0;
    let baseline = jian_widgets::centered_text_baseline_y(mark, 20.0);
    spaced_text(
        cx,
        "OpenPencil",
        Point2D::new(name_x, baseline),
        20.0,
        palette.ink,
        700,
        -0.7,
    );
    let name_w = spaced_width(cx, "OpenPencil", 20.0, -0.7);
    let divider_x = name_x + name_w + 6.0;
    cx.backend.stroke_line(
        Point2D::new(divider_x, mark.origin.y + 8.5),
        Point2D::new(divider_x, mark.origin.y + 25.5),
        palette.divider,
        1.0,
    );
    text(
        cx,
        copy::home_str(locale, "home.topbar.context"),
        Point2D::new(divider_x + 14.0, baseline),
        13.0,
        palette.context,
    );
    // The account avatar, painted through the professional TopBar's own
    // painter so the two entry points cannot drift into different
    // avatars. Home is a first-run surface: making the way in to an
    // account reachable only from the professional canvas hid it behind
    // the one screen a new user has no reason to open.
    if surface.ui.account_ui_available {
        let hovered = surface.state.hover == Some(HomeHit::Account);
        let pressed = surface.state.pressed == Some(HomeHit::Account);
        crate::widgets::top_bar_paint::paint_account_button(
            cx,
            &surface.theme,
            &surface.ui.account,
            layout.account.origin.x,
            layout.account.origin.y + layout.account.size.y / 2.0,
            hovered,
            pressed,
        );
    }
    // The two 38 px outline buttons.
    for (button, hit, label, icon) in [
        (
            layout.open_file,
            HomeHit::OpenFile,
            copy::home_str(locale, "home.topbar.openFile"),
            Icon::FolderOpen,
        ),
        (
            layout.professional,
            HomeHit::Professional,
            copy::home_str(locale, "home.topbar.professional"),
            Icon::ArrowUpRight,
        ),
    ] {
        let hovered = surface.state.hover == Some(hit);
        let pressed = surface.state.pressed == Some(hit);
        cx.backend.fill_round_rect(
            button,
            9.0,
            if pressed || hovered {
                palette.button_hover
            } else {
                palette.raised
            },
        );
        cx.backend.stroke_round_rect(
            button,
            9.0,
            if hovered {
                palette.button_hover_line
            } else {
                palette.raised_line
            },
            1.0,
        );
        let label_w = cx.backend.measure_text_family(label, 15.0, SANS);
        let icon_x = button.origin.x + (button.size.x - label_w - 17.0 - 9.0) / 2.0;
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(icon_x, button.origin.y + (button.size.y - 17.0) / 2.0),
            17.0,
            fade(palette.ink, 0.8),
            1.6,
        );
        text(
            cx,
            label,
            Point2D::new(
                icon_x + 17.0 + 9.0,
                jian_widgets::centered_text_baseline_y(button, 15.0),
            ),
            15.0,
            palette.ink,
        );
    }
}

fn paint_welcome(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    (dy, alpha): (f32, f32),
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let palette = sections::fade_all(palette, alpha);
    let lead = copy::home_str(locale, "home.welcome.titleLead");
    let marked = copy::home_str(locale, "home.welcome.titleMarked");
    let lead_w = cx.backend.measure_text_family(lead, 36.0, SANS);
    let marked_w = cx.backend.measure_text_family(marked, 36.0, SANS);
    let baseline = layout.welcome.origin.y + dy + 34.0;
    text_weighted(
        cx,
        lead,
        Point2D::new(layout.welcome.origin.x, baseline),
        36.0,
        palette.ink,
        760,
    );
    let marked_x = layout.welcome.origin.x + lead_w;
    // The marker band and its rays go down FIRST: the prototype seats
    // them behind the glyphs (`.marker{z-index:-1}`), and painting them
    // after the text let an opaque yellow band cover the lower half of
    // 做点什么 instead of highlighting it.
    paint_marker(cx, marked_x, baseline, marked_w, palette);
    text_weighted(
        cx,
        marked,
        Point2D::new(marked_x, baseline),
        36.0,
        palette.ink,
        760,
    );
    text(
        cx,
        copy::home_str(locale, "home.welcome.sub"),
        Point2D::new(
            layout.welcome_sub.origin.x,
            layout.welcome_sub.origin.y + dy + 18.0,
        ),
        16.0,
        palette.sub,
    );
}

#[allow(clippy::too_many_lines)]
fn paint_tabs(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    viewport_w: f32,
    (dy, alpha): (f32, f32),
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let palette = sections::fade_all(palette, alpha);
    // Wide tab chrome: radius 10, icon 22, icon→copy gap 13 (10 on
    // viewports ≤1180, where the prototype also drops the padding).
    let copy_gap = tab_copy_gap(viewport_w);
    let icon_size = 22.0;
    for (index, family) in op_editor_core::HomeFamily::ALL.into_iter().enumerate() {
        let rect = layout.tabs[index];
        if rect.size.x <= 0.0 {
            continue;
        }
        let rect = Rect::xywh(rect.origin.x, rect.origin.y + dy, rect.size.x, rect.size.y);
        let task = copy::task_copy(locale, family, surface.state.draft_for(family));
        let selected = surface.state.task == family;
        let hovered = surface.state.hover == Some(HomeHit::Tab(family));
        paint_tab_shell(cx, rect, palette, hovered, selected);
        // Icon + two-line copy stack, centred as one group.
        let name_w = cx.backend.measure_text_family(task.name, 14.0, SANS);
        let summary_w = cx.backend.measure_text_family(task.summary, 11.0, SANS);
        let stack_w = name_w.max(summary_w);
        let icon_x = rect.origin.x + (rect.size.x - icon_size - copy_gap - stack_w) / 2.0;
        let icon_y = rect.origin.y + (rect.size.y - icon_size) / 2.0;
        draw_icon(
            cx.backend,
            copy::task_icon(family),
            Point2D::new(icon_x, icon_y),
            icon_size,
            if selected {
                palette.tab_selected_ink
            } else {
                fade(palette.ink, 0.75)
            },
            1.6,
        );
        let copy_x = icon_x + icon_size + copy_gap;
        text_weighted(
            cx,
            task.name,
            Point2D::new(copy_x, rect.origin.y + 24.0),
            14.0,
            if selected {
                palette.tab_selected_ink
            } else {
                palette.ink
            },
            if selected { 650 } else { 500 },
        );
        text(
            cx,
            task.summary,
            Point2D::new(copy_x, rect.origin.y + 44.0),
            11.0,
            palette.muted,
        );
        if selected {
            // The 2 px blue bar along the tab's bottom edge, inset 14 px
            // and sunk 1 px past it (wide `[aria-selected]:after`).
            cx.backend.fill_round_rect(
                Rect::xywh(
                    rect.origin.x + 14.0,
                    rect.origin.y + rect.size.y - 1.0,
                    rect.size.x - 28.0,
                    2.0,
                ),
                1.0,
                palette.tab_selected_bar,
            );
        }
    }
    // The 更多 ▾ button on compact widths.
    if layout.more_button.size.x > 0.0 {
        let rect = Rect::xywh(
            layout.more_button.origin.x,
            layout.more_button.origin.y + dy,
            layout.more_button.size.x,
            layout.more_button.size.y,
        );
        let hovered = surface.state.hover == Some(HomeHit::More);
        let open = surface.state.more_open;
        paint_tab_shell(cx, rect, palette, hovered, open);
        let label = copy::home_str(locale, "home.tabs.more");
        let label_w = cx.backend.measure_text_family(label, 14.0, SANS);
        let group_w = label_w + 6.0 + 14.0;
        let x = rect.origin.x + (rect.size.x - group_w) / 2.0;
        text_weighted(
            cx,
            label,
            Point2D::new(x, rect.origin.y + 35.0),
            14.0,
            palette.ink,
            500,
        );
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(x + label_w + 6.0, rect.origin.y + 23.0),
            14.0,
            palette.muted,
            1.6,
        );
    }
    // The row's bottom hairline (12 px under the tabs, wide variant).
    let hairline_y = layout.tabs_row.origin.y + dy + layout.tabs_row.size.y;
    cx.backend.stroke_line(
        Point2D::new(layout.tabs_row.origin.x, hairline_y),
        Point2D::new(
            layout.tabs_row.origin.x + layout.tabs_row.size.x,
            hairline_y,
        ),
        palette.tabs_hairline,
        1.0,
    );
}

/// One tab's 60 px rounded shell: white at 49 % at rest with a
/// transparent border, `#F0F5FD` + `#E1EAF7` on hover, `#EAF2FF` +
/// `#C8DBFF` when selected (wide `.task-tab` overrides).
fn paint_tab_shell(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    palette: StudioPalette,
    hovered: bool,
    selected: bool,
) {
    cx.backend.fill_round_rect(
        rect,
        10.0,
        if selected {
            palette.tab_selected_fill
        } else if hovered {
            palette.tab_hover_fill
        } else {
            palette.tab_fill
        },
    );
    if selected {
        cx.backend
            .stroke_round_rect(rect, 10.0, palette.tab_selected_line, 1.0);
    } else if hovered {
        cx.backend
            .stroke_round_rect(rect, 10.0, palette.tab_hover_line, 1.0);
    }
}

fn paint_more_popover(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let popover = layout.more_popover;
    if popover.size.x <= 0.0 {
        return;
    }
    let hidden = HomeLayout::hidden_tasks(layout.tabs_row.size.x, surface.state.task);
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            popover.origin.x + 4.0,
            popover.origin.y + 8.0,
            popover.size.x - 8.0,
            popover.size.y - 8.0,
        ),
        10.0,
        12.0,
        fade(palette.ink, 0.12),
    );
    cx.backend.fill_round_rect(popover, 12.0, palette.raised);
    cx.backend
        .stroke_round_rect(popover, 12.0, palette.raised_line, 1.0);
    text(
        cx,
        copy::home_str(locale, "home.tabs.moreCaption"),
        Point2D::new(popover.origin.x + 12.0, popover.origin.y + 17.0),
        11.0,
        palette.muted,
    );
    for (index, family) in hidden.iter().enumerate().take(4) {
        let row = layout.more_rows[index];
        let task = copy::task_copy(locale, *family, surface.state.draft_for(*family));
        let hovered = surface.state.hover == Some(HomeHit::MoreItem(*family));
        if hovered {
            cx.backend
                .fill_round_rect(row, 8.0, fade(palette.blue_soft, 0.6));
        }
        draw_icon(
            cx.backend,
            copy::task_icon(*family),
            Point2D::new(row.origin.x + 8.0, row.origin.y + (row.size.y - 16.0) / 2.0),
            16.0,
            fade(palette.ink, 0.75),
            1.5,
        );
        text_weighted(
            cx,
            task.name,
            Point2D::new(row.origin.x + 34.0, row.origin.y + 18.0),
            13.0,
            palette.ink,
            500,
        );
        text(
            cx,
            task.summary,
            Point2D::new(row.origin.x + 34.0, row.origin.y + 34.0),
            11.0,
            palette.muted,
        );
        draw_icon(
            cx.backend,
            Icon::ChevronRight,
            Point2D::new(
                row.origin.x + row.size.x - 18.0,
                row.origin.y + (row.size.y - 14.0) / 2.0,
            ),
            14.0,
            palette.muted,
            1.5,
        );
    }
}

#[path = "home_surface_paint_art.rs"]
pub(super) mod art;
#[path = "home_surface_paint_art_copy.rs"]
pub(super) mod art_copy;
#[path = "home_surface_paint_art_screens.rs"]
pub(super) mod art_screens;
#[path = "home_surface_paint_cards.rs"]
pub(super) mod cards;
#[path = "home_surface_paint_compact.rs"]
mod compact;
#[path = "home_surface_paint_explore_copy.rs"]
pub(super) mod explore_copy;
#[path = "home_surface_paint_panels.rs"]
pub(super) mod panels;
#[path = "home_surface_paint_sections.rs"]
pub(super) mod sections;
