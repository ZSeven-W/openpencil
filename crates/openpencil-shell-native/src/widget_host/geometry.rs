//! Geometry + cursor-affordance methods on `WidgetHostNative`.
//! Pure-math helpers that map (viewport_w, viewport_h, x, y) into
//! the rects + cursor hints the host serves. Pulled out of
//! `widget_host.rs` to keep the spine file under the 800-line
//! ceiling.

use super::helpers::{PANEL_RESIZE_GUTTER, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::{CursorHint, PanelResizeKind, WidgetHostNative};
use openpencil_shell_core::widgets::{
    rotation_corner_at_point, selection_handle_at_point, LayoutCx, LocalePicker, ShapePicker,
    Toolbar, TopBar, Widget, AI_CHAT_COLLAPSED_HEIGHT, AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_HEIGHT,
    AI_CHAT_WIDTH, LOCALE_PICKER_WIDTH, SHAPE_PICKER_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{document::ChatAnchor, Point2D, Rect};

use super::helpers::{AICHAT_INSET_BOTTOM, AICHAT_INSET_LEFT};

impl WidgetHostNative {
    /// Hit-test which screen region the cursor is over. Used by
    /// the wheel + drag handlers so wheel-zoom + Hand-pan only
    /// fire when the cursor is over the canvas (not over a panel).
    /// Uses `canvas_region` so it stays in sync with paint when
    /// the sidebar is collapsed (codex Step 6 stop-hook fix:
    /// "native collapsed-sidebar canvas input still uses the old
    /// left offset").
    pub(in crate::widget_host) fn over_canvas(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        x >= cx0 && x <= cx0 + cw && y >= cy0 && y <= cy0 + ch
    }

    /// True when the cursor is over either resize gutter — used by
    /// the runner to set `CursorIcon::EwResize`. None = no gutter.
    pub fn panel_resize_hover(&self, x: f32, y: f32, viewport_w: f32) -> Option<PanelResizeKind> {
        if y < TOP_BAR_HEIGHT {
            return None;
        }
        if self.document.ui.sidebar_open {
            let edge = self.document.ui.layer_panel_width;
            if (x - edge).abs() <= PANEL_RESIZE_GUTTER {
                return Some(PanelResizeKind::LayerRight);
            }
        }
        if self.document.property_panel_visible() {
            let edge = viewport_w - self.document.ui.property_panel_width;
            if (x - edge).abs() <= PANEL_RESIZE_GUTTER {
                return Some(PanelResizeKind::PropertyLeft);
            }
        }
        None
    }

    /// Whether a panel resize is in progress. Runner uses this to
    /// keep the resize cursor active even when the cursor briefly
    /// leaves the gutter mid-drag.
    pub fn is_resizing_panel(&self) -> bool {
        self.panel_resize.is_some()
    }

    /// Update `Document.ui.hovered_layer_id` from the current
    /// cursor position. Returns `true` if the hover state
    /// changed (host should request a redraw so the layer
    /// panel re-paints the eye/lock affordances).
    pub fn update_layer_hover(&mut self, x: f32, y: f32, viewport_h: f32) -> bool {
        use openpencil_shell_core::widgets::{LayerPanel, LayerPanelHit};
        let (new_layer, new_page) = if self.document.ui.sidebar_open
            && y >= openpencil_shell_core::widgets::TOP_BAR_HEIGHT
            && x >= 0.0
            && x <= self.document.ui.layer_panel_width
        {
            let layer_rect = openpencil_shell_core::Rect {
                origin: openpencil_shell_core::Point2D::new(
                    0.0,
                    openpencil_shell_core::widgets::TOP_BAR_HEIGHT,
                ),
                size: openpencil_shell_core::Point2D::new(
                    self.document.ui.layer_panel_width,
                    (viewport_h - openpencil_shell_core::widgets::TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let panel = LayerPanel::from_document(&self.document);
            match panel.hit_test(layer_rect, openpencil_shell_core::Point2D::new(x, y)) {
                Some(LayerPanelHit::Layer(id))
                | Some(LayerPanelHit::ToggleHidden(id))
                | Some(LayerPanelHit::ToggleLocked(id))
                | Some(LayerPanelHit::ToggleCollapsed(id)) => (Some(id), None),
                Some(LayerPanelHit::Page(idx)) | Some(LayerPanelHit::DeletePage(idx)) => {
                    (None, Some(idx))
                }
                _ => (None, None),
            }
        } else {
            (None, None)
        };
        let changed = new_layer != self.document.ui.hovered_layer_id
            || new_page != self.document.ui.hovered_page_index;
        self.document.ui.hovered_layer_id = new_layer;
        self.document.ui.hovered_page_index = new_page;
        changed
    }

    /// True when the cursor is over a draggable node inside the
    /// canvas region (used by the runner to flip cursor → Move).
    /// Returns false when the cursor is in chrome, over empty
    /// canvas, or while the Hand tool is active.
    pub fn cursor_over_node(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        if matches!(
            self.document.tool,
            openpencil_shell_core::document::Tool::Hand
        ) {
            return false;
        }
        if !self.over_canvas(x, y, viewport_w, viewport_h) {
            return false;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc_point = self.document.viewport.to_document(canvas_local);
        self.document.node_at_doc_point(doc_point).is_some()
    }

    /// True while a node-drag is in flight — runner keeps the move
    /// cursor pinned even when the cursor briefly slips outside the
    /// node's bounds during a fast drag.
    pub fn is_dragging_node(&self) -> bool {
        self.node_drag.is_some()
    }

    /// Aggregate cursor recommendation — covers everything the
    /// runner used to compute piecemeal (resize gutter, in-flight
    /// drag, handle / rotation ring, node hover). The runner just
    /// maps the `CursorHint` to its platform cursor.
    /// Recompute the hovered provider-card index on the agent
    /// settings modal. Returns true iff the cached value changed
    /// and a repaint is needed (drives the hover red-disconnect
    /// affordance on connected cards).
    pub fn update_agent_settings_hover(&mut self, x: f32, y: f32) -> bool {
        use openpencil_shell_core::document::AgentSettingsTab;
        use openpencil_shell_core::widgets::agent_settings_panel::AgentSettingsPanel;
        let point = Point2D::new(x, y);
        // Compute both hover values from an immutable borrow before
        // any mutation — `panel` keeps the borrow alive through the
        // hit-tests, so we materialise both into locals first.
        let (new_nav, new_card) = {
            let panel = AgentSettingsPanel::for_document(&self.document);
            let panel_rect = panel.rect(self.last_viewport_w, self.last_viewport_h);
            let nav = panel.nav_at(panel_rect, point);
            let card = if matches!(self.document.ui.agent_settings.tab, AgentSettingsTab::Agents) {
                Some(panel.card_at(panel_rect, point).unwrap_or(usize::MAX))
            } else {
                None
            };
            (nav, card)
        };
        let mut changed = false;
        if new_nav != self.document.ui.agent_settings.hover_nav {
            self.document.ui.agent_settings.hover_nav = new_nav;
            changed = true;
        }
        if let Some(v) = new_card {
            if v != self.document.ui.agent_settings.hover_provider {
                self.document.ui.agent_settings.hover_provider = v;
                changed = true;
            }
        }
        changed
    }

    pub fn cursor_hint(&self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> CursorHint {
        use openpencil_shell_core::document::Tool;
        // Modal overlays — keep the pointer the OS default so the
        // sidebar nav and toggle rows don't show a Move cursor as
        // if the user could drag the underlying canvas.
        if self.document.ui.agent_settings_open || self.document.ui.color_picker.is_some() {
            return CursorHint::Default;
        }
        // Chrome / drag-in-flight wins regardless of tool.
        if self.panel_resize_hover(x, y, viewport_w).is_some() || self.is_resizing_panel() {
            return CursorHint::ResizeEw;
        }
        if self.is_dragging_node() {
            return CursorHint::Grabbing;
        }
        if self.rotate_drag.is_some() {
            return CursorHint::Rotate;
        }
        if let Some(handle) = self.handle_drag.map(|d| d.handle) {
            return CursorHint::for_handle(handle);
        }
        if !self.over_canvas(x, y, viewport_w, viewport_h) {
            return CursorHint::Default;
        }
        // Off-canvas was handled above; now branch on tool because
        // selection / resize / rotate affordances only make sense
        // for the Select tool. With a shape tool active the cursor
        // shouldn't pretend you can drag handles — you can only
        // draw.
        match self.document.tool {
            Tool::Hand => CursorHint::Grab,
            Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::Pen | Tool::Frame => {
                CursorHint::Crosshair
            }
            Tool::Text => CursorHint::Text,
            Tool::Select => {
                let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
                let canvas_rect = Rect {
                    origin: Point2D::new(cx0, cy0),
                    size: Point2D::new(cw, ch),
                };
                let point = Point2D::new(x, y);
                if let Some(handle) = selection_handle_at_point(canvas_rect, &self.document, point)
                {
                    return CursorHint::for_handle(handle);
                }
                if rotation_corner_at_point(canvas_rect, &self.document, point).is_some() {
                    return CursorHint::Rotate;
                }
                let canvas_local = Point2D::new(x - cx0, y - cy0);
                let doc_point = self.document.viewport.to_document(canvas_local);
                if self.document.node_at_doc_point(doc_point).is_some() {
                    return CursorHint::Move;
                }
                CursorHint::Default
            }
        }
    }

    /// Canvas origin (logical px) — independent of viewport size,
    /// so cursor-move handlers (which don't carry vw/vh) can
    /// compute screen→doc conversions without re-passing the
    /// viewport. The full `canvas_region` is still required when
    /// the width/height of the canvas matters.
    pub(in crate::widget_host) fn canvas_origin(&self) -> (f32, f32) {
        let cx0 = if self.document.ui.sidebar_open {
            self.document.ui.layer_panel_width
        } else {
            0.0
        };
        (cx0, TOP_BAR_HEIGHT)
    }

    /// Canvas region (logical px, viewport-relative). Reflects
    /// the LayerPanel sidebar collapse state — when sidebar is
    /// hidden the canvas stretches to the left edge.
    pub(in crate::widget_host) fn canvas_region(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (f32, f32, f32, f32) {
        let canvas_left = if self.document.ui.sidebar_open {
            self.document.ui.layer_panel_width
        } else {
            0.0
        };
        let has_property = self.document.property_panel_visible();
        let canvas_right = if has_property {
            viewport_w - self.document.ui.property_panel_width
        } else {
            viewport_w
        };
        let canvas_w = (canvas_right - canvas_left).max(0.0);
        let canvas_h = (viewport_h - TOP_BAR_HEIGHT).max(0.0);
        (canvas_left, TOP_BAR_HEIGHT, canvas_w, canvas_h)
    }

    pub(in crate::widget_host) fn shape_picker_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Rect {
        let (cx0, _cy, cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let toolbar = Toolbar::for_document(&self.document);
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
        // Anchor to the right of the toolbar PANEL (not just the
        // button) plus a small breathing-room gap, so the dropdown
        // doesn't visually touch the toolbar edge.
        let max_x = cx0 + cw - SHAPE_PICKER_WIDTH - 4.0;
        let toolbar_right = toolbar_rect.origin.x + toolbar_rect.size.x;
        let x = (toolbar_right + 8.0).min(max_x);
        let y = slot.origin.y;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(SHAPE_PICKER_WIDTH, panel_h),
        }
    }

    pub(in crate::widget_host) fn locale_picker_rect(&self, viewport_w: f32) -> Rect {
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let globe = TopBar::globe_rect(top_bar_rect);
        let panel_h = LocalePicker::panel_height();
        // Anchor under the globe icon, right-aligned to its center
        // so the panel doesn't run off the right edge.
        let x = (globe.origin.x + globe.size.x / 2.0 - LOCALE_PICKER_WIDTH / 2.0)
            .max(8.0)
            .min(viewport_w - LOCALE_PICKER_WIDTH - 8.0);
        let y = globe.origin.y + globe.size.y + 6.0;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(LOCALE_PICKER_WIDTH, panel_h),
        }
    }

    pub(in crate::widget_host) fn ai_chat_size(&self) -> (f32, f32) {
        if self.document.chat.collapsed {
            (AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_COLLAPSED_HEIGHT)
        } else {
            (AI_CHAT_WIDTH, AI_CHAT_HEIGHT)
        }
    }

    pub(in crate::widget_host) fn ai_chat_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let (panel_w, panel_h) = self.ai_chat_size();
        if cw <= panel_w + AICHAT_INSET_LEFT + 16.0 || ch <= panel_h + 16.0 {
            return None;
        }
        if let Some(d) = self.chat_drag {
            return Some(Rect {
                origin: Point2D::new(d.pos_x, d.pos_y),
                size: Point2D::new(panel_w, panel_h),
            });
        }
        let (x, y) = match self.document.chat.anchor {
            ChatAnchor::TopLeft => (cx0 + AICHAT_INSET_LEFT, cy0 + AICHAT_INSET_BOTTOM),
            ChatAnchor::TopRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + AICHAT_INSET_BOTTOM,
            ),
            ChatAnchor::BottomLeft => (
                cx0 + AICHAT_INSET_LEFT,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
            ChatAnchor::BottomRight => (
                cx0 + cw - panel_w - AICHAT_INSET_BOTTOM,
                cy0 + ch - panel_h - AICHAT_INSET_BOTTOM,
            ),
        };
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(panel_w, panel_h),
        })
    }

    /// Resolve a screen point to an `AlignAction` if it lands on the
    /// floating align toolbar (visible when 2+ selected). Returns
    /// None when the toolbar isn't shown or the cursor misses every
    /// button. Used by press dispatch + cursor-move hover sync so
    /// the geometry stays in one place.
    pub(in crate::widget_host) fn align_toolbar_hit(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<openpencil_shell_core::document::AlignAction> {
        use openpencil_shell_core::widgets::AlignToolbar;
        use openpencil_shell_core::widgets::TOP_BAR_HEIGHT;
        let (cx, _, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_region = Rect {
            origin: Point2D::new(cx, TOP_BAR_HEIGHT),
            size: Point2D::new(cw, ch),
        };
        AlignToolbar::for_canvas_region(canvas_region, &self.document)?
            .hit_test(Point2D::new(x, y))
    }
}
