//! Placement of the overlays whose rect is not simply "a box in the
//! middle of the canvas".

use super::*;

const VIEWPORT_W: f32 = 1_600.0;
const VIEWPORT_H: f32 = 900.0;

fn gallery_state(layer_panel_width: f32) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.sidebar_open = true;
    state.editor_ui.layer_panel_width = layer_panel_width;
    state.editor_ui.scene_template_center.open = true;
    state
}

/// The gallery is centred on the WINDOW, not on the canvas region. It
/// used to inset the canvas region, which took the left rail's width off
/// one side only and left the whole panel sitting visibly right of
/// centre whenever the rail was open.
#[test]
fn the_asset_center_is_centred_on_the_whole_viewport() {
    for width in [180.0_f32, 240.0, 480.0] {
        let state = gallery_state(width);
        let rect =
            scene_template_panel_rect(&state, VIEWPORT_W, VIEWPORT_H).expect("the gallery is open");
        let left = rect.origin.x;
        let right = VIEWPORT_W - (rect.origin.x + rect.size.x);
        assert!(
            (left - right).abs() < 0.01,
            "a {width}px rail pushed the gallery off centre: {left} left, {right} right"
        );
    }
}

/// It spans the same width as its own scrim, less the margin — the two
/// have to read as one surface.
#[test]
fn the_asset_center_spans_its_scrim_and_clears_the_top_bar() {
    let state = gallery_state(240.0);
    let rect =
        scene_template_panel_rect(&state, VIEWPORT_W, VIEWPORT_H).expect("the gallery is open");
    let scrim =
        scene_template_scrim_rect(&state, VIEWPORT_W, VIEWPORT_H).expect("the scrim is painted");
    assert_eq!(scrim.origin.x, 0.0);
    assert_eq!(scrim.size.x, VIEWPORT_W);
    assert!(
        rect.origin.x > scrim.origin.x && rect.origin.x < scrim.origin.x + 40.0,
        "the gallery is inset from the scrim's edge, not from the canvas region's"
    );
    // It overlaps the left rail on purpose; only the top bar stays clear.
    assert!(rect.origin.x < state.editor_ui.layer_panel_width);
    assert!(rect.origin.y > TOP_BAR_HEIGHT);
    assert_eq!(
        rect.origin.y - TOP_BAR_HEIGHT,
        VIEWPORT_H - (rect.origin.y + rect.size.y),
        "the top and bottom margins match"
    );
}

/// Touch hosts already pass a safe-area-adjusted viewport and do not paint
/// the desktop top bar. Reusing its height here left a blank, untouchable band
/// above the gallery on both phone and tablet.
#[test]
fn the_touch_asset_center_uses_the_full_safe_viewport() {
    use op_editor_core::size_class::EditorSizeClass;

    for (size_class, viewport_w, viewport_h) in [
        (EditorSizeClass::Compact, 390.0_f32, 844.0_f32),
        (EditorSizeClass::Medium, 834.0_f32, 1_112.0_f32),
    ] {
        let mut state = gallery_state(0.0);
        state.editor_ui.touch = true;
        state.editor_ui.size_class = size_class;
        let rect =
            scene_template_panel_rect(&state, viewport_w, viewport_h).expect("the gallery is open");
        let inset = rect.origin.x;
        assert_eq!(rect.origin.y, inset);
        assert!((viewport_h - (rect.origin.y + rect.size.y) - inset).abs() < 0.01);
        assert!(rect.origin.y < TOP_BAR_HEIGHT);

        let scrim = scene_template_scrim_rect(&state, viewport_w, viewport_h)
            .expect("the scrim is painted");
        assert_eq!(scrim, Rect::xywh(0.0, 0.0, viewport_w, viewport_h));
    }
}

/// A window too small for the nominal margin gets a smaller one rather
/// than an inverted rect.
#[test]
fn a_tiny_viewport_shrinks_the_margin_instead_of_going_negative() {
    let state = gallery_state(240.0);
    let rect = scene_template_panel_rect(&state, 120.0, 90.0).expect("the gallery is open");
    assert!(rect.size.x > 0.0 && rect.size.y > 0.0);
    assert!(rect.origin.x >= 0.0);
    assert!(rect.origin.x + rect.size.x <= 120.0 + 0.01);
}

/// A closed gallery has no rect at all, so no host can paint or hit-test
/// one.
#[test]
fn a_closed_asset_center_has_neither_panel_nor_scrim() {
    let mut state = gallery_state(240.0);
    state.editor_ui.scene_template_center.open = false;
    assert!(scene_template_panel_rect(&state, VIEWPORT_W, VIEWPORT_H).is_none());
    assert!(scene_template_scrim_rect(&state, VIEWPORT_W, VIEWPORT_H).is_none());
}

fn prompt_center_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.prompt_center.open = true;
    state
}

/// The Prompt Center is sized as a fraction of the window, not as a box.
///
/// The numbers matter: a 720x520 dialog on this viewport covered a fifth of
/// the screen and showed under two rows of cards. Pin the ratios so putting a
/// fixed size back fails here rather than in a screenshot months later.
#[test]
fn the_prompt_center_scales_with_the_viewport() {
    let state = prompt_center_state();
    for (viewport_w, viewport_h) in [(1_800.0_f32, 1_100.0_f32), (VIEWPORT_W, VIEWPORT_H)] {
        let rect =
            prompt_center_panel_rect(&state, viewport_w, viewport_h).expect("the panel is open");
        let available_h = viewport_h - TOP_BAR_HEIGHT;
        assert!(
            (rect.size.x - viewport_w * PROMPT_CENTER_VIEWPORT_W_RATIO).abs() < 0.01,
            "width must track the viewport, got {}",
            rect.size.x
        );
        assert!(
            (rect.size.y - available_h * PROMPT_CENTER_VIEWPORT_H_RATIO).abs() < 0.01,
            "height must track the space under the top bar, got {}",
            rect.size.y
        );
        assert!(
            rect.size.x > 720.0 && rect.size.y > 520.0,
            "a window this size must yield more than the retired fixed dialog"
        );
        assert!(rect.origin.x >= 0.0 && rect.origin.y >= TOP_BAR_HEIGHT);
        assert!(rect.origin.x + rect.size.x <= viewport_w + 0.01);
        assert!(rect.origin.y + rect.size.y <= viewport_h + 0.01);
        assert!(
            (rect.origin.y - TOP_BAR_HEIGHT - (viewport_h - (rect.origin.y + rect.size.y))).abs()
                < 0.01,
            "top and bottom margins match"
        );
    }
}

/// A window smaller than the floors gets a full-bleed panel rather than one
/// hanging off the edge — the floor is clamped to what the viewport has.
#[test]
fn a_tiny_viewport_keeps_the_prompt_center_inside_the_window() {
    let state = prompt_center_state();
    for (viewport_w, viewport_h) in [(320.0_f32, 240.0_f32), (PROMPT_CENTER_MIN_W, 100.0)] {
        let rect =
            prompt_center_panel_rect(&state, viewport_w, viewport_h).expect("the panel is open");
        assert!(rect.size.x > 0.0 && rect.size.y > 0.0);
        assert!(rect.origin.x >= 0.0 && rect.origin.y >= 0.0);
        assert!(rect.origin.x + rect.size.x <= viewport_w + 0.01);
        assert!(rect.origin.y + rect.size.y <= viewport_h + 0.01);
    }
}

/// Above the floors, the floor is what a small-but-not-tiny window gets:
/// the panel gives up its margin before it gives up the second column.
#[test]
fn the_prompt_center_floor_wins_on_a_small_window() {
    let state = prompt_center_state();
    let viewport_w = PROMPT_CENTER_MIN_W / PROMPT_CENTER_VIEWPORT_W_RATIO - 40.0;
    let rect = prompt_center_panel_rect(&state, viewport_w, 900.0).expect("the panel is open");
    assert_eq!(rect.size.x, PROMPT_CENTER_MIN_W);
    assert!(rect.size.x <= viewport_w);
}

/// An open left rail must not push the panel off centre horizontally, and a
/// closed one must not move it either — it is centred in the canvas region,
/// then clamped into the window.
#[test]
fn the_prompt_center_stays_inside_the_window_with_a_wide_rail() {
    for width in [180.0_f32, 480.0] {
        let mut state = prompt_center_state();
        state.editor_ui.sidebar_open = true;
        state.editor_ui.layer_panel_width = width;
        let rect =
            prompt_center_panel_rect(&state, VIEWPORT_W, VIEWPORT_H).expect("the panel is open");
        assert!(rect.origin.x >= 0.0);
        assert!(rect.origin.x + rect.size.x <= VIEWPORT_W + 0.01);
    }
}

/// A closed Prompt Center has no rect at all.
#[test]
fn a_closed_prompt_center_has_no_rect() {
    let mut state = prompt_center_state();
    state.editor_ui.prompt_center.open = false;
    assert!(prompt_center_panel_rect(&state, VIEWPORT_W, VIEWPORT_H).is_none());
}
