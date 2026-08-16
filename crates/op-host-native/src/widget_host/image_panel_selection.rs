//! Mouse selection for Search / Generate popover text inputs.
//!
//! The state walk lives in the shared
//! `op_editor_ui::widgets::image_popover_input_flow` (the web twin drives
//! the same functions); this file only threads the host's geometry cache
//! + active drag through it.

use super::WidgetHostNative;
use op_editor_ui::widgets::image_popover_input_flow as input_flow;
use op_editor_ui::widgets::property_panel_image_assets::ImagePopoverInputKind;
use op_editor_ui::widgets::PropertyPanel;
use op_editor_ui::{Point2D, Rect};

pub(in crate::widget_host) type ImageInputSelectionDragState = input_flow::ImageInputSelectionDrag;

impl WidgetHostNative {
    pub(in crate::widget_host) fn image_popover_input_at(
        &self,
        panel: &PropertyPanel,
        rect: Rect,
        point: Point2D,
    ) -> Option<(ImagePopoverInputKind, usize)> {
        input_flow::input_at(
            &self.editor_state,
            self.image_input_geometry.as_ref(),
            panel,
            rect,
            point,
        )
    }

    pub(in crate::widget_host) fn cached_image_input_caret_rect(&self) -> Option<Rect> {
        input_flow::cached_caret_rect(&self.editor_state, self.image_input_geometry.as_ref())
    }

    pub(in crate::widget_host) fn begin_image_input_selection_drag(
        &mut self,
        kind: ImagePopoverInputKind,
        offset: usize,
    ) -> bool {
        let extend = self.shift_held;
        let now_ms = self.now_ms;
        let Some(drag) =
            input_flow::begin_selection_drag(&mut self.editor_state, kind, offset, extend, now_ms)
        else {
            return false;
        };
        self.image_input_selection_drag = Some(drag);
        self.mark_dirty();
        true
    }

    fn image_input_drag_offset_at_screen(
        &mut self,
        kind: ImagePopoverInputKind,
        x: f32,
        y: f32,
    ) -> Option<usize> {
        let point = Point2D::new(x, y);
        if let Some(offset) = input_flow::cached_drag_offset(
            &self.editor_state,
            self.image_input_geometry.as_ref(),
            kind,
            point,
        ) {
            return Some(offset);
        }
        self.refresh_layout_scene();
        let rect = self.property_rect(self.last_viewport_w, self.last_viewport_h);
        let panel = PropertyPanel::for_selection(&self.editor_state)?;
        panel.image_popover_input_drag_offset_at(rect, kind, point)
    }

    pub(in crate::widget_host) fn apply_image_input_selection_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(drag) = self.image_input_selection_drag else {
            return false;
        };
        let Some(focus) = self.image_input_drag_offset_at_screen(drag.kind, x, y) else {
            self.image_input_selection_drag = None;
            return false;
        };
        self.drag_image_input_selection_to(drag, focus)
    }

    pub(in crate::widget_host) fn drag_image_input_selection_to(
        &mut self,
        drag: ImageInputSelectionDragState,
        focus: usize,
    ) -> bool {
        let now_ms = self.now_ms;
        let Some(changed) =
            input_flow::drag_selection_to(&mut self.editor_state, drag, focus, now_ms)
        else {
            return false;
        };
        if changed {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn clear_image_input_selection_drag(&mut self) {
        self.image_input_selection_drag = None;
    }
}
