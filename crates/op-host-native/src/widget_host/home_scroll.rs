//! Wheel and trackpad scrolling for the Home drafting-table stack.

use super::WidgetHostNative;
use op_editor_ui::widgets::{HomeSurface, HOME_TOPBAR_H};

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
        let footer_top = viewport_height - 14.0 - 22.0;
        if y < HOME_TOPBAR_H || y > footer_top {
            return false;
        }
        let _ = x;
        let max_scroll = HomeSurface::max_scroll_for(
            viewport_width,
            viewport_height,
            self.editor_state.editor_ui.home.bound,
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
