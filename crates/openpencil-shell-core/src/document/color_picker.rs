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
            variable: None,
            variable_pre_color: None,
        });
        true
    }

    /// Open the picker rooted at a Color variable instead of the
    /// selected node. Seeds HSV from the variable's currently-resolved
    /// colour under the active theme. Returns false when no variable
    /// of that name exists or it isn't Color-kind. The picker's
    /// commit path (live HSV → RGB) routes through
    /// `VariableTable::set_color_hex` instead of `set_selected_color`.
    pub fn open_color_picker_for_variable(
        &mut self,
        variable: impl Into<String>,
        anchor_y: f32,
    ) -> bool {
        let name = variable.into();
        let Some(current) = self.var_table.resolve_color(&name) else {
            return false;
        };
        let (h, s, v) = picker::rgb_to_hsv(current);
        let pre_snap = Some(self.snapshot_for_history());
        self.ui.color_picker = Some(ColorPickerState {
            // `target` is unused on the variable path but must
            // carry a sane default for any code that pattern-matches
            // on it without checking `variable` first.
            target: ColorTarget::Fill,
            hue: h,
            sat: s,
            val: v,
            pre_snap,
            drag: None,
            anchor_y,
            variable: Some(name),
            variable_pre_color: Some(current),
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
        let variable = state.variable.clone();
        let rgb = picker::hsv_to_rgb(state.hue, state.sat, state.val);
        if let Some(name) = variable {
            // Variable-mode commit — write through the var_table
            // so paint reflects the change everywhere the variable
            // resolves. Set_color_hex re-uses the same write-path
            // tests (subset / default / no-shadow) the variable
            // edits land through.
            let hex = format_color_hex_for_var(rgb);
            self.var_table.set_color_hex(&name, &hex);
            return true;
        }
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
            // Variable-mode close: did the resolved colour
            // actually change? `variable_pre_color` was captured
            // when the picker opened; compare against the
            // post-HSV-drag resolution. snapshot.var_table doesn't
            // exist in the DocumentSnapshot shape, so we can't go
            // through `snap`.
            let changed = if let Some(name) = &state.variable {
                let before = state.variable_pre_color;
                let after = self.var_table.resolve_color(name);
                before != after
            } else {
                self.selected_node()
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
                    .unwrap_or(false)
            };
            if changed {
                self.history_push_past(snap);
            }
        }
        true
    }
}

/// Format an RGBA color as `#rrggbb`, suitable for storing back
/// into a Color-kind variable. Used by `color_picker_set_hsv`'s
/// variable-mode commit path.
fn format_color_hex_for_var(c: crate::Color) -> String {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    format!("#{:02x}{:02x}{:02x}", ch(c.r), ch(c.g), ch(c.b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Variable, VariableKind, VariableScalar, VariableValue};

    fn doc_with_color_var(name: &str, hex: &str) -> Document {
        let mut doc = Document::empty();
        doc.var_table.variables.push(Variable {
            name: name.into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str(hex.into())),
        });
        doc
    }

    #[test]
    fn open_color_picker_for_variable_seeds_hsv_from_resolved_color() {
        let mut doc = doc_with_color_var("brand", "#ff8800");
        assert!(doc.open_color_picker_for_variable("brand", 100.0));
        let state = doc.ui.color_picker.as_ref().expect("picker open");
        assert!(state.variable.as_deref() == Some("brand"));
        assert!(state.variable_pre_color.is_some());
        // HSV anchor: hue near 32° for #ff8800 (orange).
        assert!(state.hue > 20.0 && state.hue < 45.0, "hue {}", state.hue);
        assert!(state.sat > 0.95, "sat {}", state.sat);
        assert!(state.val > 0.95, "val {}", state.val);
    }

    #[test]
    fn open_color_picker_for_variable_fails_when_var_missing_or_wrong_kind() {
        let mut doc = Document::empty();
        // Unknown name → false.
        assert!(!doc.open_color_picker_for_variable("nope", 0.0));
        // Number-kind variable → not a colour → false.
        doc.var_table.variables.push(Variable {
            name: "spacing".into(),
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(16.0)),
        });
        assert!(!doc.open_color_picker_for_variable("spacing", 0.0));
        assert!(doc.ui.color_picker.is_none(), "picker must not open");
    }

    #[test]
    fn color_picker_set_hsv_writes_through_variable_path() {
        // End-to-end: open picker on a variable, push a new HSV
        // (pure-red anchor: H=0, S=1, V=1), confirm the variable's
        // resolved colour flips and the node's fill stays untouched.
        let mut doc = doc_with_color_var("brand", "#ff8800");
        assert!(doc.open_color_picker_for_variable("brand", 0.0));
        assert!(doc.color_picker_set_hsv(0.0, 1.0, 1.0));
        // Variable now resolves to red.
        let resolved = doc.var_table.resolve_color("brand").unwrap();
        assert!((resolved.r - 1.0).abs() < 0.01);
        assert!(resolved.g < 0.01);
        assert!(resolved.b < 0.01);
    }

    #[test]
    fn close_color_picker_after_variable_edit_pushes_history_only_when_changed() {
        let mut doc = doc_with_color_var("brand", "#ff8800");
        let history_depth_before = doc.history.past.len();
        assert!(doc.open_color_picker_for_variable("brand", 0.0));
        // No HSV change → close should NOT push history.
        assert!(doc.close_color_picker());
        assert_eq!(doc.history.past.len(), history_depth_before);

        // Re-open + drag + close → history grows by one.
        assert!(doc.open_color_picker_for_variable("brand", 0.0));
        assert!(doc.color_picker_set_hsv(180.0, 1.0, 1.0));
        assert!(doc.close_color_picker());
        assert_eq!(doc.history.past.len(), history_depth_before + 1);
    }
}
