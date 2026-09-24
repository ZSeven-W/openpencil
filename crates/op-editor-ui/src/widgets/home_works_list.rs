//! The compact Home's 作品 (works) page: the current in-memory work and
//! the recent documents the host already tracks, between the pinned top
//! bar and bottom nav.
//!
//! It shows what the editor state has and nothing more: `recent_files`
//! is whatever the host records (the desktop shell does; a mobile engine
//! without a file registry has none), and the current work is the live
//! document. No storage is invented here. Geometry is one function
//! shared by the paint pass and `HomeSurface::hit_test`.

use super::{copy, fade, HomeSurface, StudioPalette, HOME_TOPBAR_H};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::{EditorState, HomeHit};

const PAD_X: f32 = 17.0;
const HEADING_TOP: f32 = 20.0;
const HEADING_H: f32 = 32.0;
const HEADING_GAP: f32 = 12.0;
const CURRENT_H: f32 = 96.0;
const SECTION_GAP: f32 = 22.0;
const SECTION_H: f32 = 22.0;
const SECTION_GAP_BELOW: f32 = 8.0;
/// Recent rows keep the 44 pt floor with room for two lines.
const ROW_H: f32 = 56.0;
const ROW_GAP: f32 = 8.0;
const EMPTY_H: f32 = 64.0;
/// How many recent rows the page lists at most.
pub const WORKS_RECENT_CAP: usize = 12;

/// The live document as the 作品 page presents it.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentWork {
    pub title: String,
    pub subtitle: String,
}

impl CurrentWork {
    /// The current work, when there is one to reopen: a workspace for this
    /// document, or a page the user drew on (a blank canvas that produced
    /// content counts; the untouched starter does not).
    pub fn for_editor(state: &EditorState) -> Option<Self> {
        let locale = state.editor_ui.locale;
        let workspace = &state.editor_ui.workspace;
        let title = super::super::workspace_surface::workspace_title(state);
        if workspace.active {
            let subtitle = format!(
                "{} · {}",
                super::super::workspace_surface::family_label(locale, workspace.family),
                op_i18n::translate(
                    locale,
                    super::super::workspace_surface::phase_key(workspace.phase)
                )
            );
            return Some(Self { title, subtitle });
        }
        if op_editor_core::blank_starter::active_page_is_blank_starter(state)
            || state.active_children().is_empty()
        {
            return None;
        }
        let boards = op_editor_core::preview_slideshow::active_page_boards(state).len();
        let subtitle = op_i18n::translate(locale, "works.boards")
            .replace("{{count}}", &boards.max(1).to_string());
        Some(Self { title, subtitle })
    }
}

/// Every rect of the 作品 page.
#[derive(Debug, Clone, PartialEq)]
pub struct WorksListLayout {
    pub heading: Rect,
    pub current: Option<Rect>,
    pub recent_heading: Rect,
    /// One rect per listed recent file (capped to what fits).
    pub rows: Vec<Rect>,
    /// The empty-state note when there is nothing at all to list.
    pub empty: Option<Rect>,
}

/// Pure layout for a phone `viewport`, `bottom_nav_h` tall nav included.
pub fn works_list_layout(
    viewport_w: f32,
    viewport_h: f32,
    bottom_nav_h: f32,
    has_current: bool,
    recent_count: usize,
) -> WorksListLayout {
    let w = (viewport_w - PAD_X * 2.0).max(0.0);
    let mut y = HOME_TOPBAR_H + HEADING_TOP;
    let heading = Rect::xywh(PAD_X, y, w, HEADING_H);
    y += HEADING_H + HEADING_GAP;
    let current = has_current.then(|| {
        let rect = Rect::xywh(PAD_X, y, w, CURRENT_H);
        y += CURRENT_H + SECTION_GAP;
        rect
    });
    let recent_heading = Rect::xywh(PAD_X, y, w, SECTION_H);
    y += SECTION_H + SECTION_GAP_BELOW;
    let floor = viewport_h - bottom_nav_h - 12.0;
    let mut rows = Vec::new();
    for _ in 0..recent_count.min(WORKS_RECENT_CAP) {
        if y + ROW_H > floor {
            break;
        }
        rows.push(Rect::xywh(PAD_X, y, w, ROW_H));
        y += ROW_H + ROW_GAP;
    }
    // Nothing at all to list: one note instead of an empty section.
    let empty = (recent_count == 0 && !has_current).then(|| Rect::xywh(PAD_X, y, w, EMPTY_H));
    WorksListLayout {
        heading,
        current,
        recent_heading,
        rows,
        empty,
    }
}

impl HomeSurface<'_> {
    pub fn works_layout(&self, viewport_w: f32, viewport_h: f32) -> WorksListLayout {
        works_list_layout(
            viewport_w,
            viewport_h,
            super::HOME_BOTTOM_NAV_H,
            self.current_work.is_some(),
            self.works_recent.len(),
        )
    }

    /// Hit-test the 作品 page body (the pinned chrome is tested first by
    /// `hit_test`).
    pub(super) fn works_hit(&self, layout: &WorksListLayout, point: Point2D) -> Option<HomeHit> {
        if layout.current.is_some_and(|rect| rect.contains(point)) {
            return Some(HomeHit::WorksCurrent);
        }
        layout
            .rows
            .iter()
            .position(|rect| rect.contains(point))
            .map(HomeHit::WorksRecent)
    }
}

fn text_weighted(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    color: Color,
    weight: u16,
) {
    let layout =
        crate::TextLayout::single_run(content, "system-ui", size, color.to_jian(), Point2D::ZERO)
            .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}

/// Cut to `max_w` with an ellipsis.
fn fit(cx: &mut PaintCx<'_>, content: &str, size: f32, max_w: f32) -> String {
    if cx.backend.measure_text_family(content, size, "system-ui") <= max_w {
        return content.to_string();
    }
    let mut chars: Vec<char> = content.chars().collect();
    while chars.pop().is_some() {
        let candidate = chars.iter().collect::<String>() + "…";
        if cx
            .backend
            .measure_text_family(&candidate, size, "system-ui")
            <= max_w
        {
            return candidate;
        }
    }
    String::new()
}

/// Paint the 作品 page body (under the pinned chrome).
pub(super) fn paint_works_page(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    viewport: Rect,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let layout = surface.works_layout(viewport.size.x, viewport.size.y);
    text_weighted(
        cx,
        copy::home_str(locale, "home.nav.projects"),
        Point2D::new(layout.heading.origin.x, layout.heading.origin.y + 24.0),
        24.0,
        palette.ink,
        720,
    );
    if let (Some(rect), Some(work)) = (layout.current, surface.current_work.as_ref()) {
        let pressed = surface.state.pressed == Some(HomeHit::WorksCurrent);
        cx.backend.fill_round_rect(
            rect,
            16.0,
            if pressed {
                palette.button_hover
            } else {
                palette.panel
            },
        );
        cx.backend.stroke_round_rect(rect, 16.0, palette.line, 1.0);
        let tile = Rect::xywh(rect.origin.x + 14.0, rect.origin.y + 14.0, 68.0, 68.0);
        cx.backend.fill_round_rect(tile, 12.0, palette.preview);
        draw_icon(
            cx.backend,
            Icon::Frame,
            Point2D::new(tile.origin.x + 22.0, tile.origin.y + 22.0),
            24.0,
            palette.link,
            1.6,
        );
        let text_x = tile.origin.x + tile.size.x + 14.0;
        let text_w = (rect.origin.x + rect.size.x - 36.0 - text_x).max(0.0);
        text_weighted(
            cx,
            op_i18n::translate(locale, "works.current"),
            Point2D::new(text_x, rect.origin.y + 28.0),
            11.0,
            palette.eyebrow,
            600,
        );
        let title = fit(cx, &work.title, 16.0, text_w);
        text_weighted(
            cx,
            &title,
            Point2D::new(text_x, rect.origin.y + 52.0),
            16.0,
            palette.ink,
            650,
        );
        let subtitle = fit(cx, &work.subtitle, 12.0, text_w);
        text_weighted(
            cx,
            &subtitle,
            Point2D::new(text_x, rect.origin.y + 73.0),
            12.0,
            palette.muted,
            500,
        );
        draw_icon(
            cx.backend,
            Icon::ChevronRight,
            Point2D::new(
                rect.origin.x + rect.size.x - 30.0,
                rect.origin.y + (rect.size.y - 18.0) / 2.0,
            ),
            18.0,
            fade(palette.ink, 0.45),
            1.8,
        );
    }
    if layout.rows.is_empty() {
        // No recent documents: the section heading would title nothing.
        paint_empty_note(surface, cx, &layout, palette);
        return;
    }
    text_weighted(
        cx,
        copy::home_str(locale, "home.recent.title"),
        Point2D::new(
            layout.recent_heading.origin.x,
            layout.recent_heading.origin.y + 16.0,
        ),
        13.0,
        palette.sub,
        600,
    );
    for (index, rect) in layout.rows.iter().enumerate() {
        let Some(name) = surface.works_recent.get(index) else {
            continue;
        };
        let pressed = surface.state.pressed == Some(HomeHit::WorksRecent(index));
        cx.backend.fill_round_rect(
            *rect,
            12.0,
            if pressed {
                palette.button_hover
            } else {
                palette.panel
            },
        );
        cx.backend.stroke_round_rect(*rect, 12.0, palette.line, 1.0);
        draw_icon(
            cx.backend,
            Icon::FileText,
            Point2D::new(
                rect.origin.x + 14.0,
                rect.origin.y + (rect.size.y - 20.0) / 2.0,
            ),
            20.0,
            palette.link,
            1.6,
        );
        let label = fit(cx, name, 14.0, (rect.size.x - 80.0).max(0.0));
        text_weighted(
            cx,
            &label,
            Point2D::new(
                rect.origin.x + 46.0,
                jian_widgets::centered_text_baseline_y(*rect, 14.0),
            ),
            14.0,
            palette.ink,
            500,
        );
        draw_icon(
            cx.backend,
            Icon::ChevronRight,
            Point2D::new(
                rect.origin.x + rect.size.x - 28.0,
                rect.origin.y + (rect.size.y - 16.0) / 2.0,
            ),
            16.0,
            fade(palette.ink, 0.4),
            1.8,
        );
    }
}

fn paint_empty_note(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorksListLayout,
    palette: StudioPalette,
) {
    let Some(rect) = layout.empty else {
        return;
    };
    let note = op_i18n::translate(surface.ui.locale, "works.empty");
    let note = fit(cx, note, 13.0, rect.size.x);
    text_weighted(
        cx,
        &note,
        Point2D::new(rect.origin.x, rect.origin.y + 22.0),
        13.0,
        palette.muted,
        500,
    );
}

#[cfg(test)]
#[path = "home_works_list_tests.rs"]
mod tests;
