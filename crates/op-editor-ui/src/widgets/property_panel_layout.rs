//! Layout walkers for the property panel — pure-math helpers that
//! emit the on-screen rect of every editable input and every
//! button/checkbox the panel paints, so hit-tests stay aligned
//! with paint at all section-visibility / fill-type combinations.
//!
//! Pulled out of `property_panel_sections.rs` to keep that file
//! under the 800-line ceiling.

use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::{
    HEADER_HEIGHT, INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::{Point2D, Rect};
use op_editor_core::{FillType, FlexLayout, PropertyFocus};

/// Whether each section currently paints — drives the layout
/// walk so when per-kind filtering hides a section, the rects
/// that follow shift up.
#[derive(Debug, Clone, Copy)]
pub struct VisibleSections {
    pub flex_layout: bool,
    pub size_options: bool,
    /// `Opacity` from the Layer section.
    pub opacity: bool,
    /// `FillHex` from the Fill section.
    pub fill: bool,
    /// `StrokeHex` + `StrokeWidth` from the Stroke section.
    pub stroke: bool,
    /// Effects section paints (header + add chip, no inputs yet).
    /// Tracked because the export-rect walker needs to know
    /// whether it consumed vertical space ahead of the Export
    /// section.
    pub effects: bool,
    /// Export section paints — `OpenExportDialog` action emits
    /// only when this is true.
    pub export: bool,
    /// Active fill type — affects fill-section body height so
    /// the walk past Fill stays aligned with paint when the user
    /// flips Solid / Gradient / Image.
    pub fill_type: FillType,
}

impl VisibleSections {
    /// Every section visible — matches the legacy unfiltered layout.
    pub const ALL: Self = Self {
        flex_layout: true,
        size_options: true,
        opacity: true,
        fill: true,
        stroke: true,
        effects: true,
        export: true,
        fill_type: FillType::Solid,
    };
}

/// State for the 5 "尺寸" checkboxes (fill / hug / clip).
#[derive(Debug, Clone, Copy)]
pub struct SizeFlags {
    pub fill_width: bool,
    pub fill_height: bool,
    pub hug_width: bool,
    pub hug_height: bool,
    pub clip_content: bool,
}

/// Height of the body that follows the head row in the Fill
/// section, per fill type. Used by every layout walker so the
/// y-offset of sections after Fill stays aligned with paint when
/// the user flips fill types.
pub fn fill_body_height(fill_type: FillType) -> f32 {
    match fill_type {
        FillType::Solid => INPUT_HEIGHT + 6.0,
        FillType::LinearGradient => {
            INPUT_HEIGHT + 6.0 + SECTION_HEADER_HEIGHT + INPUT_HEIGHT + 4.0 + INPUT_HEIGHT + 6.0
        }
        FillType::RadialGradient => SECTION_HEADER_HEIGHT + INPUT_HEIGHT + 4.0 + INPUT_HEIGHT + 6.0,
        FillType::Image => INPUT_HEIGHT + 6.0,
    }
}

/// Rects of every clickable button / checkbox in the panel —
/// flex-layout 3 buttons, size-options 5 checkboxes. Same y-walk
/// math as `editable_input_rects` so paint + hit-test stay in
/// sync regardless of which sections are filtered.
pub fn action_button_rects(
    panel_rect: Rect,
    visible: VisibleSections,
) -> Vec<(PropertyPanelAction, Rect)> {
    action_button_rects_with_fill_picker(panel_rect, visible, false)
}

/// Same as `action_button_rects` but `fill_picker_open == true`
/// emits hit-rects for the 4 picker rows that overlay the Fill
/// section.
pub fn action_button_rects_with_fill_picker(
    panel_rect: Rect,
    visible: VisibleSections,
    fill_picker_open: bool,
) -> Vec<(PropertyPanelAction, Rect)> {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;

    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    y += 8.0 + 36.0 + 12.0;
    // Position section.
    y += SECTION_HEADER_HEIGHT;
    y += INPUT_HEIGHT + 6.0;
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;

    let mut out: Vec<(PropertyPanelAction, Rect)> = Vec::new();

    if visible.flex_layout {
        y += SECTION_HEADER_HEIGHT;
        let btn_w = 56.0;
        let gap = 8.0;
        let row_x = x0 + PAD_X;
        let modes = [
            FlexLayout::Free,
            FlexLayout::Vertical,
            FlexLayout::Horizontal,
        ];
        for (i, mode) in modes.iter().enumerate() {
            let bx = row_x + i as f32 * (btn_w + gap);
            out.push((
                PropertyPanelAction::SetFlexLayout(*mode),
                Rect {
                    origin: Point2D::new(bx, y),
                    size: Point2D::new(btn_w, 32.0),
                },
            ));
        }
        y += 32.0 + 12.0;
        y += SECTION_GAP;
    }

    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 10.0;
        let row_h = 22.0;
        out.push((
            PropertyPanelAction::ToggleSizeFillWidth,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(half_w, row_h),
            },
        ));
        out.push((
            PropertyPanelAction::ToggleSizeFillHeight,
            Rect {
                origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
                size: Point2D::new(half_w, row_h),
            },
        ));
        y += row_h;
        out.push((
            PropertyPanelAction::ToggleSizeHugWidth,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(half_w, row_h),
            },
        ));
        out.push((
            PropertyPanelAction::ToggleSizeHugHeight,
            Rect {
                origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
                size: Point2D::new(half_w, row_h),
            },
        ));
        y += row_h;
        out.push((
            PropertyPanelAction::ToggleSizeClipContent,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(usable_w, row_h),
            },
        ));
        y += row_h + 12.0;
        y += SECTION_GAP;
    }

    if visible.opacity {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 12.0;
        y += SECTION_GAP;
    }
    if visible.fill {
        y += SECTION_HEADER_HEIGHT;
        // Swatch hit-test mirrors the paint rect in property_panel_fill.rs.
        let swatch_rect = Rect {
            origin: Point2D::new(x0 + PAD_X, y + 2.0),
            size: Point2D::new(22.0, 22.0),
        };
        out.push((
            PropertyPanelAction::OpenColorPicker(op_editor_core::ColorTarget::Fill),
            swatch_rect,
        ));
        let dropdown_rect = Rect {
            origin: Point2D::new(x0 + PAD_X + 22.0 + 6.0, y),
            size: Point2D::new(usable_w - 22.0 - 6.0 - 50.0 - 22.0 - 12.0, INPUT_HEIGHT),
        };
        out.push((PropertyPanelAction::ToggleFillTypePicker, dropdown_rect));
        if fill_picker_open {
            let row_h = 32.0;
            let panel_w = dropdown_rect.size.x;
            let panel_x = dropdown_rect.origin.x;
            let panel_y = dropdown_rect.origin.y + dropdown_rect.size.y + 4.0;
            let types = [
                FillType::Solid,
                FillType::LinearGradient,
                FillType::RadialGradient,
                FillType::Image,
            ];
            for (i, t) in types.iter().enumerate() {
                out.push((
                    PropertyPanelAction::SetFillType(*t),
                    Rect {
                        origin: Point2D::new(panel_x, panel_y + 6.0 + i as f32 * row_h),
                        size: Point2D::new(panel_w, row_h),
                    },
                ));
            }
        }
        // Consume the rest of the Fill section so subsequent
        // sections' y math stays aligned with paint. Mirrors the
        // y-walk in `paint_fill_section`: head row + body + divider.
        y += INPUT_HEIGHT + 6.0; // head row (swatch + dropdown + opacity + X)
        y += fill_body_height(visible.fill_type) - 6.0 + 12.0; // body + divider gap
        y += SECTION_GAP;
    }
    if visible.stroke {
        // Mirrors paint_stroke_section: header + hex/width row.
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 12.0;
        y += SECTION_GAP;
    }
    if visible.effects {
        // Mirrors paint_effects_section: header + 8 px filler.
        // The header's "+" button (drawn by
        // `paint_section_label_with_add` at the right edge) maps to
        // an `AddEffect` action.
        let plus = Rect {
            origin: Point2D::new(x0 + w - PAD_X - 22.0, y),
            size: Point2D::new(28.0, SECTION_HEADER_HEIGHT),
        };
        out.push((PropertyPanelAction::AddEffect, plus));
        y += SECTION_HEADER_HEIGHT;
        y += 8.0;
        y += SECTION_GAP;
    }
    if visible.export {
        // Mirrors paint_export_section: header + a single dropdown
        // row. We emit one `OpenExportDialog` rect covering both
        // the scale + format pills + the full label strip so
        // clicking anywhere in the section opens the dialog. The
        // ExportDialog modal owns the format/scale picker UI, so
        // segmenting the row into two separate pills isn't worth
        // the extra hit-test complexity yet.
        y += SECTION_HEADER_HEIGHT;
        let row = Rect {
            origin: Point2D::new(x0 + PAD_X, y),
            size: Point2D::new(usable_w, INPUT_HEIGHT),
        };
        out.push((PropertyPanelAction::OpenExportDialog, row));
        // No further sections after Export today, but maintain the
        // y advance for symmetry / future additions.
        y += INPUT_HEIGHT + 12.0;
    }
    let _ = y; // suppress unused-write lint if export is last

    out
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
        let half = usable_w / 2.0 - 4.0;
        rects.push((
            PropertyFocus::Opacity,
            Rect {
                origin: Point2D::new(x0 + PAD_X, y),
                size: Point2D::new(half, INPUT_HEIGHT),
            },
        ));
        y += INPUT_HEIGHT + 12.0;
        y += SECTION_GAP;
    }
    if visible.fill {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 6.0;
        if matches!(visible.fill_type, FillType::Solid) {
            rects.push((
                PropertyFocus::FillHex,
                Rect {
                    origin: Point2D::new(x0 + PAD_X, y),
                    size: Point2D::new(usable_w, INPUT_HEIGHT),
                },
            ));
        }
        y += fill_body_height(visible.fill_type) - 6.0;
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
