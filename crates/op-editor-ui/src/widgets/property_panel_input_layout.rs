//! Editable-input rect walker for the property panel.
//!
//! Split from `property_panel_layout.rs` so the action-rect walker
//! and input-rect walker stay under the repository file-size cap.

use crate::widgets::property_panel_inputs::{
    HEADER_HEIGHT, INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::property_panel_layout::{fill_body_height_with_stops, VisibleSections};
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
    let hex_w = usable_w - pct_w - 8.0;
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
) -> Vec<(PropertyFocus, Rect)> {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;

    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    y += 8.0 + 36.0 + 12.0;
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
    let radius_rect = Rect {
        origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;
    if visible.flex_layout {
        y += SECTION_HEADER_HEIGHT;
        y += 32.0 + 12.0;
        y += SECTION_GAP;
    }
    let mut rects = vec![
        (PropertyFocus::PositionX, x_rect),
        (PropertyFocus::PositionY, y_rect),
        (PropertyFocus::Rotation, rotation_rect),
        (PropertyFocus::PositionR, radius_rect),
    ];
    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        rects.push((
            PropertyFocus::SizeW,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(half_w, INPUT_HEIGHT),
            },
        ));
        rects.push((
            PropertyFocus::SizeH,
            Rect {
                origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
                size: Point2D::new(half_w, INPUT_HEIGHT),
            },
        ));
        y += INPUT_HEIGHT + 10.0;
        let check_h = 22.0;
        y += check_h * 3.0;
        y += 12.0;
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
        y += INPUT_HEIGHT + 12.0;
        y += SECTION_GAP;
    }
    if visible.fill {
        y += SECTION_HEADER_HEIGHT;
        rects.push((
            PropertyFocus::FillOpacity,
            Rect {
                origin: Point2D::new(x0 + w - PAD_X - 78.0, y),
                size: Point2D::new(50.0, INPUT_HEIGHT),
            },
        ));
        y += INPUT_HEIGHT + 6.0;
        match visible.fill_type {
            FillType::Solid => {
                rects.push((
                    PropertyFocus::FillHex,
                    Rect {
                        origin: Point2D::new(x0 + PAD_X, y),
                        size: Point2D::new(usable_w, INPUT_HEIGHT),
                    },
                ));
            }
            FillType::LinearGradient => {
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
            FillType::RadialGradient => {
                let mut stop_y = y + SECTION_HEADER_HEIGHT;
                push_gradient_stop_rects(
                    &mut rects,
                    x0,
                    &mut stop_y,
                    usable_w,
                    visible.gradient_stop_count,
                );
            }
            FillType::Image => {}
        }
        y += fill_body_height_with_stops(visible.fill_type, visible.gradient_stop_count) - 6.0;
        y += 12.0;
        y += SECTION_GAP;
    }
    if visible.stroke {
        y += SECTION_HEADER_HEIGHT;
        let stroke_width_w = 60.0;
        let stroke_hex_w = usable_w - stroke_width_w - 8.0;
        rects.push((
            PropertyFocus::StrokeHex,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(stroke_hex_w, INPUT_HEIGHT),
            },
        ));
        rects.push((
            PropertyFocus::StrokeWidth,
            Rect {
                origin: Point2D::new(x0 + PAD_X + stroke_hex_w + 8.0, y),
                size: Point2D::new(stroke_width_w, INPUT_HEIGHT),
            },
        ));
    }
    rects
}
