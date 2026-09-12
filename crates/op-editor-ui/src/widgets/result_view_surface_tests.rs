use super::{
    board_enter, panel_enter, ResultBoard, ResultHit, ResultViewSurface, RESULT_BUTTON_HITS,
};
use crate::{Point2D, Rect};
use op_editor_core::{EditorState, HomeFamily};

const W: f32 = 1440.0;
const H: f32 = 900.0;
const THREE_PHONES: [(f32, f32); 3] = [(375.0, 812.0), (375.0, 812.0), (375.0, 812.0)];

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.5,
        "expected ≈{expected}, got {actual}"
    );
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}

#[test]
fn result_layout_fits_three_boards_inside_the_stage_at_1440x900() {
    let layout = ResultViewSurface::layout_for(W, H, &THREE_PHONES);
    // Stage spans x 80 → width−360 and y 170 → height−140.
    assert_close(layout.stage.origin.x, 80.0);
    assert_close(layout.stage.size.x, W - 440.0);
    assert_close(layout.stage.origin.y, 170.0);
    assert_close(layout.stage.size.y, H - 310.0);
    // Panel: 290 wide, same vertical span, to the right of the stage.
    assert_close(layout.panel.size.x, 290.0);
    assert_close(layout.panel.origin.y, layout.stage.origin.y);
    assert_close(layout.panel.size.y, layout.stage.size.y);
    assert!(layout.panel.origin.x >= layout.stage.origin.x + layout.stage.size.x);
    // Boards: equal heights fitted into stage_h − 100, inside the stage,
    // left→right with captions below and no stage/panel overlap.
    let board_h = layout.stage.size.y - 100.0;
    assert_eq!(layout.screens.len(), 3);
    for (index, screen) in layout.screens.iter().enumerate() {
        assert_close(screen.size.y, board_h);
        assert!(screen.origin.y >= layout.stage.origin.y);
        assert!(
            screen.origin.y + screen.size.y <= layout.stage.origin.y + layout.stage.size.y,
            "board {index} overflows the stage vertically"
        );
        assert!(
            screen.origin.x >= layout.stage.origin.x
                && screen.origin.x + screen.size.x <= layout.stage.origin.x + layout.stage.size.x,
            "board {index} overflows the stage horizontally"
        );
        let caption = layout.captions[index];
        assert!(caption.origin.y >= screen.origin.y + screen.size.y);
        assert!(
            caption.origin.y + caption.size.y <= layout.stage.origin.y + layout.stage.size.y,
            "caption {index} overflows the stage"
        );
        if index > 0 {
            let previous = layout.screens[index - 1];
            assert!(
                screen.origin.x >= previous.origin.x + previous.size.x + 29.0,
                "boards {prev} and {index} are closer than the 30px gap",
                prev = index - 1
            );
        }
    }
    assert!(!overlap(layout.stage, layout.panel));
    for button in layout.buttons {
        assert!(!overlap(button, layout.stage));
    }
}

#[test]
fn result_buttons_stack_at_44px_with_10px_gaps() {
    let layout = ResultViewSurface::layout_for(W, H, &THREE_PHONES);
    for (index, button) in layout.buttons.iter().enumerate() {
        assert_close(button.size.y, 44.0);
        assert!(button.origin.x >= layout.panel.origin.x);
        assert!(
            button.origin.x + button.size.x <= layout.panel.origin.x + layout.panel.size.x,
            "button {index} escapes the panel"
        );
        if index > 0 {
            let previous = layout.buttons[index - 1];
            assert_close(
                button.origin.y - (previous.origin.y + previous.size.y),
                10.0,
            );
        }
    }
    // The button stack ends above the footer line.
    let last = layout.buttons[5];
    assert!(last.origin.y + last.size.y <= layout.footer.origin.y + layout.footer.size.y);
}

#[test]
fn wide_board_rows_scale_down_uniformly_instead_of_overflowing() {
    // Six wide dashboard boards cannot fit at full height — the row must
    // scale down uniformly and every board stays equal height.
    let boards = [(1920.0, 1080.0); 6];
    let layout = ResultViewSurface::layout_for(W, H, &boards);
    let first = layout.screens[0];
    assert!(
        first.size.y < layout.stage.size.y - 100.0,
        "a six-board row must shrink"
    );
    for screen in &layout.screens {
        assert_close(screen.size.y, first.size.y);
    }
    for screen in &layout.screens {
        assert!(
            screen.origin.x + screen.size.x <= layout.stage.origin.x + layout.stage.size.x + 0.5,
            "a scaled row must never overflow the stage (float epsilon allowed)"
        );
    }
}

#[test]
fn result_hits_cover_back_button_primary_action_and_boards() {
    let mut state = EditorState::new();
    state
        .editor_ui
        .result_view
        .open(vec!["a".into(), "b".into()], 1_000);
    state.editor_ui.result_view.family = Some(HomeFamily::AppUi);
    let surface = ResultViewSurface::for_editor_at(&state, 2_000).unwrap();
    let layout = surface.layout(W, H);

    let back = Point2D::new(
        layout.breadcrumb_back.origin.x + 10.0,
        layout.breadcrumb_back.origin.y + 10.0,
    );
    assert_eq!(surface.hit_test(W, H, back), Some(ResultHit::BackHome));
    let primary = Point2D::new(
        layout.buttons[0].origin.x + 10.0,
        layout.buttons[0].origin.y + 10.0,
    );
    assert_eq!(
        surface.hit_test(W, H, primary),
        Some(ResultHit::EditThisScreen)
    );
    let second_board = Point2D::new(
        layout.screens[1].origin.x + layout.screens[1].size.x / 2.0,
        layout.screens[1].origin.y + 40.0,
    );
    assert_eq!(
        surface.hit_test(W, H, second_board),
        Some(ResultHit::Screen(1))
    );
    // An empty stage point is no interactive target — the host still
    // swallows it (full-surface takeover) but no hover feedback shows.
    let stage_pad = Point2D::new(layout.stage.origin.x + 6.0, layout.stage.origin.y + 6.0);
    assert_eq!(surface.hit_test(W, H, stage_pad), None);
}

#[test]
fn the_hidden_view_never_constructs_and_collects_no_boards() {
    let state = EditorState::new();
    assert!(ResultViewSurface::for_editor_at(&state, 0).is_none());
    assert!(ResultBoard::collect(&state).is_empty());
}

#[test]
fn entrance_motion_rises_fades_and_settles() {
    let shown = 10_000u64;
    // Board 0 starts immediately: fully risen before its window, settled after.
    assert_eq!(board_enter(0, shown, shown), (12.0, 0.0));
    let (mid_y, mid_a) = board_enter(0, shown, shown + 160);
    assert!(mid_y > 0.0 && mid_y < 12.0);
    assert!(mid_a > 0.0 && mid_a < 1.0);
    assert_eq!(board_enter(0, shown, shown + 320), (0.0, 1.0));
    // Board 2 waits out two stagger steps before moving and settles at
    // 2 × 70 + 320 ms after the shown instant.
    assert_eq!(board_enter(2, shown, shown + 69), (12.0, 0.0));
    assert_eq!(board_enter(2, shown, shown + 70), (12.0, 0.0));
    assert_eq!(board_enter(2, shown, shown + 140 + 320), (0.0, 1.0));
    // The panel slides in over its own shorter window.
    assert_eq!(panel_enter(shown, shown), 16.0);
    assert_eq!(panel_enter(shown, shown + 260), 0.0);
}

#[test]
fn screen_draw_rect_lifts_the_hovered_board() {
    let mut state = EditorState::new();
    state
        .editor_ui
        .result_view
        .open(vec!["a".into(), "b".into()], 10_000);
    state.editor_ui.result_view.family = Some(HomeFamily::AppUi);
    let now = 10_000 + 320 + 140; // entrance fully settled
    let surface = ResultViewSurface::for_editor_at(&state, now).unwrap();
    let layout = surface.layout(W, H);
    assert_eq!(
        surface.screen_draw_rect(&layout, 0).unwrap().origin.y,
        layout.screens[0].origin.y
    );
    let mut hovered = state.editor_ui.result_view.clone();
    hovered.hover = Some(ResultHit::Screen(0));
    let surface = ResultViewSurface {
        state: &hovered,
        ..ResultViewSurface::for_editor_at(&state, now).unwrap()
    };
    let lifted = surface.screen_draw_rect(&layout, 0).unwrap();
    assert_close(lifted.origin.y, layout.screens[0].origin.y - 4.0);
}

#[test]
fn button_hit_order_matches_the_panel_paint_order() {
    assert_eq!(
        RESULT_BUTTON_HITS,
        [
            ResultHit::EditThisScreen,
            ResultHit::PlayPrototype,
            ResultHit::ComponentsVariables,
            ResultHit::Restyle,
            ResultHit::Export,
            ResultHit::FullEdit,
        ]
    );
}
