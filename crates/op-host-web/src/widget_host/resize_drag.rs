use op_editor_ui::widgets::{selection_handle_at_point, SelectionHandle};
use op_editor_ui::{Point2D, Rect};

use super::WidgetHost;

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct HandleDragState {
    pub(in crate::widget_host) handle: SelectionHandle,
    pub(in crate::widget_host) start_screen_x: f32,
    pub(in crate::widget_host) start_screen_y: f32,
    pub(in crate::widget_host) start_bounds: Rect,
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
        self.editor_state
            .set_selected_bounds(rect_to_doc_rect(new_bounds));
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn release_selection_handle_drag(&mut self) -> bool {
        self.handle_drag.take().is_some()
    }
}

fn resize_bounds(start: Rect, handle: SelectionHandle, dx: f32, dy: f32) -> Rect {
    let mut x = start.origin.x;
    let mut y = start.origin.y;
    let mut w = start.size.x;
    let mut h = start.size.y;
    match handle {
        SelectionHandle::TopLeft => {
            x += dx;
            y += dy;
            w -= dx;
            h -= dy;
        }
        SelectionHandle::Top => {
            y += dy;
            h -= dy;
        }
        SelectionHandle::TopRight => {
            y += dy;
            w += dx;
            h -= dy;
        }
        SelectionHandle::Right => {
            w += dx;
        }
        SelectionHandle::BottomRight => {
            w += dx;
            h += dy;
        }
        SelectionHandle::Bottom => {
            h += dy;
        }
        SelectionHandle::BottomLeft => {
            x += dx;
            w -= dx;
            h += dy;
        }
        SelectionHandle::Left => {
            x += dx;
            w -= dx;
        }
    }
    if w < 1.0 {
        if matches!(
            handle,
            SelectionHandle::Left | SelectionHandle::TopLeft | SelectionHandle::BottomLeft
        ) {
            x = start.origin.x + start.size.x - 1.0;
        }
        w = 1.0;
    }
    if h < 1.0 {
        if matches!(
            handle,
            SelectionHandle::Top | SelectionHandle::TopLeft | SelectionHandle::TopRight
        ) {
            y = start.origin.y + start.size.y - 1.0;
        }
        h = 1.0;
    }
    Rect::xywh(x, y, w, h)
}

fn rect_to_doc_rect(r: Rect) -> op_editor_core::DocRect {
    op_editor_core::DocRect {
        x: r.origin.x as f64,
        y: r.origin.y as f64,
        w: r.size.x as f64,
        h: r.size.y as f64,
    }
}
