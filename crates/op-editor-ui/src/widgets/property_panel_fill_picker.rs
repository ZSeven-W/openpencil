use crate::theme::Theme;
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

/// Type-dropdown rect for a fill row. The caller must pass the
/// `ToggleFillTypePicker(index)` action rect emitted by the shared
/// property-panel action walker.
pub fn fill_type_dropdown_rect(action_rect: Rect) -> Rect {
    action_rect
}

/// Picker overlay rect anchored just below the fill's type dropdown.
pub fn fill_type_picker_rect(action_rect: Rect) -> Rect {
    let dropdown = fill_type_dropdown_rect(action_rect);
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
    action_rect: Rect,
    point: Point2D,
    theme: &Theme,
) -> SelectHit {
    let picker = fill_type_picker_rect(action_rect);
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
