//! Shared geometry helpers for the web widget host.

use super::{WidgetHost, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use op_editor_ui::widgets::{
    LayoutCx, LocalePicker, Toolbar, TopBar, Widget, LOCALE_PICKER_WIDTH, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    pub(in crate::widget_host) fn top_bar(&self) -> TopBar {
        TopBar::for_editor_ui(&self.editor_state.editor_ui).with_traffic_controls(false)
    }

    pub(in crate::widget_host) fn top_bar_rect(&self, viewport_w: f32) -> Rect {
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        }
    }

    pub(in crate::widget_host) fn canvas_region(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (f32, f32, f32, f32) {
        let canvas_left = if self.editor_state.editor_ui.sidebar_open {
            self.editor_state.editor_ui.layer_panel_width
        } else {
            0.0
        };
        let rail_occupied = self.editor_state.right_rail_visible();
        let canvas_right = if rail_occupied {
            viewport_w - self.editor_state.editor_ui.property_panel_width
        } else {
            viewport_w
        };
        let canvas_w = (canvas_right - canvas_left).max(0.0);
        let canvas_h = (viewport_h - TOP_BAR_HEIGHT).max(0.0);
        (canvas_left, TOP_BAR_HEIGHT, canvas_w, canvas_h)
    }

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

    pub(in crate::widget_host) fn locale_picker_rect(&self, viewport_w: f32) -> Rect {
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let globe = TopBar::globe_rect(top_bar_rect);
        let panel_h = LocalePicker::panel_height();
        let x = (globe.origin.x + globe.size.x / 2.0 - LOCALE_PICKER_WIDTH / 2.0)
            .max(8.0)
            .min(viewport_w - LOCALE_PICKER_WIDTH - 8.0);
        let y = globe.origin.y + globe.size.y + 6.0;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(LOCALE_PICKER_WIDTH, panel_h),
        }
    }

    pub(in crate::widget_host) fn layer_panel_rect(&self, viewport_h: f32) -> Rect {
        Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.editor_state.editor_ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        }
    }

    pub(in crate::widget_host) fn toolbar_rect(&mut self, viewport_w: f32) -> Rect {
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_w, f32::INFINITY);
        self.refresh_layout_scene();
        let toolbar = Toolbar::for_editor(&self.editor_state);
        let h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, h),
        }
    }

    /// Per-button hover wash on the floating toolbar. Mirrors
    /// `op_host_native::widget_host::geometry::update_toolbar_hover`.
    /// Returns `true` if the hover state changed.
    pub(in crate::widget_host) fn update_toolbar_hover(&mut self, x: f32, y: f32) -> bool {
        let rect = self.toolbar_rect(self.last_viewport_w);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        let new_hover = toolbar
            .hit_test(rect, Point2D::new(x, y))
            .map(op_editor_ui::widgets::editor_state_ext::toolbar_hover);
        if new_hover != self.editor_state.editor_ui.toolbar_hover {
            self.editor_state.editor_ui.toolbar_hover = new_hover;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub(in crate::widget_host) fn path_anchor_hit(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<(String, usize, super::AnchorDragTarget)> {
        use super::AnchorDragTarget;
        use op_editor_core::pen::PathHandleSide;
        use op_editor_ui::layout_scene::NodeKind;
        use op_editor_ui::widgets::path_handle_positions;
        if !matches!(
            self.editor_state.tool,
            op_editor_core::Tool::Pen | op_editor_core::Tool::Select
        ) {
            return None;
        }
        if self.editor_state.selection_count() != 1 {
            return None;
        }
        let sel = self.editor_state.selection.anchor.as_str().to_string();
        let node = self.layout_scene.active_page()?.find(&sel)?;
        if !matches!(node.kind, NodeKind::Path) {
            return None;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let mut doc = self.editor_state.viewport.to_document(canvas_local);
        if node.rotation.abs() > f32::EPSILON {
            let b = node.aggregate_bounds();
            let centre = Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
            doc = op_editor_ui::widgets::rotate_point(doc, centre, -node.rotation);
        }
        let r2 = 64.0 / (zoom * zoom);
        let hit = |p: Point2D| (doc.x - p.x).powi(2) + (doc.y - p.y).powi(2) <= r2;
        let pen_tool = matches!(self.editor_state.tool, op_editor_core::Tool::Pen);
        for (i, a) in node.path_anchors.iter().enumerate() {
            let (hin, hout) = path_handle_positions(a, zoom);
            if (a.handle_out.is_some() || pen_tool) && hit(hout) {
                return Some((
                    sel.clone(),
                    i,
                    AnchorDragTarget::Handle(PathHandleSide::Out),
                ));
            }
            if (a.handle_in.is_some() || pen_tool) && hit(hin) {
                return Some((sel.clone(), i, AnchorDragTarget::Handle(PathHandleSide::In)));
            }
        }
        for (i, a) in node.path_anchors.iter().enumerate() {
            if hit(a.pos) {
                return Some((sel.clone(), i, AnchorDragTarget::Anchor));
            }
        }
        if node.path_anchors.is_empty() {
            for (i, p) in node.points.iter().enumerate() {
                if hit(*p) {
                    return Some((sel.clone(), i, AnchorDragTarget::Anchor));
                }
            }
        }
        None
    }
}
