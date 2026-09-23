//! Wheel and trackpad scrolling for the Studio Home page stack.

use super::WidgetHostNative;
use op_editor_ui::widgets::home_surface::{
    max_scroll_for_mode, model_chip_width, HOME_BOTTOM_NAV_H, HOME_TOPBAR_H,
};

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
        // The top bar (and, on the phone, the bottom nav) stays pinned;
        // anything between them scrolls the page.
        let compact = self.editor_state.editor_ui.compact_layout();
        let lower_bound = if compact {
            viewport_height - HOME_BOTTOM_NAV_H
        } else {
            viewport_height
        };
        if y < HOME_TOPBAR_H || y > lower_bound {
            return false;
        }
        let _ = x;
        let label = op_editor_ui::widgets::home_surface::model_chip_label(&self.editor_state);
        let chip_w = model_chip_width(&label);
        let max_scroll = self.home_max_scroll(viewport_width, viewport_height, chip_w, compact);
        let home = &mut self.editor_state.editor_ui.home;
        let next = (home.scroll_y - delta_y).clamp(0.0, max_scroll);
        if (next - home.scroll_y).abs() > f32::EPSILON {
            home.scroll_y = next;
            self.mark_dirty();
        }
        true
    }

    /// The Home page's furthest scroll against the height the reader can
    /// actually see. A raised software keyboard does not shorten the
    /// viewport the page is laid out in — it covers its bottom — so the
    /// scroll range has to grow by the covered band or the content under
    /// the keyboard becomes unreachable. With no keyboard up this is the
    /// plain viewport height and the range is unchanged.
    pub(in crate::widget_host) fn home_max_scroll(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        chip_w: f32,
        compact: bool,
    ) -> f32 {
        max_scroll_for_mode(
            viewport_width,
            self.keyboard_visible_bottom(viewport_height),
            self.editor_state.editor_ui.home.task,
            chip_w,
            compact,
        )
    }
}
