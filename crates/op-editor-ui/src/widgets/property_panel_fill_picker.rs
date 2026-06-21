use crate::theme::Theme;
use crate::widgets::property_panel_inputs::{
    CREATE_COMPONENT_BLOCK_H, HEADER_HEIGHT, INPUT_HEIGHT, PAD_X, SECTION_GAP,
    SECTION_HEADER_HEIGHT, TAB_HEIGHT,
};
use crate::widgets::property_panel_layout::VisibleSections;
use crate::{Point2D, Rect};
use jian_widgets::components::select::{Select, SelectHit, SelectState};
use jian_widgets::Tokens;
use op_editor_core::FillType;

pub const FILL_TYPE_ROW_HEIGHT: f32 = 30.0;
pub const FILL_TYPE_COUNT: usize = 4;
pub const FILL_TYPES: [FillType; FILL_TYPE_COUNT] = [
    FillType::Solid,
    FillType::LinearGradient,
    FillType::RadialGradient,
    FillType::Image,
];

pub fn fill_type_at(idx: usize) -> Option<FillType> {
    FILL_TYPES.get(idx).copied()
}

/// Type-dropdown rect for the fill at vertical offset `row_offset_y`
/// (the sum of the head-row + body heights of every fill above it).
/// Pass `0.0` for the primary (first) fill.
pub fn fill_type_dropdown_rect(
    panel_rect: Rect,
    visible: VisibleSections,
    row_offset_y: f32,
) -> Rect {
    let usable_w = panel_rect.size.x - PAD_X * 2.0;
    Rect {
        origin: Point2D::new(
            panel_rect.origin.x + PAD_X + 22.0 + 6.0,
            fill_type_dropdown_y(panel_rect, visible) + row_offset_y,
        ),
        size: Point2D::new(usable_w - 22.0 - 6.0 - 50.0 - 22.0 - 12.0, INPUT_HEIGHT),
    }
}

/// Picker overlay rect for the fill at vertical offset `row_offset_y`
/// — anchored just below that fill's type dropdown.
pub fn fill_type_picker_rect(
    panel_rect: Rect,
    visible: VisibleSections,
    row_offset_y: f32,
) -> Rect {
    let dropdown = fill_type_dropdown_rect(panel_rect, visible, row_offset_y);
    Rect {
        origin: Point2D::new(dropdown.origin.x, dropdown.origin.y + dropdown.size.y + 4.0),
        size: Point2D::new(
            dropdown.size.x,
            FILL_TYPE_ROW_HEIGHT * FILL_TYPE_COUNT as f32,
        ),
    }
}

pub fn fill_type_picker_hit(
    state: &SelectState,
    panel_rect: Rect,
    visible: VisibleSections,
    row_offset_y: f32,
    point: Point2D,
    theme: &Theme,
) -> SelectHit {
    let picker = fill_type_picker_rect(panel_rect, visible, row_offset_y);
    Select::hit(
        state,
        popup_anchor(picker),
        picker,
        FILL_TYPE_COUNT,
        point,
        &tokens_from_theme(theme),
    )
}

pub(crate) fn popup_anchor(popup: Rect) -> Rect {
    Rect {
        origin: popup.origin,
        size: Point2D::new(popup.size.x, 0.0),
    }
}

pub(crate) fn tokens_from_theme(theme: &Theme) -> Tokens {
    crate::widgets::button::tokens_from_theme(theme)
}

fn fill_type_dropdown_y(panel_rect: Rect, visible: VisibleSections) -> f32 {
    let mut y = panel_rect.origin.y;
    y += TAB_HEIGHT;
    y += HEADER_HEIGHT;
    if visible.create_component {
        y += CREATE_COMPONENT_BLOCK_H;
    }
    y += SECTION_HEADER_HEIGHT;
    y += INPUT_HEIGHT + 6.0;
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;
    if visible.flex_layout {
        y += crate::widgets::property_panel_flex::flex_section_height(
            visible.flex_layout_mode,
            visible.padding_edit_mode,
        );
    }
    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 10.0;
        y += 22.0 * if visible.clip_content { 3.0 } else { 2.0 };
        y += 12.0 + SECTION_GAP;
    }
    if visible.icon {
        y += crate::widgets::property_panel_icon::icon_section_height();
    }
    if visible.text {
        y += crate::widgets::property_panel_text::text_section_height();
        y += SECTION_GAP;
    }
    if visible.image {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 34.0;
        y += SECTION_GAP;
    }
    if visible.opacity {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 12.0 + SECTION_GAP;
    }
    y + SECTION_HEADER_HEIGHT
}
