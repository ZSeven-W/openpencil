use super::WidgetHost;

const NODE_DRAG_THRESHOLD_PX: f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct NodeDragState {
    pub(in crate::widget_host) last_screen_x: f32,
    pub(in crate::widget_host) last_screen_y: f32,
    pub(in crate::widget_host) press_screen_x: f32,
    pub(in crate::widget_host) press_screen_y: f32,
    pub(in crate::widget_host) moved: bool,
    pub(in crate::widget_host) total_dx: f64,
    pub(in crate::widget_host) total_dy: f64,
}

impl WidgetHost {
    pub(in crate::widget_host) fn start_node_drag(&mut self, x: f32, y: f32) {
        self.editor_state.commit_history();
        self.node_drag = Some(NodeDragState {
            last_screen_x: x,
            last_screen_y: y,
            press_screen_x: x,
            press_screen_y: y,
            moved: false,
            total_dx: 0.0,
            total_dy: 0.0,
        });
    }

    pub(in crate::widget_host) fn apply_node_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        let drag = self.node_drag?;
        if !drag.moved
            && (x - drag.press_screen_x).abs() <= NODE_DRAG_THRESHOLD_PX
            && (y - drag.press_screen_y).abs() <= NODE_DRAG_THRESHOLD_PX
        {
            return Some(false);
        }
        if !drag.moved {
            if let Some(d) = self.node_drag.as_mut() {
                d.moved = true;
            }
        }

        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        if let Some(d) = self.node_drag.as_mut() {
            d.total_dx = ((x - d.press_screen_x) / zoom) as f64;
            d.total_dy = ((y - d.press_screen_y) / zoom) as f64;
        }
        let dx = (x - drag.last_screen_x) / zoom;
        let dy = (y - drag.last_screen_y) / zoom;
        if dx == 0.0 && dy == 0.0 {
            return Some(false);
        }

        if let Some(drag) = self.node_drag.as_mut() {
            drag.last_screen_x = x;
            drag.last_screen_y = y;
        }
        if self.editor_state.translate_selected(dx as f64, dy as f64) {
            self.mark_dirty();
        } else {
            self.editor_state.editor_ui.active_guides.clear();
        }
        Some(true)
    }

    pub(in crate::widget_host) fn release_node_drag(&mut self) -> bool {
        let Some(_drag) = self.node_drag.take() else {
            return false;
        };
        self.editor_state.editor_ui.active_guides.clear();
        self.mark_dirty();
        true
    }
}
