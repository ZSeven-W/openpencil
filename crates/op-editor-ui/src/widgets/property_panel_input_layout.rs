//! Editable-input rect walker for the property panel.
//!
//! Split from `property_panel_layout.rs` so the action-rect walker
//! and input-rect walker stay under the repository file-size cap.

use crate::widgets::property_panel::{FillSummary, PropertyPanelAction};
use crate::widgets::property_panel_fill::{fill_head_rects, fill_row_body_height};
use crate::widgets::property_panel_fill_picker::{
    fill_type_at, fill_type_picker_rect, FILL_TYPE_COUNT, FILL_TYPE_ROW_HEIGHT,
};
use crate::widgets::property_panel_inputs::{
    COLOR_VARIABLE_BUTTON_W, COLOR_VARIABLE_GAP, HEADER_HEIGHT, INPUT_HEIGHT, PAD_X, SECTION_GAP,
    SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::property_panel_layout::{push_color_variable_picker_rects, VisibleSections};
use crate::widgets::property_panel_stroke::stroke_side_input_rects;
use crate::{Point2D, Rect};
use op_editor_core::{FillType, PropertyFocus};

/// Emit `(GradientStopHex, rect)` + `(GradientStopOffset, rect)`
/// pairs for `stop_count` rows starting at `*y`, advancing `y` past
/// the last row. Geometry mirrors `paint_fill_gradient_body`'s row
/// layout so click targets land on the boxes the user sees.
fn push_gradient_stop_rects(
    rects: &mut Vec<(PropertyFocus, Rect)>,
    x0: f32,
    y: &mut f32,
    usable_w: f32,
    stop_count: usize,
) {
    let pct_w = 56.0;
    let remove_w = if stop_count > 2 { 26.0 } else { 0.0 };
    let remove_gap = if stop_count > 2 { 6.0 } else { 0.0 };
    let hex_w = usable_w - pct_w - 8.0 - remove_w - remove_gap;
    for index in 0..stop_count {
        rects.push((
            PropertyFocus::GradientStopHex(index),
            Rect {
                origin: Point2D::new(x0 + PAD_X, *y),
                size: Point2D::new(hex_w, INPUT_HEIGHT),
            },
        ));
        rects.push((
            PropertyFocus::GradientStopOffset(index),
            Rect {
                origin: Point2D::new(x0 + PAD_X + hex_w + 8.0, *y),
                size: Point2D::new(pct_w, INPUT_HEIGHT),
            },
        ));
        *y += INPUT_HEIGHT + 4.0;
    }
}

/// On-screen rects of every editable input. Same y walk as
/// `action_button_rects` so paint + hit-test stay aligned.
pub fn editable_input_rects(
    panel_rect: Rect,
    visible: VisibleSections,
    fills: &[FillSummary],
) -> Vec<(PropertyFocus, Rect)> {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;

    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    if visible.create_component {
        // Variant-aware: the instance pair adds a second button row.
        y += crate::widgets::property_panel_inputs::create_component_block_height(
            visible.component_button,
        );
    }
    y += SECTION_HEADER_HEIGHT;
    let x_rect = Rect {
        origin: Point2D::new(x0 + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let y_rect = Rect {
        origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    y += INPUT_HEIGHT + 6.0;
    let rotation_rect = Rect {
        origin: Point2D::new(x0 + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let radius_rect = visible.corner_radius.then_some(Rect {
        origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    });
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;
    let mut rects = vec![
        (PropertyFocus::PositionX, x_rect),
        (PropertyFocus::PositionY, y_rect),
        (PropertyFocus::Rotation, rotation_rect),
    ];
    if let Some(radius_rect) = radius_rect {
        rects.push((PropertyFocus::PositionR, radius_rect));
    }
    if visible.flex_layout {
        crate::widgets::property_panel_flex::push_flex_input_rects(
            &mut rects,
            x0,
            y,
            w,
            visible.flex_layout_mode,
            visible.layout_justify,
            visible.padding_edit_mode,
        );
        y += crate::widgets::property_panel_flex::flex_section_height(
            visible.flex_layout_mode,
            visible.padding_edit_mode,
        );
    }
    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        // Mirror paint_size_section: omit the W/H hit-rect when its
        // dimension is fill/hug, and reflow H into the left slot when W
        // is hidden — but keep the row's vertical advance fixed so later
        // sections don't shift. (TS size-section.tsx: input rendered
        // only when the dimension is a concrete number.)
        let w_left = Point2D::new(x0 + PAD_X, y);
        let h_right = Point2D::new(x0 + PAD_X + half_w + 8.0, y);
        let w_visible = !visible.size_fill_width && !visible.size_hug_width;
        let h_visible = !visible.size_fill_height && !visible.size_hug_height;
        if w_visible {
            rects.push((
                PropertyFocus::SizeW,
                Rect {
                    origin: w_left,
                    size: Point2D::new(half_w, INPUT_HEIGHT),
                },
            ));
        }
        if h_visible {
            rects.push((
                PropertyFocus::SizeH,
                Rect {
                    origin: if w_visible { h_right } else { w_left },
                    size: Point2D::new(half_w, INPUT_HEIGHT),
                },
            ));
        }
        // Collapse the input row when both dimensions are fill/hug (same
        // rule as paint_size_section) so the checkboxes shift up.
        if w_visible || h_visible {
            y += INPUT_HEIGHT + 10.0;
        }
        let check_h = 22.0;
        y += check_h * if visible.clip_content { 3.0 } else { 2.0 };
        y += 12.0;
        y += SECTION_GAP;
    }
    if visible.icon {
        y += crate::widgets::property_panel_icon::icon_section_height();
    }
    if visible.text {
        crate::widgets::property_panel_text::push_text_input_rects(&mut rects, x0, y, usable_w);
        y += crate::widgets::property_panel_text::text_section_height();
        y += SECTION_GAP;
    }
    if let Some(kind) = visible.widget {
        crate::widgets::property_panel_widget::push_widget_input_rects(
            &mut rects, kind, x0, y, usable_w,
        );
        y += crate::widgets::property_panel_widget::widget_section_height(kind);
        y += SECTION_GAP;
    }
    if visible.image {
        y += crate::widgets::property_panel_image_node::image_section_height(visible.image_warning);
        y += SECTION_GAP;
    }
    if visible.opacity {
        y += SECTION_HEADER_HEIGHT;
        rects.push((
            PropertyFocus::Opacity,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(usable_w / 2.0 - 4.0, INPUT_HEIGHT),
            },
        ));
        if visible.polygon_sides {
            rects.push((
                PropertyFocus::PolygonSides,
                Rect {
                    origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
                    size: Point2D::new(half_w, INPUT_HEIGHT),
                },
            ));
        }
        y += INPUT_HEIGHT;
        if visible.ellipse_arc {
            y += 6.0;
            let col_w = (usable_w - 12.0) / 3.0;
            let focuses = [
                PropertyFocus::EllipseStart,
                PropertyFocus::EllipseSweep,
                PropertyFocus::EllipseInnerRadius,
            ];
            for (i, focus) in focuses.into_iter().enumerate() {
                rects.push((
                    focus,
                    Rect {
                        origin: Point2D::new(x0 + PAD_X + i as f32 * (col_w + 6.0), y),
                        size: Point2D::new(col_w, INPUT_HEIGHT),
                    },
                ));
            }
            y += INPUT_HEIGHT;
        }
        y += 12.0;
        y += SECTION_GAP;
    }
    if visible.fill {
        y += SECTION_HEADER_HEIGHT;
        let primary_stop_count = visible.gradient_stop_count;
        for (fi, fill) in fills.iter().enumerate() {
            let is_primary = fi == 0;
            let fill_type = fill.fill_type;
            // Head row: per-fill opacity input (the `%` box).
            rects.push((
                PropertyFocus::FillOpacity(fi),
                fill_head_rects(x0, y, w).opacity,
            ));
            y += INPUT_HEIGHT + 6.0;
            match fill_type {
                FillType::Solid => {
                    let show_var = is_primary
                        && (visible.color_variable_count > 0 || visible.fill_variable_bound);
                    let variable_w = if show_var {
                        COLOR_VARIABLE_BUTTON_W + COLOR_VARIABLE_GAP
                    } else {
                        0.0
                    };
                    // A variable-bound primary fill paints a `$name` chip,
                    // not an editable hex input.
                    if !(is_primary && visible.fill_variable_bound) {
                        rects.push((
                            PropertyFocus::FillHex(fi),
                            Rect {
                                origin: Point2D::new(x0 + PAD_X, y),
                                size: Point2D::new(usable_w - variable_w, INPUT_HEIGHT),
                            },
                        ));
                    }
                }
                // Gradient / image bodies only paint editable inputs for
                // the primary fill (they key off `fills[0]`).
                FillType::LinearGradient if is_primary => {
                    rects.push((
                        PropertyFocus::GradientAngle,
                        Rect {
                            origin: Point2D::new(x0 + PAD_X, y),
                            size: Point2D::new(usable_w, INPUT_HEIGHT),
                        },
                    ));
                    let mut stop_y = y + INPUT_HEIGHT + 6.0 + SECTION_HEADER_HEIGHT;
                    push_gradient_stop_rects(
                        &mut rects,
                        x0,
                        &mut stop_y,
                        usable_w,
                        visible.gradient_stop_count,
                    );
                }
                FillType::RadialGradient if is_primary => {
                    let mut stop_y = y + SECTION_HEADER_HEIGHT;
                    push_gradient_stop_rects(
                        &mut rects,
                        x0,
                        &mut stop_y,
                        usable_w,
                        visible.gradient_stop_count,
                    );
                }
                // Mesh has no editable inputs (per-vertex editing deferred).
                FillType::LinearGradient
                | FillType::RadialGradient
                | FillType::MeshGradient
                | FillType::Shader
                | FillType::Image => {}
            }
            y += fill_row_body_height(fill_type, is_primary, primary_stop_count);
        }
        y += 12.0;
        y += SECTION_GAP;
    }
    if visible.stroke {
        y += SECTION_HEADER_HEIGHT;
        let show_var = visible.color_variable_count > 0 || visible.stroke_variable_bound;
        let variable_w = if show_var {
            COLOR_VARIABLE_BUTTON_W + COLOR_VARIABLE_GAP
        } else {
            0.0
        };
        // Inline width only in Single mode (Axis/Individual use the per-side
        // grid); must match paint_stroke_main_row so the hit rects don't
        // linger where the width is no longer painted.
        let inline = visible.stroke_edit_mode == op_editor_core::PaddingEditMode::Single;
        let stroke_width_w = if inline { 60.0 } else { 0.0 };
        let stroke_width_gap = if inline { 8.0 } else { 0.0 };
        let stroke_hex_w = usable_w - stroke_width_w - stroke_width_gap - variable_w;
        if !visible.stroke_variable_bound {
            rects.push((
                PropertyFocus::StrokeHex,
                Rect {
                    origin: Point2D::new(x0 + PAD_X, y),
                    size: Point2D::new(stroke_hex_w, INPUT_HEIGHT),
                },
            ));
        }
        if inline {
            rects.push((
                PropertyFocus::StrokeWidth,
                Rect {
                    origin: Point2D::new(x0 + PAD_X + stroke_hex_w + variable_w + 8.0, y),
                    size: Point2D::new(stroke_width_w, INPUT_HEIGHT),
                },
            ));
        }
        rects.extend(stroke_side_input_rects(x0, y, w, visible.stroke_edit_mode));
    }
    rects
}

/// Push the Fill section's action rects (header "+", per-fill type
/// dropdown / × / type-picker rows / colour-swatch + gradient-stop /
/// image-popover affordances) starting at `*y`'s row, returning the y
/// just past the section's trailing 12 px gap (the caller adds the
/// `SECTION_GAP`). Split out of `action_button_rects_with_fill_picker`
/// for the 800-line cap; the y-walk mirrors `paint_one_fill`.
pub(crate) fn push_fill_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    panel_rect: Rect,
    visible: VisibleSections,
    fills: &[FillSummary],
    fill_picker_open: bool,
    fill_type_picker_index: usize,
    mut y: f32,
) -> f32 {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    // Header "+" appends a fill (TS-style stacked list).
    out.push((
        PropertyPanelAction::AddFill,
        Rect {
            origin: Point2D::new(x0 + w - PAD_X - 22.0, y),
            size: Point2D::new(28.0, SECTION_HEADER_HEIGHT),
        },
    ));
    y += SECTION_HEADER_HEIGHT;
    let primary_stop_count = visible.gradient_stop_count;
    // One row per fill — mirror `paint_one_fill`'s y-walk.
    for (fi, fill) in fills.iter().enumerate() {
        let is_primary = fi == 0;
        let fill_type = fill.fill_type;
        // Head row: type dropdown + opacity + adjacent reorder / remove controls.
        let head = fill_head_rects(x0, y, w);
        let dropdown_rect = head.dropdown;
        out.push((PropertyPanelAction::ToggleFillTypePicker(fi), dropdown_rect));
        if fi > 0 {
            out.push((
                PropertyPanelAction::MoveFill {
                    from: fi,
                    to: fi - 1,
                },
                head.move_up,
            ));
        }
        if fi + 1 < fills.len() {
            out.push((
                PropertyPanelAction::MoveFill {
                    from: fi,
                    to: fi + 1,
                },
                head.move_down,
            ));
        }
        out.push((PropertyPanelAction::RemoveFill(fi), head.remove));
        // This fill's open type-picker overlay rows.
        if fill_picker_open && fill_type_picker_index == fi {
            let picker_rect = fill_type_picker_rect(dropdown_rect);
            for i in 0..FILL_TYPE_COUNT {
                let Some(t) = fill_type_at(i) else {
                    continue;
                };
                out.push((
                    PropertyPanelAction::SetFillType {
                        index: fi,
                        fill_type: t,
                    },
                    Rect {
                        origin: Point2D::new(
                            picker_rect.origin.x,
                            picker_rect.origin.y + i as f32 * FILL_TYPE_ROW_HEIGHT,
                        ),
                        size: Point2D::new(picker_rect.size.x, FILL_TYPE_ROW_HEIGHT),
                    },
                ));
            }
        }
        y += INPUT_HEIGHT + 6.0; // head row
                                 // Body action rects, keyed off this fill's type.
        match fill_type {
            FillType::Solid => {
                // Solid body: leading swatch opens the colour picker
                // (the hex-input focus hit-test runs after this, so a
                // swatch click opens the picker instead of focusing).
                if is_primary {
                    let show_var = visible.color_variable_count > 0 || visible.fill_variable_bound;
                    let variable_w = if show_var {
                        COLOR_VARIABLE_BUTTON_W + COLOR_VARIABLE_GAP
                    } else {
                        0.0
                    };
                    let hex_w = usable_w - variable_w;
                    if !visible.fill_variable_bound {
                        out.push((
                            PropertyPanelAction::OpenColorPicker(op_editor_core::ColorTarget::Fill),
                            Rect {
                                origin: Point2D::new(x0 + PAD_X, y),
                                size: Point2D::new(28.0, INPUT_HEIGHT),
                            },
                        ));
                    }
                    if show_var {
                        let var_rect = Rect {
                            origin: Point2D::new(x0 + PAD_X + hex_w + COLOR_VARIABLE_GAP, y),
                            size: Point2D::new(COLOR_VARIABLE_BUTTON_W, INPUT_HEIGHT),
                        };
                        out.push((
                            PropertyPanelAction::ToggleColorVariablePicker(
                                op_editor_core::ColorTarget::Fill,
                            ),
                            var_rect,
                        ));
                        if visible.color_variable_picker_open
                            == Some(op_editor_core::ColorTarget::Fill)
                        {
                            push_color_variable_picker_rects(
                                out,
                                op_editor_core::ColorTarget::Fill,
                                var_rect,
                                visible.color_variable_count,
                                visible.fill_variable_bound,
                            );
                        }
                    }
                } else {
                    // Non-primary solid: swatch opens a picker bound to
                    // THIS fill (no colour-variable button — the variable
                    // subsystem keys off the primary fill).
                    out.push((
                        PropertyPanelAction::OpenFillColorPicker(fi),
                        Rect {
                            origin: Point2D::new(x0 + PAD_X, y),
                            size: Point2D::new(28.0, INPUT_HEIGHT),
                        },
                    ));
                }
            }
            // Gradient / image bodies key off the primary fill, so only
            // the primary fill emits their action rects (the non-primary
            // head row's dropdown is enough to retype it).
            FillType::Image if is_primary => {
                out.push((
                    PropertyPanelAction::ToggleImageFillPopover,
                    Rect {
                        origin: Point2D::new(x0 + PAD_X, y),
                        size: Point2D::new(usable_w, INPUT_HEIGHT),
                    },
                ));
            }
            FillType::LinearGradient | FillType::RadialGradient if is_primary => {
                let mut stop_y = y;
                if fill_type == FillType::LinearGradient {
                    stop_y += INPUT_HEIGHT + 6.0; // Angle row above the stops header.
                }
                out.push((
                    PropertyPanelAction::AddGradientStop,
                    Rect {
                        origin: Point2D::new(x0 + w - PAD_X - 22.0, stop_y),
                        size: Point2D::new(28.0, SECTION_HEADER_HEIGHT),
                    },
                ));
                stop_y += SECTION_HEADER_HEIGHT; // 色标 header
                for index in 0..visible.gradient_stop_count {
                    out.push((
                        PropertyPanelAction::OpenColorPicker(
                            op_editor_core::ColorTarget::GradientStop(index),
                        ),
                        Rect {
                            origin: Point2D::new(x0 + PAD_X, stop_y),
                            size: Point2D::new(28.0, INPUT_HEIGHT),
                        },
                    ));
                    if visible.gradient_stop_count > 2 {
                        out.push((
                            PropertyPanelAction::RemoveGradientStop(index),
                            Rect {
                                origin: Point2D::new(x0 + w - PAD_X - 22.0, stop_y),
                                size: Point2D::new(28.0, INPUT_HEIGHT),
                            },
                        ));
                    }
                    stop_y += INPUT_HEIGHT + 4.0;
                }
            }
            // Mesh emits no body action rects (per-vertex editing deferred).
            FillType::LinearGradient
            | FillType::RadialGradient
            | FillType::MeshGradient
            | FillType::Shader
            | FillType::Image => {}
        }
        y += fill_row_body_height(fill_type, is_primary, primary_stop_count);
    }
    y + 12.0 // trailing gap before the divider
}
