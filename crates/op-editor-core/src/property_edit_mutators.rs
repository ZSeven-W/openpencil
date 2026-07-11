//! Property-panel edit mutators split out of `mutators.rs`.

use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::ui_draft::PropertyFocus;
use crate::walkers::find_node_mut;
use jian_ops_schema::node::container::Padding;
use jian_ops_schema::node::PenNode;

fn stroke_side_for_focus(focus: PropertyFocus) -> Option<crate::StrokeSide> {
    match focus {
        PropertyFocus::StrokeTopWidth => Some(crate::StrokeSide::Top),
        PropertyFocus::StrokeRightWidth => Some(crate::StrokeSide::Right),
        PropertyFocus::StrokeBottomWidth => Some(crate::StrokeSide::Bottom),
        PropertyFocus::StrokeLeftWidth => Some(crate::StrokeSide::Left),
        _ => None,
    }
}

fn stroke_values_for_mode_update(
    mode: crate::PaddingEditMode,
    focus: PropertyFocus,
    value: f32,
    mut values: [f32; 4],
) -> [f32; 4] {
    match mode {
        crate::PaddingEditMode::Single => [value; 4],
        crate::PaddingEditMode::Axis => match focus {
            PropertyFocus::StrokeTopWidth | PropertyFocus::StrokeBottomWidth => {
                values[0] = value;
                values[2] = value;
                values
            }
            PropertyFocus::StrokeRightWidth | PropertyFocus::StrokeLeftWidth => {
                values[1] = value;
                values[3] = value;
                values
            }
            _ => values,
        },
        crate::PaddingEditMode::Individual => {
            let Some(side) = stroke_side_for_focus(focus) else {
                return values;
            };
            let index = match side {
                crate::StrokeSide::Top => 0,
                crate::StrokeSide::Right => 1,
                crate::StrokeSide::Bottom => 2,
                crate::StrokeSide::Left => 3,
            };
            values[index] = value;
            values
        }
    }
}

impl EditorState {
    /// Apply a parsed numeric property edit to the anchor node.
    /// True on a real, editable selection.
    pub fn commit_property_edit(&mut self, focus: PropertyFocus, value: f32) -> bool {
        let instance_scope = self.begin_instance_write_for_anchor();
        let wrote = self.commit_property_edit_inner(focus, value);
        if let Some(scope) = instance_scope {
            self.finish_instance_write(scope);
        }
        wrote
    }

    fn commit_property_edit_inner(&mut self, focus: PropertyFocus, value: f32) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        match focus {
            PropertyFocus::PositionR => return self.cmd_set_node_corner_radius(&sel, value),
            PropertyFocus::StrokeWidth => return self.cmd_set_node_stroke_width(&sel, value),
            PropertyFocus::StrokeTopWidth
            | PropertyFocus::StrokeRightWidth
            | PropertyFocus::StrokeBottomWidth
            | PropertyFocus::StrokeLeftWidth => {
                if !value.is_finite() {
                    return false;
                }
                let values = self
                    .selected_node()
                    .and_then(crate::fills::node_stroke_side_widths)
                    .unwrap_or([0.0; 4]);
                let mode = self.editor_ui.stroke_edit_mode.unwrap_or_else(|| {
                    crate::PaddingEditMode::from_values(values[0], values[1], values[2], values[3])
                });
                let values = stroke_values_for_mode_update(mode, focus, value.max(0.0), values);
                for (side, width) in [
                    (crate::StrokeSide::Top, values[0]),
                    (crate::StrokeSide::Right, values[1]),
                    (crate::StrokeSide::Bottom, values[2]),
                    (crate::StrokeSide::Left, values[3]),
                ] {
                    if !self.cmd_set_node_stroke_side_width(&sel, side, width) {
                        return false;
                    }
                }
                return true;
            }
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
            // TS parity (text-section.tsx:134): font-size input is min=1 max=999.
            PropertyFocus::FontSize => {
                return self.cmd_set_node_font_size(&sel, value.clamp(1.0, 999.0))
            }
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
                // TS parity (text-section.tsx:150): line-height input is
                // min=50 max=400 (percent); stored as a ratio.
                return self.cmd_set_node_layout_prop(
                    &sel,
                    "lineHeight",
                    &crate::LayoutPropValue::Number((value.clamp(50.0, 400.0) / 100.0) as f64),
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
            | PropertyFocus::PaddingLeft
            | PropertyFocus::StrokeTopWidth
            | PropertyFocus::StrokeRightWidth
            | PropertyFocus::StrokeBottomWidth
            | PropertyFocus::StrokeLeftWidth => {
                unreachable!("shape-specific properties handled before node borrow")
            }
            // StrokeWidth handled in the first match (before the node
            // borrow). Opacity / FillHex / StrokeHex are applied by the
            // host commit path, not this numeric commit. The widget
            // TEXT focuses (placeholder / value / label) are likewise
            // committed via the host commit path's string arm, not this
            // numeric one.
            PropertyFocus::Opacity
            | PropertyFocus::FillHex(_)
            | PropertyFocus::StrokeHex
            | PropertyFocus::WidgetPlaceholder
            | PropertyFocus::WidgetValue
            | PropertyFocus::WidgetLabel
            | PropertyFocus::WidgetLeadingIcon
            | PropertyFocus::WidgetTrailingIcon
            | PropertyFocus::WidgetBindKey => {}
            PropertyFocus::FillOpacity(index) => {
                // The primary fill (index 0) keeps `set_selected_fill_opacity`
                // (writes the first fill's body opacity); a non-primary row
                // writes its own fill's opacity by index.
                let opacity = (value / 100.0).clamp(0.0, 1.0);
                if index == 0 {
                    let _ = self.set_selected_fill_opacity(opacity);
                } else {
                    let _ = self.set_selected_fill_opacity_at(index, opacity);
                }
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

    /// Write the `bind:value` two-way state binding (`$state.<key>`)
    /// on the anchor widget node. An empty key clears it. True on a
    /// real, editable selection whose variant carries `bindings`.
    pub fn set_selected_widget_bind_value(&mut self, key: &str) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        self.cmd_set_node_widget_bind_value(&sel, key)
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

    /// Normalise the selected node's stroke side widths to the
    /// canonical shape for `mode`: Single copies top to all sides,
    /// Axis mirrors top→bottom and right→left, Individual preserves
    /// all four current values.
    pub fn set_selected_stroke_mode_shape(&mut self, mode: crate::PaddingEditMode) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let v = self
            .selected_node()
            .and_then(crate::fills::node_stroke_side_widths)
            .unwrap_or([0.0; 4]);
        let reshaped = match mode {
            crate::PaddingEditMode::Single => [v[0]; 4],
            crate::PaddingEditMode::Axis => [v[0], v[1], v[0], v[1]],
            crate::PaddingEditMode::Individual => v,
        };
        for (side, width) in [
            (crate::StrokeSide::Top, reshaped[0]),
            (crate::StrokeSide::Right, reshaped[1]),
            (crate::StrokeSide::Bottom, reshaped[2]),
            (crate::StrokeSide::Left, reshaped[3]),
        ] {
            if !self.cmd_set_node_stroke_side_width(&sel, side, width) {
                return false;
            }
        }
        true
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

    // ── Multi-fill (indexed) selected-node mutators ────────────────
    // The Fill section stacks one row per `PenFill`; these address a fill
    // by index so the header "+" appends and each row's "×" / type / colour
    // / opacity edits its own fill.

    /// Run `f` on the selected, editable node, returning `false` when there
    /// is no real editable selection.
    fn with_selected_node<R>(&mut self, f: impl FnOnce(&mut PenNode) -> R) -> Option<R> {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return None;
        }
        find_node_mut(self.active_children_mut(), &sel).map(f)
    }

    pub fn add_selected_fill(&mut self) -> bool {
        self.with_selected_node(crate::fills::add_fill)
            .unwrap_or(false)
    }

    pub fn remove_selected_fill(&mut self, index: usize) -> bool {
        let sel = self.selection.anchor.clone();
        let removed = self
            .with_selected_node(|node| crate::fills::remove_fill(node, index))
            .unwrap_or(false);
        // `fill_refs` binds the node's PRIMARY (index 0) fill to a colour
        // variable. The scene resolver's `fill_for` lets a registered fill ref
        // WIN over `container.fill`, so a dangling ref keeps painting the
        // variable colour after the fill row is removed. Drop it when the
        // bound (primary) fill goes — the "deleted the fill but the colour
        // stays" bug on token-based (old .op) designs.
        if removed && index == 0 {
            self.ui.variables.fill_refs.remove(&sel);
        }
        removed
    }

    pub fn set_selected_fill_type_at(&mut self, index: usize, fill_type: crate::FillType) -> bool {
        self.with_selected_node(|node| crate::fills::set_fill_type_at(node, index, fill_type))
            .unwrap_or(false)
    }

    pub fn set_selected_fill_hex_at(&mut self, index: usize, hex: &str) -> bool {
        self.with_selected_node(|node| crate::fills::set_fill_hex_at(node, index, hex))
            .unwrap_or(false)
    }

    pub fn set_selected_fill_opacity_at(&mut self, index: usize, opacity: f32) -> bool {
        self.with_selected_node(|node| crate::fills::set_fill_opacity_at(node, index, opacity))
            .unwrap_or(false)
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
