//! Native-host routing for the left rail's slides tab.
//!
//! Windows-gated like the other tests that solve layout: building the
//! scene runs `jian_skia::SkiaMeasure`, which aborts the process under
//! Windows CI's DirectWrite.

#![cfg(all(test, not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
use op_editor_core::{EditorState, LeftPanelTab, SlidesPanelTarget};
use op_editor_ui::Point2D;
use std::sync::{LazyLock, Mutex, MutexGuard};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

const VW: f32 = 1_400.0;
const VH: f32 = 900.0;

/// Three 16:9 boards side by side — the shape a generated deck has.
const THREE_BOARD_DECK: &str = r##"{
    "version": "1.0.0",
    "children": [
        { "type": "frame", "id": "slide-1", "name": "Cover", "x": 0, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#ffffff"}], "children": [] },
        { "type": "frame", "id": "slide-2", "name": "议程", "x": 2100, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#eeeeee"}], "children": [] },
        { "type": "frame", "id": "slide-3", "name": "Closing", "x": 4200, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#dddddd"}], "children": [] }
    ]
}"##;

fn host_with(scenario: Option<TemplateScene>) -> WidgetHostNative {
    let document = jian_ops_schema::load_str(THREE_BOARD_DECK)
        .expect("parse deck fixture")
        .value;
    let mut host = WidgetHostNative::new();
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = scenario;
    host.install_imported_state(state);
    // After the install, so it survives whatever the install resets.
    host.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host
}

fn row_centre(host: &mut WidgetHostNative, index: usize) -> Point2D {
    let slides = host
        .slides_panel_frame(VW, VH)
        .expect("a deck shows the slides tab");
    let rect = slides.layout.row_rect(index);
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// Each board's authored position, keyed by id so the comparison is
/// about GEOMETRY and not about the order — the order is exactly what a
/// reorder is allowed to change.
fn board_geometry(host: &WidgetHostNative) -> std::collections::BTreeMap<String, (f64, f64)> {
    host.editor_state
        .active_children()
        .iter()
        .map(|node| {
            let base = op_editor_core::PenNodeExt::base(node);
            (
                base.id.clone(),
                (base.x.unwrap_or(0.0), base.y.unwrap_or(0.0)),
            )
        })
        .collect()
}

#[test]
fn any_document_with_boards_gets_the_slides_tab() {
    let _guard = test_lock();
    let mut deck = host_with(Some(TemplateScene::Slides));
    let slides = deck
        .slides_panel_frame(VW, VH)
        .expect("a deck shows the slides tab");
    assert_eq!(slides.chips.len(), 3);
    assert_eq!(slides.chips[1].name, "议程");
    assert!(deck.slides_tab_row(VW, VH).is_some());

    // The same three boards with no scenario recorded: an ordinary
    // design page still gets the navigator, which is the whole point of
    // the tab being permanent.
    let mut ordinary = host_with(None);
    let listed = ordinary
        .slides_panel_frame(VW, VH)
        .expect("an untagged page with boards still lists them");
    assert_eq!(listed.chips.len(), 3);
    assert!(ordinary.slides_tab_row(VW, VH).is_some());
}

#[test]
fn a_page_with_no_boards_has_nothing_to_list_and_shows_no_tab() {
    let _guard = test_lock();
    let mut empty = WidgetHostNative::new();
    empty.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    empty.last_viewport_w = VW;
    empty.last_viewport_h = VH;
    empty.editor_state.active_children_mut().clear();
    assert!(op_editor_core::preview_slideshow::active_page_boards(&empty.editor_state).is_empty());
    assert!(empty.slides_tab_row(VW, VH).is_none());
    assert!(empty.slides_panel_frame(VW, VH).is_none());
    assert_eq!(
        empty.layers_content_rect(VW, VH).origin.y,
        op_editor_ui::widgets::TOP_BAR_HEIGHT,
        "and the layer tree keeps the whole rail"
    );
}

/// Rows keep one height whatever shape the boards are, and each board
/// is letterboxed into that box rather than stretched to fill it.
#[test]
fn a_mixed_page_lists_every_board_at_the_same_row_height() {
    let _guard = test_lock();
    const MIXED: &str = r##"{
        "version": "1.0.0",
        "children": [
            { "type": "frame", "id": "phone", "name": "Phone", "x": 0, "y": 0,
              "width": 390, "height": 844, "children": [] },
            { "type": "frame", "id": "board", "name": "Deck", "x": 600, "y": 0,
              "width": 1920, "height": 1080, "children": [] },
            { "type": "frame", "id": "square", "name": "Card", "x": 2700, "y": 0,
              "width": 1080, "height": 1080, "children": [] }
        ]
    }"##;
    let document = jian_ops_schema::load_str(MIXED)
        .expect("parse mixed fixture")
        .value;
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(document));
    host.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;

    let slides = host.slides_panel_frame(VW, VH).expect("three boards list");
    assert_eq!(slides.chips.len(), 3);
    let heights: Vec<f32> = (0..3).map(|i| slides.layout.row_rect(i).size.y).collect();
    assert!(
        heights.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01),
        "a phone screen and a dashboard must not give different row heights: {heights:?}"
    );
    // The phone and the square card are pillarboxed; the 16:9 board the
    // box is shaped for fills it. Each keeps its own shape inside the
    // one shared box.
    let phone = slides.layout.thumb_rect(0);
    let board = slides.layout.thumb_rect(1);
    let card = slides.layout.thumb_rect(2);
    assert!(
        phone.size.y > phone.size.x,
        "the phone screen stays portrait"
    );
    assert!(
        board.size.x > board.size.y,
        "the deck board stays landscape"
    );
    assert!(phone.size.x < slides.layout.thumb_box.x - 1.0);
    assert!(card.size.x < slides.layout.thumb_box.x - 1.0);
    assert!(
        (board.size.y - slides.layout.thumb_box.y).abs() < 0.01,
        "a deck board fills its plate: {:?} in {:?}",
        board.size,
        slides.layout.thumb_box
    );
}

#[test]
fn a_collapsed_sidebar_has_no_slides_tab_at_all() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    host.editor_state.editor_ui.sidebar_open = false;
    assert!(host.slides_panel_frame(VW, VH).is_none());
    assert!(host.slides_tab_row(VW, VH).is_none());
}

#[test]
fn presenting_hides_the_rail_and_it_eats_no_clicks() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = row_centre(&mut host, 1);
    assert!(host.enter_preview((VW, VH)), "the deck presents");
    assert!(host.preview_slideshow_active());
    assert!(
        host.slides_panel_frame(VW, VH).is_none(),
        "the presentation owns the screen"
    );
    assert!(host.slides_tab_row(VW, VH).is_none());
    // A press where a row used to be reaches the presentation.
    let before = host
        .editor_state
        .preview_slideshow()
        .map(|show| show.index());
    host.apply_press(point.x, point.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert_eq!(before, Some(0));
    assert_eq!(
        host.editor_state
            .preview_slideshow()
            .map(|show| show.index()),
        Some(1),
        "the press belonged to the presentation underneath"
    );
}

#[test]
fn clicking_a_row_frames_that_board() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = row_centre(&mut host, 2);
    let before = host.editor_state.viewport;

    host.apply_press(point.x, point.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);

    assert_ne!(host.editor_state.viewport, before, "the camera moved");
    // The third board's centre is doc (4200 + 960, 540); framing it puts
    // that point at the centre of the canvas region.
    let (cx0, cy0, cw, ch) = host.canvas_region(VW, VH);
    let viewport = host.editor_state.viewport;
    let screen_x = 5_160.0 * viewport.zoom + viewport.pan_x;
    let screen_y = 540.0 * viewport.zoom + viewport.pan_y;
    assert!(
        (screen_x - cw / 2.0).abs() < 2.0 && (screen_y - ch / 2.0).abs() < 2.0,
        "board 3 is centred in the canvas region ({cx0},{cy0},{cw},{ch}) — got ({screen_x},{screen_y})"
    );
    assert!(
        host.editor_state.history.past.is_empty(),
        "navigating a deck is camera-only and never lands on the undo stack"
    );
}

#[test]
fn dragging_a_row_reorders_the_deck_without_moving_any_board() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let geometry_before = board_geometry(&host);
    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-1", "slide-2", "slide-3"]
    );

    let from = row_centre(&mut host, 0);
    let to = row_centre(&mut host, 2);
    host.apply_press(from.x, from.y, VW, VH);
    host.apply_cursor_move(to.x, to.y);
    host.apply_release_with_viewport(VW, VH);

    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-2", "slide-1", "slide-3"],
        "the first slide moved past the second"
    );
    assert_eq!(
        board_geometry(&host),
        geometry_before,
        "a reorder rewrites child ORDER only — no board moved on the canvas"
    );

    assert!(host.apply_undo(), "the reorder is undoable");
    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-1", "slide-2", "slide-3"]
    );
    assert_eq!(board_geometry(&host), geometry_before);
}

#[test]
fn the_footer_button_starts_the_presentation() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let button = {
        let slides = host.slides_panel_frame(VW, VH).expect("slides tab");
        Point2D::new(
            slides.layout.actions.present.origin.x + slides.layout.actions.present.size.x / 2.0,
            slides.layout.actions.present.origin.y + slides.layout.actions.present.size.y / 2.0,
        )
    };
    host.apply_press(button.x, button.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert!(
        host.preview_slideshow_active(),
        "the footer starts the same slideshow the top bar's Play does"
    );
}

#[test]
fn the_tab_row_switches_the_rail_both_ways() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let tabs = host.slides_tab_row(VW, VH).expect("tab row");
    let layers_tab = Point2D::new(
        tabs.layers.origin.x + tabs.layers.size.x / 2.0,
        tabs.layers.origin.y + tabs.layers.size.y / 2.0,
    );
    let slides_tab = Point2D::new(
        tabs.slides.origin.x + tabs.slides.size.x / 2.0,
        tabs.slides.origin.y + tabs.slides.size.y / 2.0,
    );

    host.apply_press(layers_tab.x, layers_tab.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        LeftPanelTab::Layers
    );
    assert!(
        host.slides_panel_frame(VW, VH).is_none(),
        "the layer tree owns the rail now"
    );

    // The tab row still takes clicks while the tree owns the rail.
    host.apply_press(slides_tab.x, slides_tab.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        LeftPanelTab::Slides
    );
    assert!(host.slides_panel_frame(VW, VH).is_some());
}

#[test]
fn the_layer_tree_starts_below_the_tab_row() {
    let _guard = test_lock();
    let mut deck = host_with(Some(TemplateScene::Slides));
    deck.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Layers;
    let tabs = deck.slides_tab_row(VW, VH).expect("tab row");
    let content = deck.layers_content_rect(VW, VH);
    assert_eq!(content.origin.y, tabs.row.origin.y + tabs.row.size.y);
}

#[test]
fn touch_layers_sheet_uses_one_visible_rect_for_tabs_layout_and_input() {
    let _guard = test_lock();
    for (size_class, viewport_w, viewport_h) in [
        (EditorSizeClass::Compact, 390.0, 844.0),
        (EditorSizeClass::Medium, 834.0, 1_112.0),
    ] {
        let mut host = host_with(Some(TemplateScene::Slides));
        {
            let ui = &mut host.editor_state.editor_ui;
            ui.touch = true;
            ui.size_class = size_class;
            ui.sidebar_open = false;
            ui.mobile_sheet = None;
        }
        host.last_viewport_w = viewport_w;
        host.last_viewport_h = viewport_h;

        assert!(host.slides_tab_row(viewport_w, viewport_h).is_none());
        assert!(host.slides_panel_frame(viewport_w, viewport_h).is_none());
        assert!(
            !host.slides_panel_press(20.0, 80.0, viewport_w, viewport_h),
            "a closed touch rail has no invisible tab targets"
        );

        host.editor_state.editor_ui.mobile_sheet = Some(MobileSheetKind::Layers);
        let sheet = host.mobile_sheet_rect(viewport_w, viewport_h, MobileSheetKind::Layers);
        let body = op_editor_ui::widgets::mobile_chrome::sheet_content_rect(sheet);
        let tabs = host
            .slides_tab_row(viewport_w, viewport_h)
            .expect("an open touch Layers sheet shows navigator tabs");
        assert_eq!(tabs.row.origin, body.origin);
        assert!(tabs.layers.size.x >= 44.0 && tabs.layers.size.y >= 44.0);
        assert!(tabs.slides.size.x >= 44.0 && tabs.slides.size.y >= 44.0);
        for rect in [tabs.row, tabs.layers, tabs.slides] {
            assert!(
                sheet.contains(rect.origin)
                    && sheet.contains(Point2D::new(
                        rect.origin.x + rect.size.x,
                        rect.origin.y + rect.size.y,
                    )),
                "visible tab geometry stays inside the sheet: {rect:?} vs {sheet:?}"
            );
        }

        let slides = host
            .slides_panel_frame(viewport_w, viewport_h)
            .expect("Slides owns the open sheet body");
        assert_eq!(slides.layout.panel, body);
        assert_eq!(slides.layout.tabs, tabs);

        // This is where the pre-sheet rail put its tab. On Medium it is
        // hidden behind the title band; on Compact it is outside the sheet.
        // Neither paint nor scroll uses it, so input must not use it either.
        let legacy_panel = op_editor_ui::widgets::host_canvas_geometry::layer_panel_rect(
            &host.editor_state,
            viewport_h,
        );
        let legacy_tabs =
            op_editor_ui::widgets::slides_panel_flow::tab_row(&host.editor_state, legacy_panel)
                .expect("fixture has boards");
        let hidden_point = Point2D::new(
            legacy_tabs.layers.origin.x + legacy_tabs.layers.size.x / 2.0,
            legacy_tabs.layers.origin.y + legacy_tabs.layers.size.y / 2.0,
        );
        assert!(!body.contains(hidden_point));
        assert!(!host.slides_panel_press(hidden_point.x, hidden_point.y, viewport_w, viewport_h,));
        assert_eq!(
            host.slides_panel_scroll(hidden_point, -80.0, viewport_w, viewport_h),
            None,
            "the old invisible rail geometry does not own scrolling"
        );

        let visible_layers = Point2D::new(
            tabs.layers.origin.x + tabs.layers.size.x / 2.0,
            tabs.layers.origin.y + tabs.layers.size.y / 2.0,
        );
        assert!(host.slides_panel_press(
            visible_layers.x,
            visible_layers.y,
            viewport_w,
            viewport_h,
        ));
        assert!(host.slides_panel_release(viewport_w, viewport_h));
        assert_eq!(
            host.editor_state.editor_ui.slides_panel.tab,
            LeftPanelTab::Layers
        );
        let layer_content = host.layers_content_rect(viewport_w, viewport_h);
        assert_eq!(layer_content.origin.y, tabs.row.origin.y + tabs.row.size.y);
        assert!(sheet.contains(layer_content.origin));
    }
}

#[test]
fn hovering_a_row_washes_it_and_leaving_the_rail_clears_it() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = row_centre(&mut host, 1);
    host.apply_cursor_move(point.x, point.y);
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.hover,
        Some(SlidesPanelTarget::Slide(1))
    );
    host.apply_cursor_move(VW / 2.0, VH / 2.0);
    assert_eq!(host.editor_state.editor_ui.slides_panel.hover, None);
}

#[test]
fn the_wheel_over_the_rail_scrolls_the_list_and_not_the_canvas() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    // A three-slide deck fits, so nothing scrolls but the event is still
    // the panel's — the canvas must not zoom under the rail.
    let point = row_centre(&mut host, 0);
    let zoom_before = host.editor_state.viewport.zoom;
    host.apply_wheel(point.x, point.y, -400.0, VW, VH);
    assert_eq!(
        host.editor_state.viewport.zoom, zoom_before,
        "the rail swallowed the wheel"
    );
}

/// The left rail's resize gutter must survive the drag going back OVER
/// the rail. A live resize is pointer capture; the slides tab claims
/// every point on the rail for its hover wash, and while it ran first the
/// rail could only ever be dragged wider (regression guard for
/// `cursor_move_panel_resize_tier`).
#[test]
fn the_rail_resize_gutter_drags_narrower_as_well_as_wider() {
    let _guard = test_lock();
    let start = {
        let host = host_with(Some(TemplateScene::Slides));
        host.editor_state.editor_ui.layer_panel_width
    };
    let y = op_editor_ui::widgets::TOP_BAR_HEIGHT + 200.0;

    let mut host = host_with(Some(TemplateScene::Slides));
    host.apply_press(start, y, VW, VH);
    assert!(
        host.is_resizing_panel(),
        "a press on the gutter starts a resize, not a slide click"
    );
    host.apply_cursor_move(start - 40.0, y);
    assert_eq!(
        host.editor_state.editor_ui.layer_panel_width,
        start - 40.0,
        "dragging left over the rail narrows it"
    );
    host.apply_release_with_viewport(VW, VH);
    assert!(!host.is_resizing_panel());

    let mut host = host_with(Some(TemplateScene::Slides));
    host.apply_press(start, y, VW, VH);
    host.apply_cursor_move(start + 40.0, y);
    assert_eq!(
        host.editor_state.editor_ui.layer_panel_width,
        start + 40.0,
        "dragging right still widens it"
    );
}

/// Both halves of the ±4 px gutter start a resize. The inner half sits
/// over the rail's own content, so the panel press tiers must not reach
/// it first.
#[test]
fn the_inner_half_of_the_gutter_starts_a_resize_too() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let start = host.editor_state.editor_ui.layer_panel_width;
    let y = op_editor_ui::widgets::TOP_BAR_HEIGHT + 200.0;
    host.apply_press(start - 2.0, y, VW, VH);
    assert!(
        host.is_resizing_panel(),
        "a press 2 px inside the edge is still the gutter's"
    );
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.pressed, None,
        "the slides list must not also arm a row press"
    );
    host.apply_cursor_move(start - 42.0, y);
    assert_eq!(
        host.editor_state.editor_ui.layer_panel_width,
        start - 40.0,
        "the drag is measured from the press point, not the edge"
    );
}
