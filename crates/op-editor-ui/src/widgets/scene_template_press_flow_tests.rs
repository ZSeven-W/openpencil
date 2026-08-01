use super::*;
use crate::widgets::SCENE_TEMPLATE_CLOSE_HOVER;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::{EditorState, SceneFilter};

const PANEL: Rect = Rect {
    origin: Point2D { x: 100.0, y: 80.0 },
    size: Point2D { x: 720.0, y: 520.0 },
};

fn open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.open_scene_template_center(0);
    state
}

fn point_in(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn a_press_outside_the_panel_falls_through() {
    let mut state = open_state();
    let outside = Point2D::new(PANEL.origin.x - 10.0, PANEL.origin.y - 10.0);
    assert_eq!(
        press_scene_template_center(&mut state, PANEL, outside, 0),
        None
    );
    assert!(
        state.editor_ui.scene_template_center.open,
        "falling through must not close the panel; the host decides that"
    );
}

#[test]
fn a_press_on_empty_panel_space_is_swallowed() {
    let mut state = open_state();
    // Between the header text and the close button: inside, but not a control.
    let inside = Point2D::new(PANEL.origin.x + 300.0, PANEL.origin.y + 20.0);
    assert_eq!(
        press_scene_template_center(&mut state, PANEL, inside, 0),
        Some(false),
        "swallowed without a state change"
    );
}

#[test]
fn the_close_button_closes_the_panel() {
    let mut state = open_state();
    let close = point_in(SceneTemplatePanel::close_rect(PANEL));
    assert_eq!(
        press_scene_template_center(&mut state, PANEL, close, 0),
        Some(true)
    );
    assert!(!state.editor_ui.scene_template_center.open);
}

#[test]
fn selecting_a_scene_filters_and_resets_scroll_and_hover() {
    let mut state = open_state();
    state.editor_ui.scene_template_center.scroll.offset = 120.0;
    state.editor_ui.scene_template_center.hover = Some(3);

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let (rect, filter) = panel
        .filter_chip_rects(PANEL)
        .into_iter()
        .find(|(_, filter)| *filter == SceneFilter::Scene(TemplateScene::Slides))
        .expect("slides chip");

    assert_eq!(
        press_scene_template_center(&mut state, PANEL, point_in(rect), 0),
        Some(true)
    );
    let center = &state.editor_ui.scene_template_center;
    assert_eq!(center.filter, filter);
    assert_eq!(
        center.scroll.offset, 0.0,
        "stale offset points at other cards"
    );
    assert_eq!(center.hover, None);

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let visible = panel.filtered();
    assert!(!visible.is_empty(), "the slides scene ships a template");
    assert!(visible
        .iter()
        .all(|template| template.scene == TemplateScene::Slides));
}

#[test]
fn choosing_a_card_requests_an_open_and_closes_the_panel() {
    let mut state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let first = panel.filtered().first().copied().expect("catalogue");
    let expected = first.id.clone();
    let (_, rect) = panel
        .card_rects_for_count(PANEL, panel.filtered().len())
        .into_iter()
        .next()
        .expect("a card rect");

    assert_eq!(
        press_scene_template_center(&mut state, PANEL, point_in(rect), 0),
        Some(true)
    );
    let center = &mut state.editor_ui.scene_template_center;
    assert!(!center.open, "the panel closes once a choice is made");
    assert_eq!(
        center.take_pending_open().as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        center.take_pending_open(),
        None,
        "the request drains exactly once"
    );
}

#[test]
fn scrolling_is_swallowed_and_clamped_even_when_the_grid_does_not_move() {
    let mut state = open_state();
    // Upward past the top: clamps at zero but must not reach the canvas.
    assert_eq!(
        scroll_scene_template_center(&mut state, PANEL, point_in(PANEL), 50.0),
        Some(false)
    );
    assert_eq!(state.editor_ui.scene_template_center.scroll.offset, 0.0);

    let outside = Point2D::new(PANEL.origin.x - 5.0, PANEL.origin.y);
    assert_eq!(
        scroll_scene_template_center(&mut state, PANEL, outside, 50.0),
        None,
        "outside the panel the canvas keeps the wheel"
    );
}

#[test]
fn hover_reports_being_over_the_panel_so_hosts_can_gate_canvas_hover() {
    let mut state = open_state();
    let close = point_in(SceneTemplatePanel::close_rect(PANEL));
    let (over, changed) = hover_scene_template_center(&mut state, PANEL, close);
    assert!(over, "the host needs this to suppress hover underneath");
    assert!(changed);
    assert_eq!(
        state.editor_ui.scene_template_center.hover,
        Some(SCENE_TEMPLATE_CLOSE_HOVER)
    );

    let outside = Point2D::new(PANEL.origin.x - 20.0, PANEL.origin.y);
    let (over, changed) = hover_scene_template_center(&mut state, PANEL, outside);
    assert!(!over);
    assert!(changed, "leaving the panel clears the hover");
    assert_eq!(state.editor_ui.scene_template_center.hover, None);
}

#[test]
fn opening_one_centre_closes_the_other() {
    // Both are full-size centred panels; two open at once would stack.
    let mut state = EditorState::default();
    state.editor_ui.open_prompt_center(0);
    state.editor_ui.open_scene_template_center(0);
    assert!(state.editor_ui.scene_template_center.open);
    assert!(!state.editor_ui.prompt_center.open);

    state.editor_ui.open_prompt_center(0);
    assert!(state.editor_ui.prompt_center.open);
    assert!(!state.editor_ui.scene_template_center.open);
}
