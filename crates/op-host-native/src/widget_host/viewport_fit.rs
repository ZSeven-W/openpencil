//! Public viewport-fit adapter for desktop runner startup.

use super::WidgetHostNative;

impl WidgetHostNative {
    pub fn fit_content_to_viewport(&mut self, viewport_w: f32, viewport_h: f32) {
        self.zoom_to_fit(viewport_w, viewport_h);
    }
}
