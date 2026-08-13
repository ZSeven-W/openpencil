use super::host_canvas_geometry as geometry;
use super::mobile_chrome;
use super::mobile_more_panel;
use crate::widgets::icons::Icon;
use crate::widgets::test_capture_backend::CaptureBackend;
use crate::widgets::PaintCx;
use crate::{Point2D, Theme};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{EditorState, PropertyTab};

fn touch_state(class: EditorSizeClass) -> EditorState {
    let mut state = EditorState::starter();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = class;
    state.editor_ui.sidebar_open = class.is_expanded();
    state
}

#[test]
fn compact_canvas_uses_phone_bars_without_desktop_dead_zone() {
    let state = touch_state(EditorSizeClass::Compact);
    let canvas = geometry::canvas_rect(&state, 390.0, 844.0);
    assert_eq!(canvas.origin.x, 0.0);
    assert_eq!(canvas.origin.y, geometry::MOBILE_APP_BAR_HEIGHT);
    assert_eq!(canvas.size.x, 390.0);
    assert_eq!(
        canvas.origin.y + canvas.size.y,
        844.0 - geometry::MOBILE_DOCK_HEIGHT
    );
}

#[test]
fn medium_uses_bounded_side_surfaces_not_phone_sheets() {
    let state = touch_state(EditorSizeClass::Medium);
    let layers = geometry::layer_panel_rect(&state, 1_112.0);
    let properties = geometry::property_panel_rect(&state, 834.0, 1_112.0);
    let more = mobile_more_panel::more_panel_rect(&state, 834.0, 1_112.0);

    assert_eq!(layers.size.x, geometry::TABLET_LAYER_WIDTH);
    assert_eq!(layers.origin.x, geometry::TABLET_PANEL_INSET);
    assert_eq!(properties.size.x, geometry::TABLET_PROPERTY_WIDTH);
    assert!(properties.origin.x > 0.0);
    assert_eq!(
        properties.origin.x + properties.size.x,
        834.0 - geometry::TABLET_PANEL_INSET
    );
    assert_eq!(more.size.x, 320.0);
    assert!(more.origin.x > 400.0);
    assert!(more.origin.y >= geometry::TABLET_APP_BAR_HEIGHT);
}

#[test]
fn expanded_touch_reserves_real_rails_and_keeps_touch_chrome() {
    let mut state = touch_state(EditorSizeClass::Expanded);
    state.editor_ui.property_tab = PropertyTab::Code;
    let canvas = geometry::canvas_rect(&state, 1_194.0, 834.0);
    let app_bar = geometry::touch_app_bar_rect(&state, 1_194.0);
    let dock = geometry::touch_dock_rect(&state, 1_194.0, 834.0);

    assert_eq!(app_bar.size.y, geometry::TABLET_APP_BAR_HEIGHT);
    assert_eq!(canvas.origin.x, geometry::TABLET_LAYER_WIDTH);
    assert_eq!(
        canvas.size.x,
        1_194.0 - geometry::TABLET_LAYER_WIDTH - geometry::TABLET_PROPERTY_WIDTH
    );
    assert!(dock.size.x < 1_194.0);
    assert!(dock.origin.x > canvas.origin.x);
}

#[test]
fn tiny_visible_height_never_panics_or_inverts_sheet() {
    let state = touch_state(EditorSizeClass::Compact);
    for height in [1.0, 16.0, 120.0, 280.0] {
        let sheet = mobile_chrome::sheet_rect(390.0, height, 0.68);
        let property = geometry::property_panel_rect(&state, 390.0, height);
        assert!(sheet.size.y >= 0.0 && sheet.size.y <= height);
        assert!(property.size.y >= 0.0 && property.size.y <= height);
    }
}

#[test]
fn glyph_is_centered_inside_touch_target() {
    let target = crate::Rect::xywh(100.0, 200.0, 44.0, 44.0);
    let icon = mobile_chrome::centered_icon_rect(target, 20.0);
    assert_eq!(icon, crate::Rect::xywh(112.0, 212.0, 20.0, 20.0));
}

#[test]
fn compound_touch_icon_keeps_one_shared_viewbox_transform() {
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    mobile_chrome::paint_touch_icon(
        &mut cx,
        crate::Rect::xywh(100.0, 200.0, 44.0, 44.0),
        Icon::Settings,
        20.0,
        Theme::dark().foreground,
    );
    assert!(backend.svg_strokes.len() > 1);
    for (_, origin, size, _, stroke) in &backend.svg_strokes {
        assert_eq!(*origin, Point2D::new(112.0, 212.0));
        assert_eq!(*size, 20.0);
        assert_eq!(*stroke, 1.75);
    }
}

#[test]
fn app_bar_edge_targets_are_symmetric_and_centered() {
    let bar = crate::Rect::xywh(0.0, 0.0, 834.0, 56.0);
    let layers = mobile_chrome::MobileAppBar::layers_rect(bar);
    let overflow = mobile_chrome::MobileAppBar::overflow_rect(bar);
    assert_eq!(layers.origin.x + layers.size.x / 2.0, 28.0);
    assert_eq!(overflow.origin.x + overflow.size.x / 2.0, 834.0 - 28.0);
    assert_eq!(layers.origin.y + layers.size.y / 2.0, 28.0);
    assert_eq!(overflow.origin.y + overflow.size.y / 2.0, 28.0);
}

#[test]
fn tablet_bottom_controls_share_one_centerline_without_overlap() {
    let state = touch_state(EditorSizeClass::Medium);
    let dock = geometry::touch_dock_rect(&state, 834.0, 1_112.0);
    let page = mobile_chrome::page_pill_rect_for(&state, 834.0, 1_112.0);
    let actions = mobile_chrome::selection_actions_rect_for(&state, 834.0, 1_112.0);
    let center_y = dock.origin.y + dock.size.y / 2.0;
    assert_eq!(page.origin.y + page.size.y / 2.0, center_y);
    assert_eq!(actions.origin.y + actions.size.y / 2.0, center_y);
    assert!(page.origin.x + page.size.x + 12.0 <= dock.origin.x);
    assert!(dock.origin.x + dock.size.x + 12.0 <= actions.origin.x);
}

#[test]
fn compact_page_and_selection_actions_use_separate_rows() {
    let state = touch_state(EditorSizeClass::Compact);
    let page = mobile_chrome::page_pill_rect_for(&state, 390.0, 844.0);
    let actions = mobile_chrome::selection_actions_rect_for(&state, 390.0, 844.0);
    assert!(actions.origin.y + actions.size.y + 12.0 <= page.origin.y);
    assert_eq!(
        mobile_chrome::selection_action_hit(
            actions,
            Point2D::new(actions.origin.x + 20.0, actions.origin.y + 22.0),
        ),
        Some(mobile_chrome::SelectionActionHit::Properties)
    );
    assert_eq!(
        mobile_chrome::selection_action_hit(
            actions,
            Point2D::new(
                actions.origin.x + actions.size.x - 22.0,
                actions.origin.y + 22.0
            ),
        ),
        Some(mobile_chrome::SelectionActionHit::Delete)
    );
}
