//! Floating-overlay rect getters for the web `WidgetHost`.
//!
//! Ported from the native host's `widget_host/overlay_rects.rs` (and
//! the StatusBar placement helpers formerly in `press.rs`) so the web
//! paint pass + press dispatch share one source of truth for every
//! overlay's on-screen rect. Keeping these in a sibling module keeps
//! `press.rs` under the repo's 800-line cap.

use super::{WidgetHost, STATUS_INSET, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use op_editor_ui::widgets::{
    LayoutCx, ShapePicker, Toolbar, TopBar, Widget, ICON_PICKER_PANEL_H, ICON_PICKER_PANEL_W,
    SHAPE_PICKER_WIDTH, STATUS_BAR_HEIGHT, STATUS_BAR_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    /// The floating bottom-right StatusBar pill rect, or `None` when
    /// the canvas is too narrow to float it (matches the paint guard).
    pub(in crate::widget_host) fn status_bar_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        let (canvas_left, _top, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        if canvas_w <= STATUS_BAR_WIDTH + STATUS_INSET * 2.0 {
            return None;
        }
        let canvas_right = canvas_left + canvas_w;
        Some(Rect {
            origin: Point2D::new(
                canvas_right - STATUS_BAR_WIDTH - STATUS_INSET,
                TOP_BAR_HEIGHT + canvas_h - STATUS_BAR_HEIGHT - STATUS_INSET,
            ),
            size: Point2D::new(STATUS_BAR_WIDTH, STATUS_BAR_HEIGHT),
        })
    }

    /// Step the canvas zoom from a StatusBar `[-]` / `[+]` click,
    /// anchored at the canvas-region centre so the visible content
    /// scales in place (≈ ±20 % per click via `Viewport::zoom_at`).
    pub(in crate::widget_host) fn status_bar_zoom(
        &mut self,
        zoom_in: bool,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        const STEP: f32 = 120.0;
        // `Viewport::zoom_at` works in CANVAS-LOCAL coords (the wheel
        // path feeds it `window_pt - canvas_origin`), so the anchor is
        // the canvas-region centre relative to the canvas origin — not
        // the window-space centre.
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let center = Point2D::new(cw / 2.0, ch / 2.0);
        self.editor_state
            .viewport
            .zoom_at(center, if zoom_in { STEP } else { -STEP });
        self.mark_dirty();
    }

    /// Zoom + pan so the active page's content is framed within the canvas.
    pub(in crate::widget_host) fn zoom_to_fit(&mut self, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        let Some(content) = self.layout_scene.content_bounds() else {
            return;
        };
        let (_l, _t, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        self.editor_state
            .viewport
            .fit_to_with_max_zoom(content, canvas_w, canvas_h, 64.0, 1.0);
        self.mark_dirty();
    }

    /// Shape-picker dropdown rect — anchored to the right of the
    /// toolbar's shape slot. Mirrors the native host so paint, press
    /// dispatch, and hover updates can't drift apart.
    pub(in crate::widget_host) fn shape_picker_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Rect {
        let (cx0, _cy, cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        let slot = toolbar
            .shape_slot_rect(toolbar_rect)
            .unwrap_or(toolbar_rect);
        let panel_h = ShapePicker::panel_height();
        let max_x = cx0 + cw - SHAPE_PICKER_WIDTH - 4.0;
        let toolbar_right = toolbar_rect.origin.x + toolbar_rect.size.x;
        let x = (toolbar_right + 8.0).min(max_x);
        let y = slot.origin.y;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(SHAPE_PICKER_WIDTH, panel_h),
        }
    }

    /// The open File-menu dropdown rect, or `None` when closed. The
    /// rect depends only on the menu's row count (recent-file list),
    /// not on time, so a `0` clock is fine here.
    pub(in crate::widget_host) fn file_menu_rect(&self, viewport_w: f32) -> Option<Rect> {
        use op_editor_ui::widgets::file_menu::FileMenu;
        if !self.editor_state.editor_ui.file_menu_open {
            return None;
        }
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let anchor =
            TopBar::file_menu_rect(top_bar_rect, self.editor_state.editor_ui.window_fullscreen);
        let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, 0);
        Some(menu.rect_at(anchor))
    }

    /// Floating Design-MD panel rect — `None` when the panel is
    /// closed. The top-left comes from `editor_ui.design_md_panel_pos`
    /// (centred on open), clamped so the header bar stays reachable
    /// after a viewport resize. Mirrors the native host.
    pub(in crate::widget_host) fn design_md_panel_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        use op_editor_ui::widgets::{DESIGN_MD_PANEL_H, DESIGN_MD_PANEL_W};
        let ui = &self.editor_state.editor_ui;
        if !ui.design_md_panel_open {
            return None;
        }
        let (px, py) = ui.design_md_panel_pos.unwrap_or_else(|| {
            (
                ((viewport_w - DESIGN_MD_PANEL_W) / 2.0).max(0.0),
                ((viewport_h - DESIGN_MD_PANEL_H) / 2.0).max(0.0),
            )
        });
        // Keep at least the header bar on-screen.
        let x = px.clamp(0.0, (viewport_w - 80.0).max(0.0));
        let y = py.clamp(0.0, (viewport_h - 40.0).max(0.0));
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(DESIGN_MD_PANEL_W, DESIGN_MD_PANEL_H),
        })
    }

    /// Floating Component-Browser panel rect — `None` when closed.
    /// Same centred-on-open + clamped placement as the Design-MD panel.
    pub(in crate::widget_host) fn component_browser_panel_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        use op_editor_ui::widgets::{COMPONENT_BROWSER_PANEL_H, COMPONENT_BROWSER_PANEL_W};
        let ui = &self.editor_state.editor_ui;
        if !ui.component_browser_open {
            return None;
        }
        let (px, py) = ui.component_browser_pos.unwrap_or_else(|| {
            (
                ((viewport_w - COMPONENT_BROWSER_PANEL_W) / 2.0).max(0.0),
                ((viewport_h - COMPONENT_BROWSER_PANEL_H) / 2.0).max(0.0),
            )
        });
        let x = px.clamp(0.0, (viewport_w - 80.0).max(0.0));
        let y = py.clamp(0.0, (viewport_h - 40.0).max(0.0));
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(COMPONENT_BROWSER_PANEL_W, COMPONENT_BROWSER_PANEL_H),
        })
    }

    /// Floating Icon-picker panel rect — `None` when closed. Centred
    /// compact searchable panel, same placement as the native host.
    pub(in crate::widget_host) fn icon_picker_panel_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        if !self.editor_state.editor_ui.icon_picker_open {
            return None;
        }
        let ui = &self.editor_state.editor_ui;
        let (px, py) = ui.icon_picker_panel_pos.unwrap_or_else(|| {
            (
                ((viewport_w - ICON_PICKER_PANEL_W) / 2.0).max(0.0),
                ((viewport_h - ICON_PICKER_PANEL_H) / 2.0).max(0.0),
            )
        });
        let x = px.clamp(0.0, (viewport_w - 80.0).max(0.0));
        let y = py.clamp(0.0, (viewport_h - 40.0).max(0.0));
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H),
        })
    }

    /// Whether `point` is inside ANY top-most floating panel
    /// (Design-MD / Icon picker / Component browser). Used to suppress
    /// dropdown hover updates under an overlapping floating panel.
    pub(in crate::widget_host) fn over_topmost_panel(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let p = Point2D::new(x, y);
        self.design_md_panel_rect(viewport_w, viewport_h)
            .is_some_and(|r| (r).contains(p))
            || self
                .icon_picker_panel_rect(viewport_w, viewport_h)
                .is_some_and(|r| (r).contains(p))
            || self
                .component_browser_panel_rect(viewport_w, viewport_h)
                .is_some_and(|r| (r).contains(p))
    }
}
