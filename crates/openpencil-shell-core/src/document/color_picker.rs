//! Color-picker mutators on `Document`. Reads/writes the
//! `Document.ui.color_picker` state and applies HSV → RGB to the
//! selected node's fill or stroke on every drag step.

use super::*;
use crate::widgets::color_picker as picker;

impl Document {
    /// Open the floating color picker on the given target. Seeds
    /// HSV from the currently-selected node's fill / stroke colour
    /// so the SV box + hue slider land on the right anchor.
    /// Returns false when there's no selection to edit.
    pub fn open_color_picker(&mut self, target: ColorTarget, anchor_y: f32) -> bool {
        let Some(node) = self.selected_node() else {
            return false;
        };
        let current = match target {
            ColorTarget::Fill => node.fill.unwrap_or(crate::Color::BLACK),
            ColorTarget::Stroke => node.stroke.map(|s| s.color).unwrap_or(crate::Color::BLACK),
        };
        let (h, s, v) = picker::rgb_to_hsv(current);
        let pre_snap = Some(self.snapshot_for_history());
        self.ui.color_picker = Some(ColorPickerState {
            target,
            hue: h,
            sat: s,
            val: v,
            pre_snap,
            drag: None,
            anchor_y,
        });
        true
    }

    /// Update the picker HSV and live-apply the resulting RGB to
    /// the selected node. Tolerates `color_picker = None` so hosts
    /// can pipe move events unconditionally.
    pub fn color_picker_set_hsv(&mut self, hue: f32, sat: f32, val: f32) -> bool {
        let Some(state) = self.ui.color_picker.as_mut() else {
            return false;
        };
        state.hue = hue.rem_euclid(360.0);
        state.sat = sat.clamp(0.0, 1.0);
        state.val = val.clamp(0.0, 1.0);
        let target = state.target;
        let rgb = picker::hsv_to_rgb(state.hue, state.sat, state.val);
        let is_fill = matches!(target, ColorTarget::Fill);
        self.set_selected_color(is_fill, rgb);
        true
    }

    /// Update the active drag kind (so apply_cursor_move can route
    /// the move to the right control).
    pub fn color_picker_set_drag(&mut self, drag: Option<ColorPickerDrag>) {
        if let Some(state) = self.ui.color_picker.as_mut() {
            state.drag = drag;
        }
    }

    /// Close the picker. Pushes the pre-edit snapshot onto undo if
    /// the colour actually changed; otherwise drops it. Returns
    /// true when a picker was open.
    pub fn close_color_picker(&mut self) -> bool {
        let Some(state) = self.ui.color_picker.take() else {
            return false;
        };
        if let Some(snap) = state.pre_snap {
            let changed = self
                .selected_node()
                .map(|n| {
                    let before = match state.target {
                        ColorTarget::Fill => snap
                            .pages
                            .iter()
                            .find_map(|p| p.find(n.id))
                            .and_then(|nn| nn.fill),
                        ColorTarget::Stroke => snap
                            .pages
                            .iter()
                            .find_map(|p| p.find(n.id))
                            .and_then(|nn| nn.stroke.map(|s| s.color)),
                    };
                    let after = match state.target {
                        ColorTarget::Fill => n.fill,
                        ColorTarget::Stroke => n.stroke.map(|s| s.color),
                    };
                    before != after
                })
                .unwrap_or(false);
            if changed {
                self.history_push_past(snap);
            }
        }
        true
    }
}
