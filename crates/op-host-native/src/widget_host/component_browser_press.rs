//! Component-Browser panel press dispatch.
//!
//! Mirrors `design_md_press.rs`: a deliberately-opened top-most
//! floating panel that owns clicks on its rect ahead of every lower
//! layer. Hit kinds: close, drag-header, category-pill, insert.

use op_editor_ui::widgets::{ComponentBrowserHit, ComponentBrowserPanel};
use op_editor_ui::Point2D;

use super::{ComponentBrowserDragState, WidgetHostNative};

impl WidgetHostNative {
    /// Dispatch a press inside the floating Component-Browser panel.
    ///
    /// Returns `true` when consumed — a close, a category-pill swap,
    /// a queued insert request, a drag start, or a swallowed click.
    pub(in crate::widget_host) fn dispatch_component_browser_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.component_browser_panel_rect(viewport_width, viewport_height)
        else {
            return false;
        };
        let hit = ComponentBrowserPanel::for_editor(&self.editor_state)
            .and_then(|p| p.hit_test(panel_rect, Point2D::new(x, y)));
        let Some(hit) = hit else {
            return false;
        };
        match hit {
            ComponentBrowserHit::Close => {
                self.editor_state.editor_ui.component_browser_open = false;
            }
            ComponentBrowserHit::DragHeader => {
                self.component_browser_drag = Some(ComponentBrowserDragState {
                    grab_dx: x - panel_rect.origin.x,
                    grab_dy: y - panel_rect.origin.y,
                });
            }
            ComponentBrowserHit::SelectCategory(cat) => {
                self.editor_state.editor_ui.component_browser_category = cat;
            }
            ComponentBrowserHit::InsertComponent(kit_id, comp_id) => {
                // The desktop host drains this against the viewport
                // centre — it owns the viewport metrics needed to
                // compute the document-space drop point.
                self.editor_state.editor_ui.component_browser_pending_insert =
                    Some((kit_id, comp_id));
            }
            ComponentBrowserHit::Inside => {}
        }
        self.mark_dirty();
        true
    }
}
