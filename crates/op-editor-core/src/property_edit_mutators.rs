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
                let idx = match focus {
                    PropertyFocus::PaddingTop => 0,
                    PropertyFocus::PaddingRight => 1,
                    PropertyFocus::PaddingBottom => 2,
                    PropertyFocus::PaddingLeft => 3,
                    _ => unreachable!(),
                };
                values[idx] = value.max(0.0) as f64;
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
            PropertyFocus::PositionR => unreachable!("corner radius handled before node borrow"),
            PropertyFocus::PolygonSides
            | PropertyFocus::EllipseStart
            | PropertyFocus::EllipseSweep
            | PropertyFocus::EllipseInnerRadius
            | PropertyFocus::FontSize
            | PropertyFocus::FontWeight
            | PropertyFocus::LineHeight
            | PropertyFocus::LetterSpacing
            | PropertyFocus::LayoutGap
            | PropertyFocus::PaddingTop
            | PropertyFocus::PaddingRight
            | PropertyFocus::PaddingBottom
            | PropertyFocus::PaddingLeft => {
                unreachable!("shape-specific properties handled before node borrow")
            }
            PropertyFocus::StrokeWidth
            | PropertyFocus::Opacity
            | PropertyFocus::FillHex
            | PropertyFocus::StrokeHex => {}
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
