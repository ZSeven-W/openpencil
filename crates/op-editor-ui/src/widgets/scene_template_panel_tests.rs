use super::*;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::EditorState;

use super::test_rects::{MEDIUM as PANEL, NARROW, WIDE};
use crate::widgets::panel_control_metrics::SEGMENT_TRACK_PAD;

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

/// The filter row is one line with no wrap or scroll, so it has to fit in
/// every locale — and the locale with the longest labels is not the default
/// one the test above runs in. Adding a scene adds a chip, which is exactly
/// when the row runs off the panel edge for languages that spell the scene
/// names out.
#[test]
fn the_filter_row_fits_the_panel_in_every_locale() {
    for locale in op_editor_core::Locale::ALL {
        let mut state = open_state();
        state.editor_ui.locale = locale;
        let panel = SceneTemplatePanel::for_editor(&state).expect("open");
        let chips = panel.filter_chip_rects(PANEL);
        let (last, _) = chips.last().copied().expect("chips");
        let overflow = last.origin.x + last.size.x - (PANEL.origin.x + PANEL.size.x - PAD);
        assert!(
            overflow <= 0.0,
            "{locale:?}: the filter row overflows the panel by {overflow:.1}px"
        );
    }
}

#[test]
fn cards_fill_their_row_and_wrap_inside_the_viewport() {
    let state = open_state();
    let _guard = crate::widgets::asset_center_template_cards::template_test_support::exclusive_user_templates();

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let viewport = panel.cards_viewport(PANEL);
    let (columns, _, _) = panel.grid_metrics(PANEL);
    let rects = panel.card_rects(PANEL);
    assert!(
        rects.len() > columns,
        "the catalogue ships more than one row of templates"
    );

    let (_, first) = rects[0];
    let (_, second) = rects[1];
    assert_eq!(first.origin.y, second.origin.y, "first two share a row");
    assert!(second.origin.x > first.origin.x);
    assert!(
        rects[columns].1.origin.y > first.origin.y,
        "the card past the last column wraps to the next row"
    );

    for (_, rect) in &rects {
        assert!(rect.origin.x >= viewport.origin.x - 0.5);
        assert!(
            rect.origin.x + rect.size.x <= viewport.origin.x + viewport.size.x + 0.5,
            "card {rect:?} runs past the viewport"
        );
    }
}

/// The gallery has no fixed column count: a wider window buys more cards per
/// row, and a narrow one falls back rather than shrinking cards to slivers.
///
/// The ladder runs past four on purpose. A fixed ceiling is what left a 5K
/// display showing four cards in the middle of an otherwise empty gallery,
/// which is the whole defect this exercises.
#[test]
fn the_column_count_follows_the_panel_width() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let columns_at = |rect| panel.grid_metrics(rect).0;
    assert_eq!(columns_at(NARROW), 2);
    assert_eq!(columns_at(PANEL), 3);
    assert_eq!(columns_at(WIDE), 5);

    for (viewport_w, expected) in [(1200.0, 3), (1700.0, 4), (2200.0, 5), (2700.0, 6)] {
        let rect = test_rects::for_viewport(viewport_w);
        assert_eq!(
            columns_at(rect),
            expected,
            "a {viewport_w}px window must lay out {expected} columns"
        );
    }
}

/// Whatever the ladder answers, no card is ever wider than the ceiling that
/// drives it — a "column count" that let cards grow without bound would be
/// decoration rather than a rule.
#[test]
fn no_breakpoint_lets_a_card_grow_past_the_ceiling() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let mut width = 700.0;
    while width <= 5200.0 {
        let (columns, card_w, _) = panel.grid_metrics(test_rects::for_viewport(width));
        assert!(
            card_w <= CARD_MAX_W + 0.01,
            "a {width}px window gave {columns} columns of {card_w}px cards"
        );
        width += 37.0;
    }
}

/// A superwide window must be *filled*, not centred in. This is the reported
/// defect stated as an assertion: at 5120 px the last card in a row has to
/// reach the panel's right margin.
#[test]
fn a_superwide_window_leaves_no_empty_half() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let rect = test_rects::for_viewport(5120.0);
    let viewport = panel.cards_viewport(rect);
    let (columns, card_w, _) = panel.grid_metrics(rect);
    assert!(
        columns > 6,
        "5120px must buy more than a 2700px window: {columns}"
    );

    let row_w = columns as f32 * card_w + (columns - 1) as f32 * CARD_GAP;
    assert!(
        (row_w - viewport.size.x).abs() < 0.5,
        "a row of {columns} cards spans {row_w} of a {} viewport",
        viewport.size.x
    );
    let right_gap = (rect.origin.x + rect.size.x) - (viewport.origin.x + viewport.size.x);
    assert!(
        (right_gap - PAD).abs() < 0.5,
        "the grid must stop at the panel's own margin, not {right_gap}px short"
    );
}

/// The whole point of going full-screen: previews get bigger, not just more
/// numerous. A card must also be exactly as tall as its own preview, its
/// palette band, and the fixed caption, or the derived height and the painted
/// preview disagree.
#[test]
fn card_height_is_derived_from_the_preview_it_holds() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    for rect in [NARROW, PANEL, WIDE] {
        let (_, card_w, card_h) = panel.grid_metrics(rect);
        assert!(
            card_w - CARD_PREVIEW_INSET * 2.0 > 320.0,
            "a {card_w}px card leaves a preview no bigger than the old dialog's"
        );
        let expected = CARD_PREVIEW_INSET + preview_height(card_w) + CARD_PALETTE_H + CARD_TEXT_H;
        assert!(
            (card_h - expected).abs() < 0.01,
            "card height must leave the caption exactly {CARD_TEXT_H}px"
        );
    }
}

/// The trough padding belongs to the control, not to the panel behind it.
///
/// A segmented control has a 3 px inset all round. Testing only the segments
/// would leave that inset as a band that looks pressable, highlights
/// nothing, and swallows the click as "somewhere inside the panel".
#[test]
fn the_segmented_tracks_own_padding_still_selects_a_tab() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let track = panel.tab_track_rect(PANEL);
    let tabs = panel.tab_chip_rects(PANEL);

    let left_gutter = Point2D::new(track.origin.x + 1.0, track.origin.y + track.size.y / 2.0);
    assert_eq!(
        panel.hit_test(PANEL, left_gutter),
        Some(SceneTemplateHit::SelectTab(tabs[0].1)),
        "the leading trough padding belongs to the first segment"
    );
    assert_eq!(
        panel.hover_at(PANEL, left_gutter),
        Some(super::tab_hover_token(0)),
        "and it must light that segment up, not nothing"
    );

    let right_gutter = Point2D::new(
        track.origin.x + track.size.x - 1.0,
        track.origin.y + track.size.y / 2.0,
    );
    assert_eq!(
        panel.hit_test(PANEL, right_gutter),
        Some(SceneTemplateHit::SelectTab(tabs[tabs.len() - 1].1))
    );

    // Outside the track is still outside: the row must not claim the
    // whole width of the panel.
    let beyond = Point2D::new(
        track.origin.x + track.size.x + 24.0,
        track.origin.y + track.size.y / 2.0,
    );
    assert_ne!(
        panel.hit_test(PANEL, beyond),
        Some(SceneTemplateHit::SelectTab(tabs[0].1))
    );
}

/// Every control in the panel comes off one ladder of heights and radii.
///
/// This is the assertion that would have caught the state this replaced: a
/// 28 px chip, a 30 px card button, a 32 px close button and two 38 px
/// fields, each declared beside the code that painted it. Nothing was wrong
/// alone; together the panel read as four kits.
#[test]
fn every_control_comes_off_the_shared_height_ladder() {
    let mut state = open_state();
    state.editor_ui.scene_template_generate_supported = true;
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");

    // Text fields and the button on their row are one height.
    let search = SceneTemplatePanel::search_rect(PANEL);
    let input = panel.generate_input_rect(PANEL).expect("the row paints");
    let button = panel.generate_button_rect(PANEL).expect("the row paints");
    for rect in [search, input, button] {
        assert_eq!(rect.size.y, CONTROL_H, "{rect:?} is off the control ladder");
    }
    // …and the button sits on exactly the field's baseline, not near it.
    assert_eq!(input.origin.y, button.origin.y);
    assert_eq!(
        input.origin.x + input.size.x + GENERATE_GAP,
        button.origin.x,
        "the button must abut its field by the row gap, not by a guess"
    );

    // Chips are one height, whichever row they stand in.
    let mut chips: Vec<f32> = panel
        .filter_chip_rects(PANEL)
        .into_iter()
        .map(|(rect, _)| rect.size.y)
        .collect();
    chips.extend(
        panel
            .tab_chip_rects(PANEL)
            .into_iter()
            .map(|(rect, _)| rect.size.y),
    );
    chips.push(CLOSE_BTN);
    for height in chips {
        assert_eq!(height, CHIP_H, "a chip escaped the chip height");
    }

    // The tab track wraps its segments by the track padding on every side.
    let track = panel.tab_track_rect(PANEL);
    let segments = panel.tab_chip_rects(PANEL);
    assert_eq!(track.size.y, CHIP_H + SEGMENT_TRACK_PAD * 2.0);
    assert_eq!(
        segments[0].0.origin.y,
        track.origin.y + SEGMENT_TRACK_PAD,
        "segments must sit inside the trough"
    );
}

/// The grid takes the whole panel; a single control does not.
///
/// Both start at the same left edge, so the column still reads as one column
/// — the grid simply runs longer than it. Capping the grid too was what put
/// half a 5K gallery permanently out of use; not capping the search field
/// would put a two-metre text input in its place.
#[test]
fn the_grid_fills_the_panel_while_a_single_control_stays_capped() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let wide = test_rects::for_viewport(5120.0);
    let content = SceneTemplatePanel::content_rect(wide);
    let control = SceneTemplatePanel::control_rect(wide);

    assert_eq!(content.size.x, wide.size.x - PAD * 2.0);
    assert_eq!(control.size.x, SCENE_TEMPLATE_CONTENT_MAX_W);
    assert_eq!(
        control.origin.x, content.origin.x,
        "one left edge, two widths"
    );

    // Every row hangs off that shared edge, so none of them can drift out of
    // alignment with the grid.
    // The tab row contributes its *track*, not its first segment: the
    // segment is inset by the track's own padding, which is the seam that
    // makes it a segmented control.
    for origin_x in [
        SceneTemplatePanel::search_rect(wide).origin.x,
        panel.cards_viewport(wide).origin.x,
        panel.tab_track_rect(wide).origin.x,
        panel.filter_chip_rects(wide)[0].0.origin.x,
    ] {
        assert_eq!(origin_x, content.origin.x);
    }
    assert!(
        SceneTemplatePanel::search_rect(wide).size.x < panel.cards_viewport(wide).size.x,
        "the search field must not stretch with the grid"
    );

    // A panel narrower than the cap has nothing to cap: the two agree.
    assert_eq!(
        SceneTemplatePanel::control_rect(PANEL).size.x,
        SceneTemplatePanel::content_rect(PANEL).size.x
    );
}

#[test]
fn a_press_on_a_card_resolves_to_that_template() {
    let state = open_state();
    let _guard = crate::widgets::asset_center_template_cards::template_test_support::exclusive_user_templates();

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let templates = panel.filtered();
    let (index, rect) = panel.card_rects(PANEL).into_iter().next().expect("a card");
    let hit = panel.hit_test(
        PANEL,
        Point2D::new(rect.origin.x + 10.0, rect.origin.y + 10.0),
    );
    assert_eq!(
        hit,
        Some(SceneTemplateHit::AddTemplateToCanvas(
            templates[index].id.clone()
        ))
    );
}

#[test]
fn search_narrows_the_grid_and_an_empty_result_is_representable() {
    let mut state = open_state();
    let _guard = crate::widgets::asset_center_template_cards::template_test_support::exclusive_user_templates();

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
    let _guard = crate::widgets::asset_center_template_cards::template_test_support::exclusive_user_templates();

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

/// The widget answers only for its own rect. What happens to a press on the
/// scrim beside it is the press flow's call, not this layer's.
#[test]
fn presses_outside_the_panel_are_not_claimed() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let outside = Point2D::new(PANEL.origin.x - 1.0, PANEL.origin.y + 10.0);
    assert_eq!(panel.hit_test(PANEL, outside), None);
    assert_eq!(panel.hover_at(PANEL, outside), None);
}
