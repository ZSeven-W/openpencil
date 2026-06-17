//! Shape / Frame / Text click-drag creation for the web host.

use super::WidgetHost;
use op_editor_core::{DocRect, Tool};
use op_editor_ui::Point2D;

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct CreateDragState {
    pub(in crate::widget_host) start_doc_x: f32,
    pub(in crate::widget_host) start_doc_y: f32,
    pub(in crate::widget_host) tool: Tool,
    pub(in crate::widget_host) node_created: bool,
}

fn initial_size_for_tool(tool: Tool) -> (f64, f64) {
    if matches!(tool, Tool::Text) {
        (
            f64::from(op_editor_core::DEFAULT_TEXT_NODE_WIDTH),
            f64::from(op_editor_core::DEFAULT_TEXT_NODE_HEIGHT),
        )
    } else {
        (1.0, 1.0)
    }
}

fn min_size_for_tool(tool: Tool) -> (f32, f32) {
    if matches!(tool, Tool::Text) {
        (
            op_editor_core::DEFAULT_TEXT_NODE_WIDTH as f32,
            op_editor_core::DEFAULT_TEXT_NODE_HEIGHT as f32,
        )
    } else {
        (1.0, 1.0)
    }
}

fn defers_until_drag(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Rect | Tool::Ellipse | Tool::Polygon | Tool::Line | Tool::Frame
    )
}

fn draw_size_exceeds_threshold(tool: Tool, raw_w: f32, raw_h: f32) -> bool {
    if matches!(tool, Tool::Line) {
        raw_w.hypot(raw_h) >= 2.0
    } else {
        raw_w >= 2.0 && raw_h >= 2.0
    }
}

impl WidgetHost {
    pub(in crate::widget_host) fn start_create_drag_at(&mut self, doc_point: Point2D) -> bool {
        let tool = self.editor_state.tool;
        if defers_until_drag(tool) {
            self.create_drag = Some(CreateDragState {
                start_doc_x: doc_point.x,
                start_doc_y: doc_point.y,
                tool,
                node_created: false,
            });
            return true;
        }

        let pre_create = self.editor_state.snapshot_for_history();
        let is_text_tool = matches!(tool, Tool::Text);
        let (init_w, init_h) = initial_size_for_tool(tool);
        let Some(node_id) = self.editor_state.create_node_for_tool(
            tool,
            &mut self.next_node_id,
            doc_point.x as f64,
            doc_point.y as f64,
            init_w,
            init_h,
        ) else {
            return false;
        };

        self.editor_state.history_push_past(pre_create);
        self.editor_state.set_single_selection(node_id.clone());
        if is_text_tool && self.editor_state.start_text_edit(node_id) {
            self.editor_state.ui.text_edit_input.touch(self.now_ms);
            self.editor_state.text_edit_select_all_now(self.now_ms);
            // Creating a text node already pushed the pre-create
            // snapshot. Let the first typed burst share that undo entry
            // instead of deep-cloning a large document on the first key.
            self.editor_state.ui.text_edit_last_ms = self.now_ms.max(1);
        }
        self.create_drag = Some(CreateDragState {
            start_doc_x: doc_point.x,
            start_doc_y: doc_point.y,
            tool,
            node_created: true,
        });
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn update_create_drag(&mut self, x: f32, y: f32) -> bool {
        let Some(drag) = self.create_drag else {
            return false;
        };

        let (cx0, cy0, _, _) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let cur = self.editor_state.viewport.to_document(canvas_local);
        let min_x = drag.start_doc_x.min(cur.x);
        let min_y = drag.start_doc_y.min(cur.y);
        let raw_w = (drag.start_doc_x - cur.x).abs();
        let raw_h = (drag.start_doc_y - cur.y).abs();
        let (min_w, min_h) = min_size_for_tool(drag.tool);
        let w = raw_w.max(min_w);
        let h = raw_h.max(min_h);

        if !drag.node_created {
            if !draw_size_exceeds_threshold(drag.tool, raw_w, raw_h) {
                return false;
            }
            let pre_create = self.editor_state.snapshot_for_history();
            let Some(node_id) = self.editor_state.create_node_for_tool(
                drag.tool,
                &mut self.next_node_id,
                f64::from(min_x),
                f64::from(min_y),
                f64::from(w),
                f64::from(h),
            ) else {
                self.create_drag = None;
                return false;
            };
            self.editor_state.history_push_past(pre_create);
            self.editor_state.set_single_selection(node_id);
            self.create_drag = Some(CreateDragState {
                node_created: true,
                ..drag
            });
        }

        self.editor_state.set_selected_bounds(DocRect {
            x: f64::from(min_x),
            y: f64::from(min_y),
            w: f64::from(w),
            h: f64::from(h),
        });
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn release_create_drag(&mut self) -> bool {
        if self.create_drag.take().is_none() {
            return false;
        }
        self.editor_state.tool = Tool::Select;
        self.mark_dirty();
        true
    }
}
