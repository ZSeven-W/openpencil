//! Web property-input commits and focus-value formatting.

use super::super::WidgetHost;
use super::current_stop_alpha;
use op_editor_core::PropertyFocus;
use op_editor_ui::util::{
    color_to_hex, color_to_hex_with_alpha, format_panel_number, parse_hex_color,
};
use op_editor_ui::Color;

impl WidgetHost {
    pub(in crate::widget_host) fn commit_effect_param_focus_if_any(&mut self) {
        let Some(focus) = self.editor_state.editor_ui.effect_param_focus.take() else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = self.editor_state.ui.property_input.text().to_owned();
        self.editor_state.ui.property_input.set_text("");
        self.editor_state.ui.property_input_draft.clear();
        self.editor_state.ui.property_caret_pos = 0;
        if let Ok(value) = draft.trim().parse::<f32>() {
            if value.is_finite() {
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    let instance_scope = self.editor_state.begin_instance_write_for_anchor();
                    self.editor_state.commit_history();
                    let _ =
                        self.editor_state
                            .apply(op_editor_core::EditorCommand::SetEffectParam {
                                node_id: id,
                                index: focus.effect as u32,
                                field: focus.field,
                                value,
                            });
                    if let Some(scope) = instance_scope {
                        self.editor_state.finish_instance_write(scope);
                    }
                }
            }
        }
        self.mark_dirty();
    }

    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.commit_effect_param_focus_if_any();
        let Some(focus) = self.editor_state.ui.property_focus.take() else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = self.editor_state.ui.property_input.text().to_owned();
        self.editor_state.ui.property_input.set_text("");
        self.editor_state.ui.property_input_draft.clear();
        self.editor_state.ui.property_caret_pos = 0;
        let before = self.editor_state.snapshot_for_history();
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
        match focus {
            PropertyFocus::FillHex(index) => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let hex = color_to_hex(color);
                        if index == 0 {
                            let _ = self.editor_state.set_selected_color(true, &hex);
                        } else {
                            let _ = self.editor_state.set_selected_fill_hex_at(index, &hex);
                        }
                    }
                }
            }
            PropertyFocus::StrokeHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self
                            .editor_state
                            .set_selected_color(false, &color_to_hex(color));
                    }
                }
            }
            PropertyFocus::GradientStopHex(index) => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let existing_alpha = self
                            .editor_state
                            .selected_node()
                            .and_then(|n| current_stop_alpha(n, index))
                            .unwrap_or(1.0);
                        let with_alpha = Color {
                            r: color.r,
                            g: color.g,
                            b: color.b,
                            a: existing_alpha,
                        };
                        let _ = self.editor_state.set_selected_gradient_stop_hex(
                            index,
                            &color_to_hex_with_alpha(with_alpha),
                        );
                    }
                }
            }
            PropertyFocus::WidgetPlaceholder => {
                let _ = self.editor_state.set_selected_widget_text(
                    op_editor_core::WidgetTextField::Placeholder,
                    draft.trim(),
                );
            }
            PropertyFocus::WidgetValue => {
                let _ = self
                    .editor_state
                    .set_selected_widget_text(op_editor_core::WidgetTextField::Value, draft.trim());
            }
            PropertyFocus::WidgetLabel => {
                let _ = self
                    .editor_state
                    .set_selected_widget_text(op_editor_core::WidgetTextField::Label, draft.trim());
            }
            PropertyFocus::WidgetLeadingIcon => {
                let _ = self.editor_state.set_selected_widget_text(
                    op_editor_core::WidgetTextField::LeadingIcon,
                    draft.trim(),
                );
            }
            PropertyFocus::WidgetTrailingIcon => {
                let _ = self.editor_state.set_selected_widget_text(
                    op_editor_core::WidgetTextField::TrailingIcon,
                    draft.trim(),
                );
            }
            PropertyFocus::WidgetBindKey => {
                let _ = self
                    .editor_state
                    .set_selected_widget_bind_value(draft.trim());
            }
            _ => {
                if let Ok(value) = draft.trim().parse::<f32>() {
                    let _ = self.editor_state.commit_property_edit(focus, value);
                }
            }
        }
        if let Some(scope) = instance_scope {
            self.editor_state.finish_instance_write(scope);
        }
        if self.editor_state.snapshot_for_history() != before {
            self.editor_state.history_push_past(before);
        }
        self.mark_dirty();
    }
}

pub(in crate::widget_host) fn property_focus_initial(
    focus: PropertyFocus,
    panel: &op_editor_ui::widgets::PropertyPanel,
) -> String {
    match focus {
        PropertyFocus::PositionX => panel.snapshot.x.to_string(),
        PropertyFocus::PositionY => panel.snapshot.y.to_string(),
        PropertyFocus::SizeW => panel.snapshot.width.to_string(),
        PropertyFocus::SizeH => panel.snapshot.height.to_string(),
        PropertyFocus::LayoutGap => format_panel_number(panel.snapshot.layout_gap),
        PropertyFocus::PaddingTop
        | PropertyFocus::PaddingRight
        | PropertyFocus::PaddingBottom
        | PropertyFocus::PaddingLeft => panel
            .snapshot
            .layout_padding
            .value_for(focus)
            .map(format_panel_number)
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::Rotation => (panel.snapshot.rotation_deg.round() as i32).to_string(),
        PropertyFocus::PositionR => (panel.snapshot.corner_radius.round() as i32).to_string(),
        PropertyFocus::Opacity => "100".to_string(),
        PropertyFocus::PolygonSides => panel.snapshot.polygon_sides.unwrap_or(3).to_string(),
        PropertyFocus::EllipseStart => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.start_deg)
                .unwrap_or(0.0),
        ),
        PropertyFocus::EllipseSweep => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.sweep_deg)
                .unwrap_or(360.0),
        ),
        PropertyFocus::EllipseInnerRadius => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.inner_percent)
                .unwrap_or(0.0),
        ),
        PropertyFocus::FontSize => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.font_size))
            .unwrap_or_else(|| "16".to_string()),
        PropertyFocus::FontWeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| t.font_weight.to_string())
            .unwrap_or_else(|| "400".to_string()),
        PropertyFocus::LineHeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.line_height_percent))
            .unwrap_or_else(|| "120".to_string()),
        PropertyFocus::LetterSpacing => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.letter_spacing))
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::FillOpacity(index) => {
            let opacity = panel
                .snapshot
                .fills
                .get(index)
                .map(|f| f.opacity)
                .unwrap_or(panel.snapshot.fill_opacity);
            ((opacity * 100.0).round() as i32).to_string()
        }
        PropertyFocus::FillHex(index) => panel
            .snapshot
            .fills
            .get(index)
            .map(|f| f.color)
            .or(panel.snapshot.fill)
            .map(color_to_hex)
            .unwrap_or_else(|| "#FFFFFF".to_string()),
        PropertyFocus::StrokeHex => color_to_hex(panel.snapshot.stroke_swatch_color()),
        // Seed the SAME width the inline input paints (0 when unset, and
        // un-rounded) so clicking in never changes the displayed value.
        PropertyFocus::StrokeWidth => {
            format_panel_number(panel.snapshot.stroke.map(|s| s.width).unwrap_or(0.0))
        }
        PropertyFocus::StrokeTopWidth
        | PropertyFocus::StrokeRightWidth
        | PropertyFocus::StrokeBottomWidth
        | PropertyFocus::StrokeLeftWidth => panel
            .snapshot
            .stroke_side_width_for(focus)
            .map(format_panel_number)
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::GradientAngle => {
            let a = panel.snapshot.gradient_angle.unwrap_or(0.0);
            if a.fract() == 0.0 {
                format!("{}", a as i32)
            } else {
                format!("{a}")
            }
        }
        PropertyFocus::GradientStopHex(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            .map(|s| op_editor_ui::widgets::property_panel_fill::stop_hex_rgb_only(&s.hex))
            .unwrap_or_else(|| "#000000".to_string()),
        PropertyFocus::GradientStopOffset(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            .map(|s| ((s.offset * 100.0).round() as i32).to_string())
            .unwrap_or_else(|| "0".to_string()),
        // Widget-section fields — seed the input draft from the selected
        // widget's current value so editing starts from it (mirrors the
        // native `property_focus_initial`).
        PropertyFocus::WidgetPlaceholder => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.placeholder.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetValue => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.value.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetLabel => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.label.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetLeadingIcon => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.leading_icon.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetTrailingIcon => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.trailing_icon.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetBindKey => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.bind_key.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetMin => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.min.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetMax => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.max.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetStep => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.step.clone())
            .unwrap_or_default(),
    }
}
