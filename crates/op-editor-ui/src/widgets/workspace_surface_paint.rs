//! Immediate-mode paint pass for the generation-workspace chrome: the
//! white header (back circle, doc tile, title + subtitle, 导出 /
//! 专业编辑), the canvas toolbar (chat toggle, view segments, pager,
//! zoom), the dock background + drag handle, the deck strip's
//! placeholder plates. The real canvas and the pinned chat panel are
//! painted by the host between these layers; the canvas banners paint
//! after them (`workspace_surface_banner.rs`).

use super::workspace_enter;
use super::{StudioPalette, WorkspaceLayout, WorkspaceSurface};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::{
    HomeDevice, HomeFamily, InfoKind, WorkspaceHit, WorkspacePhase, WorkspaceView,
    WORKSPACE_TOOLBAR_H,
};

pub(super) const SANS: &str = "system-ui";

pub(super) fn text(cx: &mut PaintCx<'_>, content: &str, origin: Point2D, size: f32, color: Color) {
    let layout = crate::TextLayout::single_run(content, SANS, size, color.to_jian(), Point2D::ZERO);
    cx.backend.draw_text(&layout, origin);
}

fn text_weighted(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    color: Color,
    weight: u16,
) {
    let layout = crate::TextLayout::single_run(content, SANS, size, color.to_jian(), Point2D::ZERO)
        .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}

pub(super) fn fade(color: Color, factor: f32) -> Color {
    Color {
        a: color.a * factor,
        ..color
    }
}

/// Fade every palette token by `factor` (the entrance crossfade).
pub(super) fn fade_all(palette: StudioPalette, factor: f32) -> StudioPalette {
    palette.faded(factor)
}

pub(super) fn tr(locale: op_i18n::Locale, key: &'static str) -> &'static str {
    op_i18n::translate(locale, key)
}

/// The subtitle's options segment: 手机/电脑, 16:9/4:3, 数据/流程/对比.
fn options_label(
    locale: op_i18n::Locale,
    family: HomeFamily,
    options: &op_editor_core::TaskDraft,
) -> &'static str {
    match family {
        HomeFamily::AppUi => {
            if options.device == HomeDevice::Desktop {
                tr(locale, "home.segment.desktop")
            } else {
                tr(locale, "home.segment.mobile")
            }
        }
        HomeFamily::Presentation => match options.ratio {
            op_editor_core::SlideRatio::Wide169 => tr(locale, "home.segment.wide"),
            op_editor_core::SlideRatio::Classic43 => tr(locale, "home.segment.classic"),
        },
        HomeFamily::Infographic => match options.info_kind {
            InfoKind::Data => tr(locale, "home.segment.infoData"),
            InfoKind::Flow => tr(locale, "home.segment.infoFlow"),
            InfoKind::Comparison => tr(locale, "home.segment.infoCompare"),
        },
        _ => "",
    }
}

pub(crate) fn phase_key(phase: WorkspacePhase) -> &'static str {
    match phase {
        WorkspacePhase::Generating => "workspace.phase.generating",
        WorkspacePhase::Done => "workspace.phase.done",
        WorkspacePhase::Stopped => "workspace.phase.stopped",
        WorkspacePhase::Failed => "workspace.phase.failed",
    }
}

/// One family's short name through the shared `home.task.*` keys.
pub(crate) fn family_label(locale: op_i18n::Locale, family: HomeFamily) -> &'static str {
    let key: &'static str = match family {
        HomeFamily::AppUi => "home.task.app.name",
        HomeFamily::Web => "home.task.web.name",
        HomeFamily::Presentation => "home.task.presentation.name",
        HomeFamily::KnowledgeCards => "home.task.knowledge.name",
        HomeFamily::ScreenshotTutorial => "home.task.tutorial.name",
        HomeFamily::Infographic => "home.task.infographic.name",
        HomeFamily::EventPoster => "home.task.poster.name",
    };
    tr(locale, key)
}

/// One view mode's segment label.
fn view_label(locale: op_i18n::Locale, family: HomeFamily, view: WorkspaceView) -> &'static str {
    match view {
        WorkspaceView::AllBoards => tr(locale, "workspace.view.all"),
        WorkspaceView::Single { .. } => {
            if family == HomeFamily::Presentation {
                tr(locale, "workspace.view.singlePage")
            } else {
                tr(locale, "workspace.view.singleScreen")
            }
        }
        WorkspaceView::LongPage => tr(locale, "workspace.view.long"),
        WorkspaceView::Overview => tr(locale, "workspace.view.overview"),
    }
}

pub(super) fn paint_workspace(surface: &WorkspaceSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect) {
    let layout = surface.layout(rect.size.x, rect.size.y);
    let (_, alpha) = workspace_enter(
        surface.ui.motion_stamp(surface.state.shown_at_ms),
        surface.now_ms,
    );
    let palette = fade_all(
        StudioPalette::for_mode(surface.ui.effective_theme_mode()),
        alpha,
    );

    // Dock background under the pinned chat panel + its drag handle.
    if let Some(dock) = layout.dock {
        cx.backend.fill_rect(dock, palette.panel);
        cx.backend.stroke_line(
            Point2D::new(dock.origin.x + dock.size.x, dock.origin.y),
            Point2D::new(dock.origin.x + dock.size.x, dock.origin.y + dock.size.y),
            palette.line,
            1.0,
        );
    }
    if let Some(handle) = layout.dock_handle {
        let hovered = surface.state.hover == Some(WorkspaceHit::DockResize);
        cx.backend.fill_rect(
            handle,
            if hovered {
                palette.blue_soft
            } else {
                palette.panel
            },
        );
        // The grip notch, vertically centered.
        let notch = Rect::xywh(
            handle.origin.x + 1.5,
            handle.origin.y + handle.size.y * 0.46,
            2.0,
            29.0,
        );
        cx.backend
            .fill_round_rect(notch, 1.0, fade(palette.muted, 0.55));
    }

    paint_header(surface, cx, &layout, palette);
    paint_toolbar(surface, cx, &layout, palette);
    paint_strip(surface, cx, &layout, palette);
    // The canvas banners paint over the canvas, not here — see
    // `WorkspaceSurface::paint_canvas_banners`.
}

fn paint_header(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    cx.backend.fill_rect(layout.header, palette.topbar);
    cx.backend.stroke_line(
        Point2D::new(0.0, layout.header.size.y),
        Point2D::new(layout.header.size.x, layout.header.size.y),
        palette.topbar_line,
        1.0,
    );

    // ← back circle.
    let back_hovered = surface.state.hover == Some(WorkspaceHit::Back);
    cx.backend.fill_round_rect(
        layout.back,
        layout.back.size.y / 2.0,
        if back_hovered {
            palette.button_hover
        } else {
            palette.raised
        },
    );
    cx.backend.stroke_round_rect(
        layout.back,
        layout.back.size.y / 2.0,
        if back_hovered {
            palette.button_hover_line
        } else {
            palette.raised_line
        },
        1.0,
    );
    draw_icon(
        cx.backend,
        Icon::ChevronLeft,
        Point2D::new(
            layout.back.origin.x + (layout.back.size.x - 18.0) / 2.0,
            layout.back.origin.y + (layout.back.size.y - 18.0) / 2.0,
        ),
        18.0,
        palette.ink,
        1.8,
    );

    // Doc tile.
    cx.backend
        .fill_round_rect(layout.doc_tile, 8.0, palette.chip_bg);
    cx.backend
        .stroke_round_rect(layout.doc_tile, 8.0, palette.chip_line, 1.0);
    draw_icon(
        cx.backend,
        Icon::Frame,
        Point2D::new(
            layout.doc_tile.origin.x + (layout.doc_tile.size.x - 17.0) / 2.0,
            layout.doc_tile.origin.y + (layout.doc_tile.size.y - 17.0) / 2.0,
        ),
        17.0,
        palette.link,
        1.6,
    );

    // Title + subtitle.
    text_weighted(
        cx,
        &surface.title,
        Point2D::new(layout.title.origin.x, layout.title.origin.y + 17.0),
        15.0,
        palette.ink,
        650,
    );
    let options = options_label(locale, surface.state.family, &surface.state.options);
    let phase = tr(locale, phase_key(surface.state.phase));
    let mut subtitle = family_label(locale, surface.state.family).to_string();
    if !options.is_empty() {
        subtitle.push_str(" · ");
        subtitle.push_str(options);
    }
    subtitle.push_str(" · ");
    subtitle.push_str(phase);
    text(
        cx,
        &subtitle,
        Point2D::new(layout.title.origin.x, layout.title.origin.y + 33.0),
        12.0,
        palette.muted,
    );

    // 质检 chip (finished runs with an audited report only).
    super::quality_paint::paint_quality_chip(surface, cx, layout, palette);

    // 导出 / 专业编辑 outline buttons.
    for (button, hit, label, icon) in [
        (
            layout.export,
            WorkspaceHit::Export,
            tr(locale, "workspace.export"),
            Icon::Download,
        ),
        (
            layout.professional,
            WorkspaceHit::Professional,
            tr(locale, "workspace.professional"),
            Icon::ArrowUpRight,
        ),
    ] {
        let hovered = surface.state.hover == Some(hit);
        let pressed = surface.state.pressed == Some(hit);
        cx.backend.fill_round_rect(
            button,
            8.0,
            if pressed || hovered {
                palette.button_hover
            } else {
                palette.raised
            },
        );
        cx.backend.stroke_round_rect(
            button,
            8.0,
            if hovered {
                palette.button_hover_line
            } else {
                palette.raised_line
            },
            1.0,
        );
        let label_w = cx.backend.measure_text_family(label, 13.0, SANS);
        let icon_x = button.origin.x + (button.size.x - label_w - 15.0 - 8.0) / 2.0;
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(icon_x, button.origin.y + (button.size.y - 15.0) / 2.0),
            15.0,
            fade(palette.ink, 0.85),
            1.5,
        );
        text(
            cx,
            label,
            Point2D::new(
                icon_x + 15.0 + 8.0,
                jian_widgets::centered_text_baseline_y(button, 13.0),
            ),
            13.0,
            palette.ink,
        );
    }
}

fn paint_toolbar(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    cx.backend.fill_rect(layout.toolbar, palette.panel);
    cx.backend.stroke_line(
        Point2D::new(
            layout.toolbar.origin.x,
            layout.toolbar.origin.y + WORKSPACE_TOOLBAR_H,
        ),
        Point2D::new(
            layout.toolbar.origin.x + layout.toolbar.size.x,
            layout.toolbar.origin.y + WORKSPACE_TOOLBAR_H,
        ),
        palette.line,
        1.0,
    );

    // Chat toggle (the only remaining control when the dock collapses).
    let toggle = Rect::xywh(
        layout.toolbar.origin.x + 12.0,
        layout.toolbar.origin.y + (WORKSPACE_TOOLBAR_H - 28.0) / 2.0,
        28.0,
        28.0,
    );
    let toggle_hovered = surface.state.hover == Some(WorkspaceHit::ToggleDock);
    cx.backend.fill_round_rect(
        toggle,
        7.0,
        if toggle_hovered {
            palette.button_hover
        } else {
            fade(palette.panel, 1.0)
        },
    );
    draw_icon(
        cx.backend,
        Icon::PanelLeft,
        Point2D::new(toggle.origin.x + 5.0, toggle.origin.y + 5.0),
        18.0,
        if surface.ui.chat_pinned() {
            palette.link
        } else {
            palette.muted
        },
        1.7,
    );

    // View segments.
    let views = super::family_views(surface.state.family);
    for (index, segment) in layout.view_segments.iter().enumerate() {
        let Some(view) = views.get(index).copied() else {
            continue;
        };
        let selected = surface.state.view == view
            || matches!(
                (surface.state.view, view),
                (WorkspaceView::Single { .. }, WorkspaceView::Single { .. })
            );
        let hovered = surface.state.hover == Some(WorkspaceHit::View(view));
        cx.backend.fill_round_rect(
            *segment,
            6.0,
            if selected {
                palette.blue_soft
            } else if hovered {
                palette.button_hover
            } else {
                palette.segment_bg
            },
        );
        let label = view_label(locale, surface.state.family, view);
        text(
            cx,
            label,
            Point2D::new(
                segment.origin.x,
                jian_widgets::centered_text_baseline_y(*segment, 12.0),
            ),
            12.0,
            if selected {
                palette.link
            } else {
                palette.muted
            },
        );
    }

    // Pager (prev / next) with a position label between when present.
    if let (Some(prev), Some(next)) = (layout.prev, layout.next) {
        let boards = surface.boards.len();
        let label = if boards == 0 {
            "0 / 0".to_string()
        } else {
            format!("{} / {}", surface.state.selected + 1, boards)
        };
        let label_w = cx.backend.measure_text_family(&label, 11.0, SANS);
        let between_w = next.origin.x - prev.origin.x - prev.size.x;
        let label_x = prev.origin.x + prev.size.x + (between_w - label_w) / 2.0;
        text(
            cx,
            &label,
            Point2D::new(label_x, jian_widgets::centered_text_baseline_y(prev, 11.0)),
            11.0,
            palette.muted,
        );
        paint_toolbar_icon(
            cx,
            surface,
            prev,
            WorkspaceHit::Prev,
            Icon::ChevronLeft,
            palette,
            surface.state.selected > 0,
        );
        paint_toolbar_icon(
            cx,
            surface,
            next,
            WorkspaceHit::Next,
            Icon::ChevronRight,
            palette,
            surface.state.selected + 1 < boards,
        );
    }

    // Zoom cluster.
    paint_toolbar_icon(
        cx,
        surface,
        layout.zoom_out,
        WorkspaceHit::ZoomOut,
        Icon::Minus,
        palette,
        true,
    );
    paint_toolbar_icon(
        cx,
        surface,
        layout.zoom_in,
        WorkspaceHit::ZoomIn,
        Icon::Plus,
        palette,
        true,
    );
    let fit_hovered = surface.state.hover == Some(WorkspaceHit::ZoomFit);
    cx.backend.fill_round_rect(
        layout.zoom_fit,
        6.0,
        if fit_hovered {
            palette.button_hover
        } else {
            fade(palette.panel, 1.0)
        },
    );
    text(
        cx,
        tr(locale, "workspace.fit"),
        Point2D::new(
            layout.zoom_fit.origin.x,
            jian_widgets::centered_text_baseline_y(layout.zoom_fit, 11.0),
        ),
        11.0,
        palette.sub,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_toolbar_icon(
    cx: &mut PaintCx<'_>,
    surface: &WorkspaceSurface<'_>,
    rect: Rect,
    hit: WorkspaceHit,
    icon: Icon,
    palette: StudioPalette,
    enabled: bool,
) {
    let hovered = surface.state.hover == Some(hit);
    cx.backend.fill_round_rect(
        rect,
        6.0,
        if hovered && enabled {
            palette.button_hover
        } else {
            fade(palette.panel, 1.0)
        },
    );
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 5.0, rect.origin.y + 5.0),
        rect.size.x - 10.0,
        if enabled {
            palette.sub
        } else {
            fade(palette.muted, 0.4)
        },
        1.7,
    );
}

fn paint_strip(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let Some(strip) = layout.strip else {
        return;
    };
    let locale = surface.ui.locale;
    cx.backend.fill_rect(strip, palette.topbar);
    cx.backend.stroke_line(
        Point2D::new(strip.origin.x, strip.origin.y),
        Point2D::new(strip.origin.x + strip.size.x, strip.origin.y),
        palette.line,
        1.0,
    );

    // 总览 / 放映 read as two more cells of the thumbnail row: a plate
    // the same height as a thumbnail with a centred caption underneath.
    let overview_selected = surface.state.view == WorkspaceView::Overview;
    paint_strip_tile(
        cx,
        layout.overview,
        Icon::LayoutGrid,
        tr(locale, "workspace.view.overview"),
        palette,
        StripTileState {
            selected: overview_selected,
            hovered: surface.state.hover == Some(WorkspaceHit::Overview),
            enabled: true,
        },
    );
    if let Some(play) = layout.play {
        let enabled = surface.play_enabled();
        paint_strip_tile(
            cx,
            play,
            Icon::Play,
            tr(locale, "workspace.present"),
            palette,
            StripTileState {
                selected: false,
                hovered: surface.state.hover == Some(WorkspaceHit::Play),
                enabled,
            },
        );
    }

    // Thumbnail plates (the host blits the real rasters on top).
    for (slot, thumb) in layout.thumbs.iter().enumerate() {
        let index = layout.thumb_first + slot;
        let selected =
            surface.state.selected == index && surface.state.view != WorkspaceView::Overview;
        let plate = Rect::xywh(
            thumb.origin.x,
            thumb.origin.y,
            thumb.size.x,
            thumb.size.y - 16.0,
        );
        cx.backend.fill_round_rect(plate, 4.0, palette.panel);
        cx.backend.stroke_round_rect(
            plate,
            4.0,
            if selected {
                palette.blue
            } else {
                palette.segment_line
            },
            if selected { 2.0 } else { 1.0 },
        );
        let label = (index + 1).to_string();
        let label_w = cx.backend.measure_text_family(&label, 10.0, SANS);
        text(
            cx,
            &label,
            Point2D::new(
                thumb.origin.x + (thumb.size.x - label_w) / 2.0,
                thumb.origin.y + thumb.size.y - 3.0,
            ),
            10.0,
            if selected {
                palette.blue
            } else {
                palette.muted
            },
        );
    }
}

/// Interaction state of one strip tile.
#[derive(Clone, Copy)]
struct StripTileState {
    selected: bool,
    hovered: bool,
    enabled: bool,
}

/// Paint one 总览 / 放映 tile as a cell of the thumbnail row: a plate of
/// `THUMB_H`, a centred icon, and a centred caption on the thumbnail
/// numbers' baseline. Centring uses the same `measure_text_family` the
/// numbers do, so no caption is left hanging off its tile's left edge.
fn paint_strip_tile(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    icon: Icon,
    caption: &str,
    palette: StudioPalette,
    state: StripTileState,
) {
    const ICON: f32 = 18.0;
    const CAPTION: f32 = 10.0;
    let plate = Rect::xywh(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y - 16.0,
    );
    let ink = if !state.enabled {
        fade(palette.muted, 0.4)
    } else if state.selected {
        palette.blue
    } else {
        palette.muted
    };
    cx.backend.fill_round_rect(
        plate,
        6.0,
        if state.selected {
            palette.blue_soft
        } else if state.hovered && state.enabled {
            palette.button_hover
        } else {
            palette.panel
        },
    );
    cx.backend.stroke_round_rect(
        plate,
        6.0,
        if state.selected {
            palette.blue
        } else {
            palette.segment_line
        },
        if state.selected { 2.0 } else { 1.0 },
    );
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            plate.origin.x + (plate.size.x - ICON) / 2.0,
            plate.origin.y + (plate.size.y - ICON) / 2.0,
        ),
        ICON,
        ink,
        1.6,
    );
    let caption_w = cx.backend.measure_text_family(caption, CAPTION, SANS);
    text(
        cx,
        caption,
        Point2D::new(
            rect.origin.x + (rect.size.x - caption_w) / 2.0,
            rect.origin.y + rect.size.y - 3.0,
        ),
        CAPTION,
        ink,
    );
}
