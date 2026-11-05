//! Canvas comments — this host's thin arm over the shared
//! `op_editor_ui::widgets::comment_*` flows: paint the pins and the
//! popover at their z-order slots, route presses at the matching ladder
//! positions, and run the repaint tail. Everything else (anchoring,
//! lifecycle, collaboration gating, undo) is single-sourced.

use op_editor_ui::widgets::{comment_flow, comment_pins, comment_popover, PaintCx};
use op_editor_ui::{Point2D, RenderBackend};

use super::WidgetHostNative;

impl WidgetHostNative {
    /// Pins: right after the canvas, below every panel.
    pub(in crate::widget_host) fn paint_comment_pins_layer(
        &self,
        backend: &mut dyn RenderBackend,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        comment_pins::paint_comment_pins(
            &mut PaintCx { backend },
            &self.editor_state,
            &self.layout_scene,
            viewport_width,
            viewport_height,
        );
    }

    /// Popover: bottom of the top-most overlay band.
    pub(in crate::widget_host) fn paint_comment_popover_layer(
        &mut self,
        backend: &mut dyn RenderBackend,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        comment_popover::paint_comment_popover(
            &mut PaintCx { backend },
            &mut self.editor_state,
            &self.layout_scene,
            viewport_width,
            viewport_height,
        );
    }

    /// Top-most overlay tier: a press on the popover card.
    pub(in crate::widget_host) fn press_comment_popover(&mut self, x: f32, y: f32) -> bool {
        let press =
            comment_flow::press_popover(&mut self.editor_state, Point2D::new(x, y), self.now_ms);
        if press.changed {
            self.mark_dirty();
        }
        press.consumed
    }

    /// Canvas tier, ahead of tool dispatch: pins and comment mode.
    pub(in crate::widget_host) fn press_comment_canvas(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let press = comment_flow::press_canvas(
            &mut self.editor_state,
            &self.layout_scene,
            Point2D::new(x, y),
            viewport_width,
            viewport_height,
        );
        if press.changed {
            self.mark_dirty();
        }
        press.consumed
    }

    /// `C` — arm / disarm comment mode (same as the toolbar button).
    pub fn apply_toggle_comment_mode(&mut self) -> bool {
        self.dispatch_toolbar_action(op_editor_ui::widgets::ToolbarAction::ToggleComments)
    }

    /// Hover wash on the popover's buttons.
    pub(in crate::widget_host) fn hover_comment_popover(&mut self, x: f32, y: f32) {
        if comment_flow::hover_popover(&mut self.editor_state, Point2D::new(x, y)) {
            self.mark_dirty();
        }
    }
}
