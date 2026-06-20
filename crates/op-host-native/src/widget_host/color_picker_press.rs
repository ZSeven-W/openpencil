//! Colour-picker overlay press dispatch — extracted from `press.rs`
//! to keep that file under the repo's 800-line cap.

use op_editor_ui::widgets::color_picker::{drag_for_hit, ColorPicker, ColorPickerHit};
use op_editor_ui::Point2D;

use super::WidgetHostNative;

impl WidgetHostNative {
    /// Dispatch a press against the floating colour-picker overlay.
    ///
    /// Returns `true` when the press was consumed. Returns `false`
    /// when the press fell outside the panel — the picker is closed
    /// as a side effect and the caller keeps hit-testing.
    pub(in crate::widget_host) fn dispatch_color_picker_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(state) = self.editor_state.ui.color_picker.clone() else {
            return false;
        };
        let picker = ColorPicker::for_state(&self.editor_state, state.clone());
        let panel = picker.rect(viewport_width, viewport_height);
        let point = Point2D::new(x, y);
        match picker.hit_test(panel, point) {
            Some(ColorPickerHit::Close) => {
                let _ = self.editor_state.close_color_picker();
                self.mark_dirty();
                true
            }
            Some(ColorPickerHit::HexInput) => {
                self.editor_state.color_picker_focus_hex();
                self.mark_dirty();
                true
            }
            Some(ColorPickerHit::Eyedropper) | Some(ColorPickerHit::Inside) => {
                // A press elsewhere in the panel commits + blurs the hex field.
                self.editor_state.color_picker_blur_hex();
                self.mark_dirty();
                true
            }
            Some(hit @ (ColorPickerHit::SvBox | ColorPickerHit::HueSlider)) => {
                self.editor_state.color_picker_blur_hex();
                if let Some(kind) = drag_for_hit(hit) {
                    // Live-apply once for the press point. Instance
                    // anchors route through the override redirect
                    // (GAP #10 choke point — see property_dispatch).
                    let instance_scope = self.editor_state.begin_instance_write_for_anchor();
                    match hit {
                        ColorPickerHit::SvBox => {
                            let (s, v) = picker.sv_at(panel, point);
                            let _ = self.editor_state.color_picker_set_hsv(state.hue, s, v);
                        }
                        ColorPickerHit::HueSlider => {
                            let h = picker.hue_at(panel, point);
                            let _ = self
                                .editor_state
                                .color_picker_set_hsv(h, state.sat, state.val);
                        }
                        _ => {}
                    }
                    if let Some(scope) = instance_scope {
                        self.editor_state.finish_instance_write(scope);
                    }
                    // `drag_for_hit` returns op-editor-core's
                    // `ColorPickerDrag` — no translation needed.
                    self.editor_state.color_picker_set_drag(Some(kind));
                }
                self.mark_dirty();
                true
            }
            None => {
                // Press outside the panel — close it; the caller's
                // remaining hit-test layers still see the click.
                let _ = self.editor_state.close_color_picker();
                self.mark_dirty();
                false
            }
        }
    }
}
