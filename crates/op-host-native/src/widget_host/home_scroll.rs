//! Wheel and trackpad scrolling for the Studio Home page stack.

use super::WidgetHostNative;
use op_editor_ui::widgets::home_surface::{model_chip_width, HomeSurface, HOME_TOPBAR_H};

impl WidgetHostNative {
    pub(in crate::widget_host) fn try_scroll_home(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.home_visible() {
            return false;
        }
        // The top bar stays pinned; anything below it scrolls the page.
        if y < HOME_TOPBAR_H {
            return false;
        }
        let _ = x;
        let label = op_editor_ui::widgets::home_surface::model_chip_label(&self.editor_state);
        let max_scroll = HomeSurface::max_scroll_for(
            viewport_width,
            viewport_height,
            self.editor_state.editor_ui.home.task,
            model_chip_width(&label),
        );
        let home = &mut self.editor_state.editor_ui.home;
        let next = (home.scroll_y - delta_y).clamp(0.0, max_scroll);
        if (next - home.scroll_y).abs() > f32::EPSILON {
            home.scroll_y = next;
            self.mark_dirty();
        }
        true
    }
}
