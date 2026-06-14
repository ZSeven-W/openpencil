//! Property-panel edit mutators split out of `mutators.rs`.

use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::ui_draft::PropertyFocus;
use crate::walkers::find_node_mut;
use jian_ops_schema::node::container::Padding;
use jian_ops_schema::node::PenNode;

impl EditorState {
    /// Apply a parsed numeric property edit to the anchor node.
    /// True on a real, editable selection.
    pub fn commit_property_edit(&mut self, focus: PropertyFocus, value: f32) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        match focus {
            PropertyFocus::PositionR => return self.cmd_set_node_corner_radius(&sel, value),
            PropertyFocus::StrokeWidth => return self.cmd_set_node_stroke_width(&sel, value),
            PropertyFocus::PolygonSides => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_polygon_count(&sel, value.round().clamp(3.0, 100.0) as u32);
            }
            PropertyFocus::EllipseStart => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_ellipse_arc(
                    &sel,
                    Some(value.clamp(0.0, 360.0) as f64),
                    None,
                    None,
                );
            }
            PropertyFocus::EllipseSweep => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_ellipse_arc(
                    &sel,
                    None,
                    Some(value.clamp(0.0, 360.0) as f64),
                    None,
                );
            }
            PropertyFocus::EllipseInnerRadius => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_ellipse_arc(
                    &sel,
                    None,
                    None,
                    Some((value.clamp(0.0, 99.0) / 100.0) as f64),
                );
            }
            PropertyFocus::WidgetMin => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_node_widget_number(
                    &sel,
                    crate::WidgetNumberField::Min,
                    value as f64,
                );
            }
            PropertyFocus::WidgetMax => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_node_widget_number(
                    &sel,
                    crate::WidgetNumberField::Max,
                    value as f64,
                );
            }
            PropertyFocus::WidgetStep => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_node_widget_number(
                    &sel,
                    crate::WidgetNumberField::Step,
                    value as f64,
                );
            }
            PropertyFocus::FontSize => return self.cmd_set_node_font_size(&sel, value.max(1.0)),
            PropertyFocus::FontWeight => {
                if !value.is_finite() {
                    return false;
                }
                return self
                    .cmd_set_node_font_weight(&sel, value.round().clamp(1.0, 1000.0) as u16);
            }
            PropertyFocus::LineHeight => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_node_layout_prop(
                    &sel,
                    "lineHeight",
                    &crate::LayoutPropValue::Number((value / 100.0) as f64),
                );
            }
            PropertyFocus::LetterSpacing => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_node_layout_prop(
                    &sel,
                    "letterSpacing",
                    &crate::LayoutPropValue::Number(value as f64),
                );
            }
            PropertyFocus::LayoutGap => {
                if !value.is_finite() {
                    return false;
                }
                return self.cmd_set_node_layout_prop(
                    &sel,
                    "gap",
                    &crate::LayoutPropValue::Number(value.max(0.0) as f64),
                );
            }
            PropertyFocus::PaddingTop
            | PropertyFocus::PaddingRight
            | PropertyFocus::PaddingBottom
            | PropertyFocus::PaddingLeft => {
                if !value.is_finite() {
                    return false;
                }
                let mut values = self
                    .selected_node()
                    .map(container_padding_values)
                    .unwrap_or([0.0; 4]);
                let v = value.max(0.0) as f64;
                // Fan the edit out to the sides the active mode binds
                // together (TS: Single writes all four; Axis-V =
                // top+bottom, Axis-H = right+left).
                let mode = self.editor_ui.padding_edit_mode.unwrap_or_else(|| {
                    crate::PaddingEditMode::from_values(
                        values[0] as f32,
                        values[1] as f32,
                        values[2] as f32,
                        values[3] as f32,
                    )
                });
                match (mode, focus) {
                    (crate::PaddingEditMode::Single, _) => values = [v; 4],
                    (crate::PaddingEditMode::Axis, PropertyFocus::PaddingTop) => {
                        values[0] = v;
                        values[2] = v;
                    }
                    (crate::PaddingEditMode::Axis, PropertyFocus::PaddingRight) => {
                        values[1] = v;
                        values[3] = v;
                    }
                    _ => {
                        let idx = match focus {
                            PropertyFocus::PaddingTop => 0,
                            PropertyFocus::PaddingRight => 1,
                            PropertyFocus::PaddingBottom => 2,
                            PropertyFocus::PaddingLeft => 3,
                            _ => unreachable!(),
                        };
                        values[idx] = v;
                    }
                }
                return self.cmd_set_node_layout_prop(
                    &sel,
                    "padding",
                    &crate::LayoutPropValue::NumberArray(values.to_vec()),
                );
            }
            _ => {}
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        let v = value as f64;
        match focus {
            PropertyFocus::PositionX => node.base_mut().x = Some(v),
            PropertyFocus::PositionY => node.base_mut().y = Some(v),
            PropertyFocus::SizeW => node.set_width_px(v.max(0.0)),
            PropertyFocus::SizeH => node.set_height_px(v.max(0.0)),
            PropertyFocus::Rotation => node.base_mut().rotation = Some(v),
            PropertyFocus::PositionR | PropertyFocus::StrokeWidth => {
                unreachable!("corner radius / stroke width handled before node borrow")
            }
            PropertyFocus::PolygonSides
            | PropertyFocus::EllipseStart
            | PropertyFocus::EllipseSweep
            | PropertyFocus::EllipseInnerRadius
            | PropertyFocus::FontSize
            | PropertyFocus::FontWeight
            | PropertyFocus::LineHeight
            | PropertyFocus::LetterSpacing
            | PropertyFocus::LayoutGap
            | PropertyFocus::WidgetMin
            | PropertyFocus::WidgetMax
            | PropertyFocus::WidgetStep
            | PropertyFocus::PaddingTop
            | PropertyFocus::PaddingRight
            | PropertyFocus::PaddingBottom
            | PropertyFocus::PaddingLeft => {
                unreachable!("shape-specific properties handled before node borrow")
            }
            // StrokeWidth handled in the first match (before the node
            // borrow). Opacity / FillHex / StrokeHex are applied by the
            // host commit path, not this numeric commit. The widget
            // TEXT focuses (placeholder / value / label) are likewise
            // committed via the host commit path's string arm, not this
            // numeric one.
            PropertyFocus::Opacity
            | PropertyFocus::FillHex
            | PropertyFocus::StrokeHex
            | PropertyFocus::WidgetPlaceholder
            | PropertyFocus::WidgetValue
            | PropertyFocus::WidgetLabel => {}
            PropertyFocus::FillOpacity => {
                let _ = self.set_selected_fill_opacity((value / 100.0).clamp(0.0, 1.0));
            }
            PropertyFocus::GradientAngle => {
                let _ = crate::fills::set_primary_gradient_angle(node, value);
            }
            PropertyFocus::GradientStopOffset(index) => {
                let _ = crate::fills::set_primary_gradient_stop_offset(node, index, value / 100.0);
            }
            PropertyFocus::GradientStopHex(_) => {}
        }
        true
    }

    /// Write a string-typed widget prop (placeholder / value / label)
    /// on the anchor node. True on a real, editable selection whose
    /// variant actually carries the field.
    pub fn set_selected_widget_text(&mut self, field: crate::WidgetTextField, text: &str) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        self.cmd_set_node_widget_text(&sel, field, text)
    }

    /// Set the `checked` flag on the selected Switch / Checkbox node.
    pub fn set_selected_widget_checked(&mut self, checked: bool) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        self.cmd_set_node_widget_checked(&sel, checked)
    }

    pub fn set_selected_gradient_stop_hex(&mut self, index: usize, hex: &str) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        crate::fills::set_primary_gradient_stop_hex(node, index, hex)
    }

    pub fn add_selected_gradient_stop(&mut self) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        crate::fills::add_primary_gradient_stop(node)
    }

    pub fn remove_selected_gradient_stop(&mut self, index: usize) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        crate::fills::remove_primary_gradient_stop(node, index)
    }

    pub fn set_selected_fill_type(&mut self, fill_type: crate::FillType) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        crate::fills::set_primary_fill_type(node, fill_type)
    }

    /// Normalise the selected node's padding to the canonical shape for
    /// `mode` (TS `handleModeChange`): Single collapses to the top
    /// value on all sides; Axis mirrors top→bottom and right→left;
    /// Individual leaves the four values as-is.
    pub fn set_selected_padding_mode_shape(&mut self, mode: crate::PaddingEditMode) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let v = self
            .selected_node()
            .map(container_padding_values)
            .unwrap_or([0.0; 4]);
        let reshaped = match mode {
            crate::PaddingEditMode::Single => [v[0]; 4],
            crate::PaddingEditMode::Axis => [v[0], v[1], v[0], v[1]],
            crate::PaddingEditMode::Individual => v,
        };
        self.cmd_set_node_layout_prop(
            &sel,
            "padding",
            &crate::LayoutPropValue::NumberArray(reshaped.to_vec()),
        )
    }

    pub fn clear_selected_fills(&mut self) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        crate::fills::clear_primary_fills(node)
    }

    pub fn set_selected_image_fill_mode(&mut self, mode: crate::ImageFillMode) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        if crate::image_node_props::set_image_node_mode(node, mode) {
            true
        } else {
            crate::fills::set_primary_image_fill_mode(node, mode)
        }
    }

    pub fn set_selected_image_adjustment(
        &mut self,
        field: crate::ImageAdjustmentField,
        value: f32,
    ) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        if crate::image_node_props::set_image_node_adjustment(node, field, value) {
            true
        } else {
            crate::fills::set_primary_image_adjustment(node, field, value)
        }
    }

    pub fn reset_selected_image_adjustments(&mut self) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        if crate::image_node_props::reset_image_node_adjustments(node) {
            true
        } else {
            crate::fills::reset_primary_image_adjustments(node)
        }
    }
}

fn container_padding_values(node: &PenNode) -> [f64; 4] {
    let padding = match node {
        PenNode::Frame(n) => n.container.padding.as_ref(),
        PenNode::Group(n) => n.container.padding.as_ref(),
        PenNode::Rectangle(n) => n.container.padding.as_ref(),
        _ => None,
    };
    match padding {
        Some(Padding::Uniform(v)) => [*v, *v, *v, *v],
        Some(Padding::XY(v)) => [v[0], v[1], v[0], v[1]],
        Some(Padding::LtrB(v)) => *v,
        Some(Padding::Expression(_)) | None => [0.0; 4],
    }
}
