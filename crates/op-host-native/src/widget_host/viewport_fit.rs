//! Public viewport-fit adapter for desktop and mobile host lifecycle changes.

use super::WidgetHostNative;

impl WidgetHostNative {
    /// Rebuild the newly-active page scene before deriving its fit. Page
    /// navigation must use this helper instead of calling `zoom_to_fit`
    /// immediately after mutating `editor_state`, otherwise the lazy scene
    /// gate can still expose the previously painted page's bounds.
    pub(in crate::widget_host) fn fit_active_page_after_switch(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        self.mark_dirty();
        self.zoom_to_fit(viewport_w, viewport_h);
    }

    pub fn fit_content_to_viewport(&mut self, viewport_w: f32, viewport_h: f32) {
        // Ordinary Works shows one board. A post-load or resize fit must not
        // undo the reader camera by framing the entire multi-board canvas.
        if self.works_reader_visible() {
            self.frame_reader_board(viewport_w, viewport_h);
        } else {
            self.zoom_to_fit(viewport_w, viewport_h);
        }
    }
}
