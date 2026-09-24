//! The Studio generation-workspace chrome: docked-chat header, canvas
//! toolbar, and the presentation deck strip.
//!
//! Geometry and hit-testing live here; the immediate-mode paint pass is
//! in `workspace_surface_paint.rs`. The surface is document-agnostic —
//! it reads the same `EditorState` as the canvas. The CANVAS itself and
//! the pinned chat panel are painted by the host in between this
//! chrome's layers (chrome → canvas → chat → strip thumbnails).

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use op_editor_core::{
    EditorState, EditorUiState, HomeFamily, WorkspaceHit, WorkspaceState, WorkspaceView,
    WORKSPACE_DECK_STRIP_H, WORKSPACE_HEADER_H, WORKSPACE_TOOLBAR_H,
};

pub use super::home_surface::StudioPalette;

/// Back circle button (36 px) inset.
/// Left inset of the header's back circle. The desktop window is
/// frameless, so the macOS traffic lights float over the app's own
/// header — measured, they occupy x ≈ 12..72. A 14 px inset put the
/// back circle UNDER the close/minimize buttons and made 返回首页
/// unclickable (the window buttons swallow the press). Home solved
/// this by starting its brand mark at 80; the workspace header uses
/// the same gutter so the two chromes line up as well.
const BACK_INSET: f32 = 80.0;
/// Vertical inset of the back circle, kept independent of the left
/// gutter so the circle stays centred in the 64 px header.
const BACK_INSET_Y: f32 = 14.0;
const BACK_SIZE: f32 = 36.0;
/// Doc icon tile beside the title.
const DOC_TILE: f32 = 34.0;
/// Header outline buttons.
const HEADER_BUTTON_H: f32 = 32.0;
const HEADER_BUTTON_Y: f32 = 16.0;
const EXPORT_BUTTON_W: f32 = 68.0;
const PROFESSIONAL_BUTTON_W: f32 = 104.0;
const HEADER_RIGHT_INSET: f32 = 14.0;
/// Toolbar icon-button size (toggle / prev / next / zoom).
const TOOLBAR_ICON: f32 = 28.0;
const TOOLBAR_BUTTON_H: f32 = 28.0;
/// View-mode segmented control metrics.
const SEGMENT_W: f32 = 64.0;
const SEGMENT_H: f32 = 26.0;
const SEGMENT_GAP: f32 = 0.0;
/// Fit button width (适应 / 100%).
const ZOOM_FIT_W: f32 = 48.0;
const ZOOM_ICON_W: f32 = 28.0;
/// Deck-strip metrics.
const STRIP_PAD_X: f32 = 14.0;
const THUMB_W: f32 = 112.0;
const THUMB_H: f32 = 63.0;
const THUMB_LABEL_H: f32 = 16.0;
const THUMB_GAP: f32 = 10.0;
/// Top inset shared by every cell in the strip row (tiles and thumbs).
const STRIP_ROW_TOP: f32 = 8.0;
/// The strip's 总览 / 放映 tiles are laid out as two more cells of the
/// thumbnail row, not as free-floating buttons: same top edge, same
/// plate height, same centred caption underneath. Sizing them
/// independently (44×68, vertically centred in the strip) left all three
/// kinds of cell on different baselines and the captions hanging off
/// their tiles' left edge.
const STRIP_ACTION_W: f32 = 56.0;
const STRIP_ACTION_H: f32 = THUMB_H + THUMB_LABEL_H;
/// Failed-phase banner buttons.
const BANNER_BUTTON_W: f32 = 96.0;
const BANNER_BUTTON_H: f32 = 34.0;

/// ease-out-cubic — the settle curve the entrance choreography uses.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// The chrome's entrance phase at `now_ms`: `(rise_offset_y, alpha)`.
/// A `shown_at_ms` of 0 paints settled.
pub fn workspace_enter(shown_at_ms: u64, now_ms: u64) -> (f32, f32) {
    if shown_at_ms == 0 {
        return (0.0, 1.0);
    }
    let elapsed = now_ms.saturating_sub(shown_at_ms);
    let t = (elapsed as f32 / op_editor_core::WORKSPACE_ENTER_MS as f32).clamp(0.0, 1.0);
    let eased = ease_out_cubic(t);
    ((1.0 - eased) * 8.0, eased)
}

/// The view segments a family's toolbar offers, in paint order.
pub fn family_views(family: HomeFamily) -> &'static [WorkspaceView] {
    match family {
        HomeFamily::Presentation => &[WorkspaceView::Single { index: 0 }],
        HomeFamily::Web | HomeFamily::Infographic => &[WorkspaceView::LongPage],
        _ => &[WorkspaceView::AllBoards, WorkspaceView::Single { index: 0 }],
    }
}

/// Whether this family paints the bottom deck strip.
pub fn family_has_strip(family: HomeFamily) -> bool {
    matches!(family, HomeFamily::Presentation)
}

/// Every rect the paint pass, the host blit and the tests need.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceLayout {
    pub header: Rect,
    pub back: Rect,
    pub doc_tile: Rect,
    pub title: Rect,
    pub export: Rect,
    pub professional: Rect,
    pub toolbar: Rect,
    /// The docked chat column; `None` while collapsed.
    pub dock: Option<Rect>,
    /// The dock's drag handle — the rightmost 5 px of the dock.
    pub dock_handle: Option<Rect>,
    pub view_segments: Vec<Rect>,
    pub prev: Option<Rect>,
    pub next: Option<Rect>,
    pub zoom_out: Rect,
    pub zoom_fit: Rect,
    pub zoom_in: Rect,
    pub strip: Option<Rect>,
    /// Thumbnail slots, left to right. Slot `i` shows board
    /// `thumb_first + i` — a deck longer than the strip slides this window
    /// to keep the selected board in view instead of dropping the tail.
    pub thumbs: Vec<Rect>,
    pub thumb_first: usize,
    pub overview: Rect,
    pub play: Option<Rect>,
    /// The canvas region the host paints the real canvas into.
    pub canvas: Rect,
    /// Failed-phase banner actions, centered near the canvas top.
    pub retry: Option<Rect>,
    pub return_edit: Option<Rect>,
}

/// Pure layout: shared by paint, hit-test and the host. `board_count`
/// sizes the deck strip's thumbnail row. `dock_width` / `collapsed` are
/// the LEFT PANEL's `layer_panel_width` and `!sidebar_open` — passed in
/// as values so the pure function stays testable.
/// First board of the thumbnail window: centred on the selection, clamped so
/// the window never runs past either end of the deck.
pub fn thumb_window_start(board_count: usize, slots: usize, selected: usize) -> usize {
    if slots == 0 || board_count <= slots {
        return 0;
    }
    selected
        .min(board_count - 1)
        .saturating_sub(slots / 2)
        .min(board_count - slots)
}

impl WorkspaceLayout {
    /// The strip slot showing `board`, if it is inside the window.
    pub fn thumb_rect(&self, board: usize) -> Option<Rect> {
        board
            .checked_sub(self.thumb_first)
            .and_then(|slot| self.thumbs.get(slot))
            .copied()
    }
}

pub fn layout_for(
    viewport_width: f32,
    viewport_height: f32,
    family: HomeFamily,
    dock_width: f32,
    collapsed: bool,
    board_count: usize,
) -> WorkspaceLayout {
    let vw = viewport_width.max(1.0);
    let vh = viewport_height.max(1.0);
    let dock = (!collapsed)
        .then(|| Rect::xywh(0.0, WORKSPACE_HEADER_H, dock_width, vh - WORKSPACE_HEADER_H));
    let dock_handle = (!collapsed).then(|| {
        Rect::xywh(
            dock_width - 5.0,
            WORKSPACE_HEADER_H,
            5.0,
            vh - WORKSPACE_HEADER_H,
        )
    });
    let canvas_x = if collapsed { 0.0 } else { dock_width };

    let header = Rect::xywh(0.0, 0.0, vw, WORKSPACE_HEADER_H);
    let back = Rect::xywh(BACK_INSET, BACK_INSET_Y, BACK_SIZE, BACK_SIZE);
    let doc_tile = Rect::xywh(
        BACK_INSET + BACK_SIZE + 14.0,
        (WORKSPACE_HEADER_H - DOC_TILE) / 2.0,
        DOC_TILE,
        DOC_TILE,
    );
    let title = Rect::xywh(
        doc_tile.origin.x + DOC_TILE + 10.0,
        14.0,
        (vw / 2.0).max(160.0),
        36.0,
    );
    let export = Rect::xywh(
        vw - HEADER_RIGHT_INSET - PROFESSIONAL_BUTTON_W - 8.0 - EXPORT_BUTTON_W,
        HEADER_BUTTON_Y,
        EXPORT_BUTTON_W,
        HEADER_BUTTON_H,
    );
    let professional = Rect::xywh(
        vw - HEADER_RIGHT_INSET - PROFESSIONAL_BUTTON_W,
        HEADER_BUTTON_Y,
        PROFESSIONAL_BUTTON_W,
        HEADER_BUTTON_H,
    );

    let toolbar = Rect::xywh(
        canvas_x,
        WORKSPACE_HEADER_H,
        vw - canvas_x,
        WORKSPACE_TOOLBAR_H,
    );
    // Left cluster: the chat toggle icon then the view segments.
    let toggle = Rect::xywh(
        toolbar.origin.x + 12.0,
        toolbar.origin.y + (WORKSPACE_TOOLBAR_H - TOOLBAR_BUTTON_H) / 2.0,
        TOOLBAR_ICON,
        TOOLBAR_BUTTON_H,
    );
    let mut x = toggle.origin.x + toggle.size.x + 10.0;
    let segment_y = toolbar.origin.y + (WORKSPACE_TOOLBAR_H - SEGMENT_H) / 2.0;
    let views = family_views(family);
    let mut view_segments = Vec::with_capacity(views.len());
    for _ in views {
        view_segments.push(Rect::xywh(x, segment_y, SEGMENT_W, SEGMENT_H));
        x += SEGMENT_W + SEGMENT_GAP;
    }

    // Right cluster: prev / next, then zoom − 适应 +, right-aligned.
    let mut right = toolbar.origin.x + toolbar.size.x - 12.0;
    let icon_y = toolbar.origin.y + (WORKSPACE_TOOLBAR_H - TOOLBAR_BUTTON_H) / 2.0;
    let zoom_in = Rect::xywh(right - ZOOM_ICON_W, icon_y, ZOOM_ICON_W, TOOLBAR_BUTTON_H);
    right -= ZOOM_ICON_W;
    let zoom_fit = Rect::xywh(right - ZOOM_FIT_W, icon_y, ZOOM_FIT_W, TOOLBAR_BUTTON_H);
    right -= ZOOM_FIT_W + 2.0;
    let zoom_out = Rect::xywh(right - ZOOM_ICON_W, icon_y, ZOOM_ICON_W, TOOLBAR_BUTTON_H);
    right -= ZOOM_ICON_W + 10.0;
    let show_pager = matches!(family, HomeFamily::Presentation)
        || views.contains(&WorkspaceView::Single { index: 0 });
    let next = show_pager
        .then(|| Rect::xywh(right - TOOLBAR_ICON, icon_y, TOOLBAR_ICON, TOOLBAR_BUTTON_H));
    right -= TOOLBAR_ICON + 2.0;
    let prev = show_pager
        .then(|| Rect::xywh(right - TOOLBAR_ICON, icon_y, TOOLBAR_ICON, TOOLBAR_BUTTON_H));

    let strip_h = family_has_strip(family).then_some(WORKSPACE_DECK_STRIP_H);
    let strip_top = vh - strip_h.unwrap_or(0.0);
    let strip = strip_h.map(|h| Rect::xywh(canvas_x, strip_top, vw - canvas_x, h));
    let (overview, play, thumbs) = match (&strip, board_count) {
        (Some(strip), _) => {
            let action_y = strip.origin.y + STRIP_ROW_TOP;
            let overview = Rect::xywh(
                strip.origin.x + STRIP_PAD_X,
                action_y,
                STRIP_ACTION_W,
                STRIP_ACTION_H,
            );
            let play = Some(Rect::xywh(
                strip.origin.x + strip.size.x - STRIP_PAD_X - STRIP_ACTION_W,
                action_y,
                STRIP_ACTION_W,
                STRIP_ACTION_H,
            ));
            let mut x = overview.origin.x + overview.size.x + THUMB_GAP;
            let thumb_top = strip.origin.y + STRIP_ROW_TOP;
            let mut thumbs = Vec::with_capacity(board_count);
            for _ in 0..board_count {
                if x + THUMB_W
                    > strip.origin.x + strip.size.x - STRIP_PAD_X - STRIP_ACTION_W - THUMB_GAP
                {
                    break;
                }
                thumbs.push(Rect::xywh(x, thumb_top, THUMB_W, THUMB_H + THUMB_LABEL_H));
                x += THUMB_W + THUMB_GAP;
            }
            (overview, play, thumbs)
        }
        (None, _) => (Rect::ZERO, None, Vec::new()),
    };

    let canvas = Rect::xywh(
        canvas_x,
        WORKSPACE_HEADER_H + WORKSPACE_TOOLBAR_H,
        vw - canvas_x,
        (strip_top - (WORKSPACE_HEADER_H + WORKSPACE_TOOLBAR_H)).max(0.0),
    );
    let (retry, return_edit) = (None, None);

    WorkspaceLayout {
        header,
        back,
        doc_tile,
        title,
        export,
        professional,
        toolbar,
        dock,
        dock_handle,
        view_segments,
        prev,
        next,
        zoom_out,
        zoom_fit,
        zoom_in,
        strip,
        thumbs,
        thumb_first: 0,
        overview,
        play,
        canvas,
        retry,
        return_edit,
    }
}

pub struct WorkspaceSurface<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: &'a WorkspaceState,
    pub ui: &'a EditorUiState,
    /// Header title: the file name, else the brief's first 16 chars,
    /// else the localized untitled fallback.
    pub title: String,
    /// The active page's board ids, in document order.
    pub boards: Vec<String>,
    pub now_ms: u64,
}

impl<'a> WorkspaceSurface<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        let workspace = &state.editor_ui.workspace;
        if !workspace.visible {
            return None;
        }
        let title = state
            .editor_ui
            .file_name_display
            .clone()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| {
                let brief = workspace.brief.trim();
                if brief.is_empty() {
                    op_i18n::translate(state.editor_ui.locale, "common.untitled").to_string()
                } else {
                    brief.chars().take(16).collect()
                }
            });
        Some(Self {
            id: WidgetId::new(7700),
            theme: theme_for(&state.editor_ui),
            state: workspace,
            ui: &state.editor_ui,
            title,
            boards: op_editor_core::preview_slideshow::active_page_boards(state),
            now_ms,
        })
    }

    pub fn layout(&self, viewport_width: f32, viewport_height: f32) -> WorkspaceLayout {
        let mut layout = layout_for(
            viewport_width,
            viewport_height,
            self.state.family,
            // The dock IS the left panel: one width, one open flag.
            self.ui.layer_panel_width,
            !self.ui.sidebar_open,
            self.boards.len(),
        );
        layout.thumb_first =
            thumb_window_start(self.boards.len(), layout.thumbs.len(), self.state.selected);
        layout
    }

    /// Failed-phase banner actions, resolved against the canvas so the
    /// hit-test and paint share one answer (layout_for is failure-blind).
    /// Present runs only on a finished result with boards. The strip paints
    /// the tile disabled otherwise, and the press must agree with the paint.
    pub fn play_enabled(&self) -> bool {
        self.state.phase == op_editor_core::WorkspacePhase::Done && !self.boards.is_empty()
    }

    /// Retry / back-to-edit actions for a run that did not finish: a failed
    /// run, or one the user stopped (stopping used to leave no way to try
    /// again short of retyping the brief on Home).
    pub fn banner_buttons(&self, layout: &WorkspaceLayout) -> Option<(Rect, Rect)> {
        if !matches!(
            self.state.phase,
            op_editor_core::WorkspacePhase::Failed | op_editor_core::WorkspacePhase::Stopped
        ) {
            return None;
        }
        let cy = layout.canvas.origin.y + 28.0;
        let cx = layout.canvas.origin.x + layout.canvas.size.x / 2.0;
        Some((
            Rect::xywh(
                cx - BANNER_BUTTON_W - 5.0,
                cy,
                BANNER_BUTTON_W,
                BANNER_BUTTON_H,
            ),
            Rect::xywh(cx + 5.0, cy, BANNER_BUTTON_W, BANNER_BUTTON_H),
        ))
    }

    pub fn hit_test(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        point: Point2D,
    ) -> Option<WorkspaceHit> {
        let layout = self.layout(viewport_width, viewport_height);
        self.hit_test_layout(&layout, point)
    }

    /// Hit-test against a prebuilt layout (the host paints then presses
    /// against the same rects).
    pub fn hit_test_layout(
        &self,
        layout: &WorkspaceLayout,
        point: Point2D,
    ) -> Option<WorkspaceHit> {
        if let Some((retry, return_edit)) = self.banner_buttons(layout) {
            if retry.contains(point) {
                return Some(WorkspaceHit::Retry);
            }
            if return_edit.contains(point) {
                return Some(WorkspaceHit::ReturnEdit);
            }
        }
        // Strip sits above the canvas it overlaps.
        if let Some(strip) = layout.strip {
            if strip.contains(point) {
                if let Some(play) = layout.play {
                    if play.contains(point) {
                        // A disabled tile swallows the press instead of
                        // presenting a half-drawn deck.
                        return self.play_enabled().then_some(WorkspaceHit::Play);
                    }
                }
                if layout.overview.contains(point) {
                    return Some(WorkspaceHit::Overview);
                }
                for (slot, rect) in layout.thumbs.iter().enumerate() {
                    if rect.contains(point) {
                        return Some(WorkspaceHit::Thumb(layout.thumb_first + slot));
                    }
                }
                return Some(WorkspaceHit::Overview);
            }
        }
        if layout.toolbar.contains(point) {
            if layout.zoom_in.contains(point) {
                return Some(WorkspaceHit::ZoomIn);
            }
            if layout.zoom_fit.contains(point) {
                return Some(WorkspaceHit::ZoomFit);
            }
            if layout.zoom_out.contains(point) {
                return Some(WorkspaceHit::ZoomOut);
            }
            if let Some(next) = layout.next {
                if next.contains(point) {
                    return Some(WorkspaceHit::Next);
                }
            }
            if let Some(prev) = layout.prev {
                if prev.contains(point) {
                    return Some(WorkspaceHit::Prev);
                }
            }
            for (index, rect) in layout.view_segments.iter().enumerate() {
                if rect.contains(point) {
                    return family_views(self.state.family)
                        .get(index)
                        .copied()
                        .map(WorkspaceHit::View);
                }
            }
            // The toggle owns the toolbar's leading icon slot.
            let toggle = Rect::xywh(
                layout.toolbar.origin.x + 12.0,
                layout.toolbar.origin.y + (WORKSPACE_TOOLBAR_H - TOOLBAR_BUTTON_H) / 2.0,
                TOOLBAR_ICON,
                TOOLBAR_BUTTON_H,
            );
            if toggle.contains(point) {
                return Some(WorkspaceHit::ToggleDock);
            }
            return None;
        }
        if layout.header.contains(point) {
            if layout.professional.contains(point) {
                return Some(WorkspaceHit::Professional);
            }
            if layout.export.contains(point) {
                return Some(WorkspaceHit::Export);
            }
            if layout.back.contains(point) {
                return Some(WorkspaceHit::Back);
            }
            return None;
        }
        if let Some(handle) = layout.dock_handle {
            if handle.contains(point) {
                return Some(WorkspaceHit::DockResize);
            }
        }
        None
    }
}

impl Widget for WorkspaceSurface<'_> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, cx.available_width, 900.0),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        self.paint_workspace(cx, rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Main);
        node.set_label("工作区");
        node
    }
}

#[path = "workspace_surface_paint.rs"]
mod paint;

impl WorkspaceSurface<'_> {
    fn paint_workspace(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint::paint_workspace(self, cx, rect);
    }
}

#[cfg(test)]
#[path = "workspace_surface_tests.rs"]
mod tests;
