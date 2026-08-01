use super::*;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::EditorState;

const PANEL: Rect = Rect {
    origin: Point2D { x: 40.0, y: 60.0 },
    size: Point2D {
        x: SCENE_TEMPLATE_PANEL_W,
        y: SCENE_TEMPLATE_PANEL_H,
    },
};

fn open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.open_scene_template_center(0);
    state
}

#[test]
fn the_panel_only_builds_while_open() {
    let mut state = EditorState::default();
    assert!(SceneTemplatePanel::for_editor(&state).is_none());
    state.editor_ui.open_scene_template_center(0);
    assert!(SceneTemplatePanel::for_editor(&state).is_some());
}

#[test]
fn every_scene_chip_is_reachable_and_none_overlap() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let chips = panel.filter_chip_rects(PANEL);
    assert_eq!(chips.len(), TemplateScene::ALL.len() + 1, "scenes plus All");

    // Chips are laid out by accumulating widths; an off-by-one there would
    // silently make the last chip unclickable or overlap its neighbour.
    for pair in chips.windows(2) {
        let (left, _) = pair[0];
        let (right, _) = pair[1];
        assert!(
            left.origin.x + left.size.x <= right.origin.x,
            "chips overlap: {left:?} then {right:?}"
        );
    }
    let (last, _) = chips.last().copied().expect("chips");
    assert!(
        last.origin.x + last.size.x <= PANEL.origin.x + PANEL.size.x,
        "the last chip runs past the panel edge"
    );
    for (rect, filter) in chips {
        assert_eq!(
            panel.hit_test(
                PANEL,
                Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0)
            ),
            Some(SceneTemplateHit::SelectFilter(filter))
        );
    }
}

#[test]
fn cards_are_laid_out_two_per_row_inside_the_viewport() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let viewport = panel.cards_viewport(PANEL);
    let rects = panel.card_rects(PANEL);
    assert!(
        rects.len() >= 4,
        "the catalogue ships at least four templates"
    );

    let (_, first) = rects[0];
    let (_, second) = rects[1];
    assert_eq!(first.origin.y, second.origin.y, "first two share a row");
    assert!(second.origin.x > first.origin.x);
    let (_, third) = rects[2];
    assert!(
        third.origin.y > first.origin.y,
        "third wraps to the next row"
    );

    for (_, rect) in &rects {
        assert!(rect.origin.x >= viewport.origin.x - 0.5);
        assert!(
            rect.origin.x + rect.size.x <= viewport.origin.x + viewport.size.x + 0.5,
            "card {rect:?} runs past the viewport"
        );
    }
}

#[test]
fn a_press_on_a_card_resolves_to_that_template() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let templates = panel.filtered();
    let (index, rect) = panel.card_rects(PANEL).into_iter().next().expect("a card");
    let hit = panel.hit_test(
        PANEL,
        Point2D::new(rect.origin.x + 10.0, rect.origin.y + 10.0),
    );
    assert_eq!(
        hit,
        Some(SceneTemplateHit::SelectTemplate(
            templates[index].id.clone()
        ))
    );
}

#[test]
fn search_narrows_the_grid_and_an_empty_result_is_representable() {
    let mut state = open_state();
    state.editor_ui.scene_template_center.search.set_text("PPT");
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let hits = panel.filtered();
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|t| t.scene == TemplateScene::Slides));

    state
        .editor_ui
        .scene_template_center
        .search
        .set_text("no-such-template-anywhere");
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    assert!(panel.filtered().is_empty());
    assert_eq!(panel.max_scroll(PANEL), 0.0, "an empty grid cannot scroll");
}

#[test]
fn a_pointer_below_the_viewport_does_not_hover_a_scrolled_card() {
    // Card rects are computed for every entry, including ones scrolled out of
    // view — paint clips them. Without the viewport gate, a pointer under the
    // panel would light up a row nobody can see.
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let viewport = panel.cards_viewport(PANEL);
    let below = Point2D::new(
        viewport.origin.x + 10.0,
        viewport.origin.y + viewport.size.y + 4.0,
    );
    assert_eq!(panel.hover_at(PANEL, below), None);
}

#[test]
fn the_close_button_sits_inside_the_header_and_hit_tests_first() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let close = SceneTemplatePanel::close_rect(PANEL);
    assert!(close.origin.y >= PANEL.origin.y);
    assert!(close.origin.x + close.size.x <= PANEL.origin.x + PANEL.size.x);
    let centre = Point2D::new(
        close.origin.x + close.size.x / 2.0,
        close.origin.y + close.size.y / 2.0,
    );
    assert_eq!(panel.hit_test(PANEL, centre), Some(SceneTemplateHit::Close));
    assert_eq!(
        panel.hover_at(PANEL, centre),
        Some(SCENE_TEMPLATE_CLOSE_HOVER)
    );
}

#[test]
fn presses_outside_the_panel_are_not_claimed() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let outside = Point2D::new(PANEL.origin.x - 1.0, PANEL.origin.y + 10.0);
    assert_eq!(panel.hit_test(PANEL, outside), None);
    assert_eq!(panel.hover_at(PANEL, outside), None);
}
