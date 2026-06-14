use jian_widgets::components::select::{SelectHit, SelectState};

use crate::theme::Theme;
use crate::widgets::property_panel_fill::{
    fill_type_at, fill_type_picker_hit, fill_type_picker_rect,
};
use crate::widgets::property_panel_visibility::VisibleSections;
use crate::{Point2D, Rect};

#[test]
fn fill_type_picker_hit_uses_shared_outside_protocol() {
    let panel_rect = Rect {
        origin: Point2D::new(600.0, 48.0),
        size: Point2D::new(256.0, 700.0),
    };
    let mut state = SelectState::default();
    state.open = true;
    let visible = VisibleSections::ALL;
    let picker = fill_type_picker_rect(panel_rect, visible);

    assert_eq!(
        fill_type_picker_hit(
            &state,
            panel_rect,
            visible,
            Point2D::new(picker.origin.x + 8.0, picker.origin.y + 10.0),
            &Theme::dark(),
        ),
        SelectHit::Row(0)
    );
    assert_eq!(fill_type_at(0), Some(op_editor_core::FillType::Solid));
    assert_eq!(
        fill_type_picker_hit(
            &state,
            panel_rect,
            visible,
            Point2D::new(picker.origin.x - 1.0, picker.origin.y),
            &Theme::dark(),
        ),
        SelectHit::Outside
    );
}
