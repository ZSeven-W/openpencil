use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_ui::util::resize_bounds;
use op_editor_ui::widgets::{selection_handle_at_point, SelectionHandle};
use op_editor_ui::{Point2D, Rect};

use super::WidgetHost;

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct HandleDragState {
    pub(in crate::widget_host) handle: SelectionHandle,
    pub(in crate::widget_host) start_screen_x: f32,
    pub(in crate::widget_host) start_screen_y: f32,
    pub(in crate::widget_host) start_bounds: Rect,
    pub(in crate::widget_host) start_authored_x: Option<f64>,
    pub(in crate::widget_host) start_authored_y: Option<f64>,
}

impl WidgetHost {
    pub(in crate::widget_host) fn try_selection_handle_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_rect = Rect {
            origin: Point2D::new(cx0, cy0),
            size: Point2D::new(cw, ch),
        };
        let Some(handle) = selection_handle_at_point(
            canvas_rect,
            &self.layout_scene,
            &self.editor_state,
            Point2D::new(x, y),
        ) else {
            return false;
        };
        let selected_anchor = self.editor_state.selection.anchor.as_str().to_string();
        let Some(node) = self
            .layout_scene
            .active_page()
            .and_then(|page| page.find(&selected_anchor))
        else {
            return false;
        };
        let (start_authored_x, start_authored_y) = self
            .editor_state
            .selected_node()
            .map(|node| (node.base().x, node.base().y))
            .unwrap_or((None, None));
        let raw = node.bounds;
        if raw.size.x <= 0.0 && raw.size.y <= 0.0 {
            return false;
        }
        self.editor_state.commit_history();
        self.handle_drag = Some(HandleDragState {
            handle,
            start_screen_x: x,
            start_screen_y: y,
            start_bounds: raw,
            start_authored_x,
            start_authored_y,
        });
        true
    }

    pub(in crate::widget_host) fn apply_selection_handle_drag_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(drag) = self.handle_drag else {
            return false;
        };
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let dx = (x - drag.start_screen_x) / zoom;
        let dy = (y - drag.start_screen_y) / zoom;
        let new_bounds = resize_bounds(drag.start_bounds, drag.handle, dx, dy);
        let new_x = drag.handle.moves_left_edge().then(|| {
            drag.start_authored_x.unwrap_or(0.0)
                + f64::from(new_bounds.origin.x - drag.start_bounds.origin.x)
        });
        let new_y = drag.handle.moves_top_edge().then(|| {
            drag.start_authored_y.unwrap_or(0.0)
                + f64::from(new_bounds.origin.y - drag.start_bounds.origin.y)
        });
        self.editor_state.resize_selected_bounds(
            rect_to_doc_rect(new_bounds),
            drag.handle.resize_axes(),
            new_x,
            new_y,
        );
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn release_selection_handle_drag(&mut self) -> bool {
        self.handle_drag.take().is_some()
    }
}

fn rect_to_doc_rect(r: Rect) -> op_editor_core::DocRect {
    op_editor_core::DocRect {
        x: r.origin.x as f64,
        y: r.origin.y as f64,
        w: r.size.x as f64,
        h: r.size.y as f64,
    }
}
