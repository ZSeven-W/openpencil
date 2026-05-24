//! Icon-picker panel press dispatch.

use op_editor_ui::widgets::{IconPickerHit, IconPickerPanel};
use op_editor_ui::Point2D;

use super::WidgetHostNative;

impl WidgetHostNative {
    pub(in crate::widget_host) fn dispatch_icon_picker_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.icon_picker_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        let hit = IconPickerPanel::for_editor(&self.editor_state)
            .and_then(|p| p.hit_test(panel_rect, Point2D::new(x, y)));
        let Some(hit) = hit else {
            return false;
        };
        match hit {
            IconPickerHit::Close => {
                self.editor_state.editor_ui.icon_picker_open = false;
                self.editor_state.editor_ui.icon_picker_search.clear();
            }
            IconPickerHit::SelectIcon(name) => {
                let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_width, viewport_height);
                let doc = self
                    .editor_state
                    .viewport
                    .to_document(Point2D::new(cw / 2.0, ch / 2.0));
                let inserted = self.editor_state.insert_icon_font_node_at(
                    &name,
                    "lucide",
                    doc.x as f64,
                    doc.y as f64,
                );
                self.editor_state.editor_ui.icon_picker_open = false;
                self.editor_state.editor_ui.icon_picker_search.clear();
                if inserted.is_some() {
                    self.mark_dirty();
                }
            }
            IconPickerHit::Inside => {}
        }
        self.mark_dirty();
        true
    }
}
