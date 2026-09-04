//! Per-button hover wash for the floating tool column.
//!
//! Split out of `geometry.rs` to keep that file under the 800-line
//! cap. The two methods here are tightly coupled — `toolbar_rect`
//! centralises the inset + intrinsic-height math, and
//! `update_toolbar_hover` is the only caller that needs the
//! sidebar-collapse-aware version (paint / press / click each
//! inline equivalents against their own backend / dpi context).

use super::WidgetHostNative;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::Toolbar;
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// The on-screen rect of the floating tool column. Forwards to the
    /// shared `host_canvas_geometry` placement (which derives the origin
    /// from `canvas_origin`, per the coordinate invariant) so paint /
    /// press / click hit-test against the same bounds as the web host.
    ///
    /// The viewport arguments are vestigial — the placement only needs
    /// the canvas origin — but the signature is part of this host's
    /// internal surface, so it stays.
    pub(in crate::widget_host) fn toolbar_rect(&self, _viewport_w: f32, _viewport_h: f32) -> Rect {
        canvas_geometry::toolbar_rect_for(&self.editor_state)
    }

    /// Update the per-button hover wash on the vertical toolbar
    /// from the current cursor position. `over_topmost` suppresses
    /// updates when a floating panel covers the chrome — the panel
    /// already eats the click, so the toolbar must not light up
    /// underneath it. Returns `true` if the hover state changed.
    pub(in crate::widget_host) fn update_toolbar_hover(
        &mut self,
        x: f32,
        y: f32,
        over_topmost: bool,
    ) -> bool {
        let new_hover = if over_topmost || self.editor_state.editor_ui.touch_chrome() {
            None
        } else {
            self.refresh_layout_scene();
            let toolbar_rect = self.toolbar_rect(self.last_viewport_w, self.last_viewport_h);
            let toolbar = Toolbar::for_editor(&self.editor_state);
            toolbar
                .hit_test(toolbar_rect, Point2D::new(x, y))
                .map(op_editor_ui::widgets::editor_state_ext::toolbar_hover)
        };
        if new_hover != self.editor_state.editor_ui.toolbar_hover {
            self.editor_state.editor_ui.toolbar_hover = new_hover;
            self.mark_dirty();
            true
        } else {
            false
        }
    }
}
