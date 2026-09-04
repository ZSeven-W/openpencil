//! Web-host routing for the left rail's slides tab — the twin of
//! native's `op-host-native/src/widget_host/slides_panel_tests.rs`. Both
//! hosts run the same shared flow, and these tests are what proves it:
//! the pair has silently drifted apart every time only one side was
//! covered.

use super::WidgetHost;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::{LeftPanelTab, SlidesPanelTarget};
use op_editor_ui::Point2D;

const VW: f32 = 1_400.0;
const VH: f32 = 900.0;

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

fn host_with(scenario: Option<TemplateScene>) -> WidgetHost {
    let document = jian_ops_schema::load_str(THREE_BOARD_DECK)
        .expect("parse deck fixture")
        .value;
    let mut host = WidgetHost::new();
    host.editor_state = op_editor_core::EditorState::from_document(document);
    host.editor_state.editor_ui.scenario = scenario;
    host.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    host.editor_state_dirty = true;
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host
}

fn row_centre(host: &mut WidgetHost, index: usize) -> Point2D {
    let slides = host
        .slides_panel_frame(VW, VH)
        .expect("a deck shows the slides tab");
    let rect = slides.layout.row_rect(index);
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn any_document_with_boards_gets_the_slides_tab() {
    let mut deck = host_with(Some(TemplateScene::Slides));
    let slides = deck
        .slides_panel_frame(VW, VH)
        .expect("a deck shows the slides tab");
    assert_eq!(slides.chips.len(), 3);
    assert_eq!(slides.chips[1].name, "议程");
    assert!(deck.slides_tab_row(VH).is_some());

    // The same three boards with no scenario recorded: an ordinary
    // design page still gets the navigator, which is the whole point of
    // the tab being permanent. Twin of native's test of the same name.
    let mut ordinary = host_with(None);
    let listed = ordinary
        .slides_panel_frame(VW, VH)
        .expect("an untagged page with boards still lists them");
    assert_eq!(listed.chips.len(), 3);
    assert!(ordinary.slides_tab_row(VH).is_some());
}

#[test]
fn a_page_with_no_boards_has_nothing_to_list_and_shows_no_tab() {
    let mut empty = WidgetHost::new();
    empty.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    // The starter document opens with one empty Frame, which IS a board;
    // clear it so this covers the genuinely empty page.
    empty.editor_state.active_children_mut().clear();
    empty.editor_state_dirty = true;
    empty.last_viewport_w = VW;
    empty.last_viewport_h = VH;
    assert!(active_page_boards(&empty.editor_state).is_empty());
    assert!(empty.slides_tab_row(VH).is_none());
    assert!(empty.slides_panel_frame(VW, VH).is_none());
    assert_eq!(
        empty.layers_content_rect(VH).origin.y,
        op_editor_ui::widgets::TOP_BAR_HEIGHT,
        "a document without a tab row keeps the whole rail"
    );
}

#[test]
fn the_browser_states_it_cannot_render_thumbnails() {
    // A declared capability, not a silent gap — the rows still list,
    // navigate and reorder; only the picture is missing, and the widget
    // fills the box with the slide number instead.
    let host = host_with(Some(TemplateScene::Slides));
    assert!(
        !host.editor_state.editor_ui.slide_thumbnails_supported,
        "the browser has no local board renderer to declare"
    );
}

#[test]
fn clicking_a_row_frames_that_board() {
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = row_centre(&mut host, 2);
    let before = host.editor_state.viewport;
    host.apply_press(point.x, point.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert_ne!(host.editor_state.viewport, before, "the camera moved");
    assert!(
        host.editor_state.history.past.is_empty(),
        "navigating a deck never lands on the undo stack"
    );
}

#[test]
fn dragging_a_row_reorders_the_deck() {
    let mut host = host_with(Some(TemplateScene::Slides));
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
        ["slide-2", "slide-1", "slide-3"]
    );
}

#[test]
fn the_tab_row_switches_the_rail_both_ways() {
    let mut host = host_with(Some(TemplateScene::Slides));
    let tabs = host.slides_tab_row(VH).expect("tab row");
    let centre = |rect: op_editor_ui::Rect| {
        Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        )
    };
    let layers_tab = centre(tabs.layers);
    let slides_tab = centre(tabs.slides);

    host.apply_press(layers_tab.x, layers_tab.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        LeftPanelTab::Layers
    );
    assert!(host.slides_panel_frame(VW, VH).is_none());

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
    let mut deck = host_with(Some(TemplateScene::Slides));
    deck.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Layers;
    let tabs = deck.slides_tab_row(VH).expect("tab row");
    assert_eq!(
        deck.layers_content_rect(VH).origin.y,
        tabs.row.origin.y + tabs.row.size.y
    );
}

#[test]
fn hovering_a_row_washes_it_and_leaving_the_rail_clears_it() {
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
fn the_footer_button_enters_preview() {
    let mut host = host_with(Some(TemplateScene::Slides));
    host.editor_state.editor_ui.login_modal_open = true;
    host.editor_state.editor_ui.prompt_center.open = true;
    host.editor_state.editor_ui.prompt_center.save_open = true;
    host.editor_state.editor_ui.theme_mode = op_editor_core::ThemeMode::Light;
    host.editor_state.editor_ui.locale = op_editor_core::Locale::Ja;
    host.editor_state.editor_ui.account = op_editor_core::AccountState::dev_fake_signed_in();
    host.editor_state.ui.property_focus = Some(op_editor_core::PropertyFocus::PositionX);
    host.editor_state.ui.property_input.set_text("draft");
    host.editor_state
        .ui
        .property_input
        .set_composition("ni", 2, 0);
    let document_before = host.editor_state.doc.clone();
    let auth_actions_before = host.pending_auth_actions.clone();
    let button = {
        let slides = host.slides_panel_frame(VW, VH).expect("slides tab");
        Point2D::new(
            slides.layout.actions.present.origin.x + slides.layout.actions.present.size.x / 2.0,
            slides.layout.actions.present.origin.y + slides.layout.actions.present.size.y / 2.0,
        )
    };
    assert!(
        host.apply_press(button.x, button.y, VW, VH),
        "press is consumed"
    );
    assert!(
        host.editor_state.ui.property_focus.is_none(),
        "the ordinary slides-panel press blurs property focus before release"
    );
    assert!(host.editor_state.ui.property_input.composition().is_none());
    host.apply_release_with_viewport(VW, VH);
    assert!(!host.editor_state.editor_ui.preview.mode);
    assert_eq!(host.editor_state.doc, document_before);
    assert!(host.editor_state.editor_ui.login_modal_open);
    assert!(host.editor_state.editor_ui.prompt_center.open);
    assert_eq!(host.pending_auth_actions, auth_actions_before);
    assert_eq!(
        host.editor_state.editor_ui.preview.warnings,
        vec!["preview: CanvasKit not initialized".to_string()]
    );
}

/// The tab row must not swallow a cursor move while the chat model
/// picker is open — the picker paints above the rail, and web resolves
/// it in a LATER tier than the rail (native resolves it earlier). A
/// picker left open with no layoutable bounds heals on exactly the
/// dispatch the rail would otherwise have claimed, so this is the guard
/// that keeps the two hosts' z-order agreeing.
#[test]
fn the_tab_row_yields_the_cursor_to_an_open_model_picker() {
    let mut host = host_with(Some(TemplateScene::Slides));
    let tabs = host.slides_tab_row(VH).expect("tab row");
    let on_the_row = Point2D::new(
        tabs.row.origin.x + 20.0,
        tabs.row.origin.y + tabs.row.size.y / 2.0,
    );
    // Closed picker: the row owns its own pixels.
    assert!(
        host.slides_panel_cursor_tier(on_the_row, false).is_some(),
        "the tab row owns a point on itself"
    );

    host.editor_state.editor_ui.chat_model_picker.open = true;
    assert!(
        host.slides_panel_cursor_tier(on_the_row, true).is_none(),
        "an overlay above the rail takes the move instead"
    );
}
